use super::*;
use crate::{
    schema,
    shared_runtime::{OutputKind, Recipe, State},
    shared_transfer::PreparedPrediction,
};

fn prediction() -> PreparedPrediction {
    PreparedPrediction::new(
        &State::seeded(48).unwrap().encode().unwrap(),
        &Recipe {
            steps: 81,
            payload: 1.25,
            output: OutputKind::Trace,
        },
    )
    .unwrap()
}
fn opaque() -> Vec<u8> {
    let mut seed = 0x1234_5678_9abc_def0u64;
    (0..4096)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u8
        })
        .collect()
}

#[test]
fn cpu_conscious_skips_real_compression_only_for_zero_useful_records() {
    let artifact = schema::fit(&[b"{\"identifier\":0,\"status\":true}\n"]).unwrap();
    let p = prediction();
    let mut selector = PreparedSelector::new(&artifact).unwrap();
    for probe in [false, true] {
        let target = opaque();
        let before = selector.schema.compression_calls;
        let (_, receipt) = selector
            .prepare_combined_acknowledged(
                &target,
                None,
                &p,
                true,
                probe,
                SelectionPolicy::CpuConscious,
            )
            .unwrap();
        assert_eq!(selector.schema.compression_calls, before);
        assert_eq!(receipt.policy, SelectionPolicy::CpuConscious);
        assert_eq!(
            receipt.policy_skipped,
            vec![PolicySkippedCandidate {
                mode: Mode::SchemaZstdV1,
                reason: PolicySkipReason::NoUsefulTemplateRecords,
                useful_template_records: 0,
            }]
        );
        assert!(!receipt.pruned.iter().any(|c| c.mode == Mode::SchemaZstdV1));
        assert!(!receipt
            .candidates
            .iter()
            .any(|c| c.mode == Mode::SchemaZstdV1));
        let mut mixed = b"{\"identifier\":7,\"status\":false}\n".to_vec();
        mixed.extend_from_slice(&target);
        let (_, receipt) = selector
            .prepare_combined_acknowledged(
                &mixed,
                None,
                &p,
                true,
                probe,
                SelectionPolicy::CpuConscious,
            )
            .unwrap();
        assert_eq!(selector.schema.compression_calls, before + 1);
        assert!(receipt.policy_skipped.is_empty());
        assert!(receipt
            .candidates
            .iter()
            .any(|c| c.mode == Mode::SchemaZstdV1));
        let (_, receipt) = selector
            .prepare_combined_acknowledged(
                &target,
                None,
                &p,
                true,
                probe,
                SelectionPolicy::Exhaustive,
            )
            .unwrap();
        assert_eq!(selector.schema.compression_calls, before + 2);
        assert!(receipt.policy_skipped.is_empty());
        let (_, receipt) = selector
            .prepare_combined_acknowledged(
                &target,
                None,
                &p,
                false,
                probe,
                SelectionPolicy::CpuConscious,
            )
            .unwrap();
        assert_eq!(selector.schema.compression_calls, before + 2);
        assert!(receipt.policy_skipped.is_empty());
    }
}
