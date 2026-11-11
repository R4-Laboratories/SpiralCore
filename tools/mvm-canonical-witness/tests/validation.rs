use spiralcore_mvm_canonical_witness::{Config, ProcessSample, expected_body};
use std::path::PathBuf;
fn config() -> Config {
    Config {
        home: PathBuf::from("/var/tmp/witness"),
        rootfs: PathBuf::from("/var/tmp/input.ext4"),
        name: "spiralcore-canonical-mailbox-one".into(),
        mailbox_hex: "ab".repeat(32),
        deadline_seconds: 60,
    }
}
#[test]
fn only_bounded_isolated_native_inputs_are_admitted() {
    assert!(config().validate().is_ok());
    for bad in ["", "../other", "a/b"] {
        let mut c = config();
        c.name = bad.into();
        assert!(c.validate().is_err());
    }
    let mut c = config();
    c.home = PathBuf::from("relative");
    assert!(c.validate().is_err());
    let mut c = config();
    c.deadline_seconds = 301;
    assert!(c.validate().is_err());
    let mut c = config();
    c.mailbox_hex = "not hex".into();
    assert!(c.validate().is_err());
}
#[test]
fn proc_stat_parsing_binds_start_identity_and_never_invents_cpu() {
    let mut fields = vec!["0"; 22];
    fields[0] = "S";
    fields[11] = "17";
    fields[12] = "9";
    fields[19] = "12345";
    let raw = format!("42 (a name ) with spaces) {}", fields.join(" "));
    let s = ProcessSample::parse(42, &raw).unwrap();
    assert_eq!(s.start_ticks, 12345);
    assert_eq!(s.user_ticks, 17);
    assert_eq!(s.system_ticks, 9);
    assert!(ProcessSample::parse(43, &raw).is_err());
    assert!(ProcessSample::parse(42, "42 (broken) S").is_err());
}
#[test]
fn fixture_is_exact_nonempty_binary_with_zeros_within_default_grant() {
    let body = expected_body();
    assert_eq!(body.len(), 1024);
    assert!(body.contains(&0));
    assert_eq!(body, expected_body());
    assert_ne!(body[0], body[1]);
}
