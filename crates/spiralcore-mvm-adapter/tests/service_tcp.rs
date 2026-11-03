#![cfg(feature = "live")]
mod common;
#[path = "common/service.rs"]
mod fixture;

use mvm_contract::protocol::mailbox::*;
use mvm_core::mailbox::{
    encode_envelope,
    transport::{CancellationToken, RouteId},
};
use spiralcore_mvm_adapter::{
    live::{HandshakeRole, StreamTransport},
    receipt::{ReceiptObservation, ReceiptReturnConfig, ReceiptTracker},
    service::{CallStatus, ControlReserve, HostBudget, LocalMailboxService, ServiceLimits},
    service_sender::{self, MessageOptions},
};
use std::{
    net::{TcpListener, TcpStream},
    thread,
    time::{Duration, Instant},
};

fn budget() -> HostBudget {
    HostBudget::with_control(ServiceLimits::default(), ControlReserve::default()).unwrap()
}

#[test]
fn ciphertext_tcp_half_close_preserves_reverse_and_legacy_eof_stays_fatal() {
    use spiralcore_mvm_adapter::live::{
        LiveError, StreamCounters, read_ciphertext_frame, read_ciphertext_frame_or_eof,
        write_ciphertext_frame,
    };
    use std::{io::Write, net::Shutdown};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut socket, _) = listener.accept().unwrap();
    let cancel = CancellationToken::default();
    let mut read = StreamCounters::default();
    let mut sent = StreamCounters::default();
    let deadline = Instant::now() + Duration::from_secs(2);
    peer.write_all(&[0, 0, 0, 3, 0, 97, 0]).unwrap();
    peer.shutdown(Shutdown::Write).unwrap();
    assert_eq!(
        read_ciphertext_frame_or_eof(&mut socket, &cancel, deadline, &mut read).unwrap(),
        Some(vec![0, 97, 0])
    );
    assert!(
        read_ciphertext_frame_or_eof(&mut socket, &cancel, deadline, &mut read)
            .unwrap()
            .is_none()
    );
    write_ciphertext_frame(&mut socket, b"reverse\0", &cancel, deadline, &mut sent).unwrap();
    assert_eq!(
        read_ciphertext_frame(&mut peer, &cancel, deadline, &mut StreamCounters::default())
            .unwrap(),
        b"reverse\0"
    );
    assert_eq!(
        (
            read.framing_bytes_received,
            read.envelope_bytes_received,
            read.frames_received
        ),
        (4, 3, 1)
    );
    // The legacy endpoint reader still treats EOF as fatal, even after a frame.
    assert!(
        matches!(read_ciphertext_frame(&mut socket, &cancel, deadline, &mut read), Err(LiveError::Io(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof)
    );
    assert!(socket.write_all(b"must stay closed").is_err());
}

#[test]
fn ciphertext_tcp_optional_reader_rejects_truncation_lengths_cancel_and_deadline() {
    use spiralcore_mvm_adapter::live::{LiveError, StreamCounters, read_ciphertext_frame_or_eof};
    use std::{
        io::{Read, Write},
        net::Shutdown,
    };
    for bytes in [
        vec![0],
        vec![0, 0],
        vec![0, 0, 0],
        vec![0, 0, 0, 3, 42],
        vec![0; 4],
        262145u32.to_be_bytes().to_vec(),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut socket, _) = listener.accept().unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        peer.write_all(&bytes).unwrap();
        peer.shutdown(Shutdown::Write).unwrap();
        let mut counters = StreamCounters::default();
        let result = read_ciphertext_frame_or_eof(
            &mut socket,
            &CancellationToken::default(),
            Instant::now() + Duration::from_secs(2),
            &mut counters,
        );
        assert!(result.is_err(), "{bytes:?}");
        assert_eq!(counters.framing_bytes_received, bytes.len().min(4) as u64);
        assert_eq!(
            counters.envelope_bytes_received,
            bytes.len().saturating_sub(4) as u64
        );
        assert_eq!(counters.frames_received, 0);
        assert_eq!(peer.read(&mut [0]).unwrap(), 0);
    }
    for cancelled in [true, false] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut socket, _) = listener.accept().unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        let cancel = CancellationToken::default();
        if cancelled {
            cancel.cancel();
        }
        let mut counters = StreamCounters::default();
        let result = read_ciphertext_frame_or_eof(
            &mut socket,
            &cancel,
            Instant::now() + Duration::from_millis(25),
            &mut counters,
        );
        assert!(matches!(
            (cancelled, result),
            (true, Err(LiveError::Cancelled)) | (false, Err(LiveError::Deadline))
        ));
        assert_eq!(counters.framing_bytes_received, 0);
        assert_eq!(peer.read(&mut [0]).unwrap(), 0);
    }
}

#[test]
fn fatal_tcp_receive_fences_owning_transport_and_subsequent_send_without_new_bytes() {
    use spiralcore_mvm_adapter::{
        live::{self, LiveError, StreamCounters},
        service::ServiceError,
    };
    use std::{io::Read, net::Shutdown};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (socket, _) = listener.accept().unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    let worker = thread::spawn(move || {
        live::handshake_stream(
            &mut peer,
            HandshakeRole::Acceptor,
            &CancellationToken::default(),
            Instant::now() + Duration::from_secs(2),
            &mut StreamCounters::default(),
        )
        .unwrap();
        peer.shutdown(Shutdown::Write).unwrap();
        peer
    });
    let stream = StreamTransport::new(
        socket,
        common::config(MailboxMessageKind::Call)
            .binding
            .claims
            .route_id,
        Duration::from_secs(2),
    )
    .unwrap();
    let cancel = CancellationToken::default();
    stream.handshake(HandshakeRole::Initiator, &cancel).unwrap();
    let mut peer = worker.join().unwrap();
    assert!(
        matches!(stream.receive(&cancel, Instant::now() + Duration::from_secs(2)),
        Err(LiveError::Io(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof)
    );
    let before = serde_json::to_value(stream.counters().unwrap()).unwrap();
    assert!(matches!(
        stream.receive(&cancel, Instant::now() + Duration::from_secs(2)),
        Err(LiveError::Closed)
    ));
    let binding = fixture::binding(MailboxMessageKind::Call, true, false, true);
    let mut options =
        MessageOptions::fresh(fixture::content_type(), fixture::time(), 2000).unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    let prepared =
        service_sender::prepare(&binding, b"must be fenced", &options, fixture::time()).unwrap();
    assert_eq!(
        binding.send_prepared(&prepared, &stream, &cancel, fixture::time()),
        Err(ServiceError::ShuttingDown)
    );
    assert_eq!(
        serde_json::to_value(stream.counters().unwrap()).unwrap(),
        before
    );
    drop(stream);
    assert_eq!(peer.read(&mut [0]).unwrap(), 0);
}

#[test]
fn canonical_calls_and_encrypted_receipts_cross_tcp_without_recursive_traffic() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let expected = [
        common::random(65000, 83),
        b"\0private-service-tail\0\0".to_vec(),
    ]
    .concat();
    let server = thread::spawn(move || {
        let (socket, _) = listener.accept().unwrap();
        let stream = StreamTransport::new(
            socket,
            RouteId::from_bytes([30; 32]),
            Duration::from_secs(5),
        )
        .unwrap();
        let cancel = CancellationToken::default();
        stream.handshake(HandshakeRole::Acceptor, &cancel).unwrap();
        let budget = budget();
        let mut service = LocalMailboxService::new(budget.clone()).unwrap();
        let mut tracker = ReceiptTracker::new(budget.clone(), 8).unwrap();
        let incoming = fixture::binding(MailboxMessageKind::Call, true, false, false);
        let outgoing = fixture::binding(MailboxMessageKind::Reply, false, false, true);
        let control = fixture::binding(MailboxMessageKind::Cast, true, true, false);
        let returns = ReceiptReturnConfig::new(
            fixture::binding(MailboxMessageKind::Cast, false, true, true),
            budget,
            fixture::receipt_key(true).verifying_key(),
        )
        .unwrap();
        let original = stream
            .receive(&cancel, Instant::now() + Duration::from_secs(5))
            .unwrap();
        let (accepted, receipt) = returns
            .admit(&mut service, &incoming, &original, fixture::time())
            .unwrap();
        assert_eq!(accepted.receipt.outcome, DeliveryOutcome::AcceptedEphemeral);
        // No receipt identity or content type is sent outside canonical ciphertext.
        assert!(
            !receipt
                .encoded_bytes()
                .windows(16)
                .any(|w| w == b"mailbox.receipt\"")
        );
        receipt.send(&stream, &cancel, fixture::time()).unwrap();
        let request = service
            .receive(&incoming, fixture::time())
            .unwrap()
            .unwrap();
        let payload = request.payload().to_vec();
        let options = MessageOptions::for_reply(&request, fixture::time()).unwrap();
        let reply = service
            .reply_prepared(
                &outgoing,
                request.handle(),
                service_sender::request(&outgoing, &payload, &options, fixture::time()).unwrap(),
                fixture::time(),
            )
            .unwrap();
        let encoded = encode_envelope(reply.envelope()).unwrap();
        let send = tracker
            .register_send(
                &outgoing,
                &control,
                &fixture::receipt_key(false).verifying_key(),
                &encoded,
                fixture::time(),
            )
            .unwrap();
        outgoing
            .send_prepared(&reply, &stream, &cancel, fixture::time())
            .unwrap();
        let ack = stream
            .receive(&cancel, Instant::now() + Duration::from_secs(5))
            .unwrap();
        tracker
            .observe_receipt(&control, &ack, fixture::time())
            .unwrap();
        let ReceiptObservation::Verified(ack) = tracker
            .take_receipt(&outgoing, &send, fixture::time())
            .unwrap()
        else {
            panic!("missing application-reply admission")
        };
        assert!(
            service
                .acknowledge_reply(
                    &outgoing,
                    &ack,
                    &fixture::receipt_key(false).verifying_key(),
                    fixture::time()
                )
                .unwrap()
        );
        assert!(
            service
                .retry_reply_prepared(&incoming, &original, fixture::time())
                .unwrap()
                .is_none()
        );
        let counters = stream.counters().unwrap();
        assert_eq!((counters.frames_sent, counters.frames_received), (2, 2));
        stream.close().unwrap();
        (payload, counters)
    });
    let socket = TcpStream::connect(address).unwrap();
    let stream = StreamTransport::new(
        socket,
        RouteId::from_bytes([20; 32]),
        Duration::from_secs(5),
    )
    .unwrap();
    let cancel = CancellationToken::default();
    stream.handshake(HandshakeRole::Initiator, &cancel).unwrap();
    let budget = budget();
    let mut service = LocalMailboxService::new(budget.clone()).unwrap();
    let mut tracker = ReceiptTracker::new(budget.clone(), 8).unwrap();
    let outgoing = fixture::binding(MailboxMessageKind::Call, true, false, true);
    let incoming = fixture::binding(MailboxMessageKind::Reply, false, false, false);
    let control = fixture::binding(MailboxMessageKind::Cast, false, true, false);
    let returns = ReceiptReturnConfig::new(
        fixture::binding(MailboxMessageKind::Cast, true, true, true),
        budget,
        fixture::receipt_key(false).verifying_key(),
    )
    .unwrap();
    let mut options =
        MessageOptions::fresh(fixture::content_type(), fixture::time(), 2000).unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    let prepared =
        service_sender::prepare(&outgoing, &expected, &options, fixture::time()).unwrap();
    let encoded = encode_envelope(prepared.envelope()).unwrap();
    let call = service
        .register_call(&outgoing, &encoded, fixture::time())
        .unwrap();
    let send = tracker
        .register_send(
            &outgoing,
            &control,
            &fixture::receipt_key(true).verifying_key(),
            &encoded,
            fixture::time(),
        )
        .unwrap();
    outgoing
        .send_prepared(&prepared, &stream, &cancel, fixture::time())
        .unwrap();
    let receipt = stream
        .receive(&cancel, Instant::now() + Duration::from_secs(5))
        .unwrap();
    tracker
        .observe_receipt(&control, &receipt, fixture::time())
        .unwrap();
    let ReceiptObservation::Verified(accepted) = tracker
        .take_receipt(&outgoing, &send, fixture::time())
        .unwrap()
    else {
        panic!("missing call admission")
    };
    assert_eq!(accepted.receipt.outcome, DeliveryOutcome::AcceptedEphemeral);
    let reply = stream
        .receive(&cancel, Instant::now() + Duration::from_secs(5))
        .unwrap();
    let (accepted, ack) = returns
        .admit(&mut service, &incoming, &reply, fixture::time())
        .unwrap();
    assert_eq!(accepted.receipt.outcome, DeliveryOutcome::AcceptedEphemeral);
    tracker
        .observe_application_reply(&incoming, &reply, fixture::time())
        .unwrap();
    ack.send(&stream, &cancel, fixture::time()).unwrap();
    assert_eq!(
        service
            .poll_call(&outgoing, &call, fixture::time())
            .unwrap(),
        CallStatus::Replied(Some(expected.clone()))
    );
    let facts = tracker.facts(&outgoing, &send, fixture::time()).unwrap();
    assert!(facts.accepted && facts.application_replied);
    let counters = stream.counters().unwrap();
    assert_eq!((counters.frames_sent, counters.frames_received), (2, 2));
    let (echo, other) = server.join().unwrap();
    assert_eq!(echo, expected);
    assert_eq!(counters.envelope_bytes_sent, other.envelope_bytes_received);
    assert_eq!(counters.envelope_bytes_received, other.envelope_bytes_sent);
}

#[path = "service_tcp/idle.rs"]
mod idle;

#[path = "service_tcp/deadline.rs"]
mod deadline;
