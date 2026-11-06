#![cfg(feature = "live")]
mod common;
#[path = "common/service.rs"]
mod fixture;

use mvm_contract::protocol::mailbox::*;
use mvm_core::mailbox::{
    encode_envelope,
    transport::{CancellationToken, InMemoryPeerTransport, RouteId},
};
use spiralcore_mvm_adapter::{
    receipt::{ReceiptReturnConfig, ReceiptSendError},
    service::{
        ControlReserve, HostBudget, LocalMailboxService, MailboxBinding, ServiceError,
        ServiceLimits, ServiceTime,
    },
    service_sender::{self, MessageOptions},
};

struct Case {
    budget: HostBudget,
    service: LocalMailboxService,
    source: MailboxBinding,
    returns: ReceiptReturnConfig,
    wire: Vec<u8>,
    transport: InMemoryPeerTransport,
}
fn at(delta: u64) -> ServiceTime {
    ServiceTime {
        unix_ms: fixture::time().unix_ms + delta as i64,
        monotonic_ms: fixture::time().monotonic_ms + delta,
    }
}
fn case() -> Case {
    let budget =
        HostBudget::with_control(ServiceLimits::default(), ControlReserve::default()).unwrap();
    let producer = fixture::binding(MailboxMessageKind::Call, true, false, true);
    let mut options = MessageOptions::fresh(fixture::content_type(), at(0), 2000).unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    let wire = encode_envelope(
        service_sender::prepare(&producer, b"exact\0receipt", &options, at(0))
            .unwrap()
            .envelope(),
    )
    .unwrap();
    let source = fixture::binding(MailboxMessageKind::Call, true, false, false);
    let returns = ReceiptReturnConfig::new(
        fixture::binding(MailboxMessageKind::Cast, false, true, true),
        budget.clone(),
        fixture::receipt_key(true).verifying_key(),
    )
    .unwrap();
    let transport = InMemoryPeerTransport::default();
    transport
        .register_route(RouteId::from_bytes([30; 32]))
        .unwrap();
    Case {
        service: LocalMailboxService::new(budget.clone()).unwrap(),
        budget,
        source,
        returns,
        wire,
        transport,
    }
}

#[test]
fn reserved_receipt_send_survives_other_broker_work_without_reserving_again() {
    let mut c = case();
    let (_, prepared) = c
        .returns
        .admit(&mut c.service, &c.source, &c.wire, at(0))
        .unwrap();
    let cancel = CancellationToken::default();
    let permit = prepared.reserve_send(&cancel, at(0)).unwrap();
    // Another local dispatch advances aggregate time after this send phase's
    // timestamp was sampled. Transport must not mutate global accounting again.
    let mut other = LocalMailboxService::new(c.budget.clone()).unwrap();
    other.expire(at(2)).unwrap();
    let count = permit.send(&c.transport, &cancel, at(1)).unwrap();
    let emitted = c.transport.drain(RouteId::from_bytes([30; 32])).unwrap();
    assert_eq!(emitted, [prepared.encoded_bytes()]);
    assert_eq!(count, prepared.encoded_bytes().len());
}

#[test]
fn reserved_send_rechecks_revocation_cancellation_expiry_and_own_clock() {
    for failure in [
        ServiceError::Unauthorized,
        ServiceError::Cancelled,
        ServiceError::Expired,
        ServiceError::Clock,
    ] {
        let mut c = case();
        let (_, prepared) = c
            .returns
            .admit(&mut c.service, &c.source, &c.wire, at(0))
            .unwrap();
        let cancel = CancellationToken::default();
        let permit = prepared.reserve_send(&cancel, at(1)).unwrap();
        let send_time = match failure {
            ServiceError::Unauthorized => {
                let mut policy = common::config(MailboxMessageKind::Call)
                    .policy
                    .snapshot()
                    .unwrap();
                policy.accepted_content_types = vec![fixture::content_type()];
                c.source.replace_policy(policy).unwrap();
                at(2)
            }
            ServiceError::Cancelled => {
                cancel.cancel();
                at(2)
            }
            ServiceError::Expired => ServiceTime {
                unix_ms: 1502,
                monotonic_ms: 2100,
            },
            ServiceError::Clock => at(0),
            _ => unreachable!(),
        };
        assert!(
            matches!(permit.send(&c.transport,&cancel,send_time),Err(ReceiptSendError::Service(error)) if error==failure)
        );
        assert!(
            c.transport
                .drain(RouteId::from_bytes([30; 32]))
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn reserved_receipt_refuses_revoked_source_or_return_binding_without_sending() {
    for revoke_source in [true, false] {
        let mut c = case();
        let (_, prepared) = c
            .returns
            .admit(&mut c.service, &c.source, &c.wire, at(0))
            .unwrap();
        let cancel = CancellationToken::default();
        let permit = prepared.reserve_send(&cancel, at(1)).unwrap();
        if revoke_source {
            c.source
                .revoke_transport_binding(
                    common::config(MailboxMessageKind::Call)
                        .binding
                        .claims
                        .binding_id,
                )
                .unwrap();
        } else {
            c.returns
                .revoke_transport_binding(
                    common::config(MailboxMessageKind::Reply)
                        .binding
                        .claims
                        .binding_id,
                )
                .unwrap();
        }
        assert!(matches!(
            permit.send(&c.transport, &cancel, at(2)),
            Err(ReceiptSendError::Service(ServiceError::Unauthorized))
        ));
        assert!(
            c.transport
                .drain(RouteId::from_bytes([30; 32]))
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn dropping_send_permit_releases_queue_credits_but_retains_attempt_rate_history() {
    let mut c = case();
    let (_, prepared) = c
        .returns
        .admit(&mut c.service, &c.source, &c.wire, at(0))
        .unwrap();
    let cancel = CancellationToken::default();
    let before = c.budget.snapshot().unwrap();
    let permit = prepared.reserve_send(&cancel, at(0)).unwrap();
    let reserved = c.budget.snapshot().unwrap();
    assert_eq!(reserved.queued_messages, before.queued_messages + 1);
    assert_eq!(
        reserved.queued_bytes,
        before.queued_bytes + prepared.encoded_bytes().len() as u64
    );
    assert_eq!(reserved.identities, before.identities + 1);
    drop(permit);
    let dropped = c.budget.snapshot().unwrap();
    assert_eq!(dropped.queued_messages, before.queued_messages);
    assert_eq!(dropped.queued_bytes, before.queued_bytes);
    assert_eq!(dropped.identities, before.identities + 1);
    drop(prepared.reserve_send(&cancel, at(1)).unwrap());
    assert_eq!(
        c.budget.snapshot().unwrap().identities,
        before.identities + 2
    );
}
