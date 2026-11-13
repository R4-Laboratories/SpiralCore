use super::reconciliation_tests::gate;
use super::*;
use crate::config::{Arm, Pair, Plan, Stage};
use std::{
    net::{Shutdown, TcpStream},
    sync::mpsc,
    thread::JoinHandle,
};

struct PeerReport {
    session: Session,
    result: Result<Handled>,
    counters: StreamCounters,
    reentry: Result<Handled>,
    after_reentry: StreamCounters,
}
struct Completed<S: SocketIo> {
    pair: Pair,
    caller: Session,
    stream: StreamTransport<S>,
    signal: ReplyReplaySignal,
    cancel: CancellationToken,
    gate: Option<gate::Gate>,
    peer: Option<JoinHandle<PeerReport>>,
    done: mpsc::Receiver<()>,
    close: TcpStream,
}
impl<S: SocketIo> Completed<S> {
    fn peer_report(&mut self) -> PeerReport {
        self.done
            .recv_timeout(Duration::from_secs(5))
            .expect("receiver did not finish bounded invocation");
        self.peer.take().unwrap().join().unwrap()
    }
}
impl<S: SocketIo> Drop for Completed<S> {
    fn drop(&mut self) {
        self.cancel.cancel();
        let _ = self.close.shutdown(Shutdown::Both);
        if let Some(gate) = &self.gate {
            gate.stop();
        }
        if let Some(peer) = self.peer.take() {
            let _ = peer.join();
        }
    }
}
fn completed() -> Completed<TcpStream> {
    completed_with_socket(|socket| socket)
}
fn completed_with_socket<S: SocketIo + 'static>(wrap: impl FnOnce(TcpStream) -> S) -> Completed<S> {
    let pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 4096,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    let mut caller = Session::new(&pair.sender, &broker_context().unwrap()).unwrap();
    let mut receiver = Session::new(&pair.receiver, &broker_context().unwrap()).unwrap();
    // This fixture keeps the original TCP relay alive for the complete series
    // of authorized replays. Individual transport/receipt deadlines stay bounded.
    let [socket, a] = gate::tcp_pair().unwrap();
    let [b, peer_socket] = gate::tcp_pair().unwrap();
    let gate = gate::Gate::between(a, b, Some(2), None, Duration::from_secs(30)).unwrap();
    let close = peer_socket.try_clone().unwrap();
    let stream = StreamTransport::new(
        wrap(socket),
        pair.sender
            .outgoing
            .as_ref()
            .unwrap()
            .binding
            .claims
            .route_id,
        Duration::from_secs(5),
    )
    .unwrap();
    let peer_stream = StreamTransport::new(
        peer_socket,
        pair.receiver
            .outgoing
            .as_ref()
            .unwrap()
            .binding
            .claims
            .route_id,
        Duration::from_secs(5),
    )
    .unwrap();
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
    let cancel = CancellationToken::default();
    let peer_cancel = cancel.clone();
    let (signal, requests) = super::super::reconciliation::bounded_reply_replays();
    let (done_tx, done) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        let _store = store;
        let result = (|| {
            peer_stream.handshake(HandshakeRole::Acceptor, &peer_cancel)?;
            receiver.handle_with_reply_replay(
                [0; 32],
                &mut object,
                &peer_stream,
                &peer_cancel,
                requests,
            )
        })();
        let counters = peer_stream.counters().unwrap();
        let reentry = receiver.handle([0; 32], &mut object, &peer_stream, &peer_cancel);
        let after_reentry = peer_stream.counters().unwrap();
        let _ = done_tx.send(());
        PeerReport {
            session: receiver,
            result,
            counters,
            reentry,
            after_reentry,
        }
    });
    stream.handshake(HandshakeRole::Initiator, &cancel).unwrap();
    let (_, reply) = caller
        .call_with_completed_tail(
            &Frame {
                kind: 5,
                transfer: [0; 32],
                seq: 0,
                body: vec![],
            },
            &stream,
            &cancel,
            None,
        )
        .unwrap();
    assert_eq!(reply.kind, 127);
    assert!(caller.pending.is_none());
    assert_eq!(caller.calls, 1);
    assert!(caller.completed_tail.is_some());
    Completed {
        pair,
        caller,
        stream,
        signal,
        cancel,
        gate: Some(gate),
        peer: Some(peer),
        done,
        close,
    }
}

#[test]
fn expired_or_changed_authority_tail_fences_before_io_and_retains_exact_wire() {
    for mode in ["instant", "generation", "event-limit"] {
        let mut fixture = completed();
        let wire = fixture
            .caller
            .completed_tail
            .as_ref()
            .unwrap()
            .lease
            .wire()
            .to_vec();
        let before = fixture.stream.counters().unwrap();
        match mode {
            "instant" => fixture.caller.completed_tail.as_mut().unwrap().deadline = Instant::now(),
            "generation" => {
                fixture
                    .caller
                    .incoming
                    .replace_policy(
                        fixture
                            .pair
                            .sender
                            .incoming
                            .as_ref()
                            .unwrap()
                            .context()
                            .unwrap(),
                    )
                    .unwrap();
            }
            "event-limit" => fixture.caller.completed_tail.as_mut().unwrap().events = 16,
            _ => unreachable!(),
        }
        let error = fixture
            .caller
            .drive_completed_tail(&fixture.stream, &fixture.cancel)
            .unwrap_err();
        match mode {
            "instant" => assert_eq!(
                error.downcast_ref::<ServiceError>(),
                Some(&ServiceError::Expired)
            ),
            "generation" => assert_eq!(
                error.downcast_ref::<ServiceError>(),
                Some(&ServiceError::Unauthorized)
            ),
            _ => assert_eq!(
                error.downcast_ref::<SessionError>(),
                Some(&SessionError::EventLimit)
            ),
        }
        assert_eq!(
            serde_json::to_value(fixture.stream.counters().unwrap()).unwrap(),
            serde_json::to_value(before).unwrap()
        );
        assert!(fixture.caller.caller_fenced);
        assert_eq!(
            fixture.caller.completed_tail.as_ref().unwrap().lease.wire(),
            wire
        );
        let result = fixture.caller.call_with_completed_tail(
            &Frame {
                kind: 5,
                transfer: [1; 32],
                seq: 0,
                body: vec![],
            },
            &fixture.stream,
            &fixture.cancel,
            None,
        );
        assert_eq!(
            result
                .expect_err("Call unexpectedly succeeded")
                .downcast_ref::<SessionError>(),
            Some(&SessionError::StreamFenced)
        );
        assert_eq!(
            serde_json::to_value(fixture.stream.counters().unwrap()).unwrap(),
            serde_json::to_value(before).unwrap()
        );
        assert_eq!(fixture.caller.calls, 1);
    }
}

#[test]
fn exhausted_completed_tail_refuses_new_identity_before_any_write_or_allocation() {
    let mut fixture = completed();
    fixture.caller.completed_tail.as_mut().unwrap().events = 16;
    let before = fixture.stream.counters().unwrap();
    let error = fixture
        .caller
        .call_with_completed_tail(
            &Frame {
                kind: 5,
                transfer: [1; 32],
                seq: 0,
                body: vec![],
            },
            &fixture.stream,
            &fixture.cancel,
            None,
        )
        .expect_err("exhausted tail admitted another identity");
    assert_eq!(
        error.downcast_ref::<SessionError>(),
        Some(&SessionError::EventLimit)
    );
    assert!(
        fixture.caller.pending.is_none(),
        "no new identity may be allocated at the old tail event ceiling"
    );
    assert_eq!(fixture.caller.calls, 1);
    assert_eq!(
        serde_json::to_value(fixture.stream.counters().unwrap()).unwrap(),
        serde_json::to_value(before).unwrap()
    );
}

#[test]
fn explicit_receiver_replays_share_eight_send_bound_and_preserve_cache_on_refusal() {
    let mut fixture = completed();
    let original = fixture
        .caller
        .completed_tail
        .as_ref()
        .unwrap()
        .lease
        .wire()
        .to_vec();
    for _ in 0..7 {
        fixture.signal.request_replay().unwrap();
        let replay = fixture
            .stream
            .receive(&fixture.cancel, Instant::now() + Duration::from_secs(5))
            .unwrap();
        assert_eq!(
            replay, original,
            "all replay writes retain exact canonical identity"
        );
        // Intentionally withhold ACK, leaving one active receiver cache. The
        // reader is fixture-owned here, not a claim of a second product reader.
    }
    fixture.signal.request_replay().unwrap();
    let mut report = fixture.peer_report();
    assert_eq!(
        report
            .result
            .err()
            .expect("Call unexpectedly succeeded")
            .downcast_ref::<SessionError>(),
        Some(&SessionError::SendLimit)
    );
    assert_eq!(
        report
            .reentry
            .err()
            .expect("fenced reentry succeeded")
            .downcast_ref::<SessionError>(),
        Some(&SessionError::StreamFenced)
    );
    assert_eq!(
        serde_json::to_value(report.counters).unwrap(),
        serde_json::to_value(report.after_reentry).unwrap()
    );
    assert_eq!(
        report.counters.frames_sent, 9,
        "one Call receipt plus eight total Reply writes"
    );
    assert_eq!(report.session.calls, 0);
    assert_eq!(
        report.session.report()["recovery"]["reply_replay_commands_consumed"],
        8
    );
    assert_eq!(
        report.session.report()["recovery"]["cached_reply_retry_invocations"],
        7
    );
    assert!(report.session.receiver_incomplete);
    let state = fixture.gate.as_ref().unwrap().shared();
    let call = state.lock().unwrap().forward.captured[0].clone();
    let broker = report.session.broker.clone();
    broker
        .with_time(|_, now| -> Result<()> {
            let cached = report
                .session
                .service
                .retry_reply_prepared(&report.session.incoming, &call, now)?
                .ok_or(ServiceError::State)?;
            assert_eq!(encode_envelope(cached.envelope())?, original);
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        fixture.signal.request_replay(),
        Err(mpsc::TrySendError::Disconnected(()))
    ));
}

#[derive(Default)]
struct AckBackpressure {
    armed: bool,
    accepted: usize,
    waits: usize,
    write_timeout: Duration,
    shutdowns: usize,
}
struct PartialAck {
    socket: TcpStream,
    state: std::sync::Arc<std::sync::Mutex<AckBackpressure>>,
}
impl Read for PartialAck {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.socket.read(bytes)
    }
}
impl Write for PartialAck {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut state = self.state.lock().unwrap();
        if !state.armed {
            return self.socket.write(bytes);
        }
        let count = bytes.len().min(7usize.saturating_sub(state.accepted));
        if count > 0 {
            let count = self.socket.write(&bytes[..count])?;
            state.accepted += count;
            return Ok(count);
        }
        state.waits += 1;
        let wait = state.write_timeout;
        drop(state);
        std::thread::sleep(wait);
        Err(io::ErrorKind::WouldBlock.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.socket.flush()
    }
}
impl SocketIo for PartialAck {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        SocketIo::set_read_timeout(&self.socket, timeout)
    }
    fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.state.lock().unwrap().write_timeout = timeout.ok_or(io::ErrorKind::InvalidInput)?;
        SocketIo::set_write_timeout(&self.socket, timeout)
    }
    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.state.lock().unwrap().shutdowns += 1;
        self.socket.shutdown(how)
    }
}

#[test]
fn partial_tail_ack_uses_carried_absolute_cap_and_fences_without_refunding_owned_wire() {
    let state = std::sync::Arc::new(std::sync::Mutex::new(AckBackpressure::default()));
    let observed = state.clone();
    let mut socket_probe = None;
    let mut fixture = completed_with_socket(|socket| {
        socket_probe = Some(socket.try_clone().unwrap());
        PartialAck { socket, state }
    });
    let socket_probe = socket_probe.unwrap();
    let original = fixture
        .caller
        .completed_tail
        .as_ref()
        .unwrap()
        .lease
        .wire()
        .to_vec();
    let before = fixture.stream.counters().unwrap();
    // Narrow the retained absolute cap for a fast deterministic I/O conformance
    // test. Signed lifetime/owner clocks remain the original admitted values.
    // Ask for the original replay first and observe it actually arriving before
    // starting the injected ACK-write window. A bounded peek consumes no
    // application bytes and does not advance the protocol.
    fixture.signal.request_replay().unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    let mut pending = [0u8; 1];
    socket_probe
        .set_read_timeout(Some(until.saturating_duration_since(Instant::now())))
        .unwrap();
    assert_eq!(socket_probe.peek(&mut pending).unwrap(), 1);
    let cap = Instant::now() + Duration::from_millis(600);
    fixture.caller.completed_tail.as_mut().unwrap().deadline = cap;
    observed.lock().unwrap().armed = true;
    let error = fixture
        .caller
        .drive_completed_tail(&fixture.stream, &fixture.cancel)
        .unwrap_err();
    assert!(
        error
            .downcast_ref::<spiralcore_mvm_adapter::receipt::ReceiptSendError>()
            .is_some(),
        "{error}"
    );
    assert!(Instant::now() >= cap);
    assert!(Instant::now() < cap + Duration::from_secs(1));
    let state = observed.lock().unwrap();
    assert_eq!(
        state.accepted, 7,
        "one framing prefix and three ciphertext bytes"
    );
    assert!(state.waits > 0);
    assert!(state.shutdowns > 0);
    drop(state);
    let after = fixture.stream.counters().unwrap();
    assert_eq!(after.frames_sent, before.frames_sent);
    assert_eq!(after.framing_bytes_sent, before.framing_bytes_sent + 4);
    assert_eq!(after.envelope_bytes_sent, before.envelope_bytes_sent + 3);
    let tail = fixture.caller.completed_tail.as_ref().unwrap();
    assert_eq!(tail.lease.wire(), original);
    assert_eq!(tail.ack, ReplyAckState::InvokedUncertain);
    assert!(fixture.caller.caller_fenced);
    assert_eq!(
        fixture
            .caller
            .drive_completed_tail(&fixture.stream, &fixture.cancel)
            .unwrap_err()
            .downcast_ref::<SessionError>(),
        Some(&SessionError::StreamFenced)
    );
    assert_eq!(
        serde_json::to_value(fixture.stream.counters().unwrap()).unwrap(),
        serde_json::to_value(after).unwrap()
    );
}
