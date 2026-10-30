use std::io::Cursor;

use sha2::{Digest, Sha256};
use uor_r4_object_store::{peer, Error, Limits, Manifest, Record, Store, StoreWriter};

fn frame(kind: u8, body: &[u8]) -> Vec<u8> {
    let mut out = vec![kind];
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(body);
    out
}

fn nodes(records: &[Vec<u8>]) -> Vec<u8> {
    let mut out = (records.len() as u16).to_be_bytes().to_vec();
    for record in records {
        out.extend_from_slice(record);
    }
    frame(23, &out)
}

fn has_done(mut bytes: &[u8]) -> bool {
    while bytes.len() >= 5 {
        let len = u32::from_be_bytes(bytes[1..5].try_into().unwrap()) as usize;
        if bytes[0] == 24 {
            return true;
        }
        bytes = &bytes[5 + len..];
    }
    false
}

#[test]
fn invalid_padding_is_rejected_before_publication_or_done() {
    let limits = Limits::default();
    let mut sender = Store::new(limits.clone());
    let valid = sender.fold(&[7; 32]).unwrap().manifest;
    let invalid = Manifest::from_parts(
        31,
        valid.roots().to_vec(),
        Sha256::digest([7; 31]).into(),
        &limits,
    )
    .unwrap();
    let mut input = frame(21, &invalid.encode());
    input.extend(nodes(&[sender.record_bytes(valid.roots()[0].0).unwrap()]));
    let mut receiver = Store::new(limits);
    let mut output = Vec::new();
    let result = peer::pull(&mut receiver, &mut Cursor::new(input), &mut output);
    assert!(matches!(result, Err(Error::IntegrityFailure)));
    assert_eq!(
        receiver.record_count(),
        0,
        "failed validation must not publish records"
    );
    assert!(
        !has_done(&output),
        "failed validation must not acknowledge commit"
    );
}

#[test]
fn cached_parent_does_not_hide_missing_descendants() {
    let limits = Limits::default();
    let payload: Vec<u8> = (0..96).collect();
    let mut sender = Store::new(limits.clone());
    let manifest = sender.fold(&payload).unwrap().manifest;
    let root_bytes = sender.record_bytes(manifest.roots()[0].0).unwrap();
    let root = Record::decode(&root_bytes).unwrap();
    let mut children: Vec<_> = root
        .payload()
        .chunks_exact(12)
        .map(|id| sender.record_bytes(id.try_into().unwrap()).unwrap())
        .collect();
    children.sort_by_key(|bytes| Record::decode(bytes).unwrap().id());
    let mut receiver = Store::new(limits);
    receiver.import_records(&root_bytes).unwrap();
    let mut input = frame(21, &manifest.encode());
    input.extend(nodes(&children));
    let (_, stats) = peer::pull(&mut receiver, &mut Cursor::new(input), &mut Vec::new()).unwrap();
    assert_eq!(stats.nodes_requested, 3);
    assert_eq!(receiver.rebuild(&manifest).unwrap().bytes(), payload);
}

#[test]
fn verify_traverses_below_cached_parents() {
    let mut sender = Store::new(Limits::default());
    let manifest = sender.fold(&(0..96).collect::<Vec<_>>()).unwrap().manifest;
    let mut receiver = Store::new(Limits::default());
    receiver
        .import_records(&sender.record_bytes(manifest.roots()[0].0).unwrap())
        .unwrap();
    let stats = peer::verify(
        &receiver,
        None,
        None,
        &mut Cursor::new(frame(21, &manifest.encode())),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(!stats.verified);
    assert_eq!(stats.missing, 3);
}

#[test]
fn only_eof_at_a_frame_boundary_is_clean() {
    let mut store = Store::new(Limits::default());
    let manifest = store.fold(b"framing").unwrap().manifest;
    let request = frame(20, &[1; 32]);
    peer::serve(
        &store,
        &manifest,
        &mut Cursor::new(Vec::<u8>::new()),
        &mut Vec::new(),
    )
    .unwrap();
    for end in 1..request.len() {
        let result = peer::serve(
            &store,
            &manifest,
            &mut Cursor::new(&request[..end]),
            &mut Vec::new(),
        );
        assert!(
            matches!(
                result,
                Err(Error::Io {
                    kind: std::io::ErrorKind::UnexpectedEof,
                    ..
                })
            ),
            "partial frame of {end} bytes: {result:?}"
        );
    }
}

#[test]
fn done_requires_an_empty_body() {
    let mut store = Store::new(Limits::default());
    let manifest = store.fold(b"done").unwrap().manifest;
    assert!(matches!(
        peer::serve(
            &store,
            &manifest,
            &mut Cursor::new(frame(24, &[0])),
            &mut Vec::new()
        ),
        Err(Error::SchemaMismatch)
    ));
}

#[test]
fn configured_peer_budgets_fail_without_publication() {
    let mut sender = Store::new(Limits::default());
    let manifest = sender.fold(b"bounded").unwrap().manifest;
    let mut input = frame(21, &manifest.encode());
    input.extend(nodes(&[sender
        .record_bytes(manifest.roots()[0].0)
        .unwrap()]));
    for budget in 0..5 {
        let mut limits = peer::PeerLimits::default();
        match budget {
            0 => limits.max_message_bytes = manifest.encode().len() - 1,
            1 => limits.max_staged_bytes = 44,
            2 => limits.max_staged_records = 0,
            3 => limits.max_work = 0,
            4 => limits.max_requests = 0,
            _ => unreachable!(),
        }
        let mut receiver = Store::new(Limits::default());
        let mut output = Vec::new();
        let result = peer::pull_selected_with_limits(
            &mut receiver,
            None,
            None,
            &limits,
            &mut Cursor::new(&input),
            &mut output,
        );
        assert!(
            matches!(result, Err(Error::LimitExceeded(_))),
            "budget {budget}: {result:?}"
        );
        assert_eq!(receiver.record_count(), 0);
        assert!(!has_done(&output));
    }
}

#[test]
fn save_respects_live_writer_ownership() {
    let dir = std::env::temp_dir().join(format!("uor-owner-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("records");
    let owner = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(dir.join("records.uor-lock"))
        .unwrap();
    owner.try_lock().unwrap();
    let mut store = Store::new(Limits::default());
    store.fold(b"owned bytes").unwrap();
    let result = store.save(&path);
    drop(owner);
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(
        matches!(
            result,
            Err(Error::Io {
                kind: std::io::ErrorKind::WouldBlock,
                ..
            })
        ),
        "save must fail closed while another writer owns the path: {result:?}"
    );
}

#[test]
fn whole_transfer_work_is_cumulative_and_exactly_bounded() {
    let mut sender = Store::new(Limits::default());
    let manifest = sender.fold(&(0..96).collect::<Vec<_>>()).unwrap().manifest;
    let root = sender.record_bytes(manifest.roots()[0].0).unwrap();
    let mut ids: Vec<[u8; 12]> = Record::decode(&root)
        .unwrap()
        .payload()
        .chunks_exact(12)
        .map(|id| id.try_into().unwrap())
        .collect();
    ids.sort();
    let mut input = frame(21, &manifest.encode());
    input.extend(nodes(&[root]));
    input.extend(nodes(
        &ids.into_iter()
            .map(|id| sender.record_bytes(id).unwrap())
            .collect::<Vec<_>>(),
    ));
    // Discovery visits 1 + 4 + 4 nodes, decoding visits 4, strict reconstruction visits 4.
    for max_work in [16, 17] {
        let limits = peer::PeerLimits {
            max_work,
            ..peer::PeerLimits::default()
        };
        let mut receiver = Store::new(Limits::default());
        let mut output = Vec::new();
        let result = peer::pull_selected_with_limits(
            &mut receiver,
            None,
            None,
            &limits,
            &mut Cursor::new(&input),
            &mut output,
        );
        if max_work == 16 {
            assert!(matches!(result, Err(Error::LimitExceeded(_))));
            assert_eq!(receiver.record_count(), 0);
            assert!(!has_done(&output));
        } else {
            assert!(result.unwrap().1.verified);
            assert_eq!(receiver.record_count(), 4);
        }
    }
}

#[test]
fn corrupt_truncated_and_oversized_responses_do_not_publish() {
    let mut sender = Store::new(Limits::default());
    let manifest = sender.fold(b"one record").unwrap().manifest;
    let response = nodes(&[sender.record_bytes(manifest.roots()[0].0).unwrap()]);
    for end in 0..response.len() {
        let mut input = frame(21, &manifest.encode());
        input.extend_from_slice(&response[..end]);
        let mut receiver = Store::new(Limits::default());
        let mut output = Vec::new();
        assert!(peer::pull(&mut receiver, &mut Cursor::new(input), &mut output).is_err());
        assert_eq!(receiver.record_count(), 0);
        assert!(!has_done(&output));
    }
    for tail in [vec![23, 255, 255, 255, 255], {
        let mut corrupted = response.clone();
        *corrupted.last_mut().unwrap() ^= 1;
        corrupted
    }] {
        let mut input = frame(21, &manifest.encode());
        input.extend(tail);
        let mut receiver = Store::new(Limits::default());
        assert!(peer::pull(&mut receiver, &mut Cursor::new(input), &mut Vec::new()).is_err());
        assert_eq!(receiver.record_count(), 0);
    }
}

#[test]
fn retained_store_quota_failure_preserves_existing_cache() {
    let mut sender = Store::new(Limits::default());
    let manifest = sender.fold(b"new").unwrap().manifest;
    let mut input = frame(21, &manifest.encode());
    input.extend(nodes(&[sender
        .record_bytes(manifest.roots()[0].0)
        .unwrap()]));
    let mut receiver = Store::new(Limits {
        max_records: 1,
        ..Limits::default()
    });
    receiver.fold(b"existing").unwrap();
    let before = receiver.export_records().unwrap();
    let mut output = Vec::new();
    assert!(matches!(
        peer::pull(&mut receiver, &mut Cursor::new(input), &mut output),
        Err(Error::LimitExceeded(_))
    ));
    assert_eq!(receiver.export_records().unwrap(), before);
    assert!(!has_done(&output));
}

#[test]
fn guarded_restart_ignores_abandoned_temporary_files() {
    let dir = std::env::temp_dir().join(format!("uor-recovery-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("records");
    let mut owner = StoreWriter::open(&path).unwrap();
    let mut store = Store::new(Limits::default());
    let manifest = store.fold(b"canonical verified bytes").unwrap().manifest;
    owner.save(&store).unwrap();
    assert!(matches!(
        StoreWriter::open(&path),
        Err(Error::Io {
            kind: std::io::ErrorKind::WouldBlock,
            ..
        })
    ));
    let orphan = dir.join("records.uor-tmp-abandoned");
    std::fs::write(&orphan, b"incomplete candidate").unwrap();
    drop(owner);
    let owner = StoreWriter::open(&path).unwrap();
    let recovered = owner.load(Limits::default()).unwrap();
    assert_eq!(
        recovered.rebuild(&manifest).unwrap().bytes(),
        b"canonical verified bytes"
    );
    assert_eq!(std::fs::read(&orphan).unwrap(), b"incomplete candidate");
    drop(owner);
    assert!(dir.join("records.uor-lock").is_file());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn ownership_is_released_by_process_exit_without_drop() {
    const CHILD_PATH: &str = "UOR_STORE_LOCK_TEST_CHILD_PATH";
    if let Some(path) = std::env::var_os(CHILD_PATH) {
        let _owner = StoreWriter::open(path).unwrap();
        // Deliberately skip destructors, as a crashed process would.
        std::process::exit(0);
    }
    let dir = std::env::temp_dir().join(format!("uor-lock-exit-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("records");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "ownership_is_released_by_process_exit_without_drop",
        ])
        .env(CHILD_PATH, &path)
        .status()
        .unwrap();
    assert!(status.success());
    let owner = StoreWriter::open(&path).unwrap();
    drop(owner);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn serving_enforces_selected_closure_and_independent_response_budgets() {
    let mut store = Store::new(Limits::default());
    let first = store.fold(b"first").unwrap().manifest;
    let second = store.fold(b"second").unwrap().manifest;
    let mut request = frame(20, &[]);
    let mut fetch = 1u16.to_be_bytes().to_vec();
    fetch.extend_from_slice(&second.roots()[0].0);
    request.extend(frame(22, &fetch));
    assert!(matches!(
        peer::serve(&store, &first, &mut Cursor::new(&request), &mut Vec::new()),
        Err(Error::SchemaMismatch)
    ));
    fetch[2..].copy_from_slice(&first.roots()[0].0);
    let mut input = frame(20, &[]);
    input.extend(frame(22, &fetch));
    for budget in 0..4 {
        let mut limits = peer::PeerLimits::default();
        match budget {
            0 => limits.max_staged_bytes = 44,
            1 => limits.max_staged_records = 0,
            2 => limits.max_work = 1,
            3 => limits.max_requests = 1,
            _ => unreachable!(),
        }
        let mut output = Vec::new();
        assert!(matches!(
            peer::serve_many_with_limits(
                &store,
                std::slice::from_ref(&first),
                &limits,
                &mut Cursor::new(&input),
                &mut output
            ),
            Err(Error::LimitExceeded(_))
        ));
        assert_eq!(
            output,
            frame(21, &first.encode()),
            "failed response must not partially emit a NODES frame"
        );
    }
}

#[test]
fn wrong_root_scale_cannot_be_committed() {
    let mut sender = Store::new(Limits::default());
    let bytes: Vec<u8> = (0..288).map(|i| (i % 251) as u8).collect();
    let actual = sender.fold(&bytes).unwrap().manifest;
    let invalid = Manifest::from_parts(
        96,
        vec![(actual.roots()[0].0, 1)],
        Sha256::digest(&bytes[..96]).into(),
        &Limits::default(),
    )
    .unwrap();
    let mut input = frame(21, &invalid.encode());
    input.extend(nodes(&[sender.record_bytes(actual.roots()[0].0).unwrap()]));
    let mut receiver = Store::new(Limits::default());
    let mut output = Vec::new();
    assert!(matches!(
        peer::pull(&mut receiver, &mut Cursor::new(input), &mut output),
        Err(Error::IntegrityFailure)
    ));
    assert_eq!(receiver.record_count(), 0);
    assert!(!has_done(&output));
}

#[test]
fn small_in_flight_budget_reduces_batch_instead_of_rejecting_a_single_record() {
    let mut sender = Store::new(Limits::default());
    let manifest = sender.fold(b"one bounded response").unwrap().manifest;
    let mut input = frame(21, &manifest.encode());
    input.extend(nodes(&[sender
        .record_bytes(manifest.roots()[0].0)
        .unwrap()]));
    let limits = peer::PeerLimits {
        max_in_flight_response_bytes: 56,
        ..peer::PeerLimits::default()
    };
    let mut receiver = Store::new(Limits::default());
    let (_, stats) = peer::pull_selected_with_limits(
        &mut receiver,
        None,
        None,
        &limits,
        &mut Cursor::new(input),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(stats.verified);
}

#[test]
fn done_does_not_consume_a_manifest_or_fetch_request_budget() {
    let mut sender = Store::new(Limits::default());
    let manifest = sender.fold(b"request budget").unwrap().manifest;
    let mut input = frame(21, &manifest.encode());
    input.extend(nodes(&[sender
        .record_bytes(manifest.roots()[0].0)
        .unwrap()]));
    let limits = peer::PeerLimits {
        max_requests: 2,
        ..peer::PeerLimits::default()
    };
    let mut receiver = Store::new(Limits::default());
    let mut requests = Vec::new();
    peer::pull_selected_with_limits(
        &mut receiver,
        None,
        None,
        &limits,
        &mut Cursor::new(input),
        &mut requests,
    )
    .unwrap();
    // The same policy must accept the client's manifest request, one FETCH and final control frame.
    peer::serve_many_with_limits(
        &sender,
        &[manifest],
        &limits,
        &mut Cursor::new(requests),
        &mut Vec::new(),
    )
    .unwrap();
}
