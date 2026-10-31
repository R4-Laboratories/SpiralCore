use spiralcore_transfer::{
    client, protocol::Mode, schema, schema_selection::PreparedSelector, zstd_whole, Error,
    MAX_CHUNK,
};

fn random(n: usize) -> Vec<u8> {
    let mut state = 0x19e7_c648_ba32_05d1u64;
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}
fn body_cost(encoded_len: usize) -> u64 {
    368 + encoded_len as u64 + 134 * encoded_len.div_ceil(MAX_CHUNK) as u64
}

#[test]
fn first_block_excludes_random_one_mib_without_full_whole_candidate() {
    let artifact = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let base = random(1 << 20);
    let mut target = base.clone();
    for byte in &mut target[(1 << 19)..(1 << 19) + 32] {
        *byte ^= 0xa5;
    }
    let whole = zstd_whole::encode(&target).unwrap();
    assert_eq!(whole[4], 0xa4);
    assert_eq!(u32::from_le_bytes(whole[5..9].try_into().unwrap()), 1 << 20);
    let block = u32::from_le_bytes([whole[9], whole[10], whole[11], 0]);
    assert_eq!(block & 1, 0);
    let payload = match (block >> 1) & 3 {
        0 | 2 => (block >> 3) as usize,
        1 => 1,
        _ => panic!("invalid oracle block"),
    };
    let bound = 12 + payload + 4 + 3 * (target.len() - (128 << 10)).div_ceil(128 << 10);
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    let plan = selector
        .prepare_warm(&target, Some(&base), false, false)
        .unwrap();
    let receipt = plan.selection();
    assert_eq!(receipt.selected_mode, Mode::SparseV1);
    if !zstd_whole::first_block_probe_available() {
        assert!(receipt
            .candidates
            .iter()
            .any(|c| c.mode == Mode::ZstdWholeV1
                && c.target_http_body_bytes == body_cost(whole.len())));
        return;
    }
    assert!(
        !receipt
            .candidates
            .iter()
            .any(|c| c.mode == Mode::ZstdWholeV1),
        "Whole must be pruned from its actual first block, not fully encoded: {receipt:?}"
    );
    let prune = receipt
        .pruned
        .iter()
        .find(|p| p.mode == Mode::ZstdWholeV1)
        .expect("actual first-block lower bound required");
    assert_eq!(prune.target_http_body_lower_bound, body_cost(bound));
    assert_eq!(prune.setup_http_body_bytes, 0);
    assert_eq!(prune.total_http_body_lower_bound, body_cost(bound));
    assert!(body_cost(bound) > 600);
    assert!(body_cost(bound) <= body_cost(whole.len()));
}

#[test]
fn compressible_whole_winner_keeps_uncapped_frame_after_inconclusive_probe() {
    let artifact = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let base = vec![0; 1 << 20];
    let mut target = base.clone();
    target[(1 << 19)..(1 << 19) + 32].fill(0xa5);
    let expected = zstd_whole::encode(&target).unwrap();
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    let plan = selector
        .prepare_warm(&target, Some(&base), false, false)
        .unwrap();
    assert_eq!(plan.selection().selected_mode, Mode::ZstdWholeV1);
    assert_eq!(
        plan.selection()
            .candidates
            .iter()
            .find(|c| c.mode == Mode::ZstdWholeV1)
            .unwrap()
            .target_http_body_bytes,
        body_cost(expected.len())
    );
    let mut bodies = Vec::new();
    let result = client::transfer_warm_selection_batched(plan, [42; 32], |frames| {
        bodies.extend(
            frames
                .iter()
                .filter(|f| f.kind == 2)
                .map(|f| f.body.clone()),
        );
        Err(Error::Storage)
    });
    assert!(matches!(result, Err(Error::Storage)));
    let mut actual = Vec::new();
    for body in bodies {
        assert_eq!(body[0], Mode::ZstdWholeV1 as u8);
        assert_eq!(body[5], 1);
        assert_eq!(
            u32::from_le_bytes(body[1..5].try_into().unwrap()) as usize,
            body.len() - 6
        );
        actual.extend_from_slice(&body[6..]);
    }
    assert_eq!(actual, expected);
}
