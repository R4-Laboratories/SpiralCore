#![cfg(unix)]
//! Durable Unix storage dispatch witnesses. Portable mailbox CI runs the
//! storage-independent `canonical` target on every declared native platform.
mod common;

use mvm_contract::protocol::mailbox::MailboxMessageKind;
use mvm_core::mailbox::transport::{CancellationToken, InMemoryPeerTransport};
use spiralcore_mvm_adapter::{Direction, Error};
use spiralcore_transfer::{
    endpoint::Authority,
    object_id,
    protocol::{Frame, Mode, Offer},
};

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
