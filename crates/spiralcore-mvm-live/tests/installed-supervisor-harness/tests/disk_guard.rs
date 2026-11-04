use installed_supervisor_harness::installed_owner::{InstallationKey, InstallationOwner, Phase};
use std::{
    fs,
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    sync::atomic::{AtomicU64, Ordering},
};

fn fixture() -> (std::path::PathBuf, InstallationOwner) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let base = std::env::temp_dir().join(format!(
        "spiral-disk-guard-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
    let owner = InstallationOwner::initialize(
        &base.join("installation"),
        InstallationKey {
            installation: [96; 32],
            profile: [97; 32],
        },
    )
    .unwrap();
    (base, owner)
}
fn metadata(base: &std::path::Path, name: &str) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(base.join("installation").join(name))
        .unwrap();
    file.write_all(b"incomplete owned endpoint disk metadata\n")
        .unwrap();
    file.sync_all().unwrap();
}
#[test]
fn pending_disk_creation_refuses_begin_even_when_main_inventory_is_absent() {
    let (base, mut owner) = fixture();
    metadata(&base, ".endpoint-disks.intent");
    assert!(owner.begin().is_err());
    assert_eq!(owner.snapshot().unwrap().phase, Phase::Clean);
    drop(owner);
    fs::remove_dir_all(base).unwrap();
}
#[test]
fn unresolved_disk_inventory_cannot_be_retired_as_clean() {
    let (base, mut owner) = fixture();
    let epoch = owner.begin().unwrap();
    metadata(&base, ".endpoint-disks.bin");
    assert!(owner.finish(epoch).is_err());
    assert_eq!(owner.snapshot().unwrap().phase, Phase::Active);
    drop(owner);
    fs::remove_dir_all(base).unwrap();
}
#[test]
fn clean_record_does_not_make_incomplete_disk_publication_already_clean() {
    let (base, mut owner) = fixture();
    metadata(&base, ".endpoint-disks.next");
    assert!(owner.reconcile().is_err());
    assert_eq!(owner.snapshot().unwrap().phase, Phase::Clean);
    drop(owner);
    fs::remove_dir_all(base).unwrap();
}
