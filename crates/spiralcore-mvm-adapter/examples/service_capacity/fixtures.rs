//! Public synthetic credentials. This executable is a local conformance probe.
#[path = "../../tests/common/mod.rs"]
mod common;
use ed25519_dalek::SigningKey;
use mvm_contract::protocol::mailbox::*;
use mvm_core::{
    crypto::aead::Key,
    mailbox::{sign_capability, transport::sign_transport_binding},
};
use spiralcore_mvm_adapter::{
    Policy,
    receipt::receipt_content_type,
    service::{DestinationReceiptSigner, MailboxBinding, ServiceBindingConfig},
};

pub fn content_type() -> ContentType {
    ContentType::parse("application/x-spiralcore-capacity", 1).unwrap()
}
pub fn receipt_key(forward: bool) -> SigningKey {
    SigningKey::from_bytes(&[if forward { 44 } else { 33 }; 32])
}
pub fn reply_to(peer: u8) -> ReplyRoute {
    let c = config(peer, MailboxMessageKind::Call, true, false);
    c.reply_to.unwrap()
}
fn config(
    peer: u8,
    kind: MailboxMessageKind,
    forward: bool,
    control: bool,
) -> spiralcore_mvm_adapter::DirectionConfig {
    let mut c = common::config(if forward {
        MailboxMessageKind::Call
    } else {
        MailboxMessageKind::Reply
    });
    let mut context = c.policy.snapshot().unwrap();
    context.expected_tenant = TenantId::from_bytes([peer + 1; 32]);
    context.expected_authorization_domain = AuthorizationDomainId::from_bytes([peer + 1; 32]);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    context.now_unix_ms = now;
    context.required_operation = if kind == MailboxMessageKind::Reply {
        MailboxOperation::Reply
    } else {
        MailboxOperation::Send
    };
    let content = if control {
        receipt_content_type()
    } else {
        content_type()
    };
    c.policy = Policy::new(context.clone()).unwrap();
    c.kind = kind;
    c.capability.grant.address.tenant = context.expected_tenant;
    c.capability.grant.address.authorization_domain = context.expected_authorization_domain;
    c.capability.grant.operations =
        CapabilityOperations::from_operations(&[context.required_operation]);
    c.capability.grant.message_kinds = vec![kind];
    if kind == MailboxMessageKind::Cast && !control {
        c.capability.grant.capability_id =
            CapabilityId::from_bytes([if forward { 91 } else { 92 }; 32]);
    }
    c.capability.grant.content_types = vec![content];
    c.capability.grant.max_messages_per_minute = 100_000;
    c.capability.grant.not_before_unix_ms = now - 10_000;
    c.capability.grant.expires_at_unix_ms = now + 240_000;
    c.binding.claims.address = c.capability.grant.address;
    c.binding.claims.not_before_unix_ms = now - 10_000;
    c.binding.claims.expires_at_unix_ms = now + 240_000;
    if control {
        c.capability.grant.capability_id =
            CapabilityId::from_bytes([if forward { 71 } else { 72 }; 32]);
        c.capability.grant.max_payload_bytes = 4096;
        c.capability.grant.max_queue_contribution_bytes = 16384;
        c.encryption_key = Key::from_bytes([if forward { 61 } else { 62 }; 32]);
        c.key_id = [if forward { 81 } else { 82 }; 32];
    }
    c.capability = sign_capability(c.capability.grant, &SigningKey::from_bytes(&[22; 32])).unwrap();
    c.binding =
        sign_transport_binding(c.binding.claims, &SigningKey::from_bytes(&[22; 32])).unwrap();
    c.reply_to = if kind == MailboxMessageKind::Call {
        Some(ReplyRoute {
            address: MailboxAddress {
                mailbox: MailboxId::from_bytes([14; 32]),
                recipient: context.expected_sender,
                ..c.capability.grant.address
            },
            capability_id: CapabilityId::from_bytes([11; 32]),
        })
    } else {
        None
    };
    c
}
pub fn binding(
    peer: u8,
    kind: MailboxMessageKind,
    forward: bool,
    control: bool,
    producer: bool,
) -> MailboxBinding {
    let c = config(peer, kind, forward, control);
    let mut c = ServiceBindingConfig::from_direction(c).unwrap();
    c.context.accepted_content_types = c.capability.grant.content_types.clone();
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
