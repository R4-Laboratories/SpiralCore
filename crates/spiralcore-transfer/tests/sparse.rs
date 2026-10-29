use spiralcore_transfer::{delta, Error};

fn random(n: usize) -> Vec<u8> {
    let mut s = 42u64;
    (0..n)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s as u8
        })
        .collect()
}

#[test]
fn prefix_insertion_uses_a_small_exact_recipe() {
    let base = random(1 << 20);
    let mut target = vec![0];
    target.extend_from_slice(&base);
    let recipe = delta::encode(&base, &target).unwrap().unwrap();
    assert!(recipe.frames.iter().map(Vec::len).sum::<usize>() < 64);
    let mut out = Vec::with_capacity(target.len());
    let mut ops = 0;
    for frame in &recipe.frames {
        delta::apply(&base, &mut out, target.len(), &mut ops, frame).unwrap();
    }
    assert_eq!(out, target);
    assert_eq!(ops, recipe.operations);
}

#[test]
fn malformed_recipe_is_atomic() {
    let mut out = vec![1];
    let mut ops = 0;
    let mut body = vec![3];
    body.extend_from_slice(&2u32.to_le_bytes());
    body.extend_from_slice(&1u16.to_le_bytes());
    body.extend_from_slice(&2u16.to_le_bytes());
    body.push(1);
    body.extend_from_slice(&1u32.to_le_bytes());
    body.push(9);
    body.push(0);
    body.extend_from_slice(&u64::MAX.to_le_bytes());
    body.extend_from_slice(&1u32.to_le_bytes());
    assert!(matches!(
        delta::apply(&[0], &mut out, 3, &mut ops, &body),
        Err(Error::Limit | Error::Malformed)
    ));
    assert_eq!(out, [1]);
    assert_eq!(ops, 0);
}

#[test]
fn edits_lengths_and_fallback_preserve_every_byte() {
    for n in [0, 1, 31, 32, 33, 96, 61439, 61440, 61441, 131073] {
        let base = random(n);
        let mut cases = vec![base.clone(), vec![], vec![0; n], random(n + 103)];
        let mut prefix = vec![7];
        prefix.extend_from_slice(&base);
        cases.push(prefix);
        let mut appended = base.clone();
        appended.extend_from_slice(&[0, 3, 0]);
        cases.push(appended);
        if n > 0 {
            cases.push(base[1..].to_vec());
            cases.push(base[..n - 1].to_vec());
            let mut changed = base.clone();
            changed[n / 2] ^= 0x5a;
            cases.push(changed);
            let mut scattered = base.clone();
            for x in scattered.iter_mut().step_by(1024) {
                *x ^= 0x71;
            }
            cases.push(scattered);
        }
        for target in cases {
            let Some(recipe) = delta::encode(&base, &target).unwrap() else {
                continue;
            };
            let mut output = Vec::new();
            let mut ops = 0;
            for f in &recipe.frames {
                delta::apply(&base, &mut output, target.len(), &mut ops, f).unwrap();
            }
            assert_eq!(output, target, "length {n}");
            assert_eq!(ops, recipe.operations);
        }
    }
    let base = random(1 << 20);
    let target = base.iter().map(|b| b ^ 0xff).collect::<Vec<_>>();
    assert!(delta::encode(&base, &target).unwrap().is_none());
}

fn copy_frame(offset: u64, len: u32) -> Vec<u8> {
    let mut b = vec![3];
    b.extend_from_slice(&len.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.push(0);
    b.extend_from_slice(&offset.to_le_bytes());
    b.extend_from_slice(&len.to_le_bytes());
    b
}
#[test]
fn hostile_versions_lengths_ops_and_truncation_fail_closed() {
    let valid = copy_frame(0, 64);
    let mut bad = Vec::new();
    let mut version = valid.clone();
    version[5] = 2;
    bad.push(version);
    let mut tag = valid.clone();
    tag[9] = 9;
    bad.push(tag);
    let mut encoding = valid.clone();
    encoding[0] = 2;
    bad.push(encoding);
    let mut decoded = valid.clone();
    decoded[1] = 63;
    bad.push(decoded);
    let mut trailing = valid.clone();
    trailing.push(0);
    bad.push(trailing);
    let mut count = valid.clone();
    count[7..9].copy_from_slice(&257u16.to_le_bytes());
    bad.push(count);
    bad.push(copy_frame(1, 64));
    bad.push(copy_frame(u64::MAX, 1));
    bad.push(copy_frame(0, 0));
    for i in 0..valid.len() {
        bad.push(valid[..i].to_vec());
    }
    for frame in bad {
        let mut out = Vec::new();
        let mut ops = 0;
        assert!(delta::apply(&[0; 64], &mut out, 64, &mut ops, &frame).is_err());
        assert!(out.is_empty());
        assert_eq!(ops, 0);
    }
    let mut out = Vec::new();
    let mut ops = delta::MAX_OPS;
    assert_eq!(
        delta::apply(&[0; 64], &mut out, 64, &mut ops, &valid),
        Err(Error::Limit)
    );
    assert!(out.is_empty());
    let mut ops = 0;
    assert_eq!(
        delta::apply(&[0; 64], &mut out, 63, &mut ops, &valid),
        Err(Error::Limit)
    );
}

#[test]
fn literal_frames_split_and_large_copy_remains_small() {
    let base = random(1 << 20);
    let mut target = base.clone();
    target[100000..300000].fill(0);
    let recipe = delta::encode(&base, &target).unwrap().unwrap();
    assert!(recipe.frames.len() > 1);
    let mut out = Vec::new();
    let mut ops = 0;
    for f in &recipe.frames {
        assert!(f.len() <= 65536 - 56);
        delta::apply(&base, &mut out, target.len(), &mut ops, f).unwrap();
    }
    assert_eq!(out, target);
    let copy = delta::encode(&base, &base).unwrap().unwrap();
    assert_eq!(copy.frames.len(), 1);
    assert_eq!(copy.operations, 1);
}

#[test]
fn exact_operation_literal_and_object_limits_are_enforced() {
    let mut frame = vec![3];
    frame.extend_from_slice(&256u32.to_le_bytes());
    frame.extend_from_slice(&1u16.to_le_bytes());
    frame.extend_from_slice(&256u16.to_le_bytes());
    for _ in 0..256 {
        frame.push(0);
        frame.extend_from_slice(&0u64.to_le_bytes());
        frame.extend_from_slice(&1u32.to_le_bytes());
    }
    let mut out = Vec::new();
    let mut ops = 0;
    for _ in 0..32 {
        delta::apply(b"x", &mut out, 8193, &mut ops, &frame).unwrap();
    }
    assert_eq!(ops, 8192);
    assert_eq!(out, vec![b'x'; 8192]);
    assert!(delta::apply(b"x", &mut out, 8193, &mut ops, &copy_frame(0, 1)).is_err());
    assert_eq!(ops, 8192);
    assert_eq!(out.len(), 8192);
    for n in [61440u32, 61441] {
        let mut body = vec![3];
        body.extend_from_slice(&n.to_le_bytes());
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend_from_slice(&1u16.to_le_bytes());
        body.push(1);
        body.extend_from_slice(&n.to_le_bytes());
        body.resize(body.len() + n as usize, 0);
        let mut out = Vec::new();
        let mut ops = 0;
        assert_eq!(
            delta::apply(&[], &mut out, n as usize, &mut ops, &body).is_ok(),
            n == 61440
        );
        assert_eq!(out.len(), if n == 61440 { n as usize } else { 0 });
    }
    let base = vec![7; spiralcore_transfer::MAX_OBJECT as usize];
    let mut out = Vec::new();
    let mut ops = 0;
    delta::apply(
        &base,
        &mut out,
        base.len(),
        &mut ops,
        &copy_frame(0, base.len() as u32),
    )
    .unwrap();
    assert_eq!(out, base);
    assert_eq!(ops, 1);
}
