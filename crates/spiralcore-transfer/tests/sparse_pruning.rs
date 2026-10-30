//! Byte-identical legacy qualification and an untimed allocation regression.
//! The allocator observes only this test thread while encode runs. A nine-byte
//! allocation is the initial sparse frame header in this pinned implementation;
//! it is a materialization marker, not frame-capacity, RSS, or energy accounting.
use spiralcore_transfer::delta;
pub use spiralcore_transfer::{
    dag, hash, object_id, zstd_prefix, Error, Id, Result, MAX_CHUNK, MAX_FRAME, MAX_OBJECT,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

#[allow(dead_code)]
#[path = "support/delta_legacy_cc0199f.rs"]
mod legacy;
#[allow(dead_code)]
#[path = "support/protocol_legacy_cc0199f.rs"]
mod protocol;

thread_local! {
    static OBSERVE: Cell<bool> = const { Cell::new(false) };
    static HEADERS: Cell<usize> = const { Cell::new(0) };
}
struct ObservedAllocator;
// Test-only forwarding allocator. Observation uses allocation-free thread-local
// Cells and never inspects pointers or changes the System allocation contract.
unsafe impl GlobalAlloc for ObservedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if layout.size() == 9 && !ptr.is_null() {
            let _ = OBSERVE.try_with(|active| {
                if active.get() {
                    HEADERS.with(|count| count.set(count.get() + 1));
                }
            });
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: ObservedAllocator = ObservedAllocator;

fn header_allocations<T>(work: impl FnOnce() -> T) -> (T, usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            OBSERVE.with(|active| active.set(false));
        }
    }
    HEADERS.with(|count| count.set(0));
    OBSERVE.with(|active| assert!(!active.replace(true)));
    let reset = Reset;
    let result = work();
    drop(reset);
    (result, HEADERS.with(Cell::get))
}

fn random(n: usize, mut state: u64) -> Vec<u8> {
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

#[test]
fn provably_raw_replacement_does_not_materialize_sparse_frame_buffers() {
    let base = random(128 * 1024, 0x7342_0801);
    let target = base.iter().map(|byte| byte ^ 0xff).collect::<Vec<_>>();
    let (old, legacy_headers) = header_allocations(|| legacy::encode(&base, &target));
    assert!(old.unwrap().is_none());
    assert!(
        legacy_headers > 0,
        "legacy control must exercise actual frame allocation"
    );
    let (current, current_headers) = header_allocations(|| delta::encode(&base, &target));
    assert!(current.unwrap().is_none());
    assert_eq!(current_headers, 0, "a bound-rejected sparse candidate must not create a frame buffer; legacy allocated {legacy_headers} headers");
}

fn equivalent(label: &str, base: &[u8], target: &[u8]) -> Option<delta::Recipe> {
    let old = legacy::encode(base, target).unwrap();
    let new = delta::encode(base, target).unwrap();
    match (&old, &new) {
        (None, None) => {}
        (Some(old), Some(new)) => {
            assert_eq!(new.operations, old.operations, "{label}: operation count");
            assert_eq!(
                new.frames, old.frames,
                "{label}: serialized recipe identity"
            );
            for old_decoder in [false, true] {
                let mut output = Vec::new();
                let mut operations = 0;
                for frame in &new.frames {
                    if old_decoder {
                        legacy::apply(base, &mut output, target.len(), &mut operations, frame)
                            .unwrap();
                    } else {
                        delta::apply(base, &mut output, target.len(), &mut operations, frame)
                            .unwrap();
                    }
                }
                assert_eq!(output.len(), target.len(), "{label}: original length");
                assert_eq!(
                    output, target,
                    "{label}: exact bytes, old decoder={old_decoder}"
                );
                assert_eq!(operations, new.operations, "{label}: decoded operations");
            }
        }
        _ => panic!("{label}: legacy/current eligibility differs"),
    }
    new
}

#[test]
fn exact_legacy_recipe_matrix_covers_lengths_edits_and_binary_content() {
    for n in [
        0, 1, 31, 32, 33, 48, 49, 50, 255, 256, 257, 61_439, 61_440, 61_441, 131_073,
    ] {
        let base = random(n, 0x5350_0001 + n as u64);
        equivalent("empty base", &[], &base);
        equivalent("empty target", &base, &[]);
        equivalent("repeat", &base, &base);
        equivalent("zero replacement", &base, &vec![0; n]);
        let replacement = base.iter().map(|byte| byte ^ 0xff).collect::<Vec<_>>();
        equivalent("unrelated replacement", &base, &replacement);
        let mut prefixed = vec![0, 7, 0];
        prefixed.extend_from_slice(&base);
        equivalent("prefix insertion", &base, &prefixed);
        let mut appended = base.clone();
        appended.extend_from_slice(&[0, 3, 0, 0]);
        equivalent("append trailing zeros", &base, &appended);
        let middle = n / 2;
        let mut inserted = base[..middle].to_vec();
        inserted.extend_from_slice(&[0, 2, 0]);
        inserted.extend_from_slice(&base[middle..]);
        equivalent("middle insertion", &base, &inserted);
        if n > 0 {
            equivalent("prefix deletion", &base, &base[1..]);
            equivalent("truncate", &base, &base[..n - 1]);
            let mut deleted = base.clone();
            deleted.remove(middle);
            equivalent("middle deletion", &base, &deleted);
            let mut changed = base.clone();
            changed[middle] ^= 0x9d;
            equivalent("localized edit", &base, &changed);
            let mut scattered = base.clone();
            for byte in scattered.iter_mut().step_by(97) {
                *byte ^= 0xa7;
            }
            equivalent("scattered edit", &base, &scattered);
        }
    }
}

#[test]
fn exact_legacy_recipes_for_seeded_edits_periodic_and_compressed_content() {
    for seed in 1..=24 {
        let n = 1024 + seed * 521;
        let base = random(n, 0x7800 + seed as u64);
        let mut target = base.clone();
        for edit in 0..(seed * 3) {
            let position = (edit * 1307 + seed * 19) % n;
            target[position] ^= (seed as u8).wrapping_add(1);
        }
        target.extend_from_slice(&[0; 7]);
        equivalent("fixed-seed edits", &base, &target);
    }
    let base = (0..131_073).map(|i| (i % 43) as u8).collect::<Vec<_>>();
    let mut target = base.clone();
    target.splice(70_000..70_007, [0; 13]);
    equivalent("periodic structured", &base, &target);
    let incompressible = random(131_073, 0x1234_5678);
    let compressed = zstd::bulk::compress(&incompressible, 1).unwrap();
    let mut changed = compressed.clone();
    changed[101..133].fill(0);
    changed.extend_from_slice(&[0, 0]);
    equivalent("already compressed bytes", &compressed, &changed);
}

#[test]
fn strict_raw_boundary_and_equal_cost_tie_keep_legacy_decisions() {
    // One COPY costs 150, plus the 32-byte base identity. Raw costs n+133.
    for n in [48, 49, 50] {
        let base = vec![0x2a; n];
        let recipe = equivalent("Raw strict boundary", &base, &base);
        assert_eq!(recipe.is_some(), n == 50);
    }
    // Simple: COPY13 + LITERAL32 + COPY128. Aligned: LITERAL45 + COPY128.
    // Both cost 200 before the common base ID, but their bytes differ.
    let base = vec![0; 173];
    let mut target = base.clone();
    target[13..45].fill(1);
    let selected = equivalent("equal candidate costs", &base, &target).unwrap();
    assert_eq!(selected.operations, 3, "equal cost retains the simple plan");
    assert_eq!(selected.frames[0][9], 0, "simple plan begins with COPY");
    assert_eq!(selected.frames[0].len() + 128, 200);
}

#[test]
fn aligned_tie_is_not_materialized_after_a_simple_incumbent() {
    let base = vec![0; 173];
    let mut target = base.clone();
    target[13..45].fill(1);
    let (old, old_headers) = header_allocations(|| legacy::encode(&base, &target));
    let old = old.unwrap().unwrap();
    let (new, new_headers) = header_allocations(|| delta::encode(&base, &target));
    let new = new.unwrap().unwrap();
    assert_eq!(new.frames, old.frames);
    assert_eq!(new.operations, old.operations);
    assert_eq!(
        new_headers,
        new.frames.len() + 1,
        "only the winning simple pack may create headers"
    );
    assert!(
        old_headers > new_headers,
        "legacy control must materialize the tied aligned candidate"
    );
}

#[test]
fn constructed_simple_aligned_and_raw_winners_preserve_bytes() {
    let base = random(128 * 1024, 0x42b9);
    let mut shifted = vec![0x7f; 4096];
    shifted.extend_from_slice(&base);
    let simple = equivalent("compact simple shift", &base, &shifted).unwrap();
    assert_eq!(simple.operations, 2);
    let mut scattered = base.clone();
    for byte in scattered.iter_mut().step_by(1024) {
        *byte ^= 0xff;
    }
    let aligned = equivalent("aligned scattered winner", &base, &scattered).unwrap();
    assert!(aligned.operations > 3);
    let unrelated = base.iter().map(|byte| byte ^ 0xff).collect::<Vec<_>>();
    assert!(equivalent("both lose to Raw", &base, &unrelated).is_none());
}

#[test]
fn generated_aligned_plans_at_frame_and_total_operation_boundaries() {
    // Every 33-byte block contributes LITERAL1 + COPY32. Truncating the final
    // COPY produces odd counts, so both sides of each operation boundary occur.
    for operations in [255, 256, 257, 8191, 8192, 8193] {
        let pairs = operations / 2;
        let n = pairs * 33 + operations % 2;
        let base = vec![0; n];
        let mut target = base.clone();
        for byte in target.iter_mut().step_by(33) {
            *byte = 1;
        }
        let recipe = equivalent("aligned operation boundary", &base, &target);
        if operations <= delta::MAX_OPS {
            assert_eq!(recipe.unwrap().operations, operations);
        } else {
            assert!(
                recipe.is_none(),
                "oversized aligned plan falls back exactly as before"
            );
        }
    }
}

#[test]
fn unchanged_decoder_rejects_same_malformed_bytes_atomically() {
    let base = vec![0; 64];
    let frame = delta::encode(&base, &base)
        .unwrap()
        .unwrap()
        .frames
        .remove(0);
    let mut malformed = (0..frame.len())
        .map(|n| frame[..n].to_vec())
        .collect::<Vec<_>>();
    let mut tag = frame.clone();
    tag[9] = 17;
    malformed.push(tag);
    let mut extra = frame.clone();
    extra.push(0);
    malformed.push(extra);
    let mut offset = frame.clone();
    offset[10..18].copy_from_slice(&u64::MAX.to_le_bytes());
    malformed.push(offset);
    for frame in malformed {
        let mut old = vec![7];
        let mut new = old.clone();
        let mut old_ops = 1;
        let mut new_ops = 1;
        let old_result = legacy::apply(&base, &mut old, 65, &mut old_ops, &frame);
        let new_result = delta::apply(&base, &mut new, 65, &mut new_ops, &frame);
        assert!(new_result.is_err());
        assert_eq!(new_result, old_result);
        assert_eq!(new, [7]);
        assert_eq!(new, old);
        assert_eq!(new_ops, 1);
        assert_eq!(new_ops, old_ops);
    }
}

#[test]
fn sender_limit_precedes_exchange_and_core_limit_is_unchanged() {
    use spiralcore_transfer::client::transfer_sparse_batched;
    let target = vec![0; 16 * 1024 * 1024 + 1];
    let mut exchanges = 0;
    let result = transfer_sparse_batched(&[], &target, [0x71; 32], false, |_| {
        exchanges += 1;
        panic!("oversized sender input must not exchange");
    });
    assert_eq!(result.unwrap_err(), Error::Limit);
    assert_eq!(exchanges, 0);
    // Core still accepts above the sender ceiling; this tiny-output repeat is
    // qualification of the existing API bound, not larger-scale networking.
    assert!(equivalent("core above sender ceiling", &target, &target).is_some());
    let too_large = vec![0; MAX_OBJECT as usize + 1];
    assert_eq!(delta::encode(&too_large, &[]).unwrap_err(), Error::Limit);
    assert_eq!(legacy::encode(&too_large, &[]).unwrap_err(), Error::Limit);
    assert_eq!(delta::encode(&[], &too_large).unwrap_err(), Error::Limit);
    assert_eq!(legacy::encode(&[], &too_large).unwrap_err(), Error::Limit);
}
