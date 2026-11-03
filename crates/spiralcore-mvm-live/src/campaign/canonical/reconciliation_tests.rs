use super::*;
use crate::{
    campaign::reconciliation::bounded_requests,
    config::{Arm, Pair, Plan, Stage},
};
use mvm_core::{
    crypto::aead::Key,
    mailbox::{InMemoryReplayProtector, open_verified_envelope, verify_receipt},
};
use spiralcore_mvm_adapter::{live::LiveError, receipt::decode_destination_receipt};
use spiralcore_transfer::protocol::{Mode, Offer};
use std::{net::TcpStream, sync::mpsc, thread::JoinHandle};

#[path = "../../../tests/common/reconciliation_gate.rs"]
mod gate;

const WATCHDOG: Duration = Duration::from_secs(5);

struct CallerReport {
    session: Session,
    result: Result<Frame>,
    counters: StreamCounters,
}
struct ReceiverReport {
    session: Session,
    first: Result<Handled>,
    second: Option<Result<Handled>>,
    cache_empty: bool,
    counters: StreamCounters,
}
struct Workers {
    gate: Option<gate::Gate>,
    receiver_cancel: CancellationToken,
    release_caller: mpsc::SyncSender<()>,
    caller: Option<JoinHandle<CallerReport>>,
    receiver: Option<JoinHandle<ReceiverReport>>,
}
impl Workers {
    fn stop(&self) {
        self.receiver_cancel.cancel();
        let _ = self.release_caller.try_send(());
        if let Some(gate) = &self.gate {
            gate.stop();
        }
    }
    fn finish(mut self) -> (CallerReport, ReceiverReport, gate::Snapshot) {
        self.stop();
        let caller = self.caller.take().unwrap().join().expect("caller panicked");
        let receiver = self
            .receiver
            .take()
            .unwrap()
            .join()
            .expect("receiver panicked");
        let gate = self.gate.take().unwrap().finish().unwrap();
        (caller, receiver, gate)
    }
}
impl Drop for Workers {
    fn drop(&mut self) {
        self.stop();
        if let Some(worker) = self.caller.take() {
            let _ = worker.join();
        }
        if let Some(worker) = self.receiver.take() {
            let _ = worker.join();
        }
    }
}
fn status<T>(result: &Result<T>) -> String {
    result
        .as_ref()
        .map(|_| "ok".to_owned())
        .unwrap_or_else(|error| error.to_string())
}
fn stream(socket: TcpStream, bootstrap: &Bootstrap) -> StreamTransport<TcpStream> {
    StreamTransport::new(
        socket,
        bootstrap.outgoing.as_ref().unwrap().binding.claims.route_id,
        WATCHDOG,
    )
    .unwrap()
}
fn cache_empty(session: &mut Session, state: &gate::Shared) -> Result<bool> {
    let original = state
        .lock()
        .unwrap()
        .forward
        .captured
        .first()
        .cloned()
        .ok_or("fault gate has not received a Call")?;
    let broker = session.broker.clone();
    broker.with_time(|_, now| -> Result<_> {
        Ok(session
            .service
            .retry_reply_prepared(&session.incoming, &original, now)?
            .is_none())
    })
}

#[test]
fn explicit_retry_after_receiver_acked_reply_recovers_only_missing_call_receipt() {
    let pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 128,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    assert_eq!(
        pair.sender
            .outgoing
            .as_ref()
            .unwrap()
            .capability
            .grant
            .max_outstanding_calls,
        1
    );
    let caller_broker = broker_context().unwrap();
    let mut caller = Session::new(&pair.sender, &caller_broker).unwrap();
    let receiver_broker = broker_context().unwrap();
    let mut receiver = Session::new(&pair.receiver, &receiver_broker).unwrap();
    let control_material = pair
        .sender
        .service
        .as_ref()
        .unwrap()
        .incoming_control
        .clone();
    let destination_key = caller.peer_key;
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
    let request = Frame {
        kind: 1,
        transfer: [79; 32],
        seq: 0,
        body: Offer::cold(b"x", Mode::Raw).encode().unwrap(),
    };
    let expected_transfer = request.transfer;
    let (caller_socket, receiver_socket, gate) = gate::Gate::start_missing_first_receipt().unwrap();
    let state = gate.shared();
    let receiver_state = state.clone();
    let caller_stream = stream(caller_socket, &pair.sender);
    let receiver_stream = stream(receiver_socket, &pair.receiver);
    let (signal, requests) = bounded_requests();
    let (release_caller, release) = mpsc::sync_channel(1);
    let (caller_done_tx, caller_done) = mpsc::channel();
    let caller_worker = std::thread::spawn(move || {
        let cancel = CancellationToken::default();
        let result = (|| -> Result<Frame> {
            caller_stream.handshake(HandshakeRole::Initiator, &cancel)?;
            caller
                .call_with_reconciliation(&request, &caller_stream, &cancel, requests)
                .map(|(_, reply)| reply)
        })();
        let counters = caller_stream.counters().unwrap();
        let _ = caller_done_tx.send(result.is_ok());
        // Keep the healthy endpoint open until the owner cancels the receiver's
        // unfinished next preamble; caller completion is not a receiver EOF.
        let _ = release.recv_timeout(WATCHDOG);
        CallerReport {
            session: caller,
            result,
            counters,
        }
    });
    let (first_tx, first_done) = mpsc::channel();
    let (receiver_done_tx, receiver_done) = mpsc::channel();
    let receiver_cancel = CancellationToken::default();
    let cancel = receiver_cancel.clone();
    let receiver_worker = std::thread::spawn(move || {
        let first = (|| -> Result<Handled> {
            receiver_stream.handshake(HandshakeRole::Acceptor, &cancel)?;
            receiver.handle(expected_transfer, &mut object, &receiver_stream, &cancel)
        })();
        let empty = first.is_ok() && cache_empty(&mut receiver, &receiver_state).unwrap_or(false);
        let _ = first_tx.send((first.is_ok(), empty, receiver.calls));
        let second = if first.is_ok() && empty {
            Some(receiver.handle(expected_transfer, &mut object, &receiver_stream, &cancel))
        } else {
            None
        };
        let empty = cache_empty(&mut receiver, &receiver_state).unwrap_or(false);
        let counters = receiver_stream.counters().unwrap();
        let _ = receiver_done_tx.send(());
        ReceiverReport {
            session: receiver,
            first,
            second,
            cache_empty: empty,
            counters,
        }
    });
    let workers = Workers {
        gate: Some(gate),
        receiver_cancel,
        release_caller,
        caller: Some(caller_worker),
        receiver: Some(receiver_worker),
    };
    let first = first_done.recv_timeout(WATCHDOG).unwrap();
    assert_eq!(
        first,
        (true, true, 1),
        "receiver must actually complete and release its cache before retry"
    );
    // Ledger publication follows the real write; wait for its bounded causal
    // acknowledgment rather than racing the pump's final counter assignment.
    let ledger_deadline = Instant::now() + WATCHDOG;
    let before = loop {
        let snapshot = workers.gate.as_ref().unwrap().snapshot();
        if snapshot.forward.written.frames_sent == 2 && snapshot.reverse.written.frames_sent == 1 {
            break snapshot;
        }
        assert!(
            Instant::now() < ledger_deadline,
            "fault gate did not publish completed writes"
        );
        std::thread::yield_now();
    };
    assert_eq!(before.forward.read.frames_received, 2); // Call and actual Reply ACK.
    assert_eq!(before.reverse.read.frames_received, 2); // Receipt (withheld) and Reply.
    assert_eq!(before.reverse.written.frames_sent, 1);
    let charged = caller_broker
        .with_time::<_, ServiceError>(|budget, _| budget.snapshot())
        .unwrap();
    assert_eq!(
        charged.queued_bytes,
        before.reverse.captured[1].len() as u64
    );
    assert_eq!(charged.queued_messages, 1);
    assert!(charged.queued_bytes > 0);
    assert_eq!(signal.request_retry(), Ok(()));
    let caller_succeeded = caller_done.recv_timeout(WATCHDOG).unwrap();
    // The next preamble has no new application Call. Its owned cancellation is
    // test teardown, not a second Handled or an end-to-end recovery failure.
    workers.receiver_cancel.cancel();
    let _ = receiver_done.recv_timeout(WATCHDOG);
    let (caller, receiver, traffic) = workers.finish();
    let second = receiver
        .second
        .as_ref()
        .map(status)
        .unwrap_or_else(|| "not invoked".into());
    assert!(
        caller_succeeded,
        "caller: {}; receiver second invocation: {second}",
        status(&caller.result)
    );
    let reply = caller.result.unwrap();
    assert_eq!(
        reply.encode().unwrap(),
        receiver.first.as_ref().unwrap().reply.encode().unwrap()
    );
    assert_eq!(reply.kind, 101);
    assert_eq!(reply.transfer, expected_transfer);
    assert_eq!(caller.session.calls, 1);
    assert!(caller.session.pending.is_none());
    assert_eq!(receiver.session.calls, 1);
    assert!(receiver.second.as_ref().unwrap().is_err());
    assert!(receiver.cache_empty);
    assert!(matches!(
        receiver
            .second
            .unwrap()
            .err()
            .unwrap()
            .downcast_ref::<LiveError>(),
        Some(LiveError::Cancelled)
    ));

    let forward = &traffic.forward;
    let reverse = &traffic.reverse;
    assert!(forward.terminal.is_some() && reverse.terminal.is_some());
    assert_eq!(
        forward.captured[0], forward.captured[2],
        "retry must reuse every original wire byte"
    );
    assert_eq!(forward.read.frames_received, 3);
    assert_eq!(forward.written.frames_sent, 3);
    assert_eq!(reverse.read.frames_received, 3);
    assert_eq!(reverse.written.frames_sent, 2);
    let withheld = reverse.withheld.as_ref().unwrap();
    assert_eq!(withheld.ordinal, 1);
    assert_eq!(withheld.length, reverse.captured[0].len());
    assert_eq!(
        withheld.hash,
        spiralcore_transfer::hash(b"spiralcore.test.withheld.v1", &[&reverse.captured[0]])
    );
    assert_eq!(
        forward.read.envelope_bytes_received,
        forward
            .captured
            .iter()
            .map(|wire| wire.len() as u64)
            .sum::<u64>()
    );
    assert_eq!(
        forward.written.envelope_bytes_sent,
        forward.read.envelope_bytes_received
    );
    assert_eq!(
        reverse.read.envelope_bytes_received,
        reverse
            .captured
            .iter()
            .map(|wire| wire.len() as u64)
            .sum::<u64>()
    );
    assert_eq!(
        reverse.written.envelope_bytes_sent,
        reverse.read.envelope_bytes_received - withheld.length as u64
    );
    assert_eq!(forward.read.framing_bytes_received, 12);
    assert_eq!(forward.written.framing_bytes_sent, 12);
    assert_eq!(reverse.read.framing_bytes_received, 12);
    assert_eq!(reverse.written.framing_bytes_sent, 8);
    assert_eq!(caller.counters.frames_sent, forward.read.frames_received);
    assert_eq!(caller.counters.frames_received, reverse.written.frames_sent);
    assert_eq!(receiver.counters.frames_sent, reverse.read.frames_received);
    assert_eq!(
        receiver.counters.frames_received,
        forward.written.frames_sent
    );
    assert_eq!(
        caller.counters.envelope_bytes_sent,
        forward.read.envelope_bytes_received
    );
    assert_eq!(
        caller.counters.envelope_bytes_received,
        reverse.written.envelope_bytes_sent
    );
    assert_eq!(
        receiver.counters.envelope_bytes_sent,
        reverse.read.envelope_bytes_received
    );
    assert_eq!(
        receiver.counters.envelope_bytes_received,
        forward.written.envelope_bytes_sent
    );
    assert_eq!(
        caller.counters.framing_bytes_sent,
        forward.read.framing_bytes_received + traffic.caller_handshake.framing_bytes_received
    );
    assert_eq!(
        caller.counters.framing_bytes_received,
        reverse.written.framing_bytes_sent + traffic.caller_handshake.framing_bytes_sent
    );
    assert_eq!(
        receiver.counters.framing_bytes_sent,
        reverse.read.framing_bytes_received + traffic.receiver_handshake.framing_bytes_received
    );
    assert_eq!(
        receiver.counters.framing_bytes_received,
        forward.written.framing_bytes_sent + traffic.receiver_handshake.framing_bytes_sent
    );

    // Authenticate the actual returned receipt; cache absence alone is never
    // application-completion proof. The gate itself never opens ciphertext.
    let mut context = control_material.context().unwrap();
    context.now_unix_ms = crate::config::now_ms().unwrap();
    let opened = open_verified_envelope(
        &decode_envelope(&reverse.captured[2]).unwrap(),
        &control_material.capability,
        &context,
        &Key::from_bytes(control_material.encryption),
        &VerifyingKey::from_bytes(&control_material.verification).unwrap(),
        &VerifyingKey::from_bytes(&control_material.authority).unwrap(),
        &mut InMemoryReplayProtector::default(),
    )
    .unwrap();
    let duplicate = decode_destination_receipt(opened.payload()).unwrap();
    verify_receipt(&duplicate, &destination_key).unwrap();
    assert_eq!(duplicate.receipt.outcome, DeliveryOutcome::Duplicate);
}
