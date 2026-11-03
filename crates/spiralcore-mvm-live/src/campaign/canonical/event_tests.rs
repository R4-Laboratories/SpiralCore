//! Next C8 peer-event preparation; intentionally not wired into the relay PR.
use super::*;
use crate::config::{Arm, Pair, Plan, Stage};
use mvm_core::{
    crypto::aead::Key,
    mailbox::{InMemoryReplayProtector, open_verified_envelope, verify_receipt},
};
use spiralcore_mvm_adapter::receipt::{PreparedReceipt, decode_destination_receipt};
use std::{net::Shutdown, os::unix::net::UnixStream};

const WATCHDOG: Duration = Duration::from_secs(5);

#[derive(Clone, Copy)]
enum EventOrder {
    DuplicateReceiptsThenReply,
    ReplyThenDuplicateReceipts,
    DuplicateReplyThenReceipt,
}

enum Event {
    Receipt,
    Reply,
}

struct PeerReport {
    calls: Vec<Value>,
    executions: usize,
    acknowledged_replies: usize,
    reply_receipts_received: usize,
    counters: StreamCounters,
}

// Raw owned shutdown handles can wake a worker blocked inside StreamTransport
// without waiting for its socket mutex. Every assertion path still joins it.
struct PeerGuard {
    caller_socket: UnixStream,
    peer_socket: UnixStream,
    done: std::sync::mpsc::Receiver<()>,
    worker: Option<std::thread::JoinHandle<Result<PeerReport>>>,
}

impl PeerGuard {
    fn stop(&self) {
        let _ = self.caller_socket.shutdown(Shutdown::Both);
        let _ = self.peer_socket.shutdown(Shutdown::Both);
    }

    fn finish(mut self, caller_succeeded: bool) -> (Result<PeerReport>, bool) {
        let completed = caller_succeeded && self.done.recv_timeout(WATCHDOG).is_ok();
        self.stop();
        let result = self.worker.take().unwrap().join().expect("peer panicked");
        (result, completed)
    }
}

impl Drop for PeerGuard {
    fn drop(&mut self) {
        self.stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn send_receipt(
    receipt: &PreparedReceipt<'_>,
    broker: &BrokerContext,
    stream: &StreamTransport<UnixStream>,
    cancel: &CancellationToken,
) -> Result<()> {
    let (permit, now) = broker
        .with_time(|_, now| -> Result<_> { Ok((receipt.reserve_send(cancel, now)?, now)) })?;
    permit.send(stream, cancel, now)?;
    Ok(())
}

fn request(index: usize) -> Frame {
    Frame {
        kind: 6,
        transfer: [29; 32],
        seq: index as u64,
        body: vec![0, index as u8, 0],
    }
}

fn reply(index: usize) -> Frame {
    Frame {
        kind: 101,
        transfer: [29; 32],
        seq: index as u64,
        body: vec![91, 0, index as u8, 0, 92],
    }
}

fn duplicate_receipt_event_order(order: EventOrder) {
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
    let peer_broker = broker_context().unwrap();
    let mut peer = Session::new(&pair.receiver, &peer_broker).unwrap();
    let receipt_material = pair
        .receiver
        .service
        .as_ref()
        .unwrap()
        .incoming_control
        .clone();
    let (socket, peer_socket) = UnixStream::pair().unwrap();
    let caller_shutdown = socket.try_clone().unwrap();
    let peer_shutdown = peer_socket.try_clone().unwrap();
    let stream = StreamTransport::new(
        socket,
        pair.sender
            .outgoing
            .as_ref()
            .unwrap()
            .binding
            .claims
            .route_id,
        WATCHDOG,
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
        WATCHDOG,
    )
    .unwrap();
    let (done_tx, done) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || -> Result<PeerReport> {
        let result = (|| -> Result<PeerReport> {
            let cancel = CancellationToken::default();
            peer_stream.handshake(HandshakeRole::Acceptor, &cancel)?;
            let mut report = PeerReport {
                calls: Vec::new(),
                executions: 0,
                acknowledged_replies: 0,
                reply_receipts_received: 0,
                counters: StreamCounters::default(),
            };
            for index in 0..2 {
                let original = peer_stream.receive(&cancel, Instant::now() + WATCHDOG)?;
                let (receipt, proof) = peer_broker.with_time(|_, now| -> Result<_> {
                    let (accepted, receipt) =
                        peer.returns
                            .admit(&mut peer.service, &peer.incoming, &original, now)?;
                    if accepted.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
                        return Err("fixture Call was not accepted exactly once".into());
                    }
                    let proof = peer
                        .service
                        .receive(&peer.incoming, now)?
                        .ok_or("fixture Call had no admitted proof")?;
                    Ok((receipt, proof))
                })?;
                if proof.payload() != request(index).encode()? {
                    return Err("caller changed the application request".into());
                }
                report.executions += 1;
                report.calls.push(json!({
                    "message_id":proof.header().message_id,
                    "correlation_id":proof.header().correlation_id,
                    "sequence":proof.header().sequence.get(),
                }));
                let (prepared, send) = peer_broker.with_time(|_, now| -> Result<_> {
                    let payload = reply(index).encode()?;
                    let options =
                        peer.ordering
                            .next_reply(&peer.outgoing, &proof, now, ISSUANCE_POLICY)?;
                    let prepared = peer.service.reply_prepared(
                        &peer.outgoing,
                        proof.handle(),
                        service_sender::request(&peer.outgoing, &payload, &options, now)?,
                        now,
                    )?;
                    let wire = encode_envelope(prepared.envelope())?;
                    let send = peer.tracker.register_send(
                        &peer.outgoing,
                        &peer.control,
                        &peer.peer_key,
                        &wire,
                        now,
                    )?;
                    Ok((prepared, send))
                })?;
                let events: &[Event] = match (index, order) {
                    (0, EventOrder::DuplicateReceiptsThenReply) => {
                        &[Event::Receipt, Event::Receipt, Event::Reply]
                    }
                    (0, EventOrder::ReplyThenDuplicateReceipts) => {
                        &[Event::Reply, Event::Receipt, Event::Receipt]
                    }
                    (0, EventOrder::DuplicateReplyThenReceipt) => {
                        &[Event::Reply, Event::Reply, Event::Receipt]
                    }
                    _ => &[Event::Receipt, Event::Reply],
                };
                for event in events {
                    match event {
                        Event::Receipt => {
                            // The same canonical encrypted control and signed
                            // receipt are retransmitted with a fresh send permit.
                            send_receipt(&receipt, &peer_broker, &peer_stream, &cancel)?;
                        }
                        Event::Reply => {
                            let now = peer_broker.with_time(|_, now| -> Result<_> { Ok(now) })?;
                            peer.outgoing
                                .send_prepared(&prepared, &peer_stream, &cancel, now)?;
                        }
                    }
                }
                let receipt_count =
                    if index == 0 && matches!(order, EventOrder::DuplicateReplyThenReceipt) {
                        2
                    } else {
                        1
                    };
                for receipt_index in 0..receipt_count {
                    let ack = peer_stream.receive(&cancel, Instant::now() + WATCHDOG)?;
                    let released = peer_broker.with_time(|_, now| -> Result<bool> {
                        if decode_envelope(&ack)?.encryption.key_id != peer.control_key {
                            return Err("expected canonical Reply destination receipt".into());
                        }
                        // Independently open the actual control with the canonical
                        // API to check Duplicate even when the monotone tracker
                        // retains Accepted and exposes no second available value.
                        let mut context = receipt_material.context()?;
                        context.now_unix_ms = now.unix_ms;
                        let opened = open_verified_envelope(
                            &decode_envelope(&ack)?,
                            &receipt_material.capability,
                            &context,
                            &Key::from_bytes(receipt_material.encryption),
                            &VerifyingKey::from_bytes(&receipt_material.verification)?,
                            &VerifyingKey::from_bytes(&receipt_material.authority)?,
                            &mut InMemoryReplayProtector::default(),
                        )?;
                        let signed = decode_destination_receipt(opened.payload())?;
                        verify_receipt(&signed, &peer.peer_key)?;
                        let expected = if receipt_index == 0 {
                            DeliveryOutcome::AcceptedEphemeral
                        } else {
                            DeliveryOutcome::Duplicate
                        };
                        if signed.receipt.outcome != expected {
                            return Err(
                                "Reply receipt did not report canonical acceptance/duplicate"
                                    .into(),
                            );
                        }
                        peer.tracker.observe_receipt(&peer.control, &ack, now)?;
                        match (
                            receipt_index,
                            peer.tracker.take_receipt(&peer.outgoing, &send, now)?,
                        ) {
                            (0, ReceiptObservation::Verified(ack)) => {
                                if ack.receipt.outcome != DeliveryOutcome::AcceptedEphemeral
                                    || !peer.service.acknowledge_reply(
                                        &peer.outgoing,
                                        &ack,
                                        &peer.peer_key,
                                        now,
                                    )?
                                {
                                    return Err(
                                        "Reply acknowledgment did not release the cached Reply"
                                            .into(),
                                    );
                                }
                                Ok(true)
                            }
                            (1, ReceiptObservation::Pending) => {
                                if !peer.tracker.facts(&peer.outgoing, &send, now)?.accepted {
                                    return Err(
                                        "lower-ranked duplicate erased authenticated acceptance"
                                            .into(),
                                    );
                                }
                                if peer.service.acknowledge_reply(
                                    &peer.outgoing,
                                    &signed,
                                    &peer.peer_key,
                                    now,
                                )? {
                                    return Err(
                                        "duplicate Reply receipt released the cache twice".into()
                                    );
                                }
                                Ok(false)
                            }
                            _ => Err(
                                "Reply receipt did not match expected monotone tracker state"
                                    .into(),
                            ),
                        }
                    })?;
                    report.reply_receipts_received += 1;
                    report.acknowledged_replies += usize::from(released);
                }
                peer_broker.with_time(|_, now| -> Result<()> {
                    if peer.service.receive(&peer.incoming, now)?.is_some()
                        || peer
                            .service
                            .retry_reply_prepared(&peer.incoming, &original, now)?
                            .is_some()
                    {
                        return Err("duplicate application execution or unreleased Reply".into());
                    }
                    Ok(())
                })?;
            }
            report.counters = peer_stream.counters()?;
            Ok(report)
        })();
        let _ = done_tx.send(());
        result
    });
    let guard = PeerGuard {
        caller_socket: caller_shutdown,
        peer_socket: peer_shutdown,
        done,
        worker: Some(worker),
    };
    let cancel = CancellationToken::default();
    let outcome = (|| -> Result<()> {
        stream.handshake(HandshakeRole::Initiator, &cancel)?;
        for index in 0..2 {
            let (_, actual) = caller.call(&request(index), &stream, &cancel)?;
            if actual.encode()? != reply(index).encode()? {
                return Err("caller completed with a changed or stale application Reply".into());
            }
            if caller.calls != index as u64 + 1 || caller.pending.is_some() {
                return Err("caller did not complete exactly its current pending identity".into());
            }
        }
        Ok(())
    })();
    let counters = stream.counters();
    let (peer_result, completed) = guard.finish(outcome.is_ok());
    assert!(
        outcome.is_ok(),
        "caller failed event demultiplexing: {outcome:?}"
    );
    assert!(
        completed,
        "peer did not finish its canonical Reply acknowledgments"
    );
    let report = peer_result.unwrap();
    let counters = counters.unwrap();
    assert_eq!(report.executions, 2);
    assert_eq!(report.acknowledged_replies, 2);
    let reply_receipt_count = if matches!(order, EventOrder::DuplicateReplyThenReceipt) {
        3
    } else {
        2
    };
    assert_eq!(report.reply_receipts_received, reply_receipt_count);
    assert_eq!(report.calls.len(), 2);
    assert_eq!(report.calls[0]["sequence"], 1);
    assert_eq!(report.calls[1]["sequence"], 2);
    assert_ne!(report.calls[0]["message_id"], report.calls[1]["message_id"]);
    assert_ne!(
        report.calls[0]["correlation_id"],
        report.calls[1]["correlation_id"]
    );
    // Two Calls plus actual canonical Reply receipts; retransmitted Replies
    // reuse one prepared envelope and never rerun application execution.
    let caller_frames = 2 + reply_receipt_count as u64;
    assert_eq!(
        (counters.frames_sent, counters.frames_received),
        (caller_frames, 5)
    );
    assert_eq!(
        (report.counters.frames_sent, report.counters.frames_received),
        (5, caller_frames)
    );
    assert_eq!(
        counters.envelope_bytes_sent,
        report.counters.envelope_bytes_received
    );
    assert_eq!(
        counters.envelope_bytes_received,
        report.counters.envelope_bytes_sent
    );
    assert_eq!(
        counters.framing_bytes_sent,
        report.counters.framing_bytes_received
    );
    assert_eq!(
        counters.framing_bytes_received,
        report.counters.framing_bytes_sent
    );
}

#[test]
fn duplicate_authenticated_call_receipts_before_reply_do_not_complete_or_execute_twice() {
    duplicate_receipt_event_order(EventOrder::DuplicateReceiptsThenReply);
}

#[test]
fn reply_before_duplicated_call_receipts_does_not_poison_the_next_pending_call() {
    duplicate_receipt_event_order(EventOrder::ReplyThenDuplicateReceipts);
}

#[test]
fn exact_duplicate_reply_before_call_receipt_is_acknowledged_without_reexecution() {
    duplicate_receipt_event_order(EventOrder::DuplicateReplyThenReceipt);
}
