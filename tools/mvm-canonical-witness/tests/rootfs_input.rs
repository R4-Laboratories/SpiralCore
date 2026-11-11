#![cfg(all(feature = "native", target_os = "linux"))]
#[test]
fn canonical_input_accepts_private_owned_directory_and_refuses_alias_or_public_directory() {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt, symlink};
    let root = std::env::temp_dir().join(format!(
        "spiralcore-rootfs-input-test-{}",
        std::process::id()
    ));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let directory = root.join("rootfs");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&directory)
        .unwrap();
    let file = root.join("rootfs.ext4");
    std::fs::write(&file, b"format verification belongs to the canonical SDK").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    let alias = root.join("alias");
    symlink(&directory, &alias).unwrap();
    let check = spiralcore_mvm_canonical_witness::native::validate_rootfs_input;
    let result = (check(&directory), check(&file), check(&alias));
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o777)).unwrap();
    let public = check(&directory);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o666)).unwrap();
    let writable_image = check(&file);
    std::fs::remove_dir_all(&root).unwrap();
    assert!(
        result.0.is_ok(),
        "private source tree should use canonical SDK materialization"
    );
    assert!(result.1.is_ok());
    assert!(result.2.is_err());
    assert!(public.is_err());
    assert!(
        writable_image.is_err(),
        "a writable image input must be refused"
    );
}
