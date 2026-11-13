use super::*;
use crate::installed_owner::{HolderLease, InstallationKey};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
    process::{Child, ChildStdin, Command, ExitStatus, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Case {
    controller: InstalledSupervisor,
    grants: Vec<WorkerGrant>,
    inputs: Vec<Option<ChildStdin>>,
    root: PathBuf,
    base: PathBuf,
    key: InstallationKey,
}
impl Case {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "spiral-worker-ledger-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let root = base.join("installation");
        let config = InstalledConfig::native_profile();
        let key = InstallationKey {
            installation: [0x72; 32],
            profile: config.profile_hash(),
        };
        let owner = InstallationOwner::initialize(&root, key).unwrap();
        Self {
            controller: InstalledSupervisor::start(owner, config).unwrap(),
            grants: Vec::new(),
            inputs: Vec::new(),
            root,
            base,
            key,
        }
    }
    fn reserve(&mut self, role: WorkerRole) -> usize {
        let index = self.grants.len();
        self.grants
            .push(self.controller.reserve_worker(role).unwrap());
        self.inputs.push(None);
        index
    }
    fn bind(&mut self, index: usize, exit: u8) {
        // A pipe keeps this exact owned process live until explicitly closed.
        let mut child = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("read -r command; exit {exit}"))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        assert!(child.try_wait().unwrap().is_none());
        let input = child.stdin.take().unwrap();
        let mut pending = Some(child);
        if let Err(error) = self
            .controller
            .bind_worker(&self.grants[index], &mut pending)
        {
            if let Some(mut child) = pending {
                let _ = child.kill();
                let _ = child.wait();
            }
            panic!("valid reserved Child rejected: {error}");
        }
        if let Some(mut child) = pending {
            let _ = child.kill();
            let _ = child.wait();
            panic!("controller did not take the exact Child");
        }
        self.inputs[index] = Some(input);
    }
    fn reap(&mut self, index: usize) -> ExitStatus {
        self.inputs[index].take();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self
                .controller
                .try_reap_worker(&self.grants[index])
                .unwrap()
            {
                return status;
            }
            assert!(Instant::now() < deadline, "owned Child reap watchdog");
            std::thread::yield_now();
        }
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        self.inputs.clear();
        for grant in &self.grants {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                match self.controller.try_reap_worker(grant) {
                    Ok(None) if Instant::now() < deadline => std::thread::yield_now(),
                    _ => break,
                }
            }
        }
        let _ = fs::remove_dir_all(&self.base);
    }
}

#[test]
fn two_worker_slots_require_reaping_and_confirmation_and_reject_stale_tokens() {
    mvm_core::util::test_env::isolated_lock_probe(probe_two_worker_slots);
}

fn probe_two_worker_slots() {
    {
        let mut abandoned = Case::new();
        let first = abandoned
            .controller
            .reserve_worker(WorkerRole::Sender)
            .unwrap();
        let second = abandoned
            .controller
            .reserve_worker(WorkerRole::Receiver)
            .unwrap();
        drop(first);
        drop(second);
        assert!(
            abandoned
                .controller
                .reserve_worker(WorkerRole::Verify)
                .is_err()
        );
        assert!(
            abandoned.controller.try_finish().is_err(),
            "unbound reservations remain unproven"
        );
    }
    let mut case = Case::new();
    let first = case.reserve(WorkerRole::Sender);
    let second = case.reserve(WorkerRole::Receiver);
    assert!(case.controller.reserve_worker(WorkerRole::Verify).is_err());
    case.bind(first, 0);
    case.bind(second, 0);
    assert!(case.reap(first).success());
    assert!(
        case.controller.reserve_worker(WorkerRole::Verify).is_err(),
        "successful reap alone must not recycle a slot"
    );
    case.controller
        .confirm_worker_exit(&case.grants[first])
        .unwrap();
    let replacement = case.reserve(WorkerRole::Sender);
    case.bind(replacement, 0);
    assert!(case.reap(second).success());
    case.controller
        .confirm_worker_exit(&case.grants[second])
        .unwrap();
    assert!(
        case.controller
            .try_reap_worker(&case.grants[first])
            .is_err(),
        "stale token cannot reap the replacement Child"
    );
    let mut rejected: Option<Child> = Some(
        Command::new("/bin/sh")
            .args(["-c", "exit 0"])
            .spawn()
            .unwrap(),
    );
    let id = rejected.as_ref().unwrap().id();
    let refused = case
        .controller
        .bind_worker(&case.grants[first], &mut rejected)
        .is_err();
    let retained = rejected.as_ref().map(Child::id);
    let status = rejected.take().map(|mut child| child.wait().unwrap());
    assert!(refused);
    assert_eq!(
        retained,
        Some(id),
        "rejected binding must retain exact caller custody"
    );
    assert!(status.unwrap().success());
    assert!(
        case.reap(replacement).success(),
        "new exact Child remains independently reapable"
    );
}

#[test]
fn live_children_block_finish_and_reaped_failure_closes_launch_but_allows_global_retirement() {
    mvm_core::util::test_env::isolated_lock_probe(probe_live_children);
}

fn probe_live_children() {
    {
        let mut live = Case::new();
        let worker = live.reserve(WorkerRole::Sender);
        live.bind(worker, 0);
        assert!(
            live.controller
                .confirm_worker_exit(&live.grants[worker])
                .is_err(),
            "live Child cannot be confirmed"
        );
        let active = fs::read(live.root.join("state.bin")).unwrap();
        assert!(live.controller.try_finish().is_err());
        assert_eq!(fs::read(live.root.join("state.bin")).unwrap(), active);
        assert!(live.reap(worker).success());
        // Reaped success is enough for final exclusion, but does not authorize
        // launch reuse without the separate backend confirmation above.
        live.controller.try_finish().unwrap();
    }
    let mut failed = Case::new();
    let worker = failed.reserve(WorkerRole::Receiver);
    failed.bind(worker, 1);
    assert!(failed.controller.authority.admission_open());
    assert!(!failed.reap(worker).success());
    assert!(!failed.controller.authority.admission_open());
    assert!(
        failed
            .controller
            .reserve_worker(WorkerRole::Verify)
            .is_err()
    );
    assert!(
        failed
            .controller
            .confirm_worker_exit(&failed.grants[worker])
            .is_err()
    );
    let holder = HolderLease::acquire(&failed.root, failed.key, 1).unwrap();
    let active = fs::read(failed.root.join("state.bin")).unwrap();
    assert!(failed.controller.try_finish().is_err());
    assert_eq!(fs::read(failed.root.join("state.bin")).unwrap(), active);
    drop(holder);
    failed.controller.try_finish().unwrap();
}

// This is original-parent fallback reaping, never PID-based signaling. It is
// outside the controller's scope, so controller Drop runs before this fallback
// on unwind. EOF ends the exact shell's only read if product Drop forgot it.
struct DropChildFallback {
    pending: Option<Child>,
    input: Option<ChildStdin>,
    pid: Option<libc::pid_t>,
}
impl DropChildFallback {
    fn settle(&mut self) {
        self.input.take();
        if let Some(mut child) = self.pending.take() {
            let _ = child.kill();
            let _ = child.wait();
            self.pid = None;
        }
        if let Some(pid) = self.pid.take() {
            loop {
                let mut status = 0;
                // SAFETY: this test spawned/transferred this exact Child and
                // is its original parent; waitpid only observes/reaps it.
                let result = unsafe { libc::waitpid(pid, &mut status, 0) };
                if result >= 0
                    || std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR)
                {
                    break;
                }
            }
        }
    }
}
impl Drop for DropChildFallback {
    fn drop(&mut self) {
        self.settle();
    }
}

struct DropRootCleanup(PathBuf);
impl Drop for DropRootCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn controller_drop_reaps_its_exact_live_child_before_releasing_owner_exclusion() {
    mvm_core::util::test_env::isolated_lock_probe(probe_controller_drop);
}

fn probe_controller_drop() {
    let base = std::env::temp_dir().join(format!(
        "spiral-worker-drop-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
    let _root_cleanup = DropRootCleanup(base.clone());
    let root = base.join("installation");
    let config = InstalledConfig::native_profile();
    let key = InstallationKey {
        installation: [0x73; 32],
        profile: config.profile_hash(),
    };
    let mut cleanup = DropChildFallback {
        pending: None,
        input: None,
        pid: None,
    };
    // Outer fallback survives the controller and handles assertion/unwind
    // paths without leaking a live process on the original failing source.
    let (observed_wait, observed_errno) = {
        let owner = InstallationOwner::initialize(&root, key).unwrap();
        let mut controller = InstalledSupervisor::start(owner, config).unwrap();
        let grant = controller.reserve_worker(WorkerRole::Sender).unwrap();
        let mut child = Command::new("/bin/sh")
            .args(["-c", "printf 'READY\\n'; read -r command; exit 0"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        cleanup.input = child.stdin.take();
        let output = child.stdout.take().unwrap();
        cleanup.pid = Some(libc::pid_t::try_from(child.id()).unwrap());
        cleanup.pending = Some(child);
        let mut ready = String::new();
        std::io::BufRead::read_line(&mut std::io::BufReader::new(output), &mut ready).unwrap();
        assert_eq!(ready, "READY\n");
        controller
            .bind_worker(&grant, &mut cleanup.pending)
            .unwrap();
        assert!(cleanup.pending.is_none());
        let pid = cleanup.pid.unwrap();
        drop(grant);
        drop(controller);
        let mut status = 0;
        // SAFETY: the test is the original parent; no controller/grant remains
        // to reap this Child. ECHILD proves product Drop already reaped it.
        let observed_wait = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        let observed_errno = std::io::Error::last_os_error().raw_os_error();
        let successor = InstallationOwner::open(&root, key);
        cleanup.settle(); // Before every post-Drop assertion, including RED.
        let mut successor = successor.unwrap();
        assert_eq!(
            successor.snapshot().unwrap().phase,
            crate::installed_owner::Phase::Active
        );
        assert_eq!(successor.snapshot().unwrap().epoch, 1);
        successor.reconcile().unwrap();
        assert_eq!(
            successor.snapshot().unwrap().phase,
            crate::installed_owner::Phase::Clean
        );
        (observed_wait, observed_errno)
    };
    assert_eq!(
        (observed_wait, observed_errno),
        (-1, Some(libc::ECHILD)),
        "controller Drop released owner EX without positively reaping its exact live Child"
    );
}

#[test]
fn explicitly_provisioned_pair_charges_the_selected_limits_and_refuses_a_foreign_stamp() {
    mvm_core::util::test_env::isolated_lock_probe(probe_explicit_pair_limits);
}

fn probe_explicit_pair_limits() {
    let fallback = InstalledConfig::two_endpoint_witness_profile();
    let fallback_hash = fallback.profile_hash();
    let mut host = fallback.managed_limits().0;
    let mut tenant = fallback.managed_limits().1;
    host.host_resident_bytes = 2 << 30;
    tenant.host_resident_bytes = host.host_resident_bytes;
    let explicit = InstalledConfig::with_funded_pair_limits(host, tenant).unwrap();
    assert_ne!(explicit.profile_hash(), fallback_hash);
    let home = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let parent = home.path().join("operator");
    let key = InstallationKey {
        installation: [32; 32],
        profile: explicit.profile_hash(),
    };
    let owner = InstallationOwner::initialize(&parent, key).unwrap();
    assert!(matches!(
        InstalledSupervisor::start(owner, fallback),
        Err(SupervisorError::Profile)
    ));
    let owner = InstallationOwner::open(&parent, key).unwrap();
    let mut supervisor = InstalledSupervisor::start(owner, explicit).unwrap();
    let claim = crate::installed_resources::ManagedClaim {
        host_metadata_bytes: host.host_resident_bytes,
        ..Default::default()
    };
    let original = supervisor
        .managed_resources()
        .reserve([2; 32], claim)
        .unwrap();
    assert!(
        supervisor
            .managed_resources()
            .reserve(
                [2; 32],
                crate::installed_resources::ManagedClaim {
                    host_metadata_bytes: 1,
                    ..Default::default()
                }
            )
            .is_err()
    );
    drop(original);
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
    tenant.workers = host.workers + 1;
    assert!(InstalledConfig::with_funded_pair_limits(host, tenant).is_err());
    assert_eq!(
        InstalledConfig::two_endpoint_witness_profile().profile_hash(),
        fallback_hash
    );
}
