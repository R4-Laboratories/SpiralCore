use serde_json::Value;
use sha2::{Digest, Sha256};
use spiralcore_transfer::shared_runtime::{
    replay, OutputKind, Recipe, Root, State, MAX_CHECKPOINT, MAX_OUTPUT,
};

fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../docs/network/evidence/2026-10-30-shared-runtime/reference/vectors.json"
    ))
    .unwrap()
}

fn near(actual: f64, expected: f64, label: &str) {
    let tolerance = 1e-10 * (1.0 + expected.abs());
    assert!(
        actual.is_finite() && (actual - expected).abs() <= tolerance,
        "{label}: actual={actual:.17e}, expected={expected:.17e}, tolerance={tolerance:.3e}"
    );
}

fn check_reference(state: &State, reference: &Value, label: &str) {
    assert_eq!(state.n, reference["n"].as_u64().unwrap(), "{label}");
    assert_eq!(state.width, reference["width"].as_u64().unwrap() as usize);
    for i in 0..16 {
        near(
            state.phases[i],
            reference["phases"][i].as_f64().unwrap(),
            label,
        );
        near(
            state.omegas[i],
            reference["omegas"][i].as_f64().unwrap(),
            label,
        );
    }
    for i in 0..6 {
        let expected = reference["rings"][i]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as u8)
            .collect::<Vec<_>>();
        assert_eq!(state.rings[i], expected, "{label}: ring {i}");
    }
    assert_eq!(
        state.roots.len(),
        reference["roots"].as_array().unwrap().len()
    );
    for (root, expected) in state
        .roots
        .iter()
        .zip(reference["roots"].as_array().unwrap())
    {
        assert_eq!(
            root.memory_count,
            expected["memory_count"].as_u64().unwrap()
        );
        near(root.value, expected["value"].as_f64().unwrap(), label);
        for i in 0..4 {
            near(
                root.phase_sum[i][0],
                expected["phase_sum"][i]["re"].as_f64().unwrap(),
                label,
            );
            near(
                root.phase_sum[i][1],
                expected["phase_sum"][i]["im"].as_f64().unwrap(),
                label,
            );
        }
    }
}

#[test]
fn seeded_default_ca_row_matches_pinned_python_reference() {
    let vectors = vectors();
    let expected = vectors["ca"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "184:runtime_default_48")
        .unwrap()["input"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_u64().unwrap() as u8)
        .collect::<Vec<_>>();
    let state = State::seeded(48).unwrap();
    assert_eq!(state.rings[0], expected);
}

#[test]
fn full_transition_matches_pinned_python_ca_fold_clock_and_memory_fields() {
    let fixture = vectors();
    let mut transitions = 0;
    for case in fixture["runtime_transition"].as_array().unwrap() {
        let mut state = State::seeded(case["width"].as_u64().unwrap() as usize).unwrap();
        state.fixed_dt = case["fixed_dt"].as_f64().unwrap();
        state.memory_enabled = case["memory_enabled"].as_bool().unwrap();
        let label = case["id"].as_str().unwrap();
        check_reference(&state, &case["snapshots"][0], label);
        for expected in case["snapshots"].as_array().unwrap().iter().skip(1) {
            state.step(case["payload"].as_f64().unwrap()).unwrap();
            check_reference(&state, expected, label);
            transitions += 1;
        }
    }
    assert_eq!(transitions, 123);
}

#[test]
fn every_valid_reference_ring_evolves_exactly_for_all_three_rules() {
    let fixture = vectors();
    let mut cases = 0;
    for case in fixture["ca"].as_array().unwrap() {
        let input = case["input"].as_array().unwrap();
        if input.is_empty() || input.iter().any(|x| !matches!(x.as_i64(), Some(0 | 1))) {
            continue;
        }
        let mut state = State::seeded(input.len()).unwrap();
        let ring_index = match case["rule"].as_u64().unwrap() {
            184 => 0,
            110 => 2,
            30 => 4,
            _ => unreachable!(),
        };
        state.rings[ring_index] = input.iter().map(|x| x.as_u64().unwrap() as u8).collect();
        for expected in case["states"].as_array().unwrap().iter().skip(1) {
            state.step(0.0).unwrap();
            let expected = expected
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as u8)
                .collect::<Vec<_>>();
            assert_eq!(state.rings[ring_index], expected, "{}", case["id"]);
        }
        cases += 1;
    }
    assert_eq!(cases, 57);
}

#[test]
fn checkpoints_roundtrip_every_resume_boundary_without_fidelity_loss() {
    let mut uninterrupted = State::seeded(48).unwrap();
    let mut resumed = State::seeded(48).unwrap();
    for step in 1..=81 {
        let payload = f64::from(step);
        uninterrupted.step(payload).unwrap();
        resumed.step(payload).unwrap();
        let bytes = resumed.encode().unwrap();
        resumed = State::decode(&bytes).unwrap();
        assert_eq!(resumed.encode().unwrap(), bytes);
        assert_eq!(bytes, uninterrupted.encode().unwrap(), "resume at {step}");
    }
}

#[test]
fn fold_values_preserve_observed_unweighted_summary_semantics() {
    let mut state = State::seeded(48).unwrap();
    for value in 1..=9 {
        state.step(f64::from(value)).unwrap();
    }
    assert_eq!(state.roots.len(), 1);
    assert_eq!(state.roots[0].memory_count, 9);
    near(state.roots[0].value, 6.2, "historical fold");
    assert_ne!(state.roots[0].value, 5.0);
}

#[test]
fn memory_coupler_affects_phases_without_changing_ca_or_leaf_count() {
    let mut enabled = State::seeded(48).unwrap();
    let mut disabled = State::decode(&enabled.encode().unwrap()).unwrap();
    disabled.memory_enabled = false;
    for _ in 0..9 {
        enabled.step(1.).unwrap();
        disabled.step(1.).unwrap();
    }
    assert_eq!(enabled.rings, disabled.rings);
    assert_eq!(
        enabled.roots[0].memory_count,
        disabled.roots[0].memory_count
    );
    assert!(enabled
        .phases
        .iter()
        .zip(disabled.phases)
        .any(|(a, b)| (a - b).abs() > 1e-8));
}

#[test]
fn exact_replay_final_checkpoint_matches_explicit_steps() {
    let checkpoint = State::seeded(256).unwrap().encode().unwrap();
    let recipe = Recipe {
        steps: 27,
        payload: -0.25,
        output: OutputKind::FinalCheckpoint,
    };
    let output = replay(&checkpoint, &recipe, MAX_OUTPUT).unwrap();
    let mut expected = State::decode(&checkpoint).unwrap();
    for _ in 0..27 {
        expected.step(-0.25).unwrap();
    }
    assert_eq!(output, expected.encode().unwrap());
}

#[test]
fn zero_step_final_replay_is_identity_and_zero_step_trace_is_header_only() {
    let checkpoint = State::seeded(1).unwrap().encode().unwrap();
    let mut recipe = Recipe {
        steps: 0,
        payload: 1.,
        output: OutputKind::FinalCheckpoint,
    };
    assert_eq!(
        replay(&checkpoint, &recipe, MAX_OUTPUT).unwrap(),
        checkpoint
    );
    recipe.output = OutputKind::Trace;
    assert_eq!(
        replay(&checkpoint, &recipe, MAX_OUTPUT).unwrap(),
        b"SCTRAC01"
    );
}

#[test]
fn trace_contains_every_complete_post_step_checkpoint_in_order() {
    let mut state = State::seeded(48).unwrap();
    let checkpoint = state.encode().unwrap();
    let recipe = Recipe {
        steps: 9,
        payload: 2.,
        output: OutputKind::Trace,
    };
    let trace = replay(&checkpoint, &recipe, MAX_OUTPUT).unwrap();
    assert_eq!(&trace[..8], b"SCTRAC01");
    let mut remaining = &trace[8..];
    for _ in 0..9 {
        let length = u32::from_le_bytes(remaining[..4].try_into().unwrap()) as usize;
        assert!(length <= MAX_CHECKPOINT);
        let serialized = &remaining[4..4 + length];
        state.step(2.).unwrap();
        assert_eq!(serialized, state.encode().unwrap());
        assert_eq!(
            State::decode(serialized).unwrap().encode().unwrap(),
            serialized
        );
        remaining = &remaining[4 + length..];
    }
    assert!(remaining.is_empty());
}

#[test]
fn checkpoint_rejects_every_truncation_trailing_bytes_and_bad_magic() {
    let mut state = State::seeded(48).unwrap();
    state.step(1.).unwrap();
    let bytes = state.encode().unwrap();
    for end in 0..bytes.len() {
        assert!(
            State::decode(&bytes[..end]).is_err(),
            "accepted prefix {end}"
        );
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(State::decode(&trailing).is_err());
    let mut corrupted = bytes;
    corrupted[0] ^= 1;
    assert!(State::decode(&corrupted).is_err());
    assert!(State::decode(&vec![0; MAX_CHECKPOINT + 1]).is_err());
}

#[test]
fn recipe_roundtrip_rejects_truncation_trailing_and_unknown_header() {
    for output in [OutputKind::FinalCheckpoint, OutputKind::Trace] {
        let recipe = Recipe {
            steps: 17,
            payload: -1.,
            output,
        };
        let bytes = recipe.encode().unwrap();
        let decoded = Recipe::decode(&bytes).unwrap();
        assert_eq!(decoded.encode().unwrap(), bytes);
        for end in 0..bytes.len() {
            assert!(Recipe::decode(&bytes[..end]).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(Recipe::decode(&trailing).is_err());
        let mut corrupt = bytes;
        corrupt[0] ^= 1;
        assert!(Recipe::decode(&corrupt).is_err());
    }
}

#[test]
fn invalid_dimensions_rings_and_nonfinite_state_are_refused() {
    assert!(State::seeded(0).is_err());
    assert!(State::seeded(4097).is_err());
    assert!(State::seeded(usize::MAX).is_err());
    let mut state = State::seeded(48).unwrap();
    state.rings[0][0] = 2;
    assert!(state.encode().is_err());
    assert!(state.step(1.).is_err());
    state.rings[0][0] = 0;
    state.rings[1].pop();
    assert!(state.encode().is_err());
    let mut state = State::seeded(48).unwrap();
    state.phases[0] = f64::NAN;
    assert!(state.encode().is_err());
    assert!(state.step(1.).is_err());
}

#[test]
fn invalid_frequency_coupling_clock_root_and_payload_are_refused() {
    let base = State::seeded(48).unwrap().encode().unwrap();
    for change in [
        |s: &mut State| s.omegas[0] = f64::INFINITY,
        |s: &mut State| s.memory_gain = 17.,
        |s: &mut State| s.k_coupling = -17.,
        |s: &mut State| s.kuramoto_k = f64::NAN,
        |s: &mut State| s.fixed_dt = 0.,
        |s: &mut State| s.fixed_dt = 5.1,
        |s: &mut State| s.phases[0] = -0.1,
    ] {
        let mut state = State::decode(&base).unwrap();
        change(&mut state);
        assert!(state.encode().is_err());
        assert!(state.step(1.).is_err());
    }
    for payload in [f64::NAN, f64::INFINITY, 1e12 + 1.] {
        let mut state = State::decode(&base).unwrap();
        assert!(state.step(payload).is_err());
        assert_eq!(
            state.encode().unwrap(),
            base,
            "invalid payload must not mutate state"
        );
        assert!(Recipe {
            steps: 1,
            payload,
            output: OutputKind::FinalCheckpoint
        }
        .encode()
        .is_err());
    }
}

#[test]
fn logical_step_root_and_replay_limits_are_bounded() {
    let mut state = State::seeded(1).unwrap();
    state.n = 1_000_000_000;
    assert!(state.step(1.).is_err());
    state.n = 82;
    state.roots = (0..82)
        .map(|_| Root {
            value: 1.,
            memory_count: 1,
            phase_sum: [[0.; 2]; 4],
        })
        .collect();
    assert!(state.encode().is_err());
    let checkpoint = State::seeded(4096).unwrap().encode().unwrap();
    assert!(checkpoint.len() <= MAX_CHECKPOINT);
    let large_trace = Recipe {
        steps: 9,
        payload: 1.,
        output: OutputKind::Trace,
    };
    assert!(replay(&checkpoint, &large_trace, checkpoint.len()).is_err());
    assert!(replay(&checkpoint, &large_trace, MAX_OUTPUT + 1).is_err());
    let too_many_steps = Recipe {
        steps: 4097,
        payload: 1.,
        output: OutputKind::FinalCheckpoint,
    };
    assert!(too_many_steps.encode().is_err());
    assert!(replay(&checkpoint, &too_many_steps, MAX_OUTPUT).is_err());
    let valid = Recipe {
        steps: 0,
        payload: 1.,
        output: OutputKind::FinalCheckpoint,
    };
    assert!(replay(&checkpoint, &valid, checkpoint.len() - 1).is_err());
}

#[test]
fn root_counts_and_phase_bounds_are_validated() {
    let mut state = State::seeded(48).unwrap();
    state.step(1.).unwrap();
    let checkpoint = state.encode().unwrap();
    for change in [
        |s: &mut State| s.roots[0].memory_count = 0,
        |s: &mut State| s.roots[0].memory_count = 2,
        |s: &mut State| s.roots[0].phase_sum[0][0] = 2.,
        |s: &mut State| s.roots[0].value = f64::NAN,
    ] {
        let mut mutated = State::decode(&checkpoint).unwrap();
        change(&mut mutated);
        assert!(mutated.encode().is_err());
    }
}

#[test]
fn checkpoint_rejects_noncanonical_float_padding_flags_and_root_count() {
    let checkpoint = State::seeded(1).unwrap().encode().unwrap();
    for (offset, value) in [(18, 2), (19, 1), (310, 1)] {
        let mut bytes = checkpoint.clone();
        bytes[offset] = value;
        assert!(
            State::decode(&bytes).is_err(),
            "noncanonical byte at {offset}"
        );
    }
    for float_bits in [
        (-0.0f64).to_bits(),
        f64::NAN.to_bits(),
        f64::INFINITY.to_bits(),
    ] {
        let mut bytes = checkpoint.clone();
        bytes[28..36].copy_from_slice(&float_bits.to_le_bytes());
        assert!(State::decode(&bytes).is_err());
    }
    let mut bytes = checkpoint;
    bytes[308..310].copy_from_slice(&82u16.to_le_bytes());
    assert!(State::decode(&bytes).is_err());
}

#[test]
fn recipe_rejects_noncanonical_payload_reserved_bits_and_output_kind() {
    let recipe = Recipe {
        steps: 1,
        payload: 0.,
        output: OutputKind::FinalCheckpoint,
    };
    let canonical = recipe.encode().unwrap();
    for (offset, value) in [(20, 2), (21, 1), (22, 1), (23, 1)] {
        let mut bytes = canonical.clone();
        bytes[offset] = value;
        assert!(Recipe::decode(&bytes).is_err());
    }
    for float_bits in [
        (-0.0f64).to_bits(),
        f64::NAN.to_bits(),
        f64::INFINITY.to_bits(),
    ] {
        let mut bytes = canonical.clone();
        bytes[12..20].copy_from_slice(&float_bits.to_le_bytes());
        assert!(Recipe::decode(&bytes).is_err());
    }
    let mut negative_zero = recipe;
    negative_zero.payload = -0.;
    assert_eq!(negative_zero.encode().unwrap(), canonical);
    let mut bytes = canonical;
    bytes[8..12].copy_from_slice(&4097u32.to_le_bytes());
    assert!(Recipe::decode(&bytes).is_err());
}

#[test]
fn native_canonical_checkpoint_and_trace_vectors_are_stable() {
    let mut state = State::seeded(48).unwrap();
    state.fixed_dt = 0.05;
    let initial = state.encode().unwrap();
    let mut records = Vec::new();
    let mut record = |label: &str, bytes: Vec<u8>| {
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        records.push(serde_json::json!({
            "id": label, "bytes": bytes.len(),
            "sha256": spiralcore_transfer::hex(&digest),
            "object_id": spiralcore_transfer::hex(&spiralcore_transfer::object_id(&bytes)),
        }));
    };
    record("width48_dt005_n0", initial.clone());
    for step in 1..=81 {
        state.step(1.0).unwrap();
        if step == 9 || step == 81 {
            record(&format!("width48_dt005_n{step}"), state.encode().unwrap());
        }
    }
    record(
        "width48_dt005_trace9",
        replay(
            &initial,
            &Recipe {
                steps: 9,
                payload: 1.,
                output: OutputKind::Trace,
            },
            MAX_OUTPUT,
        )
        .unwrap(),
    );
    let records = Value::Array(records);
    eprintln!("NATIVE_CANONICAL_VECTORS={records}");
    let expected: Value = serde_json::from_str(include_str!(
        "../../../docs/network/evidence/2026-10-30-shared-runtime/reference/native-vectors.json"
    ))
    .unwrap();
    assert_eq!(records, expected);
}

#[test]
fn splitting_replay_across_independent_checkpoint_receivers_is_exact() {
    let mut seeded = State::seeded(48).unwrap();
    seeded.fixed_dt = 0.05;
    let initial = seeded.encode().unwrap();
    let whole = replay(
        &initial,
        &Recipe {
            steps: 81,
            payload: 1.,
            output: OutputKind::FinalCheckpoint,
        },
        MAX_OUTPUT,
    )
    .unwrap();
    for boundary in [1, 2, 3, 8, 9, 26, 27, 80] {
        let middle = replay(
            &initial,
            &Recipe {
                steps: boundary,
                payload: 1.,
                output: OutputKind::FinalCheckpoint,
            },
            MAX_OUTPUT,
        )
        .unwrap();
        let final_checkpoint = replay(
            &middle,
            &Recipe {
                steps: 81 - boundary,
                payload: 1.,
                output: OutputKind::FinalCheckpoint,
            },
            MAX_OUTPUT,
        )
        .unwrap();
        assert_eq!(final_checkpoint, whole, "split at step {boundary}");
    }
}

#[test]
fn historical_root_retention_cap_drops_oldest_summary_without_exceeding_bounds() {
    let mut state = State::seeded(1).unwrap();
    state.n = 81;
    state.roots = (1..=81)
        .map(|value| Root {
            value: f64::from(value),
            memory_count: 1,
            phase_sum: [[0.; 2]; 4],
        })
        .collect();
    let checkpoint = state.encode().unwrap();
    state.step(82.).unwrap();
    assert_eq!(state.roots.len(), 81);
    assert_eq!(state.roots.first().unwrap().value, 2.);
    assert_eq!(state.roots.last().unwrap().value, 82.);
    assert_eq!(
        state
            .roots
            .iter()
            .map(|root| root.memory_count)
            .sum::<u64>(),
        81
    );
    assert_eq!(state.n, 82);
    let replayed = replay(
        &checkpoint,
        &Recipe {
            steps: 1,
            payload: 82.,
            output: OutputKind::FinalCheckpoint,
        },
        MAX_OUTPUT,
    )
    .unwrap();
    assert_eq!(replayed, state.encode().unwrap());
}

#[test]
fn trace_reserves_exact_required_output_instead_of_entire_admission_ceiling() {
    let checkpoint = State::seeded(48).unwrap().encode().unwrap();
    let recipe = Recipe {
        steps: 9,
        payload: 1.,
        output: OutputKind::Trace,
    };
    let trace = replay(&checkpoint, &recipe, MAX_OUTPUT).unwrap();
    assert_eq!(
        trace.capacity(),
        trace.len(),
        "qualified allocator capacity should track required bytes, not16MiB ceiling"
    );
}

#[test]
fn replay_exact_output_budget_succeeds_and_one_byte_short_fails() {
    for width in [1, 48, 4096] {
        let checkpoint = State::seeded(width).unwrap().encode().unwrap();
        for steps in [0, 1, 3, 9, 81] {
            for output in [OutputKind::FinalCheckpoint, OutputKind::Trace] {
                let recipe = Recipe {
                    steps,
                    payload: 1.,
                    output,
                };
                let expected = replay(&checkpoint, &recipe, MAX_OUTPUT).unwrap();
                assert_eq!(
                    replay(&checkpoint, &recipe, expected.len()).unwrap(),
                    expected
                );
                assert!(replay(&checkpoint, &recipe, expected.len() - 1).is_err());
            }
        }
    }
}
