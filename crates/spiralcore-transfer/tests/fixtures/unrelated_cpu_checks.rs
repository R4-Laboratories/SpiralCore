use super::*;
#[allow(dead_code)]
#[path = "unrelated_delta_before.rs"]
mod before;

fn scalar(a: &[u8], b: &[u8], from: usize) -> Option<(usize, usize)> {
    let common = a.len().min(b.len());
    let mut i = from;
    while i < common {
        while i < common && a[i] != b[i] {
            i += 1;
        }
        let start = i;
        while i < common && a[i] == b[i] {
            i += 1;
        }
        if i - start >= 32 {
            return Some((start, i));
        }
    }
    None
}

#[test]
fn unrelated_cpu_anchor_search_matches_first_maximal_scalar_run() {
    for n in [0, 1, 15, 16, 30, 31, 32, 33, 63, 64, 65, 127, 128, 129, 257] {
        let a: Vec<u8> = (0..n).map(|i| i as u8).collect();
        for at in 0..=n {
            for len in [0, 1, 15, 16, 30, 31, 32, 33, 63, 64, 65, 129] {
                let mut b: Vec<u8> = a.iter().map(|v| v ^ 0xff).collect();
                let end = (at + len).min(n);
                b[at..end].copy_from_slice(&a[at..end]);
                for from in [0, at.saturating_sub(1), at, at.saturating_add(1).min(n), n] {
                    assert_eq!(
                        next_copy_run(&a, &b, from),
                        scalar(&a, &b, from),
                        "n={n},at={at},len={len},from={from}"
                    );
                }
            }
        }
    }
    // Multiple short runs, including aligned anchors, must not hide a later
    // qualifying run. Different slice lengths retain the same common ceiling.
    let a = vec![7; 4097];
    for period in [2, 7, 31, 32, 33, 63, 64, 65] {
        let mut b = vec![7; 4096];
        for at in (0..b.len()).step_by(period) {
            b[at] ^= 1;
        }
        for from in 0..128 {
            assert_eq!(next_copy_run(&a, &b, from), scalar(&a, &b, from));
        }
    }
    let a = vec![7; 64];
    let mut b = a.clone();
    b[31] = 6;
    assert_eq!(next_copy_run(&a, &b, 0), Some((32, 64)));
}

fn check(base: &[u8], target: &[u8]) {
    let expected = before::encode(base, target).unwrap();
    let actual = encode(base, target).unwrap();
    assert_eq!(
        actual.as_ref().map(|r| (&r.frames, r.operations)),
        expected.as_ref().map(|r| (&r.frames, r.operations))
    );
    if let Some(recipe) = actual {
        let mut out = Vec::new();
        let mut ops = 0;
        for frame in recipe.frames {
            apply(base, &mut out, target.len(), &mut ops, &frame).unwrap();
        }
        assert_eq!(out, target);
    }
}

#[test]
fn unrelated_cpu_preserves_frozen_frames_on_short_runs_edges_and_operation_cap() {
    for n in [
        0, 1, 31, 32, 33, 63, 64, 65, 4096, 61439, 61440, 61441, 131073,
    ] {
        let base: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
        let unrelated: Vec<u8> = base.iter().map(|v| v ^ 0xff).collect();
        check(&base, &unrelated);
        check(&base, &base);
        for offset in [0, 1, 7, 31, 32, 33, 63, 64, 65, 127] {
            for length in [31, 32, 33, 63, 64, 65] {
                if offset + length > n {
                    continue;
                }
                let mut target = unrelated.clone();
                target[offset..offset + length].copy_from_slice(&base[offset..offset + length]);
                check(&base, &target);
            }
        }
        if n > 0 {
            check(&base, &base[1..]);
            check(&base, &[base.as_slice(), b"\0append\0"].concat());
        }
    }
    for runs in [4095, 4096, 4097, 8193] {
        let base = vec![1; runs * 33];
        let mut target = base.clone();
        for at in (32..target.len()).step_by(33) {
            target[at] = 2;
        }
        check(&base, &target);
    }
}
