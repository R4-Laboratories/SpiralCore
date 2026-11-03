use super::*;
use crate::installed_owner::{HolderLease, InstallationKey};
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
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
