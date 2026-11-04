//! Actual original-parent cancellation before canonical invocation, not native exit.
#![cfg(target_os = "linux")]
use mvm_client::{HostLifetimeIntent, LaunchRequest, LifecycleMode, LocalBackend, RootfsSource};
use mvm_core::{host_lifetime::*, vm_backend::BackendKind};
use mvm_hostd::host_lifetime_parent::ParentAdmissionProvider;
use sha2::{Digest, Sha256};
use spiralcore_mvm_live::{
    installed_owner::{InstallationKey, InstallationOwner},
    installed_parent::{InstalledParentProvider, ParentLaunchOwner},
    installed_supervisor::{InstalledConfig, InstalledSupervisor},
};
use std::{fs, os::unix::fs::PermissionsExt, sync::Arc};

fn fixture() -> (
    tempfile::TempDir,
    InstalledSupervisor,
    InstalledParentProvider,
) {
    let home = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let config = InstalledConfig::canonical_native_profile();
    let key = InstallationKey {
        installation: [62; 32],
        profile: config.profile_hash(),
    };
    let owner = InstallationOwner::initialize(&home.path().join("installation"), key).unwrap();
    let supervisor = InstalledSupervisor::start(owner, config).unwrap();
    let parent = InstalledParentProvider::new(&supervisor).unwrap();
    (home, supervisor, parent)
}
fn begin(parent: &InstalledParentProvider) -> (PreparedHostLifetime, ParentLaunchOwner) {
    parent
        .prepare_vm(
            &HostLifetimeIntent {
                name: Some("unused-original-parent".into()),
                mode: LifecycleMode::Transient,
                cpus: 1,
                memory_mib: 1024,
                backend: None,
            },
            "local",
            BackendKind::Firecracker,
        )
        .unwrap()
        .begin_launch()
        .unwrap()
}
fn unused(lifetime: PreparedHostLifetime) -> mvm_client::UnusedHostLifetimePreparation {
    let request = LaunchRequest::builder(
        LifecycleMode::Persistent,
        RootfsSource::LocalPath("/nonexistent/unused-rootfs".into()),
    )
    .name("unused-original-parent")
    .cpus(1)
    .memory_mib(1024)
    .build()
    .unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let failure = runtime
        .block_on(
            LocalBackend::with_hypervisor("firecracker")
                .try_launch_with_host_lifetime(request, lifetime),
        )
        .unwrap_err();
    assert!(
        failure
            .error()
            .to_string()
            .contains("explicitly named transient")
    );
    failure.into_parts().1.unwrap()
}
fn foreign(home: &std::path::Path, binding: HostLaunchBinding) -> PreparedHostLifetime {
    let directory = home.join("foreign-preparation");
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(directory.join("state"), b"prepared").unwrap();
    fs::write(directory.join("holder"), b"").unwrap();
    for name in ["state", "holder"] {
        fs::set_permissions(directory.join(name), fs::Permissions::from_mode(0o600)).unwrap();
    }
    PreparedHostLifetime::bind(
        HostLifetimeInputs {
            directory: fs::File::open(&directory).unwrap(),
            holder: fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(directory.join("holder"))
                .unwrap(),
            expected: HostLeaseExpectation {
                state_name: "state".into(),
                state_bytes: 8,
                state_sha256: Sha256::digest(b"prepared").into(),
                absent_names: [".state.next".into(), ".state.intent".into()],
            },
            binding,
        },
        Arc::new(String::from("matching DTO is not the original grant")),
    )
    .unwrap()
}
fn admitted(home: &std::path::Path) -> mvm_hostd::plan_admission::AdmittedPlan {
    use mvm_core::plan::{
        PlanSeccompTier, SecretReleasePolicy, SynthesisInput, Variant, synthesize_plan,
    };
    use mvm_hostd::plan_admission::{
        InMemoryNonceLedger, RunPosture, SystemClock, admit_plan_for_run,
    };
    let input = SynthesisInput::builder()
        .vm_name("unused-original-parent")
        .tenant("local")
        .backend_name("firecracker")
        .image_name("synthetic-unused-image")
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
    admit_plan_for_run(
        &synthesize_plan(&input).unwrap(),
        &SystemClock,
        &InMemoryNonceLedger::new(),
        Some(&home.join("admission-keys")),
        None,
        None,
        RunPosture::on_backend(Variant::Dev, BackendKind::Firecracker),
    )
    .unwrap()
}

#[test]
fn foreign_matching_binding_cannot_cancel_and_normal_close_allows_original_last_owner_refund() {
    let (home, mut supervisor, parent) = fixture();
    let (lifetime, mut owner) = begin(&parent);
    let false_certificate = unused(foreign(home.path(), owner.binding().clone()));
    assert_eq!(false_certificate.binding(), owner.binding());
    assert!(owner.cancel_unused(&false_certificate).is_err());
    assert_eq!(
        supervisor
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        1
    );
    let certificate = unused(lifetime);
    parent.close_admission();
    supervisor.close_admission();
    owner.cancel_unused(&certificate).unwrap();
    owner.cancel_unused(&certificate).unwrap();
    drop(owner);
    assert_eq!(
        supervisor
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        1
    );
    drop(certificate);
    assert_eq!(
        supervisor
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        0
    );
    drop(parent);
    supervisor.try_finish().unwrap();
}

#[test]
fn same_preparation_retries_without_new_identity_and_cancelled_grant_cannot_reauthorize() {
    let (home, mut supervisor, parent) = fixture();
    let (lifetime, mut owner) = begin(&parent);
    let serial = owner.binding().grant_serial;
    let first = unused(lifetime);
    let retried = unused(first.into_prepared());
    assert_eq!(retried.binding().grant_serial, serial);
    assert_eq!(
        supervisor
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        1
    );
    owner.cancel_unused(&retried).unwrap();
    assert!(
        parent
            .bind(&admitted(home.path()), owner.binding())
            .is_err()
    );
    let anchor = retried.into_prepared().into_anchor();
    drop(owner);
    assert_eq!(
        supervisor
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        1
    );
    drop(anchor);
    assert_eq!(
        supervisor
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .guests,
        0
    );
    drop(parent);
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}

#[test]
fn a_previous_bound_admission_prevents_unused_cancellation_even_for_the_same_preparation() {
    let (home, mut supervisor, parent) = fixture();
    let prepared = parent
        .prepare_vm(
            &HostLifetimeIntent {
                name: Some("unused-original-parent".into()),
                mode: LifecycleMode::Transient,
                cpus: 1,
                memory_mib: 1024,
                backend: None,
            },
            "local",
            BackendKind::Firecracker,
        )
        .unwrap();
    let permit = parent
        .bind(&admitted(home.path()), prepared.binding())
        .unwrap();
    let (lifetime, mut owner) = prepared.begin_launch().unwrap();
    let certificate = unused(lifetime);
    assert!(owner.cancel_unused(&certificate).is_err());
    drop(certificate);
    drop(permit);
    drop(owner);
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
    // Previous admission is not erased by an unused preparation from this call.
}
