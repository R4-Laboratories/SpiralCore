use installed_supervisor_harness::{
    installed_owner::{InstallationKey, InstallationOwner},
    installed_resources::{ManagedClaim, ManagedResourceError},
    installed_supervisor::{InstalledConfig, InstalledSupervisor, SupervisorError},
};
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "spiral-managed-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&p).unwrap();
        Self(p)
    }
    fn owner(&self) -> (InstalledSupervisor, InstallationKey) {
        let config = InstalledConfig::native_profile();
        let key = InstallationKey {
            installation: [91; 32],
            profile: config.profile_hash(),
        };
        let owner = InstallationOwner::initialize(&self.0.join("installation"), key).unwrap();
        (InstalledSupervisor::start(owner, config).unwrap(), key)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn clones_share_one_host_pool_and_failed_reservation_changes_no_usage() {
    let f = Fixture::new();
    let (mut owner, _) = f.owner();
    let pool = owner.managed_resources().clone();
    let sibling = pool.clone();
    let claim = ManagedClaim {
        host_plaintext_bytes: 200 << 20,
        ..Default::default()
    };
    let a = pool.reserve([1; 32], claim).unwrap();
    let b = sibling.reserve([2; 32], claim).unwrap();
    let before = pool.snapshot().unwrap();
    assert!(matches!(
        sibling.reserve([3; 32], claim),
        Err(ManagedResourceError::Full)
    ));
    let after = pool.snapshot().unwrap();
    assert_eq!(before.usage, after.usage);
    assert_eq!(after.active_reservations, 2);
    assert_eq!(after.usage.host_plaintext_bytes, 400 << 20);
    drop(a);
    drop(b);
    owner.close_admission();
    owner.try_finish().unwrap();
}
#[test]
fn tenant_total_includes_codec_plaintext_and_carrier() {
    let f = Fixture::new();
    let (owner, _) = f.owner();
    let pool = owner.managed_resources();
    let a = pool
        .reserve(
            [1; 32],
            ManagedClaim {
                host_plaintext_bytes: 100 << 20,
                host_codec_bytes: 150 << 20,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(matches!(
        pool.reserve(
            [1; 32],
            ManagedClaim {
                host_carrier_bytes: 10 << 20,
                ..Default::default()
            }
        ),
        Err(ManagedResourceError::Full)
    ));
    let b = pool
        .reserve(
            [2; 32],
            ManagedClaim {
                host_carrier_bytes: 10 << 20,
                ..Default::default()
            },
        )
        .unwrap();
    drop(b);
    drop(a);
}
#[test]
fn guest_envelopes_are_separate_and_never_authorize_a_third_guest() {
    let f = Fixture::new();
    let (owner, _) = f.owner();
    let pool = owner.managed_resources();
    let claim = ManagedClaim {
        guest_memory_bytes: 1 << 30,
        guests: 1,
        ..Default::default()
    };
    let a = pool.reserve([1; 32], claim).unwrap();
    let b = pool.reserve([2; 32], claim).unwrap();
    assert!(matches!(
        pool.reserve([3; 32], claim),
        Err(ManagedResourceError::Full)
    ));
    assert_eq!(pool.snapshot().unwrap().usage.guest_memory_bytes, 2 << 30);
    let host = pool
        .reserve(
            [1; 32],
            ManagedClaim {
                host_codec_bytes: 1 << 20,
                ..Default::default()
            },
        )
        .unwrap();
    drop(host);
    drop(a);
    drop(b);
}
#[test]
fn disk_domains_are_shared_and_refusals_do_not_spend_other_resources() {
    let f = Fixture::new();
    let (owner, _) = f.owner();
    let pool = owner.managed_resources();
    let claim = ManagedClaim {
        cache_disk_bytes: 512 << 20,
        staging_disk_bytes: 256 << 20,
        ..Default::default()
    };
    let a = pool.reserve([1; 32], claim).unwrap();
    let b = pool.reserve([2; 32], claim).unwrap();
    let before = pool.snapshot().unwrap().usage;
    assert!(matches!(
        pool.reserve(
            [3; 32],
            ManagedClaim {
                cache_disk_bytes: 1,
                host_plaintext_bytes: 1,
                ..Default::default()
            }
        ),
        Err(ManagedResourceError::Full)
    ));
    assert_eq!(pool.snapshot().unwrap().usage, before);
    drop(a);
    drop(b);
}
#[test]
fn returned_owned_value_and_lease_clones_hold_one_charge_until_last_drop() {
    let f = Fixture::new();
    let (mut owner, _) = f.owner();
    let pool = owner.managed_resources().clone();
    let lease = pool
        .reserve(
            [1; 32],
            ManagedClaim {
                host_plaintext_bytes: 4096,
                ..Default::default()
            },
        )
        .unwrap();
    let shared = lease.clone();
    let value = lease.attach(vec![7u8; 4096]);
    drop(shared);
    assert_eq!(value.as_ref()[0], 7);
    assert_eq!(pool.snapshot().unwrap().usage.host_plaintext_bytes, 4096);
    owner.close_admission();
    assert!(matches!(owner.try_finish(), Err(SupervisorError::Busy)));
    drop(value);
    assert_eq!(pool.snapshot().unwrap().active_reservations, 0);
    owner.try_finish().unwrap();
}
#[test]
fn close_and_controller_drop_preserve_exclusion_until_last_budget_owner() {
    let f = Fixture::new();
    let (mut owner, key) = f.owner();
    let pool = owner.managed_resources().clone();
    let lease = pool
        .reserve(
            [1; 32],
            ManagedClaim {
                host_stack_bytes: 2 << 20,
                workers: 1,
                ..Default::default()
            },
        )
        .unwrap();
    owner.close_admission();
    assert!(matches!(
        lease.check_current(),
        Err(ManagedResourceError::Fenced)
    ));
    assert!(matches!(
        pool.reserve(
            [1; 32],
            ManagedClaim {
                host_plaintext_bytes: 1,
                ..Default::default()
            }
        ),
        Err(ManagedResourceError::Fenced)
    ));
    drop(owner);
    assert!(InstallationOwner::open(&f.0.join("installation"), key).is_err());
    drop(lease);
    assert!(InstallationOwner::open(&f.0.join("installation"), key).is_err());
    drop(pool);
    let mut reopened = InstallationOwner::open(&f.0.join("installation"), key).unwrap();
    reopened.reconcile().unwrap();
}

#[test]
fn malformed_claims_do_not_mutate_or_fence_the_live_pool() {
    let f = Fixture::new();
    let (owner, _) = f.owner();
    let pool = owner.managed_resources();
    for claim in [
        ManagedClaim::default(),
        ManagedClaim {
            guest_memory_bytes: 2 << 30,
            guests: 1,
            ..Default::default()
        },
        ManagedClaim {
            guest_memory_bytes: 1 << 30,
            ..Default::default()
        },
        ManagedClaim {
            guests: 1,
            ..Default::default()
        },
        ManagedClaim {
            host_plaintext_bytes: u64::MAX,
            host_codec_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(pool.reserve([1; 32], claim).is_err());
        assert_eq!(pool.snapshot().unwrap().active_reservations, 0);
        assert!(!pool.snapshot().unwrap().fenced);
    }
}

#[test]
fn values_drop_before_their_actual_resource_charge() {
    struct ValueDrop {
        pool: installed_supervisor_harness::installed_resources::ManagedResourceBudget,
    }
    impl Drop for ValueDrop {
        fn drop(&mut self) {
            assert_eq!(self.pool.snapshot().unwrap().usage.host_plaintext_bytes, 1);
        }
    }
    let f = Fixture::new();
    let (owner, _) = f.owner();
    let pool = owner.managed_resources();
    let lease = pool
        .reserve(
            [1; 32],
            ManagedClaim {
                host_plaintext_bytes: 1,
                ..Default::default()
            },
        )
        .unwrap();
    let value = lease.attach(ValueDrop { pool: pool.clone() });
    drop(value);
    assert_eq!(pool.snapshot().unwrap().active_reservations, 0);
}

#[test]
fn bounded_active_slots_and_stable_tenant_tags_never_grant_more_capacity() {
    let f = Fixture::new();
    let (owner, _) = f.owner();
    let pool = owner.managed_resources();
    let claim = ManagedClaim {
        host_metadata_bytes: 1,
        ..Default::default()
    };
    let leases: Vec<_> = (0..128)
        .map(|_| pool.reserve([1; 32], claim).unwrap())
        .collect();
    assert!(matches!(
        pool.reserve([1; 32], claim),
        Err(ManagedResourceError::Full)
    ));
    drop(leases);
    for tag in 2..=32 {
        drop(pool.reserve([tag; 32], claim).unwrap());
    }
    assert_eq!(pool.snapshot().unwrap().registered_tenants, 32);
    assert!(matches!(
        pool.reserve([33; 32], claim),
        Err(ManagedResourceError::Full)
    ));
    drop(pool.reserve([1; 32], claim).unwrap());
}

#[test]
fn concurrent_reservations_cannot_overspend_the_same_parent_pool() {
    let f = Fixture::new();
    let (owner, _) = f.owner();
    let pool = owner.managed_resources();
    let release = std::sync::Barrier::new(9);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        for tag in 1..=8 {
            let tx = tx.clone();
            let release = &release;
            scope.spawn(move || {
                let lease = pool.reserve(
                    [tag; 32],
                    ManagedClaim {
                        host_codec_bytes: 100 << 20,
                        ..Default::default()
                    },
                );
                tx.send(lease.is_ok()).unwrap();
                release.wait();
                drop(lease);
            });
        }
        let successes = (0..8).filter(|_| rx.recv().unwrap()).count();
        let snapshot = pool.snapshot().unwrap();
        assert_eq!(successes, 5);
        assert_eq!(snapshot.active_reservations, 5);
        assert_eq!(snapshot.usage.host_codec_bytes, 500 << 20);
        release.wait();
    });
    assert_eq!(pool.snapshot().unwrap().usage, ManagedClaim::default());
}

#[test]
fn current_lease_validation_closes_all_siblings_after_epoch_storage_changes() {
    let f = Fixture::new();
    let (owner, _) = f.owner();
    let pool = owner.managed_resources();
    let lease = pool
        .reserve(
            [1; 32],
            ManagedClaim {
                host_plaintext_bytes: 1,
                ..Default::default()
            },
        )
        .unwrap();
    fs::write(f.0.join("installation/state.bin"), b"invalid owned fixture").unwrap();
    assert!(matches!(
        lease.check_current(),
        Err(ManagedResourceError::Fenced)
    ));
    assert!(pool.snapshot().unwrap().fenced);
    assert_eq!(pool.snapshot().unwrap().usage.host_plaintext_bytes, 1);
    drop(lease);
    assert_eq!(pool.snapshot().unwrap().active_reservations, 0);
}
