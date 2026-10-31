mod common;

use mvm_contract::protocol::mailbox::MailboxMessageKind;
use mvm_core::mailbox::transport::{CancellationToken, InMemoryPeerTransport};
use spiralcore_mvm_adapter::Direction;
use spiralcore_transfer::protocol::Frame;

#[test]
fn canonical_call_and_independent_reply_preserve_exact_application_frames() {
    let transport = InMemoryPeerTransport::default();
    let cancellation = CancellationToken::default();
    let request = Frame { kind: 5, transfer: [42; 32], seq: 9, body: vec![] };
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
