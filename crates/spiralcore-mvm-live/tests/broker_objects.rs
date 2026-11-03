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

#[test]
fn refused_large_raw_campaign_records_failure_without_claiming_complete_traffic() {
    let root = tempfile::tempdir().unwrap();
    let scratch = root.path().join("refused-cell");
    let mut records = Vec::new();
    assert!(probe::run_cell(&scratch, Arm::RawMixed, 16 << 20, 0, &mut records).is_err());
    let rows = serde_json::Deserializer::from_slice(&records)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 1);
    let failure = &rows[0];
    assert_eq!(failure["event"], "broker_objects_failed");
    assert_eq!(failure["payload_store_preserved"], true);
    assert_eq!(failure["partial_stream_counters_available"], false);
    assert!(failure["actual_both_direction_bytes"].is_null());
    assert!(!failure["sender_error"].is_null());
    assert!(failure["sender_records"].as_array().unwrap().len() > 1);
    assert!(failure["receiver_records"].as_array().unwrap().len() > 1);
    assert!(scratch.exists());
}
