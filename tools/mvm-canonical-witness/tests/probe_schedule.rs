use spiralcore_mvm_canonical_witness::receive_probe_offsets;
use std::time::Duration;
#[test]
fn external_receive_probes_are_bounded_escalating_and_inside_original_deadline() {
    let offsets = receive_probe_offsets(Duration::from_secs(60));
    assert_eq!(offsets.len(), 5);
    assert_eq!(offsets[0], Duration::ZERO);
    assert!(offsets.windows(2).all(|x| x[0] < x[1]));
    assert!(offsets.iter().all(|x| *x < Duration::from_secs(60)));
    let short = receive_probe_offsets(Duration::from_millis(500));
    assert!(short.len() <= 5 && short.iter().all(|x| *x < Duration::from_millis(500)));
}
