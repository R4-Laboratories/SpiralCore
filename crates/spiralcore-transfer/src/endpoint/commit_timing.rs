//! Fixed opt-in process totals. Concurrent snapshots and nested spans overlap.
use cpu_time::ThreadTime;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Stage {
    Quota,
    Victim,
    CreateWrite,
    FileSync,
    NamespacePublish,
    VerifyPublished,
    Readback,
}
const STAGES: [Stage; 7] = [
    Stage::Quota,
    Stage::Victim,
    Stage::CreateWrite,
    Stage::FileSync,
    Stage::NamespacePublish,
    Stage::VerifyPublished,
    Stage::Readback,
];
#[derive(Default)]
struct Cell {
    calls: AtomicU64,
    active: AtomicU64,
    wall_ns: AtomicU64,
    max_wall_ns: AtomicU64,
    cpu_ns: AtomicU64,
    max_cpu_ns: AtomicU64,
    cpu_unavailable: AtomicU64,
}
static COUNTERS: [Cell; 7] = [const {
    Cell {
        calls: AtomicU64::new(0),
        active: AtomicU64::new(0),
        wall_ns: AtomicU64::new(0),
        max_wall_ns: AtomicU64::new(0),
        cpu_ns: AtomicU64::new(0),
        max_cpu_ns: AtomicU64::new(0),
        cpu_unavailable: AtomicU64::new(0),
    }
}; 7];

fn add(value: &AtomicU64, n: u64) {
    let _ = value.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |old| {
        Some(old.saturating_add(n))
    });
}
fn nanos(value: std::time::Duration) -> u64 {
    u64::try_from(value.as_nanos()).unwrap_or(u64::MAX)
}
pub(crate) struct Span {
    cell: &'static Cell,
    wall: Instant,
    cpu: Option<ThreadTime>,
}
pub(crate) fn span(stage: Stage) -> Span {
    let cell = &COUNTERS[stage as usize];
    add(&cell.active, 1);
    Span {
        cell,
        wall: Instant::now(),
        cpu: ThreadTime::try_now().ok(),
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        let wall = nanos(self.wall.elapsed());
        add(&self.cell.calls, 1);
        add(&self.cell.wall_ns, wall);
        self.cell.max_wall_ns.fetch_max(wall, Ordering::Relaxed);
        if let Some(cpu) = self.cpu.as_ref().and_then(|start| start.try_elapsed().ok()) {
            let cpu = nanos(cpu);
            add(&self.cell.cpu_ns, cpu);
            self.cell.max_cpu_ns.fetch_max(cpu, Ordering::Relaxed);
        } else {
            add(&self.cell.cpu_unavailable, 1);
        }
        self.cell.active.fetch_sub(1, Ordering::Relaxed);
    }
}
/// A bounded snapshot across both endpoints; fields are independently sampled.
pub struct Snapshot;
pub const fn fixed_static_bytes() -> usize {
    std::mem::size_of::<[Cell; 7]>()
}
pub fn snapshot() -> Snapshot {
    Snapshot
}
impl std::fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fields=calls,active,total_wall_ns,max_wall_ns,total_thread_cpu_ns,max_thread_cpu_ns,cpu_unavailable ")?;
        let mut map = f.debug_map();
        for stage in STAGES {
            let cell = &COUNTERS[stage as usize];
            map.entry(
                &stage,
                &[
                    cell.calls.load(Ordering::Relaxed),
                    cell.active.load(Ordering::Relaxed),
                    cell.wall_ns.load(Ordering::Relaxed),
                    cell.max_wall_ns.load(Ordering::Relaxed),
                    cell.cpu_ns.load(Ordering::Relaxed),
                    cell.max_cpu_ns.load(Ordering::Relaxed),
                    cell.cpu_unavailable.load(Ordering::Relaxed),
                ],
            );
        }
        map.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_operation_records_completion_and_retires_active_span() {
        let cell = Box::leak(Box::new(Cell::default()));
        fn fail(cell: &'static Cell) -> Result<(), ()> {
            add(&cell.active, 1);
            let _span = Span {
                cell,
                wall: Instant::now(),
                cpu: None,
            };
            assert_eq!(cell.active.load(Ordering::Relaxed), 1);
            Err(())
        }
        assert!(fail(cell).is_err());
        assert_eq!(cell.calls.load(Ordering::Relaxed), 1);
        assert_eq!(cell.active.load(Ordering::Relaxed), 0);
        assert_eq!(cell.cpu_unavailable.load(Ordering::Relaxed), 1);
        assert_eq!(
            cell.wall_ns.load(Ordering::Relaxed),
            cell.max_wall_ns.load(Ordering::Relaxed)
        );
        add(&cell.wall_ns, u64::MAX);
        assert_eq!(cell.wall_ns.load(Ordering::Relaxed), u64::MAX);
        let report = format!("{:?}", snapshot());
        assert!(report.contains("cpu_unavailable"));
        assert!(report.contains("CreateWrite"));
    }
}
