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
