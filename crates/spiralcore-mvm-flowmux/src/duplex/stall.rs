//! One receiver phase and a nonblocking view of the writer's original pending
//! frame. No data bytes, file paths, credentials or transfer IDs are retained.
use super::Pending;
use spiralcore_transfer::witness_stall::{CommitObserver, CommitStage, with_observer};
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
    CommitLookup,
    CommitQuota,
    CommitStageWrite,
    CommitFileSync,
    CommitNamespacePublish,
    CommitVerifyPublished,
    CommitOrderedReadback,
}
const PHASE_NAMES: [&str; 16] = [
    "starting",
    "poll",
    "receive",
    "endpoint_handle",
    "retention",
    "readback",
    "delivery",
    "reply_lock",
    "reply_write",
    "commit_lookup",
    "commit_quota",
    "commit_stage_write",
    "commit_file_sync",
    "commit_namespace_publish",
    "commit_verify_published",
    "commit_ordered_readback",
];
// Only the receiver writes this word. A new phase clears all delivery flags.
const DELIVERY_FULL: u64 = 1 << 5;
const DELIVERY_TIMEOUT: u64 = 1 << 6;
const DELIVERY_ERROR: u64 = 1 << 7;

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

impl CommitObserver for Marks {
    fn mark_stage(&self, stage: Option<CommitStage>) {
        let phase = match stage {
            None => WorkerPhase::EndpointHandle,
            Some(CommitStage::Lookup) => WorkerPhase::CommitLookup,
            Some(CommitStage::Quota) => WorkerPhase::CommitQuota,
            Some(CommitStage::StageWrite) => WorkerPhase::CommitStageWrite,
            Some(CommitStage::FileSync) => WorkerPhase::CommitFileSync,
            Some(CommitStage::NamespacePublish) => WorkerPhase::CommitNamespacePublish,
            Some(CommitStage::VerifyPublished) => WorkerPhase::CommitVerifyPublished,
            Some(CommitStage::OrderedReadback) => WorkerPhase::CommitOrderedReadback,
        };
        // The bound worker is the sole writer. Preserve its kind and ordinal.
        let word = self.worker.load(Ordering::Relaxed);
        self.worker
            .store((word & !0xff) | phase as u64, Ordering::Relaxed);
    }
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
    pub(super) fn with_commit_stages<R>(&self, operation: impl FnOnce() -> R) -> R {
        with_observer(self.marks.clone(), operation)
    }

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
    pub(super) fn delivery_full(&self) {
        self.marks.worker.fetch_or(DELIVERY_FULL, Ordering::Relaxed);
    }
    pub(super) fn delivery_failed(&self, error: &crate::Error) {
        let flag = if matches!(error, crate::Error::Timeout) {
            DELIVERY_TIMEOUT
        } else {
            DELIVERY_ERROR
        };
        self.marks.worker.fetch_or(flag, Ordering::Relaxed);
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
        let worker_phase = match phase & (DELIVERY_FULL | DELIVERY_TIMEOUT | DELIVERY_ERROR) {
            0 => PHASE_NAMES[(phase as u8) as usize],
            DELIVERY_FULL => "delivery_queue_full",
            DELIVERY_TIMEOUT => "delivery_timeout",
            DELIVERY_ERROR => "delivery_error",
            flags if flags == DELIVERY_FULL | DELIVERY_TIMEOUT => "delivery_timeout_after_full",
            _ => "delivery_error_after_full",
        };
        StallSample {
            sender_phase: SENDER_NAMES[self.marks.sender.load(Ordering::Relaxed) as usize],
            worker_phase,
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
        check_commit_stages(&observer, now);
        observer.mark(WorkerPhase::Delivery, 3, 2024);
        observer.delivery_failed(&crate::Error::Timeout);
        assert_eq!(observer.sample(now).worker_phase, "delivery_timeout");
        observer.mark(WorkerPhase::Delivery, 3, 2025);
        assert_eq!(observer.sample(now).worker_phase, "delivery");
        observer.delivery_full();
        let full = observer.sample(now);
        assert_eq!(
            (full.worker_phase, full.worker_kind, full.worker_ordinal),
            ("delivery_queue_full", 3, 2025)
        );
        observer.delivery_failed(&crate::Error::Timeout);
        assert_eq!(
            observer.sample(now).worker_phase,
            "delivery_timeout_after_full"
        );
        observer.mark(WorkerPhase::Delivery, 3, 2026);
        observer.delivery_failed(&crate::Error::Cancelled);
        assert_eq!(observer.sample(now).worker_phase, "delivery_error");
        observer.mark(WorkerPhase::ReplyLock, 3, 2027);
        assert_eq!(observer.sample(now).worker_phase, "reply_lock");
        let guard = pending.lock().unwrap();
        assert_eq!(observer.sample(now).pending, PendingSample::Busy);
        drop(guard);
        *pending.lock().unwrap() = None;
        assert_eq!(observer.sample(now).pending, PendingSample::Empty);
        drop(pending);
        assert_eq!(observer.sample(now).pending, PendingSample::Detached);
    }

    fn check_commit_stages(observer: &StallObserver, now: Instant) {
        use spiralcore_transfer::witness_stall::{CommitStage::*, with_stage};
        let sibling_pending = Arc::new(Mutex::new(None));
        let sibling = StallObserver::new(sibling_pending.clone());
        sibling.mark(WorkerPhase::EndpointHandle, 3, 17);
        let sample = |phase| {
            let current = observer.sample(now);
            assert_eq!(
                (
                    current.worker_phase,
                    current.worker_kind,
                    current.worker_ordinal
                ),
                (phase, 3, 2024)
            );
            assert_eq!(current.sender_phase, "wait_reply");
            assert_eq!(
                current.pending,
                PendingSample::Frame {
                    kind: 3,
                    sequence: 2,
                    remaining_ms: 99
                }
            );
            let other = sibling.sample(now);
            assert_eq!(
                (other.worker_phase, other.worker_ordinal),
                ("endpoint_handle", 17)
            );
            assert_eq!(other.pending, PendingSample::Empty);
        };
        observer.with_commit_stages(|| {
            for (stage, name) in [
                (Lookup, "commit_lookup"),
                (Quota, "commit_quota"),
                (StageWrite, "commit_stage_write"),
                (FileSync, "commit_file_sync"),
                (NamespacePublish, "commit_namespace_publish"),
                (VerifyPublished, "commit_verify_published"),
                (OrderedReadback, "commit_ordered_readback"),
            ] {
                assert!(matches!(
                    with_stage(stage, || {
                        sample(name);
                        Err::<(), _>(crate::Error::Timeout)
                    }),
                    Err(crate::Error::Timeout)
                ));
                sample("endpoint_handle");
            }
            with_stage(StageWrite, || {
                let panic = std::panic::catch_unwind(|| {
                    with_stage(FileSync, || std::panic::panic_any(37u8))
                });
                assert_eq!(panic.unwrap_err().downcast_ref::<u8>(), Some(&37));
                sample("commit_stage_write");
            });
            sample("endpoint_handle");
            let panic = std::panic::catch_unwind(|| {
                sibling.with_commit_stages(|| {
                    with_stage(Quota, || {
                        assert_eq!(sibling.sample(now).worker_phase, "commit_quota");
                        std::panic::panic_any(93u8)
                    })
                })
            });
            assert_eq!(panic.unwrap_err().downcast_ref::<u8>(), Some(&93));
            with_stage(Lookup, || sample("commit_lookup"));
        });
        with_stage(Quota, || sample("endpoint_handle"));
    }
}
