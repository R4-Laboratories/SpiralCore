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

#[test]
fn installed_session_refuses_missing_assets_without_legacy_fallback_or_activation() {
    for stage in [Stage::One, Stage::Two] {
        let fixture = tempfile::tempdir().unwrap();
        fs::set_permissions(fixture.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let config = InstalledConfig::native_profile();
        let key = InstallationKey {
            installation: [0x58; 32],
            profile: config.profile_hash(),
        };
        let installation = fixture.path().join("installation");
        let owner = InstallationOwner::initialize(&installation, key).unwrap();
        let mut controller = InstalledSupervisor::start(owner, config).unwrap();
        let active = fs::read(installation.join("state.bin")).unwrap();
        let run = fixture.path().join("must-not-create-run");
        let kernel = fixture.path().join("missing-kernel");
        let guest = fixture.path().join("missing-guest");
        let plan = Plan {
            stage,
            arm: Arm::Whole,
            size: 4096,
            trial: 0,
            bandwidth_profile: MailboxBandwidthProfile::Standard,
        };
        let error = orchestrator::session_installed(
            &mut controller, &run, &kernel, &guest, plan.clone(),
        )
        .unwrap_err();
        assert_eq!(
            error.downcast_ref::<std::io::Error>().map(std::io::Error::kind),
            Some(std::io::ErrorKind::NotFound),
            "installed entry must perform its own asset preflight: {error}"
        );
        assert!(!run.exists());
        assert_eq!(fs::read(installation.join("state.bin")).unwrap(), active);
        assert_eq!(controller.budget().snapshot().unwrap().usage.links, 0);
        assert!(controller.budget().scope(0).is_err(), "error closes admission");
        assert!(orchestrator::session_installed(
            &mut controller, &run, &kernel, &guest, plan,
        ).is_err());
        assert!(!run.exists());
        controller.try_finish().unwrap();
        drop(controller);
        let owner = InstallationOwner::open(&installation, key).unwrap();
        assert_eq!(owner.snapshot().unwrap().phase, Phase::Clean);
        assert_eq!(owner.snapshot().unwrap().epoch, 1);
    }
}
