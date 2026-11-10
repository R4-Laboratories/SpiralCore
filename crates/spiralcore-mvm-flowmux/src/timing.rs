//! Fixed diagnostic counters. Snapshots are concurrent observations, not a clock.
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn saturation_retains_maximum_and_marks_overflow_without_error() {
        let owner = TimingObserver::default();
        let metric = Metric::PauseActual(PauseReason::Read);
        owner.record(metric, Duration::MAX);
        owner.record(metric, Duration::from_nanos(1));
        let snapshot = owner.snapshot();
        let row = snapshot.get(metric);
        assert_eq!(row.count, 2);
        assert_eq!(row.total_ns, u64::MAX);
        assert_eq!(row.max_ns, u64::MAX);
        assert!(snapshot.overflow);
        owner.inner.cells[metric.slot()]
            .count
            .store(u64::MAX, Ordering::Relaxed);
        owner.record(metric, Duration::ZERO);
        assert_eq!(owner.snapshot().get(metric).count, u64::MAX);
    }

    #[test]
    fn guard_records_error_return_and_closed_frame_mapping() {
        let owner = TimingObserver::default();
        let metric = Metric::Frame(Phase::ReceiverHandle, FrameKind::from_wire(3));
        fn failed_operation(owner: &TimingObserver, metric: Metric) -> Result<(), ()> {
            let _span = owner.span(metric);
            Err(())
        }
        let result = failed_operation(&owner, metric);
        assert!(result.is_err());
        assert_eq!(owner.snapshot().get(metric).count, 1);
        assert_eq!(FrameKind::from_wire(255), FrameKind::Other);
        assert_ne!(
            Metric::PauseActual(PauseReason::Read).slot(),
            Metric::PauseRequested(PauseReason::Read).slot()
        );
        assert_ne!(
            Metric::Poll(PollOutcome::Empty).slot(),
            Metric::Poll(PollOutcome::Wake).slot()
        );
        assert!(TimingObserver::owned_bytes() < 8192);
        for slot in 0..SLOTS {
            assert_eq!(Metric::at(slot).slot(), slot);
        }
        for wire in 1..=6 {
            assert_ne!(FrameKind::from_wire(wire), FrameKind::Other);
        }
    }
}
use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug)]
pub enum PauseReason {
    Handshake,
    Poller,
    Session,
    Read,
    Write,
    Flush,
    Shutdown,
}
impl PauseReason {
    const ALL: [Self; 7] = [
        Self::Handshake,
        Self::Poller,
        Self::Session,
        Self::Read,
        Self::Write,
        Self::Flush,
        Self::Shutdown,
    ];
}
#[derive(Clone, Copy, Debug)]
pub enum PollOutcome {
    Requested,
    Actual,
    SocketRead,
    SocketWrite,
    Wake,
    Empty,
    Interrupted,
    Error,
    OwnerRefused,
}
impl PollOutcome {
    const ALL: [Self; 9] = [
        Self::Requested,
        Self::Actual,
        Self::SocketRead,
        Self::SocketWrite,
        Self::Wake,
        Self::Empty,
        Self::Interrupted,
        Self::Error,
        Self::OwnerRefused,
    ];
}
#[derive(Clone, Copy, Debug)]
pub enum Phase {
    SenderLock,
    SenderSend,
    SenderWait,
    ReceiverHandle,
    ReceiverLock,
    ReceiverSend,
}
impl Phase {
    const ALL: [Self; 6] = [
        Self::SenderLock,
        Self::SenderSend,
        Self::SenderWait,
        Self::ReceiverHandle,
        Self::ReceiverLock,
        Self::ReceiverSend,
    ];
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameKind {
    Begin,
    Data,
    Commit,
    Resume,
    Status,
    Cancel,
    Other,
}
impl FrameKind {
    const ALL: [Self; 7] = [
        Self::Begin,
        Self::Data,
        Self::Commit,
        Self::Resume,
        Self::Status,
        Self::Cancel,
        Self::Other,
    ];
    pub fn from_wire(kind: u8) -> Self {
        match kind {
            1 => Self::Begin,
            2 => Self::Data,
            3 => Self::Commit,
            4 => Self::Resume,
            5 => Self::Status,
            6 => Self::Cancel,
            _ => Self::Other,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Observation {
    SessionOperation,
    SessionContended,
    SenderLockContended,
    ReceiverLockContended,
    WaitOk,
    WaitTimeout,
    WaitDisconnected,
    WaitRefused,
    Ack,
}
impl Observation {
    const ALL: [Self; 9] = [
        Self::SessionOperation,
        Self::SessionContended,
        Self::SenderLockContended,
        Self::ReceiverLockContended,
        Self::WaitOk,
        Self::WaitTimeout,
        Self::WaitDisconnected,
        Self::WaitRefused,
        Self::Ack,
    ];
}
#[derive(Clone, Copy, Debug)]
pub enum Metric {
    PauseRequested(PauseReason),
    PauseActual(PauseReason),
    Poll(PollOutcome),
    Frame(Phase, FrameKind),
    Count(Observation),
}
const SLOTS: usize = 74;
impl Metric {
    fn slot(self) -> usize {
        match self {
            Self::PauseRequested(r) => r as usize,
            Self::PauseActual(r) => 7 + r as usize,
            Self::Poll(r) => 14 + r as usize,
            Self::Frame(p, k) => 23 + p as usize * 7 + k as usize,
            Self::Count(o) => 65 + o as usize,
        }
    }
    fn at(slot: usize) -> Self {
        match slot {
            0..7 => Self::PauseRequested(PauseReason::ALL[slot]),
            7..14 => Self::PauseActual(PauseReason::ALL[slot - 7]),
            14..23 => Self::Poll(PollOutcome::ALL[slot - 14]),
            23..65 => Self::Frame(Phase::ALL[(slot - 23) / 7], FrameKind::ALL[(slot - 23) % 7]),
            _ => Self::Count(Observation::ALL[slot - 65]),
        }
    }
}
#[derive(Default)]
struct Cell {
    count: AtomicU64,
    total: AtomicU64,
    maximum: AtomicU64,
}
struct Owner {
    cells: [Cell; SLOTS],
    overflow: AtomicBool,
}
#[derive(Clone)]
pub struct TimingObserver {
    inner: Arc<Owner>,
}
impl Default for TimingObserver {
    fn default() -> Self {
        Self {
            inner: Arc::new(Owner {
                cells: std::array::from_fn(|_| Cell::default()),
                overflow: AtomicBool::new(false),
            }),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct SpanSnapshot {
    pub count: u64,
    pub total_ns: u64,
    pub max_ns: u64,
}
pub struct Snapshot {
    rows: [SpanSnapshot; SLOTS],
    pub overflow: bool,
}
impl Snapshot {
    pub fn get(&self, metric: Metric) -> SpanSnapshot {
        self.rows[metric.slot()]
    }
}
impl fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_map();
        map.entry(&"overflow", &self.overflow);
        for (i, row) in self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.count != 0)
        {
            map.entry(&Metric::at(i), row);
        }
        map.finish()
    }
}
impl TimingObserver {
    /// Owned fixed state plus Arc counters, excluding allocator bookkeeping.
    pub const fn owned_bytes() -> usize {
        std::mem::size_of::<Owner>() + 2 * std::mem::size_of::<usize>()
    }
    fn add(&self, value: &AtomicU64, amount: u64) {
        let _ = value.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |old| {
            Some(old.checked_add(amount).unwrap_or_else(|| {
                self.inner.overflow.store(true, Ordering::Relaxed);
                u64::MAX
            }))
        });
    }
    pub fn record(&self, metric: Metric, elapsed: Duration) {
        let ns = u64::try_from(elapsed.as_nanos()).unwrap_or_else(|_| {
            self.inner.overflow.store(true, Ordering::Relaxed);
            u64::MAX
        });
        let cell = &self.inner.cells[metric.slot()];
        self.add(&cell.count, 1);
        self.add(&cell.total, ns);
        cell.maximum.fetch_max(ns, Ordering::Relaxed);
    }
    pub fn count(&self, observation: Observation) {
        self.record(Metric::Count(observation), Duration::ZERO);
    }
    pub fn span(&self, metric: Metric) -> Span<'_> {
        Span {
            owner: self,
            metric,
            began: Instant::now(),
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            rows: std::array::from_fn(|i| {
                let c = &self.inner.cells[i];
                SpanSnapshot {
                    count: c.count.load(Ordering::Relaxed),
                    total_ns: c.total.load(Ordering::Relaxed),
                    max_ns: c.maximum.load(Ordering::Relaxed),
                }
            }),
            overflow: self.inner.overflow.load(Ordering::Relaxed),
        }
    }
}
pub struct Span<'a> {
    owner: &'a TimingObserver,
    metric: Metric,
    began: Instant,
}
impl Drop for Span<'_> {
    fn drop(&mut self) {
        self.owner.record(self.metric, self.began.elapsed());
    }
}
