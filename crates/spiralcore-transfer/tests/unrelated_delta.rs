//! Rejected all-literal candidates must not materialize payload-sized buffers.
//! Count allocator requests on this thread only; this is not timing or RSS.
use spiralcore_transfer::{delta, MAX_CHUNK};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct CountLargeAllocations;

thread_local! {
    static LARGE_ALLOCATIONS: Cell<Option<usize>> = const { Cell::new(None) };
}

fn allocated(size: usize) {
    if size >= MAX_CHUNK {
        let _ = LARGE_ALLOCATIONS.try_with(|count| {
            if let Some(value) = count.get() {
                count.set(Some(value + 1));
            }
        });
    }
}

// SAFETY: Requests are forwarded unchanged to System. The thread-local counter
// does not allocate and never changes allocator pointers, sizes, or layouts.
unsafe impl GlobalAlloc for CountLargeAllocations {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocated(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        allocated(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        allocated(size);
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountLargeAllocations = CountLargeAllocations;

fn counted<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            LARGE_ALLOCATIONS.set(None);
        }
    }
    LARGE_ALLOCATIONS.set(Some(0));
    let guard = Reset;
    let result = operation();
    let count = LARGE_ALLOCATIONS.get().unwrap();
    drop(guard);
    (result, count)
}

#[test]
fn all_literal_rejection_does_not_allocate_payload_sized_buffers() {
    // No common byte, prefix, or suffix can yield a COPY. Inputs are allocated
    // before counting, so only encode's temporary work is observed.
    let base = vec![0x31; 131_073];
    let target = vec![0x93; 131_073];
    let (result, allocations) =
        counted(|| delta::encode(std::hint::black_box(&base), std::hint::black_box(&target)));
    assert!(result.unwrap().is_none());
    assert_eq!(
        allocations, 0,
        "a provably rejected all-literal recipe must not allocate a frame payload"
    );
}

#[test]
fn allocation_counter_observes_large_requests_only_within_its_scope() {
    let (bytes, allocations) = counted(|| std::hint::black_box(vec![7u8; MAX_CHUNK]));
    assert_eq!(bytes.len(), MAX_CHUNK);
    assert_eq!(allocations, 1);
    assert_eq!(LARGE_ALLOCATIONS.get(), None);
    let (small, allocations) = counted(|| std::hint::black_box(vec![7u8; MAX_CHUNK - 1]));
    assert_eq!(small.len(), MAX_CHUNK - 1);
    assert_eq!(allocations, 0);
}
