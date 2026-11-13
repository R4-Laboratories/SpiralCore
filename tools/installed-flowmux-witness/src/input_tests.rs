use super::*;
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
use tempfile::tempdir;

#[test]
fn exact_private_inputs_and_changed_records() {
    let root = tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.path().join("issuer");
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let name = path.join("fixture");
    fs::write(&name, b"abc").unwrap();
    fs::set_permissions(&name, fs::Permissions::from_mode(0o600)).unwrap();
    let dir = staged(&path).unwrap();
    assert_eq!(dir.read_private_member("fixture", 3).unwrap(), b"abc");
    assert!(dir.read_private_member("missing", 3).is_err());
    assert!(dir.read_private_member("fixture", 2).is_err());
    let pin = ArchivePin {
        decoded_bytes: 3,
        sha256: hex::encode(Sha256::digest(b"abc")),
    };
    assert!(capture_archive(&path, "fixture", &pin, root.path()).is_ok());
    let wrong = ArchivePin {
        decoded_bytes: 3,
        sha256: "00".repeat(32),
    };
    assert!(capture_archive(&path, "fixture", &wrong, root.path()).is_err());
    let short = ArchivePin {
        decoded_bytes: 2,
        sha256: pin.sha256.clone(),
    };
    assert!(capture_archive(&path, "fixture", &short, root.path()).is_err());
    symlink(&name, path.join("alias")).unwrap();
    assert!(capture_archive(&path, "alias", &pin, root.path()).is_err());
    fs::set_permissions(&name, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(capture_archive(&path, "fixture", &pin, root.path()).is_err());
    assert!(dir.read_private_member("fixture", 3).is_err());
}

#[test]
fn accepts_only_the_declared_input_directory_argument() {
    assert!(parse_inputs(["--inputs", "/operator/private/peer"].map(OsString::from)).is_ok());
    for args in [
        vec![],
        vec!["--inputs"],
        vec!["--inputs", "relative"],
        vec!["--inputs", "/root/../extra"],
        vec!["--inputs", "/root/peer", "extra"],
    ] {
        assert!(parse_inputs(args.into_iter().map(OsString::from)).is_err());
    }
}

#[test]
fn peer_scope_requires_the_configured_route_and_exact_archive() {
    let cfg = r#"{"issuer_id":"fixture","allowed_ip":"192.0.2.1","port":38443,"server_name":"test.example","archive":{"sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","decoded_bytes":3}}"#;
    let profile: Profile = serde_json::from_str(cfg).unwrap();
    assert!(profile.check().is_ok());
    assert!(profile.matches_archive(&"ab".repeat(32), 3).is_err());
    assert!(profile.matches_archive(&"aa".repeat(32), 4).is_err());
    assert!(profile.matches_archive(&"aa".repeat(32), 3).is_ok());
    assert!(
        serde_json::from_str::<Profile>(&cfg.replace("\"issuer_id\"", "\"extra\":0,\"issuer_id\""))
            .is_err()
    );
    assert!(
        serde_json::from_str::<Profile>(&cfg.replace("38443", "0"))
            .unwrap()
            .check()
            .is_err()
    );
    assert!(
        serde_json::from_str::<Profile>(&cfg.replace("38443", "443"))
            .unwrap()
            .check()
            .is_err()
    );
}
