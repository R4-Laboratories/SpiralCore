//! Linux PID 1 and private launch-input validation for the live witness.

#![cfg(any(target_os = "linux", test))]

use crate::{
    Result,
    config::{Bootstrap, Role, Stage},
};
use mvm_contract::protocol::mailbox::MailboxMessageKind;

const MAX_BOOTSTRAP: usize = 65536;

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
    use spiralcore_mvm_adapter::live::HandshakeRole;
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

    fn perform(data_mount_attempted: &mut bool, out: &mut impl Write) -> PhaseResult<()> {
        guest_mount::mount_early_filesystems().map_err(|_| "mount_early")?;
        let bootstrap = bootstrap().map_err(|_| "bootstrap")?;
        restore_clock::resync(bootstrap.utc_seconds).map_err(|_| "clock_resync")?;
        record(
            out,
            json!({
                "event": "guest_ready", "os": "linux", "pid": std::process::id(),
                "role": bootstrap.role, "utc_unix_ms": now_ms().map_err(|_| "clock_read")?,
            }),
        )?;
        if bootstrap.role != Role::Sender {
            // Rust flush only queues tty bytes. Finish the ready line before
            // mount can emit kernel printk output through the same UART.
            drain_console(out)?;
            *data_mount_attempted = true;
            guest_mount::mount_volumes(
                &[VolumeConfig {
                    tag: "spiral-cache".into(),
                    mountpoint: "/data".into(),
                    read_only: bootstrap.role == Role::Verify,
                    kind: VolumeConfigKind::Block,
                    device: Some("/dev/vda".into()),
                    label: Some("spiral-cache".into()),
                }],
                Path::new("/"),
            )
            .map_err(|_| "mount_data")?;
            record(
                out,
                json!({
                    "event": "guest_disk_mounted", "read_only": bootstrap.role == Role::Verify,
                    "label": "spiral-cache",
                }),
            )?;
        }
        if bootstrap.role == Role::Verify {
            campaign::verify(
                &bootstrap.plan,
                Path::new("/data/cache"),
                &bootstrap.published,
                out,
            )
            .map_err(|_| "verify_campaign")?;
        } else {
            let fd =
                vsock::sys::dial(vsock::HOST_CID, vsock::MAILBOX_PORT).map_err(|_| "vsock_dial")?;
            let socket = UnixStream::from(fd);
            record(
                out,
                json!({
                    "event": "guest_connected", "cid": vsock::HOST_CID,
                    "port": vsock::MAILBOX_PORT,
                }),
            )?;
            match bootstrap.role {
                Role::Sender => {
                    campaign::sender(&bootstrap, socket, HandshakeRole::Initiator, out)
                        .map_err(|_| "sender_campaign")?;
                }
                Role::Receiver => {
                    campaign::receiver(
                        &bootstrap,
                        Path::new("/data/cache"),
                        socket,
                        HandshakeRole::Initiator,
                        out,
                    )
                    .map_err(|_| "receiver_campaign")?;
                }
                Role::Verify => unreachable!(),
            }
        }
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
        let work = perform(&mut data_mount_attempted, &mut out);
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
        stop(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Arm, Pair, Plan, Published, Role, Stage};

    fn provision() -> Pair {
        Pair::provision(Plan {
            stage: Stage::Two,
            arm: Arm::Warm,
            size: 4096,
            trial: 0,
        })
        .unwrap()
    }

    fn parse(bootstrap: &Bootstrap) -> Result<Bootstrap> {
        parse_bootstrap(&serde_json::to_vec(bootstrap).unwrap())
    }

    fn verifier() -> Bootstrap {
        let sender = provision().sender;
        Bootstrap {
            role: Role::Verify,
            plan: sender.plan,
            utc_seconds: sender.utc_seconds,
            incoming: None,
            outgoing: None,
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
