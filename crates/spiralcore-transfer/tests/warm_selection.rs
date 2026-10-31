use spiralcore_transfer::{
    client, delta, object_id,
    protocol::{Mode, Offer},
    schema,
    schema_selection::{PreparedSelector, WarmSelection},
    schema_zstd, zstd_whole, Error, MAX_CHUNK,
};

fn random(n: usize) -> Vec<u8> {
    let mut x = 0x9a41_7bea_304d_119bu64;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect()
}

struct Reference {
    mode: Mode,
    target_cost: u64,
    setup: u64,
    bodies: Vec<Vec<u8>>,
}
impl Reference {
    fn total(&self) -> u64 {
        self.target_cost + self.setup
    }
}
fn frame_cost(mode: Mode, bodies: &[Vec<u8>], probe: bool) -> u64 {
    368 + u64::from(probe) * 40
        + if matches!(mode, Mode::SparseV1 | Mode::SchemaZstdV1) {
            32
        } else {
            0
        }
        + bodies.iter().map(|b| b.len() as u64 + 128).sum::<u64>()
}
fn raw_bodies(target: &[u8]) -> Vec<Vec<u8>> {
    target
        .chunks(MAX_CHUNK)
        .map(|chunk| {
            let mut b = vec![0];
            b.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
            b.extend_from_slice(chunk);
            b
        })
        .collect()
}
fn encoded_bodies(mode: Mode, encoded: &[u8]) -> Vec<Vec<u8>> {
    encoded
        .chunks(MAX_CHUNK)
        .map(|chunk| {
            let mut b = vec![mode as u8];
            b.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
            b.push(1);
            b.extend_from_slice(chunk);
            b
        })
        .collect()
}
fn exhaustive(
    artifact: &[u8],
    base: Option<&[u8]>,
    target: &[u8],
    ready: bool,
    probe: bool,
) -> Vec<Reference> {
    let raw = raw_bodies(target);
    let whole = encoded_bodies(Mode::ZstdWholeV1, &zstd_whole::encode(target).unwrap());
    let schema = encoded_bodies(
        Mode::SchemaZstdV1,
        &schema_zstd::encode(artifact, target).unwrap(),
    );
    let mut candidates = vec![
        Reference {
            mode: Mode::Raw,
            target_cost: frame_cost(Mode::Raw, &raw, probe),
            setup: 0,
            bodies: raw,
        },
        Reference {
            mode: Mode::ZstdWholeV1,
            target_cost: frame_cost(Mode::ZstdWholeV1, &whole, probe),
            setup: 0,
            bodies: whole,
        },
    ];
    if let Some(base) = base {
        if let Some(recipe) = delta::encode(base, target).unwrap() {
            candidates.push(Reference {
                mode: Mode::SparseV1,
                target_cost: frame_cost(Mode::SparseV1, &recipe.frames, probe),
                setup: 0,
                bodies: recipe.frames,
            });
        }
    }
    candidates.push(Reference {
        mode: Mode::SchemaZstdV1,
        target_cost: frame_cost(Mode::SchemaZstdV1, &schema, probe),
        setup: if ready {
            0
        } else {
            frame_cost(Mode::Raw, &raw_bodies(artifact), false)
        },
        bodies: schema,
    });
    candidates
}
fn check_receipt(receipt: &WarmSelection, reference: &[Reference], ready: bool) {
    let best = reference.iter().min_by_key(|c| c.total()).unwrap();
    assert_eq!(receipt.selected_mode, best.mode, "{receipt:?}");
    assert_eq!(
        receipt.needs_schema_seed,
        !ready && best.mode == Mode::SchemaZstdV1
    );
    assert_eq!(
        receipt.candidates.len() + receipt.pruned.len(),
        reference.len()
    );
    let position = |mode| reference.iter().position(|c| c.mode == mode).unwrap();
    assert!(receipt
        .candidates
        .windows(2)
        .all(|w| position(w[0].mode) < position(w[1].mode)));
    assert!(receipt
        .pruned
        .windows(2)
        .all(|w| position(w[0].mode) < position(w[1].mode)));
    for c in &receipt.candidates {
        let r = reference.iter().find(|r| r.mode == c.mode).unwrap();
        assert_eq!(c.target_http_body_bytes, r.target_cost);
        assert_eq!(c.setup_http_body_bytes, r.setup);
        assert_eq!(c.total_http_body_bytes, r.total());
        assert!(!receipt.pruned.iter().any(|p| p.mode == c.mode));
    }
    for p in &receipt.pruned {
        let r = reference.iter().find(|r| r.mode == p.mode).unwrap();
        assert!(p.target_http_body_lower_bound <= r.target_cost);
        assert_eq!(p.setup_http_body_bytes, r.setup);
        assert_eq!(
            p.total_http_body_lower_bound,
            p.target_http_body_lower_bound + r.setup
        );
        assert!(
            (p.total_http_body_lower_bound, position(p.mode))
                >= (best.total(), position(best.mode))
        );
    }
}

// Capture the actual immutable selected request bodies, then deliberately fail
// transport. Endpoint/durable behavior is independently covered by client tests.
fn compare(
    artifact: &[u8],
    base: Option<&[u8]>,
    target: &[u8],
    ready: bool,
    probe: bool,
    selector: &mut PreparedSelector,
) -> Mode {
    let reference = exhaustive(artifact, base, target, ready, probe);
    let plan = selector.prepare_warm(target, base, ready, probe).unwrap();
    check_receipt(plan.selection(), &reference, ready);
    let best = reference.iter().min_by_key(|c| c.total()).unwrap();
    if !probe {
        let mut captured = Vec::new();
        let result = client::transfer_warm_selection_batched(plan, [91; 32], |frames| {
            captured.extend_from_slice(frames);
            Err(Error::Storage)
        });
        assert!(matches!(result, Err(Error::Storage)));
        let offer = Offer::decode(&captured[0].body).unwrap();
        assert_eq!(offer.mode, best.mode);
        assert_eq!(offer.object, object_id(target));
        assert_eq!(offer.len, target.len() as u64);
        assert_eq!(
            offer.base,
            match best.mode {
                Mode::SparseV1 => Some(object_id(base.unwrap())),
                Mode::SchemaZstdV1 => Some(object_id(artifact)),
                _ => None,
            }
        );
        assert_eq!(captured.last().unwrap().kind, 3);
        assert_eq!(
            captured
                .iter()
                .filter(|f| f.kind == 2)
                .map(|f| f.body.clone())
                .collect::<Vec<_>>(),
            best.bodies
        );
    }
    best.mode
}

#[test]
fn verified_base_makes_sparse_an_exact_scored_candidate() {
    let artifact = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let base = vec![1; 1 << 20];
    let mut target = base.clone();
    target[base.len() / 2..base.len() / 2 + 32].fill(2);
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    let plan = selector
        .prepare_warm(&target, Some(&base), false, false)
        .unwrap();
    let receipt = plan.selection();
    assert!(receipt
        .candidates
        .iter()
        .any(|c| c.mode == Mode::SparseV1 && c.target_http_body_bytes == 600));
}

#[test]
fn sampled_short_period_repetition_schedules_full_encoding_without_pruning() {
    let artifact = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    for period in [1, 2, 4, 8] {
        // Deliberately deceptive: only the sampled windows repeat, while their
        // gaps and the rest of the object remain incompressible. Scheduling may
        // cost CPU; it must still retain the completed exact Whole candidate.
        let mut base = random(1 << 20);
        for window in 0..8 {
            for i in 0..64 {
                base[window * 16384 + i] = (i % period) as u8;
            }
        }
        let mut target = base.clone();
        for byte in &mut target[(1 << 19)..(1 << 19) + 32] {
            *byte ^= 0xa5;
        }
        let reference = exhaustive(&artifact, Some(&base), &target, false, false);
        let plan = selector
            .prepare_warm(&target, Some(&base), false, false)
            .unwrap();
        let receipt = plan.selection();
        check_receipt(receipt, &reference, false);
        assert_eq!(receipt.selected_mode, Mode::SparseV1);
        assert!(
            receipt
                .candidates
                .iter()
                .any(|c| c.mode == Mode::ZstdWholeV1),
            "short-period samples must schedule full encoding, never act as a bound: {receipt:?}"
        );
        assert!(!receipt.pruned.iter().any(|p| p.mode == Mode::ZstdWholeV1));
        compare(&artifact, Some(&base), &target, false, false, &mut selector);
    }
}

#[test]
fn period64_without_short_period_still_uses_actual_rejection_certificate() {
    let artifact = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let base: Vec<u8> = (0..1 << 20).map(|i| (i % 64) as u8).collect();
    let mut target = base.clone();
    for byte in &mut target[(1 << 19)..(1 << 19) + 32] {
        *byte ^= 0xa5;
    }
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    let reference = exhaustive(&artifact, Some(&base), &target, false, false);
    let plan = selector
        .prepare_warm(&target, Some(&base), false, false)
        .unwrap();
    check_receipt(plan.selection(), &reference, false);
    if zstd_whole::first_block_probe_available() {
        assert!(plan
            .selection()
            .pruned
            .iter()
            .any(|p| { p.mode == Mode::ZstdWholeV1 && p.target_http_body_lower_bound > 600 }));
    }
    compare(&artifact, Some(&base), &target, false, false, &mut selector);
}

#[test]
fn all_four_winners_match_exhaustive_selected_frame_bytes() {
    let key: String = random(2048)
        .iter()
        .map(|b| char::from(b'a' + b % 26))
        .collect();
    let schema_target = format!("{{\"{key}\":7}}\n").into_bytes();
    let artifact = schema::fit(&[&schema_target]).unwrap();
    let base = random(1 << 20);
    let mut changed = base.clone();
    changed[524288..524320].fill(42);
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    for probe in [false, true] {
        for (target, supplied, ready, mode) in [
            (b"tiny".as_slice(), None, false, Mode::Raw),
            (vec![0; 65537].as_slice(), None, false, Mode::ZstdWholeV1),
            (
                changed.as_slice(),
                Some(base.as_slice()),
                false,
                Mode::SparseV1,
            ),
            (schema_target.as_slice(), None, true, Mode::SchemaZstdV1),
        ] {
            assert_eq!(
                compare(&artifact, supplied, target, ready, probe, &mut selector),
                mode
            );
        }
    }
}

#[test]
fn boundaries_length_changes_and_context_reuse_preserve_exhaustive_minimum() {
    let artifact = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    for len in [
        0,
        1,
        2,
        31,
        32,
        33,
        63,
        64,
        65,
        255,
        256,
        4096,
        MAX_CHUNK - 1,
        MAX_CHUNK,
        MAX_CHUNK + 1,
        131071,
        131072,
        131073,
    ] {
        let base = random(len);
        let mut target = base.clone();
        if !target.is_empty() {
            target[len / 2] ^= 0xa5;
        }
        for ready in [false, true] {
            compare(&artifact, Some(&base), &target, ready, false, &mut selector);
            compare(&artifact, None, &target, ready, true, &mut selector);
        }
    }
    let base = random(65537);
    let mut appended = base.clone();
    appended.extend_from_slice(b"\0append\0\0");
    let mut inserted = vec![0];
    inserted.extend_from_slice(&base);
    let mut scattered = base.clone();
    for at in (0..scattered.len()).step_by(4096) {
        scattered[at] ^= 0x80;
    }
    for target in [
        &base[..],
        &base[..base.len() - 1],
        &appended,
        &inserted,
        &scattered,
        b"{\"value\":1e+09}\r\n",
        b"\xff\0\0\xff",
    ] {
        for probe in [false, true] {
            compare(&artifact, Some(&base), target, false, probe, &mut selector);
        }
    }
}

#[test]
fn structural_bounds_skip_only_candidates_that_cannot_win() {
    let artifact = schema::fit(&[b"{\"value\":0}\n"]).unwrap();
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    for (len, expected_bound) in [(1 << 20, 536), (16 << 20, 896)] {
        let base = random(len);
        let mut target = base.clone();
        for b in &mut target[len / 2..len / 2 + 32] {
            *b ^= 0xa5;
        }
        for probe in [false, true] {
            let plan = selector
                .prepare_warm(&target, Some(&base), false, probe)
                .unwrap();
            let r = plan.selection();
            assert_eq!(r.selected_mode, Mode::SparseV1);
            let schema = r
                .pruned
                .iter()
                .find(|p| p.mode == Mode::SchemaZstdV1)
                .unwrap();
            assert_eq!(
                schema.target_http_body_lower_bound,
                547 + u64::from(probe) * 40
            );
            if len == 16 << 20 {
                let whole = r
                    .pruned
                    .iter()
                    .find(|p| p.mode == Mode::ZstdWholeV1)
                    .unwrap();
                assert_eq!(
                    whole.target_http_body_lower_bound,
                    expected_bound + u64::from(probe) * 40
                );
            } else if zstd_whole::first_block_probe_available() {
                assert!(r.pruned.iter().any(|p| p.mode == Mode::ZstdWholeV1
                    && p.target_http_body_lower_bound > 600 + u64::from(probe) * 40));
            } else {
                assert!(r.candidates.iter().any(|c| c.mode == Mode::ZstdWholeV1));
            }
            let reference = exhaustive(&artifact, Some(&base), &target, false, probe);
            check_receipt(r, &reference, false);
        }
        // Already paid schema has a lower bound below Sparse, so it must encode.
        let warm = selector
            .prepare_warm(&target, Some(&base), true, false)
            .unwrap();
        assert!(warm
            .selection()
            .candidates
            .iter()
            .any(|c| c.mode == Mode::SchemaZstdV1));
    }
}

#[test]
fn static_tie_requires_whole_evaluation_or_a_stronger_actual_bound() {
    let len = 19 * 131072;
    let base = random(len);
    let mut target = base.clone();
    target[len / 2] ^= 0xa5;
    let artifact = schema::fit(&[]).unwrap();
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    let plan = selector
        .prepare_warm(&target, Some(&base), false, false)
        .unwrap();
    let r = plan.selection();
    // Whole lower bound512+3*19 and a one-byte COPY/LITERAL update both cost569.
    assert_eq!(
        r.candidates
            .iter()
            .find(|c| c.mode == Mode::SparseV1)
            .unwrap()
            .target_http_body_bytes,
        569
    );
    if zstd_whole::first_block_probe_available() {
        let bound = r
            .pruned
            .iter()
            .find(|p| p.mode == Mode::ZstdWholeV1)
            .unwrap();
        assert!(bound.target_http_body_lower_bound > 569);
    } else {
        assert!(r.candidates.iter().any(|c| c.mode == Mode::ZstdWholeV1));
        assert!(!r.pruned.iter().any(|p| p.mode == Mode::ZstdWholeV1));
    }
    check_receipt(
        r,
        &exhaustive(&artifact, Some(&base), &target, false, false),
        false,
    );
}

#[test]
fn raw_ties_and_old_api_results_survive_interleaved_warm_calls_and_limits() {
    let artifact = schema::fit(&[]).unwrap();
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    let mut found_tie = false;
    for len in 0..129 {
        let target = vec![0; len];
        let reference = exhaustive(&artifact, None, &target, false, false);
        let plan = selector.prepare_warm(&target, None, false, false).unwrap();
        check_receipt(plan.selection(), &reference, false);
        if reference[0].total() == reference[1].total() {
            found_tie = true;
            assert_eq!(plan.selection().selected_mode, Mode::Raw);
        }
        let old = selector.prepare(&target, false, false).unwrap();
        let fresh = PreparedSelector::new(&artifact)
            .unwrap()
            .prepare(&target, false, false)
            .unwrap();
        assert_eq!(
            format!("{:?}", old.selection()),
            format!("{:?}", fresh.selection())
        );
    }
    assert!(found_tie);
    let oversized = vec![0; client::MAX_BATCHED_OBJECT + 1];
    assert!(matches!(
        selector.prepare_warm(&oversized, None, false, false),
        Err(Error::Limit)
    ));
    assert!(matches!(
        selector.prepare_warm(b"valid", Some(&oversized), false, false),
        Err(Error::Limit)
    ));
    compare(
        &artifact,
        None,
        b"still valid\0",
        false,
        false,
        &mut selector,
    );
    assert!(PreparedSelector::new(b"invalid artifact").is_err());
}

#[test]
fn cold_schema_can_win_after_paying_seed_instead_of_greedily_using_sparse() {
    let keys: Vec<_> = (0..8)
        .map(|i| format!("stable_schema_measurement_identifier_{i}"))
        .collect();
    let record = |values: &[u64]| {
        format!(
            "{{{}}}\n",
            keys.iter()
                .zip(values)
                .map(|(k, v)| format!("\"{k}\":{v}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let training = record(&[0; 8]);
    let artifact = schema::fit(&[training.as_bytes()]).unwrap();
    let mut state = 0x8434_93ae_b2af_1d57u64;
    let mut target = Vec::new();
    for _ in 0..4096 {
        let mut values = [0; 8];
        for v in &mut values {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *v = state;
        }
        target.extend_from_slice(record(&values).as_bytes());
    }
    let base = random(target.len());
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    assert_eq!(
        compare(&artifact, Some(&base), &target, false, false, &mut selector),
        Mode::SchemaZstdV1
    );
    assert!(
        selector
            .prepare_warm(&target, Some(&base), false, false)
            .unwrap()
            .selection()
            .needs_schema_seed
    );
}
