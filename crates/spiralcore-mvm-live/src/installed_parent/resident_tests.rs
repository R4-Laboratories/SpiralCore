use super::*;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    task::Waker,
};

fn prepared_service() -> (tempfile::TempDir, InstalledResident) {
    prepared_service_with_backend(LocalBackend::with_hypervisor("mock"))
}
fn prepared_service_with_backend(backend: LocalBackend) -> (tempfile::TempDir, InstalledResident) {
    let home = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let config = crate::installed_supervisor::InstalledConfig::canonical_native_profile();
    let owner = crate::installed_owner::InstallationOwner::initialize(
        &home.path().join("installation"),
        crate::installed_owner::InstallationKey {
            installation: [66; 32],
            profile: config.profile_hash(),
        },
    )
    .unwrap();
    let supervisor = InstalledSupervisor::start(owner, config).unwrap();
    let service = InstalledResident::prepare(supervisor, backend).unwrap();
    (home, service)
}

#[test]
fn missing_helper_profile_refuses_without_consuming_resident_or_credit() {
    let (_home, mut service) = prepared_service();
    let before = service.usage().unwrap();
    assert!(service.start_owned_host_agent().is_err());
    assert!(!service.closed && !service.quarantined);
    assert_eq!(service.usage().unwrap().usage, before.usage);
    assert_eq!(
        service.usage().unwrap().active_reservations,
        before.active_reservations
    );
    service.try_finish().unwrap();
}

#[test]
fn helper_start_refuses_after_vm_reservation_and_after_admission_close() {
    let (_home, mut service) = prepared_service();
    let id = service
        .begin_launch(request("helper-after-vm-refusal"), "local")
        .unwrap();
    let before = service.usage().unwrap().usage;
    assert!(service.start_owned_host_agent().is_err());
    assert_eq!(service.usage().unwrap().usage, before);
    service.request_stop(id).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    assert_eq!(
        runtime.block_on(service.drive_once()).unwrap().state,
        ResidentStatus::UnusedRetired
    );
    let error = service.start_owned_host_agent().unwrap_err();
    assert!(
        error.to_string().contains("helper must precede"),
        "completed original preparation was forgotten: {error}"
    );
    service.close_admission();
    let before = service.usage().unwrap().usage;
    assert!(service.start_owned_host_agent().is_err());
    assert_eq!(service.usage().unwrap().usage, before);
    service.try_finish().unwrap();
}

#[test]
fn poisoned_helper_state_preserves_whole_resident_without_a_vm() {
    let (_home, mut service) = prepared_service();
    let live = service.live.as_ref().unwrap();
    let backend = Arc::downgrade(&live.backend);
    let pool = live.provider.shared.pool.clone();
    let before = pool.snapshot().unwrap().usage;
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _held = live.provider.shared.host_agent.lock().unwrap();
        panic!("controlled helper-state poison before startup");
    }));
    assert!(service.start_owned_host_agent().is_err());
    assert!(service.quarantined && service.closed);
    assert!(
        service.live.is_some(),
        "unknown startup lost actual resident custody"
    );
    assert!(service.try_finish().is_err());
    drop(service);
    assert!(backend.upgrade().is_some());
    assert_eq!(pool.snapshot().unwrap().usage, before);
    // No helper or guest ran. Only original managed-owner retention is tested.
}

#[test]
#[ignore = "requires protected same-revision canonical helper binaries and a fresh synthetic MVM_HOME"]
fn resident_stops_actual_helper_then_drops_backend_before_final_alias_drain() {
    assert!(std::env::var_os("MVM_HOST_AGENT_PATH").is_some());
    assert!(std::env::var_os("MVM_HOME").is_some());
    let keys = mvm_core::config::mvm_keys_dir();
    assert!(!keys.exists());
    let _identity = mvm_hostd::audit::host_keypair::load_or_init_at(&keys).unwrap();
    let (_home, mut service) = prepared_service_with_backend(LocalBackend::with_hypervisor(
        if cfg!(target_os = "linux") {
            "firecracker"
        } else {
            "hvf"
        },
    ));
    let live = service.live.as_ref().unwrap();
    live.provider
        .configure_host_agent(
            service.supervisor.as_ref().unwrap(),
            "local",
            super::super::host_agent::QualifiedHostAgentProfile::fixture(ManagedClaim {
                // Functional custody fixture; these candidate numbers do not
                // qualify a shipping resource model or activate a native profile.
                host_plaintext_bytes: 40 << 20,
                host_metadata_bytes: 12 << 20,
                host_stack_bytes: if cfg!(target_os = "linux") {
                    32 << 20
                } else {
                    34 << 20
                },
                workers: 2,
                ..Default::default()
            }),
        )
        .unwrap();
    let pool = live.provider.shared.pool.clone();
    let backend = Arc::downgrade(&live.backend);
    service.start_owned_host_agent().unwrap();
    let live = service.live.as_ref().unwrap();
    assert!(Arc::ptr_eq(&backend.upgrade().unwrap(), &live.backend));
    let alias = live.helper.as_ref().unwrap().registrar().unwrap();
    assert!(service.start_owned_host_agent().is_err());
    assert_eq!(pool.snapshot().unwrap().usage.workers, 2);
    service.close_admission();
    assert!(
        service.try_finish().is_err(),
        "external registrar alias still owns its credit"
    );
    assert!(
        service.live.is_none(),
        "positive helper shutdown did not release the backend table"
    );
    assert!(
        backend.upgrade().is_none(),
        "original backend registrar still retained"
    );
    assert_eq!(pool.snapshot().unwrap().usage.workers, 2);
    drop(alias);
    assert_eq!(pool.snapshot().unwrap().usage.workers, 0);
    service.try_finish().unwrap();
}
fn request(name: &str) -> LaunchRequest {
    LaunchRequest::builder(
        mvm_client::LifecycleMode::Transient,
        mvm_client::RootfsSource::LocalPath("/nonexistent/resident-policy-rootfs".into()),
    )
    .name(name)
    .cpus(1)
    .memory_mib(1024)
    .build()
    .unwrap()
}

struct Gate {
    inner: Pin<Box<dyn Future<Output = OperationResult>>>,
    released: Arc<AtomicBool>,
    polls: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
}
impl Future for Gate {
    type Output = OperationResult;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        if !self.released.load(Ordering::SeqCst) {
            return Poll::Pending;
        }
        self.inner.as_mut().poll(cx)
    }
}
impl Drop for Gate {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn canceled_driver_preserves_both_original_operations_and_polls_them_fairly() {
    let (_home, mut service) = prepared_service();
    // This private policy fixture intentionally leaves the global provider
    // absent. Once released, canonical preflight returns genuine unused proof.
    let first = service
        .begin_launch(request("resident-policy-a"), "local")
        .unwrap();
    let second = service
        .begin_launch(request("resident-policy-b"), "local")
        .unwrap();
    service.request_stop(first).unwrap();
    let release = Arc::new(AtomicBool::new(false));
    let drops = Arc::new(AtomicUsize::new(0));
    let polls = [Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0))];
    for (index, slot) in service.live.as_mut().unwrap().slots.iter_mut().enumerate() {
        let op = slot.as_mut().unwrap().operation.as_mut().unwrap();
        let inner = std::mem::replace(
            &mut op.future,
            Box::pin(std::future::pending::<OperationResult>()),
        );
        // Extra wrappers are controlled test instrumentation, not a resource
        // claim for the production future layout.
        op.future = Box::pin(Gate {
            inner,
            released: release.clone(),
            polls: polls[index].clone(),
            drops: drops.clone(),
        });
    }
    let mut cx = Context::from_waker(Waker::noop());
    {
        let mut drive = Box::pin(service.drive_once());
        assert!(drive.as_mut().poll(&mut cx).is_pending());
    }
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(polls[0].load(Ordering::SeqCst), 1);
    assert_eq!(polls[1].load(Ordering::SeqCst), 1);
    assert_eq!(service.usage().unwrap().usage.guests, 2);
    release.store(true, Ordering::SeqCst);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let a = runtime.block_on(service.drive_once()).unwrap();
    assert_eq!(a.state, ResidentStatus::UnusedRetired);
    assert_eq!(a.id, first);
    assert_eq!(service.usage().unwrap().usage.guests, 1);
    let b = runtime.block_on(service.drive_once()).unwrap();
    assert_eq!(b.id, second);
    assert_eq!(b.state, ResidentStatus::UnusedRetired);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
    assert_eq!(service.usage().unwrap().usage.guests, 0);
    service.close_admission();
    service.try_finish().unwrap();
}

#[test]
fn incomplete_drop_retains_entire_original_backend_table_and_guest_owner() {
    let (home, mut service) = prepared_service();
    let pool = service.live.as_ref().unwrap().provider.shared.pool.clone();
    let backend = Arc::downgrade(&service.live.as_ref().unwrap().backend);
    service
        .begin_launch(request("resident-drop-quarantine"), "local")
        .unwrap();
    let before = pool.snapshot().unwrap().usage;
    drop(service);
    assert!(backend.upgrade().is_some());
    assert_eq!(pool.snapshot().unwrap().usage, before);
    assert_eq!(before.guests, 1);
    assert!(matches!(
        crate::installed_owner::InstallationOwner::open(
            &home.path().join("installation"),
            crate::installed_owner::InstallationKey {
                installation: [66; 32],
                profile: crate::installed_supervisor::InstalledConfig::canonical_native_profile()
                    .profile_hash(),
            },
        ),
        Err(crate::installed_owner::OwnerError::Busy)
    ));
    // Deliberate bounded quarantine until this test process exits. No VM ran.
}

struct Panics {
    polls: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
}
impl Future for Panics {
    type Output = OperationResult;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        panic!("controlled canonical operation unwind");
    }
}
impl Drop for Panics {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn caught_operation_unwind_fences_admission_reentry_and_global_finish_without_refund() {
    let (_home, mut service) = prepared_service();
    service
        .begin_launch(request("resident-panic-original"), "local")
        .unwrap();
    let pool = service.live.as_ref().unwrap().provider.shared.pool.clone();
    let backend = Arc::downgrade(&service.live.as_ref().unwrap().backend);
    let polls = Arc::new(AtomicUsize::new(0));
    let drops = Arc::new(AtomicUsize::new(0));
    service.live.as_mut().unwrap().slots[0]
        .as_mut()
        .unwrap()
        .operation
        .as_mut()
        .unwrap()
        .future = Box::pin(Panics {
        polls: polls.clone(),
        drops: drops.clone(),
    });
    let before = pool.snapshot().unwrap().usage;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(service.drive_once())
    }));
    assert!(result.is_err());
    assert!(
        service
            .begin_launch(request("resident-after-panic"), "local")
            .is_err(),
        "caught operation unwind left resident admission open"
    );
    assert!(runtime.block_on(service.drive_once()).is_err());
    assert!(service.try_finish().is_err());
    assert_eq!(polls.load(Ordering::SeqCst), 1);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(pool.snapshot().unwrap().usage, before);
    assert!(backend.upgrade().is_some());
    drop(service);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(pool.snapshot().unwrap().usage, before);
}

#[derive(Debug)]
struct CleanupDropPanic;
impl std::fmt::Display for CleanupDropPanic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("controlled cleanup diagnostic drop")
    }
}
impl std::error::Error for CleanupDropPanic {}
impl Drop for CleanupDropPanic {
    fn drop(&mut self) {
        panic!("controlled completed-slot cleanup unwind");
    }
}

#[test]
fn caught_request_stop_cleanup_unwind_quarantines_the_other_original_slot() {
    let (_home, mut service) = prepared_service();
    let first = service
        .begin_launch(request("resident-stop-unwind"), "local")
        .unwrap();
    let second = service
        .begin_launch(request("resident-still-owned"), "local")
        .unwrap();
    let pool = service.live.as_ref().unwrap().provider.shared.pool.clone();
    let backend = Arc::downgrade(&service.live.as_ref().unwrap().backend);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    // Execute the actual first canonical pure preflight, retain its original
    // sealed unused certificate, and leave the second future entirely unpolled.
    let slot = service.live.as_mut().unwrap().slots[first.slot]
        .as_mut()
        .unwrap();
    let mut operation = slot.operation.take().unwrap();
    let OperationResult::Launch(Err(failure)) = runtime.block_on(operation.future.as_mut()) else {
        panic!("expected real unsupported-backend preflight result");
    };
    let (_, unused) = failure.into_parts();
    slot.state = State::Unused(unused.expect("original unused certificate"));
    slot.error = Some(anyhow::Error::new(CleanupDropPanic));
    let original_second = Arc::downgrade(
        &service.live.as_ref().unwrap().slots[second.slot]
            .as_ref()
            .unwrap()
            .owner
            .grant,
    );
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| service.request_stop(first)));
    assert!(result.is_err());
    assert!(
        service
            .begin_launch(request("resident-after-stop-unwind"), "local")
            .is_err(),
        "caught request_stop cleanup unwind left admission open"
    );
    assert!(runtime.block_on(service.drive_once()).is_err());
    assert!(service.try_finish().is_err());
    // Only the positively unused first grant may release. The actual unpolled
    // second grant and original backend must survive the caught cleanup panic.
    assert_eq!(pool.snapshot().unwrap().usage.guests, 1);
    assert_eq!(
        pool.snapshot().unwrap().usage.guest_memory_bytes,
        1024 * 1024 * 1024
    );
    assert!(backend.upgrade().is_some());
    assert!(original_second.upgrade().is_some());
    drop(service);
    assert!(backend.upgrade().is_some());
    assert!(original_second.upgrade().is_some());
}
