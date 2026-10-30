use spiralcore_transfer::shared_runtime::{replay, OutputKind, Recipe, State, MAX_OUTPUT};

fn scalar_replay(checkpoint: &[u8], recipe: &Recipe) -> Vec<u8> {
    let mut state = State::decode(checkpoint).unwrap();
    let mut trace = b"SCTRAC01".to_vec();
    for _ in 0..recipe.steps {
        state.step(recipe.payload).unwrap();
        if recipe.output == OutputKind::Trace {
            let encoded = state.encode().unwrap();
            trace.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
            trace.extend_from_slice(&encoded);
        }
    }
    match recipe.output {
        OutputKind::FinalCheckpoint => state.encode().unwrap(),
        OutputKind::Trace => trace,
    }
}

#[test]
fn packed_replay_is_byte_identical_to_public_scalar_steps() {
    for width in [1, 2, 7, 8, 63, 64, 65, 127, 4095, 4096] {
        let mut state = State::seeded(width).unwrap();
        state.fixed_dt = 0.05;
        // Resume a populated state, with varied rings rather than only the seed.
        for _ in 0..8 {
            state.step(-3.25).unwrap();
        }
        let mut seed = 0x5a31_792e_d18a_493fu64;
        for row in &mut state.rings {
            for bit in row {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                *bit = (seed & 1) as u8;
            }
        }
        let checkpoint = state.encode().unwrap();
        for steps in [0, 1, 2, 3, 9, 81] {
            for output in [OutputKind::FinalCheckpoint, OutputKind::Trace] {
                let recipe = Recipe {
                    steps,
                    payload: 7.5,
                    output,
                };
                let expected = scalar_replay(&checkpoint, &recipe);
                assert_eq!(
                    replay(&checkpoint, &recipe, MAX_OUTPUT).unwrap(),
                    expected,
                    "width={width}, steps={steps}, output={output:?}"
                );
                assert_eq!(
                    replay(&checkpoint, &recipe, expected.len()).unwrap(),
                    expected
                );
                assert!(replay(&checkpoint, &recipe, expected.len() - 1).is_err());
            }
        }
    }
}

#[test]
fn packed_replay_preserves_extreme_valid_clock_and_memory_settings() {
    for (dt, memory_enabled) in [(0.00001, false), (0.001, true), (0.5, false), (5.0, true)] {
        let mut state = State::seeded(65).unwrap();
        state.fixed_dt = dt;
        state.memory_enabled = memory_enabled;
        state.memory_gain = -16.0;
        state.k_coupling = 16.0;
        state.kuramoto_k = -16.0;
        state.omegas = std::array::from_fn(|i| if i % 2 == 0 { -16.0 } else { 16.0 });
        let checkpoint = state.encode().unwrap();
        let recipe = Recipe {
            steps: 81,
            payload: -1e12,
            output: OutputKind::Trace,
        };
        assert_eq!(
            replay(&checkpoint, &recipe, MAX_OUTPUT).unwrap(),
            scalar_replay(&checkpoint, &recipe)
        );
    }
}
