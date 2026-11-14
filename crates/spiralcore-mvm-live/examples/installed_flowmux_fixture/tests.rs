use super::{produce, verify_sidecar};
use std::{fs, net::Ipv4Addr, os::unix::fs::PermissionsExt};

fn refusal(ip: &str, mode: u32, occupied: bool, expected: &str) {
    let private = tempfile::tempdir().unwrap();
    let input = std::env::current_exe().unwrap();
    let parent = private.path().join("packets");
    fs::create_dir(&parent).unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(mode)).unwrap();
    let output = parent.join("fresh");
    if occupied {
        fs::create_dir(&output).unwrap();
        fs::write(output.join("original"), b"retain").unwrap();
    }
    let err = produce(&input, &"0".repeat(64), ip, &output).unwrap_err();
    assert_eq!(err.to_string(), expected);
    if occupied {
        assert_eq!(fs::read(output.join("original")).unwrap(), b"retain");
        assert_eq!(fs::read_dir(&output).unwrap().count(), 1);
    } else {
        assert_eq!(
            fs::symlink_metadata(&output).unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
    }
}

#[test]
fn wrong_staged_elf_digest_refuses_publication() {
    refusal("192.0.2.1", 0o700, false, "staged ELF SHA-256 differs");
}
#[test]
fn noncanonical_ipv4_refuses_publication() {
    refusal("192.0.2.01", 0o700, false, "expected canonical IPv4");
}
#[test]
fn occupied_destination_remains_intact() {
    refusal("192.0.2.1", 0o700, true, "output already exists");
}
#[test]
fn unsafe_parent_refuses_publication() {
    refusal("192.0.2.1", 0o750, false, "output parent must be private");
}
#[test]
fn mismatched_baked_argv_refuses_signing() {
    use mvm_build::builder_vm::GuestSidecar;
    let tree = tempfile::tempdir().unwrap();
    let argv = vec![
        "/usr/local/bin/spiralcore-flowmux-proxy-guest".to_string(),
        "denied.flowmux.test:38443".to_string(),
        format!("{}:38443", Ipv4Addr::new(192, 0, 2, 1)),
    ];
    let mut different = argv.clone();
    different[2] = "192.0.2.2:38443".into();
    GuestSidecar::for_oci_run("flowmux", true, true)
        .with_entrypoint_argv(different)
        .write_to_dir(tree.path())
        .unwrap();
    let result = verify_sidecar(&tree.path().join("mvm-meta.json"), &argv);
    assert_eq!(result.unwrap_err().to_string(), "baked entrypoint differs");
}
