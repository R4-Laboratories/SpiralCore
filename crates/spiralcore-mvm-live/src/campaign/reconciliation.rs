use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};

/// An explicit request to reconcile one current Call, never another identity.
/// Full coalesces requests; Disconnected means that invocation has ended.
pub struct ReconciliationSignal {
    sender: SyncSender<()>,
}
impl ReconciliationSignal {
    pub fn request_retry(&self) -> Result<(), TrySendError<()>> {
        self.sender.try_send(())
    }
}

pub(super) struct RetryRequests {
    receiver: Receiver<()>,
}
impl RetryRequests {
    pub(super) fn take_request(&self) -> bool {
        self.receiver.try_recv().is_ok()
    }
}

pub(super) fn bounded_requests() -> (ReconciliationSignal, RetryRequests) {
    let (sender, receiver) = sync_channel(1);
    (ReconciliationSignal { sender }, RetryRequests { receiver })
}

/// Explicit replay of one active cached Reply. No automatic retry or new Call.
pub struct ReplyReplaySignal {
    sender: SyncSender<()>,
}
impl ReplyReplaySignal {
    pub fn request_replay(&self) -> Result<(), TrySendError<()>> {
        self.sender.try_send(())
    }
}
pub(super) struct ReplyReplayRequests {
    receiver: Receiver<()>,
}
impl ReplyReplayRequests {
    pub(super) fn take_request(&self) -> bool {
        self.receiver.try_recv().is_ok()
    }
}
pub(super) fn bounded_reply_replays() -> (ReplyReplaySignal, ReplyReplayRequests) {
    let (sender, receiver) = sync_channel(1);
    (
        ReplyReplaySignal { sender },
        ReplyReplayRequests { receiver },
    )
}

#[derive(Default, serde::Serialize)]
pub(super) struct RecoveryCounters {
    pub(super) idle_outcomes: u64,
    pub(super) commands_consumed: u64,
    pub(super) call_retry_invocations: u64,
    pub(super) historical_control_events: u64,
    pub(super) historical_receipt_invocations: u64,
    pub(super) historical_receipts_sent: u64,
    pub(super) cached_reply_retry_invocations: u64,
    pub(super) reply_send_invocations: u64,
    pub(super) receiver_failed_invocations: u64,
    pub(super) tail_idle_outcomes: u64,
    pub(super) tail_events: u64,
    pub(super) tail_ack_invocations: u64,
    pub(super) tail_acks_written: u64,
    pub(super) reply_replay_commands_consumed: u64,
}

pub(super) fn increment(counter: &mut u64) -> crate::Result<()> {
    *counter = counter.checked_add(1).ok_or("recovery counter overflow")?;
    Ok(())
}

pub(super) type ProcessingPhase = (cpu_time::ProcessTime, std::time::Instant);

#[derive(Default, serde::Serialize)]
pub(super) struct ProcessingTotals {
    cpu_us: u64,
    wall_us: u64,
    segments: u64,
}
impl ProcessingTotals {
    pub(super) fn finish(
        &mut self,
        phase: &mut Option<ProcessingPhase>,
    ) -> crate::Result<super::Meter> {
        let Some((cpu, wall)) = phase.take() else {
            return Ok(super::Meter::default());
        };
        let cpu = u64::try_from(cpu.elapsed().as_micros())?;
        let wall = u64::try_from(wall.elapsed().as_micros())?;
        let total_cpu = self.cpu_us.checked_add(cpu).ok_or("CPU counter overflow")?;
        let total_wall = self
            .wall_us
            .checked_add(wall)
            .ok_or("wall counter overflow")?;
        let segments = self
            .segments
            .checked_add(1)
            .ok_or("segment counter overflow")?;
        self.cpu_us = total_cpu;
        self.wall_us = total_wall;
        self.segments = segments;
        Ok(super::Meter {
            cpu_us: cpu.into(),
            wall_us: wall.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::TrySendError;

    #[test]
    fn reply_replay_signal_is_capacity_one_and_cannot_control_a_later_handle() {
        let (signal, requests) = bounded_reply_replays();
        assert_eq!(signal.request_replay(), Ok(()));
        assert_eq!(signal.request_replay(), Err(TrySendError::Full(())));
        assert!(requests.take_request());
        assert!(!requests.take_request());
        drop(requests);
        let (_current, current_requests) = bounded_reply_replays();
        assert_eq!(signal.request_replay(), Err(TrySendError::Disconnected(())));
        assert!(!current_requests.take_request());
    }

    #[test]
    fn retry_commands_have_one_slot_and_drop_disconnects_the_current_signal() {
        let (signal, requests) = bounded_requests();
        assert!(!requests.take_request());
        assert_eq!(signal.request_retry(), Ok(()));
        assert_eq!(signal.request_retry(), Err(TrySendError::Full(())));
        assert!(requests.take_request());
        assert!(!requests.take_request());
        assert_eq!(signal.request_retry(), Ok(()));
        drop(requests);
        assert_eq!(signal.request_retry(), Err(TrySendError::Disconnected(())));
    }

    #[test]
    fn stale_signal_cannot_enqueue_on_the_next_calls_channel() {
        let (old, old_requests) = bounded_requests();
        assert_eq!(old.request_retry(), Ok(()));
        drop(old_requests);
        let (current, requests) = bounded_requests();
        assert_eq!(old.request_retry(), Err(TrySendError::Disconnected(())));
        assert!(!requests.take_request());
        assert_eq!(current.request_retry(), Ok(()));
        assert!(requests.take_request());
        drop(current);
        assert!(!requests.take_request());
    }
}
