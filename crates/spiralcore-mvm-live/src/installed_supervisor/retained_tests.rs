//! Actual filesystem and shared-ledger tests, without VM admission or guests.
use super::*;
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
};

#[test]
fn retained_catalog_preserves_original_debt_after_scope_drop_and_close() {
    mvm_core::util::test_env::isolated_lock_probe(
        probe_retained_catalog_preserves_original_debt_after_scope_drop_and_close,
    );
}

fn probe_retained_catalog_preserves_original_debt_after_scope_drop_and_close() {
    let home = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let config = InstalledConfig::canonical_native_profile();
    let installation = home.path().join("installation");
    let owner = InstallationOwner::initialize(
        &installation,
        crate::installed_owner::InstallationKey {
            installation: [81; 32],
            profile: config.profile_hash(),
        },
    )
    .unwrap();
    let mut supervisor = InstalledSupervisor::start(owner, config).unwrap();
    let inventory = supervisor.retained_telemetry_inventory().unwrap();
    let before = supervisor.managed_resources().snapshot().unwrap();
    let scope = inventory
        .reserve_for_endpoint([82; 32], [83; 32], 7)
        .unwrap();
    let after_reserve = supervisor.managed_resources().snapshot().unwrap();
    assert_eq!(
        after_reserve.usage.cache_disk_bytes,
        before.usage.cache_disk_bytes + (8 << 20) + (32 << 10)
    );
    let vms = home.path().join("vms");
    fs::DirBuilder::new().mode(0o700).create(&vms).unwrap();
    let state = vms.join("retained-fixture");
    fs::DirBuilder::new().mode(0o700).create(&state).unwrap();
    let directory = fs::File::open(&state).unwrap();
    inventory.prepare_state(&scope, &state, &directory).unwrap();
    let catalog = fs::read(installation.join(".retained-telemetry.bin")).unwrap();
    assert_eq!(&catalog[..8], b"SCRETN01");
    let marker = fs::read(state.join(".installed-retained-telemetry")).unwrap();
    assert!(!marker.is_empty());
    // The simulated writer's exact data is outside the disposable namespace.
    fs::write(
        state.join("telemetry-records.jsonl"),
        b"synthetic retained bytes\n",
    )
    .unwrap();
    drop((scope, inventory, directory));
    supervisor.close_admission();
    assert!(supervisor.try_finish().is_err());
    assert_eq!(
        supervisor.managed_resources().snapshot().unwrap().usage,
        after_reserve.usage
    );
    assert_eq!(
        fs::read(state.join("telemetry-records.jsonl")).unwrap(),
        b"synthetic retained bytes\n"
    );
    assert_eq!(
        fs::read(installation.join(".retained-telemetry.bin")).unwrap(),
        catalog
    );
}

#[test]
fn lost_retained_catalog_cannot_be_replaced_by_a_new_scope_append() {
    mvm_core::util::test_env::isolated_lock_probe(probe_lost_catalog);
}

fn probe_lost_catalog() {
    let home = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let config = InstalledConfig::canonical_native_profile();
    let installation = home.path().join("installation");
    let owner = InstallationOwner::initialize(
        &installation,
        crate::installed_owner::InstallationKey {
            installation: [91; 32],
            profile: config.profile_hash(),
        },
    )
    .unwrap();
    let supervisor = InstalledSupervisor::start(owner, config).unwrap();
    let inventory = supervisor.retained_telemetry_inventory().unwrap();
    let vms = home.path().join("vms");
    fs::DirBuilder::new().mode(0o700).create(&vms).unwrap();
    let a = vms.join("scope-a");
    let b = vms.join("scope-b");
    for state in [&a, &b] {
        fs::DirBuilder::new().mode(0o700).create(state).unwrap();
    }
    let first = inventory
        .reserve_for_endpoint([92; 32], [93; 32], 17)
        .unwrap();
    inventory
        .prepare_state(&first, &a, &fs::File::open(&a).unwrap())
        .unwrap();
    let original_marker = fs::read(a.join(".installed-retained-telemetry")).unwrap();
    let catalog = installation.join(".retained-telemetry.bin");
    fs::remove_file(&catalog).unwrap();
    fs::File::open(&installation).unwrap().sync_all().unwrap();
    let second = inventory
        .reserve_for_endpoint([92; 32], [94; 32], 18)
        .unwrap();
    let before = supervisor.managed_resources().snapshot().unwrap().usage;
    assert!(
        inventory
            .prepare_state(&second, &b, &fs::File::open(&b).unwrap())
            .is_err(),
        "missing published catalog was silently replaced with a new scope"
    );
    assert!(
        !catalog.exists(),
        "new catalog forgot the original retained scope"
    );
    assert!(!b.join(".installed-retained-telemetry").exists());
    assert_eq!(
        fs::read(a.join(".installed-retained-telemetry")).unwrap(),
        original_marker
    );
    assert_eq!(
        supervisor.managed_resources().snapshot().unwrap().usage,
        before
    );
    assert!(
        inventory
            .reserve_for_endpoint([92; 32], [95; 32], 19)
            .is_err(),
        "lost durable accounting did not fence new admission"
    );
}

#[test]
fn lost_catalog_restart_helper() {
    let Some(home) = std::env::var_os("SPIRALCORE_RETAINED_RESTART_FIXTURE") else {
        return;
    };
    let home = std::path::PathBuf::from(home);
    let config = InstalledConfig::canonical_native_profile();
    let owner = InstallationOwner::initialize(
        &home.join("installation"),
        crate::installed_owner::InstallationKey {
            installation: [101; 32],
            profile: config.profile_hash(),
        },
    )
    .unwrap();
    let supervisor = InstalledSupervisor::start(owner, config).unwrap();
    let inventory = supervisor.retained_telemetry_inventory().unwrap();
    let vms = home.join("vms");
    fs::DirBuilder::new().mode(0o700).create(&vms).unwrap();
    let state = vms.join("restart-scope");
    fs::DirBuilder::new().mode(0o700).create(&state).unwrap();
    let scope = inventory
        .reserve_for_endpoint([102; 32], [103; 32], 27)
        .unwrap();
    inventory
        .prepare_state(&scope, &state, &fs::File::open(&state).unwrap())
        .unwrap();
    fs::remove_file(home.join("installation/.retained-telemetry.bin")).unwrap();
    fs::File::open(home.join("installation"))
        .unwrap()
        .sync_all()
        .unwrap();
    // Kernel closes the actual original process descriptors; no Drop path
    // may erase data or turn this into positive VM/endpoint retirement.
    std::process::exit(0);
}

#[test]
fn lost_catalog_after_process_exit_cannot_reset_the_epoch() {
    mvm_core::util::test_env::isolated_lock_probe(
        probe_lost_catalog_after_process_exit_cannot_reset_the_epoch,
    );
}

fn probe_lost_catalog_after_process_exit_cannot_reset_the_epoch() {
    let home = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "installed_supervisor::retained_tests::lost_catalog_restart_helper",
            "--test-threads=1",
        ])
        .env("SPIRALCORE_RETAINED_RESTART_FIXTURE", home.path())
        .status()
        .unwrap();
    assert!(status.success(), "actual retention fixture process failed");
    let installation = home.path().join("installation");
    let state = home.path().join("vms/restart-scope");
    let marker = fs::read(state.join(".installed-retained-telemetry")).unwrap();
    let before = fs::read(installation.join("state.bin")).unwrap();
    let config = InstalledConfig::canonical_native_profile();
    let mut owner = InstallationOwner::open(
        &installation,
        crate::installed_owner::InstallationKey {
            installation: [101; 32],
            profile: config.profile_hash(),
        },
    )
    .unwrap();
    assert!(
        owner.reconcile().is_err(),
        "lost catalog allowed an empty retired epoch"
    );
    assert_eq!(fs::read(installation.join("state.bin")).unwrap(), before);
    assert_eq!(
        fs::read(state.join(".installed-retained-telemetry")).unwrap(),
        marker
    );
}

#[test]
fn write_only_records_refuse_before_bound_publication_and_can_retry_readable() {
    mvm_core::util::test_env::isolated_lock_probe(probe_write_only_records);
}

fn probe_write_only_records() {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let home = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let config = InstalledConfig::canonical_native_profile();
    let installation = home.path().join("installation");
    let owner = InstallationOwner::initialize(
        &installation,
        crate::installed_owner::InstallationKey {
            installation: [111; 32],
            profile: config.profile_hash(),
        },
    )
    .unwrap();
    let supervisor = InstalledSupervisor::start(owner, config).unwrap();
    let inventory = supervisor.retained_telemetry_inventory().unwrap();
    let vms = home.path().join("vms");
    fs::DirBuilder::new().mode(0o700).create(&vms).unwrap();
    let state = vms.join("fd-mode-scope");
    fs::DirBuilder::new().mode(0o700).create(&state).unwrap();
    let directory = fs::File::open(&state).unwrap();
    let scope = inventory
        .reserve_for_endpoint([112; 32], [113; 32], 37)
        .unwrap();
    inventory.prepare_state(&scope, &state, &directory).unwrap();
    let record_path = state.join("telemetry-records.jsonl");
    let mut write_only = fs::OpenOptions::new()
        .append(true)
        .create_new(true)
        .mode(0o600)
        .open(&record_path)
        .unwrap();
    write_only.write_all(b"synthetic ciphertext\n").unwrap();
    let key_path = state.join("telemetry-records.key");
    let mut key_writer = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&key_path)
        .unwrap();
    key_writer.write_all(&[b'A'; 80]).unwrap();
    drop(key_writer);
    let key = fs::File::open(&key_path).unwrap();
    let catalog = fs::read(installation.join(".retained-telemetry.bin")).unwrap();
    assert!(
        inventory
            .bind_files(&scope, &state, &directory, &write_only, &key)
            .is_err(),
        "write-only records were bound although sealing needs original read access"
    );
    assert_eq!(
        fs::read(installation.join(".retained-telemetry.bin")).unwrap(),
        catalog
    );
    let readable = fs::OpenOptions::new()
        .read(true)
        .append(true)
        .open(&record_path)
        .unwrap();
    // Protection was prepared while private. Binding must recheck the actual
    // original directory rather than rely on that earlier observation.
    fs::set_permissions(&state, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        inventory
            .bind_files(&scope, &state, &directory, &readable, &key)
            .is_err(),
        "binding accepted a directory that is no longer private"
    );
    assert_eq!(
        fs::read(installation.join(".retained-telemetry.bin")).unwrap(),
        catalog
    );
    fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
    inventory
        .bind_files(&scope, &state, &directory, &readable, &key)
        .unwrap();
    assert_ne!(
        fs::read(installation.join(".retained-telemetry.bin")).unwrap(),
        catalog
    );
    assert_eq!(fs::read(&record_path).unwrap(), b"synthetic ciphertext\n");
}
