//! Linux/Firecracker host support for the canonical mailbox development witness.
use crate::Result;
use serde_json::Value;
use std::io;
#[cfg(target_os = "linux")]
use std::path::Path;

const MAX_GUEST: usize = 32 * 1024 * 1024;
#[cfg(target_os = "linux")]
const MAX_BOOTSTRAP: usize = 64 * 1024;
#[cfg(target_os = "linux")]
const MAX_INITRAMFS: usize = 64 * 1024 * 1024;
const MAX_CONSOLE: usize = 2 * 1024 * 1024;
#[cfg(target_os = "linux")]
const DISK_BYTES: u64 = 256 * 1024 * 1024;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn validate_guest(bytes: &[u8], machine: u16) -> Result<()> {
    if bytes.len() < 64
        || bytes.len() > MAX_GUEST
        || &bytes[..7] != b"\x7fELF\x02\x01\x01"
        || !matches!(u16::from_le_bytes([bytes[16], bytes[17]]), 2 | 3)
        || bytes[18..20] != machine.to_le_bytes()
        || bytes[20..24] != 1_u32.to_le_bytes()
        || bytes[52..54] != 64_u16.to_le_bytes()
    {
        return Err(invalid("guest ELF header does not match the Linux host architecture").into());
    }
    Ok(())
}

struct Completion {
    events: Vec<Value>,
    guest_ready_unix_ms: i64,
    clock_offset_ms: i64,
}

fn require_exit_code(code: Option<i32>) -> Result<()> {
    if code != Some(0) {
        return Err(invalid("missing or unsuccessful canonical workload exit report").into());
    }
    Ok(())
}

fn parse_console(bytes: &[u8], ready_host_ms: i64) -> Result<Completion> {
    if bytes.len() > MAX_CONSOLE {
        return Err(invalid("console exceeds 2 MiB").into());
    }
    let text = std::str::from_utf8(bytes)?;
    let mut events = Vec::new();
    let mut ready = None;
    let mut done = false;
    for line in text
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('{'))
    {
        if done {
            return Err(invalid("JSON event after guest_done").into());
        }
        let event: Value = serde_json::from_str(line)?;
        match event["event"].as_str() {
            Some("guest_ready") => {
                if ready.is_some() {
                    return Err(invalid("duplicate guest_ready").into());
                }
                ready = Some(event["utc_unix_ms"].as_i64().ok_or("missing guest clock")?);
            }
            Some("guest_done") => {
                if event["ok"] != true {
                    return Err(invalid("guest reported failure").into());
                }
                done = true;
            }
            Some(_) => (),
            None => return Err(invalid("public event is missing its type").into()),
        }
        events.push(event);
    }
    if !done {
        return Err(invalid("missing guest_done").into());
    }
    let guest_ready_unix_ms = ready.ok_or("missing guest_ready")?;
    let clock_offset_ms = guest_ready_unix_ms
        .checked_sub(ready_host_ms)
        .ok_or("clock overflow")?;
    if clock_offset_ms.unsigned_abs() > 5000 {
        return Err(invalid("guest clock offset exceeds five seconds").into());
    }
    Ok(Completion {
        events,
        guest_ready_unix_ms,
        clock_offset_ms,
    })
}

/// Pack a host-architecture guest and private bootstrap; never persist the archive.
#[cfg(target_os = "linux")]
pub fn initramfs(guest_elf: &[u8], bootstrap_json: &[u8]) -> Result<Vec<u8>> {
    use mvm_build::rootfs_inject::{CpioEntry, build_newc_cpio};
    let machine = if cfg!(target_arch = "x86_64") {
        62
    } else if cfg!(target_arch = "aarch64") {
        183
    } else {
        return Err("unsupported Linux host architecture".into());
    };
    validate_guest(guest_elf, machine)?;
    if bootstrap_json.len() > MAX_BOOTSTRAP
        || !serde_json::from_slice::<Value>(bootstrap_json)?.is_object()
    {
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
    let archive = build_newc_cpio(&entries);
    if archive.len() > MAX_INITRAMFS {
        return Err(invalid("initramfs exceeds its byte limit").into());
    }
    Ok(archive)
}

/// Exclusively create the same sparse cache disk used by the HVF witness.
#[cfg(target_os = "linux")]
pub fn create_disk(path: &Path) -> Result<()> {
    use std::{
        fs::{File, OpenOptions},
        os::unix::fs::OpenOptionsExt,
    };
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.set_len(DISK_BYTES)?;
    mvm_fs::ext4::mkfs::format_empty_ext4_labeled(&mut file, DISK_BYTES, b"spiral-cache")?;
    file.sync_all()?;
    File::open(
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?
    .sync_all()?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn worker(kernel: &Path, disk: Option<&Path>, mailbox_socket: Option<&Path>) -> Result<Value> {
    worker_with_profile(kernel, disk, mailbox_socket, Default::default())
}

/// Use MVM's existing NIC-less Firecracker driver and the canonical typed service.
#[cfg(target_os = "linux")]
pub fn worker_with_profile(
    kernel: &Path,
    disk: Option<&Path>,
    mailbox_socket: Option<&Path>,
    profile: crate::config::MailboxBandwidthProfile,
) -> Result<Value> {
    linux::worker(kernel, disk, mailbox_socket, profile)
}

/// Recover only this witness worker's owned VM after a forced worker exit.
/// The parent must invoke this in a fresh process with that worker's private
/// MVM_HOME; it must never supply another workload's state directory.
#[cfg(target_os = "linux")]
pub fn cleanup_worker(worker_pid: u32) -> Result<()> {
    use mvm_vmm::driver::VmmDriver;
    let id = mvm_core::vm_backend::VmId(format!("spiralcore-proof-{worker_pid}"));
    let state = mvm_core::config::vm_state_dir(&id.0);
    if !state.exists() {
        return Ok(());
    }
    let vm = mvm_backends::driver::fc::FcDriver::new().attach(&id)?;
    vm.kill()?;
    Ok(())
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use cpu_time::ProcessTime;
    use mvm_backends::driver::fc::FcDriver;
    use mvm_net::channel::GuestService;
    use mvm_vmm::driver::{
        RunningVm, VmmDriver,
        prepared_disks::PreparedWritableDisks,
        spec::{BlockDev, ConsoleCapture, KernelImage, VmmSpec, VsockDirection, VsockPort},
    };
    use serde_json::json;
    use std::{
        fs::{File, OpenOptions},
        io::{Read, Write},
        os::{
            fd::{AsRawFd, FromRawFd, OwnedFd},
            unix::{
                ffi::OsStrExt,
                fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
            },
        },
        time::{Duration, Instant},
    };

    struct OwnedVm(Box<dyn RunningVm>);
    impl Drop for OwnedVm {
        fn drop(&mut self) {
            let _ = self.0.kill();
        }
    }

    /// Transfer the already validated backing descriptor and its canonical
    /// cooperating-writer sidecar to the driver. Binding verifies exact inode
    /// identity and complete coverage of the final persistent writable set.
    fn prepare_disk_custody(
        spec: &VmmSpec,
        backing: Option<&File>,
    ) -> Result<PreparedWritableDisks> {
        use mvm_core::util::writable_image::{LockedWritableImage, sidecar_lock_path};
        let writable = spec
            .blocks
            .iter()
            .filter(|block| !block.read_only && !block.ephemeral)
            .collect::<Vec<_>>();
        if writable.is_empty() {
            return Ok(PreparedWritableDisks::bind(spec, Vec::new())?);
        }
        let [block] = writable.as_slice() else {
            return Err(invalid("witness requires at most one persistent writable disk").into());
        };
        if block.slot != 0 || spec.blocks.len() != 1 {
            return Err(invalid("witness writable disk must occupy physical slot zero").into());
        }
        let backing =
            backing.ok_or_else(|| invalid("validated writable backing descriptor missing"))?;
        let sidecar = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(sidecar_lock_path(&block.source))?;
        let locked = LockedWritableImage::acquire(backing.try_clone()?, sidecar)?;
        Ok(PreparedWritableDisks::bind(spec, vec![(0, locked)])?)
    }

    fn micros(value: Duration) -> Result<u64> {
        Ok(value.as_micros().try_into()?)
    }
    fn read_console(path: &Path) -> Result<Vec<u8>> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        let meta = file.metadata()?;
        if !meta.is_file() || meta.len() > MAX_CONSOLE as u64 {
            return Err(invalid("invalid console file or size").into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_CONSOLE as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > MAX_CONSOLE {
            return Err(invalid("console exceeds limit").into());
        }
        Ok(bytes)
    }
    fn observed_ready(bytes: &[u8]) -> bool {
        bytes
            .split(|b| *b == b'\n')
            .filter(|line| line.starts_with(b"{"))
            .any(|line| {
                serde_json::from_slice::<Value>(line).is_ok_and(|e| e["event"] == "guest_ready")
            })
    }
    fn proc_cpu_us(pid: u32) -> Option<u64> {
        let raw = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let fields: Vec<_> = raw.rsplit_once(')')?.1.split_whitespace().collect();
        let ticks = fields
            .get(11)?
            .parse::<u64>()
            .ok()?
            .checked_add(fields.get(12)?.parse().ok()?)?;
        // SAFETY: sysconf has no pointer parameters and this queries a fixed constant.
        let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
        if hz <= 0 {
            return None;
        }
        ticks.checked_mul(1_000_000)?.checked_div(hz as u64)
    }
    pub(super) fn worker(
        kernel: &Path,
        disk: Option<&Path>,
        socket: Option<&Path>,
        profile: crate::config::MailboxBandwidthProfile,
    ) -> Result<Value> {
        let wall = Instant::now();
        let cpu = ProcessTime::try_now()?;
        let home = std::env::var_os("MVM_HOME")
            .ok_or("Linux worker requires an isolated private MVM_HOME")?;
        let home = Path::new(&home);
        let metadata = std::fs::symlink_metadata(home)?;
        // SAFETY: geteuid has no arguments and only reads this process's identity.
        let current_uid = unsafe { libc::geteuid() };
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || metadata.mode() & 0o077 != 0
            || metadata.uid() != current_uid
        {
            return Err(invalid("MVM_HOME must be an owned private directory").into());
        }
        let kernel_file = File::open(kernel)?;
        let kernel_sha256 =
            mvm_core::crypto::verified_digest::sha256_open_file_verified(&kernel_file)?;
        let mut initrd = Vec::new();
        std::io::stdin()
            .lock()
            .take(MAX_INITRAMFS as u64 + 1)
            .read_to_end(&mut initrd)?;
        if initrd.is_empty() || initrd.len() > MAX_INITRAMFS {
            return Err(invalid("bounded initramfs input required").into());
        }
        // SAFETY: constant name and flags create a new anonymous file owned below.
        let fd = unsafe {
            libc::memfd_create(
                c"spiralcore-initramfs".as_ptr(),
                libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error().into());
        }
        // SAFETY: successful memfd_create returned a fresh owned descriptor.
        let mut initrd_file = unsafe { File::from_raw_fd(fd) };
        initrd_file.write_all(&initrd)?;
        drop(initrd);
        // SAFETY: fcntl operates on our live memfd with fixed seal flags.
        if unsafe {
            libc::fcntl(
                initrd_file.as_raw_fd(),
                libc::F_ADD_SEALS,
                libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL,
            )
        } < 0
        {
            return Err(io::Error::last_os_error().into());
        }
        let name = format!("spiralcore-proof-{}", std::process::id());
        let state = mvm_core::config::vm_state_dir(&name);
        std::fs::create_dir_all(&state)?;
        std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700))?;
        let console = state.join("console.log");
        // Watch the directory before launch: the driver recreates console.log.
        // SAFETY: creates an owned, nonblocking inotify descriptor.
        let watch = unsafe { libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK) };
        if watch < 0 {
            return Err(io::Error::last_os_error().into());
        }
        // SAFETY: successful inotify_init1 transfers a new descriptor.
        let watch = unsafe { OwnedFd::from_raw_fd(watch) };
        let state_c = std::ffi::CString::new(state.as_os_str().as_bytes())?;
        // SAFETY: NUL-terminated live path and fixed event mask.
        if unsafe {
            libc::inotify_add_watch(
                watch.as_raw_fd(),
                state_c.as_ptr(),
                libc::IN_MODIFY | libc::IN_CREATE | libc::IN_CLOSE_WRITE,
            )
        } < 0
        {
            return Err(io::Error::last_os_error().into());
        }
        let backing = disk
            .map(|p| {
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open(p)
            })
            .transpose()?;
        if let Some(file) = &backing
            && (!file.metadata()?.is_file() || file.metadata()?.len() != DISK_BYTES)
        {
            return Err(invalid("invalid 256 MiB disk").into());
        }
        let spec = VmmSpec {
            name,
            kernel: KernelImage::Path(
                format!(
                    "/proc/{}/fd/{}",
                    std::process::id(),
                    kernel_file.as_raw_fd()
                )
                .into(),
            ),
            initramfs: Some(
                format!(
                    "/proc/{}/fd/{}",
                    std::process::id(),
                    initrd_file.as_raw_fd()
                )
                .into(),
            ),
            cmdline: format!(
                "{} rdinit=/init",
                FcDriver::new().workload_base_bootargs(false)
            ),
            vcpus: 1,
            cpu_grant: None,
            memory_mib: 1024,
            mem_initial_mib: None,
            blocks: disk
                .map(|p| BlockDev {
                    source: p.to_path_buf(),
                    read_only: socket.is_none(),
                    ephemeral: false,
                    slot: 0,
                })
                .into_iter()
                .collect(),
            // The exit service is required even for the keyless verification
            // boot, which has no mailbox socket but must report its outcome.
            // MVM's Firecracker driver owns this capture and treats host_uds
            // as its exit-record path rather than a runner-owned listener.
            vsock: std::iter::once(VsockPort {
                service: GuestService::WorkloadExit,
                host_uds: mvm_core::exit_capture::exit_file_path(&state),
                direction: VsockDirection::GuestDials,
            })
            .chain(socket.map(|p| VsockPort {
                service: GuestService::Mailbox,
                host_uds: p.to_path_buf(),
                direction: VsockDirection::GuestDials,
            }))
            .collect(),
            console: ConsoleCapture {
                log_path: console.clone(),
            },
            shares: vec![],
            trusted_builder: false,
            builder_egress_endpoint: None,
            plan_binding: None,
        };
        let custody = prepare_disk_custody(&spec, backing.as_ref())?;
        let boot_at = Instant::now();
        let vm = OwnedVm(
            FcDriver::new()
                .boot_custodied(&spec, None, custody)
                .map_err(|e| io::Error::other(format!("MVM Firecracker boot failed: {e}")))?,
        );
        let boot_wall_us = micros(boot_at.elapsed())?;
        let pid =
            vm.0.host_process_id()
                .ok_or("Firecracker process identity missing")?;
        // SAFETY: pidfd_open has only scalar parameters and does not signal the process.
        let exitfd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) } as i32;
        let exitfd = if exitfd >= 0 {
            // SAFETY: successful pidfd_open returned a fresh owned descriptor.
            Some(unsafe { OwnedFd::from_raw_fd(exitfd) })
        } else if io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
            None
        } else {
            return Err(io::Error::last_os_error().into());
        };
        let deadline = Instant::now() + Duration::from_secs(120);
        let mut ready = None;
        let mut vmm_cpu = None;
        loop {
            let bytes = read_console(&console)?;
            if ready.is_none() && observed_ready(&bytes) {
                ready = Some(crate::config::now_ms()?);
            }
            if let Some(sample) = proc_cpu_us(pid) {
                vmm_cpu = Some(sample);
            }
            let exit_code = mvm_core::exit_capture::read_captured(&state);
            if exit_code.is_some() {
                require_exit_code(exit_code)?;
                break;
            }
            let Some(ref exitfd) = exitfd else {
                return Err("VMM exited without a canonical workload exit report".into());
            };
            let mut poll = [
                libc::pollfd {
                    fd: watch.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: exitfd.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or("Firecracker completion deadline")?;
            // SAFETY: two live pollfd entries and bounded millisecond timeout.
            let count = unsafe {
                libc::poll(
                    poll.as_mut_ptr(),
                    2,
                    remaining.as_millis().min(i32::MAX as u128) as i32,
                )
            };
            if count < 0 {
                if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(io::Error::last_os_error().into());
            }
            if count == 0 {
                return Err("Firecracker completion deadline".into());
            }
            if poll[1].revents & libc::POLLIN != 0 {
                require_exit_code(mvm_core::exit_capture::read_captured(&state))?;
                break;
            }
            let mut notifications = [0_u8; 4096];
            // SAFETY: fixed writable buffer and owned nonblocking descriptor.
            while unsafe {
                libc::read(
                    watch.as_raw_fd(),
                    notifications.as_mut_ptr().cast(),
                    notifications.len(),
                )
            } > 0
            {}
        }
        let bytes = read_console(&console)?;
        let ready_host = ready.ok_or("missing observed guest_ready arrival")?;
        let completion = parse_console(&bytes, ready_host)?;
        vm.0.kill()
            .map_err(|e| io::Error::other(format!("Firecracker cleanup failed: {e}")))?;
        if let Some(ref exitfd) = exitfd {
            let mut poll = libc::pollfd {
                fd: exitfd.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: a single live pidfd and a bounded teardown timeout.
            if unsafe { libc::poll(&mut poll, 1, 10_000) } <= 0 || poll.revents & libc::POLLIN == 0
            {
                return Err(
                    "owned Firecracker process exit was not observed after termination".into(),
                );
            }
        }
        let sync_at = Instant::now();
        if let Some(file) = &backing {
            file.sync_all()?;
        }
        let sync_us = backing
            .as_ref()
            .map(|_| micros(sync_at.elapsed()))
            .transpose()?;
        Ok(
            json!({"event":"worker_done","ok":true,"backend":"firecracker","guest_events":completion.events,"console":String::from_utf8(bytes)?,"kernel_sha256":kernel_sha256,"worker_process_cpu_us":micros(cpu.try_elapsed()?)?,"worker_wall_us":micros(wall.elapsed())?,"boot_wall_us":boot_wall_us,"backing_disk_sync_us":sync_us,"backing_disk_synced":backing.is_some(),"guest_ready_host_unix_ms":ready_host,"guest_ready_guest_unix_ms":completion.guest_ready_unix_ms,"guest_clock_offset_ms":completion.clock_offset_ms,"firecracker_observed_cpu_us":vmm_cpu,"firecracker_cpu_scope":"last observed /proc stat sample before owned VMM termination; excludes stop work; not added to guest CPU","mailbox_bandwidth_profile":profile,"mailbox_rate_enforced":false,"mailbox_rate_bytes_per_second":null,"mailbox_burst_bytes":null,"requested_mailbox_rate_bytes_per_second":profile.bytes_per_second(),"requested_mailbox_burst_bytes":profile.burst_bytes(),"stopped_by_watchdog":false,"workload_exit_code":0,"process_exit_observed":true,"terminal_contract":"canonical-workload-exit-zero-and-successful-guest-done-then-owned-host-termination","natural_poweroff_observed":false}),
        )
    }
    #[cfg(test)]
    mod custody_tests {
        use super::*;
        fn spec(path: Option<&Path>, read_only: bool) -> VmmSpec {
            VmmSpec {
                name: "custody-unit".into(),
                kernel: KernelImage::Path("/unused".into()),
                initramfs: None,
                cmdline: String::new(),
                vcpus: 1,
                cpu_grant: None,
                memory_mib: 1024,
                mem_initial_mib: None,
                blocks: path
                    .map(|p| BlockDev {
                        source: p.to_path_buf(),
                        read_only,
                        ephemeral: false,
                        slot: 0,
                    })
                    .into_iter()
                    .collect(),
                vsock: vec![],
                console: ConsoleCapture {
                    log_path: "/unused-console".into(),
                },
                shares: vec![],
                trusted_builder: false,
                builder_egress_endpoint: None,
                plan_binding: None,
            }
        }
        fn image(path: &Path) -> File {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)
                .unwrap()
        }
        #[test]
        fn writable_receiver_has_custody_and_sender_and_readonly_have_none() {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("disk");
            let backing = image(&path);
            let custody = prepare_disk_custody(&spec(Some(&path), false), Some(&backing)).unwrap();
            assert_eq!(custody.iter().len(), 1);
            let (slot, held) = custody.iter().next().unwrap();
            assert_eq!(slot, 0);
            assert_eq!(
                held.backing().metadata().unwrap().ino(),
                backing.metadata().unwrap().ino()
            );
            assert!(
                prepare_disk_custody(&spec(None, false), None)
                    .unwrap()
                    .is_empty()
            );
            assert!(
                prepare_disk_custody(&spec(Some(&path), true), Some(&backing))
                    .unwrap()
                    .is_empty()
            );
            assert!(prepare_disk_custody(&spec(Some(&path), false), None).is_err());
        }
        #[test]
        fn custody_excludes_a_competing_writer_without_truncating_the_sidecar() {
            use mvm_core::util::writable_image::{LockedWritableImage, sidecar_lock_path};
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("disk");
            let backing = image(&path);
            std::fs::write(sidecar_lock_path(&path), b"retained lock metadata").unwrap();
            let custody = prepare_disk_custody(&spec(Some(&path), false), Some(&backing)).unwrap();
            let error = LockedWritableImage::open(&path, &sidecar_lock_path(&path)).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
            assert_eq!(
                std::fs::read(sidecar_lock_path(&path)).unwrap(),
                b"retained lock metadata"
            );
            drop(custody);
            drop(backing);
            let _reopened = LockedWritableImage::open(&path, &sidecar_lock_path(&path)).unwrap();
        }
        #[test]
        fn replaced_disk_path_and_sidecar_symlink_are_rejected() {
            use mvm_core::util::writable_image::sidecar_lock_path;
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("disk");
            let backing = image(&path);
            std::fs::rename(&path, temp.path().join("original")).unwrap();
            let _replacement = image(&path);
            assert!(prepare_disk_custody(&spec(Some(&path), false), Some(&backing)).is_err());
            let second = temp.path().join("second");
            let backing = image(&second);
            let target = temp.path().join("unrelated");
            std::fs::write(&target, b"preserve").unwrap();
            std::os::unix::fs::symlink(&target, sidecar_lock_path(&second)).unwrap();
            assert!(prepare_disk_custody(&spec(Some(&second), false), Some(&backing)).is_err());
            assert_eq!(std::fs::read(target).unwrap(), b"preserve");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elf(machine: u16) -> Vec<u8> {
        let mut value = vec![0; 64];
        value[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        value[16..18].copy_from_slice(&2_u16.to_le_bytes());
        value[18..20].copy_from_slice(&machine.to_le_bytes());
        value[20..24].copy_from_slice(&1_u32.to_le_bytes());
        value[52..54].copy_from_slice(&64_u16.to_le_bytes());
        value
    }

    #[test]
    fn canonical_exit_must_be_present_and_successful() {
        assert!(require_exit_code(Some(0)).is_ok());
        assert!(require_exit_code(None).is_err());
        assert!(require_exit_code(Some(1)).is_err());
        assert!(require_exit_code(Some(-1)).is_err());
    }

    #[test]
    fn guest_header_matches_host_architecture_and_private_pack_bounds() {
        assert!(validate_guest(&elf(62), 62).is_ok());
        assert!(validate_guest(&elf(183), 183).is_ok());
        assert!(validate_guest(&elf(183), 62).is_err());
        assert!(validate_guest(&elf(62), 183).is_err());
        assert!(validate_guest(b"not elf", 62).is_err());
        let mut malformed = elf(62);
        malformed[5] = 2;
        assert!(validate_guest(&malformed, 62).is_err());
    }

    #[test]
    fn public_completion_requires_one_done_and_valid_clock() {
        let good = b"kernel log\n{\"event\":\"guest_ready\",\"utc_unix_ms\":1000}\n{\"event\":\"guest_done\",\"ok\":true}\n";
        let parsed = parse_console(good, 1000).unwrap();
        assert_eq!(parsed.events.len(), 2);
        assert_eq!(parsed.guest_ready_unix_ms, 1000);
        assert_eq!(parsed.clock_offset_ms, 0);
        assert!(parse_console(good, 7000).is_err());
        for bad in [
            &b"{\"event\":\"guest_done\",\"ok\":true}\n"[..],
            &b"{broken}\n"[..],
            &b"{\"event\":\"guest_ready\",\"utc_unix_ms\":1000}\n{\"event\":\"guest_done\",\"ok\":false}\n"[..],
            &b"{\"event\":\"guest_ready\",\"utc_unix_ms\":1000}\n{\"event\":\"guest_done\",\"ok\":true}\n{\"event\":\"extra\"}\n"[..],
        ] {
            assert!(parse_console(bad, 1000).is_err());
        }
    }
}
