//! Genuine canonical admission without original native custody cannot retire it.
//! Mock is used only to produce the negative ordinary outcome, never exit proof.
#![cfg(target_os = "linux")]

use mvm_client::{HostLifetimeIntent, LaunchRequest, LifecycleMode, LocalBackend, RootfsSource};
use mvm_core::{client::MvmClient, util::test_env::TestEnv, vm_backend::BackendKind};
use mvm_hostd::host_lifetime_parent::ParentAdmissionProvider;
use sha2::{Digest, Sha256};
use spiralcore_mvm_live::{
    installed_owner::{InstallationKey, InstallationOwner, OwnerError},
    installed_parent::InstalledParentProvider,
    installed_supervisor::{InstalledConfig, InstalledSupervisor},
};
use std::os::unix::fs::PermissionsExt;

#[test]
fn ordinary_admitted_outcome_never_completes_the_original_native_parent() {
    let home = tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let mut environment = TestEnv::new();
    environment.isolate_mvm_home(home.path());
    let rootfs = home.path().join("synthetic-rootfs.ext4");
    let bytes = b"synthetic mock input; no native guest is booted\n";
    std::fs::write(&rootfs, bytes).unwrap();
    let config = InstalledConfig::canonical_native_profile();
    let key = InstallationKey {
        installation: [61; 32],
        profile: config.profile_hash(),
    };
    let installation = home.path().join("installation");
    let owner = InstallationOwner::initialize(&installation, key).unwrap();
    let mut supervisor = InstalledSupervisor::start(owner, config).unwrap();
    let parent = InstalledParentProvider::new(&supervisor).unwrap();
    let intent = HostLifetimeIntent {
        name: Some("same-visible-vm-name".into()),
        mode: LifecycleMode::Transient,
        cpus: 1,
        memory_mib: 1024,
        backend: Some("firecracker".into()),
    };
    let prepared = parent
        .prepare_vm(&intent, "local", BackendKind::Firecracker)
        .unwrap();

    // Bind a real opaque canonical admission to the original native grant.
    // This prevents an unrelated "unadmitted grant" refusal from masking the
    // missing sealed-receipt boundary under test. No native launcher is called.
    use mvm_core::plan::{
        PlanSeccompTier, SecretReleasePolicy, SynthesisInput, Variant, synthesize_plan,
    };
    use mvm_hostd::plan_admission::{
        InMemoryNonceLedger, RunPosture, SystemClock, admit_plan_for_run,
    };
    let digest = format!("{:x}", Sha256::digest(bytes));
    let input = SynthesisInput::builder()
        .vm_name("same-visible-vm-name")
        .tenant("local")
        .backend_name("firecracker")
        .image_name("synthetic-rootfs")
        .image_sha256(&digest)
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
    let admitted = admit_plan_for_run(
        &plan,
        &SystemClock,
        &InMemoryNonceLedger::new(),
        Some(&home.path().join("native-admission-keys")),
        None,
        None,
        RunPosture::on_backend(Variant::Dev, BackendKind::Firecracker),
    )
    .unwrap();
    let permit = parent.bind(&admitted, prepared.binding()).unwrap();
    let (lifetime, mut original_owner) = prepared.begin_launch().unwrap();
    drop(lifetime);
    drop(permit);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let client = LocalBackend::with_hypervisor("mock");
        let request =
            LaunchRequest::builder(LifecycleMode::Transient, RootfsSource::LocalPath(rootfs))
                .name("same-visible-vm-name")
                .cpus(1)
                .memory_mib(1024)
                .build()
                .unwrap();
        let outcome = client.launch(request).await.unwrap();
        assert_eq!(outcome.machine.id.0, original_owner.binding().vm_name);
        assert!(outcome.with_owned_retirement(|_| ()).is_err());
        assert!(original_owner.finish_retired(&outcome).is_err());
        assert_eq!(
            supervisor
                .managed_resources()
                .snapshot()
                .unwrap()
                .usage
                .guests,
            1
        );
        client.stop_machine(&outcome.machine.id).await.unwrap();
        client.cleanup_transient("same-visible-vm-name").unwrap();
        parent.close_admission();
        assert!(original_owner.finish_retired(&outcome).is_err());
    });
    drop(original_owner);
    assert_eq!(
        supervisor
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        1
    );
    assert!(supervisor.try_finish().is_err());
    drop(parent);
    drop(supervisor);
    assert!(matches!(
        InstallationOwner::open(&installation, key),
        Err(OwnerError::Busy)
    ));
    // Deliberate uncertainty remains charged until this test process exits.
}
