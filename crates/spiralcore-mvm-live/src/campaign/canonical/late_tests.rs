use super::*;
use crate::config::{Arm, Pair, Plan, Stage};
use mvm_contract::protocol::mailbox::{
    CausationId, ContentType, CorrelationId, SignedDestinationReceipt,
};
use mvm_core::{
    crypto::aead::Key,
    mailbox::{InMemoryReplayProtector, open_verified_envelope, verify_receipt},
};
use spiralcore_mvm_adapter::receipt::decode_destination_receipt;
use std::{net::Shutdown, os::unix::net::UnixStream, sync::mpsc};

const WATCHDOG: Duration = Duration::from_secs(5);

fn unknown_nonzero(mut known: [u8; 32]) -> [u8; 32] {
    known[0] ^= 1;
    if known == [0; 32] {
        known[1] = 1;
    }
    known
}

fn open_receipt(
    material: &Material,
    key: &VerifyingKey,
    wire: &[u8],
    now: ServiceTime,
) -> Result<SignedDestinationReceipt> {
    let mut context = material.context()?;
    context.now_unix_ms = now.unix_ms;
    let opened = open_verified_envelope(
        &decode_envelope(wire)?,
        &material.capability,
        &context,
        &Key::from_bytes(material.encryption),
        &VerifyingKey::from_bytes(&material.verification)?,
        &VerifyingKey::from_bytes(&material.authority)?,
        &mut InMemoryReplayProtector::default(),
    )?;
    let receipt = decode_destination_receipt(opened.payload())?;
    verify_receipt(&receipt, key)?;
    Ok(receipt)
}

struct PeerReport {
    call_wire_len: usize,
    negative_wire_len: usize,
    current_reply_wire_len: usize,
    call_receipt_wire_len: usize,
    acknowledgment_bytes: usize,
    outcomes: Vec<DeliveryOutcome>,
    counters: StreamCounters,
}

struct PeerGuard {
    caller_socket: UnixStream,
    peer_socket: UnixStream,
    done: mpsc::Receiver<()>,
    worker: Option<std::thread::JoinHandle<Result<PeerReport>>>,
}

impl PeerGuard {
    fn stop(&self) {
        let _ = self.caller_socket.shutdown(Shutdown::Both);
        let _ = self.peer_socket.shutdown(Shutdown::Both);
    }
    fn finish(mut self, succeeded: bool) -> (Result<PeerReport>, bool) {
        let completed = succeeded && self.done.recv_timeout(WATCHDOG).is_ok();
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

#[test]
fn unsolicited_late_then_exact_duplicate_is_acknowledged_without_current_progress() {
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
    let request = Frame {
        kind: 6,
        transfer: [53; 32],
        seq: 0,
        body: vec![0, 1, 0],
    };
    let expected = request.reply(101, vec![9, 0, 8, 0, 7]);
    let current_payload = expected.encode().unwrap();
    let expected_request = request.encode().unwrap();
    let negative_payload = request
        .reply(101, b"unsolicited negative input".to_vec())
        .encode()
        .unwrap();
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
    let (done_tx, done) = mpsc::channel();
    let worker = std::thread::spawn(move || -> Result<PeerReport> {
        let result = (|| -> Result<PeerReport> {
            let cancel = CancellationToken::default();
            peer_stream.handshake(HandshakeRole::Acceptor, &cancel)?;
            let original = peer_stream.receive(&cancel, Instant::now() + WATCHDOG)?;
            let (call_receipt, proof) = peer_broker.with_time(|_, now| -> Result<_> {
                let (accepted, receipt) =
                    peer.returns
                        .admit(&mut peer.service, &peer.incoming, &original, now)?;
                if accepted.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
                    return Err("current Call was not admitted".into());
                }
                Ok((
                    receipt,
                    peer.service
                        .receive(&peer.incoming, now)?
                        .ok_or("missing current Call proof")?,
                ))
            })?;
            if proof.payload() != expected_request {
                return Err("current application request changed".into());
            }
            let call_receipt_wire_len = call_receipt.encoded_bytes().len();
            let (negative, negative_send) = peer_broker.with_time(|_, now| -> Result<_> {
                // This is explicitly negative input, not an executed source
                // Call or cached application Reply. Public producer APIs allow
                // canonical signing; the receiver decides that it is Late.
                let mut options = peer.ordering.next_for_binding(
                    &peer.outgoing,
                    ContentType::parse(spiralcore_mvm_adapter::CONTENT_TYPE, 1)?,
                    now,
                    proof.header().expires_at_unix_ms,
                    ISSUANCE_POLICY,
                )?;
                options.correlation_id = CorrelationId::from_bytes(unknown_nonzero(
                    *proof.header().correlation_id.as_bytes(),
                ));
                options.causation_id =
                    CausationId::from_bytes(unknown_nonzero(*proof.header().message_id.as_bytes()));
                let prepared =
                    service_sender::prepare(&peer.outgoing, &negative_payload, &options, now)?;
                let send = peer.tracker.register_send(
                    &peer.outgoing,
                    &peer.control,
                    &peer.peer_key,
                    &encode_envelope(prepared.envelope())?,
                    now,
                )?;
                Ok((prepared, send))
            })?;
            if negative.header().sequence.get() != 1 {
                return Err("negative input did not use the first Reply FIFO position".into());
            }
            let negative_wire_len = encode_envelope(negative.envelope())?.len();
            let mut outcomes = Vec::new();
            let mut acknowledgment_bytes = 0;
            for expected_outcome in [DeliveryOutcome::Late, DeliveryOutcome::Duplicate] {
                let now = peer_broker.with_time(|_, now| -> Result<_> { Ok(now) })?;
                // Both transmissions use exactly this prepared envelope.
                peer.outgoing
                    .send_prepared(&negative, &peer_stream, &cancel, now)?;
                let ack = peer_stream.receive(&cancel, Instant::now() + WATCHDOG)?;
                acknowledgment_bytes += ack.len();
                peer_broker.with_time(|_, now| -> Result<()> {
                    let signed = open_receipt(&receipt_material, &peer.peer_key, &ack, now)?;
                    if signed.receipt.outcome != expected_outcome {
                        return Err(
                            "unsolicited input received an invented acceptance outcome".into()
                        );
                    }
                    peer.tracker.observe_receipt(&peer.control, &ack, now)?;
                    let ReceiptObservation::Verified(observed) =
                        peer.tracker
                            .take_receipt(&peer.outgoing, &negative_send, now)?
                    else {
                        return Err("negative receipt did not match its tracked send".into());
                    };
                    if observed.receipt.outcome != expected_outcome
                        || peer
                            .tracker
                            .facts(&peer.outgoing, &negative_send, now)?
                            .accepted
                    {
                        return Err("Late/Duplicate receipt invented destination acceptance".into());
                    }
                    // No acknowledge_reply/cache release: this negative input
                    // has no admitted source Call or cached application Reply.
                    Ok(())
                })?;
                outcomes.push(expected_outcome);
            }
            let (current_reply, current_send) = peer_broker.with_time(|_, now| -> Result<_> {
                let options =
                    peer.ordering
                        .next_reply(&peer.outgoing, &proof, now, ISSUANCE_POLICY)?;
                let prepared = peer.service.reply_prepared(
                    &peer.outgoing,
                    proof.handle(),
                    service_sender::request(&peer.outgoing, &current_payload, &options, now)?,
                    now,
                )?;
                let send = peer.tracker.register_send(
                    &peer.outgoing,
                    &peer.control,
                    &peer.peer_key,
                    &encode_envelope(prepared.envelope())?,
                    now,
                )?;
                Ok((prepared, send))
            })?;
            if current_reply.header().sequence.get() != 2
                || current_reply.header().ordering_scope != negative.header().ordering_scope
            {
                return Err("current Reply did not retain its next FIFO position".into());
            }
            let current_reply_wire_len = encode_envelope(current_reply.envelope())?.len();
            let (permit, now) = peer_broker.with_time(|_, now| -> Result<_> {
                Ok((call_receipt.reserve_send(&cancel, now)?, now))
            })?;
            permit.send(&peer_stream, &cancel, now)?;
            drop(call_receipt);
            let now = peer_broker.with_time(|_, now| -> Result<_> { Ok(now) })?;
            peer.outgoing
                .send_prepared(&current_reply, &peer_stream, &cancel, now)?;
            let ack = peer_stream.receive(&cancel, Instant::now() + WATCHDOG)?;
            acknowledgment_bytes += ack.len();
            peer_broker.with_time(|_, now| -> Result<()> {
                if open_receipt(&receipt_material, &peer.peer_key, &ack, now)?
                    .receipt
                    .outcome
                    != DeliveryOutcome::AcceptedEphemeral
                {
                    return Err("current Reply did not receive canonical acceptance".into());
                }
                peer.tracker.observe_receipt(&peer.control, &ack, now)?;
                let ReceiptObservation::Verified(signed) =
                    peer.tracker
                        .take_receipt(&peer.outgoing, &current_send, now)?
                else {
                    return Err("current ACK did not match its original Reply".into());
                };
                if signed.receipt.outcome != DeliveryOutcome::AcceptedEphemeral
                    || !peer.service.acknowledge_reply(
                        &peer.outgoing,
                        &signed,
                        &peer.peer_key,
                        now,
                    )?
                {
                    return Err("current Reply cache was not released exactly once".into());
                }
                if peer.service.receive(&peer.incoming, now)?.is_some()
                    || peer
                        .service
                        .retry_reply_prepared(&peer.incoming, &original, now)?
                        .is_some()
                {
                    return Err("extra execution or unreleased current Reply".into());
                }
                Ok(())
            })?;
            outcomes.push(DeliveryOutcome::AcceptedEphemeral);
            Ok(PeerReport {
                call_wire_len: original.len(),
                negative_wire_len,
                current_reply_wire_len,
                call_receipt_wire_len,
                acknowledgment_bytes,
                outcomes,
                counters: peer_stream.counters()?,
            })
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
    stream.handshake(HandshakeRole::Initiator, &cancel).unwrap();
    let outcome = caller.call(&request, &stream, &cancel);
    let counters = stream.counters().unwrap();
    let (peer_result, completed) = guard.finish(outcome.is_ok());
    let (_, actual) =
        outcome.expect("authenticated Late and its replay must not refuse the current Call");
    assert_eq!(actual, expected);
    assert!(completed, "peer did not finish all canonical receipts");
    let report = peer_result.unwrap();
    assert_eq!(caller.calls, 1);
    assert!(caller.pending.is_none());
    assert_eq!(
        report.outcomes,
        [
            DeliveryOutcome::Late,
            DeliveryOutcome::Duplicate,
            DeliveryOutcome::AcceptedEphemeral
        ]
    );
    assert_eq!((counters.frames_sent, counters.frames_received), (4, 4));
    assert_eq!(
        (report.counters.frames_sent, report.counters.frames_received),
        (4, 4)
    );
    assert_eq!(
        counters.envelope_bytes_sent,
        (report.call_wire_len + report.acknowledgment_bytes) as u64
    );
    assert_eq!(
        counters.envelope_bytes_received,
        (2 * report.negative_wire_len
            + report.current_reply_wire_len
            + report.call_receipt_wire_len) as u64
    );
    assert_eq!(
        counters.envelope_bytes_sent,
        report.counters.envelope_bytes_received
    );
    assert_eq!(
        counters.envelope_bytes_received,
        report.counters.envelope_bytes_sent
    );
    assert_eq!(counters.framing_bytes_sent, 8 + 4 * 4);
    assert_eq!(counters.framing_bytes_received, 8 + 4 * 4);
    assert_eq!(report.counters.framing_bytes_sent, 8 + 4 * 4);
    assert_eq!(report.counters.framing_bytes_received, 8 + 4 * 4);
}
