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
pub(super) mod gate;

const WATCHDOG: Duration = Duration::from_secs(5);

struct CallerReport {
    session: Session,
    result: Result<Frame>,
    counters: StreamCounters,
    lifecycle_cpu_us: u64,
    lifecycle_wall_us: u64,
}
struct ReceiverReport {
    session: Session,
    first: Result<Handled>,
    second: Option<Result<Handled>>,
    cache_empty: bool,
    counters: StreamCounters,
    lifecycle_cpu_us: u64,
    lifecycle_wall_us: u64,
}
struct Workers {
    gate: Option<gate::Gate>,
    receiver_cancel: CancellationToken,
    release_caller: mpsc::SyncSender<()>,
    caller: Option<JoinHandle<CallerReport>>,
    receiver: Option<JoinHandle<ReceiverReport>>,
    relay: Option<Relay>,
}

pub(super) struct Relay {
    shutdown: Box<dyn Fn()>,
    worker: Option<JoinHandle<Result<Value>>>,
    budget: crate::host_budget::HostTransportBudget,
}
impl Relay {
    fn stop(&self) {
        (self.shutdown)();
    }
    pub(super) fn finish(mut self) -> Value {
        let report = self
            .worker
            .take()
            .unwrap()
            .join()
            .expect("actual relay panicked");
        let budget = self.budget.snapshot().unwrap();
        assert_eq!(budget.usage.links, 0);
        assert_eq!(budget.usage.frame_slots, 0);
        assert_eq!(budget.usage.serialized_buffer_bytes, 0);
        assert_eq!(budget.high_water.links, 2);
        assert_eq!(budget.high_water.frame_slots, 2);
        assert_eq!(budget.high_water.serialized_buffer_bytes, 524_288);
        json!({"report":report.unwrap(), "budget":budget})
    }
}
impl Drop for Relay {
    fn drop(&mut self) {
        self.stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(any(
    all(target_os = "linux", feature = "linux-host"),
    all(target_os = "macos", target_arch = "aarch64")
))]
pub(super) fn relayed_transport(
    pair: &Pair,
    forward_withheld: Option<u64>,
    reverse_withheld: Option<u64>,
) -> (TcpStream, std::os::unix::net::UnixStream, gate::Gate, Relay) {
    use crate::orchestrator::{RouteAdmission, relay_for_test, transport_budget};
    use std::os::unix::net::UnixStream;
    let [caller, gate_a] = gate::tcp_pair().unwrap();
    let (gate_b, relay_a) = UnixStream::pair().unwrap();
    let (relay_b, receiver) = UnixStream::pair().unwrap();
    let close_a = relay_a.try_clone().unwrap();
    let close_b = relay_b.try_clone().unwrap();
    let forward = RouteAdmission::with_control(
        pair.sender.outgoing.as_ref().unwrap(),
        &pair.sender.service.as_ref().unwrap().outgoing_control,
    )
    .unwrap();
    let reverse = RouteAdmission::with_control(
        pair.receiver.outgoing.as_ref().unwrap(),
        &pair.receiver.service.as_ref().unwrap().outgoing_control,
    )
    .unwrap();
    let budget = transport_budget().unwrap();
    let owner = budget.clone();
    let worker = std::thread::spawn(move || {
        let scope = owner.scope(0)?;
        let links = [owner.reserve_link(&scope)?, owner.reserve_link(&scope)?];
        relay_for_test(relay_a, relay_b, forward, reverse, [&links[0], &links[1]])
    });
    let relay = Relay {
        shutdown: Box::new(move || {
            for socket in [&close_a, &close_b] {
                if socket.shutdown(std::net::Shutdown::Both).is_err() {
                    let _ = socket.shutdown(std::net::Shutdown::Write);
                    let _ = socket.shutdown(std::net::Shutdown::Read);
                }
            }
        }),
        worker: Some(worker),
        budget,
    };
    let gate = gate::Gate::between(gate_a, gate_b, forward_withheld, reverse_withheld).unwrap();
    (caller, receiver, gate, relay)
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
        let mut gate = self.gate.take().unwrap().finish().unwrap();
        if let Some(relay) = self.relay.take() {
            gate.relay = Some(relay.finish());
        }
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
fn stream<S: SocketIo + 'static>(socket: S, bootstrap: &Bootstrap) -> StreamTransport<S> {
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

fn assert_relay_traffic(traffic: &gate::Snapshot, receiver: &StreamCounters) {
    let Some(relay) = &traffic.relay else {
        return;
    };
    let a = &relay["report"]["a"];
    let b = &relay["report"]["b"];
    for (field, expected) in [
        ("frames_received", traffic.forward.written.frames_sent),
        ("frames_sent", traffic.reverse.read.frames_received),
        (
            "envelope_bytes_received",
            traffic.forward.written.envelope_bytes_sent,
        ),
        (
            "envelope_bytes_sent",
            traffic.reverse.read.envelope_bytes_received,
        ),
        (
            "framing_bytes_received",
            traffic.forward.written.framing_bytes_sent
                + traffic.receiver_handshake.framing_bytes_sent,
        ),
        (
            "framing_bytes_sent",
            traffic.reverse.read.framing_bytes_received
                + traffic.receiver_handshake.framing_bytes_received,
        ),
    ] {
        assert_eq!(a[field].as_u64().unwrap(), expected, "relay-a {field}");
    }
    for (field, expected) in [
        ("frames_received", receiver.frames_sent),
        ("frames_sent", receiver.frames_received),
        ("envelope_bytes_received", receiver.envelope_bytes_sent),
        ("envelope_bytes_sent", receiver.envelope_bytes_received),
        ("framing_bytes_received", receiver.framing_bytes_sent),
        ("framing_bytes_sent", receiver.framing_bytes_received),
    ] {
        assert_eq!(b[field].as_u64().unwrap(), expected, "relay-b {field}");
    }
}

fn write_evidence(case: &str, value: &Value) {
    let Some(directory) = std::env::var_os("SPIRALCORE_RECONCILIATION_EVIDENCE_DIR") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let metadata = std::fs::symlink_metadata(&directory).unwrap();
    assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
    // Root supplies this owned directory. Never overwrite an earlier result or
    // persist payloads, envelope wires, keys, or capability/bootstrap material.
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(format!("{case}.json")))
        .unwrap();
    serde_json::to_writer_pretty(file, value).unwrap();
}

#[test]
fn explicit_retry_recovers_missing_reply_direct() {
    exercise_active_receiver_fault(false);
}

#[test]
fn explicit_retry_recovers_missing_call_receipt_and_reply_ack_direct() {
    exercise_active_receiver_fault(true);
}

#[test]
#[cfg(any(
    all(target_os = "linux", feature = "linux-host"),
    all(target_os = "macos", target_arch = "aarch64")
))]
fn explicit_retry_recovers_missing_reply_through_actual_duplex_relay() {
    let pair = fault_pair();
    let (a, b, gate, relay) = relayed_transport(&pair, None, Some(2));
    exercise_active_receiver_transport(
        pair,
        (a, b, gate, Some(relay), HandshakeRole::Initiator),
        false,
    );
}

#[test]
#[cfg(any(
    all(target_os = "linux", feature = "linux-host"),
    all(target_os = "macos", target_arch = "aarch64")
))]
fn explicit_retry_recovers_missing_receipt_and_ack_through_actual_duplex_relay() {
    let pair = fault_pair();
    let (a, b, gate, relay) = relayed_transport(&pair, Some(2), Some(1));
    exercise_active_receiver_transport(
        pair,
        (a, b, gate, Some(relay), HandshakeRole::Initiator),
        true,
    );
}

#[test]
#[cfg(any(
    all(target_os = "linux", feature = "linux-host"),
    all(target_os = "macos", target_arch = "aarch64")
))]
fn explicit_retry_recovers_missing_call_receipt_through_actual_duplex_relay() {
    let pair = fault_pair();
    let (a, b, gate, relay) = relayed_transport(&pair, None, Some(1));
    missing_call_receipt_transport(pair, (a, b, gate, Some(relay), HandshakeRole::Initiator));
}

fn fault_pair() -> Pair {
    Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 128,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap()
}

type Transport<S> = (TcpStream, S, gate::Gate, Option<Relay>, HandshakeRole);

fn exercise_active_receiver_fault(withhold_receipt_and_ack: bool) {
    let pair = fault_pair();
    let (a, b, gate) = gate::Gate::start_withheld(
        withhold_receipt_and_ack.then_some(2),
        Some(if withhold_receipt_and_ack { 1 } else { 2 }),
    )
    .unwrap();
    exercise_active_receiver_transport(
        pair,
        (a, b, gate, None, HandshakeRole::Acceptor),
        withhold_receipt_and_ack,
    );
}

fn exercise_active_receiver_transport<S: SocketIo + 'static>(
    pair: Pair,
    transport: Transport<S>,
    withhold_receipt_and_ack: bool,
) {
    let (a, b, fault_gate, relay, receiver_role) = transport;
    let caller_broker = broker_context().unwrap();
    let mut caller = Session::new(&pair.sender, &caller_broker).unwrap();
    let control_material = pair
        .sender
        .service
        .as_ref()
        .unwrap()
        .incoming_control
        .clone();
    let destination_key = caller.peer_key;
    let mut receiver = Session::new(&pair.receiver, &broker_context().unwrap()).unwrap();
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
        transfer: [81; 32],
        seq: 0,
        body: Offer::cold(b"x", Mode::Raw).encode().unwrap(),
    };
    let transfer = request.transfer;
    let state = fault_gate.shared();
    let receiver_state = state.clone();
    let a = stream(a, &pair.sender);
    let b = stream(b, &pair.receiver);
    let (signal, requests) = bounded_requests();
    let (release_caller, release) = mpsc::sync_channel(1);
    let (caller_tx, caller_done) = mpsc::channel();
    let caller_worker = std::thread::spawn(move || {
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        let cancel = CancellationToken::default();
        let result = (|| -> Result<Frame> {
            a.handshake(HandshakeRole::Initiator, &cancel)?;
            caller
                .call_with_reconciliation(&request, &a, &cancel, requests)
                .map(|(_, reply)| reply)
        })();
        let counters = a.counters().unwrap();
        let lifecycle_cpu_us = u64::try_from(cpu.elapsed().as_micros()).unwrap();
        let lifecycle_wall_us = u64::try_from(wall.elapsed().as_micros()).unwrap();
        let _ = caller_tx.send(result.is_ok());
        let _ = release.recv_timeout(WATCHDOG);
        CallerReport {
            session: caller,
            result,
            counters,
            lifecycle_cpu_us,
            lifecycle_wall_us,
        }
    });
    let (receiver_tx, receiver_done) = mpsc::channel();
    let receiver_cancel = CancellationToken::default();
    let cancel = receiver_cancel.clone();
    let receiver_worker = std::thread::spawn(move || {
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        let first = (|| -> Result<Handled> {
            b.handshake(receiver_role, &cancel)?;
            receiver.handle(transfer, &mut object, &b, &cancel)
        })();
        let empty = cache_empty(&mut receiver, &receiver_state).unwrap_or(false);
        let counters = b.counters().unwrap();
        let _ = receiver_tx.send(first.is_ok() && empty);
        ReceiverReport {
            session: receiver,
            first,
            second: None,
            cache_empty: empty,
            counters,
            lifecycle_cpu_us: u64::try_from(cpu.elapsed().as_micros()).unwrap(),
            lifecycle_wall_us: u64::try_from(wall.elapsed().as_micros()).unwrap(),
        }
    });
    let workers = Workers {
        gate: Some(fault_gate),
        receiver_cancel,
        release_caller,
        caller: Some(caller_worker),
        receiver: Some(receiver_worker),
        relay,
    };
    // Request recovery only after the registered complete-frame fault occurred.
    // Silence alone never authorizes a resend.
    let end = Instant::now() + WATCHDOG;
    loop {
        let snapshot = state.lock().unwrap().clone();
        let fault_seen = snapshot.reverse.withheld.is_some()
            && (!withhold_receipt_and_ack || snapshot.forward.withheld.is_some());
        if fault_seen {
            break;
        }
        assert!(
            Instant::now() < end,
            "registered frame fault never occurred"
        );
        std::thread::yield_now();
    }
    assert_eq!(signal.request_retry(), Ok(()));
    let caller_ok = caller_done.recv_timeout(WATCHDOG).unwrap_or(false);
    let receiver_ok = receiver_done.recv_timeout(WATCHDOG).unwrap_or(false);
    if caller_ok && receiver_ok {
        let deadline = Instant::now() + WATCHDOG;
        loop {
            let snapshot = state.lock().unwrap().clone();
            if snapshot.reverse.read.frames_received == 4
                && snapshot.reverse.written.frames_sent == 3
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "gate did not publish the closing control write"
            );
            std::thread::yield_now();
        }
    }
    let (caller, receiver, traffic) = workers.finish();
    let relayed = traffic.relay.is_some();
    let case = match (withhold_receipt_and_ack, relayed) {
        (false, false) => "missing-reply-direct",
        (false, true) => "missing-reply-relayed",
        (true, false) => "missing-receipt-and-ack-direct",
        (true, true) => "missing-receipt-and-ack-relayed",
    };
    write_evidence(
        case,
        &json!({
            "evidence":"canonical-reconciliation-test/v1", "case":case,
            "scope":"Unix TCP/UDS canonical component fixture; not WAN or native MVM",
            "caller_result":status(&caller.result), "receiver_result":status(&receiver.first),
            "cache_empty":receiver.cache_empty,
            "caller_session":caller.session.report(), "receiver_session":receiver.session.report(),
            "actual_traffic":{"caller":caller.counters,"receiver":receiver.counters,
                "gate_forward_incoming":traffic.forward.read,"gate_forward_outgoing":traffic.forward.written,
                "gate_reverse_incoming":traffic.reverse.read,"gate_reverse_outgoing":traffic.reverse.written,
                "gate_caller_handshake":traffic.caller_handshake,"gate_far_handshake":traffic.receiver_handshake,
                "relay":traffic.relay,
                "buffered_closing_control_frames":u64::from(!withhold_receipt_and_ack && caller_ok)},
            "withheld":{
                "forward":traffic.forward.withheld.as_ref().map(|w| json!({"ordinal":w.ordinal,"bytes":w.length,"hash":spiralcore_transfer::hex(&w.hash)})),
                "reverse":traffic.reverse.withheld.as_ref().map(|w| json!({"ordinal":w.ordinal,"bytes":w.length,"hash":spiralcore_transfer::hex(&w.hash)})),
                "hash_domain":"spiralcore.test.withheld.v1"},
            "lifecycle":{"cpu_domain":"ProcessTime process CPU during overlapping worker intervals, not exclusive thread CPU",
                "caller":{"cpu_us":caller.lifecycle_cpu_us,"wall_us":caller.lifecycle_wall_us},
                "receiver":{"cpu_us":receiver.lifecycle_cpu_us,"wall_us":receiver.lifecycle_wall_us}},
            "accounting":"Lifecycle, cumulative receiver processing and per-Handled intervals overlap; do not sum or infer energy/performance"
        }),
    );
    assert!(
        caller_ok && receiver_ok,
        "both peers must complete/cache release: caller={}, receiver={}, cache_empty={}",
        status(&caller.result),
        status(&receiver.first),
        receiver.cache_empty
    );
    assert_eq!(
        caller.result.as_ref().unwrap(),
        &receiver.first.as_ref().unwrap().reply
    );
    assert_eq!(caller.session.calls, 1);
    assert_eq!(receiver.session.calls, 1, "object executes once");
    assert_eq!(
        traffic.forward.captured[0],
        traffic.forward.captured[if withhold_receipt_and_ack { 2 } else { 1 }]
    );
    assert_eq!(
        traffic.reverse.captured[1], traffic.reverse.captured[2],
        "cached Reply must retain every original byte"
    );
    assert_eq!(
        caller.counters.frames_sent,
        traffic.forward.read.frames_received
    );
    assert_eq!(
        receiver.counters.frames_received,
        traffic.forward.written.frames_sent
    );
    assert_eq!(
        receiver.counters.frames_sent,
        traffic.reverse.read.frames_received
    );
    let buffered = u64::from(!withhold_receipt_and_ack);
    assert_eq!(
        caller.counters.frames_received + buffered,
        traffic.reverse.written.frames_sent,
        "cached-Reply ACK can complete a caller already holding its Call receipt; trailing Duplicate is buffered"
    );
    assert_eq!(traffic.reverse.read.frames_received, 4);
    assert_eq!(traffic.reverse.written.frames_sent, 3);
    assert_eq!(
        traffic.forward.read.frames_received,
        if withhold_receipt_and_ack { 4 } else { 3 }
    );
    assert_eq!(traffic.forward.written.frames_sent, 3);
    assert_eq!(
        caller.counters.envelope_bytes_received
            + if buffered == 1 {
                traffic.reverse.captured[3].len() as u64
            } else {
                0
            },
        traffic.reverse.written.envelope_bytes_sent
    );
    assert_eq!(
        caller.session.report()["recovery"]["call_retry_invocations"],
        1
    );
    let report = receiver.session.report();
    assert_eq!(report["recovery"]["cached_reply_retry_invocations"], 1);
    assert_eq!(report["recovery"]["reply_send_invocations"], 2);
    assert_eq!(report["recovery"]["receiver_failed_invocations"], 0);
    let mut context = control_material.context().unwrap();
    context.now_unix_ms = crate::config::now_ms().unwrap();
    let opened = open_verified_envelope(
        &decode_envelope(&traffic.reverse.captured[3]).unwrap(),
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
    assert_relay_traffic(&traffic, &receiver.counters);
}

#[test]
fn explicit_retry_after_receiver_acked_reply_recovers_only_missing_call_receipt() {
    let pair = fault_pair();
    let (a, b, gate) = gate::Gate::start_missing_first_receipt().unwrap();
    missing_call_receipt_transport(pair, (a, b, gate, None, HandshakeRole::Acceptor));
}

fn missing_call_receipt_transport<S: SocketIo + 'static>(pair: Pair, transport: Transport<S>) {
    let (caller_socket, receiver_socket, gate, relay, receiver_role) = transport;
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
    let state = gate.shared();
    let receiver_state = state.clone();
    let caller_stream = stream(caller_socket, &pair.sender);
    let receiver_stream = stream(receiver_socket, &pair.receiver);
    let (signal, requests) = bounded_requests();
    let (release_caller, release) = mpsc::sync_channel(1);
    let (caller_done_tx, caller_done) = mpsc::channel();
    let caller_worker = std::thread::spawn(move || {
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        let cancel = CancellationToken::default();
        let result = (|| -> Result<Frame> {
            caller_stream.handshake(HandshakeRole::Initiator, &cancel)?;
            caller
                .call_with_reconciliation(&request, &caller_stream, &cancel, requests)
                .map(|(_, reply)| reply)
        })();
        let counters = caller_stream.counters().unwrap();
        let lifecycle_cpu_us = u64::try_from(cpu.elapsed().as_micros()).unwrap();
        let lifecycle_wall_us = u64::try_from(wall.elapsed().as_micros()).unwrap();
        let _ = caller_done_tx.send(result.is_ok());
        // Keep the healthy endpoint open until the owner cancels the receiver's
        // unfinished next preamble; caller completion is not a receiver EOF.
        let _ = release.recv_timeout(WATCHDOG);
        CallerReport {
            session: caller,
            result,
            counters,
            lifecycle_cpu_us,
            lifecycle_wall_us,
        }
    });
    let (first_tx, first_done) = mpsc::channel();
    let (receiver_done_tx, receiver_done) = mpsc::channel();
    let receiver_cancel = CancellationToken::default();
    let cancel = receiver_cancel.clone();
    let receiver_worker = std::thread::spawn(move || {
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        let first = (|| -> Result<Handled> {
            receiver_stream.handshake(receiver_role, &cancel)?;
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
        let lifecycle_cpu_us = u64::try_from(cpu.elapsed().as_micros()).unwrap();
        let lifecycle_wall_us = u64::try_from(wall.elapsed().as_micros()).unwrap();
        let _ = receiver_done_tx.send(());
        ReceiverReport {
            session: receiver,
            first,
            second,
            cache_empty: empty,
            counters,
            lifecycle_cpu_us,
            lifecycle_wall_us,
        }
    });
    let workers = Workers {
        gate: Some(gate),
        receiver_cancel,
        release_caller,
        caller: Some(caller_worker),
        receiver: Some(receiver_worker),
        relay,
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

    let caller_report = caller.session.report();
    let receiver_report = receiver.session.report();
    let case = if traffic.relay.is_some() {
        "missing-call-receipt-relayed"
    } else {
        "missing-call-receipt-direct"
    };
    write_evidence(
        case,
        &json!({
            "evidence": "canonical-reconciliation-test/v1",
            "case": case,
            "scope": "Unix development-witness TCP/UDS component fixture; not WAN or native guest execution",
            "fault": {"direction":"receiver-to-caller", "post_handshake_ordinal":withheld.ordinal,
                "withheld_envelope_bytes":withheld.length, "withheld_framing_bytes":4,
                "wire_hash_domain":"spiralcore.test.withheld.v1", "wire_hash":spiralcore_transfer::hex(&withheld.hash)},
            "actual_traffic": {
                "caller_endpoint":caller.counters,"receiver_endpoint":receiver.counters,
                "gate_forward":{"incoming_leg":forward.read,"outgoing_leg":forward.written},
                "gate_reverse":{"incoming_leg":reverse.read,"outgoing_leg":reverse.written},
                "gate_caller_handshake":traffic.caller_handshake,
                "gate_receiver_handshake":traffic.receiver_handshake,
                "relay":traffic.relay,
                "request_envelope_bytes":forward.captured[0].len(),
                "reply_envelope_bytes":reverse.captured[1].len(),
                "duplicate_receipt_envelope_bytes":reverse.captured[2].len()},
            "caller_session":caller_report,"receiver_session":receiver_report,
            "receiver_first_handled_processing":{
                "cpu_us":u64::try_from(receiver.first.as_ref().unwrap().meter.cpu_us).unwrap(),
                "wall_us":u64::try_from(receiver.first.as_ref().unwrap().meter.wall_us).unwrap()},
            "receiver_cleanup":{"second_handled":false,"result":"Cancelled","cache_empty":receiver.cache_empty},
            "lifecycle": {
                "cpu_domain":"ProcessTime process CPU during each worker interval, not exclusive thread CPU; intervals overlap",
                "wall_domain":"Instant wall over worker handshake/call intervals; receiver includes second-handle cleanup/cache inspection; caller excludes post-result hold",
                "caller":{"cpu_us":caller.lifecycle_cpu_us,"wall_us":caller.lifecycle_wall_us},
                "receiver":{"cpu_us":receiver.lifecycle_cpu_us,"wall_us":receiver.lifecycle_wall_us}},
            "accounting":"Receiver cumulative processing and per-Handled meters overlap lifecycle; do not add these domains or infer energy/performance improvements"
        }),
    );
    assert_relay_traffic(&traffic, &receiver.counters);
    for report in [&caller_report, &receiver_report] {
        assert_eq!(
            report.get("session_report").and_then(Value::as_str),
            Some("canonical-session-report/v2")
        );
        assert_eq!(report["normal_frames_per_call"].as_u64().unwrap(), 4);
        assert_eq!(
            report["normal_encrypted_destination_receipts_per_call"]
                .as_u64()
                .unwrap(),
            2
        );
        assert!(report.get("frames_per_call").is_none());
        assert!(
            report
                .get("encrypted_destination_receipts_per_call")
                .is_none()
        );
    }
    assert!(caller_report["recovery"]["idle_outcomes"].as_u64().unwrap() >= 1);
    assert_eq!(
        caller_report["recovery"]["commands_consumed"]
            .as_u64()
            .unwrap(),
        1
    );
    assert_eq!(
        caller_report["recovery"]["call_retry_invocations"]
            .as_u64()
            .unwrap(),
        1
    );
    for (field, expected) in [
        ("historical_receipt_invocations", 1),
        ("historical_receipts_sent", 1),
        ("reply_send_invocations", 1),
        ("cached_reply_retry_invocations", 0),
        ("receiver_failed_invocations", 1),
        ("historical_control_events", 0),
    ] {
        assert_eq!(
            receiver_report["recovery"][field].as_u64().unwrap(),
            expected,
            "{field}"
        );
    }
    assert_eq!(
        receiver_report["receiver_processing"]["segments"]
            .as_u64()
            .unwrap(),
        3
    );
    assert!(
        receiver_report["receiver_processing"]["cpu_us"]
            .as_u64()
            .is_some()
    );
    assert!(
        receiver_report["receiver_processing"]["wall_us"]
            .as_u64()
            .unwrap()
            > 0
    );
}
