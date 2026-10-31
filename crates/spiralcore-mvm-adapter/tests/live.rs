#![cfg(feature = "live")]
mod common;

use mvm_contract::protocol::mailbox::MailboxMessageKind;
use mvm_core::mailbox::{
    sign_capability,
    transport::{CancellationToken, sign_transport_binding},
};
use spiralcore_mvm_adapter::{
    Direction, DirectionConfig,
    live::{HandshakeRole, StreamTransport},
};
use spiralcore_transfer::protocol::Frame;
use std::{
    os::unix::net::UnixStream,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn live_config(kind: MailboxMessageKind, sending: bool) -> DirectionConfig {
    let mut config = common::config(kind);
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let authority = ed25519_dalek::SigningKey::from_bytes(&[22; 32]);
    config.capability.grant.not_before_unix_ms = now - 10_000;
    config.capability.grant.expires_at_unix_ms = now + 60_000;
    config.capability = sign_capability(config.capability.grant, &authority).unwrap();
    config.binding.claims.not_before_unix_ms = now - 10_000;
    config.binding.claims.expires_at_unix_ms = now + 60_000;
    config.binding = sign_transport_binding(config.binding.claims, &authority).unwrap();
    let mut context = config.policy.snapshot().unwrap();
    context.now_unix_ms = now;
    config.policy.replace(context).unwrap();
    if !sending {
        config.signing_key = None;
    }
    config
}

#[test]
fn live_open_enforces_the_signed_incoming_queue_budget() {
    use mvm_core::mailbox::{InMemoryReplayProtector, decode_envelope, open_verified_envelope};
    let mut config = live_config(MailboxMessageKind::Call, true);
    config.capability.grant.max_queue_contribution_bytes = 4_096;
    config.capability = sign_capability(
        config.capability.grant,
        &ed25519_dalek::SigningKey::from_bytes(&[22; 32]),
    )
    .unwrap();
    let frame = Frame {
        kind: 5,
        transfer: [95; 32],
        seq: 0,
        body: vec![42; 8_192],
    };
    let payload = frame.encode().unwrap();
    let context = config.policy.snapshot().unwrap();
    // Seal through canonical MVM directly so the adapter's outbound budget
    // guard cannot mask the missing inbound admission check.
    let wire = common::sealed_payload_at(
        &config,
        &payload,
        context.now_unix_ms - 1_000,
        context.now_unix_ms + 30_000,
    );
    assert!(payload.len() <= config.capability.grant.max_payload_bytes as usize);
    assert!(wire.len() > config.capability.grant.max_queue_contribution_bytes as usize);
    assert!(wire.len() <= spiralcore_mvm_adapter::MAX_ENVELOPE_BYTES);
    let opened = open_verified_envelope(
        &decode_envelope(&wire).unwrap(),
        &config.capability,
        &context,
        &config.encryption_key,
        &config.sender_verifying_key,
        &config.authority_key,
        &mut InMemoryReplayProtector::default(),
    )
    .expect("the fixture must pass canonical authentication and payload bounds");
    assert_eq!(opened.payload(), payload);
    config.signing_key = None;
    let mut incoming = Direction::new(config).unwrap();
    assert!(matches!(
        incoming.open_live(&wire),
        Err(spiralcore_mvm_adapter::live::LiveError::Adapter(
            spiralcore_mvm_adapter::Error::Limit
        ))
    ));
    assert_eq!(incoming.counters().frame_bytes_opened, 0);
}

#[test]
fn canonical_call_and_verified_reply_cross_real_socket_io() {
    let (client_socket, server_socket) = UnixStream::pair().unwrap();
    let mut outgoing = Direction::new(live_config(MailboxMessageKind::Call, true)).unwrap();
    let mut replies = Direction::new(live_config(MailboxMessageKind::Reply, false)).unwrap();
    let client =
        StreamTransport::new(client_socket, outgoing.route_id(), Duration::from_secs(2)).unwrap();
    let server = std::thread::spawn(move || {
        let mut incoming = Direction::new(live_config(MailboxMessageKind::Call, false)).unwrap();
        let mut outgoing_reply =
            Direction::new(live_config(MailboxMessageKind::Reply, true)).unwrap();
        let stream = StreamTransport::new(
            server_socket,
            outgoing_reply.route_id(),
            Duration::from_secs(2),
        )
        .unwrap();
        let cancel = CancellationToken::default();
        stream.handshake(HandshakeRole::Acceptor, &cancel).unwrap();
        let wire = stream
            .receive(&cancel, Instant::now() + Duration::from_secs(2))
            .unwrap();
        let verified = incoming.open_live(&wire).unwrap();
        let reply = verified.frame().reply(105, b"canonical reply".to_vec());
        outgoing_reply
            .reply_to_verified(&verified, &reply, &stream, &cancel)
            .unwrap();
        stream.counters().unwrap()
    });
    let cancel = CancellationToken::default();
    client.handshake(HandshakeRole::Initiator, &cancel).unwrap();
    let request = Frame {
        kind: 5,
        transfer: [71; 32],
        seq: 9,
        body: Vec::new(),
    };
    let call = outgoing
        .send_call(
            &request,
            &replies,
            &client,
            &cancel,
            Instant::now() + Duration::from_secs(30),
        )
        .unwrap();
    let wire = client
        .receive(&cancel, Instant::now() + Duration::from_secs(2))
        .unwrap();
    let verified = replies.open_live(&wire).unwrap();
    let response = replies.accept_reply(call, verified).unwrap();
    assert_eq!(response, request.reply(105, b"canonical reply".to_vec()));
    let sent = client.counters().unwrap();
    let received = server.join().unwrap();
    assert_eq!(sent.frames_sent, 1);
    assert_eq!(sent.frames_received, 1);
    assert_eq!(sent.envelope_bytes_sent, received.envelope_bytes_received);
    assert_eq!(sent.envelope_bytes_received, received.envelope_bytes_sent);
    assert_eq!(sent.framing_bytes_sent, 12);
    assert_eq!(sent.framing_bytes_received, 12);
}

#[test]
fn live_dispatch_keeps_authenticated_request_available_for_reply() {
    use mvm_core::mailbox::transport::InMemoryPeerTransport;
    use spiralcore_transfer::protocol::{Mode, Offer};
    let mut direction = Direction::new(live_config(MailboxMessageKind::Call, true)).unwrap();
    let transport = InMemoryPeerTransport::default();
    transport.register_route(direction.route_id()).unwrap();
    let request = Frame {
        kind: 1,
        transfer: [83; 32],
        seq: 0,
        body: Offer::cold(&[], Mode::Raw).encode().unwrap(),
    };
    direction
        .send(&request, &transport, &CancellationToken::default())
        .unwrap();
    let wire = transport
        .drain(direction.route_id())
        .unwrap()
        .pop()
        .unwrap();
    let verified = direction.open_live(&wire).unwrap();
    let store = common::Store::new();
    let mut endpoint = store.open(direction.authority());
    let reply = verified.dispatch_live(&mut endpoint, 0).unwrap();
    assert_eq!(reply.kind, 101);
    assert_eq!(reply.transfer, request.transfer);
    assert_eq!(verified.frame(), &request);
    assert!(
        endpoint
            .read_object(verified.context(), spiralcore_transfer::object_id(&[]))
            .is_err()
    );
}

#[test]
fn cancellation_before_handshake_closes_the_owned_socket() {
    use std::io::Read;
    let (socket, mut peer) = UnixStream::pair().unwrap();
    peer.set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let config = live_config(MailboxMessageKind::Call, true);
    let stream = StreamTransport::new(
        socket,
        config.binding.claims.route_id,
        Duration::from_secs(1),
    )
    .unwrap();
    let cancel = CancellationToken::default();
    cancel.cancel();
    assert!(matches!(
        stream.handshake(HandshakeRole::Initiator, &cancel),
        Err(spiralcore_mvm_adapter::live::LiveError::Cancelled)
    ));
    let mut byte = [0; 1];
    assert_eq!(
        peer.read(&mut byte).unwrap(),
        0,
        "cancellation must close, not leave an idle connection"
    );
}

#[test]
fn live_sender_supports_bounded_clock_lag_and_caps_signed_call_expiry() {
    use mvm_core::mailbox::{InMemoryReplayProtector, decode_envelope, open_verified_envelope};
    let (socket, peer) = UnixStream::pair().unwrap();
    let mut sender = Direction::new(live_config(MailboxMessageKind::Call, true)).unwrap();
    let replies = Direction::new(live_config(MailboxMessageKind::Reply, false)).unwrap();
    let verifier = live_config(MailboxMessageKind::Call, false);
    let route = sender.route_id();
    let peer = std::thread::spawn(move || {
        let stream = StreamTransport::new(peer, route, Duration::from_secs(2)).unwrap();
        let cancel = CancellationToken::default();
        stream.handshake(HandshakeRole::Acceptor, &cancel).unwrap();
        stream
            .receive(&cancel, Instant::now() + Duration::from_secs(2))
            .unwrap()
    });
    let stream = StreamTransport::new(socket, route, Duration::from_secs(2)).unwrap();
    let cancel = CancellationToken::default();
    stream.handshake(HandshakeRole::Initiator, &cancel).unwrap();
    let request = Frame {
        kind: 5,
        transfer: [91; 32],
        seq: 0,
        body: Vec::new(),
    };
    let started = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let _call = sender
        .send_call(
            &request,
            &replies,
            &stream,
            &cancel,
            Instant::now() + Duration::from_secs(20),
        )
        .unwrap();
    let wire = peer.join().unwrap();
    let envelope = decode_envelope(&wire).unwrap();
    let mut context = verifier.policy.snapshot().unwrap();
    context.now_unix_ms = started - 4_000; // A peer within the registered 5 s uncertainty.
    let opened = open_verified_envelope(
        &envelope,
        &verifier.capability,
        &context,
        &verifier.encryption_key,
        &verifier.sender_verifying_key,
        &verifier.authority_key,
        &mut InMemoryReplayProtector::default(),
    )
    .expect("bounded peer clock lag must not reject a fresh live call");
    assert_eq!(opened.payload(), request.encode().unwrap());
    assert!(opened.header().issued_at_unix_ms <= context.now_unix_ms);
    assert!(
        opened.header().expires_at_unix_ms <= started + 20_100,
        "signed expiry must respect the requested call deadline, not reset to 30 seconds"
    );
}

#[test]
fn live_open_refuses_a_grant_inside_the_clock_uncertainty_margin() {
    use mvm_core::mailbox::transport::InMemoryPeerTransport;
    let mut config = live_config(MailboxMessageKind::Call, true);
    let now = config.policy.snapshot().unwrap().now_unix_ms;
    config.capability.grant.expires_at_unix_ms = now + 4_000;
    config.capability = sign_capability(
        config.capability.grant,
        &ed25519_dalek::SigningKey::from_bytes(&[22; 32]),
    )
    .unwrap();
    let mut direction = Direction::new(config).unwrap();
    let transport = InMemoryPeerTransport::default();
    transport.register_route(direction.route_id()).unwrap();
    let frame = Frame {
        kind: 5,
        transfer: [92; 32],
        seq: 0,
        body: Vec::new(),
    };
    direction
        .send(&frame, &transport, &CancellationToken::default())
        .unwrap();
    let wire = transport
        .drain(direction.route_id())
        .unwrap()
        .pop()
        .unwrap();
    assert!(
        direction.open_live(&wire).is_err(),
        "live authorization must retain more than the 5-second clock margin"
    );
}

#[test]
fn ciphertext_framing_handles_split_reads_and_rejects_bad_lengths_and_truncation() {
    use spiralcore_mvm_adapter::live::{LiveError, StreamCounters, read_ciphertext_frame};
    use std::io::Write;
    let cancel = CancellationToken::default();
    let (mut reader, mut writer) = UnixStream::pair().unwrap();
    reader
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    let writer = std::thread::spawn(move || {
        for byte in 3u32.to_be_bytes().into_iter().chain(*b"abc") {
            writer.write_all(&[byte]).unwrap();
        }
    });
    let mut counters = StreamCounters::default();
    assert_eq!(
        read_ciphertext_frame(
            &mut reader,
            &cancel,
            Instant::now() + Duration::from_secs(1),
            &mut counters
        )
        .unwrap(),
        b"abc"
    );
    writer.join().unwrap();
    assert_eq!(
        (
            counters.framing_bytes_received,
            counters.envelope_bytes_received,
            counters.frames_received
        ),
        (4, 3, 1)
    );
    for size in [0u32, 262_145] {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        writer.write_all(&size.to_be_bytes()).unwrap();
        let mut counters = StreamCounters::default();
        assert!(matches!(
            read_ciphertext_frame(
                &mut reader,
                &cancel,
                Instant::now() + Duration::from_secs(1),
                &mut counters
            ),
            Err(LiveError::Framing)
        ));
        assert_eq!(
            (
                counters.framing_bytes_received,
                counters.envelope_bytes_received
            ),
            (4, 0)
        );
    }
    let (mut reader, mut writer) = UnixStream::pair().unwrap();
    reader
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    writer.write_all(&10u32.to_be_bytes()).unwrap();
    writer.write_all(b"abc").unwrap();
    writer.shutdown(std::net::Shutdown::Write).unwrap();
    let mut counters = StreamCounters::default();
    assert!(matches!(
        read_ciphertext_frame(
            &mut reader,
            &cancel,
            Instant::now() + Duration::from_secs(1),
            &mut counters
        ),
        Err(LiveError::Io(_))
    ));
    assert_eq!(
        (counters.envelope_bytes_received, counters.frames_received),
        (3, 0)
    );
}

struct PartialWriter {
    socket: UnixStream,
    remaining: usize,
    closed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl std::io::Read for PartialWriter {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.socket, bytes)
    }
}
impl std::io::Write for PartialWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
        let len = self.remaining.min(bytes.len());
        let count = std::io::Write::write(&mut self.socket, &bytes[..len])?;
        self.remaining -= count;
        Ok(count)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl spiralcore_mvm_adapter::live::SocketIo for PartialWriter {
    fn set_read_timeout(&self, value: Option<Duration>) -> std::io::Result<()> {
        spiralcore_mvm_adapter::live::SocketIo::set_read_timeout(&self.socket, value)
    }
    fn set_write_timeout(&self, value: Option<Duration>) -> std::io::Result<()> {
        spiralcore_mvm_adapter::live::SocketIo::set_write_timeout(&self.socket, value)
    }
    fn shutdown(&self, how: std::net::Shutdown) -> std::io::Result<()> {
        self.closed
            .store(true, std::sync::atomic::Ordering::Release);
        self.socket.shutdown(how)
    }
}

#[test]
fn partial_write_timeout_and_bad_handshake_close_without_retrying_fragments() {
    use spiralcore_mvm_adapter::live::{
        LiveError, StreamCounters, handshake_stream, read_ciphertext_frame, write_ciphertext_frame,
    };
    use std::io::{Read, Write};
    let (socket, mut peer) = UnixStream::pair().unwrap();
    let closed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut writer = PartialWriter {
        socket,
        remaining: 6,
        closed: closed.clone(),
    };
    let cancel = CancellationToken::default();
    let mut counters = StreamCounters::default();
    assert!(matches!(
        write_ciphertext_frame(
            &mut writer,
            b"abcdef",
            &cancel,
            Instant::now() + Duration::from_secs(1),
            &mut counters
        ),
        Err(LiveError::Io(_))
    ));
    assert!(closed.load(std::sync::atomic::Ordering::Acquire));
    let mut received = Vec::new();
    peer.read_to_end(&mut received).unwrap();
    assert_eq!(received, [0, 0, 0, 6, b'a', b'b']);
    assert_eq!(
        (
            counters.framing_bytes_sent,
            counters.envelope_bytes_sent,
            counters.frames_sent
        ),
        (4, 2, 0)
    );
    let (mut reader, mut peer) = UnixStream::pair().unwrap();
    let mut counters = StreamCounters::default();
    assert!(matches!(
        read_ciphertext_frame(
            &mut reader,
            &cancel,
            Instant::now() + Duration::from_millis(25),
            &mut counters
        ),
        Err(LiveError::Deadline)
    ));
    assert_eq!(peer.read(&mut [0]).unwrap(), 0);
    let (mut reader, mut peer) = UnixStream::pair().unwrap();
    peer.write_all(b"WRONG001").unwrap();
    assert!(matches!(
        handshake_stream(
            &mut reader,
            HandshakeRole::Acceptor,
            &cancel,
            Instant::now() + Duration::from_secs(1),
            &mut StreamCounters::default()
        ),
        Err(LiveError::Framing)
    ));
    assert_eq!(peer.read(&mut [0]).unwrap(), 0);
}

fn pending_reply(
    legacy_reply: bool,
    lifetime: Duration,
) -> (
    Direction,
    spiralcore_mvm_adapter::live::SentCall,
    spiralcore_mvm_adapter::VerifiedFrame,
    spiralcore_mvm_adapter::MailboxAuthority,
    spiralcore_transfer::endpoint::Context,
) {
    let (socket, peer) = UnixStream::pair().unwrap();
    let mut sender = Direction::new(live_config(MailboxMessageKind::Call, true)).unwrap();
    let mut replies = Direction::new(live_config(MailboxMessageKind::Reply, false)).unwrap();
    let peer = std::thread::spawn(move || {
        let mut incoming = Direction::new(live_config(MailboxMessageKind::Call, false)).unwrap();
        let mut reply_sender =
            Direction::new(live_config(MailboxMessageKind::Reply, true)).unwrap();
        let stream =
            StreamTransport::new(peer, reply_sender.route_id(), Duration::from_secs(2)).unwrap();
        let cancel = CancellationToken::default();
        stream.handshake(HandshakeRole::Acceptor, &cancel).unwrap();
        let wire = stream
            .receive(&cancel, Instant::now() + Duration::from_secs(2))
            .unwrap();
        let request = incoming.open_live(&wire).unwrap();
        let reply = request.frame().reply(105, b"reply".to_vec());
        if legacy_reply {
            reply_sender.send(&reply, &stream, &cancel).unwrap();
        } else {
            reply_sender
                .reply_to_verified(&request, &reply, &stream, &cancel)
                .unwrap();
        }
        (incoming.authority(), request.context().clone())
    });
    let stream = StreamTransport::new(socket, sender.route_id(), Duration::from_secs(2)).unwrap();
    let cancel = CancellationToken::default();
    stream.handshake(HandshakeRole::Initiator, &cancel).unwrap();
    let request = Frame {
        kind: 5,
        transfer: [93; 32],
        seq: 0,
        body: Vec::new(),
    };
    let call = sender
        .send_call(
            &request,
            &replies,
            &stream,
            &cancel,
            Instant::now() + lifetime,
        )
        .unwrap();
    assert!(matches!(
        sender.send_call(
            &request,
            &replies,
            &stream,
            &cancel,
            Instant::now() + lifetime
        ),
        Err(spiralcore_mvm_adapter::live::LiveError::CallPending)
    ));
    let wire = stream.receive(&cancel, call.deadline()).unwrap();
    let verified = replies.open_live(&wire).unwrap();
    let (authority, context) = peer.join().unwrap();
    (replies, call, verified, authority, context)
}

#[test]
fn authenticated_but_uncorrelated_and_late_replies_cannot_complete_calls() {
    use spiralcore_mvm_adapter::live::LiveError;
    use spiralcore_transfer::endpoint::Authority;
    let (replies, call, response, _, _) = pending_reply(true, Duration::from_secs(30));
    assert!(matches!(
        replies.accept_reply(call, response),
        Err(LiveError::Policy)
    ));
    let (replies, call, response, authority, context) =
        pending_reply(false, Duration::from_secs(6));
    assert!(authority.authorize(&context).is_ok());
    std::thread::sleep(
        call.deadline().saturating_duration_since(Instant::now()) + Duration::from_millis(3),
    );
    assert!(matches!(
        replies.accept_reply(call, response),
        Err(LiveError::Deadline)
    ));
    assert!(
        authority.authorize(&context).is_err(),
        "live lease must expire without Policy::replace"
    );
}

#[test]
fn live_boot_reply_route_and_application_sequence_are_bound_to_the_proof() {
    use mvm_contract::protocol::mailbox::{BootId, CapabilityId};
    use mvm_core::mailbox::transport::InMemoryPeerTransport;
    use spiralcore_mvm_adapter::live::LiveError;
    let mut sender = Direction::new(live_config(MailboxMessageKind::Call, true)).unwrap();
    let transport = InMemoryPeerTransport::default();
    transport.register_route(sender.route_id()).unwrap();
    let request = Frame {
        kind: 5,
        transfer: [94; 32],
        seq: 0,
        body: Vec::new(),
    };
    sender
        .send(&request, &transport, &CancellationToken::default())
        .unwrap();
    let wire = transport.drain(sender.route_id()).unwrap().pop().unwrap();
    let mut wrong_boot = live_config(MailboxMessageKind::Call, false);
    wrong_boot.boot = BootId::from_bytes([99; 32]);
    assert!(
        Direction::new(wrong_boot)
            .unwrap()
            .open_live(&wire)
            .is_err()
    );
    let mut incoming = Direction::new(live_config(MailboxMessageKind::Call, false)).unwrap();
    let verified = incoming.open_live(&wire).unwrap();
    let mut wrong_route = live_config(MailboxMessageKind::Reply, true);
    wrong_route.capability.grant.capability_id = CapabilityId::from_bytes([99; 32]);
    wrong_route.capability = sign_capability(
        wrong_route.capability.grant,
        &ed25519_dalek::SigningKey::from_bytes(&[22; 32]),
    )
    .unwrap();
    let mut wrong_reply = Direction::new(wrong_route).unwrap();
    let (socket, _peer) = UnixStream::pair().unwrap();
    let stream =
        StreamTransport::new(socket, wrong_reply.route_id(), Duration::from_secs(2)).unwrap();
    let reply = request.reply(105, Vec::new());
    assert!(matches!(
        wrong_reply.reply_to_verified(&verified, &reply, &stream, &CancellationToken::default()),
        Err(LiveError::Policy)
    ));
    let mut correct_reply = Direction::new(live_config(MailboxMessageKind::Reply, true)).unwrap();
    let mut wrong_sequence = reply;
    wrong_sequence.seq += 1;
    assert!(matches!(
        correct_reply.reply_to_verified(
            &verified,
            &wrong_sequence,
            &stream,
            &CancellationToken::default()
        ),
        Err(LiveError::Policy)
    ));
    assert_eq!(stream.counters().unwrap().frames_sent, 0);
}
