//! Native host support for the bounded live guest demonstration.

use crate::Result;
use cpu_time::ProcessTime;
use mvm_build::rootfs_inject::{CpioEntry, build_newc_cpio};
use mvm_runtime::backends::hvf::{
    DiskImage, HostChannels, KernelBootResult, KernelBootUntilParams, boot_kernel_until,
    default_bootargs,
};
use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_GUEST: usize = 32 * 1024 * 1024;
const MAX_BOOTSTRAP: usize = 64 * 1024;
const MAX_INITRAMFS: usize = 64 * 1024 * 1024;
const MAX_CONSOLE: usize = 2 * 1024 * 1024;
const DISK_BYTES: u64 = 256 * 1024 * 1024;
const SYSTEM_OFF: u64 = 0x8400_0008;
const SYSTEM_RESET: u64 = 0x8400_0009;
const PINNED_KERNEL_SHA256: &str =
    "0f97f109154a173352c07db223ecff48d44b38514b500f39cd9897701dbac0b6";

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

/// In-process development witness ownership, not production launch admission.
/// Retain the canonical backing and sidecar until the device and final sync finish.
struct HeldDisk {
    file: File,
    _custody: Option<mvm_core::util::writable_image::LockedWritableImage>,
}

fn prepare_disk(path: &Path, read_only: bool) -> io::Result<(HeldDisk, DiskImage)> {
    use mvm_core::util::writable_image::{LockedWritableImage, sidecar_lock_path};
    let custody = if read_only {
        None
    } else {
        Some(LockedWritableImage::open(path, &sidecar_lock_path(path))?)
    };
    let file = match &custody {
        Some(held) => held.backing().try_clone()?,
        None => OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)?,
    };
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() != DISK_BYTES {
        return Err(invalid("live data disk must be a 256 MiB regular file"));
    }
    let image = if read_only {
        DiskImage::readonly_file(file.try_clone()?)?
    } else {
        DiskImage::writable_file(file.try_clone()?)?
    };
    Ok((
        HeldDisk {
            file,
            _custody: custody,
        },
        image,
    ))
}

/// Pack only fixed paths. The credential-bearing result must stay in memory or
/// an owned pipe; it is deliberately not written by this API.
pub fn initramfs(guest_elf: &[u8], bootstrap_json: &[u8]) -> Result<Vec<u8>> {
    if guest_elf.len() > MAX_GUEST || bootstrap_json.len() > MAX_BOOTSTRAP {
        return Err(invalid("guest or bootstrap exceeds its byte limit").into());
    }
    // This is a header/architecture check, not an ELF loader or a claim that
    // the executable will run. Linux and the completion gate check that next.
    if guest_elf.len() < 64
        || &guest_elf[..7] != b"\x7fELF\x02\x01\x01"
        || !matches!(u16::from_le_bytes([guest_elf[16], guest_elf[17]]), 2 | 3)
        || guest_elf[18..20] != 183_u16.to_le_bytes()
        || guest_elf[20..24] != 1_u32.to_le_bytes()
        || guest_elf[52..54] != 64_u16.to_le_bytes()
    {
        return Err(invalid("guest must have an ARM64 little-endian ELF64 header").into());
    }
    let bootstrap: Value = serde_json::from_slice(bootstrap_json)
        .map_err(|_| invalid("bootstrap must be a bounded JSON object"))?;
    if !bootstrap.is_object() {
        return Err(invalid("bootstrap must be a bounded JSON object").into());
    }
    let mut entries: Vec<_> = [".", "proc", "sys", "dev", "tmp", "run", "data"]
        .into_iter()
        .map(CpioEntry::dir)
        .collect();
    entries.push(CpioEntry::file("init", 0o755, guest_elf.to_vec()));
    entries.push(CpioEntry::file(
        "bootstrap.json",
        0o600,
        bootstrap_json.to_vec(),
    ));
    let archive = build_newc_cpio(&entries)?;
    if archive.len() > MAX_INITRAMFS {
        return Err(invalid("packed initramfs exceeds its byte limit").into());
    }
    Ok(archive)
}

/// Create an exclusively owned sparse data disk. Failure leaves any created
/// file in place for the caller to diagnose or remove; existing paths survive.
pub fn create_disk(path: &Path) -> Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.set_len(DISK_BYTES)?;
    mvm_fs::ext4::mkfs::format_empty_ext4_labeled(&mut file, DISK_BYTES, b"spiral-cache")?;
    file.sync_all()?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn read_bounded(reader: impl Read, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() > limit {
        return Err(invalid("initramfs input is empty or exceeds its byte limit").into());
    }
    Ok(bytes)
}

#[derive(Default)]
struct ConsoleCapture {
    bytes: Vec<u8>,
    overflow: bool,
    line_start: usize,
    ready_clock: Option<ReadyClock>,
    invalid_ready_clock: bool,
}

#[derive(Clone, Copy)]
struct ReadyClock {
    host_unix_ms: i64,
    guest_unix_ms: i64,
}

struct BoundedConsole(Arc<Mutex<ConsoleCapture>>);

impl Write for BoundedConsole {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut capture = self
            .0
            .lock()
            .map_err(|_| io::Error::other("console capture lock poisoned"))?;
        let previous_length = capture.bytes.len();
        let remaining = MAX_CONSOLE.saturating_sub(previous_length);
        capture.overflow |= bytes.len() > remaining;
        capture
            .bytes
            .extend_from_slice(&bytes[..bytes.len().min(remaining)]);
        // Scan each newly captured byte once. Timestamp the completed ready
        // line on arrival, rather than after the entire boot/campaign returns.
        for index in previous_length..capture.bytes.len() {
            if capture.bytes[index] != b'\n' {
                continue;
            }
            let line = capture.bytes[capture.line_start..index].trim_ascii();
            let event = line
                .starts_with(b"{")
                .then(|| serde_json::from_slice::<Value>(line).ok())
                .flatten();
            if let Some(event) = event
                && event.get("event").and_then(Value::as_str) == Some("guest_ready")
            {
                if capture.ready_clock.is_some() {
                    capture.invalid_ready_clock = true;
                } else if let (Some(guest_unix_ms), Ok(host_unix_ms)) = (
                    event.get("utc_unix_ms").and_then(Value::as_i64),
                    crate::config::now_ms(),
                ) {
                    capture.ready_clock = Some(ReadyClock {
                        host_unix_ms,
                        guest_unix_ms,
                    });
                } else {
                    capture.invalid_ready_clock = true;
                }
            }
            capture.line_start = index + 1;
        }
        // Continue consuming UART output without allocating after overflow.
        // Completion is refused below, rather than silently truncating proof.
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn checked_ready_clock(capture: &ConsoleCapture) -> Result<(i64, i64, i64)> {
    if capture.invalid_ready_clock {
        return Err(invalid("ambiguous or malformed guest_ready clock record").into());
    }
    let ready = capture
        .ready_clock
        .ok_or("missing guest_ready clock arrival")?;
    let offset = ready
        .guest_unix_ms
        .checked_sub(ready.host_unix_ms)
        .ok_or("guest clock offset overflow")?;
    if offset.unsigned_abs() > 5000 {
        return Err(invalid("guest clock offset exceeds the 5 second live profile").into());
    }
    Ok((ready.host_unix_ms, ready.guest_unix_ms, offset))
}

fn completion_events(
    boot: &KernelBootResult,
    console: &[u8],
    kernel_sha256: &str,
) -> Result<Vec<Value>> {
    // Pinned MVM's native Linux control records two non-HVC exceptions:
    // specs/benchmarks/327-hvf-quota-spike-findings.md:244-257. Our first two
    // boots of the pinned Image identified their class as 0x18 (system register).
    // This admits only that trusted image's observed compatibility baseline;
    // the individual register identities have not been qualified.
    let native_baseline = kernel_sha256 == PINNED_KERNEL_SHA256
        && boot.other_exceptions == 2
        && boot.other_ecs == [0x18];
    let no_other_exceptions = boot.other_exceptions == 0 && boot.other_ecs.is_empty();
    if boot.stopped_by_watchdog
        || !boot.psci_fns.contains(&SYSTEM_OFF)
        || boot.psci_fns.contains(&SYSTEM_RESET)
        || !(no_other_exceptions || native_baseline)
        || boot.workload_exit_code.is_some_and(|code| code != 0)
    {
        return Err(io::Error::other(format!(
            "guest did not complete a clean power-off; boot_result={}",
            json!({
                "stopped_by_watchdog": boot.stopped_by_watchdog,
                "exit_reason": boot.exit_reason,
                "psci_fns": boot.psci_fns,
                "hvc_calls": boot.hvc_calls,
                "other_exceptions": boot.other_exceptions,
                "other_ecs": boot.other_ecs,
                "workload_exit_code": boot.workload_exit_code,
                "final_pc": boot.final_pc,
                "kernel_sha256": kernel_sha256,
            })
        ))
        .into());
    }
    let text = std::str::from_utf8(console).map_err(|_| invalid("guest console is not UTF-8"))?;
    let mut events = Vec::new();
    let mut completed = 0;
    for line in text.lines().map(str::trim) {
        if !line.starts_with('{') {
            continue;
        }
        if completed != 0 {
            return Err(invalid("guest emitted a JSON record after guest_done").into());
        }
        let event: Value = serde_json::from_str(line)
            .map_err(|_| invalid("malformed guest console JSON record"))?;
        if event.get("event").and_then(Value::as_str).is_none() {
            return Err(invalid("guest JSON record is missing its event type").into());
        }
        if event["event"] == "guest_done" {
            if event.get("ok").and_then(Value::as_bool) != Some(true) {
                return Err(io::Error::other("guest reported unsuccessful completion").into());
            }
            completed += 1;
        }
        events.push(event);
    }
    if completed != 1 {
        return Err(io::Error::other("guest must report exactly one successful guest_done").into());
    }
    Ok(events)
}

fn failure(reason: &str, console: &[u8]) -> io::Error {
    // Only the public console is eligible for diagnostics. Neither the initrd
    // nor the bootstrap is formatted into an error, even on malformed input.
    let tail = &console[console.len().saturating_sub(16 * 1024)..];
    io::Error::other(format!(
        "{reason}; public console tail:\n{}",
        String::from_utf8_lossy(tail)
    ))
}

fn micros(duration: Duration) -> Result<u64> {
    Ok(duration.as_micros().try_into()?)
}

/// Run in a dedicated native worker process. Stdin is the sole initramfs input;
/// the caller owns the disk/socket and remains responsible for their lifecycle.
pub fn worker(kernel: &Path, disk: Option<&Path>, mailbox_socket: Option<&Path>) -> Result<Value> {
    worker_with_profile(kernel, disk, mailbox_socket, Default::default())
}

fn require_original_mailbox_path(socket: Option<&Path>) -> io::Result<()> {
    if socket.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "caller-provided HVF mailbox socket unavailable; launch the original signed SDK pair with tools/ari-own-mailbox-witness (paired-journal)",
        ));
    }
    Ok(())
}

/// Legacy profile is retained for measurement of the development worker.
pub fn worker_with_profile(
    kernel: &Path,
    disk: Option<&Path>,
    mailbox_socket: Option<&Path>,
    bandwidth_profile: crate::config::MailboxBandwidthProfile,
) -> Result<Value> {
    require_original_mailbox_path(mailbox_socket)?;
    worker_prepared(kernel, disk, bandwidth_profile, None)
}

pub fn worker_installed(
    kernel: &Path,
    disk: Option<&Path>,
    mailbox_socket: Option<&Path>,
    bandwidth_profile: crate::config::MailboxBandwidthProfile,
    expected_role: u8,
) -> Result<Value> {
    require_original_mailbox_path(mailbox_socket)?;
    let wall = Instant::now();
    let cpu = ProcessTime::try_now()?;
    let mut context =
        crate::installed_worker::InstalledWorkerContext::from_inherited_fd(expected_role)?;
    let input = context.read_initrd()?;
    let setup_cpu = micros(cpu.try_elapsed()?)?;
    let setup_wall = micros(wall.elapsed())?;
    let mut value = worker_prepared(kernel, disk, bandwidth_profile, Some((context, input)))?;
    value["installed_control_input_cpu_us"] = json!(setup_cpu);
    value["installed_control_input_wall_us"] = json!(setup_wall);
    value["worker_process_cpu_us"] = json!(micros(cpu.try_elapsed()?)?);
    value["worker_wall_us"] = json!(micros(wall.elapsed())?);
    Ok(value)
}

fn worker_prepared(
    kernel: &Path,
    disk: Option<&Path>,
    bandwidth_profile: crate::config::MailboxBandwidthProfile,
    installed: Option<(crate::installed_worker::InstalledWorkerContext, Vec<u8>)>,
) -> Result<Value> {
    let wall = Instant::now();
    let cpu = ProcessTime::try_now()?;
    for variable in [
        "MVM_HVF_BOOTARGS",
        "MVM_HVF_BOOTARGS_EXTRA",
        "MVM_HVF_AGENT_SOCKET",
        "MVM_HVF_SUBSTITUTION_SOCKET",
    ] {
        if std::env::var_os(variable).is_some() {
            return Err(invalid("live worker refuses ambient boot/channel overrides").into());
        }
    }
    let (mut installed, initrd) = match installed {
        Some((context, input)) => (Some(context), input),
        None => (None, read_bounded(io::stdin().lock(), MAX_INITRAMFS)?),
    };
    // Hash and load one held file, rather than validating a pathname and then
    // reopening a potentially different image. The helper reads raw image bytes.
    let kernel_file = File::open(kernel)?;
    let kernel_sha256 = mvm_core::crypto::verified_digest::sha256_open_file_verified(&kernel_file)?;
    let (backing, disks) = match disk {
        Some(path) => {
            let (held, image) = prepare_disk(path, true)?;
            (Some(held), vec![image])
        }
        None => (None, Vec::new()),
    };
    let capture = Arc::new(Mutex::new(ConsoleCapture::default()));
    let channels = HostChannels {
        mem_mib: 1024,
        vcpus: 1,
        console_sink: Some(Box::new(BoundedConsole(Arc::clone(&capture)))),
        ..Default::default()
    };
    let bootargs = format!("{} rdinit=/init", default_bootargs(false));
    let params =
        KernelBootUntilParams::builder_descriptor(&kernel_file, kernel, Duration::from_secs(120))
            .literal_bootargs(&bootargs)?
            .initramfs(Some(&initrd))
            .disks(disks)
            .vsock(false)
            .channels(channels)
            .build();
    let _host_lifetime = if let Some(context) = &mut installed {
        context.authorize_boot()?;
        Some(context.holder_handoff()?)
    } else {
        None
    };
    let boot_started = Instant::now();
    let boot = boot_kernel_until(params);
    let boot_wall_us = micros(boot_started.elapsed())?;
    // DiskImage's drop-time flush logs failures but cannot return them. Check
    // our retained backing descriptor explicitly, also after a failed boot.
    let sync_started = Instant::now();
    let synced = backing
        .as_ref()
        .map(|held| held.file.sync_all())
        .transpose();
    let backing_disk_sync_us = backing
        .as_ref()
        .map(|_| micros(sync_started.elapsed()))
        .transpose()?;
    let (console, overflow, clock) = {
        let mut captured = capture
            .lock()
            .map_err(|_| io::Error::other("console capture lock poisoned"))?;
        let clock = checked_ready_clock(&captured);
        (
            std::mem::take(&mut captured.bytes),
            captured.overflow,
            clock,
        )
    };
    synced.map_err(|error| failure(&format!("backing disk sync failed: {error}"), &console))?;
    let boot = boot.map_err(|error| failure(&format!("HVF boot failed: {error:?}"), &console))?;
    if overflow {
        return Err(failure("guest console exceeded 2 MiB", &console).into());
    }
    let events = completion_events(&boot, &console, &kernel_sha256)
        .map_err(|error| failure(&error.to_string(), &console))?;
    let (guest_ready_host_unix_ms, guest_ready_guest_unix_ms, guest_clock_offset_ms) =
        clock.map_err(|error| failure(&error.to_string(), &console))?;
    drop(initrd);
    let worker_process_cpu_us = micros(cpu.try_elapsed()?)?;
    let worker_wall_us = micros(wall.elapsed())?;
    Ok(json!({
        "event": "worker_done",
        "installed_host_lease": installed.is_some(),
        "installed_generation": installed.as_ref().map(|c| c.binding().generation),
        "installed_role": installed.as_ref().map(|c| c.binding().role),
        "backend_exit_confirmed": true,
        "mailbox_bandwidth_profile": bandwidth_profile,
        "mailbox_rate_enforced": false,
        "mailbox_rate_bytes_per_second": null,
        "mailbox_burst_bytes": null,
        "ok": true,
        "console": String::from_utf8(console)?,
        "guest_events": events,
        "stopped_by_watchdog": boot.stopped_by_watchdog,
        "psci_fns": boot.psci_fns,
        "workload_exit_code": boot.workload_exit_code,
        "kernel_sha256": kernel_sha256,
        "other_exceptions": boot.other_exceptions,
        "other_ecs": boot.other_ecs,
        "exception_profile": if boot.other_exceptions == 0 {
            "no-other-exceptions"
        } else {
            "pinned-native-linux-two-system-register-traps"
        },
        "resident_ram_bytes": boot.resident_ram_bytes,
        "usage": boot.usage,
        "worker_process_cpu_us": worker_process_cpu_us,
        "worker_wall_us": worker_wall_us,
        "boot_wall_us": boot_wall_us,
        "backing_disk_sync_us": backing_disk_sync_us,
        "backing_disk_synced": backing.is_some(),
        "guest_ready_host_unix_ms": guest_ready_host_unix_ms,
        "guest_ready_guest_unix_ms": guest_ready_guest_unix_ms,
        "guest_clock_offset_ms": guest_clock_offset_ms,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sparse_test_disk(path: &Path) {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .unwrap();
        file.set_len(DISK_BYTES).unwrap();
    }

    #[test]
    fn held_disk_retains_canonical_writer_exclusion_through_device_release() {
        use mvm_core::util::writable_image::{LockedWritableImage, sidecar_lock_path};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("disk");
        sparse_test_disk(&path);
        let (held, image) = prepare_disk(&path, false).unwrap();
        assert!(LockedWritableImage::open(&path, &sidecar_lock_path(&path)).is_err());
        drop(image);
        assert!(LockedWritableImage::open(&path, &sidecar_lock_path(&path)).is_err());
        held.file.sync_all().unwrap();
        drop(held);
        assert!(LockedWritableImage::open(&path, &sidecar_lock_path(&path)).is_ok());
    }

    #[test]
    fn readonly_disk_uses_the_validated_object_after_path_replacement() {
        use std::os::unix::fs::{FileExt, MetadataExt};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("disk");
        sparse_test_disk(&path);
        OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .write_at(b"A", 0)
            .unwrap();
        let (held, image) = prepare_disk(&path, true).unwrap();
        let original = held.file.metadata().unwrap();
        held.file.sync_all().unwrap(); // The worker also checks sync on readback boots.
        std::fs::rename(&path, dir.path().join("original")).unwrap();
        sparse_test_disk(&path);
        OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .write_at(b"B", 0)
            .unwrap();
        let DiskImage::File {
            file, read_only, ..
        } = &image
        else {
            panic!("expected file backing")
        };
        assert!(*read_only);
        assert_eq!(file.metadata().unwrap().ino(), original.ino());
        assert_eq!(file.metadata().unwrap().dev(), original.dev());
        let mut byte = [0];
        file.read_at(&mut byte, 0).unwrap();
        assert_eq!(&byte, b"A");
    }

    #[test]
    fn held_disk_refuses_wrong_size_directory_and_final_symlinks() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("disk");
        File::create(&path).unwrap().set_len(1).unwrap();
        for read_only in [false, true] {
            assert!(prepare_disk(&path, read_only).is_err());
            assert!(prepare_disk(dir.path(), read_only).is_err());
        }
        std::fs::remove_file(&path).unwrap();
        sparse_test_disk(&path);
        let alias = dir.path().join("alias");
        symlink(&path, &alias).unwrap();
        for read_only in [false, true] {
            assert!(prepare_disk(&alias, read_only).is_err());
        }
        let sidecar = mvm_core::util::writable_image::sidecar_lock_path(&path);
        std::fs::remove_file(&sidecar).unwrap();
        symlink(dir.path().join("uncreated"), &sidecar).unwrap();
        assert!(prepare_disk(&path, false).is_err());
    }

    const DONE: &[u8] = b"Linux public boot log\n{\"event\":\"guest_done\",\"ok\":true}\n";

    fn clean_boot() -> KernelBootResult {
        KernelBootResult {
            psci_fns: vec![SYSTEM_OFF],
            ..Default::default()
        }
    }

    #[test]
    fn bounds_pipe_reads_without_requiring_end_of_stream() {
        assert_eq!(read_bounded(&b"1234"[..], 4).unwrap(), b"1234");
        assert!(read_bounded(&b""[..], 4).is_err());
        assert!(read_bounded(io::repeat(0), 4).is_err());
    }

    #[test]
    fn completion_requires_exactly_one_success_record_and_clean_poweroff() {
        assert_eq!(completion_events(&clean_boot(), DONE, "").unwrap().len(), 1);
        for console in [
            &b"Linux log without guest completion"[..],
            &b"{\"event\":\"guest_done\",\"ok\":false}\n"[..],
            &b"{\"event\":\"guest_done\",\"ok\":true}\n{\"event\":\"guest_done\",\"ok\":true}\n"[..],
            &b"{\"event\":\"guest_done\",\"ok\":true}\n{\"event\":\"guest_error\"}\n"[..],
            &b"{broken json}\n"[..],
        ] {
            assert!(completion_events(&clean_boot(), console, "").is_err());
        }
        let mut watchdog = clean_boot();
        watchdog.stopped_by_watchdog = true;
        let mut reset = clean_boot();
        reset.psci_fns.push(SYSTEM_RESET);
        let mut exception = clean_boot();
        exception.other_exceptions = 1;
        let mut failed_workload = clean_boot();
        failed_workload.workload_exit_code = Some(1);
        for boot in [
            KernelBootResult::default(),
            watchdog,
            reset,
            exception,
            failed_workload,
        ] {
            assert!(completion_events(&boot, DONE, "").is_err());
        }
    }

    #[test]
    fn admits_only_the_pinned_native_linux_exception_baseline() {
        let mut baseline = clean_boot();
        baseline.other_exceptions = 2;
        baseline.other_ecs = vec![0x18];
        assert!(completion_events(&baseline, DONE, "unqualified-kernel").is_err());
        baseline.other_exceptions = 3;
        assert!(completion_events(&baseline, DONE, PINNED_KERNEL_SHA256).is_err());
        baseline.other_exceptions = 2;
        baseline.other_ecs = vec![0x19];
        assert!(completion_events(&baseline, DONE, PINNED_KERNEL_SHA256).is_err());
        baseline.other_ecs = vec![0x18, 0x19];
        assert!(completion_events(&baseline, DONE, PINNED_KERNEL_SHA256).is_err());
        baseline.other_ecs = vec![0x18];
        assert!(completion_events(&baseline, DONE, PINNED_KERNEL_SHA256).is_ok());
    }

    #[test]
    fn console_overflow_is_bounded_and_observable() {
        let capture = Arc::new(Mutex::new(ConsoleCapture::default()));
        let mut sink = BoundedConsole(Arc::clone(&capture));
        let data = vec![b'x'; MAX_CONSOLE];
        sink.write_all(&data).unwrap();
        assert!(!capture.lock().unwrap().overflow);
        sink.write_all(b"overflow").unwrap();
        let capture = capture.lock().unwrap();
        assert!(capture.overflow);
        assert_eq!(capture.bytes, data);
    }

    #[test]
    fn timestamps_ready_when_its_split_line_arrives_and_refuses_duplicates() {
        let capture = Arc::new(Mutex::new(ConsoleCapture::default()));
        let mut sink = BoundedConsole(Arc::clone(&capture));
        let guest = crate::config::now_ms().unwrap();
        write!(sink, "{{\"event\":\"guest_ready\",\"utc_unix_ms\":{guest}").unwrap();
        assert!(capture.lock().unwrap().ready_clock.is_none());
        sink.write_all(b"}\n").unwrap();
        let (host, recorded_guest, offset) = checked_ready_clock(&capture.lock().unwrap()).unwrap();
        assert_eq!(recorded_guest, guest);
        assert!(host >= guest);
        assert_eq!(offset, guest - host);
        writeln!(
            sink,
            "{{\"event\":\"guest_ready\",\"utc_unix_ms\":{guest}}}"
        )
        .unwrap();
        assert!(checked_ready_clock(&capture.lock().unwrap()).is_err());
    }

    #[test]
    fn refuses_missing_or_out_of_profile_ready_clock() {
        assert!(checked_ready_clock(&ConsoleCapture::default()).is_err());
        let mut capture = ConsoleCapture {
            ready_clock: Some(ReadyClock {
                host_unix_ms: 10000,
                guest_unix_ms: 15000,
            }),
            ..Default::default()
        };
        assert_eq!(checked_ready_clock(&capture).unwrap().2, 5000);
        capture.ready_clock.as_mut().unwrap().guest_unix_ms += 1;
        assert!(checked_ready_clock(&capture).is_err());
    }
}

#[cfg(test)]
#[test]
fn legacy_mailbox_path_refuses_before_worker_or_kernel_inputs() {
    let kernel = Path::new("/nonexistent-fixture-kernel");
    let socket = Some(Path::new("/nonexistent-fixture-mailbox"));
    for result in [
        worker(kernel, None, socket),
        worker_installed(kernel, None, socket, Default::default(), 1),
    ] {
        let err = result.expect_err("unadmitted caller-supplied mailbox");
        assert_eq!(
            err.downcast_ref::<io::Error>().map(io::Error::kind),
            Some(io::ErrorKind::Unsupported)
        );
    }
}
