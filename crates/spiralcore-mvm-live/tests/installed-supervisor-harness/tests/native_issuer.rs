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
    assert!(
        matches!(held_finish, Err(SupervisorError::Busy)),
        "{held_finish:?}"
    );
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

#[test]
fn two_endpoint_operator_profile_reserves_22_workers_and_keeps_the_default_at_16() {
    use installed_supervisor_harness::installed_resources::ManagedClaim;
    let base = std::env::temp_dir().join(format!("sc-paired-budget-{}", std::process::id()));
    fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
    let default = InstalledConfig::canonical_native_profile();
    let pair = InstalledConfig::two_endpoint_witness_profile();
    assert_ne!(default.profile_hash(), pair.profile_hash());
    for (name, config, capacity) in [("default", default, 16), ("pair", pair, 22)] {
        let key = InstallationKey {
            installation: [75; 32],
            profile: config.profile_hash(),
        };
        let mut supervisor = InstalledSupervisor::start(
            InstallationOwner::initialize(&base.join(name), key).unwrap(),
            config,
        )
        .unwrap();
        let pool = supervisor.managed_resources();
        assert_eq!(pool.limits().workers, capacity);
        let claim = |workers| ManagedClaim {
            workers,
            ..Default::default()
        };
        let originals = pool.reserve([7; 32], claim(10)).unwrap();
        let a = pool.reserve([7; 32], claim(6)).unwrap();
        let b = pool.reserve([7; 32], claim(6));
        assert_eq!(b.is_ok(), capacity == 22);
        assert_eq!(pool.snapshot().unwrap().usage.workers, capacity);
        assert!(pool.reserve([8; 32], claim(1)).is_err());
        drop((originals, a, b));
        supervisor.close_admission();
        supervisor.try_finish().unwrap();
    }
    fs::remove_dir_all(base).unwrap();
}
