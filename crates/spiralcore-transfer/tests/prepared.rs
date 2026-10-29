use spiralcore_transfer::{dag::Graph, prepared::PreparedObject};

#[test]
fn same_length_delta_matches_full_fold_and_reconstructs_exactly() {
    let bytes: Vec<u8> = (0..8193).map(|i| (i * 71) as u8).collect();
    let prepared = PreparedObject::new(&bytes, 1000).unwrap();
    let mut target = bytes.clone();
    target[31] ^= 0x80;
    target[32] ^= 0x10;
    target[8192] = 0;
    let delta = prepared.prepare_delta(&target).unwrap();
    let full_base = Graph::fold(&bytes, 1000).unwrap();
    let full_target = Graph::fold(&target, 1000).unwrap();
    assert_eq!(delta.manifest(), &full_target.manifest);
    let expected = full_target.delta(&full_base);
    let actual = delta
        .new_records()
        .map(|(_, n)| n.clone())
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert_eq!(delta.rebuild().unwrap(), target);
    assert_eq!(prepared.graph().rebuild().unwrap(), bytes);
}

#[test]
fn repair_authorization_is_target_closure_not_every_base_node() {
    let base = PreparedObject::new(b"old", 10).unwrap();
    let old_id = base.graph().manifest().roots[0].1;
    let delta = base.prepare_delta(b"new").unwrap();
    let current = delta.manifest().roots[0].1;
    assert!(base.graph().node(&old_id).is_some());
    assert!(delta.node(&old_id).is_none());
    assert_eq!(
        delta.repair_records(&[old_id]).unwrap_err(),
        spiralcore_transfer::Error::Integrity
    );
    let repaired = delta.repair_records(&[current]).unwrap();
    assert_eq!(repaired, vec![delta.node(&current).unwrap().clone()]);
    assert_eq!(
        delta.repair_records(&[[7; 32]]).unwrap_err(),
        spiralcore_transfer::Error::Integrity
    );
    assert_eq!(
        delta.repair_records(&vec![current; 1025]).unwrap_err(),
        spiralcore_transfer::Error::Limit
    );
}

#[test]
fn promotion_is_budgeted_and_detaches_the_old_base() {
    use std::sync::Arc;
    let bytes: Vec<u8> = (0..4097).map(|i| (i * 29) as u8).collect();
    let base = PreparedObject::new(&bytes, 1000).unwrap();
    let weak = Arc::downgrade(base.graph());
    let mut target = bytes.clone();
    target[123] ^= 1;
    let delta = base.prepare_delta(&target).unwrap();
    let peak = delta.promotion_peak_charge();
    assert!(peak > delta.resident_charge());
    assert_eq!(
        delta.promote(peak - 1).unwrap_err(),
        spiralcore_transfer::Error::Limit
    );
    let delta = base.prepare_delta(&target).unwrap();
    drop(base);
    let promoted = delta.promote(peak).unwrap();
    assert!(
        weak.upgrade().is_none(),
        "promotion retained an old base chain"
    );
    assert_eq!(promoted.bytes(), target);
    assert_eq!(promoted.graph().rebuild().unwrap(), target);
    assert!(promoted.resident_charge() < peak);
    let weak_next = Arc::downgrade(promoted.graph());
    target[4096] ^= 2;
    let next = promoted.prepare_delta(&target).unwrap();
    let next_peak = next.promotion_peak_charge();
    drop(promoted);
    let next = next.promote(next_peak).unwrap();
    assert!(weak_next.upgrade().is_none());
    assert_eq!(
        next.graph().manifest(),
        &Graph::fold(&target, 1000).unwrap().manifest
    );
    assert_eq!(next.graph().rebuild().unwrap(), target);
}

fn random_bytes(len: usize, mut state: u64) -> Vec<u8> {
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

fn assert_delta_matches(bytes: &[u8], target: &[u8]) {
    let prepared = PreparedObject::new(bytes, 262144).unwrap();
    let delta = prepared.prepare_delta(target).unwrap();
    let full_base = Graph::fold(bytes, 262144).unwrap();
    let full_target = Graph::fold(target, 262144).unwrap();
    assert_eq!(delta.manifest().encode(), full_target.manifest.encode());
    assert_eq!(delta.manifest().id(), full_target.manifest.id());
    assert_eq!(delta.node_count(), full_target.nodes.len());
    assert_eq!(
        delta
            .new_records()
            .map(|(_, n)| n.encode())
            .collect::<Vec<_>>(),
        full_target
            .delta(&full_base)
            .iter()
            .map(|n| n.encode())
            .collect::<Vec<_>>()
    );
    for (id, node) in &full_target.nodes {
        assert_eq!(delta.node(id), Some(node));
    }
    assert_eq!(delta.rebuild().unwrap(), target);
}

#[test]
fn randomized_boundary_edits_preserve_every_record_and_manifest() {
    for len in [0, 1, 2, 31, 32, 33, 95, 96, 97, 287, 288, 289, 8193] {
        for seed in 1..=6 {
            let bytes = random_bytes(len, seed * 0x9e3779b97f4a7c1);
            let mut target = bytes.clone();
            if len > 0 {
                for i in 0..seed as usize {
                    target[(i * 71 + seed as usize) % len] ^= 1 << (i % 8);
                }
            }
            assert_delta_matches(&bytes, &target);
        }
        assert_delta_matches(&vec![0; len], &vec![1; len]);
    }
    // Reuse already-existing leaves at different positions, including parents
    // whose target IDs are already present elsewhere in the base.
    let bytes: Vec<u8> = [vec![0; 96], vec![1; 96], vec![0; 96]].concat();
    let target: Vec<u8> = [vec![1; 96], vec![0; 96], vec![1; 96]].concat();
    assert_delta_matches(&bytes, &target);
}

#[test]
fn one_changed_leaf_has_only_its_ten_new_records() {
    let bytes = random_bytes(1024 * 1024, 0x9e3779b97f4a7c15);
    let mut target = bytes.clone();
    for byte in &mut target[524288..524320] {
        *byte ^= 0xa5;
    }
    let prepared = PreparedObject::new(&bytes, 262144).unwrap();
    let delta = prepared.prepare_delta(&target).unwrap();
    assert_eq!(delta.new_records().len(), 10);
    assert_eq!(
        delta
            .new_records()
            .map(|(_, node)| node.encode().len())
            .sum::<usize>(),
        1236
    );
    assert_delta_matches(&bytes, &target);
}

#[test]
fn owned_data_and_public_clones_cannot_poison_verified_nodes() {
    let mut input = b"abc\0".to_vec();
    let prepared = PreparedObject::new(&input, 10).unwrap();
    input[0] = 99;
    let mut manifest = prepared.graph().manifest().clone();
    manifest.len = 99;
    let id = prepared.graph().manifest().roots[0].1;
    let mut node = prepared.graph().node(&id).unwrap().clone();
    if let spiralcore_transfer::dag::Node::Leaf(data) = &mut node {
        data[0] = 99;
    }
    assert_eq!(prepared.bytes(), b"abc\0");
    assert_eq!(prepared.graph().rebuild().unwrap(), b"abc\0");
    assert_ne!(prepared.graph().manifest(), &manifest);
    assert_ne!(prepared.graph().node(&id), Some(&node));
}

#[test]
fn exact_repeat_shares_flat_graph_and_bytes_including_empty_and_nul() {
    use std::sync::Arc;
    for bytes in [&[][..], &[0][..], b"a\0\0".as_slice(), &[7; 97][..]] {
        let base = PreparedObject::new(bytes, 100).unwrap();
        let delta = base.prepare_delta(bytes).unwrap();
        assert!(delta.is_repeat());
        assert_eq!(delta.new_records().len(), 0);
        assert!(std::ptr::eq(delta.bytes().as_ptr(), base.bytes().as_ptr()));
        let peak = delta.promotion_peak_charge();
        let promoted = delta.promote(peak).unwrap();
        assert!(Arc::ptr_eq(promoted.graph(), base.graph()));
        assert_eq!(promoted.graph().rebuild().unwrap(), bytes);
    }
}

#[test]
fn length_and_unique_target_limits_fail_without_mutating_base() {
    use spiralcore_transfer::Error;
    let base = PreparedObject::new(&[0; 288], 3).unwrap();
    assert_eq!(base.graph().node_count(), 3);
    assert_eq!(
        base.prepare_delta(&[0; 287]).unwrap_err(),
        Error::Unsupported
    );
    assert_eq!(
        base.prepare_delta(&[0; 289]).unwrap_err(),
        Error::Unsupported
    );
    let mut target = [0; 288];
    target[0] = 1;
    assert_eq!(base.prepare_delta(&target).unwrap_err(), Error::Limit);
    assert_eq!(base.graph().rebuild().unwrap(), [0; 288]);
    // Three fresh records replace three base records: six retained nodes, but
    // only three nodes in the target closure, so the update fits this budget.
    let replacement = base.prepare_delta(&[1; 288]).unwrap();
    assert_eq!(replacement.node_count(), 3);
    assert_eq!(replacement.new_records().len(), 3);
    assert_eq!(replacement.rebuild().unwrap(), [1; 288]);
    assert!(PreparedObject::new(&[], 0).is_ok());
    assert_eq!(PreparedObject::new(&[0], 0).unwrap_err(), Error::Limit);
    assert_eq!(
        PreparedObject::new(&[], usize::MAX).unwrap_err(),
        Error::Limit
    );
}

#[test]
fn conservative_charges_include_shared_base_overlay_index_and_bytes() {
    let base = PreparedObject::new(&[0; 288], 10).unwrap();
    assert_eq!(base.graph().resident_charge(), 3 * 512 + 1024);
    assert_eq!(
        base.resident_charge(),
        base.graph().resident_charge() + 288 + 1024
    );
    let delta = base.prepare_delta(&[1; 288]).unwrap();
    assert_eq!(
        delta.resident_charge(),
        base.graph().resident_charge() + 3 * 512 + 3 * 128 + 288 + 1024
    );
    assert_eq!(
        delta.promotion_peak_charge(),
        delta.resident_charge() + 3 * 512 + 2 * 1024
    );
}
