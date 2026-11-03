//! Functional bounds for the opt-in consumer. Capacity two is test-owned and
//! signed explicitly; it does not change the production pipeline-one profile.
use super::*;
use crate::{
    campaign::reconciliation::bounded_requests,
    config::{Arm, Pair, Plan, Stage},
};
use mvm_core::mailbox::{
    sign_capability,
    transport::{PreparedMailboxEnvelope, sign_transport_binding},
};
use spiralcore_mvm_adapter::receipt::SendHandle;
use std::{net::Shutdown, os::unix::net::UnixStream, sync::mpsc, thread::JoinHandle};

const WATCHDOG: Duration = Duration::from_secs(5);

fn pair(capacity_two: bool) -> Pair {
    let mut pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 128,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    if capacity_two {
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
    }
    pair
}

fn frame(index: u8) -> Frame {
    Frame {
        kind: 6,
        transfer: [173; 32],
        seq: u64::from(index),
        body: vec![0, index, 0],
    }
}
fn response(index: u8) -> Frame {
    frame(index).reply(101, vec![9, 0, index, 0, 8])
}

fn options(session: &mut Session, now: ServiceTime) -> Result<service_sender::MessageOptions> {
    let mut options = session.ordering.next_for_binding(
        &session.outgoing,
        mvm_contract::protocol::mailbox::ContentType::parse(
            spiralcore_mvm_adapter::CONTENT_TYPE,
            1,
        )?,
        now,
        now.unix_ms + 30_000,
        ISSUANCE_POLICY,
    )?;
    options.reply_to = session.reply_to;
    Ok(options)
}

fn stream(socket: UnixStream, bootstrap: &Bootstrap) -> StreamTransport<UnixStream> {
    StreamTransport::new(
        socket,
        bootstrap.outgoing.as_ref().unwrap().binding.claims.route_id,
        WATCHDOG,
    )
    .unwrap()
}

struct Guard<T> {
    shutdown: [UnixStream; 2],
    done: mpsc::Receiver<()>,
    worker: Option<JoinHandle<Result<T>>>,
}
impl<T> Guard<T> {
    fn stop(&self) {
        for socket in &self.shutdown {
            if socket.shutdown(Shutdown::Both).is_err() {
                let _ = socket.shutdown(Shutdown::Write);
                let _ = socket.shutdown(Shutdown::Read);
            }
        }
    }
    fn finish(mut self) -> Result<T> {
        let complete = self.done.recv_timeout(WATCHDOG).is_ok();
        self.stop();
        let result = self
            .worker
            .take()
            .unwrap()
            .join()
            .expect("fixture peer panicked");
        assert!(complete, "fixture peer exceeded its owned watchdog");
        result
    }
}
impl<T> Drop for Guard<T> {
    fn drop(&mut self) {
        self.stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn prepare_reply(
    peer: &mut Session,
    broker: &BrokerContext,
    wire: &[u8],
    reply: &Frame,
) -> Result<(
    mvm_contract::protocol::mailbox::SignedDestinationReceipt,
    PreparedMailboxEnvelope,
    SendHandle,
)> {
    broker.with_time(|_, now| -> Result<_> {
        let receipt = peer.service.admit(&peer.incoming, wire, now)?;
        if receipt.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
            return Err("fixture Call not freshly admitted".into());
        }
        let proof = peer
            .service
            .receive(&peer.incoming, now)?
            .ok_or("missing fixture proof")?;
        let options = peer
            .ordering
            .next_reply(&peer.outgoing, &proof, now, ISSUANCE_POLICY)?;
        let payload = reply.encode()?;
        let prepared = peer.service.reply_prepared(
            &peer.outgoing,
            proof.handle(),
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
        Ok((receipt, prepared, send))
    })
}

fn acknowledge(
    peer: &mut Session,
    broker: &BrokerContext,
    wire: &[u8],
    send: &SendHandle,
) -> Result<()> {
    broker.with_time(|_, now| -> Result<()> {
        peer.tracker.observe_receipt(&peer.control, wire, now)?;
        let ReceiptObservation::Verified(receipt) =
            peer.tracker.take_receipt(&peer.outgoing, send, now)?
        else {
            return Err("missing current authenticated Reply receipt".into());
        };
        if !peer
            .service
            .acknowledge_reply(&peer.outgoing, &receipt, &peer.peer_key, now)?
        {
            return Err("authenticated ACK did not release cache".into());
        }
        Ok(())
    })
}

#[test]
fn fifo_ready_idle_keeps_queued_command_until_authenticated_predecessor_without_retry() {
    let pair = pair(true);
    let broker = broker_context().unwrap();
    let mut caller = Session::new(&pair.sender, &broker).unwrap();
    let peer_broker = broker_context().unwrap();
    let mut peer = Session::new(&pair.receiver, &peer_broker).unwrap();
    let (a, b) = UnixStream::pair().unwrap();
    let shutdown = [a.try_clone().unwrap(), b.try_clone().unwrap()];
    let a = stream(a, &pair.sender);
    let b = stream(b, &pair.receiver);
    let (signal, requests) = bounded_requests();
    let (done_tx, done) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let result = (|| -> Result<StreamCounters> {
            let cancel = CancellationToken::default();
            let deadline = Instant::now() + WATCHDOG;
            b.handshake(HandshakeRole::Acceptor, &cancel)?;
            let earlier_wire = b.receive(&cancel, deadline)?;
            let (_, earlier_reply, earlier_send) =
                prepare_reply(&mut peer, &peer_broker, &earlier_wire, &response(0))?;
            let current_wire = b.receive(&cancel, deadline)?;
            let (receipt, current_reply, current_send) =
                prepare_reply(&mut peer, &peer_broker, &current_wire, &response(1))?;
            assert_eq!(earlier_reply.header().sequence.get(), 1);
            assert_eq!(current_reply.header().sequence.get(), 2);
            assert_eq!(
                earlier_reply.header().ordering_scope,
                current_reply.header().ordering_scope
            );
            let receipt = peer_broker.with_time(|_, now| -> Result<_> {
                peer.returns
                    .reserve(&peer.incoming, &current_wire, now)?
                    .seal(&receipt, now)
                    .map_err(Into::into)
            })?;
            let (permit, now) = peer_broker.with_time(|_, now| -> Result<_> {
                Ok((receipt.reserve_send(&cancel, now)?, now))
            })?;
            permit.send(&b.before(deadline), &cancel, now)?;
            drop(receipt);
            let now = peer_broker.with_time(|_, now| -> Result<_> { Ok(now) })?;
            peer.outgoing
                .send_prepared(&current_reply, &b.before(deadline), &cancel, now)?;
            let ack = b.receive(&cancel, deadline)?;
            acknowledge(&mut peer, &peer_broker, &ack, &current_send)?;
            assert_eq!(signal.request_retry(), Ok(()));
            // Real reader silence spans a caller 250-ms Idle. The queued
            // request remains full and no retransmitted Call/ACK may arrive.
            assert!(matches!(
                b.receive_or_idle(&cancel, deadline, Duration::from_millis(450))?,
                spiralcore_mvm_adapter::live::ReceiveOutcome::Idle
            ));
            assert_eq!(signal.request_retry(), Err(mpsc::TrySendError::Full(())));
            let now = peer_broker.with_time(|_, now| -> Result<_> { Ok(now) })?;
            peer.outgoing
                .send_prepared(&earlier_reply, &b.before(deadline), &cancel, now)?;
            let ack = b.receive(&cancel, deadline)?;
            acknowledge(&mut peer, &peer_broker, &ack, &earlier_send)?;
            Ok(b.counters()?)
        })();
        let _ = done_tx.send(());
        result
    });
    let guard = Guard {
        shutdown,
        done,
        worker: Some(worker),
    };
    let cancel = CancellationToken::default();
    a.handshake(HandshakeRole::Initiator, &cancel).unwrap();
    let (earlier, earlier_call, current_call, now) = broker
        .with_time(|_, now| -> Result<_> {
            let earlier_options = options(&mut caller, now)?;
            let earlier = service_sender::prepare(
                &caller.outgoing,
                &frame(0).encode()?,
                &earlier_options,
                now,
            )?;
            let wire = encode_envelope(earlier.envelope())?;
            let earlier_call = caller.service.register_call(&caller.outgoing, &wire, now)?;
            caller.tracker.register_send(
                &caller.outgoing,
                &caller.control,
                &caller.peer_key,
                &wire,
                now,
            )?;
            let current_options = options(&mut caller, now)?;
            let mut pending = PendingCall::new(frame(1).encode()?, current_options, now);
            pending.register(
                &mut caller.service,
                &mut caller.tracker,
                &caller.outgoing,
                &caller.control,
                &caller.peer_key,
                now,
            )?;
            let current_call = pending.call_handle().ok_or(ServiceError::State)?;
            caller.pending = Some(pending);
            Ok((earlier, earlier_call, current_call, now))
        })
        .unwrap();
    caller
        .service
        .send_call_prepared(
            &caller.outgoing,
            &earlier_call,
            &earlier,
            &a.before(Instant::now() + WATCHDOG),
            &cancel,
            now,
        )
        .unwrap();
    let result = caller.call_with_reconciliation(&frame(1), &a, &cancel, requests);
    let counters = a.counters().unwrap();
    let peer_counters = guard.finish().unwrap();
    assert_eq!(result.unwrap().1, response(1));
    assert_eq!(caller.calls, 1);
    assert!(caller.pending.is_none());
    let report = caller.report();
    assert!(report["recovery"]["idle_outcomes"].as_u64().unwrap() >= 1);
    assert_eq!(report["recovery"]["commands_consumed"], 0);
    assert_eq!(report["recovery"]["call_retry_invocations"], 0);
    assert_eq!((counters.frames_sent, counters.frames_received), (4, 3));
    assert_eq!(
        (peer_counters.frames_sent, peer_counters.frames_received),
        (3, 4)
    );
    broker
        .with_time(|_, now| -> Result<()> {
            assert_eq!(
                caller
                    .service
                    .poll_call(&caller.outgoing, &current_call, now)?,
                CallStatus::Replied(None)
            );
            assert_eq!(
                caller
                    .service
                    .poll_call(&caller.outgoing, &earlier_call, now)?,
                CallStatus::Replied(Some(response(0).encode()?))
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn fifteen_authenticated_old_acks_then_fresh_sixteenth_call_allocates_no_application_state() {
    let pair = pair(false);
    let broker = broker_context().unwrap();
    let mut receiver = Session::new(&pair.receiver, &broker).unwrap();
    let peer_broker = broker_context().unwrap();
    let mut peer = Session::new(&pair.sender, &peer_broker).unwrap();
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
    let transfer = [181; 32];
    let probe = Frame {
        kind: 5,
        transfer,
        seq: 0,
        body: Vec::new(),
    };
    let old_wire = peer_broker
        .with_time(|_, now| -> Result<_> {
            let options = options(&mut peer, now)?;
            let call = service_sender::prepare(&peer.outgoing, &probe.encode()?, &options, now)?;
            let wire = encode_envelope(call.envelope())?;
            peer.service.register_call(&peer.outgoing, &wire, now)?;
            Ok(wire)
        })
        .unwrap();
    // Canonical fixture setup obtains a real proof/Reply/ACK and releases its
    // cache. No synthetic private tracker/service entry is inserted.
    let (proof, reply_wire, reply_send, unknown) = broker
        .with_time(|_, now| -> Result<_> {
            assert_eq!(
                receiver
                    .service
                    .admit(&receiver.incoming, &old_wire, now)?
                    .receipt
                    .outcome,
                DeliveryOutcome::AcceptedEphemeral
            );
            let proof = receiver
                .service
                .receive(&receiver.incoming, now)?
                .ok_or("missing old proof")?;
            let unknown = object.handle(&receiver.incoming, &proof, now)?;
            let options =
                receiver
                    .ordering
                    .next_reply(&receiver.outgoing, &proof, now, ISSUANCE_POLICY)?;
            let reply = receiver.service.reply_prepared(
                &receiver.outgoing,
                proof.handle(),
                service_sender::request(&receiver.outgoing, &unknown.encode()?, &options, now)?,
                now,
            )?;
            let wire = encode_envelope(reply.envelope())?;
            let send = receiver.tracker.register_send(
                &receiver.outgoing,
                &receiver.control,
                &receiver.peer_key,
                &wire,
                now,
            )?;
            Ok((proof, wire, send, unknown))
        })
        .unwrap();
    assert_eq!(unknown.kind, 127);
    assert_eq!(
        unknown.body,
        (transfer::Error::Unknown as u16).to_le_bytes()
    );
    let (signed_ack, ack_wire) = peer_broker
        .with_time(|_, now| -> Result<_> {
            let (signed, ack) =
                peer.returns
                    .admit(&mut peer.service, &peer.incoming, &reply_wire, now)?;
            let wire = ack.encoded_bytes().to_vec();
            Ok((signed, wire))
        })
        .unwrap();
    acknowledge(&mut receiver, &broker, &ack_wire, &reply_send).unwrap();
    let before = receiver.service.snapshot();
    let fresh = Frame {
        kind: 1,
        transfer,
        seq: 0,
        body: spiralcore_transfer::protocol::Offer::cold(
            b"new",
            spiralcore_transfer::protocol::Mode::Raw,
        )
        .encode()
        .unwrap(),
    };
    let prepared = peer_broker
        .with_time(|_, now| -> Result<_> {
            let options = options(&mut peer, now)?;
            Ok(service_sender::prepare(
                &peer.outgoing,
                &fresh.encode()?,
                &options,
                now,
            )?)
        })
        .unwrap();
    let (a, b) = UnixStream::pair().unwrap();
    let shutdown = [a.try_clone().unwrap(), b.try_clone().unwrap()];
    let a = stream(a, &pair.receiver);
    let b = stream(b, &pair.sender);
    let (done_tx, done) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let result = (|| -> Result<StreamCounters> {
            let cancel = CancellationToken::default();
            let deadline = Instant::now() + WATCHDOG;
            b.handshake(HandshakeRole::Initiator, &cancel)?;
            let receipt = peer_broker.with_time(|_, now| -> Result<_> {
                peer.returns
                    .reserve(&peer.incoming, &reply_wire, now)?
                    .seal(&signed_ack, now)
                    .map_err(Into::into)
            })?;
            for _ in 0..15 {
                let (permit, now) = peer_broker.with_time(|_, now| -> Result<_> {
                    Ok((receipt.reserve_send(&cancel, now)?, now))
                })?;
                permit.send(&b.before(deadline), &cancel, now)?;
            }
            let now = peer_broker.with_time(|_, now| -> Result<_> { Ok(now) })?;
            peer.outgoing
                .send_prepared(&prepared, &b.before(deadline), &cancel, now)?;
            Ok(b.counters()?)
        })();
        let _ = done_tx.send(());
        result
    });
    let guard = Guard {
        shutdown,
        done,
        worker: Some(worker),
    };
    let cancel = CancellationToken::default();
    a.handshake(HandshakeRole::Acceptor, &cancel).unwrap();
    let error = receiver
        .handle(transfer, &mut object, &a, &cancel)
        .err()
        .expect("sixteenth new Call was admitted");
    let counters = a.counters().unwrap();
    let sent = guard.finish().unwrap();
    assert_eq!(
        error.downcast_ref::<SessionError>(),
        Some(&SessionError::EventLimit)
    );
    let after = receiver.service.snapshot();
    assert_eq!(
        (
            before.accepted,
            before.calls,
            before.fifo_scopes,
            before.fifo_entries
        ),
        (
            after.accepted,
            after.calls,
            after.fifo_scopes,
            after.fifo_entries
        )
    );
    assert_eq!(receiver.calls, 0);
    assert!(receiver.receiver_incomplete);
    assert_eq!(
        receiver.report()["recovery"]["historical_control_events"],
        15
    );
    assert_eq!((counters.frames_received, counters.frames_sent), (16, 0));
    assert_eq!((sent.frames_sent, sent.frames_received), (16, 0));
    broker
        .with_time(|_, now| -> Result<()> {
            assert_eq!(
                object.handle(&receiver.incoming, &proof, now)?,
                unknown,
                "fresh Offer must not create this object transfer"
            );
            Ok(())
        })
        .unwrap();
    let reentry = receiver
        .handle(transfer, &mut object, &a, &cancel)
        .err()
        .unwrap();
    assert_eq!(
        reentry.downcast_ref::<SessionError>(),
        Some(&SessionError::StreamFenced)
    );
    let after_reentry = a.counters().unwrap();
    assert_eq!(
        (
            counters.envelope_bytes_received,
            counters.frames_received,
            counters.frames_sent
        ),
        (
            after_reentry.envelope_bytes_received,
            after_reentry.frames_received,
            after_reentry.frames_sent
        )
    );
}

#[test]
fn generation_or_original_deadline_change_after_preparation_refuses_before_connect() {
    use mvm_core::mailbox::transport::{
        AuthorizedRoute, ChannelLimits, InMemoryPeerTransport, PeerChannel, PeerTransport,
        TransportError,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Counted {
        inner: InMemoryPeerTransport,
        connects: AtomicUsize,
    }
    impl PeerTransport for Counted {
        fn connect(
            &self,
            route: &AuthorizedRoute,
            limits: ChannelLimits,
            cancel: &CancellationToken,
        ) -> std::result::Result<Box<dyn PeerChannel>, TransportError> {
            self.connects.fetch_add(1, Ordering::SeqCst);
            self.inner.connect(route, limits, cancel)
        }
    }
    // This is a canonical final-send authority seam, not an injected race in
    // the blocking consumer. A prepared retry is deliberately not a permit.
    for mode in ["generation", "deadline"] {
        let pair = pair(false);
        let broker = broker_context().unwrap();
        let mut caller = Session::new(&pair.sender, &broker).unwrap();
        let (mut pending, now) = broker
            .with_time(|_, now| -> Result<_> {
                let options = options(&mut caller, now)?;
                let mut pending = PendingCall::new(frame(0).encode()?, options, now);
                pending.register(
                    &mut caller.service,
                    &mut caller.tracker,
                    &caller.outgoing,
                    &caller.control,
                    &caller.peer_key,
                    now,
                )?;
                Ok((pending, now))
            })
            .unwrap();
        let transport = Counted {
            inner: InMemoryPeerTransport::default(),
            connects: AtomicUsize::new(0),
        };
        let route = pair
            .sender
            .outgoing
            .as_ref()
            .unwrap()
            .binding
            .claims
            .route_id;
        transport.inner.register_route(route).unwrap();
        let original = pending.encoded().unwrap().to_vec();
        let original_deadline = now
            .monotonic_ms
            .checked_add(pending.remaining(now).unwrap())
            .unwrap();
        let call = pending.call_handle().unwrap();
        let send = pending.send_handle();
        pending.start_send().unwrap();
        caller
            .service
            .send_call_prepared(
                &caller.outgoing,
                &call,
                pending.prepared().unwrap(),
                &transport,
                &CancellationToken::default(),
                now,
            )
            .unwrap();
        assert_eq!(
            transport.inner.drain(route).unwrap().as_slice(),
            std::slice::from_ref(&original)
        );
        let (signal, requests) = bounded_requests();
        signal.request_retry().unwrap();
        assert!(requests.take_request());
        let mut send_time = broker
            .with_time(|_, now| -> Result<_> {
                pending.prepare_retry(
                    &mut caller.service,
                    &mut caller.tracker,
                    &caller.outgoing,
                    &caller.control,
                    &caller.peer_key,
                    now,
                )?;
                Ok(now)
            })
            .unwrap();
        let expected = if mode == "generation" {
            caller
                .outgoing
                .replace_policy(pair.sender.outgoing.as_ref().unwrap().context().unwrap())
                .unwrap();
            ServiceError::Unauthorized
        } else {
            send_time.monotonic_ms = original_deadline;
            ServiceError::Expired
        };
        transport.connects.store(0, Ordering::SeqCst);
        let before = broker
            .with_time::<_, ServiceError>(|budget, _| budget.snapshot())
            .unwrap();
        pending.start_retry().unwrap();
        assert_eq!(
            caller.service.retry_call_prepared(
                &caller.outgoing,
                &call,
                pending.prepared().unwrap(),
                &transport,
                &CancellationToken::default(),
                send_time
            ),
            Err(expected),
            "{mode}"
        );
        assert_eq!(transport.connects.load(Ordering::SeqCst), 0, "{mode}");
        assert!(transport.inner.drain(route).unwrap().is_empty());
        assert_eq!(
            pending.send_invocations(),
            2,
            "invoked refusal is not refunded"
        );
        assert_eq!(pending.encoded().unwrap(), original);
        assert_eq!(pending.call_handle(), Some(call));
        assert_eq!(pending.send_handle(), send);
        let after = broker
            .with_time::<_, ServiceError>(|budget, _| budget.snapshot())
            .unwrap();
        assert_eq!(
            (
                before.queued_bytes,
                before.queued_messages,
                before.active_calls,
                before.identities
            ),
            (
                after.queued_bytes,
                after.queued_messages,
                after.active_calls,
                after.identities
            )
        );
    }
}
