#![cfg(feature = "live")]
mod common;
#[path = "common/service.rs"]
mod fixture;
use mvm_contract::protocol::mailbox::*;
use spiralcore_mvm_adapter::{
    service::{HostBudget, LocalMailboxService, ServiceLimits},
    service_sender::{self, MessageOptions},
};

#[test]
fn diagnostics_count_charged_state_without_resetting_replay() {
    let budget = HostBudget::new(ServiceLimits::default()).unwrap();
    let mut service = LocalMailboxService::new(budget.clone()).unwrap();
    assert_eq!(budget.snapshot().unwrap().queued_messages, 0);
    assert_eq!(service.snapshot().fifo_scopes, 0);
    let producer = fixture::binding(MailboxMessageKind::Cast, true, false, true);
    let consumer = fixture::binding(MailboxMessageKind::Cast, true, false, false);
    let options = MessageOptions::fresh(fixture::content_type(), fixture::time(), 2000).unwrap();
    let prepared =
        service_sender::prepare(&producer, b"bytes\0\0", &options, fixture::time()).unwrap();
    let encoded = mvm_core::mailbox::encode_envelope(prepared.envelope()).unwrap();
    service.admit(&consumer, &encoded, fixture::time()).unwrap();
    let high = budget.snapshot().unwrap();
    assert_eq!(high.queued_messages, 1);
    assert_eq!(high.owner_count, 1);
    assert!(high.queued_bytes >= 7);
    assert!(high.identities >= 2);
    assert_eq!(service.snapshot().accepted, 1);
    assert_eq!(service.snapshot().fifo_entries, 1);
    let message = service
        .receive(&consumer, fixture::time())
        .unwrap()
        .unwrap();
    assert_eq!(message.payload(), b"bytes\0\0");
    assert_eq!(service.snapshot().fifo_entries, 0);
    assert_eq!(service.snapshot().fifo_scopes, 1);
    assert_eq!(service.snapshot().calls, 0);
    assert_eq!(
        service
            .admit(&consumer, &encoded, fixture::time())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::Duplicate
    );
    assert!(
        service
            .receive(&consumer, fixture::time())
            .unwrap()
            .is_none()
    );
}

#[test]
fn diagnostic_names_first_refusing_scope_without_publishing_a_message() {
    use spiralcore_mvm_adapter::service::{BudgetPool, BudgetResource, BudgetScope};
    for scope in [
        BudgetScope::Host,
        BudgetScope::Tenant,
        BudgetScope::Mailbox,
        BudgetScope::Sender,
    ] {
        let mut limits = ServiceLimits::default();
        limits.sender.identities = 1;
        if scope != BudgetScope::Sender {
            limits.mailbox.identities = 1;
        }
        if matches!(scope, BudgetScope::Host | BudgetScope::Tenant) {
            limits.tenant.identities = 1;
        }
        if scope == BudgetScope::Host {
            limits.host.identities = 1;
        }
        let budget = HostBudget::new(limits).unwrap();
        let mut service = LocalMailboxService::new(budget.clone()).unwrap();
        let producer = fixture::binding(MailboxMessageKind::Cast, true, false, true);
        let consumer = fixture::binding(MailboxMessageKind::Cast, true, false, false);
        let options =
            MessageOptions::fresh(fixture::content_type(), fixture::time(), 2000).unwrap();
        let prepared =
            service_sender::prepare(&producer, b"not published\0", &options, fixture::time())
                .unwrap();
        let encoded = mvm_core::mailbox::encode_envelope(prepared.envelope()).unwrap();
        assert_eq!(
            service
                .admit(&consumer, &encoded, fixture::time())
                .unwrap()
                .receipt
                .outcome,
            DeliveryOutcome::Full
        );
        let state = budget.snapshot().unwrap();
        let reason = state.refusals.last.unwrap();
        assert_eq!(
            (reason.scope, reason.pool, reason.resource),
            (scope, BudgetPool::Total, BudgetResource::Identities)
        );
        assert_eq!((reason.current, reason.requested, reason.limit), (1, 1, 1));
        assert_eq!(state.refusals.total, 1);
        assert_eq!(state.queued_bytes, 0);
        assert_eq!(state.queued_messages, 0);
        assert_eq!(service.snapshot().accepted, 0);
        assert!(
            service
                .receive(&consumer, fixture::time())
                .unwrap()
                .is_none()
        );
        // Malformed input is not mislabeled as a new quota refusal.
        assert!(
            service
                .admit(&consumer, b"malformed", fixture::time())
                .is_err()
        );
        assert_eq!(budget.snapshot().unwrap().refusals.total, 1);
    }
}

#[test]
fn reserved_data_identity_capacity_does_not_borrow_control_credits() {
    use spiralcore_mvm_adapter::service::{BudgetPool, BudgetResource, BudgetScope, ControlReserve};
    let mut limits = ServiceLimits::default();
    limits.sender.identities = 4;
    let mut reserve = ControlReserve::default();
    reserve.sender.identities = 1;
    let budget = HostBudget::with_control(limits, reserve).unwrap();
    let mut service = LocalMailboxService::new(budget.clone()).unwrap();
    let producer = fixture::binding(MailboxMessageKind::Cast, true, false, true);
    let consumer = fixture::binding(MailboxMessageKind::Cast, true, false, false);
    let options = MessageOptions::fresh(fixture::content_type(), fixture::time(), 2000).unwrap();
    let prepared = service_sender::prepare(&producer, b"data\0", &options, fixture::time()).unwrap();
    let encoded = mvm_core::mailbox::encode_envelope(prepared.envelope()).unwrap();
    // Owner, stable scope, queue entry and accepted replay identity require
    // four data identities. The reserved control identity cannot fund the fourth.
    assert_eq!(service.admit(&consumer, &encoded, fixture::time()).unwrap().receipt.outcome, DeliveryOutcome::Full);
    let reason = budget.snapshot().unwrap().refusals.last.unwrap();
    assert_eq!((reason.scope, reason.pool, reason.resource), (BudgetScope::Sender, BudgetPool::Data, BudgetResource::Identities));
    assert_eq!((reason.current, reason.requested, reason.limit), (3, 1, 3));
    assert_eq!(service.snapshot().accepted, 0);
    assert_eq!(budget.snapshot().unwrap().queued_messages, 0);
}
