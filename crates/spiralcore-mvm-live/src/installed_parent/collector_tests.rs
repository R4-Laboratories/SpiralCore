//! Shared-pool policy controls, not booted guests or canonical admission proof.
use super::*;
use mvm_core::host_lifetime::EndpointPartitionLimits;

pub(super) fn limits() -> EndpointPartitionLimits {
    EndpointPartitionLimits {
        host_plaintext_bytes: 0,
        host_codec_bytes: 0,
        host_carrier_bytes: 0,
        host_metadata_bytes: 96 << 20,
        host_stack_bytes: 24 << 20,
        cache_disk_bytes: crate::installed_retained::RECORD_BYTES
            + crate::installed_retained::SIDECAR_BYTES,
        staging_disk_bytes: 0,
        workers: 4,
        max_streams: 1,
    }
}

fn fixture_admit(prepared: &PreparedParentVm, selected: bool) {
    let mut state = prepared.grant.lifecycle.lock().unwrap();
    state.armed = true;
    state.freshness = Some(mvm_core::plan::validity::CheckedFreshness {
        valid_from: "2020-01-01T00:00:00Z".parse().unwrap(),
        valid_until: "2099-01-01T00:00:00Z".parse().unwrap(),
        nonce: mvm_core::plan::Nonce::from_bytes([9; 16]),
    });
    let plan = "private-collector-ledger-fixture-not-canonical-admission".to_owned();
    state.admitted_plan = Some(plan.clone());
    state.admitted_session = Some([72; 32]);
    state.selected_network = None;
    state.selected_collector = selected.then_some(plan);
}

#[test]
fn collector_partition_draws_shared_credit_without_a_codec_store() {
    mvm_core::util::test_env::isolated_lock_probe(probe_collector_partition);
}

fn probe_collector_partition() {
    let (_home, mut supervisor, mut provider) = endpoint_tests::fixture();
    provider
        .configure_telemetry_only(&supervisor, limits())
        .unwrap();
    let prepared = endpoint_tests::grant(&provider, "collector-parent");
    fixture_admit(&prepared, true);
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
    let partition = prepared
        .grant
        .prepare_endpoint_partition(prepared.binding())
        .unwrap();
    assert_eq!(partition.projection().profile, 1);
    assert!(partition.projection().store.is_none());
    partition.check_current().unwrap();
    let after = supervisor.managed_resources().snapshot().unwrap().usage;
    assert_eq!(
        after.host_metadata_bytes - before.host_metadata_bytes,
        limits().host_metadata_bytes
    );
    assert_eq!(
        after.host_stack_bytes - before.host_stack_bytes,
        limits().host_stack_bytes
    );
    assert_eq!(after.workers - before.workers, 4);
    assert_eq!(
        after.cache_disk_bytes - before.cache_disk_bytes,
        limits().cache_disk_bytes
    );
    assert_eq!(after.staging_disk_bytes, before.staging_disk_bytes);
    assert_eq!(after.host_codec_bytes, before.host_codec_bytes);
    assert_eq!(after.host_carrier_bytes, before.host_carrier_bytes);
    assert_eq!(after.host_plaintext_bytes, before.host_plaintext_bytes);
    assert!(
        prepared
            .grant
            .prepare_endpoint_partition(prepared.binding())
            .is_err()
    );
    provider.close_admission();
    supervisor.close_admission();
    assert!(partition.check_current().is_err());
    let mut owner = ParentLaunchOwner {
        grant: prepared.grant.clone(),
        completed: false,
        endpoint_retirement: None,
    };
    assert!(owner.cleanup_endpoint().is_err());
    assert_eq!(
        supervisor.managed_resources().snapshot().unwrap().usage,
        after
    );
    // No VM or endpoint ran. A missing original retirement receipt does not
    // refund the private fixture's deliberately uncertain reservation.
    owner.completed = true;
    drop((owner, prepared, partition, provider));
    assert!(supervisor.try_finish().is_err());
}

#[test]
fn collector_profile_refuses_missing_retained_credit_before_effects() {
    mvm_core::util::test_env::isolated_lock_probe(probe_missing_credit);
}

fn probe_missing_credit() {
    let (_home, mut supervisor, mut provider) = endpoint_tests::fixture();
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
    let mut short = limits();
    short.cache_disk_bytes -= 1;
    assert!(
        provider
            .configure_telemetry_only(&supervisor, short)
            .is_err()
    );
    assert_eq!(
        supervisor.managed_resources().snapshot().unwrap().usage,
        before
    );
    drop(provider);
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}

#[test]
fn unselected_collector_cannot_spend_an_original_parent_partition() {
    mvm_core::util::test_env::isolated_lock_probe(probe_unselected_collector);
}

fn probe_unselected_collector() {
    let (_home, mut supervisor, mut provider) = endpoint_tests::fixture();
    provider
        .configure_telemetry_only(&supervisor, limits())
        .unwrap();
    let prepared = endpoint_tests::grant(&provider, "unselected-collector");
    fixture_admit(&prepared, false);
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
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
        before
    );
    drop((prepared, provider));
    supervisor.close_admission();
    supervisor.try_finish().unwrap();
}

#[test]
fn collector_retirement_credit_requires_original_pool_and_tenant() {
    mvm_core::util::test_env::isolated_lock_probe(probe_retirement_credit);
}

fn probe_retirement_credit() {
    let (_home, mut supervisor, provider) = endpoint_tests::fixture();
    let (_foreign_home, mut foreign, foreign_provider) = endpoint_tests::fixture();
    let claim = ManagedClaim {
        cache_disk_bytes: 1,
        ..Default::default()
    };
    let a = supervisor
        .managed_resources()
        .reserve([121; 32], claim)
        .unwrap();
    let same = supervisor
        .managed_resources()
        .reserve([121; 32], claim)
        .unwrap();
    let other = supervisor
        .managed_resources()
        .reserve([122; 32], claim)
        .unwrap();
    let alien = foreign
        .managed_resources()
        .reserve([121; 32], claim)
        .unwrap();
    a.check_same_tenant(&same).unwrap();
    assert!(a.check_same_tenant(&other).is_err());
    assert!(a.check_same_tenant(&alien).is_err());
    supervisor.close_admission();
    // Cleanup compares retained original ownership without reopening admission.
    a.check_same_tenant(&same).unwrap();
    drop((a, same, other, alien, provider, foreign_provider));
    foreign.close_admission();
    supervisor.try_finish().unwrap();
    foreign.try_finish().unwrap();
}
