#![cfg(feature = "live")]
mod common;

use ed25519_dalek::SigningKey;
use mvm_contract::protocol::mailbox::*;
use mvm_core::mailbox::{encode_envelope, sign_capability, transport::sign_transport_binding};
use spiralcore_mvm_adapter::{
    receipt::{ReceiptTracker, receipt_content_type},
    service::{
        CallStatus, ControlReserve, DestinationReceiptSigner, HostBudget, LocalMailboxService,
        MailboxBinding, ServiceBindingConfig, ServiceLimits, ServiceTime,
    },
    service_sender::{self, MessageOptions},
};

fn now() -> ServiceTime {
    ServiceTime {
        unix_ms: 1500,
        monotonic_ms: 100,
    }
}
fn budget() -> HostBudget {
    HostBudget::with_control(ServiceLimits::default(), ControlReserve::default()).unwrap()
}
fn binding(kind: MailboxMessageKind, epoch: u64, control: bool) -> MailboxBinding {
    let mut config = ServiceBindingConfig::from_direction(common::config(kind)).unwrap();
    let epoch = ExecutionEpoch::new(epoch).unwrap();
    config.context.current_execution_epoch = epoch;
    config.capability.grant.minimum_execution_epoch = epoch;
    config.capability.grant.maximum_execution_epoch = epoch;
    config.binding.claims.execution_epoch = epoch;
    config.binding =
        sign_transport_binding(config.binding.claims, &SigningKey::from_bytes(&[22; 32])).unwrap();
    if control {
        config.kind = MailboxMessageKind::Cast;
        config.context.required_operation = MailboxOperation::Send;
        config.capability.grant.operations =
            CapabilityOperations::from_operations(&[MailboxOperation::Send]);
        config.capability.grant.message_kinds = vec![MailboxMessageKind::Cast];
        config.context.accepted_content_types = vec![receipt_content_type()];
        config.capability.grant.content_types = vec![receipt_content_type()];
        config.capability.grant.max_payload_bytes = 4096;
        config.capability.grant.max_queue_contribution_bytes = 16384;
    }
    config.capability =
        sign_capability(config.capability.grant, &SigningKey::from_bytes(&[22; 32])).unwrap();
    let recipient = config.context.expected_recipient;
    MailboxBinding::new(
        config,
        Some(DestinationReceiptSigner {
            recipient,
            signing_key: SigningKey::from_bytes(&[57; 32]),
        }),
    )
    .unwrap()
}

fn exchange() -> (MailboxBinding, MailboxBinding, Vec<u8>, Vec<u8>) {
    let request = binding(MailboxMessageKind::Call, 8, false);
    let reply = binding(MailboxMessageKind::Reply, 7, false);
    let mut options = MessageOptions::fresh(
        ContentType::parse("application/spiralcore-object", 1).unwrap(),
        now(),
        2000,
    )
    .unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    let call =
        service_sender::prepare(&request, b"call to mailbox epoch eight", &options, now()).unwrap();
    let call = encode_envelope(call.envelope()).unwrap();
    let mut destination = LocalMailboxService::new(budget()).unwrap();
    assert_eq!(
        destination
            .admit(&request, &call, now())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    let admitted = destination.receive(&request, now()).unwrap().unwrap();
    let options = MessageOptions::for_reply(&admitted, now()).unwrap();
    let response = destination
        .reply_prepared(
            &reply,
            admitted.handle(),
            service_sender::request(&reply, b"reply to mailbox epoch seven", &options, now())
                .unwrap(),
            now(),
        )
        .unwrap();
    assert_eq!(
        response.header().execution_epoch,
        ExecutionEpoch::new(7).unwrap()
    );
    (
        request,
        reply,
        call,
        encode_envelope(response.envelope()).unwrap(),
    )
}

#[test]
fn call_and_reply_use_their_own_destination_mailbox_epochs() {
    let (request, reply, call, response) = exchange();
    let mut caller = LocalMailboxService::new(budget()).unwrap();
    let handle = caller.register_call(&request, &call, now()).unwrap();
    assert_eq!(
        caller
            .admit(&reply, &response, now())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    assert_eq!(
        caller.poll_call(&request, &handle, now()).unwrap(),
        CallStatus::Replied(Some(b"reply to mailbox epoch seven".to_vec()))
    );
}

#[test]
fn receipt_tracker_correlates_reply_without_equating_distinct_mailbox_epochs() {
    let (request, reply, call, response) = exchange();
    let control = binding(MailboxMessageKind::Reply, 7, true);
    let mut tracker = ReceiptTracker::new(budget(), 4).unwrap();
    let handle = tracker
        .register_send(
            &request,
            &control,
            &SigningKey::from_bytes(&[57; 32]).verifying_key(),
            &call,
            now(),
        )
        .unwrap();
    assert_eq!(
        tracker
            .observe_application_reply(&reply, &response, now())
            .unwrap(),
        handle
    );
    assert!(
        tracker
            .facts(&request, &handle, now())
            .unwrap()
            .application_replied
    );
}

#[test]
fn reply_stale_for_its_own_destination_is_still_refused() {
    let (request, _, call, response) = exchange();
    let current_reply = binding(MailboxMessageKind::Reply, 8, false);
    let mut caller = LocalMailboxService::new(budget()).unwrap();
    let handle = caller.register_call(&request, &call, now()).unwrap();
    assert_eq!(
        caller.admit(&current_reply, &response, now()),
        Err(spiralcore_mvm_adapter::service::ServiceError::Unauthorized)
    );
    assert_eq!(
        caller.poll_call(&request, &handle, now()).unwrap(),
        CallStatus::Pending
    );
    let control = binding(MailboxMessageKind::Reply, 7, true);
    let mut tracker = ReceiptTracker::new(budget(), 4).unwrap();
    tracker
        .register_send(
            &request,
            &control,
            &SigningKey::from_bytes(&[57; 32]).verifying_key(),
            &call,
            now(),
        )
        .unwrap();
    assert_eq!(
        tracker.observe_application_reply(&current_reply, &response, now()),
        Err(spiralcore_mvm_adapter::service::ServiceError::Unauthorized)
    );
}
