//! Private proof-bound checks over positive operations produced by the planner.
use super::*;

fn check_bound(ops: &[Op], target: &[u8]) -> Option<Recipe> {
    let bound = lower_bound(ops).unwrap();
    let packed = pack(ops, target).unwrap();
    if let Some(recipe) = &packed {
        assert!(
            bound <= cost(recipe),
            "bound {bound} exceeds packed cost {}",
            cost(recipe)
        );
    }
    packed
}

#[test]
fn bound_empty_positive_copy_literal_and_overflow() {
    assert_eq!(lower_bound(&[]).unwrap(), 0);
    assert_eq!(lower_bound(&[Op::Copy(0, 1)]).unwrap(), 150);
    assert_eq!(lower_bound(&[Op::Literal(0, 1)]).unwrap(), 143);
    assert_eq!(
        lower_bound(&[Op::Literal(0, usize::MAX)]),
        Err(Error::Limit)
    );
    assert_eq!(
        lower_bound(&[
            Op::Literal(0, usize::MAX / 2 + 1),
            Op::Literal(0, usize::MAX / 2 + 1)
        ]),
        Err(Error::Limit)
    );
    assert_eq!(
        lower_bound(&[Op::Copy(usize::MAX, usize::MAX)]).unwrap(),
        150
    );
}

#[test]
fn positive_operation_bound_never_exceeds_materialized_cost() {
    let target = vec![0x39; MAX_CHUNK * 2 + 1];
    let mut state = 0x7342_2819u64;
    for count in [1, 2, 3, 31, 32, 255, 256, 257, 8191, 8192, 8193] {
        for seed in 0..8 {
            let mut ops = Vec::with_capacity(count);
            for position in 0..count {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let len = 1 + ((state >> 8) as usize % 64);
                if (position + seed) % 3 == 0 {
                    ops.push(Op::Copy(position, len));
                } else {
                    let start = state as usize % (target.len() - len);
                    ops.push(Op::Literal(start, len));
                }
            }
            let packed = check_bound(&ops, &target);
            if count > MAX_OPS {
                assert!(packed.is_none());
            }
        }
    }
}

#[test]
fn pack_op_limits_and_literal_splitting_remain_in_force() {
    let target = vec![0; MAX_CHUNK + 1];
    for count in [255, 256, 257, 8191, 8192, 8193] {
        let ops = vec![Op::Copy(0, 1); count];
        match check_bound(&ops, &target) {
            Some(recipe) => {
                assert!(count <= MAX_OPS);
                assert_eq!(recipe.operations, count);
                assert_eq!(recipe.frames.len(), count.div_ceil(FRAME_OPS));
            }
            None => assert!(count > MAX_OPS),
        }
    }
    for n in [MAX_CHUNK - 1, MAX_CHUNK, MAX_CHUNK + 1] {
        let recipe = check_bound(&[Op::Literal(0, n)], &target).unwrap();
        assert_eq!(recipe.operations, n.div_ceil(MAX_CHUNK));
        assert_eq!(recipe.frames.len(), n.div_ceil(MAX_CHUNK));
    }
    for copies in [8190, 8191] {
        let mut ops = vec![Op::Copy(0, 1); copies];
        ops.push(Op::Literal(0, MAX_CHUNK + 1));
        let packed = check_bound(&ops, &target);
        if copies == 8190 {
            assert_eq!(packed.unwrap().operations, MAX_OPS);
        } else {
            assert!(
                packed.is_none(),
                "literal splitting counts against total operations"
            );
        }
    }
}

#[test]
fn strict_tie_and_raw_threshold_costs_are_not_rounded() {
    let target = vec![1; 173];
    let simple = [Op::Copy(0, 13), Op::Literal(13, 32), Op::Copy(45, 128)];
    let aligned = [Op::Literal(0, 45), Op::Copy(45, 128)];
    let a = check_bound(&simple, &target).unwrap();
    let b = check_bound(&aligned, &target).unwrap();
    assert_eq!(cost(&a), 200);
    assert_eq!(cost(&b), 200);
    assert_ne!(
        a.frames, b.frames,
        "tie must retain the original simple bytes"
    );
    assert_eq!(lower_bound(&simple).unwrap(), cost(&a));
    assert_eq!(lower_bound(&aligned).unwrap(), cost(&b));
    for n in [48usize, 49, 50] {
        let bound = lower_bound(&[Op::Copy(0, n)]).unwrap();
        let raw = n + n.div_ceil(MAX_CHUNK) * 133;
        assert_eq!(bound + 32 < raw, n == 50);
    }
}

#[test]
fn current_frame_and_literal_caps_make_body_size_branch_unreachable() {
    // Every operation contributes at most 13 bytes of non-literal content.
    // Even this loose combined maximum is below the body's allowed ceiling.
    // A generated positive-op mixture cannot hit body size first with these
    // constants; preserving the defensive check is still required.
    let loose_body_maximum = 9 + 13 * FRAME_OPS + MAX_CHUNK;
    assert_eq!(loose_body_maximum, 64_777);
    assert!(loose_body_maximum < MAX_FRAME - 56);
    let target = vec![0; MAX_CHUNK];
    let mut ops = vec![Op::Copy(0, 1); FRAME_OPS - 1];
    ops.push(Op::Literal(0, MAX_CHUNK));
    let recipe = check_bound(&ops, &target).unwrap();
    assert_eq!(recipe.frames.len(), 1);
    assert!(recipe.frames[0].len() <= loose_body_maximum);
}
