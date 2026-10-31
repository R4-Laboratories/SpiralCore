mod common;

use mvm_contract::protocol::mailbox::MailboxMessageKind;
use mvm_core::mailbox::transport::{CancellationToken, InMemoryPeerTransport};
use spiralcore_mvm_adapter::{Direction, Error, MAX_ENVELOPE_BYTES, MAX_FRAME_BYTES};
use spiralcore_transfer::{
    endpoint::Authority,
    object_id,
    protocol::{Frame, Mode, Offer},
};

#[test]
fn canonical_call_and_independent_reply_preserve_exact_application_frames() {
    let transport = InMemoryPeerTransport::default();
    let cancellation = CancellationToken::default();
    let request = Frame {
        kind: 5,
        transfer: [42; 32],
        seq: 9,
        body: vec![],
    };
    let reply = request.reply(103, vec![7; 32]);
    for (kind, frame) in [
        (MailboxMessageKind::Call, request),
        (MailboxMessageKind::Reply, reply),
    ] {
        let mut direction = Direction::new(common::config(kind)).unwrap();
        transport.register_route(direction.route_id()).unwrap();
        let encoded_len = direction.send(&frame, &transport, &cancellation).unwrap();
        let mut received = transport.drain(direction.route_id()).unwrap();
        assert_eq!(received.len(), 1);
        let wire = received.pop().unwrap();
        assert_eq!(encoded_len, wire.len());
        assert!(encoded_len > frame.encode().unwrap().len());
        assert!(!wire.windows(8).any(|w| w == b"SCMOBJ01"));
        let verified = direction.open(&wire).unwrap();
        assert_eq!(verified.frame(), &frame);
        assert_eq!(verified.context().tenant, "13".repeat(32));
        assert_eq!(verified.context().policy_epoch, 4);
    }
}

fn queued(direction: &mut Direction, frame: &Frame, transport: &InMemoryPeerTransport) -> Vec<u8> {
    direction
        .send(frame, transport, &CancellationToken::default())
        .unwrap();
    let mut wire = transport.drain(direction.route_id()).unwrap();
    assert_eq!(wire.len(), 1);
    wire.pop().unwrap()
}

fn begin() -> Frame {
    Frame {
        kind: 1,
        transfer: [42; 32],
        seq: 0,
        body: Offer::cold(&[], Mode::Raw).encode().unwrap(),
    }
}

#[test]
fn receiver_only_configuration_opens_without_the_remote_signing_secret() {
    let mut sender = Direction::new(common::config(MailboxMessageKind::Call)).unwrap();
    let mut config = common::config(MailboxMessageKind::Call);
    config.signing_key = None;
    let mut receiver = Direction::new(config).unwrap();
    let transport = InMemoryPeerTransport::default();
    transport.register_route(sender.route_id()).unwrap();
    let frame = begin();
    let wire = queued(&mut sender, &frame, &transport);
    assert_eq!(receiver.open(&wire).unwrap().frame(), &frame);
    assert!(matches!(
        receiver.send(&frame, &transport, &CancellationToken::default()),
        Err(Error::Policy)
    ));
    assert!(transport.drain(sender.route_id()).unwrap().is_empty());
}

#[test]
fn configuration_refuses_ambiguous_kind_content_and_mismatching_signer() {
    let mut config = common::config(MailboxMessageKind::Call);
    config
        .capability
        .grant
        .message_kinds
        .push(MailboxMessageKind::Reply);
    assert!(Direction::new(config).is_err());
    let mut config = common::config(MailboxMessageKind::Call);
    config
        .capability
        .grant
        .content_types
        .push(mvm_contract::protocol::mailbox::ContentType::parse("application/other", 1).unwrap());
    assert!(Direction::new(config).is_err());
    let mut config = common::config(MailboxMessageKind::Call);
    config.signing_key = Some(ed25519_dalek::SigningKey::from_bytes(&[99; 32]));
    assert!(Direction::new(config).is_err());
}

#[test]
fn malformed_or_oversized_authenticated_payload_cannot_mint_authority() {
    for payload in [
        b"not a SpiralCore frame".to_vec(),
        vec![0; MAX_FRAME_BYTES + 1],
    ] {
        let config = common::config(MailboxMessageKind::Call);
        let wire = common::sealed_payload(&config, &payload);
        let mut receiver = Direction::new(config).unwrap();
        assert!(receiver.open(&wire).is_err());
        assert!(
            receiver
                .authority()
                .authorize(&spiralcore_transfer::wan::context([1; 32]))
                .is_err()
        );
    }
    let mut receiver = Direction::new(common::config(MailboxMessageKind::Call)).unwrap();
    assert!(matches!(
        receiver.open(&vec![0; MAX_ENVELOPE_BYTES + 1]),
        Err(Error::Limit)
    ));
}

#[test]
fn altered_ciphertext_and_wrong_verification_key_are_rejected_before_authority() {
    let config = common::config(MailboxMessageKind::Call);
    let wire = common::sealed_payload(&config, &begin().encode().unwrap());
    let mut receiver = Direction::new(config).unwrap();
    let mut envelope = mvm_core::mailbox::decode_envelope(&wire).unwrap();
    envelope.ciphertext[0] ^= 1;
    let tampered = mvm_core::mailbox::encode_envelope(&envelope).unwrap();
    assert!(receiver.open(&tampered).is_err());
    let mut config = common::config(MailboxMessageKind::Call);
    config.signing_key = None;
    config.sender_verifying_key = ed25519_dalek::SigningKey::from_bytes(&[99; 32]).verifying_key();
    let mut wrong_key = Direction::new(config).unwrap();
    assert!(wrong_key.open(&wire).is_err());
    assert!(
        wrong_key
            .authority()
            .authorize(&spiralcore_transfer::wan::context([1; 32]))
            .is_err()
    );
    assert_eq!(receiver.open(&wire).unwrap().frame(), &begin());
}

#[test]
fn changed_recipient_epoch_or_revocation_fails_on_open_and_on_outbound() {
    use mvm_contract::protocol::mailbox::{ExecutionEpoch, PrincipalId, RevocationId};
    for case in 0..5 {
        let config = common::config(MailboxMessageKind::Call);
        let policy = config.policy.clone();
        let mut direction = Direction::new(config).unwrap();
        let transport = InMemoryPeerTransport::default();
        transport.register_route(direction.route_id()).unwrap();
        let wire = queued(&mut direction, &begin(), &transport);
        let mut changed = policy.snapshot().unwrap();
        match case {
            0 => changed.expected_recipient = PrincipalId::from_bytes([99; 32]),
            1 => changed.current_execution_epoch = ExecutionEpoch::new(8).unwrap(),
            2 => changed.current_policy_epoch += 1,
            3 => changed
                .revoked_capabilities
                .push(RevocationId::from_bytes([13; 32])),
            _ => changed.now_unix_ms = 3_001,
        }
        policy.replace(changed).unwrap();
        assert!(direction.open(&wire).is_err(), "case {case}");
        assert!(
            direction
                .send(&begin(), &transport, &CancellationToken::default())
                .is_err(),
            "case {case}"
        );
        assert!(transport.drain(direction.route_id()).unwrap().is_empty());
    }
}

#[test]
fn revocation_between_open_and_dispatch_invalidates_the_verified_message() {
    use mvm_contract::protocol::mailbox::RevocationId;
    let config = common::config(MailboxMessageKind::Call);
    let policy = config.policy.clone();
    let mut direction = Direction::new(config).unwrap();
    let authority = direction.authority();
    assert!(
        authority
            .authorize(&spiralcore_transfer::wan::context([1; 32]))
            .is_err()
    );
    let store = common::Store::new();
    let mut endpoint = store.open(authority.clone());
    let transport = InMemoryPeerTransport::default();
    transport.register_route(direction.route_id()).unwrap();
    let wire = queued(&mut direction, &begin(), &transport);
    let verified = direction.open(&wire).unwrap();
    let context = verified.context().clone();
    assert!(authority.authorize(&context).is_ok());
    let mut changed = policy.snapshot().unwrap();
    changed
        .revoked_capabilities
        .push(RevocationId::from_bytes([13; 32]));
    policy.replace(changed).unwrap();
    assert!(verified.dispatch(&mut endpoint, 0).is_err());
    assert!(authority.authorize(&context).is_err());
    assert!(endpoint.read_object(&context, object_id(&[])).is_err());
}

#[test]
fn open_before_dispatch_and_lost_commit_reply_require_fresh_envelope_retries() {
    let mut direction = Direction::new(common::config(MailboxMessageKind::Call)).unwrap();
    let store = common::Store::new();
    let mut endpoint = store.open(direction.authority());
    let transport = InMemoryPeerTransport::default();
    transport.register_route(direction.route_id()).unwrap();
    let request = begin();
    let wire = queued(&mut direction, &request, &transport);
    let accepted = direction.open(&wire).unwrap();
    let context = accepted.context().clone();
    drop(accepted); // Simulated interruption before object endpoint handling.
    assert!(endpoint.read_object(&context, object_id(&[])).is_err());
    assert!(matches!(
        direction.open(&wire),
        Err(Error::Mailbox(mvm_core::mailbox::MailboxError::Replay))
    ));
    let retry = queued(&mut direction, &request, &transport);
    let ready = direction
        .open(&retry)
        .unwrap()
        .dispatch(&mut endpoint, 0)
        .unwrap();
    assert_eq!(ready.kind, 101);
    let retry = queued(&mut direction, &request, &transport);
    assert_eq!(
        direction
            .open(&retry)
            .unwrap()
            .dispatch(&mut endpoint, 0)
            .unwrap(),
        ready
    );
    let finish = Frame {
        kind: 3,
        seq: 1,
        body: vec![],
        ..request
    };
    let wire = queued(&mut direction, &finish, &transport);
    let published = direction
        .open(&wire)
        .unwrap()
        .dispatch(&mut endpoint, 0)
        .unwrap();
    assert_eq!(published.kind, 103); // Simulate loss of this application reply.
    let retry = queued(&mut direction, &finish, &transport);
    assert_eq!(
        direction
            .open(&retry)
            .unwrap()
            .dispatch(&mut endpoint, 0)
            .unwrap(),
        published
    );
    drop(endpoint);
    let mut reopened = store.open(direction.authority());
    assert_eq!(
        reopened.read_object(&context, object_id(&[])).unwrap(),
        Vec::<u8>::new()
    );
}

#[test]
fn maximum_frame_fits_and_oversize_cancellation_and_queue_pressure_refuse() {
    let mut direction = Direction::new(common::config(MailboxMessageKind::Call)).unwrap();
    let transport = InMemoryPeerTransport::default();
    transport.register_route(direction.route_id()).unwrap();
    let frame = Frame {
        kind: 2,
        transfer: [91; 32],
        seq: 1,
        body: common::random(MAX_FRAME_BYTES - 56, 29),
    };
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(matches!(
        direction.send(&frame, &transport, &cancelled),
        Err(Error::Transport(_))
    ));
    assert!(transport.drain(direction.route_id()).unwrap().is_empty());
    let wire = queued(&mut direction, &frame, &transport);
    assert!(wire.len() <= MAX_ENVELOPE_BYTES);
    assert_eq!(direction.open(&wire).unwrap().frame(), &frame);
    let mut oversized = frame.clone();
    oversized.body.push(0);
    assert!(
        direction
            .send(&oversized, &transport, &CancellationToken::default())
            .is_err()
    );
    direction
        .send(&frame, &transport, &CancellationToken::default())
        .unwrap();
    direction
        .send(&frame, &transport, &CancellationToken::default())
        .unwrap();
    assert!(matches!(
        direction.send(&frame, &transport, &CancellationToken::default()),
        Err(Error::Transport(
            mvm_core::mailbox::transport::TransportError::Backpressured
        ))
    ));
    assert_eq!(transport.drain(direction.route_id()).unwrap().len(), 2);
}
