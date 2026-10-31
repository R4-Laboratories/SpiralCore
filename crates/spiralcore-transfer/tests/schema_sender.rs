//! Allocation behavior is measured only around the public prepared schema
//! encoder, on this test thread. No wall-clock threshold or allocator timing.
use spiralcore_transfer::schema::{self, PreparedSchema};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct CountAllocations;

thread_local! {
    static ALLOCATIONS: Cell<Option<usize>> = const { Cell::new(None) };
}

fn allocated() {
    let _ = ALLOCATIONS.try_with(|count| {
        if let Some(value) = count.get() {
            count.set(Some(value + 1));
        }
    });
}

// SAFETY: Every request is forwarded unchanged to System; the thread-local
// counter uses no heap allocation and never changes pointers or layouts.
unsafe impl GlobalAlloc for CountAllocations {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocated();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        allocated();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        allocated();
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountAllocations = CountAllocations;

fn counted<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            ALLOCATIONS.set(None);
        }
    }
    ALLOCATIONS.set(Some(0));
    let guard = Reset;
    let result = operation();
    let count = ALLOCATIONS.get().unwrap();
    drop(guard);
    (result, count)
}

#[test]
fn matching_prepared_records_allocate_only_the_output_buffer() {
    let row = b"{\"long_identifier\":17,\"status_message\":\"alpha\",\"enabled_flag\":true}\n";
    let artifact = schema::fit(&[row]).unwrap();
    let prepared = PreparedSchema::new(&artifact).unwrap();
    for records in [1, 128, 4096] {
        let target = row.repeat(records);
        let (encoded, allocations) = counted(|| prepared.encode(&target).unwrap());
        assert_eq!(
            allocations, 1,
            "matching {records} records must allocate only the SCVAL output buffer"
        );
        assert_eq!(prepared.decode(&encoded, target.len()).unwrap(), target);
    }
}

#[test]
fn allocation_counter_is_local_and_disabled_outside_the_scope() {
    let (_bytes, counted_allocations) = counted(|| vec![7u8; 128]);
    assert_eq!(counted_allocations, 1);
    assert_eq!(ALLOCATIONS.get(), None);
    let (_, allocations) = counted(|| 17u32);
    assert_eq!(allocations, 0);
}

#[test]
fn fitting_repeated_first_layout_has_constant_temporary_allocations() {
    let training: Vec<Vec<u8>> = (0..schema::MAX_TRAINING_RECORDS)
        .map(|i| {
            format!(
                "{{\"long_identifier\":{i},\"status_message\":\"value-{i}\",\"enabled_flag\":true}}\n"
            )
            .into_bytes()
        })
        .collect();
    let samples: Vec<&[u8]> = training.iter().map(Vec::as_slice).collect();
    let (expected, first_allocations) = counted(|| schema::fit(&samples[..1]).unwrap());
    for records in [128, 512, schema::MAX_TRAINING_RECORDS] {
        let (artifact, allocations) = counted(|| schema::fit(&samples[..records]).unwrap());
        assert_eq!(artifact, expected);
        assert_eq!(
            allocations, first_allocations,
            "{records} exact duplicate layouts must not allocate per training record"
        );
    }
}
