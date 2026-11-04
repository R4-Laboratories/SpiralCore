use installed_supervisor_harness::{
    installed_disk::EndpointDiskRequest,
    installed_owner::{InstallationKey, InstallationOwner},
    installed_supervisor::{InstalledConfig, InstalledSupervisor},
};
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    sync::atomic::{AtomicU64, Ordering},
};

fn fixture() -> (std::path::PathBuf, InstalledSupervisor) {
    fixture_profile(InstalledConfig::canonical_native_profile())
}
fn fixture_profile(cfg: InstalledConfig) -> (std::path::PathBuf, InstalledSupervisor) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let base = std::env::temp_dir().join(format!(
        "spiral-disk-inventory-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
    let owner = InstallationOwner::initialize(
        &base.join("installation"),
        InstallationKey {
            installation: [98; 32],
            profile: cfg.profile_hash(),
        },
    )
    .unwrap();
    (base, InstalledSupervisor::start(owner, cfg).unwrap())
}
fn request() -> EndpointDiskRequest {
    EndpointDiskRequest {
        tenant: [1; 32],
        session: [2; 32],
        parent_serial: 1,
        cache_bytes: 1 << 20,
        staging_bytes: 1 << 20,
    }
}

#[test]
fn prepared_disposable_store_is_durable_and_unused_cancel_returns_actual_credit() {
    let (base, mut controller) = fixture();
    let disks = controller.endpoint_disk_inventory().unwrap();
    let store = disks.prepare(request()).unwrap();
    let path = store.path().to_owned();
    assert!(path.is_dir());
    assert!(base.join("installation/.endpoint-disks.bin").is_file());
    assert_eq!(
        controller
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .cache_disk_bytes,
        1 << 20
    );
    store.cancel_unused().unwrap();
    assert!(!path.exists());
    assert!(!base.join("installation/.endpoint-disks.bin").exists());
    assert_eq!(
        controller
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .cache_disk_bytes,
        0
    );
    drop(disks);
    controller.close_admission();
    controller.try_finish().unwrap();
    drop(controller);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn disk_quota_refusal_happens_before_any_metadata_or_directory_creation() {
    let (base, mut controller) = fixture();
    let disks = controller.endpoint_disk_inventory().unwrap();
    let before = fs::read_dir(base.join("installation")).unwrap().count();
    assert!(
        disks
            .prepare(EndpointDiskRequest {
                cache_bytes: u64::MAX,
                ..request()
            })
            .is_err()
    );
    assert_eq!(
        fs::read_dir(base.join("installation")).unwrap().count(),
        before
    );
    assert_eq!(
        controller
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .cache_disk_bytes,
        0
    );
    drop(disks);
    controller.close_admission();
    controller.try_finish().unwrap();
    drop(controller);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn duplicate_before_effects_refusal_does_not_burn_a_second_disk_reservation() {
    let (base, mut controller) = fixture();
    let disks = controller.endpoint_disk_inventory().unwrap();
    let first = disks.prepare(request()).unwrap();
    let before = controller.managed_resources().snapshot().unwrap();
    assert!(disks.prepare(request()).is_err());
    let after = controller.managed_resources().snapshot().unwrap();
    assert_eq!(before.usage, after.usage);
    assert_eq!(before.active_reservations, after.active_reservations);
    first.cancel_unused().unwrap();
    drop(disks);
    controller.close_admission();
    controller.try_finish().unwrap();
    drop(controller);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn prepared_store_keeps_shared_cleanup_metadata_credit_after_inventory_drop() {
    let (base, mut controller) = fixture();
    let disks = controller.endpoint_disk_inventory().unwrap();
    let store = disks.prepare(request()).unwrap();
    let prepared = controller.managed_resources().snapshot().unwrap();
    drop(disks);
    let held = controller.managed_resources().snapshot().unwrap();
    assert_eq!(
        prepared.usage, held.usage,
        "store cleanup still needs its actual shared workspace"
    );
    assert_eq!(prepared.active_reservations, held.active_reservations);
    controller.close_admission();
    store.cancel_unused().unwrap();
    assert_eq!(
        controller
            .managed_resources()
            .snapshot()
            .unwrap()
            .usage
            .host_metadata_bytes,
        0
    );
    controller.try_finish().unwrap();
    drop(controller);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn legacy_profile_cannot_enable_new_disk_contract_or_spend_metadata_credit() {
    let (base, mut controller) = fixture_profile(InstalledConfig::native_profile());
    let before = controller.managed_resources().snapshot().unwrap();
    assert!(controller.endpoint_disk_inventory().is_err());
    let after = controller.managed_resources().snapshot().unwrap();
    assert_eq!(before.usage, after.usage);
    controller.close_admission();
    controller.try_finish().unwrap();
    drop(controller);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn canonical_profile_is_explicit_and_legacy_hash_remains_byte_identical() {
    let old = InstalledConfig::native_profile().profile_hash();
    assert_eq!(
        old,
        [
            0x9d, 0x1e, 0xc5, 0x05, 0x76, 0xed, 0xe2, 0x3e, 0x56, 0x80, 0xbb, 0xeb, 0xef, 0xe8,
            0x99, 0xb9, 0xea, 0x96, 0xda, 0xbc, 0x6c, 0xf4, 0xa3, 0x89, 0x9b, 0xcd, 0x9c, 0x6d,
            0x8a, 0xfc, 0xf8, 0x2b
        ]
    );
    let new = InstalledConfig::canonical_native_profile().profile_hash();
    assert_ne!(old, new);
    assert_eq!(
        new,
        InstalledConfig::canonical_native_profile().profile_hash()
    );
}

#[test]
fn crash_store_helper() {
    use std::{
        io::{Read, Write},
        os::unix::fs::OpenOptionsExt,
    };
    let Some(base) = std::env::var_os("SPIRAL_DISK_INVENTORY_HELPER") else {
        return;
    };
    let base = std::path::PathBuf::from(base);
    let cfg = InstalledConfig::canonical_native_profile();
    let owner = InstallationOwner::initialize(
        &base.join("installation"),
        InstallationKey {
            installation: [98; 32],
            profile: cfg.profile_hash(),
        },
    )
    .unwrap();
    let controller = InstalledSupervisor::start(owner, cfg).unwrap();
    let inventory = controller.endpoint_disk_inventory().unwrap();
    let store = inventory.prepare(request()).unwrap();
    let mut data = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(store.path().join("synthetic-cache"))
        .unwrap();
    data.write_all(b"exact synthetic cache bytes").unwrap();
    data.sync_all().unwrap();
    println!("DISK_READY");
    std::io::stdout().flush().unwrap();
    let mut control = [0];
    let _ = std::io::stdin().read_exact(&mut control);
    loop {
        std::thread::park();
    }
}

struct TestChild(std::process::Child, Option<std::thread::JoinHandle<()>>);
impl Drop for TestChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
        if let Some(reader) = self.1.take() {
            reader.join().unwrap();
        }
    }
}
fn ready_child(base: &std::path::Path, helper: &str, variable: &str, marker: &str) -> TestChild {
    use std::{
        io::{BufRead, BufReader},
        process::{Command, Stdio},
        sync::mpsc,
        time::Duration,
    };
    let mut child = TestChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", helper, "--nocapture"])
            .env(variable, base)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
        None,
    );
    let stdout = child.0.stdout.take().unwrap();
    let marker = marker.to_owned();
    let (tx, rx) = mpsc::sync_channel(1);
    let reader = std::thread::spawn(move || {
        let mut output = BufReader::new(stdout);
        let mut line = String::new();
        let ready = loop {
            line.clear();
            if output.read_line(&mut line).unwrap_or(0) == 0 {
                break false;
            }
            if line.trim() == marker {
                break true;
            }
        };
        let _ = tx.send(ready);
        // Keep the original pipe open through the harness final result.
        let _ = std::io::copy(&mut output, &mut std::io::sink());
    });
    let ready = rx.recv_timeout(Duration::from_secs(10));
    if !matches!(ready, Ok(true)) {
        child.0.kill().unwrap();
        child.0.wait().unwrap();
        reader.join().unwrap();
        panic!("bounded original child readiness failed: {ready:?}");
    }
    child.1 = Some(reader);
    child
}
fn crash_creator() -> (std::path::PathBuf, InstallationKey, TestChild) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let base = std::env::temp_dir().join(format!(
        "spiral-disk-crash-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
    let child = ready_child(
        &base,
        "crash_store_helper",
        "SPIRAL_DISK_INVENTORY_HELPER",
        "DISK_READY",
    );
    let key = InstallationKey {
        installation: [98; 32],
        profile: InstalledConfig::canonical_native_profile().profile_hash(),
    };
    (base, key, child)
}
fn crash_fixture() -> (std::path::PathBuf, InstallationKey) {
    let (base, key, mut child) = crash_creator();
    child.0.kill().unwrap();
    assert!(!child.0.wait().unwrap().success());
    (base, key)
}

#[test]
fn original_process_crash_is_recovered_without_deleting_unrelated_records() {
    let (base, key) = crash_fixture();
    let root = base.join("installation");
    fs::write(base.join("unrelated-audit-record"), b"retained audit").unwrap();
    let mut owner = InstallationOwner::open(&root, key).unwrap();
    assert!(owner.reconcile().is_err());
    assert_eq!(owner.recover_endpoint_disks().unwrap(), 1);
    assert_eq!(
        fs::read(base.join("unrelated-audit-record")).unwrap(),
        b"retained audit"
    );
    owner.reconcile().unwrap();
    let epoch = owner.begin().unwrap();
    owner.finish(epoch).unwrap();
    drop(owner);
    fs::remove_dir_all(base).unwrap();
}

fn cache_path(root: &std::path::Path) -> std::path::PathBuf {
    fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(".endpoint-cache-v1-")
        })
        .unwrap()
}

#[test]
fn missing_inventory_cannot_retire_an_orphan_namespace() {
    let (base, key) = crash_fixture();
    let root = base.join("installation");
    let cache = cache_path(&root);
    let inventory = fs::read(root.join(".endpoint-disks.bin")).unwrap();
    fs::remove_file(root.join(".endpoint-disks.bin")).unwrap();
    let mut owner = InstallationOwner::open(&root, key).unwrap();
    assert!(
        owner.reconcile().is_err(),
        "orphan namespace must fence generic retirement"
    );
    assert!(owner.recover_endpoint_disks().is_err());
    assert_eq!(
        fs::read(cache.join("synthetic-cache")).unwrap(),
        b"exact synthetic cache bytes"
    );
    // Restore the original test-owned metadata for explicit cleanup.
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(root.join(".endpoint-disks.bin"))
        .unwrap();
    file.write_all(&inventory).unwrap();
    file.sync_all().unwrap();
    assert_eq!(owner.recover_endpoint_disks().unwrap(), 1);
    owner.reconcile().unwrap();
    drop(owner);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn creation_intent_with_present_unproven_directory_preserves_its_contents() {
    use sha2::{Digest, Sha256};
    let (base, key) = crash_fixture();
    let root = base.join("installation");
    let cache = cache_path(&root);
    let path = root.join(".endpoint-disks.bin");
    let original = fs::read(&path).unwrap();
    let mut intent = original.clone();
    // A canonical creation intent deliberately has no captured inode identity.
    intent[84 + 96] = 1;
    intent[84 + 104..84 + 120].fill(0);
    let end = intent.len() - 32;
    let mut hash = Sha256::new();
    hash.update(b"spiralcore/disposable-endpoint-inventory/v1\0");
    hash.update(&intent[..end]);
    intent[end..].copy_from_slice(&hash.finalize());
    fs::write(&path, &intent).unwrap();
    let mut owner = InstallationOwner::open(&root, key).unwrap();
    assert!(
        owner.recover_endpoint_disks().is_err(),
        "name alone cannot authorize deletion"
    );
    assert_eq!(fs::read(&path).unwrap(), intent);
    assert_eq!(
        fs::read(cache.join("synthetic-cache")).unwrap(),
        b"exact synthetic cache bytes"
    );
    fs::write(&path, original).unwrap();
    assert_eq!(owner.recover_endpoint_disks().unwrap(), 1);
    owner.reconcile().unwrap();
    drop(owner);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn surviving_holder_helper() {
    use installed_supervisor_harness::installed_owner::HolderLease;
    use std::io::{Read, Write};
    let Some(base) = std::env::var_os("SPIRAL_DISK_HOLDER_HELPER") else {
        return;
    };
    let root = std::path::PathBuf::from(base).join("installation");
    let key = InstallationKey {
        installation: [98; 32],
        profile: InstalledConfig::canonical_native_profile().profile_hash(),
    };
    let lease = HolderLease::acquire(&root, key, 1).unwrap();
    println!("HOLDER_READY");
    std::io::stdout().flush().unwrap();
    let mut control = [0];
    std::io::stdin().read_exact(&mut control).unwrap();
    drop(lease);
}

#[test]
fn surviving_original_holder_blocks_disk_cleanup_after_creator_crash() {
    use installed_supervisor_harness::installed_owner::OwnerError;
    use std::{
        io::Write,
        time::{Duration, Instant},
    };
    let (base, key, mut creator) = crash_creator();
    let mut holder = ready_child(
        &base,
        "surviving_holder_helper",
        "SPIRAL_DISK_HOLDER_HELPER",
        "HOLDER_READY",
    );
    creator.0.kill().unwrap();
    assert!(!creator.0.wait().unwrap().success());
    let root = base.join("installation");
    let before = fs::read(root.join(".endpoint-disks.bin")).unwrap();
    let mut owner = InstallationOwner::open(&root, key).unwrap();
    assert!(matches!(
        owner.recover_endpoint_disks(),
        Err(OwnerError::Busy)
    ));
    assert_eq!(fs::read(root.join(".endpoint-disks.bin")).unwrap(), before);
    holder.0.stdin.take().unwrap().write_all(b"x").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = holder.0.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < deadline, "original holder did not exit");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(owner.recover_endpoint_disks().unwrap(), 1);
    owner.reconcile().unwrap();
    drop(owner);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn substituted_directory_never_authorizes_deleting_replacement_data() {
    let (base, key) = crash_fixture();
    let root = base.join("installation");
    let cache = cache_path(&root);
    let original = base.join("held-original");
    fs::rename(&cache, &original).unwrap();
    fs::DirBuilder::new().mode(0o700).create(&cache).unwrap();
    fs::write(cache.join("foreign"), b"preserve replacement").unwrap();
    let before = fs::read(root.join(".endpoint-disks.bin")).unwrap();
    let mut owner = InstallationOwner::open(&root, key).unwrap();
    assert!(owner.recover_endpoint_disks().is_err());
    assert_eq!(fs::read(root.join(".endpoint-disks.bin")).unwrap(), before);
    assert_eq!(
        fs::read(cache.join("foreign")).unwrap(),
        b"preserve replacement"
    );
    fs::remove_file(cache.join("foreign")).unwrap();
    fs::remove_dir(&cache).unwrap();
    fs::rename(original, cache).unwrap();
    assert_eq!(owner.recover_endpoint_disks().unwrap(), 1);
    owner.reconcile().unwrap();
    drop(owner);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn links_inside_disposable_cache_cannot_delete_external_records() {
    use std::{
        io::Write,
        os::unix::fs::{OpenOptionsExt, symlink},
    };
    for hard in [false, true] {
        let (base, key) = crash_fixture();
        let root = base.join("installation");
        let cache = cache_path(&root);
        let external = base.join("audit-retained");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&external)
            .unwrap();
        file.write_all(b"must remain exact").unwrap();
        file.sync_all().unwrap();
        let link = cache.join("link");
        if hard {
            fs::hard_link(&external, &link).unwrap();
        } else {
            symlink(&external, &link).unwrap();
        }
        let mut owner = InstallationOwner::open(&root, key).unwrap();
        assert!(owner.recover_endpoint_disks().is_err());
        assert_eq!(fs::read(&external).unwrap(), b"must remain exact");
        assert!(owner.reconcile().is_err());
        fs::remove_file(link).unwrap();
        assert_eq!(owner.recover_endpoint_disks().unwrap(), 1);
        owner.reconcile().unwrap();
        drop(owner);
        fs::remove_dir_all(base).unwrap();
    }
}

#[test]
fn malformed_or_foreign_snapshots_never_authorize_cleanup() {
    use sha2::{Digest, Sha256};
    let (base, key) = crash_fixture();
    let root = base.join("installation");
    let cache = cache_path(&root);
    let path = root.join(".endpoint-disks.bin");
    let original = fs::read(&path).unwrap();
    let mut variants = Vec::new();
    for offset in [0, 8, 40, 72, 82, 84 + 97, 84 + 104] {
        let mut bytes = original.clone();
        bytes[offset] ^= 1;
        variants.push(bytes);
    }
    let mut duplicate = original[..original.len() - 32].to_vec();
    duplicate[80..82].copy_from_slice(&2_u16.to_le_bytes());
    duplicate.extend_from_slice(&original[84..84 + 120]);
    duplicate.extend_from_slice(&[0; 32]);
    variants.push(duplicate);
    let mut overflow = original.clone();
    overflow[84 + 80..84 + 88].copy_from_slice(&u64::MAX.to_le_bytes());
    variants.push(overflow);
    let mut owner = InstallationOwner::open(&root, key).unwrap();
    for mut bytes in variants {
        // Recompute the public checksum to exercise semantic validation too.
        let end = bytes.len() - 32;
        let mut hash = Sha256::new();
        hash.update(b"spiralcore/disposable-endpoint-inventory/v1\0");
        hash.update(&bytes[..end]);
        bytes[end..].copy_from_slice(&hash.finalize());
        fs::write(&path, &bytes).unwrap();
        assert!(owner.recover_endpoint_disks().is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(
            fs::read(cache.join("synthetic-cache")).unwrap(),
            b"exact synthetic cache bytes"
        );
    }
    for size in [0, 7, 83, original.len() - 1] {
        fs::write(&path, &original[..size]).unwrap();
        assert!(owner.recover_endpoint_disks().is_err());
    }
    fs::write(&path, original).unwrap();
    assert_eq!(owner.recover_endpoint_disks().unwrap(), 1);
    owner.reconcile().unwrap();
    drop(owner);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn excessive_directory_depth_fences_retirement_and_can_resume_after_repair() {
    let (base, key) = crash_fixture();
    let root = base.join("installation");
    let cache = cache_path(&root);
    let mut nested = cache.clone();
    for _ in 0..4 {
        nested = nested.join("nested");
        fs::DirBuilder::new().mode(0o700).create(&nested).unwrap();
    }
    let mut owner = InstallationOwner::open(&root, key).unwrap();
    assert!(owner.recover_endpoint_disks().is_err());
    assert!(owner.reconcile().is_err());
    assert!(nested.is_dir());
    // Only remove this test's deepest empty directory; recovery owns the rest.
    fs::remove_dir(nested).unwrap();
    assert_eq!(owner.recover_endpoint_disks().unwrap(), 1);
    owner.reconcile().unwrap();
    drop(owner);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn store_projection_matches_its_original_inventory_and_actual_directory() {
    use std::os::unix::fs::MetadataExt;
    let (base, mut controller) = fixture();
    let disks = controller.endpoint_disk_inventory().unwrap();
    let store = disks.prepare(request()).unwrap();
    let before = controller.managed_resources().snapshot().unwrap();
    let binding = store.binding();
    let bytes = fs::read(base.join("installation/.endpoint-disks.bin")).unwrap();
    let metadata = fs::symlink_metadata(binding.path).unwrap();
    assert_eq!(binding.key.installation, [98; 32]);
    assert_eq!(
        binding.key.profile,
        InstalledConfig::canonical_native_profile().profile_hash()
    );
    assert_eq!(binding.generation, 1);
    assert_eq!(binding.tenant, request().tenant);
    assert_eq!(binding.session, request().session);
    assert_eq!(binding.parent_serial, request().parent_serial);
    assert_eq!(
        binding.reservation_serial,
        u64::from_le_bytes(bytes[84 + 72..84 + 80].try_into().unwrap())
    );
    assert_ne!(binding.reservation_serial, 0);
    assert_eq!(binding.device, metadata.dev());
    assert_eq!(binding.inode, metadata.ino());
    assert_eq!(binding.path, store.path());
    assert_eq!(
        controller.managed_resources().snapshot().unwrap().usage,
        before.usage
    );
    store.cancel_unused().unwrap();
    drop(disks);
    controller.close_admission();
    controller.try_finish().unwrap();
    drop(controller);
    fs::remove_dir_all(base).unwrap();
}
