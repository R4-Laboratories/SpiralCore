//! Diagnostic snapshot only. Counts Rust API call sites, not operating-system syscalls.
//! Storage is bounded and thread-local. No per-entry allocation or output.
//! Fine and empty modes deliberately add per-expression clocks; counters mode does not.
use serde::Serialize;
use std::cell::Cell;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Counters,
    Fine,
    Empty,
}
#[derive(Debug, Clone, Copy)]
#[repr(usize)]
pub enum Category {
    Enumeration,
    EntryType,
    CanonicalName,
    ExtensionPath,
    Metadata,
    NamespaceCompare,
}
pub const CATEGORY_NAMES: [&str; 6] = [
    "enumeration",
    "entry_type",
    "canonical_name",
    "extension_path",
    "metadata",
    "namespace_compare",
];
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Span {
    pub calls: u64,
    pub cpu_ns: u64,
    pub wall_ns: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ScanProfile {
    pub mode: Mode,
    pub spans: [Span; 6],
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
    static MODE: Cell<Mode> = const { Cell::new(Mode::Counters) };
}
pub fn configure(mode: Mode) {
    MODE.with(|m| m.set(mode));
}
pub(crate) fn mode() -> Mode {
    MODE.with(Cell::get)
}

pub(crate) fn span<T>(profile: &mut ScanProfile, category: Category, run: impl FnOnce() -> T) -> T {
    let slot = &mut profile.spans[category as usize];
    match slot.calls.checked_add(1) {
        Some(n) => slot.calls = n,
        None => {
            slot.calls = u64::MAX;
            profile.overflow = true;
        }
    }
    match profile.mode {
        Mode::Counters => run(),
        Mode::Fine => {
            let cpu = cpu_time::ProcessTime::now();
            let wall = std::time::Instant::now();
            let result = run();
            elapsed(slot, &mut profile.overflow, cpu, wall);
            result
        }
        Mode::Empty => {
            let cpu = cpu_time::ProcessTime::now();
            let wall = std::time::Instant::now();
            std::hint::black_box(());
            elapsed(slot, &mut profile.overflow, cpu, wall);
            run()
        }
    }
}
fn elapsed(
    slot: &mut Span,
    overflow: &mut bool,
    cpu: cpu_time::ProcessTime,
    wall: std::time::Instant,
) {
    let wall_ns = wall.elapsed().as_nanos();
    let cpu_ns = cpu.elapsed().as_nanos();
    for (value, next) in [(&mut slot.wall_ns, wall_ns), (&mut slot.cpu_ns, cpu_ns)] {
        if next > u64::MAX as u128 {
            *overflow = true;
        }
        match value.checked_add(next.min(u64::MAX as u128) as u64) {
            Some(n) => *value = n,
            None => {
                *value = u64::MAX;
                *overflow = true;
            }
        }
    }
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
        if p.calls == 0 {
            p.mode = next.mode;
        } else if p.mode != next.mode {
            p.overflow = true;
        }
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
        for (sum, value) in p.spans.iter_mut().zip(next.spans) {
            for (a, b) in [
                (&mut sum.calls, value.calls),
                (&mut sum.cpu_ns, value.cpu_ns),
                (&mut sum.wall_ns, value.wall_ns),
            ] {
                match a.checked_add(b) {
                    Some(n) => *a = n,
                    None => {
                        *a = u64::MAX;
                        p.overflow = true;
                    }
                }
            }
        }
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
    fn span_executes_once_and_preserves_error_for_every_mode() {
        for mode in [Mode::Counters, Mode::Fine, Mode::Empty] {
            let mut p = ScanProfile {
                mode,
                ..Default::default()
            };
            let mut executions = 0;
            let result = span(&mut p, Category::Metadata, || {
                executions += 1;
                Err::<(), _>(crate::Error::Storage)
            });
            assert_eq!(result, Err(crate::Error::Storage));
            assert_eq!(executions, 1);
            assert_eq!(p.spans.map(|s| s.calls), [0, 0, 0, 0, 1, 0]);
            if mode == Mode::Counters {
                assert_eq!((p.spans[4].cpu_ns, p.spans[4].wall_ns), (0, 0));
            }
        }
    }
    #[test]
    fn aggregate_preserves_categories_and_marks_mixed_mode() {
        reset();
        let mut p = ScanProfile {
            calls: 1,
            mode: Mode::Fine,
            ..Default::default()
        };
        p.spans[3] = Span {
            calls: 7,
            cpu_ns: 100,
            wall_ns: 200,
        };
        record(p);
        record(p);
        let sum = take();
        assert_eq!(
            sum.spans[3],
            Span {
                calls: 14,
                cpu_ns: 200,
                wall_ns: 400
            }
        );
        assert!(!sum.overflow);
        record(p);
        record(ScanProfile {
            calls: 1,
            mode: Mode::Empty,
            ..Default::default()
        });
        assert!(take().overflow);
    }
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
