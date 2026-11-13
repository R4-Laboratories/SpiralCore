//! Actual opaque admission plus the original launch owner; no helper or VM boot.
use super::*;
use mvm_core::vm_backend::VmStartConfig;

fn admitted(home: &std::path::Path, workload: &str) -> AdmittedPlan {
    admit_with_network(home, workload, None)
}

fn admit_with_network(
    home: &std::path::Path,
    workload: &str,
    network: Option<&mvm_core::policy::network_policy::NetworkPolicy>,
) -> AdmittedPlan {
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
    let mut plan = synthesize_plan(&input).unwrap();
    let published = network.map(|policy| test_bundle::publish(home, &mut plan, policy));
    let bundle = published.as_ref().map(|(resolver, trust)| {
        mvm_hostd::plan_admission::BundleAdmissionContext {
            resolver,
            trust,
            image: None,
        }
    });
    admit_plan_for_run(
        &plan,
        &SystemClock,
        &InMemoryNonceLedger::new(),
        Some(&home.join("admission-keys")),
        bundle.as_ref(),
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
            &VmStartConfig {
                name: name.into(),
                tenant_id: Some("local".into()),
                cpus: 1,
                memory_mib: 1024,
                ..Default::default()
            },
            "local",
            native_backend(),
        )
        .unwrap();
    let grant = prepared.grant.clone();
    let (lifetime, owner) = prepared.begin_launch().unwrap();
    // This fixture never enters a backend. Keep the original grant in the returned tuple.
    let mut owner = owner;
    owner.completed = true;
    drop(owner);
    let anchor = lifetime.into_anchor();
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
fn obtaining_a_registration_callback_requires_an_existing_original_registrar() {
    let (_home, mut supervisor, provider) = endpoint_tests::fixture();
    let preparation = provider
        .prepare_vm(
            &VmStartConfig {
                name: "missing-registrar".into(),
                tenant_id: Some("local".into()),
                memory_mib: 1024,
                cpus: 1,
                ..Default::default()
            },
            "local",
            native_backend(),
        )
        .unwrap();
    let (lifetime, mut owner) = preparation.begin_launch().unwrap();
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
    let error = owner
        .original_registrar()
        .err()
        .expect("fixture has no tenant registrar");
    assert_eq!(
        error.to_string(),
        "original registered tenant daemon missing"
    );
    assert_eq!(
        before,
        supervisor.managed_resources().snapshot().unwrap().usage
    );
    assert_eq!(before.guests, 1);
    // No backend or registrar was invoked in this component.
    owner.completed = true;
    drop((owner, lifetime, provider));
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
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

mod test_bundle {
    use ed25519_dalek::{Signer, SigningKey};
    use mvm_core::plan::bundle::{
        ARTIFACTS_DIR, ArtifactRole, BUNDLE_SCHEMA_VERSION, BundleArtifact, BundleManifest,
        BundleRegistry, FsBundleResolver, FsTrustStore, PlanArtifact, bundle_sha256,
        canonical_manifest_bytes, key_id_from_pubkey, sha256_hex, write_bundle,
    };
    pub(super) fn publish(
        home: &std::path::Path,
        plan: &mut mvm_core::plan::ExecutionPlan,
        policy: &mvm_core::policy::network_policy::NetworkPolicy,
    ) -> (FsBundleResolver, FsTrustStore) {
        let signer = SigningKey::from_bytes(&[17; 32]);
        let key = key_id_from_pubkey(&signer.verifying_key());
        let members = [
            ("kernel", ArtifactRole::Kernel, b"k".to_vec()),
            ("rootfs", ArtifactRole::Rootfs, b"r".to_vec()),
        ];
        let manifest = BundleManifest {
            schema_version: BUNDLE_SCHEMA_VERSION,
            publisher: "test".into(),
            key_id: key.clone(),
            arch: mvm_core::arch::GuestArch::host().to_string(),
            created_at: "2026-01-01T00:00:00Z".into(),
            labels: Default::default(),
            artifacts: members
                .iter()
                .map(|(name, role, bytes)| BundleArtifact {
                    name: (*name).into(),
                    role: role.clone(),
                    path: format!("{ARTIFACTS_DIR}/{name}"),
                    sha256: sha256_hex(bytes),
                    size_bytes: bytes.len() as u64,
                })
                .collect(),
            members: Vec::new(),
            kernel_version: None,
            profile: None,
            workload_label: None,
            verity: None,
            resources: None,
        };
        let signature = signer
            .sign(&canonical_manifest_bytes(&manifest).unwrap())
            .to_bytes();
        let data = write_bundle(
            &manifest,
            &signer,
            members
                .into_iter()
                .map(|(name, _, bytes)| (format!("{ARTIFACTS_DIR}/{name}"), bytes))
                .collect(),
        )
        .unwrap();
        let digest = bundle_sha256(&data);
        std::fs::write(BundleRegistry::new(home).archive_path(&digest), data).unwrap();
        std::fs::write(
            home.join(format!("{}.pub", key.0)),
            signer.verifying_key().to_bytes(),
        )
        .unwrap();
        plan.bundle = Some(PlanArtifact::new(digest, &signature, key));
        plan.asset_identities.push(
            mvm_core::plan::AssetIdentity::new(
                mvm_core::plan::AssetKind::Policy,
                "network",
                mvm_core::policy::network_policy::content_sha256(policy),
            )
            .unwrap(),
        );
        plan.plan_id = mvm_core::plan::compute_plan_id(plan);
        (FsBundleResolver::new(home), FsTrustStore::new(home))
    }
}

fn selected_policy() -> mvm_core::policy::network_policy::NetworkPolicy {
    use mvm_contract::policy::network_policy::*;
    let mapping = CooperatingPayloadPolicy::new(vec![CooperatingTcpPeer {
        host: "peer.example".into(),
        port: 443,
        server_name: "peer.example".into(),
        spki_sha256: [11; 32],
        profile: PayloadMappingProfile::SpiralCoreObjectStreamSmallV1,
    }])
    .unwrap();
    NetworkPolicy::allow_list(vec![HostPort::new("peer.example", 443)])
        .with_payload_mapping(Some(mapping))
}

#[test]
fn signed_object_policy_and_private_publisher_bind_only_to_original_armed_config() {
    let (home, mut supervisor, provider) = endpoint_tests::fixture();
    let policy = selected_policy();
    let admitted = admit_with_network(home.path(), "signed-role", Some(&policy));
    let (grant, mut cfg) = config(&provider, "selected-vm");
    cfg.plan_json = Some(serde_json::to_string(admitted.signed()).unwrap());
    cfg.network_policy = policy.clone().with_payload_mapping(Some(
        mvm_contract::policy::network_policy::CooperatingPayloadPolicy::new(
            policy
                .payload_mapping()
                .unwrap()
                .peers()
                .iter()
                .cloned()
                .map(|mut peer| {
                    peer.spki_sha256 = [12; 32];
                    peer
                })
                .collect(),
        )
        .unwrap(),
    ));
    let initial = supervisor.managed_resources().snapshot().unwrap().usage;
    assert!(
        provider
            .bind_config(&admitted, &cfg, native_backend())
            .is_err()
    );
    assert_eq!(
        initial,
        supervisor.managed_resources().snapshot().unwrap().usage
    );
    cfg.network_policy = policy;
    let permit = provider
        .bind_config(&admitted, &cfg, native_backend())
        .unwrap();
    cfg.host_lifetime
        .as_ref()
        .unwrap()
        .bind_parent_permit(&cfg, native_backend(), permit)
        .unwrap();
    assert_eq!(
        grant
            .lifecycle
            .lock()
            .unwrap()
            .selected_network
            .as_ref()
            .unwrap()
            .plan_id,
        *admitted.plan_id()
    );
    drop((cfg, grant, provider));
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}

fn operator_partition() -> mvm_core::host_lifetime::EndpointPartitionLimits {
    // Explicit fixture operator allocations from a single installed tenant.
    // Never estimated RAM or disk quota derived from the signed network limits.
    mvm_core::host_lifetime::EndpointPartitionLimits {
        host_plaintext_bytes: 16 << 20,
        host_codec_bytes: 16 << 20,
        host_carrier_bytes: 8 << 20,
        host_metadata_bytes: 48 << 20,
        host_stack_bytes: 48 << 20,
        cache_disk_bytes: 32 << 20,
        staging_disk_bytes: 4 << 20,
        workers: 6,
        max_streams: 1,
    }
}

#[cfg(target_os = "linux")]
#[test]
fn public_selected_handoff_charges_one_pool_and_refuses_second_claim_before_disk_effects() {
    let (home, supervisor, mut provider) = endpoint_tests::fixture();
    provider
        .configure_object_tcp(&supervisor, operator_partition())
        .unwrap();
    let policy = selected_policy();
    let admit = |name: &str| {
        let admitted = admit_with_network(home.path(), name, Some(&policy));
        let mut cfg = VmStartConfig {
            name: name.to_owned(),
            tenant_id: Some("local".into()),
            cpus: 1,
            memory_mib: 1024,
            network_policy: policy.clone(),
            plan_json: Some(serde_json::to_string(admitted.signed()).unwrap()),
            ..Default::default()
        };
        let preparation = provider
            .prepare_vm(&cfg, "local", native_backend())
            .unwrap();
        let (lifetime, owner) = preparation.begin_launch().unwrap();
        let anchor = lifetime.into_anchor();
        anchor.bind_to_config(&mut cfg).unwrap();
        let permit = provider
            .bind_config(&admitted, &cfg, native_backend())
            .unwrap();
        anchor
            .bind_parent_permit(&cfg, native_backend(), permit)
            .unwrap();
        (anchor, cfg, owner)
    };
    let (anchor, a, _original_a) = admit("public-selected-a");
    let (other, b, _original_b) = admit("public-selected-b");
    let before = supervisor.managed_resources().snapshot().unwrap();
    let first = anchor
        .prepare_endpoint_handoff(&a, native_backend())
        .unwrap();
    first.check_current().unwrap();
    let projection = first.partition().projection();
    assert_eq!(projection.binding.vm_name, "public-selected-a");
    assert_eq!(
        projection.binding.grant_serial,
        anchor.binding().grant_serial
    );
    assert_eq!(projection.limits, operator_partition());
    let after = supervisor.managed_resources().snapshot().unwrap();
    assert_eq!(after.usage.guests, 2);
    assert_eq!(after.usage.workers - before.usage.workers, 6);
    assert_eq!(
        after.usage.cache_disk_bytes - before.usage.cache_disk_bytes,
        32 << 20
    );
    assert_eq!(
        after.usage.staging_disk_bytes - before.usage.staging_disk_bytes,
        4 << 20
    );
    assert!(
        anchor
            .prepare_endpoint_handoff(&a, native_backend())
            .is_err()
    );
    assert!(
        other
            .prepare_endpoint_handoff(&b, native_backend())
            .is_err()
    );
    assert_eq!(
        after.usage,
        supervisor.managed_resources().snapshot().unwrap().usage
    );
    provider.close_admission();
    assert!(first.check_current().is_err());
    // A published partition has no fabricated child exit. Both original launch
    // owners remain armed and the held store is explicitly not retired here.
}

#[cfg(not(target_os = "linux"))]
#[test]
fn public_selected_configuration_refuses_missing_kernel_bound_before_charging() {
    let (_home, mut supervisor, mut provider) = endpoint_tests::fixture();
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
    let error = provider
        .configure_object_tcp(&supervisor, operator_partition())
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::Unsupported
    );
    assert_eq!(
        before,
        supervisor.managed_resources().snapshot().unwrap().usage
    );
    drop(provider);
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}

#[test]
fn tenant_helper_funds_both_process_limits_and_both_journal_generations() {
    let limits = TenantHelperLimits {
        process_address_space_bytes: std::num::NonZeroU64::new(32 << 20).unwrap(),
        audit_chain_bytes: std::num::NonZeroU64::new(1 << 20).unwrap(),
    };
    let claim = limits.claim().unwrap();
    assert_eq!(claim.host_carrier_bytes, 2 * (32 << 20));
    assert_eq!(claim.workers, 10);
    assert_eq!(claim.staging_disk_bytes, 2 * (64 << 10));
    assert!(claim.host_metadata_bytes >= 2 * (64 << 10));
    assert_eq!(claim.guests, 0);
    assert!(
        TenantHelperLimits {
            audit_chain_bytes: std::num::NonZeroU64::MIN,
            process_address_space_bytes: std::num::NonZeroU64::MAX
        }
        .claim()
        .is_err()
    );
}

#[test]
fn original_audit_bucket_is_persisted_once_and_survives_vm_disposal() {
    let (home, mut supervisor, provider) = endpoint_tests::fixture();
    let (grant, mut config) = config(&provider, "retained-audit");
    let admitted = admitted(home.path(), "retained-audit");
    config.plan_json = Some(serde_json::to_string(admitted.signed()).unwrap());
    let permit = provider
        .bind_config(&admitted, &config, native_backend())
        .unwrap();
    config
        .host_lifetime
        .as_ref()
        .unwrap()
        .bind_parent_permit(&config, native_backend(), permit)
        .unwrap();
    let facts = grant
        .lifecycle
        .lock()
        .unwrap()
        .registration
        .clone()
        .unwrap();
    let cap = std::num::NonZeroU64::new(16 * 1024).unwrap();
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
    provider
        .shared
        .reserve_audit_debt(&grant, &facts, cap)
        .unwrap();
    let after = supervisor.managed_resources().snapshot().unwrap().usage;
    assert_eq!(
        after.cache_disk_bytes - before.cache_disk_bytes,
        cap.get() + 2 * 4096
    );
    provider
        .shared
        .reserve_audit_debt(&grant, &facts, cap)
        .unwrap();
    assert!(
        provider
            .shared
            .reserve_audit_debt(&grant, &facts, std::num::NonZeroU64::MIN)
            .is_err()
    );
    assert_eq!(
        supervisor.managed_resources().snapshot().unwrap().usage,
        after
    );
    drop((config, grant, provider));
    assert!(
        supervisor
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .cache_disk_bytes
            >= cap.get()
    );
    let record = std::fs::read(home.path().join("installation/.retained-audit-0")).unwrap();
    assert_eq!(&record[..8], b"SCAUDT01");
    assert_eq!(
        &record[record.len() - 32..],
        Sha256::digest(&record[..record.len() - 32]).as_slice()
    );
    supervisor.close_admission();
    assert!(supervisor.try_finish().is_err());
}
