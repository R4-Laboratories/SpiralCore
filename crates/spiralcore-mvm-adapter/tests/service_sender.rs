#![cfg(feature = "live")]
mod common;

use ed25519_dalek::SigningKey;
use mvm_contract::protocol::mailbox::*;
use spiralcore_mvm_adapter::{
    service::{
        DestinationReceiptSigner, HostBudget, LocalMailboxService, MailboxBinding,
        ServiceBindingConfig, ServiceLimits, ServiceTime,
    },
    service_sender::{self, MessageOptions},
};

fn time() -> ServiceTime {
    ServiceTime {
        unix_ms: 1500,
        monotonic_ms: 1,
    }
}
fn binding() -> MailboxBinding {
    MailboxBinding::new(
        ServiceBindingConfig::from_direction(common::config(MailboxMessageKind::Call)).unwrap(),
        Some(DestinationReceiptSigner {
            recipient: PrincipalId::from_bytes([3; 32]),
            signing_key: SigningKey::from_bytes(&[57; 32]),
        }),
    )
    .unwrap()
}
#[test]
fn immutable_options_reseal_as_one_canonical_request() {
    let binding = binding();
    let mut options = MessageOptions::fresh(
        ContentType::parse("application/spiralcore-object", 1).unwrap(),
        time(),
        2000,
    )
    .unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    let first = service_sender::prepare(&binding, b"binary\0\0", &options, time()).unwrap();
    let mut retry_options = options.clone();
    retry_options.delivery_id = DeliveryId::from_bytes([84; 32]);
    let later = ServiceTime {
        unix_ms: 1501,
        monotonic_ms: 2,
    };
    let retry = service_sender::prepare(&binding, b"binary\0\0", &retry_options, later).unwrap();
    assert_ne!(first.envelope().ciphertext, retry.envelope().ciphertext);
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let first = mvm_core::mailbox::encode_envelope(first.envelope()).unwrap();
    let retry = mvm_core::mailbox::encode_envelope(retry.envelope()).unwrap();
    assert_eq!(
        service
            .admit(&binding, &first, time())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    assert_eq!(
        service
            .admit(&binding, &retry, later)
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::Duplicate
    );
    let message = service.receive(&binding, later).unwrap().unwrap();
    assert_eq!(message.payload(), b"binary\0\0");
    assert_eq!(message.header().message_id, options.message_id);
    assert!(service.receive(&binding, later).unwrap().is_none());
}
#[test]
fn sender_refuses_wrong_content_kind_shape_and_payload_bound() {
    let binding = binding();
    let mut options = MessageOptions::fresh(
        ContentType::parse("application/other", 1).unwrap(),
        time(),
        2000,
    )
    .unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    assert!(service_sender::prepare(&binding, b"x", &options, time()).is_err());
    options.content_type = ContentType::parse("application/spiralcore-object", 1).unwrap();
    options.reply_to = None;
    assert!(service_sender::prepare(&binding, b"x", &options, time()).is_err());
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    assert!(service_sender::prepare(&binding, &vec![0; 65537], &options, time()).is_err());
}

#[test]
fn persistent_ordering_reuses_one_scope_and_keeps_retry_identity() {
    use spiralcore_mvm_adapter::{issuance::IssuancePolicy, service_sender::OrderingStream};
    let binding = binding();
    let mut ordered = OrderingStream::new();
    let content = ContentType::parse("application/spiralcore-object", 1).unwrap();
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let mut scope = None;
    for sequence in 1..=4 {
        let mut options = ordered
            .next_for_binding(
                &binding,
                content.clone(),
                time(),
                2000,
                IssuancePolicy::Exact,
            )
            .unwrap();
        options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
        assert_eq!(options.sequence.get(), sequence);
        if let Some(ref scope) = scope {
            assert_eq!(&options.ordering_scope, scope);
        } else {
            scope = Some(options.ordering_scope.clone());
        }
        let payload = [sequence as u8, 0, 255, 0];
        let prepared = service_sender::prepare(&binding, &payload, &options, time()).unwrap();
        let encoded = mvm_core::mailbox::encode_envelope(prepared.envelope()).unwrap();
        assert_eq!(
            service
                .admit(&binding, &encoded, time())
                .unwrap()
                .receipt
                .outcome,
            DeliveryOutcome::AcceptedEphemeral
        );
        let proof = service.receive(&binding, time()).unwrap().unwrap();
        assert_eq!(proof.payload(), payload);
        assert_eq!(proof.header().sequence.get(), sequence);
        if sequence == 1 {
            let mut wrong_reply_stream = OrderingStream::new();
            assert!(
                wrong_reply_stream
                    .next_reply(&binding, &proof, time(), IssuancePolicy::Exact)
                    .is_err()
            );
            let first = wrong_reply_stream
                .next_for_binding(
                    &binding,
                    content.clone(),
                    time(),
                    2000,
                    IssuancePolicy::Exact,
                )
                .unwrap();
            assert_eq!(first.sequence.get(), 1);
        }
        let mut retry = options.clone();
        retry.delivery_id = DeliveryId::from_bytes(rand::random());
        let retry = service_sender::prepare(&binding, &payload, &retry, time()).unwrap();
        let retry = mvm_core::mailbox::encode_envelope(retry.envelope()).unwrap();
        assert_eq!(
            service
                .admit(&binding, &retry, time())
                .unwrap()
                .receipt
                .outcome,
            DeliveryOutcome::Duplicate
        );
        assert!(service.receive(&binding, time()).unwrap().is_none());
    }
}

#[test]
fn invalid_options_do_not_skip_sequence_or_switch_stream_owner() {
    use spiralcore_mvm_adapter::{
        issuance::IssuancePolicy, service::ServiceError, service_sender::OrderingStream,
    };
    let binding = binding();
    let mut ordered = OrderingStream::new();
    let content = ContentType::parse("application/spiralcore-object", 1).unwrap();
    assert!(
        ordered
            .next_for_binding(
                &binding,
                content.clone(),
                time(),
                1500,
                IssuancePolicy::Exact
            )
            .is_err()
    );
    assert!(
        ordered
            .next_for_binding(
                &binding,
                ContentType::parse("application/wrong", 1).unwrap(),
                time(),
                2000,
                IssuancePolicy::Exact
            )
            .is_err()
    );
    let first = ordered
        .next_for_binding(
            &binding,
            content.clone(),
            time(),
            2000,
            IssuancePolicy::Exact,
        )
        .unwrap();
    assert_eq!(first.sequence.get(), 1);
    let other =
        ServiceBindingConfig::from_direction(common::config(MailboxMessageKind::Cast)).unwrap();
    // The stream is bound to its canonical address/sender/epoch/kind, not a
    // transport authorization token or a bearer capability supplied by payload.
    let other = MailboxBinding::new(other, None).unwrap();
    assert_eq!(
        ordered
            .next_for_binding(&other, content.clone(), time(), 2000, IssuancePolicy::Exact)
            .err(),
        Some(ServiceError::Unauthorized)
    );
    let next = ordered
        .next_for_binding(&binding, content, time(), 2000, IssuancePolicy::Exact)
        .unwrap();
    assert_eq!(next.sequence.get(), 2);
}
