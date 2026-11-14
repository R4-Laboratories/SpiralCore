use super::fixture::BODY_LEN;
#[cfg(test)]
use super::fixture::FRAME_LEN;

/// Produce a distinct known plaintext for every measured sender ordinal without
/// introducing another wire codec. The input is a fixed, qualified fixture.
pub fn ordinal_frame(n: u64) -> Option<Vec<u8>> {
    if !(1..=2049).contains(&n) {
        return None;
    }
    #[cfg(feature = "large-object-fixture")]
    let original = include_bytes!("../../fixtures/workload-4k-frame.bin");
    #[cfg(not(feature = "large-object-fixture"))]
    let original = include_bytes!("../../fixtures/workload-frame.bin");
    let mut frame = original.to_vec();
    let body = frame.len() - BODY_LEN;
    frame[body..body + 8].copy_from_slice(&n.to_be_bytes());
    Some(frame)
}

#[cfg(test)]
mod ordinal_tests {
    use super::*;
    #[test]
    fn every_ordinal_changes_only_the_known_payload_prefix() {
        assert!(ordinal_frame(0).is_none());
        assert!(ordinal_frame(2050).is_none());
        let first = ordinal_frame(1).unwrap();
        let start = FRAME_LEN - BODY_LEN;
        for n in 1u64..=2049 {
            let frame = ordinal_frame(n).unwrap();
            assert_eq!(frame.len(), FRAME_LEN);
            assert_eq!(&frame[..start], &first[..start]);
            assert_eq!(&frame[start..start + 8], &n.to_be_bytes());
            assert_eq!(&frame[start + 8..], &first[start + 8..]);
        }
    }
}
