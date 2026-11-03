//! Next C8 peer-event preparation; intentionally not wired into the relay PR.
use super::*;
use crate::config::{Arm, Pair, Plan, Stage};
use spiralcore_mvm_adapter::receipt::PreparedReceipt;
use std::{net::Shutdown, os::unix::net::UnixStream};

const WATCHDOG: Duration = Duration::from_secs(5);

#[derive(Clone, Copy)]
enum EventOrder {
    DuplicateReceiptsThenReply,
    ReplyThenDuplicateReceipts,
}

enum Event {
    Receipt,
    Reply,
}

struct PeerReport {
    calls: Vec<Value>,
    executions: usize,
    acknowledged_replies: usize,
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
                let ack = peer_stream.receive(&cancel, Instant::now() + WATCHDOG)?;
                peer_broker.with_time(|_, now| -> Result<()> {
                    if decode_envelope(&ack)?.encryption.key_id != peer.control_key {
                        return Err("expected canonical Reply destination receipt".into());
                    }
                    peer.tracker.observe_receipt(&peer.control, &ack, now)?;
                    let ReceiptObservation::Verified(ack) =
                        peer.tracker.take_receipt(&peer.outgoing, &send, now)?
                    else {
                        return Err("Reply acknowledgment did not match its original send".into());
                    };
                    if ack.receipt.outcome != DeliveryOutcome::AcceptedEphemeral
                        || !peer.service.acknowledge_reply(
                            &peer.outgoing,
                            &ack,
                            &peer.peer_key,
                            now,
                        )?
                    {
                        return Err("Reply acknowledgment did not release the cached Reply".into());
                    }
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
                report.acknowledged_replies += 1;
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
    assert_eq!(report.calls.len(), 2);
    assert_eq!(report.calls[0]["sequence"], 1);
    assert_eq!(report.calls[1]["sequence"], 2);
    assert_ne!(report.calls[0]["message_id"], report.calls[1]["message_id"]);
    assert_ne!(
        report.calls[0]["correlation_id"],
        report.calls[1]["correlation_id"]
    );
    // Two Calls, two canonical Reply receipts; no duplicate-triggered resend.
    assert_eq!((counters.frames_sent, counters.frames_received), (4, 5));
    assert_eq!(
        (report.counters.frames_sent, report.counters.frames_received),
        (5, 4)
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
