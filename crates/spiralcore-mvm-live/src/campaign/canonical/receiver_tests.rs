use super::*;
use crate::config::{Arm, Pair, Plan, Stage};
use mvm_contract::protocol::mailbox::{ContentType, SignedDestinationReceipt};
use mvm_core::{
    crypto::aead::Key,
    mailbox::{
        InMemoryReplayProtector, OpenedMailboxEnvelope, open_verified_envelope,
        transport::PreparedMailboxEnvelope, verify_receipt,
    },
};
use spiralcore_mvm_adapter::{receipt::SendHandle, service::CallHandle};
use spiralcore_transfer::protocol::{Mode, Offer};
use std::{
    net::Shutdown,
    os::unix::net::UnixStream,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
};

const WATCHDOG: Duration = Duration::from_secs(5);

#[derive(Clone, Copy)]
enum WhileAwaitingAck {
    ExactReplay,
    DifferentCall,
    ExhaustSends,
    AckAtSendLimit,
    OlderControl,
    OlderControlLimit,
}

struct CountReads {
    socket: UnixStream,
    reads: Arc<AtomicUsize>,
}
impl Read for CountReads {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.socket.read(bytes)
    }
}
impl Write for CountReads {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.socket.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.socket.flush()
    }
}
impl SocketIo for CountReads {
    fn set_read_timeout(&self, value: Option<Duration>) -> io::Result<()> {
        SocketIo::set_read_timeout(&self.socket, value)
    }
    fn set_write_timeout(&self, value: Option<Duration>) -> io::Result<()> {
        SocketIo::set_write_timeout(&self.socket, value)
    }
    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.socket.shutdown(how)
    }
}

fn open_wire(material: &Material, wire: &[u8], now: ServiceTime) -> Result<OpenedMailboxEnvelope> {
    let mut context = material.context()?;
    context.now_unix_ms = now.unix_ms;
    Ok(open_verified_envelope(
        &decode_envelope(wire)?,
        &material.capability,
        &context,
        &Key::from_bytes(material.encryption),
        &VerifyingKey::from_bytes(&material.verification)?,
        &VerifyingKey::from_bytes(&material.authority)?,
        &mut InMemoryReplayProtector::default(),
    )?)
}

fn prepare_call(
    sender: &mut Session,
    request: &Frame,
) -> Result<(
    PreparedMailboxEnvelope,
    CallHandle,
    SendHandle,
    Vec<u8>,
    ServiceTime,
)> {
    let broker = sender.broker.clone();
    broker.with_time(|_, now| -> Result<_> {
        let payload = request.encode()?;
        let mut options = sender.ordering.next_for_binding(
            &sender.outgoing,
            ContentType::parse(spiralcore_mvm_adapter::CONTENT_TYPE, 1)?,
            now,
            now.unix_ms + 30_000,
            ISSUANCE_POLICY,
        )?;
        options.reply_to = sender.reply_to;
        let prepared = service_sender::prepare(&sender.outgoing, &payload, &options, now)?;
        let wire = encode_envelope(prepared.envelope())?;
        let call = sender.service.register_call(&sender.outgoing, &wire, now)?;
        let send = sender.tracker.register_send(
            &sender.outgoing,
            &sender.control,
            &sender.peer_key,
            &wire,
            now,
        )?;
        Ok((prepared, call, send, wire, now))
    })
}

struct SenderReport {
    original_call: Vec<u8>,
    reply_wire: Vec<u8>,
    reply: Frame,
    positive_reply_receipt: SignedDestinationReceipt,
    first_receipt_bytes: usize,
    duplicate_receipt_bytes: usize,
    sent_ack_bytes: usize,
    different_call_bytes: usize,
    reply_sends_observed: usize,
    second: Option<SecondExchange>,
    counters: StreamCounters,
}

struct SecondExchange {
    call_wire: Vec<u8>,
    reply_wire: Vec<u8>,
    reply: Frame,
    positive_receipt: SignedDestinationReceipt,
    receipt_bytes: usize,
}

struct Reentry {
    result: Result<Handled>,
    reads_before: usize,
    reads_after: usize,
    counters_before: StreamCounters,
    counters_after: StreamCounters,
}

struct ReceiverReport {
    session: Session,
    handled: Result<Handled>,
    first_handled: Option<Handled>,
    reentry: Option<Reentry>,
    counters: Result<StreamCounters>,
}

struct ReceiverGuard {
    sender_socket: UnixStream,
    receiver_socket: UnixStream,
    done: mpsc::Receiver<()>,
    worker: Option<std::thread::JoinHandle<ReceiverReport>>,
}

impl ReceiverGuard {
    fn stop(&self) {
        let _ = self.sender_socket.shutdown(Shutdown::Both);
        let _ = self.receiver_socket.shutdown(Shutdown::Both);
    }
    fn finish(mut self, succeeded: bool) -> (ReceiverReport, bool) {
        let completed = succeeded && self.done.recv_timeout(WATCHDOG).is_ok();
        self.stop();
        (
            self.worker
                .take()
                .unwrap()
                .join()
                .expect("receiver panicked"),
            completed,
        )
    }
}

impl Drop for ReceiverGuard {
    fn drop(&mut self) {
        self.stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct CaseReport {
    sender: Result<SenderReport>,
    receiver: ReceiverReport,
    completed: bool,
    _store: tempfile::TempDir,
}

fn receiver_case(mode: WhileAwaitingAck) -> CaseReport {
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
    let sender_broker = broker_context().unwrap();
    let mut sender = Session::new(&pair.sender, &sender_broker).unwrap();
    let receiver_broker = broker_context().unwrap();
    let mut receiver = Session::new(&pair.receiver, &receiver_broker).unwrap();
    let normal_material = pair.sender.incoming.as_ref().unwrap().clone();
    let control_material = pair
        .sender
        .service
        .as_ref()
        .unwrap()
        .incoming_control
        .clone();
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
    // A real Begin creates one tiny object attempt. The object core also has an
    // exact-frame ledger; byte-identical canonical replay/cache checks below
    // establish the receiver behavior without claiming an object-call counter.
    let request = Frame {
        kind: 1,
        transfer: [59; 32],
        seq: 0,
        body: Offer::cold(b"x", Mode::Raw).encode().unwrap(),
    };
    let transfer = request.transfer;
    let (socket, receiver_socket) = UnixStream::pair().unwrap();
    let sender_shutdown = socket.try_clone().unwrap();
    let receiver_shutdown = receiver_socket.try_clone().unwrap();
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
    let read_calls = Arc::new(AtomicUsize::new(0));
    let receiver_stream = StreamTransport::new(
        CountReads {
            socket: receiver_socket,
            reads: Arc::clone(&read_calls),
        },
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
    let worker = std::thread::spawn(move || {
        let cancel = CancellationToken::default();
        let mut handled = (|| -> Result<Handled> {
            receiver_stream.handshake(HandshakeRole::Acceptor, &cancel)?;
            receiver.handle(transfer, &mut object, &receiver_stream, &cancel)
        })();
        let mut first_handled = None;
        if matches!(
            mode,
            WhileAwaitingAck::OlderControl | WhileAwaitingAck::OlderControlLimit
        ) {
            handled = match handled {
                Ok(first) => {
                    first_handled = Some(first);
                    receiver.handle(transfer, &mut object, &receiver_stream, &cancel)
                }
                Err(error) => Err(error),
            };
        }
        let reentry = if handled.is_err() {
            let counters_before = receiver_stream.counters().unwrap();
            let reads_before = read_calls.load(Ordering::Relaxed);
            let result = receiver.handle(transfer, &mut object, &receiver_stream, &cancel);
            Some(Reentry {
                result,
                reads_before,
                reads_after: read_calls.load(Ordering::Relaxed),
                counters_before,
                counters_after: receiver_stream.counters().unwrap(),
            })
        } else {
            None
        };
        let counters = receiver_stream.counters().map_err(Into::into);
        // Wake the sender even though the guard owns raw shutdown handles.
        let _ = receiver_stream.close();
        let _ = done_tx.send(());
        ReceiverReport {
            session: receiver,
            handled,
            first_handled,
            reentry,
            counters,
        }
    });
    let guard = ReceiverGuard {
        sender_socket: sender_shutdown,
        receiver_socket: receiver_shutdown,
        done,
        worker: Some(worker),
    };
    let sender_result = (|| -> Result<SenderReport> {
        let cancel = CancellationToken::default();
        stream.handshake(HandshakeRole::Initiator, &cancel)?;
        let (prepared, call, send, original_call, now) = prepare_call(&mut sender, &request)?;
        sender.service.send_call_prepared(
            &sender.outgoing,
            &call,
            &prepared,
            &stream,
            &cancel,
            now,
        )?;
        let first_receipt = stream.receive(&cancel, Instant::now() + WATCHDOG)?;
        sender_broker.with_time(|_, now| -> Result<()> {
            sender
                .tracker
                .observe_receipt(&sender.control, &first_receipt, now)?;
            let ReceiptObservation::Verified(receipt) =
                sender.tracker.take_receipt(&sender.outgoing, &send, now)?
            else {
                return Err("missing original Call acceptance".into());
            };
            if receipt.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
                return Err("original Call was not accepted".into());
            }
            Ok(())
        })?;
        let reply_wire = stream.receive(&cancel, Instant::now() + WATCHDOG)?;
        let (positive, ack, reply) = sender_broker.with_time(|_, now| -> Result<_> {
            let opened = open_wire(&normal_material, &reply_wire, now)?;
            if opened.header().sequence.get() != 1 {
                return Err("first Reply skipped its FIFO cursor".into());
            }
            let (accepted, ack) =
                sender
                    .returns
                    .admit(&mut sender.service, &sender.incoming, &reply_wire, now)?;
            if accepted.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
                return Err("original Reply was not accepted".into());
            }
            sender
                .tracker
                .observe_application_reply(&sender.incoming, &reply_wire, now)?;
            let CallStatus::Replied(Some(payload)) =
                sender.service.poll_call(&sender.outgoing, &call, now)?
            else {
                return Err("missing original Begin Reply".into());
            };
            let reply = Frame::decode(&payload)?;
            if reply.kind != 101 || reply.transfer != request.transfer || reply.seq != 0 {
                return Err("real object Begin did not succeed".into());
            }
            Ok((accepted, ack, reply))
        })?;
        let mut duplicate_receipt_bytes = 0;
        let mut sent_ack_bytes = 0;
        let mut different_call_bytes = 0;
        let mut reply_sends_observed = 1;
        let mut second = None;
        match mode {
            WhileAwaitingAck::ExactReplay
            | WhileAwaitingAck::ExhaustSends
            | WhileAwaitingAck::AckAtSendLimit => {
                // Explicit test-driven retransmission of this same prepared
                // Call; no automatic retry, new options, or new registration.
                let repeats = if matches!(mode, WhileAwaitingAck::ExactReplay) {
                    1
                } else {
                    7
                };
                for _ in 0..repeats {
                    let now = sender_broker.with_time(|_, now| -> Result<_> { Ok(now) })?;
                    sender
                        .outgoing
                        .send_prepared(&prepared, &stream, &cancel, now)?;
                    // Active replay sends the exact cached Reply before its
                    // actual Duplicate Call receipt under the C8 contract.
                    let replayed = stream.receive(&cancel, Instant::now() + WATCHDOG)?;
                    if replayed != reply_wire {
                        return Err("receiver resealed or changed the cached Reply".into());
                    }
                    reply_sends_observed += 1;
                    let now = sender_broker.with_time(|_, now| -> Result<_> { Ok(now) })?;
                    if open_wire(&normal_material, &replayed, now)?
                        .header()
                        .sequence
                        .get()
                        != 1
                    {
                        return Err("cached replay allocated another Reply sequence".into());
                    }
                    let duplicate = stream.receive(&cancel, Instant::now() + WATCHDOG)?;
                    duplicate_receipt_bytes += duplicate.len();
                    sender_broker.with_time(|_, now| -> Result<()> {
                        let opened = open_wire(&control_material, &duplicate, now)?;
                        let signed = spiralcore_mvm_adapter::receipt::decode_destination_receipt(
                            opened.payload(),
                        )?;
                        verify_receipt(&signed, &sender.peer_key)?;
                        if signed.receipt.outcome != DeliveryOutcome::Duplicate {
                            return Err(
                                "replayed Call did not receive its actual Duplicate receipt".into(),
                            );
                        }
                        sender
                            .tracker
                            .observe_receipt(&sender.control, &duplicate, now)?;
                        if sender.tracker.take_receipt(&sender.outgoing, &send, now)?
                            != ReceiptObservation::Pending
                            || !sender.tracker.facts(&sender.outgoing, &send, now)?.accepted
                        {
                            return Err(
                                "lower-ranked duplicate replaced the original Call acceptance"
                                    .into(),
                            );
                        }
                        Ok(())
                    })?;
                }
                if matches!(mode, WhileAwaitingAck::ExhaustSends) {
                    drop(ack);
                    // Initial plus seven replayed Replies already reached the
                    // cap. This next repeat must not cause a ninth invocation.
                    let now = sender_broker.with_time(|_, now| -> Result<_> { Ok(now) })?;
                    sender
                        .outgoing
                        .send_prepared(&prepared, &stream, &cancel, now)?;
                } else {
                    sent_ack_bytes = ack.encoded_bytes().len();
                    let (permit, now) = sender_broker.with_time(|_, now| -> Result<_> {
                        Ok((ack.reserve_send(&cancel, now)?, now))
                    })?;
                    permit.send(&stream, &cancel, now)?;
                }
            }
            WhileAwaitingAck::DifferentCall => {
                drop(ack);
                let different = Frame {
                    kind: 6,
                    transfer: request.transfer,
                    seq: 1,
                    body: Vec::new(),
                };
                let (next, next_call, _next_send, next_wire, now) =
                    prepare_call(&mut sender, &different)?;
                if next.header().sequence.get() != 2
                    || next.header().message_id == prepared.header().message_id
                {
                    return Err(
                        "negative Call did not obtain a distinct next canonical identity".into(),
                    );
                }
                different_call_bytes = next_wire.len();
                sender.service.send_call_prepared(
                    &sender.outgoing,
                    &next_call,
                    &next,
                    &stream,
                    &cancel,
                    now,
                )?;
            }
            WhileAwaitingAck::OlderControl | WhileAwaitingAck::OlderControlLimit => {
                let old_ack_bytes = ack.encoded_bytes().len();
                let (permit, now) = sender_broker.with_time(|_, now| -> Result<_> {
                    Ok((ack.reserve_send(&cancel, now)?, now))
                })?;
                permit.send(&stream, &cancel, now)?;
                sent_ack_bytes += old_ack_bytes;
                let next_request = Frame {
                    kind: 6,
                    transfer: request.transfer,
                    seq: 1,
                    body: Vec::new(),
                };
                // Keep the original prepared control alive. The first local
                // Call has replied, so the unchanged signed grant one permits
                // this next Call; disjoint fields retain its normal identity.
                let (next, next_call, next_send, next_wire, now) =
                    sender_broker.with_time(|_, now| -> Result<_> {
                        let payload = next_request.encode()?;
                        let mut options = sender.ordering.next_for_binding(
                            &sender.outgoing,
                            ContentType::parse(spiralcore_mvm_adapter::CONTENT_TYPE, 1)?,
                            now,
                            now.unix_ms + 30_000,
                            ISSUANCE_POLICY,
                        )?;
                        options.reply_to = sender.reply_to;
                        let next =
                            service_sender::prepare(&sender.outgoing, &payload, &options, now)?;
                        let wire = encode_envelope(next.envelope())?;
                        let call = sender.service.register_call(&sender.outgoing, &wire, now)?;
                        let send = sender.tracker.register_send(
                            &sender.outgoing,
                            &sender.control,
                            &sender.peer_key,
                            &wire,
                            now,
                        )?;
                        Ok((next, call, send, wire, now))
                    })?;
                if next.header().sequence.get() != 2 {
                    return Err("second Call did not retain its increasing cursor".into());
                }
                sender.service.send_call_prepared(
                    &sender.outgoing,
                    &next_call,
                    &next,
                    &stream,
                    &cancel,
                    now,
                )?;
                let next_receipt = stream.receive(&cancel, Instant::now() + WATCHDOG)?;
                sender_broker.with_time(|_, now| -> Result<()> {
                    sender
                        .tracker
                        .observe_receipt(&sender.control, &next_receipt, now)?;
                    let ReceiptObservation::Verified(receipt) =
                        sender
                            .tracker
                            .take_receipt(&sender.outgoing, &next_send, now)?
                    else {
                        return Err("missing second Call receipt".into());
                    };
                    if receipt.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
                        return Err("second Call not accepted".into());
                    }
                    Ok(())
                })?;
                let next_reply_wire = stream.receive(&cancel, Instant::now() + WATCHDOG)?;
                let (next_positive, next_ack, next_reply) =
                    sender_broker.with_time(|_, now| -> Result<_> {
                        if open_wire(&normal_material, &next_reply_wire, now)?
                            .header()
                            .sequence
                            .get()
                            != 2
                        {
                            return Err("second Reply did not retain its FIFO cursor".into());
                        }
                        let (positive, next_ack) = sender.returns.admit(
                            &mut sender.service,
                            &sender.incoming,
                            &next_reply_wire,
                            now,
                        )?;
                        if positive.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
                            return Err("second Reply not accepted".into());
                        }
                        sender.tracker.observe_application_reply(
                            &sender.incoming,
                            &next_reply_wire,
                            now,
                        )?;
                        let CallStatus::Replied(Some(payload)) =
                            sender
                                .service
                                .poll_call(&sender.outgoing, &next_call, now)?
                        else {
                            return Err("missing second object Reply".into());
                        };
                        let frame = Frame::decode(&payload)?;
                        if frame.transfer != next_request.transfer
                            || frame.seq != 1
                            || frame.kind == 127
                        {
                            return Err("second object operation failed".into());
                        }
                        Ok((positive, next_ack, frame))
                    })?;
                reply_sends_observed += 1;
                if matches!(mode, WhileAwaitingAck::OlderControlLimit) {
                    // Call2 is event one. Fifteen exact older controls exhaust
                    // its remaining allowance, without a third application Call.
                    for _ in 0..15 {
                        let (permit, now) = sender_broker.with_time(|_, now| -> Result<_> {
                            Ok((ack.reserve_send(&cancel, now)?, now))
                        })?;
                        permit.send(&stream, &cancel, now)?;
                        sent_ack_bytes += old_ack_bytes;
                    }
                    drop(next_ack);
                    second = Some(SecondExchange {
                        call_wire: next_wire,
                        reply_wire: next_reply_wire,
                        reply: next_reply,
                        positive_receipt: next_positive,
                        receipt_bytes: next_receipt.len(),
                    });
                } else {
                    // Socket order makes the retained ACK1 precede the Call2 probe.
                    // An exact cached Reply2 afterward proves ACK1 did not release
                    // or substitute the current cache, without timing assumptions.
                    let (permit, now) = sender_broker.with_time(|_, now| -> Result<_> {
                        Ok((ack.reserve_send(&cancel, now)?, now))
                    })?;
                    permit.send(&stream, &cancel, now)?;
                    sent_ack_bytes += old_ack_bytes;
                    let now = sender_broker.with_time(|_, now| -> Result<_> { Ok(now) })?;
                    sender
                        .outgoing
                        .send_prepared(&next, &stream, &cancel, now)?;
                    if stream.receive(&cancel, Instant::now() + WATCHDOG)? != next_reply_wire {
                        return Err(
                            "old control receipt cleared or changed current cached Reply".into(),
                        );
                    }
                    reply_sends_observed += 1;
                    let duplicate = stream.receive(&cancel, Instant::now() + WATCHDOG)?;
                    sender_broker.with_time(|_, now| -> Result<()> {
                        let opened = open_wire(&control_material, &duplicate, now)?;
                        let signed = spiralcore_mvm_adapter::receipt::decode_destination_receipt(
                            opened.payload(),
                        )?;
                        verify_receipt(&signed, &sender.peer_key)?;
                        if signed.receipt.outcome != DeliveryOutcome::Duplicate {
                            return Err("current cache probe lacked Duplicate receipt".into());
                        }
                        sender
                            .tracker
                            .observe_receipt(&sender.control, &duplicate, now)?;
                        if sender
                            .tracker
                            .take_receipt(&sender.outgoing, &next_send, now)?
                            != ReceiptObservation::Pending
                        {
                            return Err("lower-ranked current duplicate replaced acceptance".into());
                        }
                        Ok(())
                    })?;
                    sent_ack_bytes += next_ack.encoded_bytes().len();
                    let (permit, now) = sender_broker.with_time(|_, now| -> Result<_> {
                        Ok((next_ack.reserve_send(&cancel, now)?, now))
                    })?;
                    permit.send(&stream, &cancel, now)?;
                    second = Some(SecondExchange {
                        call_wire: next_wire,
                        reply_wire: next_reply_wire,
                        reply: next_reply,
                        positive_receipt: next_positive,
                        receipt_bytes: next_receipt.len() + duplicate.len(),
                    });
                }
            }
        }
        Ok(SenderReport {
            original_call,
            reply_wire,
            reply,
            positive_reply_receipt: positive,
            first_receipt_bytes: first_receipt.len(),
            duplicate_receipt_bytes,
            sent_ack_bytes,
            different_call_bytes,
            reply_sends_observed,
            second,
            counters: stream.counters()?,
        })
    })();
    let (receiver, completed) = guard.finish(sender_result.is_ok());
    CaseReport {
        sender: sender_result,
        receiver,
        completed,
        _store: store,
    }
}

#[test]
fn receiver_replays_the_exact_cached_reply_for_same_call_before_its_positive_receipt() {
    let mut case = receiver_case(WhileAwaitingAck::ExactReplay);
    let handled = case
        .receiver
        .handled
        .expect("receiver must recognize the same Call while awaiting its Reply receipt");
    let sender = case
        .sender
        .expect("sender must receive exact cached Reply then Duplicate");
    assert!(case.completed);
    assert_eq!(handled.reply, sender.reply);
    assert_eq!(case.receiver.session.calls, 1);
    let receiver_counts = case.receiver.counters.unwrap();
    assert_eq!(
        (sender.counters.frames_sent, sender.counters.frames_received),
        (3, 4)
    );
    assert_eq!(
        (receiver_counts.frames_sent, receiver_counts.frames_received),
        (4, 3)
    );
    assert_eq!(
        sender.counters.envelope_bytes_sent,
        (2 * sender.original_call.len() + sender.sent_ack_bytes) as u64
    );
    assert_eq!(
        sender.counters.envelope_bytes_received,
        (sender.first_receipt_bytes + sender.duplicate_receipt_bytes + 2 * sender.reply_wire.len())
            as u64
    );
    assert_eq!(
        sender.counters.envelope_bytes_sent,
        receiver_counts.envelope_bytes_received
    );
    assert_eq!(
        sender.counters.envelope_bytes_received,
        receiver_counts.envelope_bytes_sent
    );
    assert_eq!(sender.counters.framing_bytes_sent, 8 + 3 * 4);
    assert_eq!(sender.counters.framing_bytes_received, 8 + 4 * 4);
    assert_eq!(receiver_counts.framing_bytes_sent, 8 + 4 * 4);
    assert_eq!(receiver_counts.framing_bytes_received, 8 + 3 * 4);
    let receiver = &mut case.receiver.session;
    let broker = receiver.broker.clone();
    broker
        .with_time(|_, now| -> Result<()> {
            assert!(
                receiver
                    .service
                    .retry_reply_prepared(&receiver.incoming, &sender.original_call, now)?
                    .is_none()
            );
            assert!(!receiver.service.acknowledge_reply(
                &receiver.outgoing,
                &sender.positive_reply_receipt,
                &receiver.peer_key,
                now
            )?);
            assert!(receiver.service.receive(&receiver.incoming, now)?.is_none());
            Ok(())
        })
        .unwrap();
}

#[test]
fn different_call_while_awaiting_reply_receipt_is_rejected_without_admission_or_cache_loss() {
    let mut case = receiver_case(WhileAwaitingAck::DifferentCall);
    let sender = case
        .sender
        .expect("negative input must remain a valid canonical next Call");
    assert!(case.completed);
    assert!(case.receiver.handled.is_err());
    assert_fenced_reentry(&case.receiver);
    let receiver = &mut case.receiver.session;
    assert_eq!(receiver.calls, 0);
    assert_eq!(receiver.service.snapshot().accepted, 1);
    let broker = receiver.broker.clone();
    broker
        .with_time(|_, now| -> Result<()> {
            assert!(receiver.service.receive(&receiver.incoming, now)?.is_none());
            let cached = receiver
                .service
                .retry_reply_prepared(&receiver.incoming, &sender.original_call, now)?
                .expect("different Call discarded original Reply cache");
            assert_eq!(encode_envelope(cached.envelope())?, sender.reply_wire);
            assert_eq!(cached.header().sequence.get(), 1);
            Ok(())
        })
        .unwrap();
    let counts = case.receiver.counters.unwrap();
    assert_eq!(
        (sender.counters.frames_sent, sender.counters.frames_received),
        (2, 2)
    );
    assert_eq!((counts.frames_sent, counts.frames_received), (2, 2));
    assert_eq!(
        sender.counters.envelope_bytes_sent,
        (sender.original_call.len() + sender.different_call_bytes) as u64
    );
    assert_eq!(
        sender.counters.envelope_bytes_received,
        (sender.first_receipt_bytes + sender.reply_wire.len()) as u64
    );
    assert_eq!(
        sender.counters.envelope_bytes_sent,
        counts.envelope_bytes_received
    );
    assert_eq!(
        sender.counters.envelope_bytes_received,
        counts.envelope_bytes_sent
    );
    assert_eq!(sender.counters.framing_bytes_sent, 8 + 2 * 4);
    assert_eq!(sender.counters.framing_bytes_received, 8 + 2 * 4);
}

fn assert_fenced_reentry(receiver: &ReceiverReport) {
    assert!(receiver.session.receiver_incomplete);
    let reentry = receiver
        .reentry
        .as_ref()
        .expect("missing second handle attempt");
    let error = reentry
        .result
        .as_ref()
        .err()
        .expect("incomplete receiver accepted another invocation");
    assert_eq!(
        error.downcast_ref::<SessionError>(),
        Some(&SessionError::StreamFenced)
    );
    assert_eq!(
        reentry.reads_after, reentry.reads_before,
        "fenced invocation called SocketIo::read"
    );
    assert_eq!(
        serde_json::to_value(reentry.counters_after).unwrap(),
        serde_json::to_value(reentry.counters_before).unwrap()
    );
}

#[test]
fn receiver_cached_reply_replay_stops_at_eight_invocations_and_fences_without_cache_loss() {
    let mut case = receiver_case(WhileAwaitingAck::ExhaustSends);
    let sender = case
        .sender
        .expect("eight bounded Reply transmissions must be observed");
    assert!(case.completed);
    let error = case
        .receiver
        .handled
        .as_ref()
        .err()
        .expect("ninth Reply invocation was not refused");
    assert_eq!(
        error.downcast_ref::<SessionError>(),
        Some(&SessionError::SendLimit)
    );
    assert_fenced_reentry(&case.receiver);
    assert_eq!(sender.reply_sends_observed, 8);
    let receiver = &mut case.receiver.session;
    assert_eq!(receiver.calls, 0);
    assert_eq!(receiver.service.snapshot().accepted, 1);
    let broker = receiver.broker.clone();
    broker
        .with_time(|_, now| -> Result<()> {
            assert!(receiver.service.receive(&receiver.incoming, now)?.is_none());
            let cached = receiver
                .service
                .retry_reply_prepared(&receiver.incoming, &sender.original_call, now)?
                .expect("send limit lost original cached Reply");
            assert_eq!(encode_envelope(cached.envelope())?, sender.reply_wire);
            assert_eq!(cached.header().sequence.get(), 1);
            Ok(())
        })
        .unwrap();
    let counts = case.receiver.counters.unwrap();
    // Original Call plus eight repeats; only seven repeats receive a control
    // receipt and cached Reply. The refused ninth send emits no extra frame.
    assert_eq!(
        (sender.counters.frames_sent, sender.counters.frames_received),
        (9, 16)
    );
    assert_eq!((counts.frames_sent, counts.frames_received), (16, 9));
    assert_eq!(
        sender.counters.envelope_bytes_sent,
        9 * sender.original_call.len() as u64
    );
    assert_eq!(
        sender.counters.envelope_bytes_received,
        (sender.first_receipt_bytes + sender.duplicate_receipt_bytes + 8 * sender.reply_wire.len())
            as u64
    );
    assert_eq!(
        sender.counters.envelope_bytes_sent,
        counts.envelope_bytes_received
    );
    assert_eq!(
        sender.counters.envelope_bytes_received,
        counts.envelope_bytes_sent
    );
    assert_eq!(sender.counters.framing_bytes_sent, 8 + 9 * 4);
    assert_eq!(sender.counters.framing_bytes_received, 8 + 16 * 4);
    assert_eq!(counts.framing_bytes_sent, 8 + 16 * 4);
    assert_eq!(counts.framing_bytes_received, 8 + 9 * 4);
}

#[test]
fn older_duplicate_reply_receipt_cannot_release_the_current_reply_cache() {
    let mut case = receiver_case(WhileAwaitingAck::OlderControl);
    let sender = case
        .sender
        .expect("two sequential Calls and retained control must exchange canonically");
    assert!(case.completed);
    let first = case
        .receiver
        .first_handled
        .take()
        .expect("missing first completed Call");
    let handled = case
        .receiver
        .handled
        .expect("retained older receipt incorrectly refused current Call");
    let second = sender
        .second
        .as_ref()
        .expect("missing current exchange report");
    assert_eq!(first.reply, sender.reply);
    assert_eq!(handled.reply, second.reply);
    assert_eq!(case.receiver.session.calls, 2);
    assert!(!case.receiver.session.receiver_incomplete);
    assert!(case.receiver.reentry.is_none());
    assert_eq!(sender.reply_sends_observed, 3);
    let receiver = &mut case.receiver.session;
    let broker = receiver.broker.clone();
    broker
        .with_time(|_, now| -> Result<()> {
            for (call, receipt) in [
                (&sender.original_call, &sender.positive_reply_receipt),
                (&second.call_wire, &second.positive_receipt),
            ] {
                assert!(
                    receiver
                        .service
                        .retry_reply_prepared(&receiver.incoming, call, now)?
                        .is_none()
                );
                assert!(!receiver.service.acknowledge_reply(
                    &receiver.outgoing,
                    receipt,
                    &receiver.peer_key,
                    now
                )?);
            }
            assert!(receiver.service.receive(&receiver.incoming, now)?.is_none());
            Ok(())
        })
        .unwrap();
    let counts = case.receiver.counters.unwrap();
    assert_eq!(
        (sender.counters.frames_sent, sender.counters.frames_received),
        (6, 6)
    );
    assert_eq!((counts.frames_sent, counts.frames_received), (6, 6));
    assert_eq!(
        sender.counters.envelope_bytes_sent,
        (sender.original_call.len() + 2 * second.call_wire.len() + sender.sent_ack_bytes) as u64
    );
    assert_eq!(
        sender.counters.envelope_bytes_received,
        (sender.first_receipt_bytes
            + sender.reply_wire.len()
            + second.receipt_bytes
            + 2 * second.reply_wire.len()) as u64
    );
    assert_eq!(
        sender.counters.envelope_bytes_sent,
        counts.envelope_bytes_received
    );
    assert_eq!(
        sender.counters.envelope_bytes_received,
        counts.envelope_bytes_sent
    );
    assert_eq!(sender.counters.framing_bytes_sent, 8 + 6 * 4);
    assert_eq!(sender.counters.framing_bytes_received, 8 + 6 * 4);
    assert_eq!(counts.framing_bytes_sent, 8 + 6 * 4);
    assert_eq!(counts.framing_bytes_received, 8 + 6 * 4);
}

#[test]
fn receiver_boundary_positive_ack_at_eight_reply_sends_still_completes() {
    let mut case = receiver_case(WhileAwaitingAck::AckAtSendLimit);
    let sender = case
        .sender
        .expect("eight sends and a final canonical ACK must exchange");
    let handled = case
        .receiver
        .handled
        .expect("send ceiling must not reject the actual positive ACK");
    assert!(case.completed);
    assert_eq!(handled.reply, sender.reply);
    assert_eq!(sender.reply_sends_observed, 8);
    let receiver = &mut case.receiver.session;
    assert_eq!(receiver.calls, 1);
    assert!(!receiver.receiver_incomplete);
    assert!(case.receiver.reentry.is_none());
    let broker = receiver.broker.clone();
    broker
        .with_time(|_, now| -> Result<()> {
            assert!(
                receiver
                    .service
                    .retry_reply_prepared(&receiver.incoming, &sender.original_call, now)?
                    .is_none()
            );
            assert!(!receiver.service.acknowledge_reply(
                &receiver.outgoing,
                &sender.positive_reply_receipt,
                &receiver.peer_key,
                now
            )?);
            assert!(receiver.service.receive(&receiver.incoming, now)?.is_none());
            Ok(())
        })
        .unwrap();
    let counts = case.receiver.counters.unwrap();
    assert_eq!(
        (sender.counters.frames_sent, sender.counters.frames_received),
        (9, 16)
    );
    assert_eq!((counts.frames_sent, counts.frames_received), (16, 9));
    assert_eq!(
        sender.counters.envelope_bytes_sent,
        (8 * sender.original_call.len() + sender.sent_ack_bytes) as u64
    );
    assert_eq!(
        sender.counters.envelope_bytes_received,
        (sender.first_receipt_bytes + sender.duplicate_receipt_bytes + 8 * sender.reply_wire.len())
            as u64
    );
    assert_eq!(
        sender.counters.envelope_bytes_sent,
        counts.envelope_bytes_received
    );
    assert_eq!(
        sender.counters.envelope_bytes_received,
        counts.envelope_bytes_sent
    );
    assert_eq!(sender.counters.framing_bytes_sent, 8 + 9 * 4);
    assert_eq!(sender.counters.framing_bytes_received, 8 + 16 * 4);
    assert_eq!(counts.framing_bytes_sent, 8 + 16 * 4);
    assert_eq!(counts.framing_bytes_received, 8 + 9 * 4);
}

#[test]
fn receiver_boundary_event_sixteen_from_older_controls_fences_and_keeps_current_cache() {
    let mut case = receiver_case(WhileAwaitingAck::OlderControlLimit);
    let sender = case
        .sender
        .expect("two Calls and fifteen bounded older controls must transmit");
    assert!(case.completed);
    let error = case
        .receiver
        .handled
        .as_ref()
        .err()
        .expect("sixteen-event allowance was not enforced");
    assert_eq!(
        error.downcast_ref::<SessionError>(),
        Some(&SessionError::EventLimit)
    );
    assert_fenced_reentry(&case.receiver);
    let first = case
        .receiver
        .first_handled
        .as_ref()
        .expect("first Call must complete before control replay");
    assert_eq!(first.reply, sender.reply);
    let second = sender
        .second
        .as_ref()
        .expect("missing unacknowledged current Reply");
    assert_eq!(sender.reply_sends_observed, 2);
    let receiver = &mut case.receiver.session;
    assert_eq!(receiver.calls, 1);
    assert_eq!(receiver.service.snapshot().accepted, 2);
    let broker = receiver.broker.clone();
    broker
        .with_time(|_, now| -> Result<()> {
            assert!(
                receiver
                    .service
                    .retry_reply_prepared(&receiver.incoming, &sender.original_call, now)?
                    .is_none()
            );
            assert!(!receiver.service.acknowledge_reply(
                &receiver.outgoing,
                &sender.positive_reply_receipt,
                &receiver.peer_key,
                now
            )?);
            let cached = receiver
                .service
                .retry_reply_prepared(&receiver.incoming, &second.call_wire, now)?
                .expect("older control saturation discarded current Reply");
            assert_eq!(encode_envelope(cached.envelope())?, second.reply_wire);
            assert_eq!(cached.header().sequence.get(), 2);
            assert!(receiver.service.receive(&receiver.incoming, now)?.is_none());
            Ok(())
        })
        .unwrap();
    let counts = case.receiver.counters.unwrap();
    // First Call + ACK1 + Call2 + fifteen repeated ACK1 frames. No third Call,
    // current ACK, extra receipt-control response, or cached replay is sent.
    assert_eq!(
        (sender.counters.frames_sent, sender.counters.frames_received),
        (18, 4)
    );
    assert_eq!((counts.frames_sent, counts.frames_received), (4, 18));
    assert_eq!(
        sender.counters.envelope_bytes_sent,
        (sender.original_call.len() + second.call_wire.len() + sender.sent_ack_bytes) as u64
    );
    assert_eq!(
        sender.counters.envelope_bytes_received,
        (sender.first_receipt_bytes
            + sender.reply_wire.len()
            + second.receipt_bytes
            + second.reply_wire.len()) as u64
    );
    assert_eq!(
        sender.counters.envelope_bytes_sent,
        counts.envelope_bytes_received
    );
    assert_eq!(
        sender.counters.envelope_bytes_received,
        counts.envelope_bytes_sent
    );
    assert_eq!(sender.counters.framing_bytes_sent, 8 + 18 * 4);
    assert_eq!(sender.counters.framing_bytes_received, 8 + 4 * 4);
    assert_eq!(counts.framing_bytes_sent, 8 + 4 * 4);
    assert_eq!(counts.framing_bytes_received, 8 + 18 * 4);
}
