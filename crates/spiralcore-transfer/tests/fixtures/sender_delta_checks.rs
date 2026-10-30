//! Byte-for-byte comparison with the pre-optimization encoder, not a second
//! implementation of the new scan algorithm. The fixture is pinned to 11fb809.
#[allow(dead_code)]
#[path = "sender_delta_baseline.rs"]
mod baseline;

fn check(base: &[u8], target: &[u8]) {
    let old = baseline::encode(base, target).unwrap();
    let new = crate::delta::encode(base, target).unwrap();
    assert_eq!(
        old.as_ref().map(|r| (&r.frames, r.operations)),
        new.as_ref().map(|r| (&r.frames, r.operations))
    );
    if let Some(recipe) = new {
        let mut decoded = Vec::new();
        let mut operations = 0;
        for frame in recipe.frames {
            crate::delta::apply(base, &mut decoded, target.len(), &mut operations, &frame).unwrap();
        }
        assert_eq!(decoded, target);
    }
}

#[test]
fn optimized_delta_preserves_all_frames_across_changes_and_boundaries() {
    let mut random = 0x32ab_a510_99cf_178bu64;
    for len in [
        0, 1, 7, 8, 9, 31, 32, 33, 63, 64, 65, 255, 256, 257, 4096, 61439, 61440, 61441, 65536,
        131073,
    ] {
        let base: Vec<u8> = (0..len)
            .map(|_| {
                random ^= random << 13;
                random ^= random >> 7;
                random ^= random << 17;
                random as u8
            })
            .collect();
        check(&base, &base);
        check(&base, &[]);
        check(&base, &[base.as_slice(), &[0, 0, 1, 0]].concat());
        if len > 0 {
            check(&base, &base[1..]);
            check(
                &base,
                &[&base[..len / 2], &[7, 0], &base[len / 2..]].concat(),
            );
            let mut edited = base.clone();
            for at in [0, len / 2, len - 1] {
                edited[at] ^= 1;
                check(&base, &edited);
            }
            for byte in edited.iter_mut().step_by(37) {
                *byte ^= 0xa5;
            }
            check(&base, &edited);
            check(&base, &base.iter().map(|b| b ^ 0xff).collect::<Vec<_>>());
        }
    }
    // Long equal ranges and periodic data are the controls harmed by the
    // earlier allocation-pruning experiment; retain them explicitly here.
    for period in [1, 7, 257] {
        let base: Vec<u8> = (0..16 * 1024 * 1024).map(|i| (i % period) as u8).collect();
        check(&base, &base);
        let mut edited = base.clone();
        edited[8 * 1024 * 1024] ^= 1;
        check(&base, &edited);
        check(&base, &[&base[1..], &[0]].concat());
    }
}
