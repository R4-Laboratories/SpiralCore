use super::*;
use crate::installed_owner::{HolderLease, InstallationKey, Phase as OwnerPhase};
use std::{
    fs,
    os::{
        fd::OwnedFd,
        unix::{fs::DirBuilderExt, net::UnixStream, process::ExitStatusExt},
    },
    path::PathBuf,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

struct Running {
    controller: InstalledSupervisor,
    writer: Option<UnixStream>,
    base: PathBuf,
}
impl Drop for Running {
    fn drop(&mut self) {
        self.writer.take();
        let _ = self.controller.close_and_drain_workers();
        let _ = fs::remove_dir_all(&self.base);
    }
}

#[test]
fn close_and_drain_reaps_exact_child_but_retirement_still_requires_holder_exclusion() {
    let base = std::env::temp_dir().join(format!(
        "spiral-worker-drain-{}-{}",
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
        installation: [0x92; 32],
        profile: config.profile_hash(),
    };
    let owner = InstallationOwner::initialize(&root, key).unwrap();
    let controller = InstalledSupervisor::start(owner, config).unwrap();
    let (reader, writer) = UnixStream::pair().unwrap();
    let mut running = Running {
        controller,
        writer: Some(writer),
        base,
    };
    let mut command = Command::new("/bin/sh");
    command
        .args(["-c", "read -r command; exit 0"])
        .stdin(Stdio::from(OwnedFd::from(reader)))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let grant = running
        .controller
        .spawn_worker(WorkerRole::Sender, &mut command)
        .unwrap();
    assert!(
        running
            .controller
            .try_reap_worker(&grant)
            .unwrap()
            .is_none()
    );
    let holder = HolderLease::acquire(&root, key, 1).unwrap();
    let active = fs::read(root.join("state.bin")).unwrap();
    assert_eq!(active.len(), 176);
    running.controller.close_and_drain_workers().unwrap();
    let status = running
        .controller
        .try_reap_worker(&grant)
        .unwrap()
        .expect("drain must retain the positively reaped status");
    assert!(!status.success());
    assert_eq!(status.signal(), Some(libc::SIGKILL));
    assert!(!running.controller.authority.admission_open());
    assert!(
        running
            .controller
            .reserve_worker(WorkerRole::Receiver)
            .is_err()
    );
    assert_eq!(fs::read(root.join("state.bin")).unwrap(), active);
    assert!(running.controller.try_finish().is_err());
    assert_eq!(fs::read(root.join("state.bin")).unwrap(), active);
    drop(holder);
    running.controller.try_finish().unwrap();
    let owner = running.controller.authority.owner.lock().unwrap();
    assert_eq!(owner.snapshot().unwrap().phase, OwnerPhase::Clean);
    assert_eq!(owner.snapshot().unwrap().epoch, 1);
}
