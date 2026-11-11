#![cfg(all(feature = "native", target_os = "linux"))]
#[test]
fn invalid_preflight_writes_nonempty_private_failure_evidence_without_vm_state() {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let dir = std::env::temp_dir().join(format!("spiralcore-preflight-test-{}", std::process::id()));
    std::fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
    let home = dir.join("home");
    std::fs::DirBuilder::new().mode(0o700).create(&home).unwrap();
    let report = dir.join("report.json");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_spiralcore-mvm-canonical-witness"))
        .args([dir.join("missing-rootfs").as_os_str(), std::ffi::OsStr::new("spiralcore-preflight"), std::ffi::OsStr::new(&"ab".repeat(32)), report.as_os_str()])
        .env("MVM_HOME", &home).output().unwrap();
    assert!(!output.status.success());
    let evidence: serde_json::Value = serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
    assert_eq!(evidence["status"], "failed_preflight");
    assert_eq!(evidence["native_transfer"], "NOT_RUN");
    assert_eq!(std::fs::metadata(&report).unwrap().permissions().mode() & 0o777, 0o600);
    assert!(!home.join("vms").exists());
    std::fs::remove_dir_all(dir).unwrap();
}
