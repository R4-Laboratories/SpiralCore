use super::*;
use std::{fs, os::unix::fs::PermissionsExt};

fn refusal(mode: u32, occupied: bool, digest: &str, expected: &str) {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let parent = root.path().join("packets");
    fs::create_dir(&parent).unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(mode)).unwrap();
    let output = parent.join("new");
    if occupied {
        fs::create_dir(&output).unwrap();
        fs::write(output.join("original"), b"retain").unwrap();
    }
    let elf = std::env::current_exe().unwrap();
    let error = produce(&elf, digest, &elf, &"0".repeat(64), &output).unwrap_err();
    assert_eq!(error.to_string(), expected, "{error:#}");
    if occupied {
        assert_eq!(fs::read(output.join("original")).unwrap(), b"retain");
        assert_eq!(fs::read_dir(output).unwrap().count(), 1);
    } else {
        assert!(!output.exists());
    }
}

#[test]
fn wrong_role_digest_refuses_atomic_publication() {
    refusal(0o700, false, &"0".repeat(64), "staged ELF SHA-256 differs");
}
#[test]
fn invalid_digest_refuses_before_staging() {
    refusal(0o700, false, &"F".repeat(64), "expected lowercase SHA-256");
}
#[test]
fn occupied_output_remains_unchanged() {
    refusal(0o700, true, &"0".repeat(64), "output already exists");
}
#[test]
fn unsafe_parent_refuses_publication() {
    refusal(
        0o750,
        false,
        &"0".repeat(64),
        "output parent must be private",
    );
}
#[test]
fn role_environments_have_no_effect_capabilities_and_match_serial_workers() {
    for role in [Role::Source, Role::Destination] {
        let env = package::environment(role).unwrap();
        assert_eq!(env.entrypoint.argv, [role.guest()]);
        assert_eq!(env.entrypoint.working_dir, None);
        assert!(
            env.capabilities.is_empty()
                && env.services.is_empty()
                && env.secret_slots.is_empty()
                && env.volumes.is_empty()
        );
        assert!(env.policy.as_ref().unwrap().allow_hosts.is_empty());
        assert_eq!(env.resources.as_ref().unwrap().cpu_cores, 1);
        assert_eq!(env.resources.as_ref().unwrap().memory_mb, 256);
        let runtime = package::runtime(role);
        assert_eq!(runtime.source_path, env.entrypoint.argv[0]);
        assert_eq!(runtime.module, role.module());
        use mvm_agentd::runtime_config::{ConcurrencyConfig, InProcessMode};
        let Some(ConcurrencyConfig::WarmProcess(worker)) = runtime.concurrency else {
            panic!("serial warm role worker missing");
        };
        assert_eq!(worker.max_calls_per_worker, 2);
        assert_eq!(worker.pool_size, 1);
        assert_eq!(worker.max_rss_mb, 64);
        assert_eq!(worker.max_queue_depth, Some(1));
        assert_eq!(worker.in_process, InProcessMode::Serial);
    }
}

#[test]
fn mismatched_role_sidecar_refuses_verification() {
    let root = tempfile::tempdir().unwrap();
    let argv = vec![Role::Source.guest().into()];
    mvm_build::builder_vm::GuestSidecar::for_oci_run(Role::Destination.machine(), true, true)
        .with_entrypoint_argv(vec![Role::Destination.guest().into()])
        .write_to_dir(root.path())
        .unwrap();
    let result = package::verify_sidecar(&root.path().join("mvm-meta.json"), &argv, Role::Source);
    assert_eq!(result.unwrap_err().to_string(), "baked entrypoint differs");
}
