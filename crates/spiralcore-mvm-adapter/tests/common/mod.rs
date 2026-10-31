use ed25519_dalek::SigningKey;
use mvm_contract::protocol::mailbox::*;
use mvm_core::{
    crypto::aead::Key,
    mailbox::{
        sign_capability,
        transport::{
            sign_transport_binding, RouteId, RouteLocator, TransportAssurance,
            TransportBindingClaims, TransportIdentity, TransportPath,
        },
        OpenContext,
    },
};
use spiralcore_mvm_adapter::{DirectionConfig, Policy};

// Public synthetic keys and identities, used only by conformance tests.
pub fn config(kind: MailboxMessageKind) -> DirectionConfig {
    let reply = kind == MailboxMessageKind::Reply;
    let sender = PrincipalId::from_bytes([if reply { 3 } else { 2 }; 32]);
    let recipient = PrincipalId::from_bytes([if reply { 2 } else { 3 }; 32]);
    let address = MailboxAddress {
        tenant: TenantId::from_bytes([19; 32]),
        authorization_domain: AuthorizationDomainId::from_bytes([1; 32]),
        mailbox: MailboxId::from_bytes([if reply { 14 } else { 4 }; 32]),
        recipient,
    };
    let content = ContentType::parse("application/spiralcore-object", 1).unwrap();
    let epoch = ExecutionEpoch::new(7).unwrap();
    let issuer = PrincipalId::from_bytes([12; 32]);
    let authority = SigningKey::from_bytes(&[22; 32]);
    let capability = sign_capability(
        MailboxCapabilityGrant {
            protocol_version: ProtocolVersion::V1,
            signature_algorithm: SignatureAlgorithm::ED25519,
            capability_id: CapabilityId::from_bytes([if reply { 11 } else { 10 }; 32]),
            issuer,
            subject: sender,
            address,
            operations: CapabilityOperations::from_operations(&[MailboxOperation::Send]),
            message_kinds: vec![kind],
            content_types: vec![content.clone()],
            max_payload_bytes: 65_536,
            max_messages_per_minute: 4096,
            max_outstanding_calls: 4,
            max_queue_contribution_bytes: 262_144,
            minimum_execution_epoch: epoch,
            maximum_execution_epoch: epoch,
            policy_epoch: 4,
            not_before_unix_ms: 900,
            expires_at_unix_ms: 3_000,
            revocation_id: RevocationId::from_bytes([if reply { 14 } else { 13 }; 32]),
            delegation_depth: 0,
        },
        &authority,
    )
    .unwrap();
    let policy = Policy::new(OpenContext {
        expected_tenant: address.tenant,
        expected_authorization_domain: address.authorization_domain,
        expected_mailbox: address.mailbox,
        expected_sender: sender,
        expected_recipient: recipient,
        acting_principal: sender,
        expected_capability_issuer: issuer,
        current_execution_epoch: epoch,
        current_policy_epoch: 4,
        required_operation: MailboxOperation::Send,
        now_unix_ms: 1_500,
        accepted_content_types: vec![content],
        revoked_capabilities: vec![],
    })
    .unwrap();
    let binding = sign_transport_binding(
        TransportBindingClaims {
            protocol_version: ProtocolVersion::V1,
            signature_algorithm: SignatureAlgorithm::ED25519,
            binding_id: TransportBindingId::from_bytes([if reply { 28 } else { 18 }; 32]),
            issuer,
            address,
            execution_epoch: epoch,
            transport_identity: TransportIdentity::from_bytes([if reply { 29 } else { 19 }; 32]),
            route_id: RouteId::from_bytes([if reply { 30 } else { 20 }; 32]),
            authorization_domain_route_label: AuthorizationDomainRouteLabel::from_bytes([15; 32]),
            destination_route_token: DestinationRouteToken::from_bytes([if reply { 26 } else { 16 }; 32]),
            path: TransportPath::Direct,
            locator: RouteLocator::parse(if reply { "memory:reply" } else { "memory:call" }).unwrap(),
            assurance: TransportAssurance::Development,
            policy_epoch: 4,
            not_before_unix_ms: 900,
            expires_at_unix_ms: 3_000,
        },
        &authority,
    )
    .unwrap();
    DirectionConfig {
        policy,
        capability,
        binding,
        signing_key: SigningKey::from_bytes(&[if reply { 24 } else { 21 }; 32]),
        encryption_key: Key::from_bytes([if reply { 25 } else { 23 }; 32]),
        authority_key: authority.verifying_key(),
        boot: BootId::from_bytes([if reply { 15 } else { 5 }; 32]),
        key_id: [if reply { 21 } else { 11 }; 32],
        kind,
        reply_to: if reply {
            None
        } else {
            Some(ReplyRoute {
                address: MailboxAddress {
                    mailbox: MailboxId::from_bytes([14; 32]),
                    recipient: sender,
                    ..address
                },
                capability_id: CapabilityId::from_bytes([11; 32]),
            })
        },
    }
}
