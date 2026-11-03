use spiralcore_mvm_live::config::{Arm, parse_arm};

#[path = "../examples/broker_objects.rs"]
mod probe;

#[test]
fn raw_mixed_is_a_raw_baseline_on_the_same_registered_targets() {
    let arm = parse_arm("raw-mixed").unwrap();
    assert_eq!(arm, Arm::RawMixed);
    assert!(arm.is_mixed());
    assert!(!arm.is_combined());
    assert_eq!(serde_json::to_value(arm).unwrap(), "raw-mixed");
}

#[test]
fn mixed_object_probe_refuses_unregistered_work_before_creating_payload_stores() {
    assert!(probe::validate(4096, 0).is_err());
    assert!(probe::validate(1 << 20, 3).is_err());
    assert!(probe::validate(1 << 20, 0).is_ok());
    assert!(probe::validate(16 << 20, 2).is_ok());
}

#[test]
fn raw_mixed_campaign_uses_canonical_tcp_shared_broker_and_reopens_all_targets() {
    let root = tempfile::tempdir().unwrap();
    let mut records = Vec::new();
    let row = probe::run_cell(
        &root.path().join("cell"),
        Arm::RawMixed,
        1 << 20,
        0,
        &mut records,
    )
    .unwrap();
    assert_eq!(row["completed_objects"], 8);
    assert_eq!(row["verified_exact_objects"], 8);
    assert_eq!(row["sender_receiver_counters_match"], true);
    assert_eq!(row["targets"][0]["bytes"], 1_047_202);
    assert_eq!(row["targets"][0]["mode"], "Raw");
    assert_eq!(row["cache_removed_after_verified_record"], true);
    assert!(!root.path().join("cell").exists());
    assert!(!records.is_empty());
}
