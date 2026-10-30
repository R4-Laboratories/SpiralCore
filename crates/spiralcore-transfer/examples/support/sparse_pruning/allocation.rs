//! Diagnostic-only requested frame-buffer capacity accounting. The normal
//! executable does not compile this module. Tracked growth deliberately uses
//! alloc-copy-free; these are not production System realloc/RSS measurements.
use serde::Serialize;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::RefCell,
    ptr,
    sync::atomic::{AtomicBool, Ordering},
};

const SLOTS: usize = 2048;
#[derive(Clone, Copy)]
struct Slot {
    pointer: usize,
    bytes: usize,
}
impl Slot {
    const EMPTY: Self = Self {
        pointer: 0,
        bytes: 0,
    };
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Stats {
    pub live_bytes: usize,
    pub peak_bytes: usize,
    pub cumulative_allocated_bytes: usize,
    pub header_allocations: usize,
    pub reallocations: usize,
    pub frees: usize,
    pub active_buffers: usize,
    pub registry_failed: bool,
}
struct Tracker {
    discovery: bool,
    slots: [Slot; SLOTS],
    limit: usize,
    stats: Stats,
}
impl Tracker {
    const fn new() -> Self {
        Self {
            discovery: false,
            slots: [Slot::EMPTY; SLOTS],
            limit: SLOTS,
            stats: Stats {
                live_bytes: 0,
                peak_bytes: 0,
                cumulative_allocated_bytes: 0,
                header_allocations: 0,
                reallocations: 0,
                frees: 0,
                active_buffers: 0,
                registry_failed: false,
            },
        }
    }
    fn index(&self, pointer: usize) -> Option<usize> {
        self.slots[..self.limit]
            .iter()
            .position(|s| s.pointer == pointer)
    }
    fn allocated(&mut self, pointer: usize, bytes: usize) {
        let Some(index) = self.slots[..self.limit].iter().position(|s| s.pointer == 0) else {
            self.stats.registry_failed = true;
            return;
        };
        if pointer == 0 || self.index(pointer).is_some() {
            self.stats.registry_failed = true;
            return;
        }
        self.slots[index] = Slot { pointer, bytes };
        self.stats.live_bytes += bytes;
        self.stats.cumulative_allocated_bytes += bytes;
        self.stats.peak_bytes = self.stats.peak_bytes.max(self.stats.live_bytes);
        self.stats.header_allocations += 1;
        self.stats.active_buffers += 1;
    }
    fn grew(&mut self, old: usize, new: usize, bytes: usize) {
        let Some(index) = self.index(old) else {
            self.stats.registry_failed = true;
            return;
        };
        let previous = self.slots[index].bytes;
        // System has actually allocated the new region and the old allocation
        // is still live until the explicit free below the tracking callback.
        self.stats.peak_bytes = self.stats.peak_bytes.max(self.stats.live_bytes + bytes);
        self.stats.cumulative_allocated_bytes += bytes;
        self.stats.live_bytes = self.stats.live_bytes + bytes - previous;
        self.stats.reallocations += 1;
        self.stats.frees += 1;
        self.slots[index] = Slot {
            pointer: new,
            bytes,
        };
    }
    fn freed(&mut self, pointer: usize) {
        if let Some(index) = self.index(pointer) {
            self.stats.live_bytes -= self.slots[index].bytes;
            self.stats.active_buffers -= 1;
            self.stats.frees += 1;
            self.slots[index] = Slot::EMPTY;
        }
    }
}
thread_local! {static TRACKER: RefCell<Tracker> = const {RefCell::new(Tracker::new())};}
static ENABLED: AtomicBool = AtomicBool::new(false);
static CALLBACK_FAILED: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static FAIL_NEXT_TRACKED_GROWTH: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
fn fail_next_tracked_growth() {
    FAIL_NEXT_TRACKED_GROWTH.store(true, Ordering::Relaxed);
}
unsafe fn allocate_growth(layout: Layout) -> *mut u8 {
    #[cfg(test)]
    if FAIL_NEXT_TRACKED_GROWTH.swap(false, Ordering::Relaxed) {
        return ptr::null_mut();
    }
    // SAFETY: caller supplies a valid nonzero allocation layout.
    unsafe { System.alloc(layout) }
}
fn track<R>(f: impl FnOnce(&mut Tracker) -> R) -> Option<R> {
    if !ENABLED.load(Ordering::Relaxed) {
        return None;
    }
    let result = TRACKER
        .try_with(|t| t.try_borrow_mut().ok().map(|mut t| f(&mut t)))
        .ok()
        .flatten();
    if result.is_none() {
        CALLBACK_FAILED.store(true, Ordering::Relaxed);
    }
    result
}
struct FrameAllocator;
#[global_allocator]
static ALLOCATOR: FrameAllocator = FrameAllocator;

// SAFETY: all allocations delegate to System with valid caller layouts. Only
// tracked realloc uses equivalent allocate/copy(min(old,new))/free semantics;
// allocation failure preserves the old pointer. Tracking never allocates or
// dereferences user memory and is thread-local with a fixed registry.
unsafe impl GlobalAlloc for FrameAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded GlobalAlloc caller contract.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && layout.size() == 9 && layout.align() == 1 {
            track(|t| {
                if t.discovery {
                    t.allocated(pointer as usize, layout.size())
                }
            });
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        track(|t| t.freed(pointer as usize));
        // SAFETY: pointer/layout came from this allocator, which delegates to System.
        unsafe { System.dealloc(pointer, layout) };
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let tracked = track(|t| t.index(pointer as usize).is_some()).unwrap_or(false);
        if !tracked {
            // SAFETY: forwarded GlobalAlloc reallocation contract.
            return unsafe { System.realloc(pointer, layout, new_size) };
        }
        let Ok(new_layout) = Layout::from_size_align(new_size, layout.align()) else {
            return ptr::null_mut();
        };
        // SAFETY: layout validated above; allocation is independent of the old region.
        let new = unsafe { allocate_growth(new_layout) };
        if new.is_null() {
            return new;
        }
        // SAFETY: both allocations are live, distinct and valid for min(old,new) bytes.
        unsafe { ptr::copy_nonoverlapping(pointer, new, layout.size().min(new_size)) };
        track(|t| t.grew(pointer as usize, new as usize, new_size));
        // SAFETY: old allocation is no longer used after the completed copy.
        unsafe { System.dealloc(pointer, layout) };
        new
    }
}

pub fn start() -> Result<(), &'static str> {
    ENABLED.store(true, Ordering::Relaxed);
    CALLBACK_FAILED.store(false, Ordering::Relaxed);
    track(|t| {
        if t.stats.active_buffers != 0 {
            return Err("previous diagnostic buffers are still live");
        }
        *t = Tracker::new();
        t.discovery = true;
        Ok(())
    })
    .ok_or("allocation tracker unavailable")?
}
pub fn snapshot() -> Result<Stats, &'static str> {
    let stats = track(|t| t.stats).ok_or("allocation tracker unavailable")?;
    if stats.registry_failed || CALLBACK_FAILED.load(Ordering::Relaxed) {
        return Err("allocation registry failed or exhausted");
    }
    Ok(stats)
}
pub fn stop_discovery() -> Result<Stats, &'static str> {
    track(|t| t.discovery = false).ok_or("allocation tracker unavailable")?;
    snapshot()
}

#[cfg(test)]
mod tests {
    #[test]
    fn failed_tracked_growth_preserves_pointer_data_and_registry() {
        use std::alloc::{GlobalAlloc, Layout};
        super::start().unwrap();
        let layout = Layout::from_size_align(9, 1).unwrap();
        // SAFETY: successful allocations below are accessed only within their
        // layout and deallocated once using that same allocator and layout.
        unsafe {
            let original = super::ALLOCATOR.alloc(layout);
            assert!(!original.is_null());
            original.write_bytes(0xa5, 9);
            let before = super::stop_discovery().unwrap();
            super::fail_next_tracked_growth();
            let replacement = super::ALLOCATOR.realloc(original, layout, 1000);
            assert!(replacement.is_null());
            assert_eq!(std::slice::from_raw_parts(original, 9), &[0xa5; 9]);
            let after = super::snapshot().unwrap();
            assert_eq!(after.live_bytes, before.live_bytes);
            assert_eq!(after.peak_bytes, before.peak_bytes);
            assert_eq!(
                after.cumulative_allocated_bytes,
                before.cumulative_allocated_bytes
            );
            assert_eq!(after.active_buffers, 1);
            assert_eq!(after.reallocations, 0);
            assert_eq!(after.frees, 0);
            super::ALLOCATOR.dealloc(original, layout);
        }
        let released = super::snapshot().unwrap();
        assert_eq!(released.live_bytes, 0);
        assert_eq!(released.active_buffers, 0);
        assert_eq!(released.frees, 1);
    }

    #[test]
    fn missing_growth_entry_fails_closed_without_changing_live_capacity() {
        let mut tracker = super::Tracker::new();
        tracker.allocated(16, 9);
        tracker.grew(32, 64, 1000);
        assert!(tracker.stats.registry_failed);
        assert_eq!(tracker.stats.live_bytes, 9);
        assert_eq!(tracker.stats.reallocations, 0);
        assert_eq!(tracker.stats.frees, 0);
        tracker.freed(16);
        assert_eq!(tracker.stats.live_bytes, 0);
    }

    #[test]
    fn reentrant_tracker_callback_fails_closed() {
        super::start().unwrap();
        super::TRACKER.with(|cell| {
            let _held = cell.borrow_mut();
            assert!(super::track(|_| ()).is_none());
        });
        assert!(super::snapshot().is_err());
        assert!(super::stop_discovery().is_err());
        // Each diagnostic process is single-threaded. Explicitly begin a fresh
        // empty session after checking that the failed session stays failed.
        super::start().unwrap();
        assert_eq!(super::stop_discovery().unwrap().live_bytes, 0);
    }

    #[test]
    fn registry_exhaustion_and_duplicate_pointer_fail_closed() {
        let mut tracker = super::Tracker::new();
        tracker.limit = 1;
        tracker.allocated(16, 9);
        tracker.allocated(32, 9);
        assert!(tracker.stats.registry_failed);
        tracker.freed(16);
        assert_eq!(tracker.stats.live_bytes, 0);
        let mut tracker = super::Tracker::new();
        tracker.allocated(16, 9);
        tracker.allocated(16, 9);
        assert!(tracker.stats.registry_failed);
    }

    #[test]
    fn discovery_stops_but_returned_buffers_remain_tracked_until_drop() {
        super::start().unwrap();
        let original = vec![1u8; 9];
        let at_return = super::stop_discovery().unwrap();
        let unrelated = vec![2u8; 9];
        let other_size = vec![0u8; 1000];
        assert_eq!(super::snapshot().unwrap().live_bytes, at_return.live_bytes);
        assert!(super::start().is_err());
        drop(original);
        assert_eq!(super::snapshot().unwrap().live_bytes, 0);
        drop(unrelated);
        drop(other_size);
        assert_eq!(super::snapshot().unwrap().frees, 1);
    }

    #[test]
    fn actual_capacity_growth_and_drop_are_accounted() {
        super::start().unwrap();
        let mut body = vec![3u8, 0, 0, 0, 0, 1, 0, 0, 0];
        let initial = body.capacity();
        body.extend_from_slice(&[7u8; 100]);
        let grown = body.capacity();
        let returned = super::stop_discovery().unwrap();
        assert_eq!(returned.live_bytes, grown);
        assert_eq!(returned.peak_bytes, initial + grown);
        assert_eq!(returned.cumulative_allocated_bytes, initial + grown);
        assert_eq!(returned.header_allocations, 1);
        assert_eq!(returned.reallocations, 1);
        drop(body);
        let released = super::snapshot().unwrap();
        assert_eq!(released.live_bytes, 0);
        assert_eq!(released.active_buffers, 0);
    }
}
