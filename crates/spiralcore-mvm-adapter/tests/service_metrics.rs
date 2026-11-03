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
