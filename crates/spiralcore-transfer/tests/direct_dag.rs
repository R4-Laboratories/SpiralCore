use spiralcore_transfer::{
    dag::{work_bound, Graph, Manifest, Node},
    object_id, Error, Id, Result, MAX_OBJECT,
};
use std::collections::BTreeMap;

// Frozen pre-direct-builder implementation from 354b616. This independent test
// oracle deliberately retains the legacy fold, rebuild and record conversion.
fn legacy_fold(bytes: &[u8], max_nodes: usize) -> Result<Graph> {
    if bytes.len() as u64 > MAX_OBJECT {
        return Err(Error::Limit);
    }
    let limits = uor_r4_object_store::Limits {
        max_object_bytes: MAX_OBJECT,
        max_records: max_nodes,
        max_record_bytes: max_nodes.checked_mul(49).ok_or(Error::Limit)?,
        max_returned_bytes: MAX_OBJECT,
        max_visits: work_bound(bytes.len() as u64),
        max_occurrences: work_bound(bytes.len() as u64),
    };
    let mut store = uor_r4_object_store::Store::new(limits);
    let folded = store.fold(bytes).map_err(core_error)?;
    if store.rebuild(&folded.manifest).map_err(core_error)?.bytes() != bytes {
        return Err(Error::Integrity);
    }
    let mut ids = BTreeMap::new();
    let mut nodes = BTreeMap::new();
    fn convert(
        old: [u8; 12],
        scale: u8,
        store: &uor_r4_object_store::Store,
        ids: &mut BTreeMap<[u8; 12], Id>,
        nodes: &mut BTreeMap<Id, Node>,
    ) -> Result<Id> {
        let raw = store.record_bytes(old).ok_or(Error::Missing)?;
        let rec = uor_r4_object_store::Record::decode(&raw).map_err(core_error)?;
        if rec.scale() != scale {
            return Err(Error::Integrity);
        }
        if let Some(id) = ids.get(&old) {
            return Ok(*id);
        }
        let node = if scale == 0 {
            Node::Leaf(rec.payload().try_into().map_err(|_| Error::Malformed)?)
        } else {
            let mut children = [[0; 32]; 3];
            for (dst, p) in children.iter_mut().zip(rec.payload().chunks_exact(12)) {
                *dst = convert(
                    p.try_into().map_err(|_| Error::Malformed)?,
                    scale - 1,
                    store,
                    ids,
                    nodes,
                )?;
            }
            Node::Parent { scale, children }
        };
        let id = node.id();
        nodes.insert(id, node);
        ids.insert(old, id);
        Ok(id)
    }
    let mut roots = Vec::new();
    for (id, scale) in folded.manifest.roots() {
        roots.push((*scale, convert(*id, *scale, &store, &mut ids, &mut nodes)?));
    }
    Ok(Graph {
        manifest: Manifest {
            len: bytes.len() as u64,
            object: object_id(bytes),
            roots,
        },
        nodes,
    })
}
fn core_error(e: uor_r4_object_store::Error) -> Error {
    match e {
        uor_r4_object_store::Error::LimitExceeded(_) => Error::Limit,
        _ => Error::Integrity,
    }
}

fn random_bytes(len: usize) -> Vec<u8> {
    let mut state = 0x9e3779b97f4a7c15u64;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

fn assert_identical(bytes: &[u8]) {
    let expected = legacy_fold(bytes, 262144).unwrap();
    let direct = Graph::fold(bytes, 262144).unwrap();
    assert_eq!(direct.manifest, expected.manifest, "length {}", bytes.len());
    assert_eq!(direct.manifest.encode(), expected.manifest.encode());
    assert_eq!(direct.manifest.id(), expected.manifest.id());
    assert_eq!(direct.nodes, expected.nodes, "length {}", bytes.len());
    for (id, node) in &direct.nodes {
        assert_eq!(node.id(), *id);
        assert_eq!(node.encode(), expected.nodes[id].encode());
    }
    assert_eq!(direct.rebuild().unwrap(), bytes);
}

#[test]
fn direct_builder_matches_legacy_canonical_graph() {
    let mut cases = vec![
        vec![],
        vec![0],
        vec![0, 0],
        b"a".to_vec(),
        b"a\0".to_vec(),
        b"a\0\0".to_vec(),
        (0..=255u8).collect(),
    ];
    // A leaf boundary and every ternary carry through 3^7 leaves, including
    // the partial trailing leaf on either side of each carry.
    for exponent in 0..=7 {
        let boundary = 32 * 3usize.pow(exponent);
        for len in [boundary - 1, boundary, boundary + 1] {
            cases.push(random_bytes(len));
        }
    }
    cases.push(vec![0; 32768]);
    cases.push((0..32768).map(|i| (i % 64) as u8).collect());
    cases.push((0..32768).map(|i| ((i / 96) % 251) as u8).collect());
    for bytes in cases {
        assert_identical(&bytes);
    }
}

#[test]
fn unique_node_budget_matches_legacy_at_every_small_boundary() {
    let bytes = random_bytes(1025);
    let expected = legacy_fold(&bytes, 100).unwrap();
    let unique = expected.nodes.len();
    for max_nodes in 0..=unique + 1 {
        let old = legacy_fold(&bytes, max_nodes);
        let direct = Graph::fold(&bytes, max_nodes);
        if max_nodes < unique {
            assert_eq!(old.unwrap_err(), Error::Limit);
            assert_eq!(direct.unwrap_err(), Error::Limit);
        } else {
            let direct = direct.unwrap();
            assert_eq!(direct.nodes, old.unwrap().nodes);
            assert_eq!(direct.nodes.len(), unique);
            assert!(direct.nodes.len() <= max_nodes);
        }
    }
}

#[test]
fn repeated_occurrences_only_consume_unique_node_budget() {
    // 32,768 leaf occurrences and 16,376 parents collapse to ten unique nodes.
    let bytes = vec![0; 1024 * 1024];
    let old = legacy_fold(&bytes, 10).unwrap();
    let direct = Graph::fold(&bytes, 10).unwrap();
    assert_eq!(direct.nodes.len(), 10);
    assert_eq!(direct.nodes, old.nodes);
    assert_eq!(direct.manifest, old.manifest);
    assert_eq!(direct.rebuild().unwrap(), bytes);
    assert_eq!(Graph::fold(&bytes, 9).unwrap_err(), Error::Limit);
    assert_eq!(legacy_fold(&bytes, 9).unwrap_err(), Error::Limit);
}

#[test]
fn zero_and_overflowing_budgets_preserve_legacy_behavior() {
    assert!(Graph::fold(&[], 0).unwrap().nodes.is_empty());
    assert!(legacy_fold(&[], 0).unwrap().nodes.is_empty());
    assert_eq!(Graph::fold(&[0], 0).unwrap_err(), Error::Limit);
    assert_eq!(legacy_fold(&[0], 0).unwrap_err(), Error::Limit);
    let largest = usize::MAX / 49;
    for bytes in [&[][..], &[0][..]] {
        let old = legacy_fold(bytes, largest).unwrap();
        let direct = Graph::fold(bytes, largest).unwrap();
        assert_eq!(direct.manifest, old.manifest);
        assert_eq!(direct.nodes, old.nodes);
        for overflow in [largest + 1, usize::MAX] {
            assert_eq!(legacy_fold(bytes, overflow).unwrap_err(), Error::Limit);
            assert_eq!(Graph::fold(bytes, overflow).unwrap_err(), Error::Limit);
        }
    }
}

#[test]
fn oversized_object_is_rejected_before_folding() {
    let bytes = vec![0; MAX_OBJECT as usize + 1];
    assert_eq!(legacy_fold(&bytes, 262144).unwrap_err(), Error::Limit);
    assert_eq!(Graph::fold(&bytes, 262144).unwrap_err(), Error::Limit);
}
