//! Real canonical unused-preflight completion, with two original Root grants.
//! This process installs its provider once; it boots no guest and proves no
//! positive native retirement or endpoint cleanup.
#![cfg(target_os = "linux")]

use mvm_client::{LaunchRequest, LifecycleMode, LocalBackend, RootfsSource};
use mvm_hostd::host_lifetime_parent;
use spiralcore_mvm_live::{
    installed_owner::{InstallationKey, InstallationOwner},
    installed_parent::{InstalledResident, ResidentStatus},
    installed_supervisor::{InstalledConfig, InstalledSupervisor},
};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, time::Duration};

fn request(home: &Path, name: &str) -> LaunchRequest {
    LaunchRequest::builder(
        LifecycleMode::Transient,
        RootfsSource::LocalPath(home.join("never-opened-rootfs")),
    )
    .name(name)
    .cpus(1)
    .memory_mib(1024)
    .build()
    .unwrap()
}

#[test]
fn two_original_resident_slots_complete_real_unused_preflight_then_global_drain() {
    let home = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let config = InstalledConfig::canonical_native_profile();
    let owner = InstallationOwner::initialize(
        &home.path().join("installation"),
        InstallationKey {
            installation: [66; 32],
            profile: config.profile_hash(),
        },
    )
    .unwrap();
    let supervisor = InstalledSupervisor::start(owner, config).unwrap();
    assert!(host_lifetime_parent::require_installed().is_err());
    // The actual client rejects this unsupported backend before converting
    // either original native-profile lifetime. No mock native proof is used.
    let mut resident =
        InstalledResident::new(supervisor, LocalBackend::with_hypervisor("mock")).unwrap();
    let first = resident
        .begin_launch(request(home.path(), "resident-unused-first"), "local")
        .unwrap();
    let second = resident
        .begin_launch(request(home.path(), "resident-unused-second"), "local")
        .unwrap();
    assert!(first != second);
    let full = resident.usage().unwrap();
    assert_eq!(full.usage.guests, 2);
    assert_eq!(full.usage.guest_memory_bytes, 2 * 1024 * 1024 * 1024);
    assert!(
        resident
            .begin_launch(request(home.path(), "resident-unused-third"), "local")
            .is_err()
    );
    let refused = resident.usage().unwrap();
    assert_eq!(refused.usage.guests, full.usage.guests);
    assert_eq!(refused.usage.guest_memory_bytes, full.usage.guest_memory_bytes);
    // Stop is only a latch. Both actual canonical futures still run to their
    // sealed unused result, then the SAME ParentLaunchOwner cancels them.
    resident.request_stop(first).unwrap();
    resident.request_stop(second).unwrap();
    assert_eq!(resident.usage().unwrap().usage.guests, 2);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let progress = runtime
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(5), resident.drive_once()).await
        })
        .expect("bounded first pure-preflight completion")
        .unwrap();
    assert!(progress.id == first || progress.id == second);
    assert!(matches!(progress.state, ResidentStatus::UnusedRetired));
    assert_eq!(resident.usage().unwrap().usage.guests, 1);
    assert!(resident.request_stop(progress.id).is_err());
    let completed_first = progress.id;
    let progress = runtime
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(5), resident.drive_once()).await
        })
        .expect("bounded second pure-preflight completion")
        .unwrap();
    assert!(progress.id == first || progress.id == second);
    assert!(progress.id != completed_first);
    assert!(matches!(progress.state, ResidentStatus::UnusedRetired));
    let drained = resident.usage().unwrap();
    assert_eq!(drained.usage.guests, 0);
    assert_eq!(drained.usage.guest_memory_bytes, 0);
    assert!(resident.request_stop(first).is_err());
    assert!(resident.request_stop(second).is_err());
    resident.close_admission();
    assert!(
        resident
            .begin_launch(request(home.path(), "resident-after-close"), "local")
            .is_err()
    );
    // This is the original supervisor's genuine ledger/holder finish, not a
    // local-drain Boolean or an exemption for permanent provider bookkeeping.
    resident.try_finish().unwrap();
    assert!(host_lifetime_parent::require_installed().is_err());
    assert!(!home.path().join("never-opened-rootfs").exists());
}
