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

fn request(bytes: usize) {
    let (current, overflow) = LIVE
        .fetch_add(bytes, Ordering::SeqCst)
        .overflowing_add(bytes);
    if overflow {
        INVALID.store(true, Ordering::SeqCst);
    }
    PEAK.fetch_max(current, Ordering::SeqCst);
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
