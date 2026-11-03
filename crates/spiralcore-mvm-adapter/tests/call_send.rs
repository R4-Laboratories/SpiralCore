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
    service::{HostBudget, LocalMailboxService, ServiceError, ServiceLimits, ServiceTime},
    service_sender::{self, MessageOptions},
};

fn at(delta: u64) -> ServiceTime {
    ServiceTime {
        unix_ms: fixture::time().unix_ms + delta as i64,
        monotonic_ms: fixture::time().monotonic_ms + delta,
    }
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
