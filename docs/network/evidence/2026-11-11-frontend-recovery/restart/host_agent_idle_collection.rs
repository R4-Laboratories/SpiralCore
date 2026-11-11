//! One isolated Linux fixture process: no VM, other tests, or host children.
//! Fixture reaping is cleanup only; it cannot satisfy original signer custody.
#![cfg(target_os = "linux")]

use std::io::{self, Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use mvm_core::util::test_env::TestEnv;
use mvm_hostd::audit::host_keypair;
use mvm_hostd::broker::daemon::HostAgentConfig;
use mvm_vmm::host::process_exit::{ProcessExitObserver, ProcessExitWait};

const TRACE_LIMIT: usize = 65_536;
const WORKER_GUARD: Duration = Duration::from_secs(20);
const CLEANUP_GUARD: Duration = Duration::from_secs(2);

struct FixtureWorker {
    child: Child,
    group: libc::pid_t,
    kill_sent: bool,
    collected: bool,
}

impl FixtureWorker {
    fn cleanup(&mut self) -> io::Result<(ExitStatus, usize)> {
        if !self.kill_sent {
            // The original worker is still unreaped, reserving its PID/group.
            // This group was created by this fixture's setsid, never discovered.
            // SAFETY: only this owned fixture group is targeted.
            let result = unsafe { libc::kill(-self.group, libc::SIGKILL) };
            if result != 0 && io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
                return Err(io::Error::last_os_error());
            }
            self.kill_sent = true;
        }
        let deadline = Instant::now() + CLEANUP_GUARD;
        let status = loop {
            if let Some(status) = self.child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "original fixture worker not collected after group fence",
                ));
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        self.collected = true;
        let deadline = Instant::now() + CLEANUP_GUARD;
        let mut adopted = 0;
        loop {
            let mut descendant_status = 0;
            // SAFETY: after collecting the original worker, only its adopted
            // descendants in the fixture-owned group can match this wait.
            // This is NOT production signer-owner collection evidence.
            match unsafe { libc::waitpid(-self.group, &mut descendant_status, libc::WNOHANG) } {
                pid if pid > 0 => adopted += 1,
                -1 if io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD) => {
                    return Ok((status, adopted));
                }
                -1 if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted => {}
                -1 => return Err(io::Error::last_os_error()),
                _ if Instant::now() >= deadline => {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "fixture descendants not reaped within cleanup guard",
                    ));
                }
                _ => std::thread::sleep(Duration::from_millis(5)),
            }
        }
    }
}

impl Drop for FixtureWorker {
    fn drop(&mut self) {
        if !self.collected {
            let _ = self.cleanup();
        }
    }
}

fn initialize_fixture_process() {
    // Each integration target is a fixture-owned process.
    // Change only that process, before creating any child; no other lab/process
    // disposition or subreaper state is touched.
    // SAFETY: initialized SIG_DFL action with an empty signal mask.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = libc::SIG_DFL;
        assert_eq!(libc::sigemptyset(&mut action.sa_mask), 0);
        assert_eq!(
            libc::sigaction(libc::SIGCHLD, &action, std::ptr::null_mut()),
            0
        );
        assert_eq!(libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1), 0);
    }
}

type TraceReceiver = mpsc::Receiver<(Vec<u8>, bool, io::Result<()>)>;

fn capture_stderr(child: &mut Child) -> TraceReceiver {
    let mut stderr = child.stderr.take().unwrap();
    let (trace_sender, trace_receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut trace = Vec::with_capacity(TRACE_LIMIT);
        let mut truncated = false;
        let mut buffer = [0; 4096];
        let read = loop {
            match stderr.read(&mut buffer) {
                Ok(0) => break Ok(()),
                Ok(length) => {
                    let keep = length.min(TRACE_LIMIT - trace.len());
                    trace.extend_from_slice(&buffer[..keep]);
                    truncated |= keep != length;
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => break Err(error),
            }
        };
        let _ = trace_sender.send((trace, truncated, read));
    });
    trace_receiver
}

#[test]
fn idle_42_follows_original_signer_collection() {
    initialize_fixture_process();

    let directory = tempfile::tempdir().unwrap();
    let mut environment = TestEnv::new();
    environment.set("MVM_HOME", directory.path());
    let keys = directory.path().join("keys");
    let signer = host_keypair::load_or_init_at(&keys).unwrap();
    let config = HostAgentConfig {
        tenant_id: "cooperative-idle-fixture".into(),
        control_socket: directory.path().join("control.sock"),
        registration_journal_path: Some(directory.path().join("registrations.json")),
        host_signer_public_key_path: signer.public_path,
        signer_helper_uds_path: directory.path().join("signer.sock"),
        software_chain_key_path: signer.secret_path,
        max_frame_bytes: 65_536,
    };
    // Explicit provisioning is legitimate only for this genuinely new private
    // fixture home. Production worker startup remains open-existing only.
    let descriptor = config.resource_descriptor(directory.path()).unwrap();
    mvm_core::config::create_private_dir(descriptor.root.parent().unwrap()).unwrap();
    drop(
        mvm_vmm::host::installed_owner::InstallationOwner::initialize(
            &descriptor.root,
            descriptor.key,
        )
        .unwrap(),
    );
    let mut command = Command::new(env!("CARGO_BIN_EXE_mvm-host-agent"));
    command
        .env("MVM_HOST_AGENT_WORKER", "1")
        .env("MVM_HOST_AGENT_IDLE_TIMEOUT", "1")
        .env(
            "MVM_SIGNER_HELPER_PATH",
            env!("CARGO_BIN_EXE_mvm-signer-helper"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    // SAFETY: setsid is async-signal-safe and creates only this fixture group.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    let child = command.spawn().unwrap();
    let group = child.id().try_into().unwrap();
    let mut worker = FixtureWorker {
        child,
        group,
        kill_sent: false,
        collected: false,
    };
    let observer = ProcessExitObserver::arm(group).unwrap();
    let trace_receiver = capture_stderr(&mut worker.child);
    let raw_config = serde_json::to_vec(&config).unwrap();
    assert!(
        raw_config.len() < 4096,
        "fixture config fits one pipe write"
    );
    worker
        .child
        .stdin
        .take()
        .unwrap()
        .write_all(&raw_config)
        .unwrap();
    let observed = observer.wait_event(Instant::now() + WORKER_GUARD);
    // Always fence/reap this owned group BEFORE reporting the intended RED.
    // On normal idle, the unreaped worker is already a zombie with status42;
    // group cleanup cannot change its actual status or fabricate signer logs.
    let (status, adopted) = worker.cleanup().unwrap();
    let (trace, truncated, read) = trace_receiver.recv_timeout(CLEANUP_GUARD).unwrap();
    read.unwrap();
    let rows: Vec<serde_json::Value> = trace
        .split(|byte| *byte == b'\n')
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect();
    let ready = rows
        .iter()
        .filter(|row| row["fields"]["message"] == "mvm-signer-helper ready")
        .count();
    let original_collections = rows
        .iter()
        .filter(|row| row["fields"]["event"] == "signer_original_child_collected")
        .count();
    // Do not print raw configuration/key paths or turn fixture cleanup into
    // positive production evidence. Only bounded nonsecret scalar facts leave.
    eprintln!(
        "IDLE_COLLECTION_WITNESS worker_code={:?} ready={ready} original_collections={original_collections} fixture_adopted_reaped={adopted} trace_bytes={} trace_truncated={truncated}",
        status.code(),
        trace.len()
    );
    assert_eq!(observed.unwrap(), ProcessExitWait::Exited);
    assert_eq!(status.code(), Some(42), "preserve canonical idle exit code");
    assert!(!truncated, "bounded trace cannot silently lose evidence");
    assert_eq!(ready, 1, "actual signer must reach production readiness");
    assert_eq!(
        original_collections, 1,
        "idle42 must follow actual collection by the original signer owner; fixture reaping is not that proof"
    );
}

#[test]
fn controlled_startup_distinguishes_missing_from_canonical_provisioning() {
    use mvm_vmm::host::host_agent_installation::create_new_home_and_provision;
    use std::os::unix::fs::PermissionsExt;
    initialize_fixture_process();
    for provisioned in [false, true] {
        let parent = tempfile::tempdir().unwrap();
        std::fs::set_permissions(parent.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let home = parent.path().join("mvm-home");
        let tenant = "local";
        let provisioning = if provisioned {
            Some(create_new_home_and_provision(&home, tenant).unwrap())
        } else {
            // Deliberately unprovisioned private input, never labelled Fresh.
            std::fs::create_dir(&home).unwrap();
            std::fs::set_permissions(&home, std::fs::Permissions::from_mode(0o700)).unwrap();
            None
        };
        if let Some(original) = &provisioning {
            original.verify().unwrap();
        }
        let mut environment = TestEnv::new();
        environment.set("MVM_HOME", &home);
        let signer = host_keypair::load_or_init_at(&home.join("keys")).unwrap();
        let config = HostAgentConfig {
            tenant_id: tenant.into(),
            control_socket: mvm_core::config::host_agent_control_socket(tenant),
            registration_journal_path: Some(
                mvm_core::config::host_agent_dir(tenant).join("registrations.json"),
            ),
            host_signer_public_key_path: signer.public_path,
            signer_helper_uds_path: mvm_core::config::host_agent_signer_helper_socket(tenant),
            software_chain_key_path: signer.secret_path,
            max_frame_bytes: 65_536,
        };
        let mut command = Command::new(env!("CARGO_BIN_EXE_mvm-host-agent"));
        command
            .env("MVM_HOST_AGENT_WORKER", "1")
            .env("MVM_HOST_AGENT_IDLE_TIMEOUT", "1")
            .env(
                "MVM_SIGNER_HELPER_PATH",
                env!("CARGO_BIN_EXE_mvm-signer-helper"),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        // SAFETY: same fixture-only async-signal-safe session creation as the
        // original idle witness; it cannot target another process group.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(())
                }
            });
        }
        let child = command.spawn().unwrap();
        let group = child.id().try_into().unwrap();
        let mut worker = FixtureWorker {
            child,
            group,
            kill_sent: false,
            collected: false,
        };
        let observer = ProcessExitObserver::arm(group).unwrap();
        let trace_receiver = capture_stderr(&mut worker.child);
        let raw_config = serde_json::to_vec(&config).unwrap();
        assert!(
            raw_config.len() < 4096,
            "fixture config fits one pipe write"
        );
        worker
            .child
            .stdin
            .take()
            .unwrap()
            .write_all(&raw_config)
            .unwrap();
        let observed = observer.wait_event(Instant::now() + WORKER_GUARD);
        // Always collect/fence original fixture ownership before assertions.
        let (status, adopted) = worker.cleanup().unwrap();
        let (trace, truncated, read) = trace_receiver.recv_timeout(CLEANUP_GUARD).unwrap();
        read.unwrap();
        let rows: Vec<serde_json::Value> = trace
            .split(|byte| *byte == b'\n')
            .filter_map(|line| serde_json::from_slice(line).ok())
            .collect();
        let ready = rows
            .iter()
            .filter(|row| row["fields"]["message"] == "mvm-signer-helper ready")
            .count();
        let collections = rows
            .iter()
            .filter(|row| row["fields"]["event"] == "signer_original_child_collected")
            .count();
        // Rust main's anyhow Result prints a plain `Error:` line before a
        // tracing subscriber can report startup refusal. Preserve actual line
        // order rather than assuming every early failure is tracing JSON.
        let first_error = trace
            .split(|byte| *byte == b'\n')
            .find_map(|line| {
                if let Ok(row) = serde_json::from_slice::<serde_json::Value>(line) {
                    if row["level"] != "ERROR" {
                        return None;
                    }
                    let fields = &row["fields"];
                    Some(format!(
                        "{}: {}",
                        fields["message"].as_str().unwrap_or(""),
                        fields["error"].as_str().unwrap_or("")
                    ))
                } else {
                    let line = String::from_utf8_lossy(line);
                    line.starts_with("Error:").then(|| line.into_owned())
                }
            })
            .map(|message| message.replace(&home.to_string_lossy().to_string(), "<fixture-home>"));
        let first_error_truncated = first_error
            .as_ref()
            .is_some_and(|message| message.chars().count() > 1024);
        let first_error = first_error.map(|message| message.chars().take(1024).collect::<String>());
        // The report carries only bounded scalar facts and a redacted error,
        // never serialized configuration, key bytes or full worker traces.
        let report = serde_json::json!({
            "profile": if provisioned { "canonical_absent_home_provisioned" } else { "explicit_unprovisioned_private_home" },
            "worker_code": status.code(), "ready": ready,
            "original_collections": collections, "fixture_adopted_reaped": adopted,
            "trace_bytes": trace.len(), "trace_truncated": truncated,
            "first_error": first_error, "first_error_truncated": first_error_truncated,
        });
        eprintln!("STARTUP_PRECONDITION_WITNESS {report}");
        assert_eq!(observed.unwrap(), ProcessExitWait::Exited);
        assert!(!truncated);
        assert!(
            !first_error_truncated,
            "earliest diagnostic must be retained completely"
        );
        assert_eq!(
            adopted, 0,
            "no fixture cleanup substituted for original signer collection"
        );
        if provisioned {
            assert_eq!(status.code(), Some(42));
            assert_eq!(ready, 1);
            assert_eq!(collections, 1);
            assert!(
                first_error.is_none(),
                "canonical startup must not conceal an earlier error"
            );
        } else {
            assert_eq!(status.code(), Some(1));
            assert_eq!(ready, 0);
            assert_eq!(collections, 0);
            assert!(
                first_error
                    .as_deref()
                    .is_some_and(|message| message
                        .contains("open existing canonical host-agent provisioning")),
                "negative control must reach the actual missing-owner refusal"
            );
        }
    }
}
