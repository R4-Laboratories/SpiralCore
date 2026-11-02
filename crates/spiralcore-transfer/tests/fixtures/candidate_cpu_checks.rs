use super::*;
use crate::{
    schema,
    shared_runtime::{OutputKind, Recipe, State},
    shared_transfer::PreparedPrediction,
};
#[allow(dead_code)]
#[path = "candidate_before.rs"]
mod before;

fn prediction(steps: u32) -> PreparedPrediction {
    PreparedPrediction::new(
        &State::seeded(4096).unwrap().encode().unwrap(),
        &Recipe {
            steps,
            payload: 1.0,
            output: OutputKind::Trace,
        },
    )
    .unwrap()
}
fn current(
    s: &mut PreparedSelector,
    target: &[u8],
    base: Option<&[u8]>,
    p: &PreparedPrediction,
    ready: bool,
    probe: bool,
) -> (Vec<u8>, Vec<Vec<u8>>, WarmSelection) {
    let prepared = s
        .prepare_warm_with_identity(target, base, ready, probe, |b| Ok(object_id(b)), Some(p))
        .unwrap();
    let bodies = match prepared.payload {
        Payload::Raw(bytes) => bytes
            .chunks(MAX_CHUNK)
            .map(|c| {
                let mut f = vec![0];
                f.extend_from_slice(&(c.len() as u32).to_le_bytes());
                f.extend_from_slice(c);
                f
            })
            .collect(),
        Payload::Encoded(b) => b,
    };
    (prepared.offer.encode().unwrap(), bodies, prepared.selection)
}

#[test]
fn candidate_cpu_schema_loser_is_certified_and_canonical_frames_match_frozen_baseline() {
    let p = prediction(289);
    let artifact = schema::fit(&[b"{\"value\":0,\"message\":\"public training\"}\n"]).unwrap();
    let mut edited = p.prediction().to_vec();
    for at in 1..=16 {
        let offset = at * edited.len() / 17;
        for b in &mut edited[offset..offset + 32] {
            *b ^= 0xa5;
        }
    }
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    for ready in [false, true] {
        for probe in [false, true] {
            for (target, base) in [
                (p.prediction(), None),
                (edited.as_slice(), Some(p.prediction())),
            ] {
                let expected = before::witness(target, base, &p, &artifact, ready, probe).unwrap();
                let (offer, bodies, receipt) =
                    current(&mut selector, target, base, &p, ready, probe);
                assert_eq!(
                    (offer, bodies),
                    expected,
                    "canonical Offer and every Data body"
                );
                if ready && crate::zstd_whole::first_block_probe_available() {
                    assert!(
                        receipt.pruned.iter().any(|c| c.mode == Mode::SchemaZstdV1),
                        "expensive losing schema must be certified before full encoding"
                    );
                    assert!(!receipt
                        .candidates
                        .iter()
                        .any(|c| c.mode == Mode::SchemaZstdV1));
                }
            }
        }
    }
}

fn opaque(n: usize) -> Vec<u8> {
    let mut seed = 0x123456789abcdefu64;
    (0..n)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u8
        })
        .collect()
}

// Independent copy of the original encode operation: canonical SCVAL001 then
// fresh new_encoder/compress2. It does not call either new intermediate method.
fn original_schema_frame(artifact: &[u8], target: &[u8]) -> Vec<u8> {
    let canonical = schema::encode(artifact, target).unwrap();
    let mut encoder = crate::zstd_dictionary::new_encoder().unwrap();
    let mut encoded = Vec::with_capacity(zstd::zstd_safe::compress_bound(canonical.len()));
    encoder.compress2(&mut encoded, &canonical).unwrap();
    encoded
}

#[test]
fn candidate_cpu_bounds_preserve_domain_ties_and_context_after_prune_or_inconclusive() {
    let artifact = schema::fit(&[b"{\"value\":0,\"message\":\"public training\"}\n"]).unwrap();
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    let matching = b"{\"value\":17,\"message\":\"public training\"}\n".repeat(4096);
    let mut mixed = matching.clone();
    mixed.extend_from_slice(&opaque(140000));
    // Single literal overhead is exactly53; the first two cases straddle the
    // source-qualified probe's strict 128KiB input threshold.
    let cases = [
        vec![b'x'; (128 << 10) - 53],
        vec![b'x'; (128 << 10) - 52],
        matching,
        mixed,
        opaque(1 << 20),
        vec![b'x'; 16 << 20],
    ];
    for (index, target) in cases.into_iter().enumerate() {
        let intermediate = selector.schema.prepare_intermediate(&target).unwrap();
        if index < 2 {
            assert_eq!(intermediate.bytes().len(), (128 << 10) + index);
        }
        let expected = original_schema_frame(&artifact, &target);
        for probe in [false, true] {
            let actual_cost = encoded_cost(Mode::SchemaZstdV1, &expected, probe).unwrap();
            for incumbent in [600, 787, 1390, actual_cost, actual_cost + 1] {
                let proof = selector
                    .prune_schema_intermediate(
                        intermediate.bytes(),
                        probe,
                        Mode::RuntimeV1,
                        incumbent,
                        0,
                    )
                    .unwrap();
                if index == 5 {
                    assert!(
                        proof.is_none(),
                        "valid expanded schema exceeds Whole proof domain"
                    );
                }
                if let Some(bound) = proof {
                    assert!(bound.total_http_body_lower_bound <= actual_cost);
                    assert!(!warm_wins(
                        Mode::SchemaZstdV1,
                        bound.total_http_body_lower_bound,
                        Mode::RuntimeV1,
                        incumbent
                    ));
                }
            }
        }
        // Both prepared contexts remain identical after all discarded probes.
        assert_eq!(
            selector.schema.encode_prepared(intermediate).unwrap(),
            expected
        );
        assert_eq!(selector.schema.encode(&target).unwrap(), expected);
        assert_eq!(
            selector.whole.encode(&target).unwrap(),
            zstd_whole::encode(&target).unwrap()
        );
        assert_eq!(
            schema_zstd::decode(&artifact, &expected, target.len()).unwrap(),
            target
        );
    }
    assert!(matches!(
        selector
            .schema
            .prepare_intermediate(&vec![0; (16 << 20) + 1]),
        Err(Error::Limit)
    ));
    assert_eq!(
        selector.schema.encode(b"after rejected target\0").unwrap(),
        schema_zstd::encode(&artifact, b"after rejected target\0").unwrap()
    );
    // Schema is last in the unchanged tie order. Earlier Whole still wins ties
    // against Sparse/Runtime; Raw remains first.
    for earlier in [
        Mode::Raw,
        Mode::ZstdWholeV1,
        Mode::SparseV1,
        Mode::RuntimeV1,
    ] {
        assert!(!warm_wins(Mode::SchemaZstdV1, 600, earlier, 600));
    }
    assert!(warm_wins(Mode::ZstdWholeV1, 600, Mode::RuntimeV1, 600));
    assert!(warm_wins(Mode::SparseV1, 600, Mode::RuntimeV1, 600));
}

#[test]
fn candidate_cpu_frozen_oracle_covers_fallbacks_lengths_and_equal_basis_receipts() {
    let p = prediction(81);
    let artifact = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let mut edited = p.prediction().to_vec();
    edited[32] ^= 0xa5;
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    let targets = [
        Vec::new(),
        vec![0; 4096],
        b"{\"value\":17}\n".repeat(2000),
        opaque(300000),
        edited.clone(),
        [p.prediction(), b"\0append\0"].concat(),
        p.prediction()[..p.prediction().len() - 7].to_vec(),
    ];
    for target in targets {
        for probe in [false, true] {
            for ready in [false, true] {
                for base in [None, Some(p.prediction()), Some(edited.as_slice())] {
                    let expected =
                        before::witness(&target, base, &p, &artifact, ready, probe).unwrap();
                    let (offer, bodies, receipt) =
                        current(&mut selector, &target, base, &p, ready, probe);
                    assert_eq!((offer, bodies), expected);
                    if base == Some(p.prediction()) {
                        let frames = p.prepare(&target).unwrap();
                        let runtime = receipt
                            .candidates
                            .iter()
                            .find(|c| c.mode == Mode::RuntimeV1);
                        match frames {
                            Some(frames) => assert_eq!(
                                runtime.unwrap().target_http_body_bytes,
                                selection::cost(
                                    Mode::RuntimeV1,
                                    frames.iter().map(Vec::len),
                                    probe
                                )
                                .unwrap()
                            ),
                            None => assert!(runtime.is_none()),
                        }
                    }
                }
            }
        }
    }
}
