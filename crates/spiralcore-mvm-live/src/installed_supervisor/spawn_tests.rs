use super::*;
use crate::installed_owner::{InstallationKey, Phase as OwnerPhase};
use std::{
    fs,
    net::Shutdown,
    os::{
        fd::OwnedFd,
        unix::{fs::DirBuilderExt, net::UnixStream},
    },
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Case {
    controller: InstalledSupervisor,
    root: PathBuf,
    base: PathBuf,
    grant: Option<WorkerGrant>,
    writer: Option<UnixStream>,
}
impl Case {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "spiral-spawn-transaction-{}-{}",
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
            installation: [0x81; 32],
            profile: config.profile_hash(),
        };
        let owner = InstallationOwner::initialize(&root, key).unwrap();
        Self {
            controller: InstalledSupervisor::start(owner, config).unwrap(),
            root,
            base,
            grant: None,
            writer: None,
        }
    }
    fn assert_phase(&self, phase: OwnerPhase) {
        let owner = self.controller.authority.owner.lock().unwrap();
        let snapshot = owner.snapshot().unwrap();
        assert_eq!(snapshot.phase, phase);
        assert_eq!(snapshot.epoch, 1);
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        self.writer.take();
        if let Some(grant) = &self.grant {
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
fn definite_spawn_failure_closes_launch_and_clears_only_its_proven_empty_slot() {
    {
        let mut case = Case::new();
        let missing = case.root.join("nonexistent-worker");
        assert!(!missing.exists());
        let mut command = Command::new(missing);
        assert!(
            case.controller
                .spawn_worker(WorkerRole::Sender, &mut command)
                .is_err()
        );
        assert!(!case.controller.authority.admission_open());
        assert!(
            case.controller
                .reserve_worker(WorkerRole::Receiver)
                .is_err()
        );
        case.controller.try_finish().unwrap();
        case.assert_phase(OwnerPhase::Clean);
    }
    let mut ambiguous = Case::new();
    let reserved = ambiguous
        .controller
        .reserve_worker(WorkerRole::Sender)
        .unwrap();
    let active = fs::read(ambiguous.root.join("state.bin")).unwrap();
    let mut command = Command::new(ambiguous.root.join("nonexistent-worker"));
    assert!(
        ambiguous
            .controller
            .spawn_worker(WorkerRole::Receiver, &mut command)
            .is_err()
    );
    assert!(
        ambiguous.controller.try_finish().is_err(),
        "failed spawn cannot clear a different unproven reservation"
    );
    drop(reserved);
    assert!(
        ambiguous.controller.try_finish().is_err(),
        "grant Drop cannot prove a manual reservation empty"
    );
    assert_eq!(fs::read(ambiguous.root.join("state.bin")).unwrap(), active);
    ambiguous.assert_phase(OwnerPhase::Active);
}

#[test]
fn owned_spawn_retains_live_child_until_eof_and_rejects_another_controllers_grant() {
    let mut case = Case::new();
    let (reader, writer) = UnixStream::pair().unwrap();
    case.writer = Some(writer);
    let mut command = Command::new("/bin/sh");
    command
        .args(["-c", "read -r command; exit 0"])
        .stdin(Stdio::from(OwnedFd::from(reader)))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let grant = case
        .controller
        .spawn_worker(WorkerRole::Sender, &mut command)
        .unwrap();
    case.grant = Some(grant);
    let grant = case.grant.as_ref().unwrap();
    assert!(case.controller.try_reap_worker(grant).unwrap().is_none());
    let mut foreign = Case::new();
    assert!(foreign.controller.try_reap_worker(grant).is_err());
    assert!(foreign.controller.confirm_worker_exit(grant).is_err());
    let active = fs::read(case.root.join("state.bin")).unwrap();
    assert_eq!(active.len(), 176);
    assert!(case.controller.try_finish().is_err());
    assert_eq!(fs::read(case.root.join("state.bin")).unwrap(), active);
    case.writer
        .take()
        .unwrap()
        .shutdown(Shutdown::Write)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = case.controller.try_reap_worker(grant).unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "owned spawn reap watchdog");
        std::thread::yield_now();
    };
    assert!(status.success());
    case.controller.try_finish().unwrap();
    case.assert_phase(OwnerPhase::Clean);
}
