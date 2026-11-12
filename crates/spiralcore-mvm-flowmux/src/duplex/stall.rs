//! One receiver phase and a nonblocking view of the writer's original pending
//! frame. No data bytes, file paths, credentials or transfer IDs are retained.
use super::Pending;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU8, AtomicU64, Ordering},
    },
    time::Instant,
};

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub(super) enum WorkerPhase {
    Starting,
    Poll,
    Receive,
    EndpointHandle,
    Retention,
    Readback,
    Delivery,
    ReplyLock,
    ReplyWrite,
}
const PHASE_NAMES: [&str; 9] = [
    "starting",
    "poll",
    "receive",
    "endpoint_handle",
    "retention",
    "readback",
    "delivery",
    "reply_lock",
    "reply_write",
];

#[repr(u8)]
pub(super) enum SenderPhase {
    Caller,
    TxLock,
    Send,
    Wait,
}
const SENDER_NAMES: [&str; 4] = ["caller", "tx_lock", "send", "wait_reply"];
struct Marks {
    worker: AtomicU64,
    sender: AtomicU8,
}

#[derive(Clone)]
pub struct StallObserver {
    marks: Arc<Marks>,
    pending: std::sync::Weak<std::sync::Mutex<Option<super::Expected>>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingSample {
    Empty,
    Busy,
    Poisoned,
    Detached,
    Frame {
        kind: u8,
        sequence: u64,
        remaining_ms: u64,
    },
}
#[derive(Debug, Clone, Copy)]
pub struct StallSample {
    pub sender_phase: &'static str,
    pub worker_phase: &'static str,
    pub worker_kind: u8,
    pub worker_ordinal: u64,
    pub pending: PendingSample,
}
impl StallObserver {
    pub(super) fn new(pending: Pending) -> Self {
        Self {
            marks: Arc::new(Marks {
                worker: AtomicU64::new(WorkerPhase::Starting as u64),
                sender: AtomicU8::new(SenderPhase::Caller as u8),
            }),
            pending: Arc::downgrade(&pending),
        }
    }
    pub(super) fn mark(&self, phase: WorkerPhase, kind: u8, ordinal: u64) {
        // The packed ordinal saturates at 48 bits for this diagnostic only.
        let word = (ordinal.min(u64::MAX >> 16) << 16) | (u64::from(kind) << 8) | phase as u64;
        self.marks.worker.store(word, Ordering::Relaxed);
    }
    pub(super) fn sender(&self, phase: SenderPhase) {
        self.marks.sender.store(phase as u8, Ordering::Relaxed);
    }
    pub fn sample(&self, now: Instant) -> StallSample {
        let phase = self.marks.worker.load(Ordering::Relaxed);
        let original = self.pending.upgrade();
        let pending = match original.as_ref().map(|original| original.try_lock()) {
            None => PendingSample::Detached,
            Some(Ok(slot)) => match slot.as_ref() {
                Some(frame) => PendingSample::Frame {
                    kind: frame.kind,
                    sequence: frame.seq,
                    remaining_ms: frame
                        .until
                        .saturating_duration_since(now)
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64,
                },
                None => PendingSample::Empty,
            },
            Some(Err(std::sync::TryLockError::WouldBlock)) => PendingSample::Busy,
            Some(Err(std::sync::TryLockError::Poisoned(_))) => PendingSample::Poisoned,
        };
        StallSample {
            sender_phase: SENDER_NAMES[self.marks.sender.load(Ordering::Relaxed) as usize],
            worker_phase: PHASE_NAMES[(phase & 255) as usize],
            worker_kind: (phase >> 8) as u8,
            worker_ordinal: phase >> 16,
            pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Purpose, duplex::Expected};
    use std::{
        sync::{Mutex, atomic::AtomicUsize},
        time::Duration,
    };
    #[test]
    fn sample_is_nonblocking_and_keeps_the_original_deadline() {
        let now = Instant::now();
        let pending = Arc::new(Mutex::new(Some(Expected {
            kind: 3,
            purpose: Purpose::Application,
            transfer: [3; 32],
            seq: 2,
            until: now + Duration::from_millis(99),
            seen: Arc::new(AtomicUsize::new(0)),
        })));
        let observer = StallObserver::new(pending.clone());
        observer.mark(WorkerPhase::EndpointHandle, 3, 2024);
        observer.sender(SenderPhase::Wait);
        let sample = observer.sample(now);
        assert_eq!(
            (
                sample.worker_phase,
                sample.worker_kind,
                sample.worker_ordinal,
                sample.sender_phase
            ),
            ("endpoint_handle", 3, 2024, "wait_reply")
        );
        assert_eq!(
            sample.pending,
            PendingSample::Frame {
                kind: 3,
                sequence: 2,
                remaining_ms: 99
            }
        );
        assert_eq!(
            observer.sample(now + Duration::from_secs(2)).pending,
            PendingSample::Frame {
                kind: 3,
                sequence: 2,
                remaining_ms: 0
            }
        );
        let guard = pending.lock().unwrap();
        assert_eq!(observer.sample(now).pending, PendingSample::Busy);
        drop(guard);
        *pending.lock().unwrap() = None;
        assert_eq!(observer.sample(now).pending, PendingSample::Empty);
        drop(pending);
        assert_eq!(observer.sample(now).pending, PendingSample::Detached);
    }
}
