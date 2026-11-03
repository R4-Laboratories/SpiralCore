#![cfg(any(
    all(target_os = "linux", feature = "linux-host"),
    all(target_os = "macos", target_arch = "aarch64")
))]

use spiralcore_mvm_live::{
    config::{Arm, MailboxBandwidthProfile, Plan, Stage},
    installed_owner::{InstallationKey, InstallationOwner, OwnerError, Phase, Recovery},
    installed_supervisor::{InstalledConfig, InstalledSupervisor},
    orchestrator,
};
use std::{fs, os::unix::fs::PermissionsExt};

#[test]
fn legacy_session_rejects_installed_budget_before_run_files_or_guest_assets() {
    let fixture = tempfile::tempdir().unwrap();
    fs::set_permissions(fixture.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let config = InstalledConfig::native_profile();
    let key = InstallationKey {
        installation: [0x57; 32],
        profile: config.profile_hash(),
    };
    let installation = fixture.path().join("installation");
    let owner = InstallationOwner::initialize(&installation, key).unwrap();
    let controller = InstalledSupervisor::start(owner, config).unwrap();
    let active = fs::read(installation.join("state.bin")).unwrap();
    assert_eq!(active.len(), 176);
    let budget = controller.budget().clone();
    let scope = budget.scope(0).unwrap();
    let run = fixture.path().join("must-not-create-run");
    let kernel = fixture.path().join("missing-kernel");
    let guest = fixture.path().join("missing-guest");
    assert!(!kernel.exists() && !guest.exists());
    let plan = Plan {
        stage: Stage::Two,
        arm: Arm::Whole,
        size: 4096,
        trial: 0,
        bandwidth_profile: MailboxBandwidthProfile::Standard,
    };
    let error = orchestrator::session_with_budget(&run, &kernel, &guest, plan, &budget, &scope)
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "installed budget requires its installed launch context"
    );
    assert!(!run.exists());
    assert!(!kernel.exists() && !guest.exists());
    assert_eq!(fs::read(installation.join("state.bin")).unwrap(), active);
    let snapshot = budget.snapshot().unwrap();
    assert_eq!(
        (
            snapshot.usage.links,
            snapshot.usage.frame_slots,
            snapshot.usage.serialized_buffer_bytes
        ),
        (0, 0, 0)
    );
    assert_eq!(snapshot.high_water, snapshot.usage);
    assert!(matches!(
        InstallationOwner::open(&installation, key),
        Err(OwnerError::Busy)
    ));
    drop(scope);
    drop(budget);
    drop(controller);
    let mut successor = InstallationOwner::open(&installation, key).unwrap();
    assert_eq!(successor.snapshot().unwrap().phase, Phase::Active);
    assert_eq!(successor.snapshot().unwrap().epoch, 1);
    assert_eq!(
        successor.reconcile().unwrap(),
        Recovery::Retired { epoch: 1 }
    );
    assert_eq!(successor.snapshot().unwrap().phase, Phase::Clean);
}
