//! Local metadata builders; MVM owns sealing and transport authorization.
use crate::{
    receipt::receipt_content_type,
    service::{AdmittedMessage, MailboxBinding, ServiceError, ServiceTime},
};
use mvm_contract::protocol::mailbox::*;
use mvm_core::mailbox::{
    SealRequest,
    transport::{PreparedMailboxEnvelope, prepare_mailbox_envelope},
};
use std::fmt::Write;
type Result<T> = std::result::Result<T, ServiceError>;

/// Authenticated producer namespace only, never a receiver authority lease.
pub fn object_context(
    binding: &MailboxBinding,
    prepared: &PreparedMailboxEnvelope,
    now: ServiceTime,
) -> Result<spiralcore_transfer::endpoint::Context> {
    let encoded = mvm_core::mailbox::encode_envelope(prepared.envelope())
        .map_err(|_| ServiceError::Malformed)?;
    binding.with_verified(&encoded, now, |opened, _, context, _| {
        if opened.header().kind != MailboxMessageKind::Call
            || opened.header().content_type
                != crate::content_type().map_err(|_| ServiceError::State)?
        {
            return Err(ServiceError::Unauthorized);
        }
        crate::direction::verified_object_context(context, binding.capability_grant())
            .map_err(|_| ServiceError::State)
    })
}

/// Reuse inner metadata unchanged when retrying a stable message. Only the
/// outer delivery identifier may be changed without changing its semantics.
#[derive(Clone)]
pub struct MessageOptions {
    pub message_id: MessageId,
    pub correlation_id: CorrelationId,
    pub causation_id: CausationId,
    pub ordering_scope: OrderingScope,
    pub sequence: SequenceNumber,
    pub issued_at_unix_ms: i64,
    pub expires_at_unix_ms: i64,
    pub reply_to: Option<ReplyRoute>,
    pub delivery_id: DeliveryId,
    pub nonce: ReplayNonce,
    pub content_type: ContentType,
}
impl MessageOptions {
    /// Construct immutable timestamps once, using the trusted binding's signed
    /// validity interval. Retries reuse these options without reissuing them.
    pub fn fresh_for_binding(
        binding: &MailboxBinding,
        content_type: ContentType,
        now: ServiceTime,
        expiry: i64,
        policy: crate::issuance::IssuancePolicy,
    ) -> Result<Self> {
        binding.with_sender(now, |config, _, _| {
            let window = policy.window(
                now,
                expiry,
                config
                    .capability
                    .grant
                    .not_before_unix_ms
                    .max(config.binding.claims.not_before_unix_ms),
                config
                    .capability
                    .grant
                    .expires_at_unix_ms
                    .min(config.binding.claims.expires_at_unix_ms),
            )?;
            let mut options = Self::fresh(content_type, now, window.expires)?;
            options.issued_at_unix_ms = window.issued;
            Ok(options)
        })
    }
    pub fn for_reply_with_policy(
        binding: &MailboxBinding,
        request: &AdmittedMessage,
        now: ServiceTime,
        policy: crate::issuance::IssuancePolicy,
    ) -> Result<Self> {
        if request.header().kind != MailboxMessageKind::Call {
            return Err(ServiceError::Unauthorized);
        }
        let mut options = Self::fresh_for_binding(
            binding,
            request.header().content_type.clone(),
            now,
            request.header().expires_at_unix_ms,
            policy,
        )?;
        options.correlation_id = request.header().correlation_id;
        options.causation_id = CausationId::from_bytes(*request.header().message_id.as_bytes());
        Ok(options)
    }
    pub fn fresh(content_type: ContentType, now: ServiceTime, expiry: i64) -> Result<Self> {
        if expiry <= now.unix_ms {
            return Err(ServiceError::Expired);
        }
        let message_id = MessageId::from_bytes(rand::random());
        let mut scope = String::with_capacity(64);
        for byte in message_id.as_bytes() {
            write!(&mut scope, "{byte:02x}").map_err(|_| ServiceError::State)?;
        }
        Ok(Self {
            message_id,
            correlation_id: CorrelationId::from_bytes(rand::random()),
            causation_id: CausationId::from_bytes([0; 32]),
            ordering_scope: OrderingScope::parse(scope).map_err(|_| ServiceError::Malformed)?,
            sequence: SequenceNumber::new(1).map_err(|_| ServiceError::Malformed)?,
            issued_at_unix_ms: now.unix_ms,
            expires_at_unix_ms: expiry,
            reply_to: None,
            delivery_id: DeliveryId::from_bytes(rand::random()),
            nonce: ReplayNonce::from_bytes(rand::random()),
            content_type,
        })
    }
    pub fn for_reply(request: &AdmittedMessage, now: ServiceTime) -> Result<Self> {
        if request.header().kind != MailboxMessageKind::Call {
            return Err(ServiceError::Unauthorized);
        }
        let mut options = Self::fresh(
            request.header().content_type.clone(),
            now,
            request.header().expires_at_unix_ms,
        )?;
        options.correlation_id = request.header().correlation_id;
        options.causation_id = CausationId::from_bytes(*request.header().message_id.as_bytes());
        Ok(options)
    }
}

pub fn request<'a>(
    binding: &MailboxBinding,
    payload: &'a [u8],
    options: &MessageOptions,
    now: ServiceTime,
) -> Result<SealRequest<'a>> {
    binding.with_sender(now, |config, context, _| {
        if options.content_type == receipt_content_type()
            || !config
                .capability
                .grant
                .content_types
                .contains(&options.content_type)
            || (config.kind == MailboxMessageKind::Call) != options.reply_to.is_some()
        {
            return Err(ServiceError::Unauthorized);
        }
        if payload.len() > 65_536
            || payload.len() as u64 > u64::from(config.capability.grant.max_payload_bytes)
        {
            return Err(ServiceError::Limit);
        }
        let expiry = options.expires_at_unix_ms;
        if expiry <= now.unix_ms {
            return Err(ServiceError::Expired);
        }
        if options.issued_at_unix_ms > now.unix_ms
            || expiry > config.capability.grant.expires_at_unix_ms
            || expiry > config.binding.claims.expires_at_unix_ms
            || expiry
                > options
                    .issued_at_unix_ms
                    .checked_add(30_000)
                    .ok_or(ServiceError::Clock)?
        {
            return Err(ServiceError::Unauthorized);
        }
        let routing = &config.binding.claims;
        Ok(SealRequest {
            routing: MailboxRoutingFrame {
                protocol_version: ProtocolVersion::V1,
                authorization_domain_route_label: routing.authorization_domain_route_label,
                destination_route_token: routing.destination_route_token,
                delivery_id: options.delivery_id,
                ciphertext_len: 0,
                padding: CiphertextPadding::Exact,
                expires_at_unix_ms: expiry,
                hop_limit: 8,
                transport_binding_id: routing.binding_id,
            },
            header: MailboxEnvelopeHeader {
                protocol_version: ProtocolVersion::V1,
                signature_algorithm: SignatureAlgorithm::ED25519,
                tenant: context.expected_tenant,
                authorization_domain: context.expected_authorization_domain,
                sender: context.expected_sender,
                recipient: context.expected_recipient,
                mailbox: context.expected_mailbox,
                sender_boot: config.expected_sender_boot,
                execution_epoch: context.current_execution_epoch,
                message_id: options.message_id,
                correlation_id: options.correlation_id,
                causation_id: options.causation_id,
                kind: config.kind,
                reply_to: options.reply_to,
                ordering_scope: options.ordering_scope.clone(),
                sequence: options.sequence,
                issued_at_unix_ms: options.issued_at_unix_ms,
                expires_at_unix_ms: expiry,
                nonce: options.nonce,
                content_type: options.content_type.clone(),
                capability_id: config.capability.grant.capability_id,
                policy_epoch: context.current_policy_epoch,
                encryption: EncryptionContext {
                    algorithm: EncryptionAlgorithm::AES_256_GCM,
                    key_id: config.encryption_key_id,
                },
                plaintext_len: u32::try_from(payload.len()).map_err(|_| ServiceError::Limit)?,
            },
            plaintext: payload,
        })
    })
}
pub fn prepare(
    binding: &MailboxBinding,
    payload: &[u8],
    options: &MessageOptions,
    now: ServiceTime,
) -> Result<PreparedMailboxEnvelope> {
    let request = request(binding, payload, options, now)?;
    binding.with_sender(now, |config, _, _| {
        prepare_mailbox_envelope(
            request,
            &config.encryption_key,
            config
                .signing_key
                .as_ref()
                .ok_or(ServiceError::Unauthorized)?,
        )
        .map_err(|_| ServiceError::Malformed)
    })
}
