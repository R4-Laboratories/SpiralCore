use crate::{
    Error, MAX_ENVELOPE_BYTES, MAX_REPLAY_IDENTITIES, MailboxAuthority, Policy, Result,
    content_type,
};
use ed25519_dalek::{SigningKey, VerifyingKey};
use mvm_contract::protocol::mailbox::*;
#[cfg(not(feature = "live"))]
use mvm_core::mailbox::open_envelope;
#[cfg(feature = "live")]
use mvm_core::mailbox::open_verified_envelope;
use mvm_core::{
    crypto::aead::Key,
    mailbox::{
        OpenContext, ReplayIdentity, ReplayProtector, SealRequest, decode_envelope,
        transport::{
            AuthorizedRoute, CancellationToken, ChannelLimits, OutboundAuthorization,
            PeerTransport, RouteId, SignedTransportBinding, TransportAssurance,
            TransportBindingContext, authorize_outbound, authorize_route, prepare_mailbox_envelope,
        },
    },
};
use spiralcore_transfer::{
    endpoint::{Context, Endpoint},
    protocol::Frame,
};
use std::collections::BTreeSet;
#[cfg(feature = "live")]
#[path = "live.rs"]
pub mod live;

pub struct DirectionConfig {
    pub policy: Policy,
    pub capability: SignedMailboxCapability,
    pub binding: SignedTransportBinding,
    /// Absent on a receive-only endpoint. Never provision a remote signing secret.
    pub signing_key: Option<SigningKey>,
    pub sender_verifying_key: VerifyingKey,
    pub encryption_key: Key,
    pub authority_key: VerifyingKey,
    pub boot: BootId,
    pub key_id: [u8; 32],
    pub kind: MailboxMessageKind,
    pub reply_to: Option<ReplyRoute>,
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct Counters {
    pub frame_bytes_sent: u64,
    pub envelope_bytes_sent: u64,
    pub envelopes_sent: u64,
    pub envelope_bytes_received: u64,
    pub envelopes_received: u64,
    pub frame_bytes_opened: u64,
}
impl Counters {
    pub fn checked_add(self, rhs: Self) -> Result<Self> {
        Ok(Self {
            frame_bytes_sent: add(self.frame_bytes_sent, rhs.frame_bytes_sent)?,
            envelope_bytes_sent: add(self.envelope_bytes_sent, rhs.envelope_bytes_sent)?,
            envelopes_sent: add(self.envelopes_sent, rhs.envelopes_sent)?,
            envelope_bytes_received: add(
                self.envelope_bytes_received,
                rhs.envelope_bytes_received,
            )?,
            envelopes_received: add(self.envelopes_received, rhs.envelopes_received)?,
            frame_bytes_opened: add(self.frame_bytes_opened, rhs.frame_bytes_opened)?,
        })
    }
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or(Error::Limit)
}

#[derive(Default)]
struct BoundedReplay {
    seen: BTreeSet<ReplayIdentity>,
    capacity_hit: bool,
}
impl ReplayProtector for BoundedReplay {
    fn check_and_record(&mut self, identity: ReplayIdentity) -> bool {
        if self.seen.contains(&identity) {
            return false;
        }
        if self.seen.len() >= MAX_REPLAY_IDENTITIES {
            self.capacity_hit = true;
            return false;
        }
        self.seen.insert(identity)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SentMetadata {
    message_id: MessageId,
    correlation_id: CorrelationId,
}

pub struct Direction {
    config: DirectionConfig,
    authority: MailboxAuthority,
    replay: BoundedReplay,
    sequence: u64,
    counters: Counters,
    pub(crate) last_sent: Option<SentMetadata>,
    #[cfg(feature = "live")]
    live_state: live::DirectionState,
}
impl Direction {
    pub fn new(config: DirectionConfig) -> Result<Self> {
        let content = content_type()?;
        if !matches!(
            config.kind,
            MailboxMessageKind::Call | MailboxMessageKind::Reply
        ) || config.capability.grant.message_kinds != [config.kind]
            || config.capability.grant.content_types != [content]
            || (config.kind == MailboxMessageKind::Call) != config.reply_to.is_some()
            || config
                .signing_key
                .as_ref()
                .is_some_and(|key| key.verifying_key() != config.sender_verifying_key)
        {
            return Err(Error::Policy);
        }
        // Shape checks do not authenticate a grant and cannot mint authority.
        let authority = MailboxAuthority::new(config.policy.clone());
        Ok(Self {
            config,
            authority,
            replay: BoundedReplay::default(),
            sequence: 0,
            counters: Counters::default(),
            last_sent: None,
            #[cfg(feature = "live")]
            live_state: live::DirectionState::default(),
        })
    }
    pub fn route_id(&self) -> RouteId {
        self.config.binding.claims.route_id
    }
    pub fn authority(&self) -> MailboxAuthority {
        self.authority.clone()
    }
    pub fn counters(&self) -> Counters {
        self.counters
    }
    pub fn policy(&self) -> Policy {
        self.config.policy.clone()
    }

    fn route(&self, context: &OpenContext) -> Result<AuthorizedRoute> {
        authorize_route(
            &self.config.binding,
            &TransportBindingContext {
                expected_issuer: context.expected_capability_issuer,
                expected_address: MailboxAddress {
                    tenant: context.expected_tenant,
                    authorization_domain: context.expected_authorization_domain,
                    mailbox: context.expected_mailbox,
                    recipient: context.expected_recipient,
                },
                current_execution_epoch: context.current_execution_epoch,
                current_policy_epoch: context.current_policy_epoch,
                minimum_assurance: TransportAssurance::Development,
                now_unix_ms: context.now_unix_ms,
            },
            &self.config.authority_key,
        )
        .map_err(Error::Route)
    }
    fn check_frame_kind(&self, frame: &Frame) -> Result<()> {
        let valid = match self.config.kind {
            MailboxMessageKind::Call => matches!(frame.kind, 1..=6),
            MailboxMessageKind::Reply => matches!(frame.kind, 101..=105 | 127),
            _ => false,
        };
        if !valid {
            return Err(Error::Policy);
        }
        Ok(())
    }
    pub fn send(
        &mut self,
        frame: &Frame,
        transport: &impl PeerTransport,
        cancel: &CancellationToken,
    ) -> Result<usize> {
        self.send_correlated(frame, transport, cancel, None)
    }
    pub(crate) fn send_correlated(
        &mut self,
        frame: &Frame,
        transport: &impl PeerTransport,
        cancel: &CancellationToken,
        request: Option<SentMetadata>,
    ) -> Result<usize> {
        self.send_correlated_lifetime(frame, transport, cancel, request, None)
    }

    fn send_correlated_lifetime(
        &mut self,
        frame: &Frame,
        transport: &impl PeerTransport,
        cancel: &CancellationToken,
        request: Option<SentMetadata>,
        lifetime: Option<(i64, i64)>,
    ) -> Result<usize> {
        self.check_frame_kind(frame)?;
        let signing_key = self.config.signing_key.as_ref().ok_or(Error::Policy)?;
        let bytes = frame.encode()?;
        let (context, generation) = self.config.policy.versioned()?;
        // Reverify the signed route on every send: an old AuthorizedRoute alone
        // does not carry the live expiry/epoch checks required by this profile.
        let route = self.route(&context)?;
        let next = self.sequence.checked_add(1).ok_or(Error::Limit)?;
        let expiry = context
            .now_unix_ms
            .checked_add(30_000)
            .ok_or(Error::Limit)?
            .min(self.config.capability.grant.expires_at_unix_ms)
            .min(self.config.binding.claims.expires_at_unix_ms)
            .min(lifetime.map_or(i64::MAX, |(_, expiry)| expiry));
        if expiry <= context.now_unix_ms {
            return Err(Error::Policy);
        }
        let metadata = SentMetadata {
            message_id: MessageId::from_bytes(rand::random()),
            correlation_id: request.map_or_else(
                || CorrelationId::from_bytes(rand::random()),
                |m| m.correlation_id,
            ),
        };
        let header = MailboxEnvelopeHeader {
            protocol_version: ProtocolVersion::V1,
            signature_algorithm: SignatureAlgorithm::ED25519,
            tenant: context.expected_tenant,
            authorization_domain: context.expected_authorization_domain,
            sender: context.expected_sender,
            recipient: context.expected_recipient,
            mailbox: context.expected_mailbox,
            sender_boot: self.config.boot,
            execution_epoch: context.current_execution_epoch,
            message_id: metadata.message_id,
            correlation_id: metadata.correlation_id,
            causation_id: CausationId::from_bytes(
                request.map_or([0; 32], |m| *m.message_id.as_bytes()),
            ),
            kind: self.config.kind,
            reply_to: self.config.reply_to,
            ordering_scope: OrderingScope::parse("spiralcore-object-v1")
                .map_err(|_| Error::Policy)?,
            sequence: SequenceNumber::new(next).map_err(|_| Error::Limit)?,
            issued_at_unix_ms: context
                .now_unix_ms
                .checked_sub(lifetime.map_or(0, |(backdate, _)| backdate))
                .ok_or(Error::Limit)?,
            expires_at_unix_ms: expiry,
            nonce: ReplayNonce::from_bytes(rand::random()),
            content_type: content_type()?,
            capability_id: self.config.capability.grant.capability_id,
            policy_epoch: context.current_policy_epoch,
            encryption: EncryptionContext {
                algorithm: EncryptionAlgorithm::AES_256_GCM,
                key_id: self.config.key_id,
            },
            plaintext_len: u32::try_from(bytes.len()).map_err(|_| Error::Limit)?,
        };
        let binding = &self.config.binding.claims;
        let prepared = prepare_mailbox_envelope(
            SealRequest {
                routing: MailboxRoutingFrame {
                    protocol_version: ProtocolVersion::V1,
                    authorization_domain_route_label: binding.authorization_domain_route_label,
                    destination_route_token: binding.destination_route_token,
                    delivery_id: DeliveryId::from_bytes(rand::random()),
                    ciphertext_len: 0,
                    padding: CiphertextPadding::Exact,
                    expires_at_unix_ms: expiry,
                    hop_limit: 8,
                    transport_binding_id: binding.binding_id,
                },
                header,
                plaintext: &bytes,
            },
            &self.config.encryption_key,
            signing_key,
        )
        .map_err(Error::Mailbox)?;
        let authorized = authorize_outbound(OutboundAuthorization {
            prepared: &prepared,
            capability: &self.config.capability,
            context: &context,
            capability_authority_key: &self.config.authority_key,
            route: &route,
        })
        .map_err(Error::Outbound)?;
        let encoded_len = authorized.encoded_len();
        let queue_limit =
            usize::try_from(self.config.capability.grant.max_queue_contribution_bytes)
                .map_err(|_| Error::Limit)?
                .min(MAX_ENVELOPE_BYTES);
        if encoded_len > queue_limit {
            return Err(Error::Limit);
        }
        let counters = self.counters.checked_add(Counters {
            frame_bytes_sent: bytes.len() as u64,
            envelope_bytes_sent: encoded_len as u64,
            envelopes_sent: 1,
            ..Counters::default()
        })?;
        // The final policy lock spans admission. A concurrent replace cannot
        // approve an envelope under an old generation after this check.
        let live = self.config.policy.state.lock().map_err(|_| Error::State)?;
        if live.generation != generation {
            return Err(Error::Policy);
        }
        let limits = ChannelLimits::new(queue_limit, queue_limit).map_err(Error::Transport)?;
        let mut channel = transport
            .connect(&route, limits, cancel)
            .map_err(Error::Transport)?;
        channel
            .try_send(&authorized, cancel)
            .map_err(Error::Transport)?;
        self.sequence = next;
        self.counters = counters;
        self.last_sent = Some(metadata);
        Ok(encoded_len)
    }

    pub fn open(&mut self, wire: &[u8]) -> Result<VerifiedFrame> {
        self.open_inner(wire, false)
    }

    fn open_inner(&mut self, wire: &[u8], live_mode: bool) -> Result<VerifiedFrame> {
        #[cfg(not(feature = "live"))]
        let _ = live_mode;
        self.counters = self.counters.checked_add(Counters {
            envelope_bytes_received: wire.len() as u64,
            envelopes_received: 1,
            ..Counters::default()
        })?;
        let queue_limit =
            usize::try_from(self.config.capability.grant.max_queue_contribution_bytes)
                .map_err(|_| Error::Limit)?
                .min(MAX_ENVELOPE_BYTES);
        if wire.len() > queue_limit {
            return Err(Error::Limit);
        }
        let (context, generation) = self.config.policy.versioned()?;
        self.route(&context)?;
        let envelope = decode_envelope(wire).map_err(Error::Mailbox)?;
        let binding = &self.config.binding.claims;
        if envelope.encryption.key_id != self.config.key_id
            || envelope.routing.transport_binding_id != binding.binding_id
            || envelope.routing.authorization_domain_route_label
                != binding.authorization_domain_route_label
            || envelope.routing.destination_route_token != binding.destination_route_token
        {
            return Err(Error::Policy);
        }
        self.replay.capacity_hit = false;
        #[cfg(not(feature = "live"))]
        let open_canonical = open_envelope;
        #[cfg(feature = "live")]
        let open_canonical = open_verified_envelope;
        let opened = open_canonical(
            &envelope,
            &self.config.capability,
            &context,
            &self.config.encryption_key,
            &self.config.sender_verifying_key,
            &self.config.authority_key,
            &mut self.replay,
        );
        if self.replay.capacity_hit {
            return Err(Error::ReplayCapacity);
        }
        let opened = opened.map_err(Error::Mailbox)?;
        #[cfg(feature = "live")]
        let header = opened.header().clone();
        #[cfg(feature = "live")]
        let bytes = opened.into_payload();
        #[cfg(not(feature = "live"))]
        let bytes = opened;
        let frame = Frame::decode(&bytes)?;
        self.check_frame_kind(&frame)?;
        // Only now are the grant and expected identities authenticated. No
        // caller-supplied application Context can install this lease.
        let verified = self.verified_context(&context)?;
        let counters = self.counters.checked_add(Counters {
            frame_bytes_opened: bytes.len() as u64,
            ..Counters::default()
        })?;
        #[cfg(feature = "live")]
        let (live_deadline, live_expiry_unix_ms) = if live_mode {
            if header.sender_boot != self.config.boot {
                return Err(Error::Policy);
            }
            let expiry = header
                .expires_at_unix_ms
                .min(self.config.capability.grant.expires_at_unix_ms)
                .min(self.config.binding.claims.expires_at_unix_ms);
            let deadline = live::expiry_deadline(expiry).map_err(|_| Error::Policy)?;
            let guarded_expiry = expiry
                .checked_sub(live::MAX_CLOCK_OFFSET_MS)
                .ok_or(Error::Policy)?;
            self.live_state
                .admit(false, self.config.capability.grant.max_messages_per_minute)
                .map_err(|_| Error::Limit)?;
            self.authority.install_until(
                verified.clone(),
                generation,
                Some((guarded_expiry, deadline)),
            )?;
            (Some(deadline), Some(guarded_expiry))
        } else {
            self.authority.install(verified.clone(), generation)?;
            (None, None)
        };
        #[cfg(not(feature = "live"))]
        self.authority.install(verified.clone(), generation)?;
        self.counters = counters;
        Ok(VerifiedFrame {
            frame,
            context: verified,
            policy: self.config.policy.clone(),
            generation,
            #[cfg(feature = "live")]
            header,
            #[cfg(feature = "live")]
            live_deadline,
            #[cfg(feature = "live")]
            live_expiry_unix_ms,
            #[cfg(feature = "live")]
            sender_key: self.config.sender_verifying_key.to_bytes(),
            #[cfg(feature = "live")]
            authority_key: self.config.authority_key.to_bytes(),
            #[cfg(feature = "live")]
            binding_id: self.config.binding.claims.binding_id,
        })
    }

    // Call only after canonical inbound or outbound grant authorization.
    fn verified_context(&self, context: &OpenContext) -> Result<Context> {
        let grant = &self.config.capability.grant;
        let grant_bytes = serde_jcs::to_vec(grant).map_err(|_| Error::Serialization)?;
        Ok(Context {
            tenant: spiralcore_transfer::hex(context.expected_tenant.as_bytes()),
            domain: spiralcore_transfer::hex(context.expected_authorization_domain.as_bytes()),
            sender: spiralcore_transfer::hex(context.expected_sender.as_bytes()),
            recipient: spiralcore_transfer::hex(context.expected_recipient.as_bytes()),
            mailbox: spiralcore_transfer::hex(context.expected_mailbox.as_bytes()),
            capability: spiralcore_transfer::hex(grant.capability_id.as_bytes()),
            authorization_scope: spiralcore_transfer::hex(&spiralcore_transfer::hash(
                b"spiralcore.mvm.verified-capability.v1\0",
                &[&grant_bytes],
            )),
            policy_epoch: context.current_policy_epoch,
            execution_epoch: context.current_execution_epoch.into(),
        })
    }
}

/// Proof of canonical authentication and bounded frame decoding. Fields are private.
pub struct VerifiedFrame {
    frame: Frame,
    context: Context,
    policy: Policy,
    generation: u64,
    #[cfg(feature = "live")]
    header: MailboxEnvelopeHeader,
    #[cfg(feature = "live")]
    live_deadline: Option<std::time::Instant>,
    #[cfg(feature = "live")]
    live_expiry_unix_ms: Option<i64>,
    #[cfg(feature = "live")]
    sender_key: [u8; 32],
    #[cfg(feature = "live")]
    authority_key: [u8; 32],
    #[cfg(feature = "live")]
    binding_id: TransportBindingId,
}
impl VerifiedFrame {
    pub fn frame(&self) -> &Frame {
        &self.frame
    }
    pub fn context(&self) -> &Context {
        &self.context
    }
    pub fn dispatch(
        self,
        endpoint: &mut Endpoint<MailboxAuthority>,
        monotonic_seconds: u64,
    ) -> Result<Frame> {
        self.policy.check_generation(self.generation)?;
        #[cfg(feature = "live")]
        if self.live_deadline.is_some() {
            live::check_verified(&self).map_err(|_| Error::Policy)?;
        }
        endpoint
            .handle(&self.context, &self.frame, monotonic_seconds)
            .map_err(Error::Frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_capacity_refuses_new_identity_without_evicting_prior_entries() {
        // ReplayIdentity's fields are upstream-private; a header supplies the
        // real upstream identity without thousands of unnecessary crypto calls.
        let mut header = MailboxEnvelopeHeader {
            protocol_version: ProtocolVersion::V1,
            signature_algorithm: SignatureAlgorithm::ED25519,
            tenant: TenantId::from_bytes([1; 32]),
            authorization_domain: AuthorizationDomainId::from_bytes([2; 32]),
            sender: PrincipalId::from_bytes([3; 32]),
            recipient: PrincipalId::from_bytes([4; 32]),
            mailbox: MailboxId::from_bytes([5; 32]),
            sender_boot: BootId::from_bytes([6; 32]),
            execution_epoch: ExecutionEpoch::new(1).unwrap(),
            message_id: MessageId::from_bytes([0; 32]),
            correlation_id: CorrelationId::from_bytes([7; 32]),
            causation_id: CausationId::from_bytes([8; 32]),
            kind: MailboxMessageKind::Reply,
            reply_to: None,
            ordering_scope: OrderingScope::parse("test").unwrap(),
            sequence: SequenceNumber::new(1).unwrap(),
            issued_at_unix_ms: 1,
            expires_at_unix_ms: 2,
            nonce: ReplayNonce::from_bytes([9; 16]),
            content_type: content_type().unwrap(),
            capability_id: CapabilityId::from_bytes([10; 32]),
            policy_epoch: 1,
            encryption: EncryptionContext {
                algorithm: EncryptionAlgorithm::AES_256_GCM,
                key_id: [11; 32],
            },
            plaintext_len: 0,
        };
        let mut replay = BoundedReplay::default();
        let first = ReplayIdentity::from_header(&header);
        for index in 0..MAX_REPLAY_IDENTITIES {
            let mut id = [0; 32];
            id[..8].copy_from_slice(&(index as u64).to_le_bytes());
            header.message_id = MessageId::from_bytes(id);
            assert!(replay.check_and_record(ReplayIdentity::from_header(&header)));
        }
        header.message_id = MessageId::from_bytes([255; 32]);
        assert!(!replay.check_and_record(ReplayIdentity::from_header(&header)));
        assert!(replay.capacity_hit);
        assert_eq!(replay.seen.len(), MAX_REPLAY_IDENTITIES);
        replay.capacity_hit = false;
        assert!(!replay.check_and_record(first));
        assert!(!replay.capacity_hit, "a prior identity remains a duplicate");
        assert_eq!(replay.seen.len(), MAX_REPLAY_IDENTITIES);
    }
}
