#![cfg(all(feature = "native", target_os = "linux"))]
use spiralcore_mvm_canonical_witness::native::DefinitionGuard;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
#[test]
fn original_definition_fence_rejects_content_change_and_same_bytes_replacement() {
    let root =
        std::env::temp_dir().join(format!("spiralcore-definition-test-{}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let path = root.join("machine.json");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .unwrap();
    file.write_all(b"original").unwrap();
    drop(file);
    let held = DefinitionGuard::capture(&path).unwrap();
    held.verify().unwrap();
    std::fs::write(&path, b"modified").unwrap();
    assert!(held.verify().is_err());
    std::fs::write(&path, b"original").unwrap();
    std::fs::rename(&path, root.join("retained-original.json")).unwrap();
    let mut replacement = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .unwrap();
    replacement.write_all(b"original").unwrap();
    drop(replacement);
    assert!(
        held.verify().is_err(),
        "same bytes cannot authorize removal of another definition inode"
    );
    std::fs::remove_dir_all(root).unwrap();
}
