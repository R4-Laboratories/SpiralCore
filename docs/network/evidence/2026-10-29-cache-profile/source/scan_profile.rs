//! Diagnostic snapshot only. Counts Rust API call sites, not operating-system syscalls.
//! Storage is bounded and thread-local. No per-entry clocks, allocations, or output.
use serde::Serialize;
use std::cell::Cell;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ScanProfile {
    pub calls: u64,
    pub successes: u64,
    pub errors: u64,
    pub cpu_ns: u64,
    pub wall_ns: u64,
    pub root_read_dir_calls: u64,
    pub root_entries: u64,
    pub root_file_type_calls: u64,
    pub namespace_read_dir_calls: u64,
    pub namespace_entries: u64,
    pub object_file_type_calls: u64,
    pub metadata_calls: u64,
    pub metadata_successes: u64,
    pub namespace_comparisons: u64,
    pub objects_counted: u64,
    pub returned_scoped_bytes: u64,
    pub returned_all_bytes: u64,
    pub returned_objects: u64,
    pub last_error: Option<u16>,
    pub last_error_stage: Option<&'static str>,
    pub overflow: bool,
}

thread_local! {
    static PROFILE: Cell<ScanProfile> = Cell::new(ScanProfile::default());
}

pub fn reset() {
    PROFILE.with(|p| p.set(ScanProfile::default()));
}
pub fn take() -> ScanProfile {
    PROFILE.with(|p| p.replace(ScanProfile::default()))
}

pub(crate) fn record(next: ScanProfile) {
    PROFILE.with(|cell| {
        let mut p = cell.get();
        p.overflow |= next.overflow;
        macro_rules! add {
            ($($name:ident),* $(,)?) => {$(
                match p.$name.checked_add(next.$name) {
                    Some(value) => p.$name = value,
                    None => { p.$name = u64::MAX; p.overflow = true; }
                }
            )*};
        }
        add!(
            calls,
            successes,
            errors,
            cpu_ns,
            wall_ns,
            root_read_dir_calls,
            root_entries,
            root_file_type_calls,
            namespace_read_dir_calls,
            namespace_entries,
            object_file_type_calls,
            metadata_calls,
            metadata_successes,
            namespace_comparisons,
            objects_counted,
            returned_scoped_bytes,
            returned_all_bytes,
            returned_objects
        );
        if next.last_error.is_some() {
            p.last_error = next.last_error;
            p.last_error_stage = next.last_error_stage;
        }
        cell.set(p);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aggregation_is_bounded_marks_overflow_and_preserves_error() {
        reset();
        record(ScanProfile {
            calls: u64::MAX,
            errors: 1,
            last_error: Some(9),
            last_error_stage: Some("root_read_dir"),
            ..Default::default()
        });
        record(ScanProfile {
            calls: 1,
            successes: 1,
            ..Default::default()
        });
        let p = take();
        assert_eq!(p.calls, u64::MAX);
        assert!(p.overflow);
        assert_eq!((p.successes, p.errors, p.last_error), (1, 1, Some(9)));
        assert_eq!(take(), ScanProfile::default());
    }
    #[test]
    fn observations_are_isolated_between_threads() {
        reset();
        record(ScanProfile {
            calls: 1,
            ..Default::default()
        });
        let other = std::thread::spawn(|| {
            reset();
            record(ScanProfile {
                calls: 7,
                ..Default::default()
            });
            take()
        })
        .join()
        .unwrap();
        assert_eq!(other.calls, 7);
        assert_eq!(take().calls, 1);
    }
}
