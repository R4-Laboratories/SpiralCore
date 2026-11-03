//! Public synthetic identities and keys for canonical service conformance only.
use ed25519_dalek::SigningKey;
use mvm_contract::protocol::mailbox::*;
use mvm_core::crypto::aead::Key;
use spiralcore_mvm_adapter::{
    receipt::receipt_content_type,
    service::{DestinationReceiptSigner, MailboxBinding, ServiceBindingConfig, ServiceTime},
};

pub fn time() -> ServiceTime {
    ServiceTime {
        unix_ms: 1500,
        monotonic_ms: 100,
    }
}
pub fn content_type() -> ContentType {
    ContentType::parse("application/x-mvm-service-test", 1).unwrap()
}
pub fn receipt_key(forward: bool) -> SigningKey {
    SigningKey::from_bytes(&[if forward { 44 } else { 33 }; 32])
}
pub fn binding(
    kind: MailboxMessageKind,
    forward: bool,
    control: bool,
    producer: bool,
) -> MailboxBinding {
    let base = crate::common::config(if forward {
        MailboxMessageKind::Call
    } else {
        MailboxMessageKind::Reply
    });
    let mut c = ServiceBindingConfig::from_direction(base).unwrap();
    c.kind = kind;
    c.context.required_operation = if kind == MailboxMessageKind::Reply {
        MailboxOperation::Reply
    } else {
        MailboxOperation::Send
    };
    c.capability.grant.operations =
        CapabilityOperations::from_operations(&[c.context.required_operation]);
    c.capability.grant.message_kinds = vec![kind];
    let content = if control {
        receipt_content_type()
    } else {
        content_type()
    };
    c.context.accepted_content_types = vec![content.clone()];
    c.capability.grant.content_types = vec![content];
    if control {
        c.capability.grant.capability_id =
            CapabilityId::from_bytes([if forward { 71 } else { 72 }; 32]);
        c.capability.grant.max_payload_bytes = 4096;
        c.capability.grant.max_queue_contribution_bytes = 16384;
        c.encryption_key = Key::from_bytes([if forward { 61 } else { 62 }; 32]);
        c.encryption_key_id = [if forward { 81 } else { 82 }; 32];
    }
    c.capability =
        mvm_core::mailbox::sign_capability(c.capability.grant, &SigningKey::from_bytes(&[22; 32]))
            .unwrap();
    if !producer {
        c.signing_key = None;
    }
    MailboxBinding::new(
        c,
        (!producer && !control).then(|| DestinationReceiptSigner {
            recipient: PrincipalId::from_bytes([if forward { 3 } else { 2 }; 32]),
            signing_key: receipt_key(forward),
        }),
    )
    .unwrap()
}
