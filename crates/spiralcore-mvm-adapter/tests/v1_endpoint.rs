mod common;

use mvm_contract::protocol::mailbox::MailboxMessageKind;
use mvm_core::mailbox::transport::{CancellationToken, InMemoryPeerTransport};
use spiralcore_mvm_adapter::{Direction, open_object_endpoint};
use spiralcore_transfer::{
    client,
    endpoint::{ColdGraphPolicy, Limits},
    object_id,
    protocol::Mode,
};

#[test]
fn canonical_v1_dispatch_preserves_frames_and_exact_restart_readback() {
    let store = common::Store::new();
    let mut direction = Direction::new(common::config(MailboxMessageKind::Call)).unwrap();
    let transport = InMemoryPeerTransport::default();
    transport.register_route(direction.route_id()).unwrap();
    let mut endpoint = open_object_endpoint(
        &store.0,
        Limits::default(),
        direction.authority(),
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    let bytes = [common::random(65537, 17), vec![0, 0, 0]].concat();
    let mut context = None;
    let result = client::transfer(&bytes, Mode::Raw, None, [41; 32], |frame| {
        direction
            .send(frame, &transport, &CancellationToken::default())
            .unwrap();
        let wire = transport
            .drain(direction.route_id())
            .unwrap()
            .pop()
            .unwrap();
        let verified = direction.open(&wire).unwrap();
        assert_eq!(verified.frame(), frame);
        context = Some(verified.context().clone());
        verified
            .dispatch(&mut endpoint, 0)
            .map_err(|_| spiralcore_transfer::Error::Storage)
    })
    .unwrap();
    assert_eq!(result.object, object_id(&bytes));
    let context = context.unwrap();
    assert_eq!(
        endpoint.read_object(&context, result.object).unwrap(),
        bytes
    );
    drop(endpoint);
    let mut endpoint = open_object_endpoint(
        &store.0,
        Limits::default(),
        direction.authority(),
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    assert_eq!(
        endpoint.read_object(&context, result.object).unwrap(),
        bytes
    );
}
