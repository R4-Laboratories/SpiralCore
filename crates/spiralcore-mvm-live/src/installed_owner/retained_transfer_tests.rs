//! PRIVATE FORMAT FIXTURES ONLY: no Files/locks/owners, native seal or grant.
//! Numeric phase4 rows here test canonical bytes, never EndpointRetirement.
use super::transfer::{CatalogMode, CatalogV2, OriginRecord, TransferIntent, TransferKind};
use super::*;

fn epoch(profile: u8, generation: u64) -> Epoch {
    Epoch {
        key: InstallationKey {
            installation: [11; 32],
            profile: [profile; 32],
        },
        generation,
    }
}
fn row(index: u8) -> RetainedRecord {
    RetainedRecord {
        tenant: [index + 1; 32],
        session: [index + 17; 32],
        parent: 41,
        serial: u64::from(index) + 1,
        records: 8 << 20,
        sidecars: 32 << 10,
        state: PathBuf::from(format!("/private-format-fixture/{index}")),
        state_device: 1,
        state_inode: u64::from(index) + 100,
        phase: 4,
        record_id: (1, u64::from(index) + 200),
        key_id: (1, u64::from(index) + 300),
        sealed_len: 3,
        sealed_sha256: Sha256::digest(b"abc").into(),
    }
}
fn source(count: u8) -> (Epoch, Vec<u8>) {
    let old = epoch(12, 7);
    (
        old,
        Catalog {
            epoch: old,
            entries: (0..count).map(row).collect(),
        }
        .encode()
        .unwrap(),
    )
}
fn owner_record(value: Epoch) -> [u8; 176] {
    Record {
        key: value.key,
        generation: value.generation,
        phase: Phase::Active,
        directory: ObjectId {
            device: 1,
            inode: 2,
        },
        owner: ObjectId {
            device: 1,
            inode: 3,
        },
        holders: ObjectId {
            device: 1,
            inode: 4,
        },
    }
    .encode()
}
fn intent(kind: TransferKind, old: Epoch, bytes: &[u8], target: &CatalogV2) -> TransferIntent {
    TransferIntent {
        source_version: 1,
        kind,
        old_epoch: old,
        target_epoch: target.epoch,
        objects: [(1, 2), (1, 3), (1, 4)],
        old_catalog_sha256: Sha256::digest(bytes).into(),
        target_catalog_sha256: Sha256::digest(target.encode().unwrap()).into(),
        beacon_sha256: Sha256::digest(b"format-fixture-not-an-original-beacon").into(),
        old_record: owner_record(old),
        target_record: owner_record(target.epoch),
        old_catalog_len: bytes.len() as u64,
        target_catalog_len: target.encode().unwrap().len() as u64,
        entries: target.entries.len() as u16,
        source_mode: CatalogMode::Active,
        target_mode: target.mode,
    }
}
fn rehash(bytes: &mut [u8], domain: &[u8]) {
    let end = bytes.len() - 32;
    let mut hash = Sha256::new();
    hash.update(domain);
    hash.update(&bytes[..end]);
    bytes[end..].copy_from_slice(&hash.finalize());
}

#[test]
fn v2_uses_exact_reserved_tail_and_reconstructs_v1_byte_for_byte() {
    let (old, bytes) = source(2);
    let next = epoch(13, 8);
    let target = CatalogV2::from_v1(&bytes, old, next, TransferKind::Restart).unwrap();
    let raw = target.encode().unwrap();
    assert_eq!(raw.len(), 120 + 2 * 4352);
    assert_eq!(&raw[..8], b"SCRETN02");
    assert_eq!(raw[82], 0);
    assert_eq!(&raw[83..88], &[0; 5]);
    for at in 0..2 {
        let begin = 88 + at * 4352;
        assert_eq!(&raw[begin..begin + 4296], &bytes[begin..begin + 4296]);
        assert_eq!(&raw[begin + 4296..begin + 4328], &old.key.profile);
        assert_eq!(
            &raw[begin + 4328..begin + 4336],
            &old.generation.to_le_bytes()
        );
        assert_eq!(&raw[begin + 4336..begin + 4352], &[0; 16]);
    }
    let decoded = CatalogV2::decode(&raw, next).unwrap();
    assert_eq!(
        decoded.reverse_source(1, old, CatalogMode::Active).unwrap(),
        bytes
    );
    assert_eq!(decoded.encode().unwrap(), raw);
}
#[test]
fn v2_all_sixteen_rows_keep_the_old_maximum_and_refuse_seventeenth() {
    let (old, bytes) = source(16);
    let target = CatalogV2::from_v1(&bytes, old, epoch(13, 8), TransferKind::Restart).unwrap();
    assert_eq!(target.encode().unwrap().len(), 69752);
    assert_eq!(target.entries.len(), 16);
    let mut too_many = target;
    too_many.entries.push(OriginRecord {
        origin: old,
        record: row(16),
    });
    assert!(too_many.encode().is_err());
}
#[test]
fn v2_origin_tuple_allows_only_cross_generation_serial_reuse() {
    let old = epoch(12, 7);
    let mut other = row(1);
    other.serial = 1;
    let mut catalog = CatalogV2 {
        epoch: epoch(14, 9),
        mode: CatalogMode::Active,
        entries: vec![
            OriginRecord {
                origin: old,
                record: row(0),
            },
            OriginRecord {
                origin: epoch(13, 8),
                record: other,
            },
        ],
    };
    assert!(catalog.encode().is_ok());
    catalog.entries[1].origin = old;
    assert!(catalog.encode().is_err());
    catalog.entries[1].origin = epoch(13, 8);
    catalog.entries[1].record.session = catalog.entries[0].record.session;
    assert!(catalog.encode().is_err());
}
#[test]
fn v2_padding_domain_and_exact_length_refuse_even_with_recomputed_checksum() {
    let (old, bytes) = source(1);
    let target = CatalogV2::from_v1(&bytes, old, epoch(13, 8), TransferKind::Restart).unwrap();
    let raw = target.encode().unwrap();
    for index in [83, 88 + 4336] {
        let mut bad = raw.clone();
        bad[index] = 1;
        rehash(&mut bad, b"spiralcore/retained-telemetry-catalog/v2\0");
        assert!(CatalogV2::decode(&bad, target.epoch).is_err());
    }
    let mut wrong_domain = raw.clone();
    rehash(
        &mut wrong_domain,
        b"spiralcore/retained-telemetry-catalog/v1\0",
    );
    assert!(CatalogV2::decode(&wrong_domain, target.epoch).is_err());
    let mut trailing = raw.clone();
    trailing.push(0);
    assert!(CatalogV2::decode(&trailing, target.epoch).is_err());
    assert!(CatalogV2::decode(&raw[..raw.len() - 1], target.epoch).is_err());
}
#[test]
fn v2_inverse_preserves_mixed_origin_rows_without_pretending_v1() {
    let mut second = row(1);
    second.serial = 1;
    let original = CatalogV2 {
        epoch: epoch(14, 9),
        mode: CatalogMode::StoppedRetained,
        entries: vec![
            OriginRecord {
                origin: epoch(12, 7),
                record: row(0),
            },
            OriginRecord {
                origin: epoch(13, 8),
                record: second,
            },
        ],
    };
    let bytes = original.encode().unwrap();
    let target = CatalogV2 {
        epoch: epoch(15, 10),
        mode: CatalogMode::Active,
        entries: original.entries.clone(),
    };
    assert_eq!(
        target
            .reverse_source(2, original.epoch, original.mode)
            .unwrap(),
        bytes
    );
    assert!(
        target
            .reverse_source(1, original.epoch, CatalogMode::Active)
            .is_err()
    );
}
#[test]
fn intent_has_exact_768_byte_canonical_layout_and_reverse_proof() {
    let (old, bytes) = source(2);
    let target = CatalogV2::from_v1(&bytes, old, epoch(13, 8), TransferKind::Restart).unwrap();
    let original = intent(TransferKind::Restart, old, &bytes, &target);
    let raw = original.encode().unwrap();
    assert_eq!(raw.len(), 768);
    assert_eq!(&raw[..8], b"SCRETX01");
    assert_eq!(&raw[11..16], &[0; 5]);
    assert_eq!(&raw[644..736], &[0; 92]);
    assert_eq!(&raw[272..448], &owner_record(old));
    assert_eq!(&raw[448..624], &owner_record(target.epoch));
    assert_eq!(TransferIntent::decode(&raw).unwrap().encode().unwrap(), raw);
    original.verify_reverse(&target.encode().unwrap()).unwrap();
}
#[test]
fn intent_reverse_proof_rejects_changed_row_despite_new_valid_target_hash() {
    let (old, bytes) = source(2);
    let mut target = CatalogV2::from_v1(&bytes, old, epoch(13, 8), TransferKind::Restart).unwrap();
    let mut value = intent(TransferKind::Restart, old, &bytes, &target);
    target.entries[0].record.state_inode += 1000;
    let changed = target.encode().unwrap();
    value.target_catalog_sha256 = Sha256::digest(&changed).into();
    assert!(value.verify_reverse(&changed).is_err());
}
#[test]
fn retained_stop_is_same_owner_epoch_and_distinct_nonzero_debt_mode() {
    let (old, bytes) = source(1);
    let target = CatalogV2::from_v1(&bytes, old, old, TransferKind::RetainedStop).unwrap();
    assert!(matches!(target.mode, CatalogMode::StoppedRetained));
    let original = intent(TransferKind::RetainedStop, old, &bytes, &target);
    let raw = original.encode().unwrap();
    assert_eq!(raw[9], 2);
    assert_eq!(raw[643], 1);
    assert_eq!(original.old_record, original.target_record);
    original.verify_reverse(&target.encode().unwrap()).unwrap();
    assert!(CatalogV2::from_v1(&bytes, old, epoch(13, 8), TransferKind::RetainedStop).is_err());
}
#[test]
fn intent_unknown_kind_reserved_bytes_and_generation_wrap_refuse() {
    let (old, bytes) = source(1);
    let target = CatalogV2::from_v1(&bytes, old, epoch(13, 8), TransferKind::Restart).unwrap();
    let original = intent(TransferKind::Restart, old, &bytes, &target);
    let raw = original.encode().unwrap();
    for (index, value) in [(9, 3), (10, 1), (11, 1), (644, 1)] {
        let mut bad = raw;
        bad[index] = value;
        rehash(&mut bad, b"spiralcore/retained-telemetry-transfer/v1\0");
        assert!(TransferIntent::decode(&bad).is_err());
    }
    let mut bad = original;
    bad.old_epoch = epoch(12, u64::MAX);
    bad.old_record = owner_record(bad.old_epoch);
    bad.target_epoch = epoch(13, 0);
    bad.target_record = owner_record(bad.target_epoch);
    assert!(bad.encode().is_err());
}

#[test]
fn independently_captured_old_catalog_keeps_exact_v1_bytes() {
    // Read-only copy of the actual protected OLD product's synthetic phase2
    // catalog, not a generated expectation from the refactored encoder.
    let raw: &[u8] = include_bytes!("retained_v1_golden.bin");
    let digest: String = Sha256::digest(raw)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(
        digest,
        "19aae811031be4032e46973677b534741c070f536c80095e7e9be8347f546111"
    );
    let old = Epoch {
        key: InstallationKey {
            installation: raw[8..40].try_into().unwrap(),
            profile: raw[40..72].try_into().unwrap(),
        },
        generation: u64::from_le_bytes(raw[72..80].try_into().unwrap()),
    };
    assert_eq!(Catalog::decode(raw, old).unwrap().encode().unwrap(), raw);
    let next = Epoch {
        key: InstallationKey {
            installation: old.key.installation,
            profile: [13; 32],
        },
        generation: old.generation.checked_add(1).unwrap(),
    };
    let target = CatalogV2::from_v1(raw, old, next, TransferKind::Restart).unwrap();
    assert_eq!(
        target.reverse_source(1, old, CatalogMode::Active).unwrap(),
        raw
    );
}

thread_local! {
    static REVERSE_CLONES: std::cell::Cell<[usize; 2]> = const { std::cell::Cell::new([0; 2]) };
}
// cfg(test)-only observation immediately before the ACTUAL clone operation.
// This tests control ordering, not allocator/RSS costs or native exploitability.
pub(super) fn observe_reverse_clone(version: u8) {
    REVERSE_CLONES.with(|counts| {
        let mut value = counts.get();
        value[usize::from(version - 1)] += 1;
        counts.set(value);
    });
}
fn check_clone_order(version: u8, shape: u8) {
    let (old, bytes) = source(if shape == 1 { 16 } else { 1 });
    let mut target = CatalogV2::from_v1(&bytes, old, epoch(13, 8), TransferKind::Restart).unwrap();
    if shape == 1 {
        target.entries.push(OriginRecord {
            origin: old,
            record: row(16),
        });
    } else if shape == 2 {
        target.entries[0].record.state = PathBuf::from(format!("/{}", "x".repeat(4096)));
    }
    REVERSE_CLONES.with(|counts| counts.set([0; 2]));
    let result = target.reverse_source(version, old, CatalogMode::Active);
    let observed = REVERSE_CLONES.with(|counts| counts.get());
    if shape == 0 {
        assert!(result.is_ok(), "valid reverse control was rejected");
        assert_eq!(
            observed[usize::from(version - 1)],
            1,
            "valid control did not visit actual clone boundary"
        );
    } else {
        assert!(result.is_err());
        assert_eq!(
            observed, [0; 2],
            "invalid shape reached cloning before refusal"
        );
    }
}
#[test]
fn clone_order_v1_valid_control_reaches_boundary() {
    check_clone_order(1, 0);
}
#[test]
fn clone_order_v2_valid_control_reaches_boundary() {
    check_clone_order(2, 0);
}
#[test]
fn clone_order_v1_excess_rows_refuse_before_clone() {
    check_clone_order(1, 1);
}
#[test]
fn clone_order_v2_excess_rows_refuse_before_clone() {
    check_clone_order(2, 1);
}
#[test]
fn clone_order_v1_excess_path_refuses_before_clone() {
    check_clone_order(1, 2);
}
#[test]
fn clone_order_v2_excess_path_refuses_before_clone() {
    check_clone_order(2, 2);
}
#[test]
fn clone_order_empty_catalog_keeps_existing_acceptance() {
    let (old, bytes) = source(0);
    let target = CatalogV2::from_v1(&bytes, old, epoch(13, 8), TransferKind::Restart).unwrap();
    let raw = target.encode().unwrap();
    assert_eq!(raw.len(), 120);
    assert!(
        CatalogV2::decode(&raw, target.epoch)
            .unwrap()
            .entries
            .is_empty()
    );
    assert_eq!(
        target.reverse_source(1, old, CatalogMode::Active).unwrap(),
        bytes
    );
    assert!(target.reverse_source(2, old, CatalogMode::Active).is_ok());
    assert!(
        intent(TransferKind::Restart, old, &bytes, &target)
            .encode()
            .is_err()
    );
}
