use spiralcore_mvm_live::config::{MailboxBandwidthProfile, Plan, parse_bandwidth_profile};

#[test]
fn legacy_and_explicit_profile_inputs_are_bounded_and_unambiguous() {
    let standard = parse_bandwidth_profile(None).unwrap();
    assert_eq!(standard, MailboxBandwidthProfile::Standard);
    assert_eq!(standard.bytes_per_second(), 4 << 20);
    assert_eq!(standard.burst_bytes(), 8 << 20);
    assert_eq!(parse_bandwidth_profile(Some("standard")).unwrap(), standard);
    let experimental = parse_bandwidth_profile(Some("experimental64")).unwrap();
    assert_eq!(experimental, MailboxBandwidthProfile::Experimental64MiB);
    assert_eq!(experimental.bytes_per_second(), 64 << 20);
    assert_eq!(experimental.burst_bytes(), standard.burst_bytes());
    for value in ["", "unlimited", "0", "64", "experimental128"] {
        assert!(parse_bandwidth_profile(Some(value)).is_err());
    }
    let legacy = serde_json::json!({"stage":"one","arm":"whole","size":4096,"trial":0});
    let plan: Plan = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(plan.bandwidth_profile, standard);
    let mut explicit = legacy;
    explicit["bandwidth_profile"] = serde_json::json!("experimental64");
    let plan: Plan = serde_json::from_value(explicit.clone()).unwrap();
    assert_eq!(plan.bandwidth_profile, experimental);
    assert_eq!(
        serde_json::to_value(plan).unwrap()["bandwidth_profile"],
        "experimental64"
    );
    explicit["bandwidth_profile"] = serde_json::json!("unlimited");
    assert!(serde_json::from_value::<Plan>(explicit).is_err());
}
