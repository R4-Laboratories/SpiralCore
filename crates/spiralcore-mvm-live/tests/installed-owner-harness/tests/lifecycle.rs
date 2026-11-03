#![cfg(unix)]

use installed_owner::{HolderLease, InstallationKey, InstallationOwner, OwnerError, Phase};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, MetadataExt},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

fn key() -> InstallationKey {
    InstallationKey { installation: [0x31; 32], profile: [0x47; 32] }
}
struct Fixture { base: PathBuf, root: PathBuf }
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!("spiral-installed-owner-{}-{}-{}",
            std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let root = base.join("installation");
        Self { base, root }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.base); }
}
fn identity(path: &std::path::Path) -> (u64, u64) {
    let metadata = fs::symlink_metadata(path).unwrap();
    (metadata.dev(), metadata.ino())
}

#[test]
fn exclusive_owner_issues_one_epoch_and_finishes_after_last_holder_exits() {
    let fixture = Fixture::new();
    let mut owner = InstallationOwner::initialize(&fixture.root, key()).unwrap();
    assert!(matches!(owner.snapshot().unwrap().phase, Phase::Clean));
    assert_eq!(owner.snapshot().unwrap().epoch, 0);
    let locks = [identity(&fixture.root.join("owner.lock")), identity(&fixture.root.join("holders.lock"))];
    assert!(matches!(InstallationOwner::open(&fixture.root, key()), Err(OwnerError::Busy)));
    let epoch = owner.begin().unwrap();
    assert_eq!(epoch.generation(), 1);
    assert!(matches!(owner.snapshot().unwrap().phase, Phase::Active));
    let holder = HolderLease::acquire(&fixture.root, key(), epoch.generation()).unwrap();
    drop(holder);
    owner.finish(epoch).unwrap();
    assert!(matches!(owner.snapshot().unwrap().phase, Phase::Clean));
    assert_eq!(owner.snapshot().unwrap().epoch, 1);
    assert_eq!(locks, [identity(&fixture.root.join("owner.lock")), identity(&fixture.root.join("holders.lock"))]);
}
