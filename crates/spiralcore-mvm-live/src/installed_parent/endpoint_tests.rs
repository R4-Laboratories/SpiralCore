//! Policy/storage integration; private fixture admission is not native proof.
use super::*;
use mvm_core::host_lifetime::EndpointPartitionLimits;
use std::{fs, os::unix::fs::PermissionsExt};

pub(super) fn fixture() -> (
    tempfile::TempDir,
    InstalledSupervisor,
    InstalledParentProvider,
) {
    let home = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let config = crate::installed_supervisor::InstalledConfig::canonical_native_profile();
    let owner = crate::installed_owner::InstallationOwner::initialize(
        &home.path().join("installation"),
        crate::installed_owner::InstallationKey {
            installation: [71; 32],
            profile: config.profile_hash(),
        },
    )
    .unwrap();
    let supervisor = InstalledSupervisor::start(owner, config).unwrap();
    let provider = InstalledParentProvider::new(&supervisor).unwrap();
    (home, supervisor, provider)
}
fn grant(provider: &InstalledParentProvider, name: &str) -> PreparedParentVm {
    provider
        .prepare_vm(
            &HostLifetimeIntent {
                name: Some(name.into()),
                mode: LifecycleMode::Transient,
                cpus: 1,
                memory_mib: 1,
                backend: None,
            },
            "local",
            if cfg!(target_os = "linux") {
                BackendKind::Firecracker
            } else {
                BackendKind::Hvf
            },
        )
        .unwrap()
}
fn fixture_profile() -> endpoint::QualifiedEndpointProfile {
    fixture_profile_with_cache(
        mvm_vmm::host::telemetry_provisioning::DEFAULT_RECORDS_BYTE_CAP + 65536,
    )
}
fn fixture_profile_with_cache(cache_disk_bytes: u64) -> endpoint::QualifiedEndpointProfile {
    // Only private tests may supply these synthetic policy limits. They do not
    // activate native support or qualify a real bootstrap/codec resource model.
    endpoint::QualifiedEndpointProfile::fixture(EndpointPartitionLimits {
        host_plaintext_bytes: 65536,
        host_codec_bytes: 65536,
        host_carrier_bytes: 65536,
        host_metadata_bytes: 65536,
        host_stack_bytes: 65536,
        cache_disk_bytes,
        staging_disk_bytes: 65536,
        workers: 1,
        max_streams: 1,
    })
}
fn fixture_admit(prepared: &PreparedParentVm) {
    let mut state = prepared.grant.lifecycle.lock().unwrap();
    state.admitted_plan = Some("private-policy-fixture-not-canonical-admission".into());
    state.admitted_session = Some([72; 32]);
}

#[test]
fn missing_profile_refuses_before_any_endpoint_credit_or_disk_effect() {
    let (_home, mut supervisor, provider) = fixture();
    let prepared = grant(&provider, "endpoint-profile-gate");
    fixture_admit(&prepared);
    let before = supervisor.managed_resources().snapshot().unwrap();
    assert!(
        prepared
            .grant
            .prepare_endpoint_partition(prepared.binding())
            .is_err()
    );
    assert_eq!(
        prepared.grant.lifecycle.lock().unwrap().endpoint,
        EndpointIssuance::NeverIssued
    );
    assert_eq!(
        supervisor.managed_resources().snapshot().unwrap().usage,
        before.usage
    );
    drop(prepared);
    drop(provider);
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}

#[test]
fn mandatory_telemetry_capacity_refuses_before_endpoint_credit_or_disk_effect() {
    let (home, mut supervisor, mut provider) = fixture();
    // This deliberately undersized TOTAL allowance cannot contain the
    // mandatory canonical records buffer, regardless of available host quota.
    provider
        .configure_endpoints(&supervisor, fixture_profile_with_cache(65536))
        .unwrap();
    let prepared = grant(&provider, "endpoint-telemetry-capacity");
    fixture_admit(&prepared);
    let before = supervisor.managed_resources().snapshot().unwrap();
    let names = || {
        let mut names: Vec<_> = fs::read_dir(home.path().join("installation"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        names.sort();
        names
    };
    let before_names = names();
    assert!(
        prepared
            .grant
            .prepare_endpoint_partition(prepared.binding())
            .is_err(),
        "endpoint admitted a total cache smaller than mandatory telemetry"
    );
    assert_eq!(
        prepared.grant.lifecycle.lock().unwrap().endpoint,
        EndpointIssuance::NeverIssued
    );
    assert_eq!(
        supervisor.managed_resources().snapshot().unwrap().usage,
        before.usage
    );
    assert_eq!(names(), before_names);
    drop(prepared);
    drop(provider);
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}

#[test]
fn foreign_supervisor_cannot_supply_an_endpoint_inventory() {
    let (_home, mut supervisor, mut provider) = fixture();
    let (_foreign_home, mut foreign, foreign_provider) = fixture();
    let before = foreign.managed_resources().snapshot().unwrap().usage;
    assert!(
        provider
            .configure_endpoints(&foreign, fixture_profile())
            .is_err(),
        "foreign supervisor supplied the original parent's disk inventory"
    );
    assert_eq!(
        foreign.managed_resources().snapshot().unwrap().usage,
        before
    );
    drop((provider, foreign_provider));
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
    foreign.close_admission();
    foreign.try_finish().unwrap();
}

#[test]
fn issued_original_guard_is_one_shot_acyclic_and_quarantines_unproven_resources() {
    let (_home, mut supervisor, mut provider) = fixture();
    provider
        .configure_endpoints(&supervisor, fixture_profile())
        .unwrap();
    let prepared = grant(&provider, "endpoint-owner-original");
    fixture_admit(&prepared);
    let weak = Arc::downgrade(&prepared.grant);
    let partition = prepared
        .grant
        .prepare_endpoint_partition(prepared.binding())
        .unwrap();
    let guard = prepared
        .grant
        .lifecycle
        .lock()
        .unwrap()
        .endpoint_owner
        .as_ref()
        .unwrap()
        .clone();
    let erased: Arc<dyn mvm_core::host_lifetime::EndpointPartitionGuard> = guard.clone();
    assert!(partition.retains_guard(&erased));
    partition.check_current().unwrap();
    assert!(
        prepared
            .grant
            .prepare_endpoint_partition(prepared.binding())
            .is_err()
    );
    let before = supervisor.managed_resources().snapshot().unwrap();
    assert_eq!(before.usage.workers, 1);
    assert_eq!(
        before.usage.cache_disk_bytes,
        mvm_vmm::host::telemetry_provisioning::DEFAULT_RECORDS_BYTE_CAP + 65536
    );
    assert_eq!(before.usage.guests, 1);
    provider.close_admission();
    supervisor.close_admission();
    assert!(partition.check_current().is_err());
    // Absent receipt is not proof, and cannot refund after admission closes.
    let mut owner = ParentLaunchOwner {
        grant: prepared.grant.clone(),
        completed: false,
        endpoint_retirement: None,
    };
    assert!(owner.cleanup_endpoint().is_err());
    assert_eq!(
        supervisor.managed_resources().snapshot().unwrap().usage,
        before.usage
    );
    // Test-only disarm of the separate launch owner: no launch was invoked.
    owner.completed = true;
    drop(owner);
    drop(prepared);
    assert!(
        weak.upgrade().is_none(),
        "guard formed a strong cycle with grant"
    );
    assert!(partition.check_current().is_err());
    drop((partition, erased, guard, provider));
    let after = supervisor.managed_resources().snapshot().unwrap();
    assert_eq!(after.usage.guests, 1);
    assert_eq!(after.usage.workers, 1);
    assert_eq!(
        after.usage.cache_disk_bytes,
        mvm_vmm::host::telemetry_provisioning::DEFAULT_RECORDS_BYTE_CAP + 65536
    );
    assert!(supervisor.try_finish().is_err());
    // No guest ran. Original uncertain credits intentionally survive until
    // this isolated test process exits, rather than publishing a false drain.
}
