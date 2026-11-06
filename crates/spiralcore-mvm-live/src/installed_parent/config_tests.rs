//! Actual opaque admission plus the original launch owner; no helper or VM boot.
use super::*;
use mvm_core::vm_backend::VmStartConfig;

fn admitted(home: &std::path::Path, workload: &str) -> AdmittedPlan {
    use mvm_core::plan::{
        PlanSeccompTier, SecretReleasePolicy, SynthesisInput, Variant, synthesize_plan,
    };
    use mvm_hostd::plan_admission::{
        InMemoryNonceLedger, RunPosture, SystemClock, admit_plan_for_run,
    };
    let backend = if cfg!(target_os = "linux") {
        "firecracker"
    } else {
        "hvf"
    };
    let input = SynthesisInput::builder()
        .vm_name(workload)
        .tenant("local")
        .backend_name(backend)
        .image_name("synthetic-parent-config-test")
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
        RunPosture::on_backend(Variant::Dev, native_backend()),
    )
    .unwrap()
}

fn native_backend() -> BackendKind {
    if cfg!(target_os = "linux") {
        BackendKind::Firecracker
    } else {
        BackendKind::Hvf
    }
}

fn config(provider: &InstalledParentProvider, name: &str) -> (Arc<VmGrant>, VmStartConfig) {
    let prepared = provider
        .prepare_vm(
            &HostLifetimeIntent {
                name: Some(name.into()),
                mode: LifecycleMode::Transient,
                cpus: 1,
                memory_mib: 1024,
                backend: None,
            },
            "local",
            native_backend(),
        )
        .unwrap();
    let grant = prepared.grant.clone();
    let anchor = prepared.into_lifetime().unwrap().into_anchor();
    let mut config = VmStartConfig {
        name: name.into(),
        cpus: 1,
        memory_mib: 1024,
        tenant_id: Some("local".into()),
        ..Default::default()
    };
    anchor.bind_to_config(&mut config).unwrap();
    (grant, config)
}

#[test]
fn original_config_binds_opaque_workload_to_generated_vm_and_registration_once() {
    let (home, mut supervisor, provider) = endpoint_tests::fixture();
    let admitted = admitted(home.path(), "signed-workload-label");
    let (grant, mut config) = config(&provider, "generated-vm-name");
    config.plan_json = Some(serde_json::to_string(admitted.signed()).unwrap());
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
    // The old binding-only API cannot infer a generated VM from a plan label.
    assert!(provider.bind(&admitted, &grant.binding).is_err());
    let permit = provider
        .bind_config(&admitted, &config, native_backend())
        .unwrap();
    let erased: Arc<dyn CoreCurrentParentPermit> = grant.clone();
    assert!(Arc::ptr_eq(&permit, &erased));
    config
        .host_lifetime
        .as_ref()
        .unwrap()
        .bind_parent_permit(&config, native_backend(), permit.clone())
        .unwrap();
    permit.check_current(&grant.binding).unwrap();
    let lifecycle = grant.lifecycle.lock().unwrap();
    assert_eq!(
        lifecycle.admitted_plan.as_deref(),
        Some(admitted.plan_id().0.as_str())
    );
    assert!(lifecycle.registration.is_some());
    drop(lifecycle);
    let after = supervisor.managed_resources().snapshot().unwrap().usage;
    assert_eq!(
        after.host_metadata_bytes,
        before.host_metadata_bytes + 32768
    );
    assert_eq!(after.workers, before.workers);
    assert!(
        provider
            .bind_config(&admitted, &config, native_backend())
            .is_err()
    );
    drop((permit, erased, config, grant, provider));
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}

#[test]
fn equal_projection_from_another_actual_owner_cannot_bind_original_grant() {
    let (home, mut supervisor, provider) = endpoint_tests::fixture();
    let (_foreign_home, mut foreign, foreign_provider) = endpoint_tests::fixture();
    let admitted = admitted(home.path(), "same-vm-name");
    let (grant, own) = config(&provider, "same-vm-name");
    let (foreign_grant, mut other) = config(&foreign_provider, "same-vm-name");
    assert_eq!(grant.binding, foreign_grant.binding);
    other.plan_json = Some(serde_json::to_string(admitted.signed()).unwrap());
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
    assert!(
        provider
            .bind_config(&admitted, &other, native_backend())
            .is_err()
    );
    assert!(grant.lifecycle.lock().unwrap().admitted_plan.is_none());
    assert_eq!(
        before,
        supervisor.managed_resources().snapshot().unwrap().usage
    );
    drop((other, own, grant, foreign_grant, provider, foreign_provider));
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
    foreign.close_admission();
    foreign.try_finish().unwrap();
}

#[test]
fn substituted_signed_projection_and_tenant_refuse_before_parent_binding() {
    let (home, mut supervisor, provider) = endpoint_tests::fixture();
    let admitted = admitted(home.path(), "generated-vm-name");
    let changed = self::admitted(home.path(), "substituted-workload");
    let (grant, mut config) = config(&provider, "generated-vm-name");
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
    config.plan_json = Some(serde_json::to_string(changed.signed()).unwrap());
    assert!(
        provider
            .bind_config(&admitted, &config, native_backend())
            .is_err()
    );
    config.plan_json = Some(serde_json::to_string(admitted.signed()).unwrap());
    config.tenant_id = Some("foreign-tenant".into());
    assert!(
        provider
            .bind_config(&admitted, &config, native_backend())
            .is_err()
    );
    assert!(grant.lifecycle.lock().unwrap().admitted_plan.is_none());
    assert_eq!(
        before,
        supervisor.managed_resources().snapshot().unwrap().usage
    );
    drop((config, grant, provider));
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}

#[test]
fn unsupported_registration_preflight_does_not_consume_original_binding() {
    let (home, mut supervisor, provider) = endpoint_tests::fixture();
    let admitted = admitted(home.path(), "generated-vm-name");
    let (grant, mut config) = config(&provider, "generated-vm-name");
    config.plan_json = Some(serde_json::to_string(admitted.signed()).unwrap());
    config.service_proxies.push(
        mvm_contract::protocol::broker_control::ServiceProxyBinding {
            service: mvm_contract::protocol::broker::ServiceId::parse("host.audit.v1").unwrap(),
            endpoint: "/nonexistent/synthetic-parent-config-proxy".into(),
            capabilities: vec![],
        },
    );
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
    assert!(
        provider
            .bind_config(&admitted, &config, native_backend())
            .is_err()
    );
    {
        let lifecycle = grant.lifecycle.lock().unwrap();
        assert!(
            lifecycle.admitted_plan.is_none(),
            "registration preflight consumed admission"
        );
        assert!(lifecycle.admitted_session.is_none());
        assert!(lifecycle.registration.is_none());
    }
    assert_eq!(
        before,
        supervisor.managed_resources().snapshot().unwrap().usage
    );
    config.service_proxies.clear();
    let permit = provider
        .bind_config(&admitted, &config, native_backend())
        .unwrap();
    permit.check_current(&grant.binding).unwrap();
    drop((permit, config, grant, provider));
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}

#[test]
fn original_portable_workspace_is_funded_once_and_does_not_refund_retained_disk() {
    use mvm_core::plan::{KeyId, PlanArtifact};
    use std::num::NonZeroU64;

    let (_home, mut supervisor, provider) = endpoint_tests::fixture();
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
    let pin = PlanArtifact::new("a".repeat(64), &[7; 64], KeyId("installer".into()));
    let prepare = || {
        provider
            .prepare_vm(
                &HostLifetimeIntent {
                    name: Some("quoted-runtime".into()),
                    mode: LifecycleMode::Transient,
                    cpus: 1,
                    memory_mib: 1024,
                    backend: None,
                },
                "local",
                native_backend(),
            )
            .unwrap()
    };
    let quote = || {
        PortableStartupQuote::new(
            pin.clone(),
            NonZeroU64::new(4096).unwrap(),
            NonZeroU64::new(8192).unwrap(),
            NonZeroU64::new(16384).unwrap(),
        )
        .unwrap()
    };
    assert!(
        prepare()
            .with_portable_startup(quote())
            .unwrap()
            .into_lifetime()
            .is_err()
    );
    assert_eq!(
        supervisor.managed_resources().snapshot().unwrap().usage,
        before
    );
    let prepared = prepare();
    let grant = prepared.grant.clone();
    let unfunded = supervisor.managed_resources().snapshot().unwrap().usage;
    let prepared = prepared.with_portable_startup(quote()).unwrap();
    let funded = supervisor.managed_resources().snapshot().unwrap().usage;
    assert_eq!(
        funded.host_metadata_bytes,
        unfunded.host_metadata_bytes + 12288
    );
    assert_eq!(
        funded.staging_disk_bytes,
        unfunded.staging_disk_bytes + 20480
    );
    let (lifetime, mut owner) = prepared.begin_launch().unwrap();
    let anchor = lifetime.into_anchor();
    provider
        .check_portable_startup_workspace(&anchor, &pin)
        .unwrap();
    let mut wrong = pin.clone();
    wrong.bundle_sha256 = "b".repeat(64);
    assert!(
        provider
            .check_portable_startup_workspace(&anchor, &wrong)
            .is_err()
    );
    assert!(grant.begin_startup_workspace(&grant.binding).is_err());
    assert!(grant.complete_startup_workspace(&grant.binding).is_err());

    // Isolate the workspace callbacks from opaque admission and native boot.
    grant.lifecycle.lock().unwrap().admitted_plan = Some("fixture".into());
    assert!(grant.complete_startup_workspace(&grant.binding).is_err());
    grant.begin_startup_workspace(&grant.binding).unwrap();
    assert!(grant.begin_startup_workspace(&grant.binding).is_err());
    assert!(
        provider
            .check_portable_startup_workspace(&anchor, &pin)
            .is_err()
    );
    grant.complete_startup_workspace(&grant.binding).unwrap();
    assert!(grant.complete_startup_workspace(&grant.binding).is_err());
    assert_eq!(
        supervisor.managed_resources().snapshot().unwrap().usage,
        funded
    );
    assert!(grant.lifecycle.lock().unwrap().seal_retired().is_err());

    // Test-only cleanup. The synthetic callback test issued no runtime files.
    owner.completed = true;
    drop((anchor, grant, owner));
    assert_eq!(
        supervisor.managed_resources().snapshot().unwrap().usage,
        before
    );
    drop(provider);
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}
