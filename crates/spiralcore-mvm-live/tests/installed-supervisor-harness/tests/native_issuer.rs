use installed_supervisor_harness::{
    installed_owner::{InstallationKey, InstallationOwner},
    installed_supervisor::{InstalledConfig, InstalledSupervisor, SupervisorError},
};
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    sync::atomic::{AtomicU64, Ordering},
};

#[test]
fn original_native_issuer_rechecks_close_and_retains_only_its_real_owner() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let base = std::env::temp_dir().join(format!(
        "spiral-native-issuer-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
    let config = InstalledConfig::native_profile();
    let key = InstallationKey {
        installation: [93; 32],
        profile: config.profile_hash(),
    };
    let root = base.join("installation");
    let owner = InstallationOwner::initialize(&root, key).unwrap();
    let mut controller = InstalledSupervisor::start(owner, config).unwrap();
    let issuer = controller.native_issuer();
    let (binding, handoff) = issuer.prepare().unwrap();
    assert_eq!(binding.root, root);
    assert_eq!(binding.key, key);
    assert_eq!(binding.generation, 1);
    controller.close_admission();
    assert!(matches!(issuer.prepare(), Err(SupervisorError::State)));
    let held_finish = controller.try_finish();
    assert!(matches!(held_finish, Err(SupervisorError::Busy)), "{held_finish:?}");
    drop(handoff);
    controller.try_finish().unwrap();
    drop(controller);
    assert!(InstallationOwner::open(&root, key).is_err());
    assert!(matches!(issuer.prepare(), Err(SupervisorError::State)));
    drop(issuer);
    let reopened = InstallationOwner::open(&root, key).unwrap();
    assert_eq!(reopened.snapshot().unwrap().epoch, 1);
    drop(reopened);
    fs::remove_dir_all(base).unwrap();
}
