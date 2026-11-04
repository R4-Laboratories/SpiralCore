#![cfg(all(
    feature = "canonical-host",
    any(
        all(target_os = "linux", feature = "linux-host"),
        all(target_os = "macos", target_arch = "aarch64")
    )
))]
use mvm_client::HostLifetimeIntent;
use mvm_client::LifecycleMode;
use mvm_core::vm_backend::BackendKind;
use spiralcore_mvm_live::{
    installed_owner::{InstallationKey, InstallationOwner},
    installed_parent::InstalledParentProvider,
    installed_supervisor::{InstalledConfig, InstalledSupervisor},
};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    sync::atomic::{AtomicU64, Ordering},
};

#[test]
fn parent_preparation_reserves_one_pool_and_unused_drop_refunds_without_boot() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let base = std::env::temp_dir().join(format!(
        "spiral-parent-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
    let config = InstalledConfig::canonical_native_profile();
    let key = InstallationKey {
        installation: [94; 32],
        profile: config.profile_hash(),
    };
    let owner = InstallationOwner::initialize(&base.join("installation"), key).unwrap();
    let mut controller = InstalledSupervisor::start(owner, config).unwrap();
    let provider = InstalledParentProvider::new(&controller).unwrap();
    let backend = if cfg!(target_os = "linux") {
        BackendKind::Firecracker
    } else {
        BackendKind::Hvf
    };
    let intent = |name: &str| HostLifetimeIntent {
        name: Some(name.into()),
        mode: LifecycleMode::Transient,
        cpus: 1,
        memory_mib: 1024,
        backend: None,
    };
    let a = provider
        .prepare_vm(&intent("parent-one"), "tenant-one", backend)
        .unwrap();
    let b = provider
        .prepare_vm(&intent("parent-two"), "tenant-one", backend)
        .unwrap();
    assert_eq!(
        controller
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        2
    );
    assert_eq!(a.binding().vm_name, "parent-one");
    assert!(
        provider
            .prepare_vm(&intent("parent-three"), "tenant-one", backend)
            .is_err()
    );
    drop(a);
    assert_eq!(
        controller
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        1
    );
    let c = provider
        .prepare_vm(&intent("parent-three"), "tenant-one", backend)
        .unwrap();
    assert_ne!(b.binding().grant_serial, c.binding().grant_serial);
    provider.close_admission();
    assert!(
        provider
            .prepare_vm(&intent("parent-four"), "tenant-one", backend)
            .is_err()
    );
    drop(b);
    drop(c);
    assert_eq!(
        controller
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        0
    );
    drop(provider);
    controller.close_admission();
    controller.try_finish().unwrap();
    drop(controller);
    fs::remove_dir_all(base).unwrap();
}

fn fixture() -> (tempfile::TempDir, InstalledSupervisor) {
    let base = tempfile::Builder::new()
        .prefix("spiral-parent-gate-")
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let config = InstalledConfig::canonical_native_profile();
    let owner = InstallationOwner::initialize(
        &base.path().join("installation"),
        InstallationKey {
            installation: [95; 32],
            profile: config.profile_hash(),
        },
    )
    .unwrap();
    (base, InstalledSupervisor::start(owner, config).unwrap())
}
fn native_backend() -> BackendKind {
    if cfg!(target_os = "linux") {
        BackendKind::Firecracker
    } else {
        BackendKind::Hvf
    }
}
fn intent(name: &str) -> HostLifetimeIntent {
    HostLifetimeIntent {
        name: Some(name.into()),
        mode: LifecycleMode::Transient,
        cpus: 1,
        memory_mib: 1024,
        backend: None,
    }
}

#[test]
fn independent_provider_cannot_reuse_an_original_grant_serial() {
    let (_base, mut controller) = fixture();
    let a = InstalledParentProvider::new(&controller).unwrap();
    let b = InstalledParentProvider::new(&controller).unwrap();
    let first = a
        .prepare_vm(&intent("original"), "tenant", native_backend())
        .unwrap();
    let second = b
        .prepare_vm(&intent("original"), "tenant", native_backend())
        .unwrap();
    assert_ne!(
        first.binding(),
        second.binding(),
        "same projection must not identify a replacement reservation"
    );
    drop((first, second, a, b));
    controller.close_admission();
    controller.try_finish().unwrap();
}

#[test]
fn closed_parent_refuses_conversion_and_keeps_native_unallocated() {
    let (_base, mut controller) = fixture();
    let provider = InstalledParentProvider::new(&controller).unwrap();
    let prepared = provider
        .prepare_vm(&intent("unlaunched"), "tenant", native_backend())
        .unwrap();
    provider.close_admission();
    assert!(prepared.into_lifetime().is_err());
    assert_eq!(
        controller
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        0
    );
    drop(provider);
    controller.close_admission();
    controller.try_finish().unwrap();
}

#[test]
fn invalid_shapes_refuse_without_adding_resource_claims() {
    let (_base, mut controller) = fixture();
    let provider = InstalledParentProvider::new(&controller).unwrap();
    let before = controller.managed_resources().snapshot().unwrap();
    for shape in [
        HostLifetimeIntent {
            name: None,
            ..intent("unused")
        },
        HostLifetimeIntent {
            name: Some("../foreign".into()),
            ..intent("unused")
        },
        HostLifetimeIntent {
            mode: LifecycleMode::Persistent,
            ..intent("unused")
        },
        HostLifetimeIntent {
            cpus: 2,
            ..intent("unused")
        },
        HostLifetimeIntent {
            memory_mib: 1025,
            ..intent("unused")
        },
        HostLifetimeIntent {
            memory_mib: 0,
            ..intent("unused")
        },
        HostLifetimeIntent {
            backend: Some("mock".into()),
            ..intent("unused")
        },
    ] {
        assert!(
            provider
                .prepare_vm(&shape, "tenant", native_backend())
                .is_err()
        );
    }
    assert!(
        provider
            .prepare_vm(&intent("unused"), "", native_backend())
            .is_err()
    );
    assert!(
        provider
            .prepare_vm(&intent("unused"), "tenant", BackendKind::Mock)
            .is_err()
    );
    let after = controller.managed_resources().snapshot().unwrap();
    assert_eq!(before.usage, after.usage);
    assert_eq!(before.active_reservations, after.active_reservations);
    drop(provider);
    controller.close_admission();
    controller.try_finish().unwrap();
}

fn admit(
    base: &std::path::Path,
    name: &str,
    tenant: &str,
    backend: &str,
) -> mvm_hostd::plan_admission::AdmittedPlan {
    use mvm_core::plan::{
        PlanSeccompTier, SecretReleasePolicy, SynthesisInput, Variant, synthesize_plan,
    };
    use mvm_hostd::plan_admission::{
        InMemoryNonceLedger, RunPosture, SystemClock, admit_plan_for_run,
    };
    let input = SynthesisInput::builder()
        .vm_name(name)
        .tenant(tenant)
        .backend_name(backend)
        .image_name("synthetic-parent-test")
        .image_sha256("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        .seccomp_tier(PlanSeccompTier::Standard)
        .secret_release(SecretReleasePolicy::None)
        .secrets(vec![])
        .network_mode(Default::default())
        .cpus(1)
        .mem_mib(1024)
        .disk_mib(0)
        .boot_timeout_secs(30)
        .destroy_on_exit(true)
        .shares(vec![])
        .redaction(Default::default())
        .reversible_replacement(Default::default())
        .audit_labels(Default::default())
        .services(vec![])
        .stream_edges(vec![])
        .stream_retention(Default::default())
        .build()
        .unwrap();
    let plan = synthesize_plan(&input).unwrap();
    admit_plan_for_run(
        &plan,
        &SystemClock,
        &InMemoryNonceLedger::new(),
        Some(&base.join("host-keys")),
        None,
        None,
        RunPosture::on_backend(Variant::Dev, native_backend()),
    )
    .unwrap()
}

#[test]
fn opaque_admission_binds_only_original_tenant_shape_and_backend_once() {
    use mvm_hostd::host_lifetime_parent::ParentAdmissionProvider;
    let (base, mut controller) = fixture();
    let provider = InstalledParentProvider::new(&controller).unwrap();
    let prepared = provider
        .prepare_vm(&intent("admitted"), "tenant", native_backend())
        .unwrap();
    let binding = prepared.binding().clone();
    let backend = if cfg!(target_os = "linux") {
        "firecracker"
    } else {
        "hvf"
    };
    let foreign_tenant = admit(base.path(), "admitted", "other-tenant", backend);
    assert!(provider.bind(&foreign_tenant, &binding).is_err());
    let foreign_backend = admit(base.path(), "admitted", "tenant", "mock");
    assert!(provider.bind(&foreign_backend, &binding).is_err());
    let admitted = admit(base.path(), "admitted", "tenant", backend);
    let mut substituted = binding.clone();
    substituted.grant_serial += 1;
    assert!(provider.bind(&admitted, &substituted).is_err());
    let permit = provider.bind(&admitted, &binding).unwrap();
    permit.check_current(&binding).unwrap();
    assert!(provider.bind(&admitted, &binding).is_err());
    assert!(permit.check_current(&substituted).is_err());
    drop(prepared);
    assert_eq!(
        controller
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        1
    );
    provider.close_admission();
    assert!(permit.check_current(&binding).is_err());
    drop(permit);
    assert_eq!(
        controller
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        0
    );
    drop(provider);
    controller.close_admission();
    controller.try_finish().unwrap();
}

#[test]
fn original_core_anchor_retains_charge_after_unused_preparation_is_consumed() {
    let (_base, mut controller) = fixture();
    let provider = InstalledParentProvider::new(&controller).unwrap();
    let prepared = provider
        .prepare_vm(&intent("original-anchor"), "tenant", native_backend())
        .unwrap();
    let anchor = prepared.into_lifetime().unwrap().into_anchor();
    assert_eq!(
        controller
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        1
    );
    provider.close_admission();
    drop(provider);
    assert_eq!(
        controller
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        1
    );
    drop(anchor);
    assert_eq!(
        controller
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        0
    );
    controller.close_admission();
    controller.try_finish().unwrap();
}

#[test]
fn uncertain_launch_drop_retains_the_original_charge_and_exclusion() {
    use spiralcore_mvm_live::installed_owner::OwnerError;
    let (base, mut controller) = fixture();
    let provider = InstalledParentProvider::new(&controller).unwrap();
    let prepared = provider
        .prepare_vm(&intent("uncertain"), "tenant", native_backend())
        .unwrap();
    let original = prepared.binding().clone();
    let (lifetime, owner) = prepared.begin_launch().unwrap();
    assert_eq!(owner.binding(), &original);
    drop(lifetime);
    drop(owner);
    assert_eq!(
        controller
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        1
    );
    assert!(controller.try_finish().is_err());
    drop(provider);
    drop(controller);
    let config = InstalledConfig::canonical_native_profile();
    assert!(matches!(
        InstallationOwner::open(
            &base.path().join("installation"),
            InstallationKey {
                installation: [95; 32],
                profile: config.profile_hash(),
            }
        ),
        Err(OwnerError::Busy)
    ));
    // No VM was invoked. This deliberate uncertain-Drop witness retains one
    // original grant until this test process exits, not a native-exit proof.
}

#[test]
fn original_launch_owner_requires_the_canonical_outcomes_sealed_retirement() {
    use spiralcore_mvm_live::installed_parent::ParentLaunchOwner;
    let _: fn(&mut ParentLaunchOwner, &mvm_client::LaunchOutcome) -> anyhow::Result<()> =
        ParentLaunchOwner::finish_retired;
    let _: fn(&mut ParentLaunchOwner, &mvm_client::OwnedRetirement) -> anyhow::Result<()> =
        ParentLaunchOwner::finish_pending_retired;
}
