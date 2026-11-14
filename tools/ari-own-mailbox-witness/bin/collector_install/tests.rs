use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};

#[test]
fn external_publisher_must_be_explicit_canonical_hex() {
    for value in ["".into(), "f".repeat(63), "A".repeat(64), "g".repeat(64)] {
        assert!(publisher(&value).is_err());
    }
}

#[test]
fn packet_public_key_cannot_override_external_pin() {
    let root = tempfile::tempdir().unwrap();
    let first = ed25519_dalek::SigningKey::from_bytes(&[4; 32]).verifying_key();
    let second = ed25519_dalek::SigningKey::from_bytes(&[5; 32]).verifying_key();
    let path = root.path().join("publisher.pub");
    write_new(&path, first.as_bytes()).unwrap();
    verify_publisher(&path, &first).unwrap();
    assert!(verify_publisher(&path, &second).is_err());
    assert_eq!(read_private(&path, 32).unwrap(), first.as_bytes());
}

#[test]
fn private_metadata_reader_refuses_excess_extent_permissions_and_alias() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("input");
    write_new(&path, b"original").unwrap();
    assert!(read_private(&path, 4).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    assert!(read_private(&path, 32).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let alias = root.path().join("alias");
    symlink(&path, &alias).unwrap();
    assert!(read_private(&alias, 32).is_err());
    assert_eq!(read_private(&path, 32).unwrap(), b"original");
}

#[test]
fn repeated_pinned_publication_preserves_original_and_refuses_change() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("operator");
    publish_equal(&path, b"original", 32).unwrap();
    publish_equal(&path, b"original", 32).unwrap();
    assert!(publish_equal(&path, b"replacement", 32).is_err());
    assert_eq!(read_private(&path, 32).unwrap(), b"original");
}

#[test]
fn already_installed_root_uses_authenticated_archive_not_cached_declarations() {
    use mvm_core::plan::bundle::{BundleArtifact, BundleManifest, InstalledBundle};
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("rootfs.ext4");
    write_new(&path, b"original").unwrap();
    let key = ed25519_dalek::SigningKey::from_bytes(&[6; 32]).verifying_key();
    // This is the authenticated-declaration seam only, not a signed archive
    // fixture. Production supplies the manifest from verify_bundle_file.
    let authenticated = BundleManifest {
        schema_version: 1,
        publisher: "fixture".into(),
        key_id: key_id_from_pubkey(&key),
        arch: "x86_64".into(),
        kernel_version: None,
        profile: None,
        workload_label: None,
        created_at: "2026-11-14T00:00:00Z".into(),
        labels: Default::default(),
        artifacts: vec![BundleArtifact {
            name: "rootfs.ext4".into(),
            role: ArtifactRole::Rootfs,
            path: "rootfs.ext4".into(),
            sha256: sha256_file(&path).unwrap(),
            size_bytes: 8,
        }],
        members: vec![],
        verity: None,
        resources: None,
    };
    let mut installed = InstalledBundle {
        sha256: "0".repeat(64),
        root: root.path().into(),
        manifest: authenticated.clone(),
    };
    authenticated_installed_root(&installed, &authenticated).unwrap();
    fs::write(&path, b"tampered").unwrap();
    assert!(authenticated_installed_root(&installed, &authenticated).is_err());
    installed.manifest.artifacts[0].sha256 = sha256_file(&path).unwrap();
    let rejected = authenticated_installed_root(&installed, &authenticated).unwrap_err();
    assert!(rejected.to_string().contains("cached manifest differs"));
    assert_eq!(fs::read(path).unwrap(), b"tampered");
}
