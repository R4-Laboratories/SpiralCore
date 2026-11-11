#![cfg(all(feature = "native", target_os = "linux"))]

use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

#[test]
fn preparation_refuses_a_dangling_vm_namespace_before_resolving_sources() {
    let root = std::env::temp_dir().join(format!("spiralcore-runtime-namespace-{}", std::process::id()));
    std::fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    let home = root.join("home");
    std::fs::DirBuilder::new().mode(0o700).create(&home).unwrap();
    std::os::unix::fs::symlink(root.join("absent"), home.join("vms")).unwrap();
    let source = root.join("source");
    std::fs::DirBuilder::new().mode(0o700).create(&source).unwrap();
    let report = root.join("preparation.json");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_spiralcore-mvm-prepare-runtime"))
        .args([source.as_os_str(), home.as_os_str(), report.as_os_str()])
        .env("MVM_HOME", &home).output().unwrap();
    assert!(!output.status.success());
    let record: serde_json::Value = serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
    assert_eq!(record["status"], "failed_preparation");
    assert!(record["error"].as_str().unwrap().contains("fresh VM namespace"));
    assert!(!home.join("cache").exists());
    assert!(std::fs::symlink_metadata(home.join("vms")).unwrap().file_type().is_symlink());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn preparation_refuses_unprotected_home_without_building_or_creating_vm_state() {
    let root = std::env::temp_dir().join(format!(
        "spiralcore-runtime-preparation-{}",
        std::process::id()
    ));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let home = root.join("home");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&home)
        .unwrap();
    std::fs::set_permissions(&home, std::fs::Permissions::from_mode(0o755)).unwrap();
    let source = root.join("source");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&source)
        .unwrap();
    let report = root.join("preparation.json");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_spiralcore-mvm-prepare-runtime"))
        .args([source.as_os_str(), home.as_os_str(), report.as_os_str()])
        .env("MVM_HOME", &home)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
    assert_eq!(record["status"], "failed_preparation");
    assert_eq!(record["native_transfer"], "NOT_RUN");
    assert!(record["error"].as_str().unwrap().contains("0700"));
    assert_eq!(
        std::fs::metadata(&report).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(!home.join("cache").exists());
    assert!(!home.join("vms").exists());
    std::fs::remove_dir_all(root).unwrap();
}
