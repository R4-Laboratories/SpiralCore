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

fn write_evidence(value: &Value) {
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
        .open(directory.join("missing-call-receipt-direct.json"))
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

fn exercise_active_receiver_fault(withhold_receipt_and_ack: bool) {
    let pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 128,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    let caller_broker = broker_context().unwrap();
    let mut caller = Session::new(&pair.sender, &caller_broker).unwrap();
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
    let (a, b, fault_gate) = gate::Gate::start_withheld(
        withhold_receipt_and_ack.then_some(2),
        Some(if withhold_receipt_and_ack { 1 } else { 2 }),
    )
    .unwrap();
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
            b.handshake(HandshakeRole::Acceptor, &cancel)?;
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
    let (caller, receiver, traffic) = workers.finish();
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
        traffic.reverse.captured[1], traffic.reverse.captured[3],
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
    assert_eq!(
        caller.counters.frames_received,
        traffic.reverse.written.frames_sent
    );
    assert_eq!(
        caller.session.report()["recovery"]["call_retry_invocations"],
        1
    );
    let report = receiver.session.report();
    assert_eq!(report["recovery"]["cached_reply_retry_invocations"], 1);
    assert_eq!(report["recovery"]["reply_send_invocations"], 2);
    assert_eq!(report["recovery"]["receiver_failed_invocations"], 0);
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
    write_evidence(&json!({
        "evidence": "canonical-reconciliation-test/v1",
        "case": "missing-call-receipt-direct",
        "scope": "Unix development-witness TCP fixture; not WAN or native guest execution",
        "fault": {"direction":"receiver-to-caller", "post_handshake_ordinal":withheld.ordinal,
            "withheld_envelope_bytes":withheld.length, "withheld_framing_bytes":4,
            "wire_hash_domain":"spiralcore.test.withheld.v1", "wire_hash":spiralcore_transfer::hex(&withheld.hash)},
        "actual_traffic": {
            "caller_endpoint":caller.counters,"receiver_endpoint":receiver.counters,
            "gate_forward":{"incoming_leg":forward.read,"outgoing_leg":forward.written},
            "gate_reverse":{"incoming_leg":reverse.read,"outgoing_leg":reverse.written},
            "gate_caller_handshake":traffic.caller_handshake,
            "gate_receiver_handshake":traffic.receiver_handshake,
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
    }));
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
