//! Explicit signed capacity-two fixture for generic service FIFO behavior.
//! Ordinary/native pipeline-one provisioning signs max_outstanding_calls=1 and
//! correctly refuses a second active Call. This test does not qualify that
//! profile for pipelining or change its grants, limits, or claimed support.
use super::*;
use crate::config::{Arm, Pair, Plan, Stage};
use mvm_core::mailbox::{sign_capability, transport::sign_transport_binding};
use spiralcore_mvm_adapter::{receipt::SendHandle, service::AdmittedMessage};
use std::{net::Shutdown, os::unix::net::UnixStream, sync::mpsc};

const WATCHDOG: Duration = Duration::from_secs(5);

fn signed_capacity_two_pair() -> Pair {
    let mut pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 128,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    // New test-owned authority; retain every grant field except this explicit
    // capacity. Re-sign both endpoint copies and their existing route bindings.
    let authority = SigningKey::from_bytes(&rand::random());
    for bootstrap in [&mut pair.sender, &mut pair.receiver] {
        let service = bootstrap.service.as_mut().unwrap();
        for material in [
            bootstrap.incoming.as_mut().unwrap(),
            bootstrap.outgoing.as_mut().unwrap(),
            &mut service.incoming_control,
            &mut service.outgoing_control,
        ] {
            assert_eq!(material.capability.grant.max_outstanding_calls, 1);
            material.capability.grant.max_outstanding_calls = 2;
            material.capability =
                sign_capability(material.capability.grant.clone(), &authority).unwrap();
            material.binding =
                sign_transport_binding(material.binding.claims.clone(), &authority).unwrap();
            material.authority = authority.verifying_key().to_bytes();
        }
    }
    pair
}

fn request(index: u64) -> Frame {
    Frame {
        kind: 6,
        transfer: [47; 32],
        seq: index,
        body: vec![0, index as u8, 0],
    }
}

fn reply(index: u64) -> Frame {
    request(index).reply(101, vec![9, 0, index as u8, 0, 8])
}

fn acknowledge(
    peer: &mut Session,
    broker: &BrokerContext,
    wire: &[u8],
    send: &SendHandle,
) -> Result<()> {
    broker.with_time(|_, now| -> Result<()> {
        if decode_envelope(wire)?.encryption.key_id != peer.control_key {
            return Err("expected provisioned canonical Reply receipt".into());
        }
        peer.tracker.observe_receipt(&peer.control, wire, now)?;
        let ReceiptObservation::Verified(receipt) =
            peer.tracker.take_receipt(&peer.outgoing, send, now)?
        else {
            return Err("Reply receipt did not match its original tracked send".into());
        };
        if receipt.receipt.outcome != DeliveryOutcome::AcceptedEphemeral
            || !peer
                .service
                .acknowledge_reply(&peer.outgoing, &receipt, &peer.peer_key, now)?
        {
            return Err("canonical Reply receipt did not release its cache".into());
        }
        Ok(())
    })
}

struct PeerReport {
    earlier_call: Vec<u8>,
    current_call: Vec<u8>,
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
fn signed_capacity_two_current_reply_waits_for_the_earlier_fifo_reply() {
    let pair = signed_capacity_two_pair();
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
    let (done_tx, done) = mpsc::channel();
    let worker = std::thread::spawn(move || -> Result<PeerReport> {
        let result = (|| -> Result<PeerReport> {
            let cancel = CancellationToken::default();
            peer_stream.handshake(HandshakeRole::Acceptor, &cancel)?;
            let earlier_call = peer_stream.receive(&cancel, Instant::now() + WATCHDOG)?;
            let earlier_proof: AdmittedMessage = peer_broker.with_time(|_, now| -> Result<_> {
                let (accepted, receipt) =
                    peer.returns
                        .admit(&mut peer.service, &peer.incoming, &earlier_call, now)?;
                if accepted.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
                    return Err("earlier fixture Call was not admitted".into());
                }
                // The earlier Call is fixture setup; its destination receipt
                // is prepared but intentionally not transmitted in this gate.
                drop(receipt);
                Ok(peer
                    .service
                    .receive(&peer.incoming, now)?
                    .ok_or("missing earlier proof")?)
            })?;
            if earlier_proof.payload() != request(0).encode()?
                || earlier_proof.header().sequence.get() != 1
            {
                return Err("earlier fixture request/cursor changed".into());
            }
            let (earlier_reply, earlier_send) = peer_broker.with_time(|_, now| -> Result<_> {
                let payload = reply(0).encode()?;
                let options = peer.ordering.next_reply(
                    &peer.outgoing,
                    &earlier_proof,
                    now,
                    ISSUANCE_POLICY,
                )?;
                let prepared = peer.service.reply_prepared(
                    &peer.outgoing,
                    earlier_proof.handle(),
                    service_sender::request(&peer.outgoing, &payload, &options, now)?,
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
            let current_call = peer_stream.receive(&cancel, Instant::now() + WATCHDOG)?;
            let (current_receipt, current_proof) =
                peer_broker.with_time(|_, now| -> Result<_> {
                    let (accepted, receipt) = peer.returns.admit(
                        &mut peer.service,
                        &peer.incoming,
                        &current_call,
                        now,
                    )?;
                    if accepted.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
                        return Err("current fixture Call was not admitted".into());
                    }
                    Ok((
                        receipt,
                        peer.service
                            .receive(&peer.incoming, now)?
                            .ok_or("missing current proof")?,
                    ))
                })?;
            if current_proof.payload() != request(1).encode()?
                || current_proof.header().sequence.get() != 2
                || current_proof.header().ordering_scope != earlier_proof.header().ordering_scope
                || current_proof.header().correlation_id == earlier_proof.header().correlation_id
            {
                return Err("current request did not retain its next FIFO identity".into());
            }
            let (current_reply, current_send) = peer_broker.with_time(|_, now| -> Result<_> {
                let payload = reply(1).encode()?;
                let options = peer.ordering.next_reply(
                    &peer.outgoing,
                    &current_proof,
                    now,
                    ISSUANCE_POLICY,
                )?;
                let prepared = peer.service.reply_prepared(
                    &peer.outgoing,
                    current_proof.handle(),
                    service_sender::request(&peer.outgoing, &payload, &options, now)?,
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
            if earlier_reply.header().sequence.get() != 1
                || current_reply.header().sequence.get() != 2
                || earlier_reply.header().ordering_scope != current_reply.header().ordering_scope
            {
                return Err("prepared canonical Replies did not share the increasing FIFO".into());
            }
            let (permit, now) = peer_broker.with_time(|_, now| -> Result<_> {
                Ok((current_receipt.reserve_send(&cancel, now)?, now))
            })?;
            permit.send(&peer_stream, &cancel, now)?;
            drop(current_receipt);
            let now = peer_broker.with_time(|_, now| -> Result<_> { Ok(now) })?;
            peer.outgoing
                .send_prepared(&current_reply, &peer_stream, &cancel, now)?;
            let current_ack = peer_stream.receive(&cancel, Instant::now() + WATCHDOG)?;
            acknowledge(&mut peer, &peer_broker, &current_ack, &current_send)?;
            // Only release Reply1 after authenticating Reply2's real receipt.
            // Before this, observation readiness cannot imply FIFO readiness.
            let now = peer_broker.with_time(|_, now| -> Result<_> { Ok(now) })?;
            peer.outgoing
                .send_prepared(&earlier_reply, &peer_stream, &cancel, now)?;
            let earlier_ack = peer_stream.receive(&cancel, Instant::now() + WATCHDOG)?;
            acknowledge(&mut peer, &peer_broker, &earlier_ack, &earlier_send)?;
            Ok(PeerReport {
                earlier_call,
                current_call,
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
    let (prepared, earlier_call, earlier_send, setup_wire, now) = broker
        .with_time(|_, now| -> Result<_> {
            let payload = request(0).encode()?;
            let mut options = caller.ordering.next_for_binding(
                &caller.outgoing,
                mvm_contract::protocol::mailbox::ContentType::parse(
                    spiralcore_mvm_adapter::CONTENT_TYPE,
                    1,
                )?,
                now,
                now.unix_ms + 30_000,
                ISSUANCE_POLICY,
            )?;
            options.reply_to = caller.reply_to;
            let prepared = service_sender::prepare(&caller.outgoing, &payload, &options, now)?;
            let wire = encode_envelope(prepared.envelope())?;
            let call = caller.service.register_call(&caller.outgoing, &wire, now)?;
            let send = caller.tracker.register_send(
                &caller.outgoing,
                &caller.control,
                &caller.peer_key,
                &wire,
                now,
            )?;
            Ok((prepared, call, send, wire, now))
        })
        .unwrap();
    // Real, separately accounted fixture traffic on this same ready stream.
    caller
        .service
        .send_call_prepared(
            &caller.outgoing,
            &earlier_call,
            &prepared,
            &stream,
            &cancel,
            now,
        )
        .unwrap();
    let outcome = caller.call(&request(1), &stream, &cancel);
    let counters = stream.counters().unwrap();
    let (peer_result, completed) = guard.finish(outcome.is_ok());
    let (_, current_payload) =
        outcome.expect("current Call must wait for its preceding FIFO Reply");
    assert_eq!(current_payload, reply(1));
    assert!(
        completed,
        "peer did not complete both canonical Reply receipts"
    );
    let report = peer_result.unwrap();
    assert_eq!(caller.calls, 1);
    assert!(caller.pending.is_none());
    assert_eq!(report.earlier_call, setup_wire);
    assert_ne!(report.current_call, setup_wire);
    let earlier = broker
        .with_time(|_, now| -> Result<_> {
            assert!(
                caller
                    .tracker
                    .facts(&caller.outgoing, &earlier_send, now)?
                    .application_replied
            );
            Ok(caller
                .service
                .poll_call(&caller.outgoing, &earlier_call, now)?)
        })
        .unwrap();
    assert_eq!(
        earlier,
        CallStatus::Replied(Some(reply(0).encode().unwrap()))
    );
    // Includes one transmitted setup Call, one current Call, and two real ACKs.
    assert_eq!((counters.frames_sent, counters.frames_received), (4, 3));
    assert_eq!(
        (report.counters.frames_sent, report.counters.frames_received),
        (3, 4)
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
    assert_eq!(counters.framing_bytes_received, 8 + 3 * 4);
    assert_eq!(report.counters.framing_bytes_received, 8 + 4 * 4);
    assert_eq!(report.counters.framing_bytes_sent, 8 + 3 * 4);
}
