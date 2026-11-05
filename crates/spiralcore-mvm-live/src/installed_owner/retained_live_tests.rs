//! Original funded historical-table custody. These are metadata/accounting
//! fixtures, never native profile, old grant or EndpointRetirement evidence.
use super::*;

#[test]
#[ignore = "protected original process + actual closed table/file custody"]
fn live_inventory_preparation_moves_historical_table_without_funding_again() {
    let home = super::tests::sealed_format_filesystem_fixture();
    let committed = super::tests::review_prepare(home.path())
        .commit_closed()
        .unwrap();
    let original = committed.snapshot().unwrap().managed.usage;
    let custody = committed.original_custody_identity();
    let prepared = committed.prepare_live_inventory().unwrap();
    assert_eq!(prepared.original_custody_identity(), custody);
    let snapshot = prepared.snapshot().unwrap();
    assert_eq!(snapshot.historical_rows, 1);
    assert_eq!(snapshot.new_rows, 0);
    assert_eq!(snapshot.remaining_slots, 15);
    assert_eq!(snapshot.managed.usage, original);
    assert_eq!(snapshot.managed.usage.host_metadata_bytes, 1 << 20);
    assert!(snapshot.managed.fenced);
    prepared.check_current().unwrap();
}

#[test]
#[ignore = "actual closed historical held-set fault; no grant or native admission"]
fn live_inventory_preparation_refuses_changed_historical_membership() {
    let home = sixteen_original_history_filesystem_fixture();
    let committed = super::tests::review_prepare(home.path())
        .commit_closed()
        .unwrap();
    let binding = committed.binding().unwrap();
    let epoch = Epoch {
        key: binding.key,
        generation: binding.generation,
    };
    let prepared = committed.prepare_live_inventory().unwrap();
    let custody = prepared.original_custody_identity();
    let path = home.path().join("installation").join(CATALOG);
    let original_bytes = fs::read(&path).unwrap();
    let mut catalog = CatalogV2::decode(&original_bytes, epoch).unwrap();
    assert_eq!(catalog.entries.len(), 16);
    catalog.entries.pop();
    // Preserve a valid format/checksum while removing an actually funded row.
    // This tests held-set equality, not merely malformed-input rejection.
    let subset = catalog.encode().unwrap();
    assert_eq!(CatalogV2::decode(&subset, epoch).unwrap().entries.len(), 15);
    fs::write(&path, &subset).unwrap();
    assert!(prepared.check_current().is_err());
    assert!(prepared.snapshot().is_err());
    assert_eq!(fs::read(&path).unwrap(), subset);
    assert_eq!(prepared.original_custody_identity(), custody);
    fs::write(&path, original_bytes).unwrap();
    assert!(prepared.check_current().is_err());
}

fn sixteen_original_history_filesystem_fixture() -> tempfile::TempDir {
    use std::{
        io::Write,
        os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    };
    let home = super::tests::sealed_format_filesystem_fixture();
    let compatibility = RetainedRecoveryCompatibility::published_v1().unwrap();
    let key = InstallationKey {
        installation: [121; 32],
        profile: compatibility.profile,
    };
    let owner = InstallationOwner::open(&home.path().join("installation"), key).unwrap();
    let _holders = owner.holders_for_transition().unwrap();
    let current = owner.current().unwrap();
    let epoch = Epoch {
        key: current.key,
        generation: current.generation,
    };
    let mut catalog = owner.storage.retained_catalog(epoch).unwrap();
    let template = catalog.entries[0].clone();
    let parent = home.path().join("vms");
    let protection =
        RuntimeStateProtection::acquire(&parent, &File::open(&parent).unwrap()).unwrap();
    for at in 1..MAX_ENTRIES {
        // Private format/accounting fixture only: no native receipt or grant
        // is manufactured. Recovery must conservatively fund EVERY row.
        let path = parent.join(format!("historical-{at}"));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        let directory = File::open(&path).unwrap();
        protection.check_state(&path, &directory).unwrap();
        let mut row = template.clone();
        row.state = path;
        row.session = [124 + at as u8; 32];
        row.serial = 1000 + at as u64; // deliberately not imported ledger serial
        let id = object_id(&directory).unwrap();
        row.state_device = id.device;
        row.state_inode = id.inode;
        let mut records = File::options()
            .read(true)
            .append(true)
            .create_new(true)
            .mode(0o600)
            .open(row.state.join(TELEMETRY_RECORDS_FILE))
            .unwrap();
        let payload = format!("synthetic historical ciphertext row {at}\n");
        records.write_all(payload.as_bytes()).unwrap();
        records.sync_all().unwrap();
        let record_id = object_id(&records).unwrap();
        row.record_id = (record_id.device, record_id.inode);
        row.sealed_len = payload.len() as u64;
        row.sealed_sha256 = Sha256::digest(payload.as_bytes()).into();
        let mut wrapped = File::options()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(row.state.join(TELEMETRY_RECORDS_KEY_FILE))
            .unwrap();
        wrapped.write_all(&[b'A'; 80]).unwrap();
        wrapped.sync_all().unwrap();
        let key_id = object_id(&wrapped).unwrap();
        row.key_id = (key_id.device, key_id.inode);
        let mut marker = File::options()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(row.state.join(RETAINED_TELEMETRY_MARKER_FILE))
            .unwrap();
        marker.write_all(&row.marker(epoch)).unwrap();
        marker.sync_all().unwrap();
        directory.sync_all().unwrap();
        catalog.entries.push(row);
    }
    owner.storage.publish_retained(&catalog).unwrap();
    home
}

#[test]
#[ignore = "sixteen original filesystem row fixtures; resource custody, not native fleet capacity"]
fn live_inventory_imports_sixteen_rows_in_one_table_and_one_metadata_charge() {
    let home = sixteen_original_history_filesystem_fixture();
    let committed = super::tests::review_prepare(home.path())
        .commit_closed()
        .unwrap();
    let before = committed.snapshot().unwrap().managed.usage;
    let custody = committed.original_custody_identity();
    let prepared = committed.prepare_live_inventory().unwrap();
    assert_eq!(prepared.original_custody_identity(), custody);
    let snapshot = prepared.snapshot().unwrap();
    assert_eq!(snapshot.historical_rows, 16);
    assert_eq!(snapshot.new_rows, 0);
    assert_eq!(snapshot.remaining_slots, 0);
    assert_eq!(snapshot.managed.active_reservations, 17);
    assert_eq!(snapshot.managed.usage, before);
    assert_eq!(snapshot.managed.usage.host_metadata_bytes, 1 << 20);
    assert_eq!(
        snapshot.managed.usage.cache_disk_bytes,
        16 * ((8 << 20) + (32 << 10)) + 8192
    );
    assert!(snapshot.managed.fenced);
    prepared.check_current().unwrap();
}
