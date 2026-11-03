#![cfg(feature = "live")]
mod common;
#[path = "common/service.rs"]
mod fixture;

use mvm_contract::protocol::mailbox::MailboxMessageKind;
use mvm_core::mailbox::{
    encode_envelope,
    transport::{CancellationToken, InMemoryPeerTransport},
};
use spiralcore_mvm_adapter::{
    service::{
        CallStatus, HostBudget, LocalMailboxService, ServiceError, ServiceLimits, ServiceTime,
    },
    service_sender::{self, MessageOptions},
};

fn at(delta: u64) -> ServiceTime {
    ServiceTime {
        unix_ms: fixture::time().unix_ms + delta as i64,
        monotonic_ms: fixture::time().monotonic_ms + delta,
    }
}

#[test]
fn registered_call_retry_preserves_unconsumed_canonical_reply_and_exact_request() {
    let budget = HostBudget::new(ServiceLimits::default()).unwrap();
    let mut caller = LocalMailboxService::new(budget.clone()).unwrap();
    let calls = fixture::binding(MailboxMessageKind::Call, true, false, true);
    let mut options = MessageOptions::fresh(fixture::content_type(), at(0), 2000).unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    let prepared = service_sender::prepare(&calls, b"original\0call", &options, at(0)).unwrap();
    let wire = encode_envelope(prepared.envelope()).unwrap();
    let call = caller.register_call(&calls, &wire, at(0)).unwrap();

    // A separate real receiver admits the Call and prepares its authorized Reply.
    let mut receiver =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let incoming = fixture::binding(MailboxMessageKind::Call, true, false, false);
    receiver.admit(&incoming, &wire, at(0)).unwrap();
    let proof = receiver.receive(&incoming, at(0)).unwrap().unwrap();
    let replies = fixture::binding(MailboxMessageKind::Reply, false, false, true);
    let response = b"retained\0reply\xff";
    let reply_options = MessageOptions::for_reply(&proof, at(0)).unwrap();
    let reply = receiver
        .reply_prepared(
            &replies,
            proof.handle(),
            service_sender::request(&replies, response, &reply_options, at(0)).unwrap(),
            at(0),
        )
        .unwrap();
    let reply_wire = encode_envelope(reply.envelope()).unwrap();
    let incoming_reply = fixture::binding(MailboxMessageKind::Reply, false, false, false);
    caller.admit(&incoming_reply, &reply_wire, at(0)).unwrap();
    let charged = budget.snapshot().unwrap();
    assert_eq!(charged.queued_bytes, reply_wire.len() as u64);
    assert_eq!(charged.queued_messages, 1);
    assert_eq!(charged.active_calls, 0);
    let before = caller.snapshot();

    let transport = InMemoryPeerTransport::default();
    let route = common::config(MailboxMessageKind::Call)
        .binding
        .claims
        .route_id;
    transport.register_route(route).unwrap();
    let cancel = CancellationToken::default();
    assert_eq!(
        caller.send_call_prepared(&calls, &call, &prepared, &transport, &cancel, at(1)),
        Err(ServiceError::Conflict)
    );
    assert!(transport.drain(route).unwrap().is_empty());
    assert_eq!(
        caller.retry_call_prepared(&calls, &call, &prepared, &transport, &cancel, at(1)),
        Ok(wire.len())
    );
    assert_eq!(transport.drain(route).unwrap(), [wire]);
    let retained = budget.snapshot().unwrap();
    assert_eq!(retained.queued_bytes, charged.queued_bytes);
    assert_eq!(retained.queued_messages, charged.queued_messages);
    assert_eq!(retained.active_calls, charged.active_calls);
    assert_eq!(retained.identities, charged.identities);
    let after = caller.snapshot();
    assert_eq!(after.calls, before.calls);
    assert_eq!(after.accepted, before.accepted);
    assert_eq!(after.fifo_scopes, before.fifo_scopes);
    assert_eq!(after.fifo_entries, before.fifo_entries);
    assert_eq!(
        caller.poll_call(&calls, &call, at(1)).unwrap(),
        CallStatus::Replied(Some(response.to_vec()))
    );
    assert_eq!(budget.snapshot().unwrap().queued_bytes, 0);
    assert_eq!(budget.snapshot().unwrap().queued_messages, 0);
    assert_eq!(
        caller.poll_call(&calls, &call, at(1)).unwrap(),
        CallStatus::Replied(None)
    );
    assert_eq!(
        caller.retry_call_prepared(&calls, &call, &prepared, &transport, &cancel, at(1)),
        Err(ServiceError::Conflict)
    );
    assert!(transport.drain(route).unwrap().is_empty());
}

#[test]
fn registered_call_send_is_exact_and_does_not_advance_other_services_accounting_clock() {
    let budget = HostBudget::new(ServiceLimits::default()).unwrap();
    let mut service = LocalMailboxService::new(budget.clone()).unwrap();
    let binding = fixture::binding(MailboxMessageKind::Call, true, false, true);
    let mut options = MessageOptions::fresh(fixture::content_type(), at(0), 2000).unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    let prepared = service_sender::prepare(&binding, b"exact\0call\0", &options, at(0)).unwrap();
    let wire = encode_envelope(prepared.envelope()).unwrap();
    let call = service.register_call(&binding, &wire, at(0)).unwrap();
    let mut other = LocalMailboxService::new(budget).unwrap();
    other.expire(at(2)).unwrap();
    let transport = InMemoryPeerTransport::default();
    let route = common::config(MailboxMessageKind::Call)
        .binding
        .claims
        .route_id;
    transport.register_route(route).unwrap();
    assert_eq!(
        service
            .send_call_prepared(
                &binding,
                &call,
                &prepared,
                &transport,
                &CancellationToken::default(),
                at(1)
            )
            .unwrap(),
        wire.len()
    );
    assert_eq!(transport.drain(route).unwrap(), [wire]);
}

#[test]
fn registered_call_send_rejects_changed_generation_request_lifecycle_or_time_before_transmission() {
    for mode in [
        "generation",
        "request",
        "cancel",
        "shutdown",
        "unix-expiry",
        "monotonic-expiry",
        "clock",
        "foreign-service",
    ] {
        let budget = HostBudget::new(ServiceLimits::default()).unwrap();
        let mut service = LocalMailboxService::new(budget.clone()).unwrap();
        let binding = fixture::binding(MailboxMessageKind::Call, true, false, true);
        let mut options = MessageOptions::fresh(fixture::content_type(), at(0), 2000).unwrap();
        options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
        let mut prepared = service_sender::prepare(&binding, b"original", &options, at(0)).unwrap();
        let call = service
            .register_call(
                &binding,
                &encode_envelope(prepared.envelope()).unwrap(),
                at(0),
            )
            .unwrap();
        let transport = InMemoryPeerTransport::default();
        let route = common::config(MailboxMessageKind::Call)
            .binding
            .claims
            .route_id;
        transport.register_route(route).unwrap();
        let mut time = at(1);
        let expected = match mode {
            "generation" => {
                let mut policy = common::config(MailboxMessageKind::Call)
                    .policy
                    .snapshot()
                    .unwrap();
                policy.accepted_content_types = vec![fixture::content_type()];
                binding.replace_policy(policy).unwrap();
                ServiceError::Unauthorized
            }
            "request" => {
                let mut different =
                    MessageOptions::fresh(fixture::content_type(), at(0), 2000).unwrap();
                different.reply_to = options.reply_to;
                prepared =
                    service_sender::prepare(&binding, b"different", &different, at(0)).unwrap();
                ServiceError::Conflict
            }
            "cancel" => {
                service.cancel(&binding, &call, at(0)).unwrap();
                ServiceError::Cancelled
            }
            "shutdown" => {
                service.shutdown(at(0)).unwrap();
                ServiceError::ShuttingDown
            }
            "unix-expiry" => {
                time.unix_ms = 2000;
                ServiceError::Expired
            }
            "monotonic-expiry" => {
                time.monotonic_ms = 600;
                ServiceError::Expired
            }
            "clock" => {
                time.monotonic_ms = 99;
                ServiceError::Clock
            }
            "foreign-service" => {
                service = LocalMailboxService::new(budget).unwrap();
                ServiceError::Unauthorized
            }
            _ => unreachable!(),
        };
        assert_eq!(
            service.send_call_prepared(
                &binding,
                &call,
                &prepared,
                &transport,
                &CancellationToken::default(),
                time
            ),
            Err(expected),
            "{mode}"
        );
        assert!(transport.drain(route).unwrap().is_empty(), "{mode}");
    }
}
