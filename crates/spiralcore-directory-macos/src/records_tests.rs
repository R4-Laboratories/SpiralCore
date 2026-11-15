use super::*;
use std::{
    ffi::OsString,
    os::unix::ffi::{OsStrExt, OsStringExt},
};

fn entry(name: &[u8], padded: bool) -> Vec<u8> {
    let needed = (36 + name.len() + 1 + 3) & !3;
    let length = if padded { (needed + 7) & !7 } else { needed };
    let mut bytes = vec![0; length];
    bytes[..4].copy_from_slice(&(length as u32).to_ne_bytes());
    bytes[4..8].copy_from_slice(&REQUESTED.to_ne_bytes());
    bytes[28..32].copy_from_slice(&8i32.to_ne_bytes());
    bytes[32..36].copy_from_slice(&((name.len() + 1) as u32).to_ne_bytes());
    bytes[36..36 + name.len()].copy_from_slice(name);
    bytes
}
fn names(bytes: &[u8], count: usize) -> io::Result<Vec<OsString>> {
    let mut names = Vec::new();
    let flow = visit_batch(bytes, count, &mut |name| {
        names.push(name.to_owned());
        ControlFlow::Continue(())
    })?;
    assert_eq!(flow, ControlFlow::Continue(()));
    Ok(names)
}

#[test]
fn multiple_records_preserve_arbitrary_component_bytes() {
    let mut bytes = entry(b"original", true);
    bytes.extend(entry(b"non-utf8-\xff", true));
    assert_eq!(
        names(&bytes, 2).unwrap(),
        [
            OsString::from_vec(b"original".to_vec()),
            OsString::from_vec(b"non-utf8-\xff".to_vec()),
        ]
    );
}

#[test]
fn final_record_can_omit_padding_without_consuming_the_buffer_tail() {
    let mut bytes = entry(b"first", true);
    let last = entry(b"abcd", false);
    assert_ne!(last.len() % 8, 0);
    bytes.extend(last);
    bytes.extend([0; 2]);
    assert_eq!(names(&bytes, 2).unwrap().len(), 2);
}

#[test]
fn count_and_record_boundaries_refuse_partial_or_oversized_results() {
    let good = entry(b"x", true);
    assert!(names(&good, 2).is_err());
    for length in [0, 1, 35, u32::MAX] {
        let mut bad = good.clone();
        bad[..4].copy_from_slice(&length.to_ne_bytes());
        assert!(names(&bad, 1).is_err(), "record length {length}");
    }
    for length in 0..good.len() {
        assert!(names(&good[..length], 1).is_err(), "truncated {length}");
    }
    let mut bad = entry(b"abcd", false);
    bad.extend(entry(b"next", true));
    assert!(names(&bad, 2).is_err());
    assert!(names(&good, usize::MAX).is_err());
}

#[test]
fn attribute_sets_and_entry_errors_cannot_be_silently_skipped() {
    for mask in [
        0,
        REQUESTED & !NAME,
        REQUESTED & !RETURNED,
        REQUESTED & !ENTRY_ERROR,
        REQUESTED | 2,
    ] {
        let mut bytes = entry(b"x", true);
        bytes[4..8].copy_from_slice(&mask.to_ne_bytes());
        assert!(names(&bytes, 1).is_err());
    }
    for offset in [8, 12, 16, 20, 24] {
        let mut bytes = entry(b"x", true);
        bytes[offset..offset + 4].copy_from_slice(&1u32.to_ne_bytes());
        assert!(
            names(&bytes, 1).is_err(),
            "invalid attribute/error at {offset}"
        );
    }
}

#[test]
fn references_cannot_escape_their_record_or_point_into_fixed_metadata() {
    for offset in [i32::MIN, -1, 0, 7, i32::MAX] {
        let mut bytes = entry(b"x", true);
        bytes[28..32].copy_from_slice(&offset.to_ne_bytes());
        assert!(names(&bytes, 1).is_err());
    }
    for length in [0, 1, 1025, u32::MAX] {
        let mut bytes = entry(b"x", true);
        bytes[32..36].copy_from_slice(&length.to_ne_bytes());
        assert!(names(&bytes, 1).is_err());
    }
    let mut cross = entry(b"x", true);
    cross[28..32].copy_from_slice(&12i32.to_ne_bytes());
    cross.extend(entry(b"y", true));
    assert!(names(&cross, 2).is_err());
}

#[test]
fn component_terminators_and_separators_are_checked_without_lossy_conversion() {
    for name in [b"".as_slice(), b"a/b", b"a\0b"] {
        assert!(names(&entry(name, true), 1).is_err());
    }
    let mut no_nul = entry(b"x", true);
    no_nul[37] = b'x';
    assert!(names(&no_nul, 1).is_err());
    let max = vec![b'x'; 1023];
    assert_eq!(names(&entry(&max, true), 1).unwrap()[0].as_bytes(), max);
    assert!(names(&entry(&vec![b'x'; 1024], true), 1).is_err());
}

#[test]
fn early_stop_is_distinct_from_complete_enumeration_and_preserves_first_outcome() {
    let mut bytes = entry(b"first", true);
    bytes.extend([0; 40]); // An unvisited malformed record cannot replace the callback's stop.
    let mut visited = 0;
    assert_eq!(
        visit_batch(&bytes, 2, &mut |_| {
            visited += 1;
            ControlFlow::Break(())
        })
        .unwrap(),
        ControlFlow::Break(())
    );
    assert_eq!(visited, 1);
    assert!(names(&bytes, 2).is_err());
}

#[test]
fn bounded_mutations_never_panic_or_yield_unchecked_names() {
    let source = entry(b"component", true);
    for position in 0..source.len() {
        for value in [0, 1, 0x7f, 0x80, 0xff] {
            let mut bytes = source.clone();
            bytes[position] = value;
            let _ = visit_batch(&bytes, 1, &mut |name| {
                let value = name.as_bytes();
                assert!(!value.is_empty() && value.len() <= 1023);
                assert!(!value.contains(&0) && !value.contains(&b'/'));
                ControlFlow::Continue(())
            });
        }
    }
}
