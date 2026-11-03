use super::*;
use crate::{
    campaign::reconciliation::bounded_requests,
    config::{Arm, Pair, Plan, Stage},
};
use std::{
    collections::VecDeque,
    net::Shutdown,
    sync::{Arc, Mutex, mpsc},
    thread::JoinHandle,
};

const SIGNED_TTL: Duration = Duration::from_millis(900);
const PARENT_TIMEOUT: Duration = Duration::from_secs(2);
const WATCHDOG: Duration = Duration::from_millis(1500);
const RETRY_PAYLOAD_PREFIX: usize = 7;

struct State {
    incoming: VecDeque<u8>,
    written: Vec<u8>,
    read_timeout: Duration,
    write_timeout: Duration,
    initial_length: Option<usize>,
    initial_sent: Option<mpsc::SyncSender<Instant>>,
    retry_started: Option<Instant>,
    stalls: usize,
    reads: usize,
    writes: usize,
    shutdowns: usize,
}
struct PartialRetry(Arc<Mutex<State>>);
impl Read for PartialRetry {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let timeout = {
            let mut state = self.0.lock().unwrap();
            state.reads = state.reads.checked_add(1).unwrap();
            if !state.incoming.is_empty() {
                let count = bytes.len().min(state.incoming.len());
                for byte in &mut bytes[..count] {
                    *byte = state.incoming.pop_front().unwrap();
                }
                return Ok(count);
            }
            state.read_timeout
        };
        std::thread::sleep(timeout);
        Err(io::ErrorKind::TimedOut.into())
    }
}
impl Write for PartialRetry {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let timeout = {
            let mut state = self.0.lock().unwrap();
            state.writes = state.writes.checked_add(1).unwrap();
            let cap = state
                .initial_length
                .map(|length| 12 + length + 4 + RETRY_PAYLOAD_PREFIX);
            let count = cap.map_or(bytes.len(), |cap| {
                bytes.len().min(cap.saturating_sub(state.written.len()))
            });
            if count > 0 {
                if state
                    .initial_length
                    .is_some_and(|length| state.written.len() >= 12 + length)
                    && state.retry_started.is_none()
                {
                    state.retry_started = Some(Instant::now());
                }
                state.written.extend_from_slice(&bytes[..count]);
                if state.initial_length.is_none() && state.written.len() >= 12 {
                    state.initial_length =
                        Some(u32::from_be_bytes(state.written[8..12].try_into().unwrap()) as usize);
                }
                if state
                    .initial_length
                    .is_some_and(|length| state.written.len() == 12 + length)
                    && let Some(ready) = state.initial_sent.take()
                {
                    ready
                        .send(Instant::now())
                        .map_err(|_| io::Error::other("lost initial-send observer"))?;
                }
                return Ok(count);
            }
            state.stalls = state.stalls.checked_add(1).unwrap();
            state.write_timeout
        };
        // Bounded SocketIo backpressure: only the prefix above was accepted.
        // This is write-loop conformance, not a real OS/TCP pressure claim.
        std::thread::sleep(timeout);
        Err(io::ErrorKind::WouldBlock.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl SocketIo for PartialRetry {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.0.lock().unwrap().read_timeout = timeout
            .filter(|timeout| !timeout.is_zero())
            .ok_or_else(|| io::Error::other("missing positive read timeout"))?;
        Ok(())
    }
    fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.0.lock().unwrap().write_timeout = timeout
            .filter(|timeout| !timeout.is_zero())
            .ok_or_else(|| io::Error::other("missing positive write timeout"))?;
        Ok(())
    }
    fn shutdown(&self, _how: Shutdown) -> io::Result<()> {
        let mut state = self.0.lock().unwrap();
        state.shutdowns = state.shutdowns.checked_add(1).unwrap();
        Ok(())
    }
}

struct Report {
    result: Result<(Context, Frame)>,
    session: Session,
    original_wire: Vec<u8>,
    original_header: mvm_contract::protocol::mailbox::MailboxEnvelopeHeader,
    counters: StreamCounters,
    after_reentry: StreamCounters,
    reads_before: usize,
    reads_after: usize,
    writes_before: usize,
    writes_after: usize,
    reentry: Result<(Context, Frame)>,
    lifecycle_wall_us: u64,
}
struct Worker {
    cancel: CancellationToken,
    worker: Option<JoinHandle<Report>>,
}
impl Worker {
    fn finish(mut self) -> Report {
        self.cancel.cancel();
        self.worker
            .take()
            .unwrap()
            .join()
            .expect("late retry worker panicked")
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[test]
fn late_explicit_call_retry_partial_write_keeps_original_signed_deadline_and_identity() {
    let pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 128,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    let broker = broker_context().unwrap();
    let mut caller = Session::new(&pair.sender, &broker).unwrap();
    let request = Frame {
        kind: 5,
        transfer: [91; 32],
        seq: 0,
        body: Vec::new(),
    };
    let payload = request.encode().unwrap();
    let seeded_at = Instant::now();
    broker
        .with_time(|_, now| -> Result<()> {
            // Allocate the ordinary signed options once, with a shorter legitimate
            // lifetime, before the initial registration/send. No header mutation.
            let mut options = caller.ordering.next_for_binding(
                &caller.outgoing,
                mvm_contract::protocol::mailbox::ContentType::parse(
                    spiralcore_mvm_adapter::CONTENT_TYPE,
                    1,
                )?,
                now,
                now.unix_ms + i64::try_from(SIGNED_TTL.as_millis())?,
                ISSUANCE_POLICY,
            )?;
            options.reply_to = caller.reply_to;
            caller.pending = Some(PendingCall::new(payload, options, now));
            Ok(())
        })
        .unwrap();
    let (initial_tx, initial_sent) = mpsc::sync_channel(1);
    let state = Arc::new(Mutex::new(State {
        incoming: b"SCMVS001".iter().copied().collect(),
        written: Vec::new(),
        read_timeout: Duration::from_millis(50),
        write_timeout: Duration::from_millis(50),
        initial_length: None,
        initial_sent: Some(initial_tx),
        retry_started: None,
        stalls: 0,
        reads: 0,
        writes: 0,
        shutdowns: 0,
    }));
    let stream = StreamTransport::new(
        PartialRetry(state.clone()),
        pair.sender
            .outgoing
            .as_ref()
            .unwrap()
            .binding
            .claims
            .route_id,
        PARENT_TIMEOUT,
    )
    .unwrap();
    let (signal, requests) = bounded_requests();
    let cancel = CancellationToken::default();
    let token = cancel.clone();
    let observed = state.clone();
    let (done_tx, done) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let wall = Instant::now();
        let result = (|| -> Result<_> {
            stream.handshake(HandshakeRole::Initiator, &token)?;
            caller.call_with_reconciliation(&request, &stream, &token, requests)
        })();
        let lifecycle_wall_us = u64::try_from(wall.elapsed().as_micros()).unwrap();
        let pending = caller.pending.as_ref().unwrap();
        let original_wire = pending.encoded().unwrap().to_vec();
        let original_header = pending.prepared().unwrap().header().clone();
        let original_call = pending.call_handle();
        let original_send = pending.send_handle();
        let counters = stream.counters().unwrap();
        let (reads_before, writes_before) = {
            let state = observed.lock().unwrap();
            (state.reads, state.writes)
        };
        let (_, fresh_requests) = bounded_requests();
        let reentry = caller.call_with_reconciliation(&request, &stream, &token, fresh_requests);
        assert_eq!(
            caller.pending.as_ref().unwrap().call_handle(),
            original_call
        );
        assert_eq!(
            caller.pending.as_ref().unwrap().send_handle(),
            original_send
        );
        let after_reentry = stream.counters().unwrap();
        let (reads_after, writes_after) = {
            let state = observed.lock().unwrap();
            (state.reads, state.writes)
        };
        let _ = done_tx.send(());
        Report {
            result,
            session: caller,
            original_wire,
            original_header,
            counters,
            after_reentry,
            reads_before,
            reads_after,
            writes_before,
            writes_after,
            reentry,
            lifecycle_wall_us,
        }
    });
    let worker = Worker {
        cancel,
        worker: Some(worker),
    };
    let initial = initial_sent
        .recv_timeout(Duration::from_millis(500))
        .unwrap();
    let command_at = initial + Duration::from_millis(600);
    while let Some(wait) = command_at.checked_duration_since(Instant::now()) {
        std::thread::park_timeout(wait);
    }
    assert_eq!(signal.request_retry(), Ok(()));
    let completed = done
        .recv_timeout((seeded_at + WATCHDOG).saturating_duration_since(Instant::now()))
        .is_ok();
    let report = worker.finish();
    let state = state.lock().unwrap();
    let outcome = report.result.as_ref().err().map(ToString::to_string);
    if let Some(directory) = std::env::var_os("SPIRALCORE_RECONCILIATION_EVIDENCE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        let metadata = std::fs::symlink_metadata(&directory).unwrap();
        assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("late-call-retry-write-deadline.json"))
            .unwrap();
        serde_json::to_writer_pretty(file, &json!({
            "evidence":"canonical-reconciliation-test/v1", "case":"late-call-retry-write-deadline",
            "scope":"Scripted bounded SocketIo writer-loop conformance; not OS TCP backpressure or performance",
            "signed_remaining_ms":SIGNED_TTL.as_millis(), "parent_timeout_ms":PARENT_TIMEOUT.as_millis(),
            "watchdog_ms":WATCHDOG.as_millis(),"completed_before_watchdog":completed,
            "outcome":outcome,"worker_wall_us":report.lifecycle_wall_us,
            "retry_started_after_seed_us":state.retry_started.map(|at| at.duration_since(seeded_at).as_micros()),
            "stream_counters":report.counters,"counters_after_refused_reentry":report.after_reentry,
            "script_accepted_bytes":state.written.len(),"script_write_stalls":state.stalls,
            "original_envelope_bytes":report.original_wire.len(),"partial_retry_payload_bytes":RETRY_PAYLOAD_PREFIX,
            "send_invocations":report.session.pending.as_ref().unwrap().send_invocations(),
            "events_received":report.session.pending.as_ref().unwrap().events_received(),
            "application_calls_completed":report.session.calls,
            "read_calls_before_after_reentry":[report.reads_before,report.reads_after],
            "write_calls_before_after_reentry":[report.writes_before,report.writes_after],
            "accounting":"Only returned successful SocketIo bytes are counted; payloads and envelopes are not persisted"
        })).unwrap();
    }
    assert!(
        completed,
        "retry write exceeded original 900ms lifetime: {outcome:?}; partial bytes={}, stalls={}",
        state.written.len(),
        state.stalls
    );
    assert!(matches!(
        report.result.err().unwrap().downcast_ref::<ServiceError>(),
        Some(ServiceError::Unavailable)
    ));
    assert!(report.lifecycle_wall_us < u64::try_from(WATCHDOG.as_micros()).unwrap());
    let retry_started = state
        .retry_started
        .expect("explicit retry was never invoked");
    assert!(retry_started >= command_at);
    assert!(retry_started < seeded_at + SIGNED_TTL);
    assert!(
        state.stalls > 0,
        "must exercise I/O expiry after partial progress"
    );
    assert!(state.shutdowns > 0);
    let pending = report.session.pending.as_ref().unwrap();
    assert_eq!(pending.send_invocations(), 2);
    assert_eq!(pending.events_received(), 0);
    assert_eq!(pending.encoded().unwrap(), report.original_wire);
    assert_eq!(
        pending.prepared().unwrap().header(),
        &report.original_header
    );
    assert_eq!(pending.prepared().unwrap().header().sequence.get(), 1);
    assert_eq!(report.session.calls, 0);
    assert_eq!(&state.written[..8], b"SCMVS001");
    let end = 12 + report.original_wire.len();
    assert_eq!(
        &state.written[8..12],
        &(report.original_wire.len() as u32).to_be_bytes()
    );
    assert_eq!(&state.written[12..end], report.original_wire);
    assert_eq!(
        &state.written[end..end + 4],
        &(report.original_wire.len() as u32).to_be_bytes()
    );
    assert_eq!(
        &state.written[end + 4..],
        &report.original_wire[..RETRY_PAYLOAD_PREFIX]
    );
    assert_eq!(report.counters.frames_sent, 1);
    assert_eq!(report.counters.frames_received, 0);
    assert_eq!(report.counters.framing_bytes_sent, 16);
    assert_eq!(
        report.counters.envelope_bytes_sent,
        (report.original_wire.len() + RETRY_PAYLOAD_PREFIX) as u64
    );
    assert!(matches!(
        report.reentry.err().unwrap().downcast_ref::<SessionError>(),
        Some(SessionError::StreamFenced)
    ));
    assert_eq!(
        serde_json::to_value(report.counters).unwrap(),
        serde_json::to_value(report.after_reentry).unwrap()
    );
    assert_eq!(report.reads_before, report.reads_after);
    assert_eq!(report.writes_before, report.writes_after);
}
