use super::*;
use crate::{
    campaign::reconciliation::bounded_requests,
    config::{Arm, Pair, Plan, Stage},
};
use spiralcore_mvm_adapter::receipt::ReceiptSendError;
use spiralcore_transfer::protocol::{Mode, Offer};
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
    seeded_at: Instant,
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

// Complete-frame boundaries are discovered from accepted canonical writes.
// Capture is bounded; this socket never reports success for stalled bytes.
struct Script {
    incoming: VecDeque<u8>,
    written: Vec<u8>,
    read_timeout: Duration,
    write_timeout: Duration,
    stall_frame: Option<usize>,
    stalled_at: Option<Instant>,
    stalls: usize,
    announced: usize,
    frames: mpsc::SyncSender<usize>,
    reads: usize,
    writes: usize,
}
fn frames(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut offset = 8;
    let mut frames = Vec::new();
    while bytes.len() >= offset + 4 {
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        assert!(length <= spiralcore_mvm_adapter::MAX_ENVELOPE_BYTES);
        let end = offset.checked_add(4).unwrap().checked_add(length).unwrap();
        if bytes.len() < end {
            break;
        }
        assert!(frames.len() < 32, "bounded fixture frame ledger");
        frames.push(bytes[offset + 4..end].to_vec());
        offset = end;
    }
    frames
}
fn frame_start(bytes: &[u8], ordinal: usize) -> Option<usize> {
    let mut offset = 8;
    for _ in 1..ordinal {
        if bytes.len() < offset + 4 {
            return None;
        }
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let end = offset.checked_add(4)?.checked_add(length)?;
        if bytes.len() < end {
            return None;
        }
        offset = end;
    }
    (bytes.len() >= offset).then_some(offset)
}
struct ScriptSocket(Arc<Mutex<Script>>);
impl Read for ScriptSocket {
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
impl Write for ScriptSocket {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let timeout = {
            let mut state = self.0.lock().unwrap();
            state.writes = state.writes.checked_add(1).unwrap();
            let start = state
                .stall_frame
                .and_then(|ordinal| frame_start(&state.written, ordinal));
            let count = start.map_or(bytes.len(), |offset| {
                bytes
                    .len()
                    .min((offset + 4 + RETRY_PAYLOAD_PREFIX).saturating_sub(state.written.len()))
            });
            if count > 0 {
                if start.is_some_and(|offset| state.written.len() >= offset)
                    && state.stalled_at.is_none()
                {
                    state.stalled_at = Some(Instant::now());
                }
                assert!(
                    state.written.len() + count <= 2 << 20,
                    "bounded fixture byte ledger"
                );
                state.written.extend_from_slice(&bytes[..count]);
                let complete = frames(&state.written).len();
                while state.announced < complete {
                    state.announced += 1;
                    state.frames.try_send(state.announced).map_err(|_| {
                        io::Error::other("fixture frame observer full/disconnected")
                    })?;
                }
                return Ok(count);
            }
            state.stalls = state.stalls.checked_add(1).unwrap();
            state.write_timeout
        };
        std::thread::sleep(timeout);
        Err(io::ErrorKind::WouldBlock.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl SocketIo for ScriptSocket {
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
        Ok(())
    }
}
fn scripted(
    bootstrap: &Bootstrap,
    stall_frame: Option<usize>,
) -> (
    StreamTransport<ScriptSocket>,
    Arc<Mutex<Script>>,
    mpsc::Receiver<usize>,
) {
    let (frames, observed) = mpsc::sync_channel(16);
    let state = Arc::new(Mutex::new(Script {
        incoming: b"SCMVS001".iter().copied().collect(),
        written: Vec::new(),
        read_timeout: Duration::from_millis(50),
        write_timeout: Duration::from_millis(50),
        stall_frame,
        stalled_at: None,
        stalls: 0,
        announced: 0,
        frames,
        reads: 0,
        writes: 0,
    }));
    let stream = StreamTransport::new(
        ScriptSocket(state.clone()),
        bootstrap.outgoing.as_ref().unwrap().binding.claims.route_id,
        PARENT_TIMEOUT,
    )
    .unwrap();
    (stream, state, observed)
}
fn enqueue(state: &Arc<Mutex<Script>>, wire: &[u8]) {
    assert!(wire.len() <= spiralcore_mvm_adapter::MAX_ENVELOPE_BYTES);
    let mut state = state.lock().unwrap();
    assert!(state.incoming.len() + 4 + wire.len() <= 2 << 20);
    state.incoming.extend((wire.len() as u32).to_be_bytes());
    state.incoming.extend(wire.iter().copied());
}
fn short_call(session: &mut Session, request: &Frame, ttl: Duration, as_pending: bool) -> Vec<u8> {
    let broker = session.broker.clone();
    broker
        .with_time(|_, now| -> Result<Vec<u8>> {
            let payload = request.encode()?;
            let mut options = session.ordering.next_for_binding(
                &session.outgoing,
                mvm_contract::protocol::mailbox::ContentType::parse(
                    spiralcore_mvm_adapter::CONTENT_TYPE,
                    1,
                )?,
                now,
                now.unix_ms + i64::try_from(ttl.as_millis())?,
                ISSUANCE_POLICY,
            )?;
            options.reply_to = session.reply_to;
            if as_pending {
                // Seal once through the real retained registration path. A
                // separately prepared oracle has a different fresh AEAD nonce.
                let mut pending = PendingCall::new(payload, options, now);
                pending.register(
                    &mut session.service,
                    &mut session.tracker,
                    &session.outgoing,
                    &session.control,
                    &session.peer_key,
                    now,
                )?;
                let wire = pending.encoded()?.to_vec();
                session.pending = Some(pending);
                Ok(wire)
            } else {
                let prepared = service_sender::prepare(&session.outgoing, &payload, &options, now)?;
                let wire = encode_envelope(prepared.envelope())?;
                let call = session
                    .service
                    .register_call(&session.outgoing, &wire, now)?;
                let captured = CapturedRoute::default();
                session.service.send_call_prepared(
                    &session.outgoing,
                    &call,
                    &prepared,
                    &captured,
                    &CancellationToken::default(),
                    now,
                )?;
                assert_eq!(captured.0.lock().unwrap().as_deref(), Some(wire.as_slice()));
                Ok(wire)
            }
        })
        .unwrap()
}

#[derive(Default)]
struct CapturedRoute(Arc<Mutex<Option<Vec<u8>>>>);
impl mvm_core::mailbox::transport::PeerTransport for CapturedRoute {
    fn connect(
        &self,
        _route: &mvm_core::mailbox::transport::AuthorizedRoute,
        _limits: mvm_core::mailbox::transport::ChannelLimits,
        _cancel: &CancellationToken,
    ) -> std::result::Result<
        Box<dyn mvm_core::mailbox::transport::PeerChannel>,
        mvm_core::mailbox::transport::TransportError,
    > {
        Ok(Box::new(CapturedFrame(self.0.clone())))
    }
}
struct CapturedFrame(Arc<Mutex<Option<Vec<u8>>>>);
impl mvm_core::mailbox::transport::PeerChannel for CapturedFrame {
    fn try_send(
        &mut self,
        frame: &mvm_core::mailbox::transport::AuthorizedMailboxFrame,
        _cancel: &CancellationToken,
    ) -> std::result::Result<(), mvm_core::mailbox::transport::TransportError> {
        *self.0.lock().unwrap() = Some(frame.encoded_bytes().to_vec());
        Ok(())
    }
}
fn canonical_reply(peer: &mut Session, request: &[u8], reply: &Frame) -> Vec<u8> {
    let broker = peer.broker.clone();
    broker
        .with_time(|_, now| -> Result<Vec<u8>> {
            peer.returns
                .admit(&mut peer.service, &peer.incoming, request, now)?;
            let proof = peer
                .service
                .receive(&peer.incoming, now)?
                .ok_or(ServiceError::State)?;
            let options = peer
                .ordering
                .next_reply(&peer.outgoing, &proof, now, ISSUANCE_POLICY)?;
            let prepared = peer.service.reply_prepared(
                &peer.outgoing,
                proof.handle(),
                service_sender::request(&peer.outgoing, &reply.encode()?, &options, now)?,
                now,
            )?;
            let wire = encode_envelope(prepared.envelope())?;
            peer.tracker.register_send(
                &peer.outgoing,
                &peer.control,
                &peer.peer_key,
                &wire,
                now,
            )?;
            let captured = CapturedRoute::default();
            peer.outgoing.send_prepared(
                &prepared,
                &captured,
                &CancellationToken::default(),
                now,
            )?;
            assert_eq!(captured.0.lock().unwrap().as_deref(), Some(wire.as_slice()));
            Ok(wire)
        })
        .unwrap()
}
fn pair() -> Pair {
    Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 128,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap()
}
fn evidence(name: &str, value: &Value) {
    let Some(directory) = std::env::var_os("SPIRALCORE_RECONCILIATION_EVIDENCE_DIR") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let metadata = std::fs::symlink_metadata(&directory).unwrap();
    assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(name))
        .unwrap();
    serde_json::to_writer_pretty(file, value).unwrap();
}
struct Case {
    session: Session,
    result: Result<()>,
    reentry: Result<()>,
    counters: StreamCounters,
    after: StreamCounters,
    io_before: (usize, usize),
    io_after: (usize, usize),
    wall_us: u64,
}
struct CaseWorker {
    cancel: CancellationToken,
    worker: Option<JoinHandle<Case>>,
}
impl CaseWorker {
    fn finish(mut self) -> Case {
        self.cancel.cancel();
        self.worker
            .take()
            .unwrap()
            .join()
            .expect("scripted write worker panicked")
    }
}
impl Drop for CaseWorker {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[test]
fn canonical_ack_initial_reply_and_cached_reply_partial_writes_keep_original_expiry() {
    for mode in ["ack", "initial-reply", "cached-reply"] {
        let pair = pair();
        let sender_broker = broker_context().unwrap();
        let receiver_broker = broker_context().unwrap();
        let mut sender = Session::new(&pair.sender, &sender_broker).unwrap();
        let receiver = Session::new(&pair.receiver, &receiver_broker).unwrap();
        let request = Frame {
            kind: 1,
            transfer: [94; 32],
            seq: 0,
            body: Offer::cold(b"x", Mode::Raw).encode().unwrap(),
        };

        let stall_frame = if mode == "cached-reply" { 3 } else { 2 };
        let bootstrap = if mode == "ack" {
            &pair.sender
        } else {
            &pair.receiver
        };
        let (stream, state, observed) = scripted(bootstrap, Some(stall_frame));

        let budget_owner = if mode == "ack" {
            sender_broker.clone()
        } else {
            receiver_broker.clone()
        };
        let cancel = CancellationToken::default();
        let token = cancel.clone();
        let watched = state.clone();
        let (done_tx, done) = mpsc::channel();
        let store = tempfile::tempdir().unwrap();
        let policy = spiralcore_mvm_adapter::Policy::new(
            pair.receiver.incoming.as_ref().unwrap().context().unwrap(),
        )
        .unwrap();
        let mut object = ObjectMailbox::open(
            &store.path().join("cache"),
            Limits::default(),
            policy,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        // Start the real signed 900 ms lifetime after the disk and original
        // stream have been prepared. The writer accepts an authenticated prefix
        // and then stalls until that same original deadline.
        let seeded = Instant::now();
        let call_wire = short_call(&mut sender, &request, SIGNED_TTL, mode == "ack");
        if mode != "ack" {
            enqueue(&state, &call_wire);
        }
        let mut session = if mode == "ack" { sender } else { receiver };
        let is_caller = mode == "ack";
        let worker = std::thread::spawn(move || {
            let wall = Instant::now();
            let result = (|| -> Result<()> {
                stream.handshake(
                    if is_caller {
                        HandshakeRole::Initiator
                    } else {
                        HandshakeRole::Acceptor
                    },
                    &token,
                )?;
                if is_caller {
                    session.call(&request, &stream, &token).map(|_| ())
                } else {
                    session
                        .handle(request.transfer, &mut object, &stream, &token)
                        .map(|_| ())
                }
            })();
            let wall_us = u64::try_from(wall.elapsed().as_micros()).unwrap();
            let counters = stream.counters().unwrap();
            let io_before = {
                let state = watched.lock().unwrap();
                (state.reads, state.writes)
            };
            let reentry = if is_caller {
                session.call(&request, &stream, &token).map(|_| ())
            } else {
                session
                    .handle(request.transfer, &mut object, &stream, &token)
                    .map(|_| ())
            };
            let after = stream.counters().unwrap();
            let io_after = {
                let state = watched.lock().unwrap();
                (state.reads, state.writes)
            };
            let _ = done_tx.send(());
            Case {
                session,
                result,
                reentry,
                counters,
                after,
                io_before,
                io_after,
                wall_us,
            }
        });
        let worker = CaseWorker {
            cancel,
            worker: Some(worker),
        };
        assert_eq!(
            observed.recv_timeout(Duration::from_millis(500)).unwrap(),
            1
        );
        let mut reply_wire = None;
        if mode == "ack" {
            let actual = frames(&state.lock().unwrap().written)[0].clone();
            assert_eq!(actual, call_wire);
            let reply = Frame {
                kind: 101,
                transfer: [94; 32],
                seq: 0,
                body: vec![8, 0, 7],
            };
            let wire = canonical_reply(
                &mut Session::new(&pair.receiver, &receiver_broker).unwrap(),
                &actual,
                &reply,
            );
            enqueue(&state, &wire);
            reply_wire = Some(wire);
        } else if mode == "cached-reply" {
            assert_eq!(
                observed.recv_timeout(Duration::from_millis(500)).unwrap(),
                2
            );
            reply_wire = Some(frames(&state.lock().unwrap().written)[1].clone());
            enqueue(&state, &call_wire);
        }
        let completed = done
            .recv_timeout((seeded + WATCHDOG).saturating_duration_since(Instant::now()))
            .is_ok();
        let case = worker.finish();
        let state = state.lock().unwrap();
        let complete = frames(&state.written);
        let partial_at = frame_start(&state.written, stall_frame).unwrap();
        let declared = u32::from_be_bytes(
            state.written[partial_at..partial_at + 4]
                .try_into()
                .unwrap(),
        ) as usize;
        let charged = budget_owner
            .with_time::<_, ServiceError>(|budget, _| budget.snapshot())
            .unwrap();
        evidence(
            &format!("{mode}-write-deadline.json"),
            &json!({
                "evidence":"canonical-reconciliation-test/v1","case":mode,
                "scope":"Scripted canonical writer-loop conformance, not OS TCP backpressure/performance",
                "signed_remaining_ms":900,"parent_timeout_ms":2000,"completed_before_watchdog":completed,
                "outcome":case.result.as_ref().err().map(ToString::to_string),"worker_wall_us":case.wall_us,
                "stream_counters":case.counters,"after_refused_reentry":case.after,
                "accepted_script_bytes":state.written.len(),"write_stalls":state.stalls,
                "target_frame_ordinal":stall_frame,"target_envelope_bytes":declared,"partial_payload_bytes":7,
                "retained_queued_bytes":charged.queued_bytes,"retained_queued_messages":charged.queued_messages,
                "application_calls_completed":case.session.calls,"io_before":case.io_before,"io_after":case.io_after
            }),
        );
        assert!(
            completed,
            "{mode}: original expiry was refreshed: {:?}",
            case.result.as_ref().err().map(|error| error.to_string())
        );
        let error = case.result.err().unwrap();
        if mode == "ack" {
            assert!(
                matches!(
                    error.downcast_ref::<ReceiptSendError>(),
                    Some(ReceiptSendError::Service(ServiceError::Unavailable))
                ),
                "{mode}"
            );
        } else {
            assert!(
                matches!(
                    error.downcast_ref::<ServiceError>(),
                    Some(ServiceError::Unavailable)
                ),
                "{mode}"
            );
        }
        assert!(state.stalls > 0);
        assert!(state.stalled_at.unwrap() < seeded + SIGNED_TTL);
        assert_eq!(case.session.calls, 0);
        assert!(matches!(
            case.reentry.err().unwrap().downcast_ref::<SessionError>(),
            Some(SessionError::StreamFenced)
        ));
        assert_eq!(case.io_before, case.io_after);
        assert_eq!(
            serde_json::to_value(case.counters).unwrap(),
            serde_json::to_value(case.after).unwrap()
        );
        assert_eq!(complete.len(), stall_frame - 1);
        assert_eq!(state.written.len(), partial_at + 4 + RETRY_PAYLOAD_PREFIX);
        assert_eq!(case.counters.frames_sent, (stall_frame - 1) as u64);
        assert_eq!(
            case.counters.framing_bytes_sent,
            (8 + stall_frame * 4) as u64
        );
        assert_eq!(
            case.counters.envelope_bytes_sent,
            complete.iter().map(|wire| wire.len() as u64).sum::<u64>() + 7
        );
        assert!(charged.queued_bytes > 0);
        if mode == "ack" {
            let pending = case.session.pending.as_ref().unwrap();
            assert_eq!(pending.send_invocations(), 1);
            assert_eq!(
                pending.reply_ack_state(),
                super::super::session::ReplyAckState::InvokedUncertain
            );
            assert_eq!(pending.reply_wire(), reply_wire.as_deref());
            assert_eq!(
                charged.queued_bytes,
                reply_wire.as_ref().unwrap().len() as u64
            );
            assert_eq!(case.counters.frames_received, 1);
        } else {
            assert!(case.session.receiver_incomplete);
            assert_eq!(charged.queued_bytes, declared as u64);
            assert_eq!(
                case.counters.frames_received,
                if mode == "cached-reply" { 2 } else { 1 }
            );
            if let Some(original_reply) = reply_wire {
                assert_eq!(declared, original_reply.len());
                assert_eq!(&state.written[partial_at + 4..], &original_reply[..7]);
            }
        }
    }
}

#[test]
fn explicit_controller_stops_at_eight_identical_wire_sends_and_silence_never_retries() {
    for requests_enabled in [true, false] {
        let pair = pair();
        let broker = broker_context().unwrap();
        let mut caller = Session::new(&pair.sender, &broker).unwrap();
        let request = Frame {
            kind: 5,
            transfer: [96; 32],
            seq: 0,
            body: Vec::new(),
        };
        let original = short_call(
            &mut caller,
            &request,
            if requests_enabled {
                Duration::from_secs(10)
            } else {
                SIGNED_TTL
            },
            true,
        );
        let (stream, state, observed) = scripted(&pair.sender, None);
        let (signal, requests) = bounded_requests();
        let cancel = CancellationToken::default();
        let token = cancel.clone();
        let watched = state.clone();
        let (done_tx, done) = mpsc::channel();
        let started = Instant::now();
        // The send-ceiling case observes eight distinct idle turns, each with
        // original signature verification. Its watchdog is separate from the
        // short signed expiry used to prove silence never retransmits.
        let deadline = started
            + if requests_enabled {
                Duration::from_secs(8)
            } else {
                Duration::from_millis(2700)
            };
        let worker = std::thread::spawn(move || {
            let wall = Instant::now();
            let result = (|| -> Result<()> {
                stream.handshake(HandshakeRole::Initiator, &token)?;
                caller
                    .call_with_reconciliation(&request, &stream, &token, requests)
                    .map(|_| ())
            })();
            let wall_us = u64::try_from(wall.elapsed().as_micros()).unwrap();
            let counters = stream.counters().unwrap();
            let io_before = {
                let state = watched.lock().unwrap();
                (state.reads, state.writes)
            };
            let (_, again) = bounded_requests();
            let reentry = caller
                .call_with_reconciliation(&request, &stream, &token, again)
                .map(|_| ());
            let after = stream.counters().unwrap();
            let io_after = {
                let state = watched.lock().unwrap();
                (state.reads, state.writes)
            };
            let _ = done_tx.send(());
            Case {
                session: caller,
                result,
                reentry,
                counters,
                after,
                io_before,
                io_after,
                wall_us,
            }
        });
        let worker = CaseWorker {
            cancel,
            worker: Some(worker),
        };
        for ordinal in 1..=if requests_enabled { 8 } else { 1 } {
            assert_eq!(
                observed
                    .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                    .unwrap(),
                ordinal
            );
            if requests_enabled {
                assert_eq!(signal.request_retry(), Ok(()));
            }
        }
        let completed = done
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .is_ok();
        let case = worker.finish();
        let state = state.lock().unwrap();
        let accepted = frames(&state.written);
        let count = if requests_enabled { 8 } else { 1 };
        let report = case.session.report();
        evidence(
            if requests_enabled {
                "eight-call-send-ceiling.json"
            } else {
                "silence-no-retry.json"
            },
            &json!({
                "evidence":"canonical-reconciliation-test/v1", "explicit_controller_enabled":requests_enabled,
                "scope":"Canonical scripted SocketIo qualification, not OS TCP/WAN or performance",
                "completed_before_watchdog":completed,"worker_wall_us":case.wall_us,
                "outcome":case.result.as_ref().err().map(ToString::to_string),
                "stream_counters":case.counters,"after_refused_reentry":case.after,
                "accepted_complete_frames":accepted.len(),"original_envelope_bytes":original.len(),
                "session":report,"io_before":case.io_before,"io_after":case.io_after
            }),
        );
        assert!(completed);
        assert!(case.result.is_err());
        assert_eq!(accepted.len(), count);
        for wire in accepted {
            assert_eq!(wire, original);
        }
        let pending = case.session.pending.as_ref().unwrap();
        assert_eq!(pending.send_invocations(), count as u8);
        assert_eq!(pending.events_received(), 0);
        assert_eq!(pending.encoded().unwrap(), original);
        assert_eq!(case.session.calls, 0);
        assert_eq!(case.counters.frames_sent, count as u64);
        assert_eq!(case.counters.frames_received, 0);
        assert_eq!(
            case.counters.envelope_bytes_sent,
            (count * original.len()) as u64
        );
        assert_eq!(case.counters.framing_bytes_sent, (8 + 4 * count) as u64);
        assert_eq!(case.io_before, case.io_after);
        assert_eq!(
            serde_json::to_value(case.counters).unwrap(),
            serde_json::to_value(case.after).unwrap()
        );
        assert!(case.reentry.is_err());
        assert!(
            report["recovery"]["idle_outcomes"].as_u64().unwrap()
                >= if requests_enabled { 8 } else { 2 }
        );
        assert_eq!(
            report["recovery"]["call_retry_invocations"]
                .as_u64()
                .unwrap(),
            (count - 1) as u64
        );
        assert_eq!(
            report["recovery"]["commands_consumed"].as_u64().unwrap(),
            if requests_enabled { 8 } else { 0 }
        );
        if requests_enabled {
            assert!(matches!(
                case.result.err().unwrap().downcast_ref::<SessionError>(),
                Some(SessionError::SendLimit)
            ));
        }
        assert_eq!(
            signal.request_retry(),
            Err(std::sync::mpsc::TrySendError::Disconnected(()))
        );
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
        stream.handshake(HandshakeRole::Initiator, &token).unwrap();
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
        let result = (|| -> Result<_> {
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
            seeded_at,
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
    let command_at = initial_sent.recv_timeout(PARENT_TIMEOUT).unwrap();
    assert_eq!(signal.request_retry(), Ok(()));
    let completed = done.recv_timeout(WATCHDOG).is_ok();
    let report = worker.finish();
    let seeded_at = report.seeded_at;
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
