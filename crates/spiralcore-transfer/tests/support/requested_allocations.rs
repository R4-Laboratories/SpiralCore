//! Requested Rust heap bytes, including a conservative old+new realloc peak.
//! System allocator metadata, native stacks and direct libc allocations are not counted.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

pub struct RequestedAllocations;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static REQUESTED: AtomicUsize = AtomicUsize::new(0);
static INVALID: AtomicBool = AtomicBool::new(false);
static WINDOW: AtomicBool = AtomicBool::new(false);
static WINDOW_LARGEST: AtomicUsize = AtomicUsize::new(0);
static WINDOW_REQUESTED: AtomicUsize = AtomicUsize::new(0);
static WINDOW_PEAK: AtomicUsize = AtomicUsize::new(0);
static WINDOW_COUNT: AtomicUsize = AtomicUsize::new(0);

fn request(bytes: usize) {
    let in_window = WINDOW.load(Ordering::SeqCst);
    if in_window {
        if WINDOW_COUNT
            .fetch_add(1, Ordering::SeqCst)
            .checked_add(1)
            .is_none()
        {
            INVALID.store(true, Ordering::SeqCst);
        }
        WINDOW_LARGEST.fetch_max(bytes, Ordering::SeqCst);
        if WINDOW_REQUESTED
            .fetch_add(bytes, Ordering::SeqCst)
            .checked_add(bytes)
            .is_none()
        {
            INVALID.store(true, Ordering::SeqCst);
        }
    }
    let (current, overflow) = LIVE
        .fetch_add(bytes, Ordering::SeqCst)
        .overflowing_add(bytes);
    if overflow {
        INVALID.store(true, Ordering::SeqCst);
    }
    PEAK.fetch_max(current, Ordering::SeqCst);
    if in_window {
        WINDOW_PEAK.fetch_max(current, Ordering::SeqCst);
    }
    if REQUESTED
        .fetch_add(bytes, Ordering::SeqCst)
        .checked_add(bytes)
        .is_none()
    {
        INVALID.store(true, Ordering::SeqCst);
    }
}
fn release(bytes: usize) {
    if LIVE.fetch_sub(bytes, Ordering::SeqCst) < bytes {
        INVALID.store(true, Ordering::SeqCst);
    }
}

// SAFETY: Every operation delegates the unchanged pointer/layout to System.
// Counters never allocate and do not affect allocation ownership or failure.
unsafe impl GlobalAlloc for RequestedAllocations {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        request(layout.size());
        // SAFETY: The caller supplies the layout required by GlobalAlloc.
        let pointer = unsafe { System.alloc(layout) };
        if pointer.is_null() {
            release(layout.size());
        }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        request(layout.size());
        // SAFETY: The caller supplies the layout required by GlobalAlloc.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if pointer.is_null() {
            release(layout.size());
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: GlobalAlloc's caller retains this allocation and its original layout.
        unsafe { System.dealloc(pointer, layout) };
        release(layout.size());
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // Charge old and new simultaneously, even if System can resize in place.
        request(size);
        // SAFETY: Forward the caller's live allocation, original layout and new size.
        let replacement = unsafe { System.realloc(pointer, layout, size) };
        release(if replacement.is_null() {
            size
        } else {
            layout.size()
        });
        replacement
    }
}

pub fn snapshot() -> (usize, usize, usize) {
    (
        LIVE.load(Ordering::SeqCst),
        PEAK.load(Ordering::SeqCst),
        REQUESTED.load(Ordering::SeqCst),
    )
}

pub fn valid() -> bool {
    !INVALID.load(Ordering::SeqCst)
}

/// Cumulative requests bound newly requested payload without subtracting frees
/// of allocations that predate the window. This is not allocator overhead/RSS.
/// One coordinator owns the window and joins participating workers before return;
/// every process request while open is included, including fixture overhead.
pub fn measure_requests<R>(run: impl FnOnce() -> R) -> (R, usize, usize) {
    let (result, window) = measure_heap(run);
    (result, window.largest, window.cumulative)
}

#[derive(Clone, Copy, Debug)]
pub struct HeapWindow {
    pub count: usize,
    pub largest: usize,
    pub cumulative: usize,
    pub live_before: usize,
    pub peak_live: usize,
    pub live_after: usize,
}

pub fn measure_heap<R>(run: impl FnOnce() -> R) -> (R, HeapWindow) {
    assert!(!WINDOW.load(Ordering::SeqCst), "nested allocation window");
    let before = LIVE.load(Ordering::SeqCst);
    WINDOW_LARGEST.store(0, Ordering::SeqCst);
    WINDOW_REQUESTED.store(0, Ordering::SeqCst);
    WINDOW_PEAK.store(before, Ordering::SeqCst);
    WINDOW_COUNT.store(0, Ordering::SeqCst);
    assert!(!WINDOW.swap(true, Ordering::SeqCst));
    struct Close;
    impl Drop for Close {
        fn drop(&mut self) {
            WINDOW.store(false, Ordering::SeqCst);
        }
    }
    let close = Close;
    let result = run();
    drop(close);
    (
        result,
        HeapWindow {
            count: WINDOW_COUNT.load(Ordering::SeqCst),
            largest: WINDOW_LARGEST.load(Ordering::SeqCst),
            cumulative: WINDOW_REQUESTED.load(Ordering::SeqCst),
            live_before: before,
            peak_live: WINDOW_PEAK.load(Ordering::SeqCst),
            live_after: LIVE.load(Ordering::SeqCst),
        },
    )
}
