#![cfg(feature = "live")]
#[path = "../examples/service_capacity.rs"]
mod probe;

#[test]
fn real_loopback_capacity_probe_preserves_binary_payload_and_counts_both_directions() {
    let row = probe::run_cell(2, true, 4, 0).unwrap();
    assert_eq!(row["configured_endpoints"], 2);
    assert_eq!(row["accepted_calls"], 2);
    assert_eq!(row["accepted_casts"], 2);
    assert_eq!(row["exact_readbacks"], 6);
    assert_eq!(row["fidelity_failures"], 0);
    assert_eq!(row["admission_latency"]["samples"], 4);
    assert_eq!(row["application_reply_latency"]["samples"], 2);
    assert_eq!(row["unverified_admitted_requests"], 0);
    assert_eq!(row["frames_sent"], 12);
    assert!(row["actual_caller_to_receiver_bytes"].as_u64().unwrap() > 8192);
    assert!(row["actual_receiver_to_caller_bytes"].as_u64().unwrap() > 8192);
    assert_eq!(row["sender_receiver_counters_match"], true);
}

#[test]
fn capacity_registration_rejects_unbounded_endpoint_or_work_requests() {
    assert!(probe::run_cell(3, true, 4, 0).is_err());
    assert!(probe::run_cell(512, true, 4, 0).is_err());
    assert!(probe::run_cell(2, true, 100_001, 0).is_err());
    assert!(probe::run_cell(2, true, 4, 120_001).is_err());
}
