#![allow(dead_code)] // Shared fixtures are compiled separately for each integration target.

use ed25519_dalek::SigningKey;
use mvm_contract::protocol::mailbox::*;
use mvm_core::{
    crypto::aead::Key,
    mailbox::{
        OpenContext, sign_capability,
        transport::{
            RouteId, RouteLocator, TransportAssurance, TransportBindingClaims, TransportIdentity,
            TransportPath, sign_transport_binding,
        },
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
            destination_route_token: DestinationRouteToken::from_bytes(
                [if reply { 26 } else { 16 }; 32],
            ),
            path: TransportPath::Direct,
            locator: RouteLocator::parse(if reply { "memory:reply" } else { "memory:call" })
                .unwrap(),
            assurance: TransportAssurance::Development,
            policy_epoch: 4,
            not_before_unix_ms: 900,
            expires_at_unix_ms: 3_000,
        },
        &authority,
    )
    .unwrap();
    let signing_key = SigningKey::from_bytes(&[if reply { 24 } else { 21 }; 32]);
    DirectionConfig {
        policy,
        capability,
        binding,
        sender_verifying_key: signing_key.verifying_key(),
        signing_key: Some(signing_key),
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

pub fn sealed_payload(config: &DirectionConfig, payload: &[u8]) -> Vec<u8> {
    sealed_payload_at(config, payload, 1_000, 2_000)
}

pub fn sealed_payload_at(
    config: &DirectionConfig,
    payload: &[u8],
    issued_at_unix_ms: i64,
    expires_at_unix_ms: i64,
) -> Vec<u8> {
    use mvm_core::mailbox::{SealRequest, encode_envelope, seal_envelope};
    let context = config.policy.snapshot().unwrap();
    let route = &config.binding.claims;
    let envelope = seal_envelope(
        SealRequest {
            routing: MailboxRoutingFrame {
                protocol_version: ProtocolVersion::V1,
                authorization_domain_route_label: route.authorization_domain_route_label,
                destination_route_token: route.destination_route_token,
                delivery_id: DeliveryId::from_bytes(rand::random()),
                ciphertext_len: 0,
                padding: CiphertextPadding::Exact,
                expires_at_unix_ms,
                hop_limit: 8,
                transport_binding_id: route.binding_id,
            },
            header: MailboxEnvelopeHeader {
                protocol_version: ProtocolVersion::V1,
                signature_algorithm: SignatureAlgorithm::ED25519,
                tenant: context.expected_tenant,
                authorization_domain: context.expected_authorization_domain,
                sender: context.expected_sender,
                recipient: context.expected_recipient,
                mailbox: context.expected_mailbox,
                sender_boot: config.boot,
                execution_epoch: context.current_execution_epoch,
                message_id: MessageId::from_bytes(rand::random()),
                correlation_id: CorrelationId::from_bytes(rand::random()),
                causation_id: CausationId::from_bytes([0; 32]),
                kind: config.kind,
                reply_to: config.reply_to,
                ordering_scope: OrderingScope::parse("conformance").unwrap(),
                sequence: SequenceNumber::new(1).unwrap(),
                issued_at_unix_ms,
                expires_at_unix_ms,
                nonce: ReplayNonce::from_bytes(rand::random()),
                content_type: config.capability.grant.content_types[0].clone(),
                capability_id: config.capability.grant.capability_id,
                policy_epoch: context.current_policy_epoch,
                encryption: EncryptionContext {
                    algorithm: EncryptionAlgorithm::AES_256_GCM,
                    key_id: config.key_id,
                },
                plaintext_len: payload.len().try_into().unwrap(),
            },
            plaintext: payload,
        },
        &config.encryption_key,
        config.signing_key.as_ref().unwrap(),
    )
    .unwrap();
    encode_envelope(&envelope).unwrap()
}

pub struct Store(pub std::path::PathBuf);
impl Store {
    pub fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "spiral-mvm-tests-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        )))
    }
    pub fn open(
        &self,
        authority: spiralcore_mvm_adapter::MailboxAuthority,
    ) -> spiralcore_transfer::endpoint::Endpoint<spiralcore_mvm_adapter::MailboxAuthority> {
        use spiralcore_transfer::endpoint::{ColdGraphPolicy, Endpoint, Limits};
        Endpoint::open_with_policy(
            &self.0,
            Limits::default(),
            authority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap()
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn random(len: usize, mut seed: u64) -> Vec<u8> {
    (0..len)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u8
        })
        .collect()
}

pub fn edited(bytes: &[u8]) -> Vec<u8> {
    let mut result = bytes.to_vec();
    let middle = result.len() / 2;
    for byte in &mut result[middle..middle + 32] {
        *byte ^= 0xa5;
    }
    result
}
