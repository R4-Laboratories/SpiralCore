#![cfg(all(feature = "native", target_os = "linux"))]
use spiralcore_mvm_canonical_witness::{
    Config,
    native::{PRODUCER_PATH, request},
};
#[test]
fn canonical_client_constructs_only_the_named_transient_mailbox_request() {
    if std::env::var_os("SPIRALCORE_REQUEST_TEST_CHILD").is_some() {
        let home = std::env::var_os("MVM_HOME").expect("isolated child home");
        let config = Config {
            home: home.into(),
            rootfs: "/var/tmp/verified-input.ext4".into(),
            name: "spiralcore-canonical-request".into(),
            mailbox_hex: "ab".repeat(32),
            deadline_seconds: 60,
        };
        let actual = request(&config).unwrap();
        assert_eq!(actual.name(), Some(config.name.as_str()));
        assert_eq!(actual.mode(), mvm_client::LifecycleMode::Transient);
        assert_eq!(actual.command(), [PRODUCER_PATH]);
        assert!(config.home.join("keys").is_dir());
        assert!(
            !config.home.join("vms").exists(),
            "constructor must not boot a VM"
        );
        return;
    }
    let home = std::env::temp_dir().join(format!(
        "spiralcore-current-request-test-{}",
        std::process::id()
    ));
    std::fs::create_dir(&home).unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "canonical_client_constructs_only_the_named_transient_mailbox_request",
        ])
        .env("SPIRALCORE_REQUEST_TEST_CHILD", "1")
        .env("MVM_HOME", &home)
        .status()
        .unwrap();
    std::fs::remove_dir_all(&home).unwrap();
    assert!(status.success());
}
