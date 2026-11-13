//! Linux PID 1 and private launch-input validation for the live witness.

#![cfg(any(target_os = "linux", test))]

use crate::{
    Result,
    config::{Bootstrap, Role, Stage},
};
use mvm_contract::protocol::mailbox::MailboxMessageKind;

const MAX_BOOTSTRAP: usize = 65536;

fn report_exit_to(
    stream: &mut (impl std::io::Read + std::io::Write),
    code: i32,
) -> std::io::Result<()> {
    stream.write_all(&code.to_le_bytes())?;
    stream.flush()?;
    let mut ack = [0];
    stream.read_exact(&mut ack)?;
    if ack != [1] {
        return Err(std::io::Error::other(
            "invalid MVM exit-report acknowledgment",
        ));
    }
    Ok(())
}

fn parse_bootstrap(bytes: &[u8]) -> Result<Bootstrap> {
    if bytes.is_empty() || bytes.len() > MAX_BOOTSTRAP {
        return Err("private bootstrap exceeds its input bound".into());
    }
    let bootstrap: Bootstrap =
        serde_json::from_slice(bytes).map_err(|_| "invalid private bootstrap JSON")?;
    if !(64..=(16 << 20)).contains(&bootstrap.plan.size) || bootstrap.utc_seconds == 0 {
        return Err("invalid guest plan or clock range".into());
    }
    let utc_ms = bootstrap
        .utc_seconds
        .checked_mul(1000)
        .and_then(|time| time.checked_add(5000))
        .and_then(|time| i64::try_from(time).ok())
        .ok_or("guest clock overflows its millisecond range")?
        - 5000;
    if bootstrap.role == Role::Verify {
        if bootstrap.plan.stage != Stage::Two
            || bootstrap.incoming.is_some()
            || bootstrap.outgoing.is_some()
            || bootstrap.service.is_some()
            || bootstrap.published.is_empty()
            || bootstrap.published.len() > 32
            || bootstrap.published.iter().any(|object| {
                object.length == 0
                    || object.length > (16 << 20)
                    || object.kind.is_empty()
                    || object.kind.len() > 64
                    || !object
                        .kind
                        .bytes()
                        .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
                    || object.namespace != bootstrap.published[0].namespace
            })
        {
            return Err("invalid keyless verifier bootstrap".into());
        }
        return Ok(bootstrap);
    }
    if bootstrap.service.is_none() {
        return Err("normal roles require canonical mailbox service provisioning".into());
    }
    if !bootstrap.published.is_empty()
        || (bootstrap.role == Role::Receiver && bootstrap.plan.stage != Stage::Two)
    {
        return Err("mailbox role carries invalid guest state".into());
    }
    let incoming = bootstrap
        .incoming
        .as_ref()
        .ok_or("missing incoming material")?;
    let outgoing = bootstrap
        .outgoing
        .as_ref()
        .ok_or("missing outgoing material")?;
    if incoming.signing.is_some()
        || outgoing.signing.is_none()
        || incoming.encryption == outgoing.encryption
        || incoming.authority != outgoing.authority
    {
        return Err("invalid directional key partition".into());
    }
    let signing = outgoing.signing.as_ref().ok_or("missing outgoing signer")?;
    if ed25519_dalek::SigningKey::from_bytes(signing)
        .verifying_key()
        .to_bytes()
        != outgoing.verification
    {
        return Err("outgoing signer does not match its public identity".into());
    }
    let (call, reply) = if bootstrap.role == Role::Sender {
        (outgoing, incoming)
    } else {
        (incoming, outgoing)
    };
    let forward = &call.capability.grant;
    let reverse = &reply.capability.grant;
    if call.kind != MailboxMessageKind::Call
        || reply.kind != MailboxMessageKind::Reply
        || reply.reply_to.is_some()
        || call.reply_to.is_none_or(|route| {
            route.address != reverse.address || route.capability_id != reverse.capability_id
        })
        || forward.subject != reverse.address.recipient
        || reverse.subject != forward.address.recipient
        || forward.address.tenant != reverse.address.tenant
        || forward.address.authorization_domain != reverse.address.authorization_domain
        || forward.issuer != reverse.issuer
    {
        return Err("mailbox directions do not form the admitted call/reply pair".into());
    }
    for material in [incoming, outgoing] {
        let grant = &material.capability.grant;
        let binding = &material.binding.claims;
        if grant.message_kinds != [material.kind]
            || binding.address != grant.address
            || binding.issuer != grant.issuer
            || binding.policy_epoch != grant.policy_epoch
            || binding.execution_epoch != grant.minimum_execution_epoch
            || utc_ms < grant.not_before_unix_ms
            || utc_ms + 5000 >= grant.expires_at_unix_ms
            || utc_ms < binding.not_before_unix_ms
            || utc_ms + 5000 >= binding.expires_at_unix_ms
        {
            return Err("bootstrap grant or route does not match its launch context".into());
        }
    }
    // These are launch shape checks. The live adapter still authenticates the
    // signatures, routes, and current authority before handling every message.
    crate::campaign::validate_service_bootstrap(&bootstrap)?;
    Ok(bootstrap)
}

#[cfg(target_os = "linux")]
pub fn run() -> Result<()> {
    linux::run()
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use crate::{campaign, config::now_ms};
    use mvm_agentd::{
        guest_mount, restore_clock,
        vsock::{self, VolumeConfig, VolumeConfigKind},
    };
    use serde_json::{Value, json};
    use std::{
        fs::{File, OpenOptions},
        io::{self, Read, Write},
        os::{
            fd::AsRawFd,
            unix::{
                fs::{MetadataExt, OpenOptionsExt},
                net::UnixStream,
            },
        },
        path::Path,
    };

    type PhaseResult<T> = std::result::Result<T, &'static str>;

    fn record(out: &mut impl Write, event: Value) -> PhaseResult<()> {
        serde_json::to_writer(&mut *out, &event).map_err(|_| "console_write")?;
        out.write_all(b"\n").map_err(|_| "console_write")?;
        out.flush().map_err(|_| "console_flush")
    }

    fn bootstrap() -> Result<Bootstrap> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/bootstrap.json")?;
        let metadata = file.metadata()?;
        if !metadata.is_file()
            || metadata.uid() != 0
            || metadata.mode() & 0o777 != 0o600
            || metadata.len() > MAX_BOOTSTRAP as u64
        {
            return Err("private bootstrap file has invalid ownership, mode, or size".into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_BOOTSTRAP as u64 + 1)
            .read_to_end(&mut bytes)?;
        let bootstrap = parse_bootstrap(&bytes)?;
        std::fs::remove_file("/bootstrap.json")?;
        Ok(bootstrap)
    }

    fn perform(
        data_mount_attempted: &mut bool,
        report_exit: &mut bool,
        out: &mut impl Write,
    ) -> PhaseResult<()> {
        guest_mount::mount_early_filesystems().map_err(|_| "mount_early")?;
        let bootstrap = bootstrap().map_err(|_| "bootstrap")?;
        *report_exit = bootstrap.report_exit;
        if bootstrap.role != Role::Verify {
            return Err("signed_sdk_guest_mailbox_required");
        }
        restore_clock::resync(bootstrap.utc_seconds).map_err(|_| "clock_resync")?;
        record(
            out,
            json!({
                "event": "guest_ready", "os": "linux", "pid": std::process::id(),
                "role": bootstrap.role, "utc_unix_ms": now_ms().map_err(|_| "clock_read")?,
            }),
        )?;
        // Flush the ready line before mount writes to the same UART.
        drain_console(out)?;
        *data_mount_attempted = true;
        guest_mount::mount_volumes(
            &[VolumeConfig {
                tag: "spiral-cache".into(),
                mountpoint: "/data".into(),
                read_only: true,
                kind: VolumeConfigKind::Block,
                device: Some("/dev/vda".into()),
                label: Some("spiral-cache".into()),
            }],
            Path::new("/"),
        )
        .map_err(|_| "mount_data")?;
        record(
            out,
            json!({"event":"guest_disk_mounted","read_only":true,"label":"spiral-cache"}),
        )?;
        campaign::verify(
            &bootstrap.plan,
            Path::new("/data/cache"),
            &bootstrap.published,
            out,
        )
        .map_err(|_| "verify_campaign")?;
        Ok(())
    }

    fn finish_data_mount() -> PhaseResult<()> {
        let synced = File::open("/data")
            .map_err(|_| "open_data_for_sync")
            .and_then(|file| {
                // SAFETY: a live owned descriptor names the fixed mounted data filesystem.
                if unsafe { libc::syncfs(file.as_raw_fd()) } == 0 {
                    Ok(())
                } else {
                    Err("sync_data")
                }
            });
        // All campaign/store descriptors and the sync descriptor have dropped.
        // SAFETY: this constant NUL-terminated path is the only owned data mount.
        let unmounted = if unsafe { libc::umount2(c"/data".as_ptr(), 0) } == 0 {
            Ok(())
        } else {
            Err("unmount_data")
        };
        synced.and(unmounted)
    }

    fn drain_console(out: &mut impl Write) -> PhaseResult<()> {
        out.flush().map_err(|_| "console_flush")?;
        io::stderr().flush().map_err(|_| "stderr_flush")?;
        for fd in [libc::STDOUT_FILENO, libc::STDERR_FILENO] {
            // SAFETY: standard descriptors refer to the inherited Linux console.
            if unsafe { libc::tcdrain(fd) } != 0 {
                return Err("console_drain");
            }
        }
        Ok(())
    }

    fn stop(reset: bool) -> Result<()> {
        let command = if reset {
            libc::RB_AUTOBOOT
        } else {
            libc::RB_POWER_OFF
        };
        // SAFETY: PID 1 owns this disposable VM; no pointers cross the syscall.
        let returned = unsafe { libc::reboot(command) };
        if returned != 0 {
            Err(io::Error::last_os_error().into())
        } else {
            Err("guest shutdown unexpectedly returned".into())
        }
    }

    pub(super) fn run() -> Result<()> {
        if std::process::id() != 1 {
            return Err("guest entry requires the owned VM's PID 1".into());
        }
        let mut out = io::stdout().lock();
        let mut data_mount_attempted = false;
        let mut report_exit = false;
        let work = perform(&mut data_mount_attempted, &mut report_exit, &mut out);
        // Drain campaign JSON before sync/unmount can print on the shared UART.
        // A drain failure still permits cleanup, but prevents a success marker.
        let before_cleanup = drain_console(&mut out);
        // Attempt cleanup even after a partial mount or failed campaign.
        let cleanup = if data_mount_attempted {
            finish_data_mount()
        } else {
            Ok(())
        };
        let status = work
            .and(before_cleanup)
            .and(cleanup)
            .and(drain_console(&mut out));
        let exit_code = if status.is_ok() { 0 } else { 1 };
        let done = record(
            &mut out,
            json!({
                "event": "guest_done", "ok": status.is_ok(), "failed_phase": status.err(),
            }),
        );
        if done.is_err() || drain_console(&mut out).is_err() {
            // A partial success record must not be paired with a clean OFF.
            // RESET is explicitly rejected by the host completion gate.
            return stop(true);
        }
        if report_exit {
            let fd = vsock::sys::dial(vsock::HOST_CID, vsock::WORKLOAD_EXIT_PORT)?;
            let mut stream = UnixStream::from(fd);
            stream.set_read_timeout(Some(std::time::Duration::from_secs(3)))?;
            stream.set_write_timeout(Some(std::time::Duration::from_secs(3)))?;
            if report_exit_to(&mut stream, exit_code).is_err() {
                return stop(true);
            }
        }
        stop(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Arm, Pair, Plan, Published, Role, Stage};

    #[test]
    fn exit_report_interoperates_with_mvm_capture_and_requires_its_ack() {
        use std::{
            io::{Read, Write},
            os::unix::net::UnixStream,
        };
        let directory = tempfile::tempdir().unwrap();
        let (mut guest, host) = UnixStream::pair().unwrap();
        let path = directory.path().to_path_buf();
        let capture =
            std::thread::spawn(move || mvm_core::exit_capture::capture_stream(host, &path));
        report_exit_to(&mut guest, 0).unwrap();
        assert_eq!(capture.join().unwrap().unwrap(), 0);
        assert_eq!(
            mvm_core::exit_capture::read_captured(directory.path()),
            Some(0)
        );
        let (mut guest, mut host) = UnixStream::pair().unwrap();
        let wrong_ack = std::thread::spawn(move || {
            let mut code = [0; 4];
            host.read_exact(&mut code).unwrap();
            assert_eq!(i32::from_le_bytes(code), 1);
            host.write_all(&[0]).unwrap();
        });
        assert!(report_exit_to(&mut guest, 1).is_err());
        wrong_ack.join().unwrap();
    }

    fn provision() -> Pair {
        Pair::provision(Plan {
            stage: Stage::Two,
            arm: Arm::Warm,
            size: 4096,
            trial: 0,
            bandwidth_profile: Default::default(),
        })
        .unwrap()
    }

    fn parse(bootstrap: &Bootstrap) -> Result<Bootstrap> {
        parse_bootstrap(&serde_json::to_vec(bootstrap).unwrap())
    }

    fn verifier() -> Bootstrap {
        let sender = provision().sender;
        Bootstrap {
            report_exit: false,
            role: Role::Verify,
            plan: sender.plan,
            utc_seconds: sender.utc_seconds,
            incoming: None,
            outgoing: None,
            service: None,
            published: vec![Published {
                namespace: [1; 32],
                object: [2; 32],
                length: 4096,
                kind: "seed".into(),
            }],
        }
    }

    #[test]
    fn accepts_private_sender_receiver_and_keyless_verifier_inputs() {
        let pair = provision();
        assert_eq!(parse(&pair.sender).unwrap().role, Role::Sender);
        assert_eq!(parse(&pair.receiver).unwrap().role, Role::Receiver);
        assert_eq!(parse(&verifier()).unwrap().role, Role::Verify);
    }

    #[test]
    fn refuses_unbounded_or_malformed_input_without_echoing_private_bytes() {
        for bytes in [
            b"private-secret-that-must-never-be-echoed".to_vec(),
            vec![b' '; 65537],
            b"null".to_vec(),
        ] {
            let error = parse_bootstrap(&bytes).err().expect("invalid bootstrap");
            assert!(!error.to_string().contains("private-secret"));
        }
        let mut bootstrap = provision().sender;
        bootstrap.utc_seconds = 0;
        assert!(parse(&bootstrap).is_err());
        bootstrap.utc_seconds = u64::MAX;
        assert!(parse(&bootstrap).is_err());
        bootstrap.utc_seconds = 1;
        bootstrap.plan.size = (16 << 20) + 1;
        assert!(parse(&bootstrap).is_err());
    }

    #[test]
    fn refuses_missing_outgoing_or_excess_incoming_signing_authority() {
        let mut missing = provision().sender;
        missing.outgoing.as_mut().unwrap().signing = None;
        assert!(parse(&missing).is_err());
        let mut excessive = provision().receiver;
        excessive.incoming.as_mut().unwrap().signing = Some([9; 32]);
        assert!(parse(&excessive).is_err());
        let mut wrong_role = provision().sender;
        wrong_role.role = Role::Receiver;
        assert!(parse(&wrong_role).is_err());
        let mut wrong_pair = provision().sender;
        wrong_pair
            .incoming
            .as_mut()
            .unwrap()
            .capability
            .grant
            .address
            .recipient = mvm_contract::protocol::mailbox::PrincipalId::from_bytes([99; 32]);
        assert!(parse(&wrong_pair).is_err());
    }

    #[test]
    fn canonical_service_refuses_foreign_control_routes_and_excess_private_keys() {
        let mut input = provision().sender;
        input.service.as_mut().unwrap().incoming_control.signing = Some([9; 32]);
        assert!(parse(&input).is_err());
        let mut input = provision().sender;
        input.service.as_mut().unwrap().outgoing_control.binding =
            input.incoming.as_ref().unwrap().binding.clone();
        assert!(parse(&input).is_err());
        let mut input = provision().sender;
        input.service.as_mut().unwrap().profile = "unsupported".into();
        assert!(parse(&input).is_err());
        let mut input = verifier();
        input.service = provision().sender.service;
        assert!(parse(&input).is_err());
    }

    #[test]
    fn current_normal_roles_require_the_explicit_canonical_service_profile() {
        for mut input in [provision().sender, provision().receiver] {
            input.service = None;
            assert!(parse(&input).is_err());
        }
        assert!(parse(&verifier()).is_ok());
    }

    #[test]
    fn launch_shape_validation_does_not_authenticate_against_pre_resync_guest_clock() {
        let mut input = provision().sender;
        input.utc_seconds += 120;
        let profile = input.service.as_mut().unwrap();
        for material in [
            input.incoming.as_mut().unwrap(),
            input.outgoing.as_mut().unwrap(),
            &mut profile.incoming_control,
            &mut profile.outgoing_control,
        ] {
            material.capability.grant.not_before_unix_ms += 120_000;
            material.capability.grant.expires_at_unix_ms += 120_000;
            material.binding.claims.not_before_unix_ms += 120_000;
            material.binding.claims.expires_at_unix_ms += 120_000;
        }
        // Parsing checks launch shape using its supplied UTC. Signatures and
        // current-clock authorization remain canonical admission's job after
        // restore_clock::resync; modified signatures here are intentionally stale.
        assert!(parse(&input).is_ok());
    }

    #[test]
    fn verifier_refuses_mailbox_keys_or_unbounded_readback_requests() {
        let mut with_keys = verifier();
        with_keys.outgoing = provision().sender.outgoing;
        assert!(parse(&with_keys).is_err());
        let mut empty = verifier();
        empty.published.clear();
        assert!(parse(&empty).is_err());
        let mut oversized = verifier();
        oversized.published[0].length = (16 << 20) + 1;
        assert!(parse(&oversized).is_err());
    }
}
