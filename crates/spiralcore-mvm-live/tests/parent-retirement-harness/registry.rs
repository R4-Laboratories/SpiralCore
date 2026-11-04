//! Real installed-provider ownership; this test boots no native VM.
#![cfg(target_os = "linux")]
use mvm_hostd::host_lifetime_parent::{self, ParentAdmissionProvider};
use spiralcore_mvm_live::{
    installed_owner::{InstallationKey, InstallationOwner},
    installed_parent::InstalledParentProvider,
    installed_supervisor::{InstalledConfig, InstalledSupervisor},
};
use std::{fs, os::unix::fs::PermissionsExt, sync::Arc};

#[test]
fn original_provider_scoped_calls_retain_credit_but_registry_cannot_prevent_final_drain() {
    let home = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let config = InstalledConfig::canonical_native_profile();
    let key = InstallationKey {
        installation: [64; 32],
        profile: config.profile_hash(),
    };
    let owner = InstallationOwner::initialize(&home.path().join("installation"), key).unwrap();
    let mut supervisor = InstalledSupervisor::start(owner, config).unwrap();
    assert!(host_lifetime_parent::require_installed().is_err());
    let provider = Arc::new(InstalledParentProvider::new(&supervisor).unwrap());
    let original: Arc<dyn ParentAdmissionProvider> = provider.clone();
    let weak = Arc::downgrade(&provider);
    host_lifetime_parent::install(original.clone()).unwrap();
    let call = host_lifetime_parent::require_installed().unwrap().clone();
    assert!(Arc::ptr_eq(&call, &original));
    assert!(host_lifetime_parent::install(original.clone()).is_err());
    provider.close_admission();
    supervisor.close_admission();
    drop(provider);
    drop(original);
    assert!(weak.upgrade().is_some());
    assert_eq!(
        supervisor
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .host_metadata_bytes,
        4096
    );
    assert!(supervisor.try_finish().is_err());
    drop(call);
    assert!(
        weak.upgrade().is_none(),
        "registry retained the last original provider"
    );
    assert_eq!(
        supervisor
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .host_metadata_bytes,
        0
    );
    assert!(host_lifetime_parent::require_installed().is_err());
    // Install-once remains consumed after expiry; no same-process replacement.
    let replacement_owner = InstallationOwner::initialize(
        &home.path().join("other-installation"),
        InstallationKey {
            installation: [65; 32],
            ..key
        },
    )
    .unwrap();
    let mut replacement_supervisor = InstalledSupervisor::start(
        replacement_owner,
        InstalledConfig::canonical_native_profile(),
    )
    .unwrap();
    let replacement = Arc::new(InstalledParentProvider::new(&replacement_supervisor).unwrap());
    assert!(host_lifetime_parent::install(replacement.clone()).is_err());
    drop(replacement);
    replacement_supervisor.close_admission();
    replacement_supervisor.try_finish().unwrap();
    supervisor.try_finish().unwrap();
}
