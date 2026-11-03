#![cfg(unix)]

use installed_supervisor_harness::{
    host_budget::HostTransportBudget,
    installed_owner::{InstallationKey, InstallationOwner, OwnerError, Phase, Recovery},
    installed_supervisor::{InstalledConfig, InstalledSupervisor},
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture {
    base: PathBuf,
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir().join(format!(
            "spiral-supervisor-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        Self {
            root: base.join("installation"),
            base,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}
fn profile() -> (InstalledConfig, InstallationKey) {
    let config = InstalledConfig::native_profile();
    let mut digest = Sha256::new();
    digest.update(b"spiralcore/installed-profile/v1\0");
    for value in [2_u64, 2, 524288, 1, 262144, 2, 1, 1024] {
        digest.update(value.to_be_bytes());
    }
    let expected: [u8; 32] = digest.finalize().into();
    assert_eq!(config.profile_hash(), expected);
    (
        config,
        InstallationKey {
            installation: [0x35; 32],
            profile: expected,
        },
    )
}

#[test]
fn sole_outliving_frame_retains_owner_ex_and_drop_never_publishes_clean() {
    let fixture = Fixture::new();
    let (config, key) = profile();
    let owner = InstallationOwner::initialize(&fixture.root, key).unwrap();
    let controller = InstalledSupervisor::start(owner, config).unwrap();
    let budget: HostTransportBudget = controller.budget().clone();
    let scope = budget.scope(0).unwrap();
    let link = budget.reserve_link(&scope).unwrap();
    let frame = link.reserve_frame(64).unwrap();
    assert_eq!(frame.capacity(), 64);
    drop(scope);
    drop(link);
    drop(budget);
    drop(controller);
    assert!(matches!(
        InstallationOwner::open(&fixture.root, key),
        Err(OwnerError::Busy)
    ));
    drop(frame);
    let mut successor = InstallationOwner::open(&fixture.root, key).unwrap();
    assert_eq!(successor.snapshot().unwrap().phase, Phase::Active);
    assert_eq!(successor.snapshot().unwrap().epoch, 1);
    assert_eq!(
        successor.reconcile().unwrap(),
        Recovery::Retired { epoch: 1 }
    );
    assert_eq!(successor.snapshot().unwrap().phase, Phase::Clean);
}

#[test]
fn close_refuses_cloned_admission_and_finish_preserves_ex_until_last_idle_clone_drops() {
    let fixture = Fixture::new();
    let (config, key) = profile();
    let owner = InstallationOwner::initialize(&fixture.root, key).unwrap();
    let mut controller = InstalledSupervisor::start(owner, config).unwrap();
    let budget: HostTransportBudget = controller.budget().clone();
    let scope = budget.scope(0).unwrap();
    let link = budget.reserve_link(&scope).unwrap();
    let frame = link.reserve_frame(64).unwrap();
    controller.close_admission();
    assert!(budget.scope(0).is_err());
    assert!(budget.reserve_link(&scope).is_err());
    assert!(link.reserve_frame(1).is_err());
    assert!(controller.try_finish().is_err());
    assert!(matches!(
        InstallationOwner::open(&fixture.root, key),
        Err(OwnerError::Busy)
    ));
    drop(frame);
    drop(link);
    drop(scope);
    let usage = budget.snapshot().unwrap().usage;
    assert_eq!(
        (
            usage.links,
            usage.frame_slots,
            usage.serialized_buffer_bytes
        ),
        (0, 0, 0)
    );
    controller.try_finish().unwrap();
    drop(controller);
    assert!(budget.scope(0).is_err());
    assert!(matches!(
        InstallationOwner::open(&fixture.root, key),
        Err(OwnerError::Busy)
    ));
    drop(budget);
    let successor = InstallationOwner::open(&fixture.root, key).unwrap();
    assert_eq!(successor.snapshot().unwrap().phase, Phase::Clean);
    assert_eq!(successor.snapshot().unwrap().epoch, 1);
}
