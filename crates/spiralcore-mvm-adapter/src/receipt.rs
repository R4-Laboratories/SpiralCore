//! Proposed canonical encrypted destination-receipt service mapping.
//! Receipt payloads stay inside canonical MVM Cast envelopes, never host frames.
use crate::service::{
    HostBudget, LocalMailboxService, MailboxBinding, Reservation, ServiceBindingConfig,
    ServiceError, ServiceTime, Verified,
};
use ed25519_dalek::VerifyingKey;
use mvm_contract::protocol::mailbox::{
    AuthorizationDomainId, BootId, CausationId, CiphertextPadding, ContentType, DeliveryId,
    DeliveryOutcome, ED25519_SIGNATURE_BYTES, EncryptionAlgorithm, EncryptionContext,
    ExecutionEpoch, MailboxAddress, MailboxEnvelopeHeader, MailboxId, MailboxMessageKind,
    MailboxOperation, MailboxRoutingFrame, MessageId, OrderingScope, PrincipalId, ProtocolVersion,
    ReplayNonce, SequenceNumber, SignatureAlgorithm, SignedDestinationReceipt, TenantId,
};
use mvm_core::mailbox::{
    OpenContext, SealRequest,
    transport::{
        AuthorizedMailboxFrame, CancellationToken, OutboundAuthorization, PeerTransport,
        PreparedMailboxEnvelope, TransportBindingContext, TransportError, authorize_outbound,
        authorize_route, prepare_mailbox_envelope,
    },
    verify_receipt,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
};

pub const RECEIPT_CONTENT_TYPE: &str = "application/vnd.mvm.mailbox.receipt";
pub const MAX_RECEIPT_PAYLOAD_BYTES: usize = 4_096;
pub const MAX_RECEIPT_ENVELOPE_BYTES: usize = 16_384;
pub const MAX_RECEIPT_ATTEMPTS: usize = 8;
type Result<T> = std::result::Result<T, ServiceError>;

pub fn receipt_content_type() -> ContentType {
    ContentType::parse(RECEIPT_CONTENT_TYPE, 1).expect("fixed canonical receipt content type")
}

/// Encodes the existing canonical receipt type; signature authority is unchanged.
pub fn encode_destination_receipt(receipt: &SignedDestinationReceipt) -> Result<Vec<u8>> {
    if receipt.receipt.protocol_version != ProtocolVersion::V1
        || receipt.receipt.signature_algorithm != SignatureAlgorithm::ED25519
        || receipt.signature.len() != ED25519_SIGNATURE_BYTES
    {
        return Err(ServiceError::Malformed);
    }
    let bytes = serde_jcs::to_vec(receipt).map_err(|_| ServiceError::Malformed)?;
    if bytes.len() > MAX_RECEIPT_PAYLOAD_BYTES {
        return Err(ServiceError::Limit);
    }
    Ok(bytes)
}

/// Rejects noncanonical JSON as well as unknown fields and malformed values.
pub fn decode_destination_receipt(bytes: &[u8]) -> Result<SignedDestinationReceipt> {
    if bytes.len() > MAX_RECEIPT_PAYLOAD_BYTES {
        return Err(ServiceError::Limit);
    }
    let receipt: SignedDestinationReceipt =
        serde_json::from_slice(bytes).map_err(|_| ServiceError::Malformed)?;
    if encode_destination_receipt(&receipt)? != bytes {
        return Err(ServiceError::Malformed);
    }
    Ok(receipt)
}

/// A local opaque handle, never a replacement mailbox identity or wire field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendHandle {
    tracker: u64,
    id: u64,
    binding: u64,
    generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptObservation {
    Pending,
    Verified(Box<SignedDestinationReceipt>),
    Expired,
}

/// Control disposition only. No variant requests a response or generates an ACK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptControlDisposition {
    Observed,
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiptFacts {
    /// A verified AcceptedEphemeral or AcceptedDurable was observed.
    /// Duplicate alone does not establish the original acceptance level.
    pub accepted: bool,
    pub application_replied: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RequestKey {
    tenant: TenantId,
    domain: AuthorizationDomainId,
    sender: PrincipalId,
    recipient: PrincipalId,
    mailbox: MailboxId,
    boot: BootId,
    epoch: ExecutionEpoch,
    message: MessageId,
}
impl RequestKey {
    fn new(h: &MailboxEnvelopeHeader) -> Self {
        Self {
            tenant: h.tenant,
            domain: h.authorization_domain,
            sender: h.sender,
            recipient: h.recipient,
            mailbox: h.mailbox,
            boot: h.sender_boot,
            epoch: h.execution_epoch,
            message: h.message_id,
        }
    }
}
struct Attempt {
    delivery: DeliveryId,
    ciphertext: [u8; 32],
    observed: Option<[u8; 32]>,
}
struct Entry {
    handle: SendHandle,
    key: RequestKey,
    header: MailboxEnvelopeHeader,
    immutable: [u8; 32],
    control_binding: u64,
    control_generation: u64,
    control_address: MailboxAddress,
    receipt_key: VerifyingKey,
    deadline: u64,
    retention: u64,
    attempts: Vec<Attempt>,
    facts: ReceiptFacts,
    reply_commitment: Option<[u8; 32]>,
    best: Option<(usize, u8, [u8; 32])>,
    available: Option<SignedDestinationReceipt>,
    payload_charge: Option<Reservation>,
    _identity_charge: Reservation,
}

/// Bounded source-side destination observations. Application replies retain
/// their independent canonical state machine and cannot be forged by receipts.
pub struct ReceiptTracker {
    budget: HostBudget,
    tracker: u64,
    next: u64,
    max_pending: usize,
    entries: BTreeMap<u64, Entry>,
    requests: BTreeMap<RequestKey, u64>,
    last_monotonic: Option<u64>,
}
static NEXT_TRACKER: AtomicU64 = AtomicU64::new(1);
impl ReceiptTracker {
    pub fn new(budget: HostBudget, max_pending: usize) -> Result<Self> {
        if max_pending == 0 || max_pending > 4_096 || !budget.control_enabled() {
            return Err(ServiceError::Limit);
        }
        let tracker = NEXT_TRACKER
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| x.checked_add(1))
            .map_err(|_| ServiceError::Limit)?;
        Ok(Self {
            budget,
            tracker,
            next: 0,
            max_pending,
            entries: BTreeMap::new(),
            requests: BTreeMap::new(),
            last_monotonic: None,
        })
    }
    fn clock(&mut self, now: ServiceTime) -> Result<()> {
        if self
            .last_monotonic
            .is_some_and(|last| now.monotonic_ms < last)
        {
            return Err(ServiceError::Clock);
        }
        self.last_monotonic = Some(now.monotonic_ms);
        Ok(())
    }
    pub fn expire(&mut self, now: ServiceTime) -> Result<usize> {
        self.clock(now)?;
        let ids: Vec<_> = self
            .entries
            .iter()
            .filter_map(|(id, e)| (now.monotonic_ms >= e.retention).then_some(*id))
            .collect();
        for id in &ids {
            let entry = self.entries.remove(id).ok_or(ServiceError::State)?;
            self.requests.remove(&entry.key);
        }
        Ok(ids.len())
    }
    /// Both bindings and the destination key come from trusted local ownership.
    /// The request must pass canonical verification before it can allocate state.
    pub fn register_send(
        &mut self,
        request_binding: &MailboxBinding,
        control_binding: &MailboxBinding,
        expected_destination_receipt_key: &VerifyingKey,
        encoded: &[u8],
        now: ServiceTime,
    ) -> Result<SendHandle> {
        self.expire(now)?;
        if request_binding.binding_id() == control_binding.binding_id() {
            return Err(ServiceError::Unauthorized);
        }
        let (control_context, control_generation) =
            control_binding.with_checked(now, |c, g| Ok((c.clone(), g)))?;
        let grant = control_binding.capability_grant();
        if control_context.required_operation != MailboxOperation::Send
            || grant.message_kinds != [MailboxMessageKind::Cast]
            || control_context.accepted_content_types != [receipt_content_type()]
            || grant.max_payload_bytes > MAX_RECEIPT_PAYLOAD_BYTES as u32
        {
            return Err(ServiceError::Unauthorized);
        }
        request_binding.with_verified(encoded, now, |opened, envelope, context, generation| {
            let header = opened.header();
            if header.content_type == receipt_content_type()
                || control_context.expected_tenant != header.tenant
                || control_context.expected_authorization_domain != header.authorization_domain
                || control_context.expected_sender != header.recipient
                || control_context.expected_recipient != header.sender
            {
                return Err(ServiceError::Unauthorized);
            }
            let key = RequestKey::new(header);
            let immutable = request_commitment(header, opened.payload())?;
            if let Some(id) = self.requests.get(&key) {
                let entry = self.entries.get_mut(id).ok_or(ServiceError::State)?;
                if entry.handle.binding != request_binding.binding_id()
                    || entry.handle.generation != generation
                    || entry.control_binding != control_binding.binding_id()
                    || entry.control_generation != control_generation
                {
                    return Err(ServiceError::Unauthorized);
                }
                if entry.immutable != immutable
                    || entry.receipt_key != *expected_destination_receipt_key
                {
                    return Err(ServiceError::Conflict);
                }
                if now.monotonic_ms >= entry.deadline {
                    return Err(ServiceError::Expired);
                }
                if let Some(old) = entry
                    .attempts
                    .iter()
                    .find(|a| a.delivery == envelope.routing.delivery_id)
                {
                    return if old.ciphertext == envelope.ciphertext_commitment {
                        Ok(entry.handle)
                    } else {
                        Err(ServiceError::Conflict)
                    };
                }
                if entry.attempts.len() >= MAX_RECEIPT_ATTEMPTS {
                    return Err(ServiceError::Limit);
                }
                entry.attempts.push(Attempt {
                    delivery: envelope.routing.delivery_id,
                    ciphertext: envelope.ciphertext_commitment,
                    observed: None,
                });
                return Ok(entry.handle);
            }
            if self.entries.len() >= self.max_pending {
                return Err(ServiceError::Full);
            }
            let remaining = u64::try_from(
                header
                    .expires_at_unix_ms
                    .checked_sub(now.unix_ms)
                    .ok_or(ServiceError::Clock)?,
            )
            .map_err(|_| ServiceError::Expired)?;
            if remaining == 0 {
                return Err(ServiceError::Expired);
            }
            let deadline = now
                .monotonic_ms
                .checked_add(remaining)
                .ok_or(ServiceError::Clock)?;
            let retention = deadline.checked_add(300_000).ok_or(ServiceError::Clock)?;
            let id = self.next.checked_add(1).ok_or(ServiceError::Limit)?;
            let handle = SendHandle {
                tracker: self.tracker,
                id,
                binding: request_binding.binding_id(),
                generation,
            };
            let identity = self.budget.reserve_control(context, 0, 0, 1)?;
            let mut attempts = Vec::with_capacity(MAX_RECEIPT_ATTEMPTS);
            attempts.push(Attempt {
                delivery: envelope.routing.delivery_id,
                ciphertext: envelope.ciphertext_commitment,
                observed: None,
            });
            self.entries.insert(
                id,
                Entry {
                    handle,
                    key,
                    header: header.clone(),
                    immutable,
                    control_binding: control_binding.binding_id(),
                    control_generation,
                    control_address: address(&control_context),
                    receipt_key: *expected_destination_receipt_key,
                    deadline,
                    retention,
                    attempts,
                    facts: ReceiptFacts {
                        accepted: false,
                        application_replied: false,
                    },
                    reply_commitment: None,
                    best: None,
                    available: None,
                    payload_charge: None,
                    _identity_charge: identity,
                },
            );
            self.requests.insert(key, id);
            self.next = id;
            Ok(handle)
        })
    }
    /// Authenticates the Cast and its existing canonical receipt signature.
    /// It returns a local observation, never an envelope requiring another ACK.
    pub fn observe_receipt(
        &mut self,
        control_binding: &MailboxBinding,
        encoded: &[u8],
        now: ServiceTime,
    ) -> Result<ReceiptControlDisposition> {
        self.expire(now)?;
        if encoded.len() > MAX_RECEIPT_ENVELOPE_BYTES {
            return Err(ServiceError::Limit);
        }
        control_binding.with_verified(encoded, now, |opened, _, context, generation| {
            let header = opened.header();
            if header.kind != MailboxMessageKind::Cast
                || header.reply_to.is_some()
                || header.content_type != receipt_content_type()
            {
                return Err(ServiceError::Unauthorized);
            }
            let signed = decode_destination_receipt(opened.payload())?;
            let receipt = &signed.receipt;
            let mut matching = self.entries.iter().filter_map(|(id, e)| {
                (e.control_binding == control_binding.binding_id()
                    && e.control_generation == generation
                    && e.control_address == address(context)
                    && e.header.message_id == receipt.message_id
                    && address_from_header(&e.header) == receipt.address
                    && e.header.sender == header.recipient
                    && e.header.recipient == header.sender
                    && e.header.correlation_id == header.correlation_id
                    && header.causation_id.as_bytes() == e.header.message_id.as_bytes()
                    && receipt.execution_epoch == e.header.execution_epoch)
                    .then_some(*id)
            });
            let id = matching.next().ok_or(ServiceError::Unauthorized)?;
            if matching.next().is_some() {
                return Err(ServiceError::Conflict);
            }
            let entry = self.entries.get_mut(&id).ok_or(ServiceError::State)?;
            verify_receipt(&signed, &entry.receipt_key).map_err(|_| ServiceError::Unauthorized)?;
            let attempt = entry
                .attempts
                .iter()
                .position(|a| {
                    a.delivery == receipt.delivery_id
                        && a.ciphertext == receipt.ciphertext_commitment
                })
                .ok_or(ServiceError::Unauthorized)?;
            if receipt.observed_at_unix_ms < entry.header.issued_at_unix_ms
                || receipt.observed_at_unix_ms
                    > now.unix_ms.checked_add(5_000).ok_or(ServiceError::Clock)?
                || (matches!(
                    receipt.outcome,
                    DeliveryOutcome::AcceptedEphemeral | DeliveryOutcome::AcceptedDurable
                ) && receipt.observed_at_unix_ms >= entry.header.expires_at_unix_ms)
            {
                return Err(ServiceError::Unauthorized);
            }
            let fingerprint: [u8; 32] = Sha256::digest(opened.payload()).into();
            // Authenticated duplicates still consume the signed control rate;
            // their temporary byte/message lease is released on this return.
            let mut charge = Some(self.budget.reserve_control_verified(
                context,
                encoded.len() as u64,
                control_binding.capability_grant(),
                now,
            )?);
            if entry.attempts[attempt].observed == Some(fingerprint)
                || entry.best.is_some_and(|(_, _, best)| best == fingerprint)
            {
                return Ok(ReceiptControlDisposition::Duplicate);
            }
            let rank = outcome_rank(receipt.outcome);
            let replace = entry.best.is_none_or(|(old_attempt, old_rank, _)| {
                rank > old_rank || (rank == old_rank && attempt >= old_attempt)
            });
            // Reserve before changing observations or monotone state. A full
            // consumer queue therefore cannot fabricate a successful receipt.
            entry.attempts[attempt].observed = Some(fingerprint);
            if matches!(
                receipt.outcome,
                DeliveryOutcome::AcceptedEphemeral | DeliveryOutcome::AcceptedDurable
            ) {
                entry.facts.accepted = true;
            }
            if replace {
                entry.best = Some((attempt, rank, fingerprint));
                entry.available = Some(signed);
                entry.payload_charge = charge.take();
            }
            Ok(ReceiptControlDisposition::Observed)
        })
    }
    pub fn take_receipt(
        &mut self,
        binding: &MailboxBinding,
        handle: &SendHandle,
        now: ServiceTime,
    ) -> Result<ReceiptObservation> {
        self.expire(now)?;
        binding.with_checked(now, |_, generation| {
            let entry = self.entry(binding, handle, generation)?;
            if let Some(receipt) = entry.available.take() {
                entry.payload_charge.take();
                Ok(ReceiptObservation::Verified(Box::new(receipt)))
            } else if now.monotonic_ms >= entry.deadline {
                Ok(ReceiptObservation::Expired)
            } else {
                Ok(ReceiptObservation::Pending)
            }
        })
    }
    /// Observes an independently authenticated application Reply. This fact
    /// neither replaces nor fabricates a destination acceptance receipt.
    /// Execution epochs belong to each destination mailbox: the canonical
    /// reverse binding verifies the Reply epoch independently of the Call epoch.
    pub fn observe_application_reply(
        &mut self,
        binding: &MailboxBinding,
        encoded: &[u8],
        now: ServiceTime,
    ) -> Result<SendHandle> {
        self.expire(now)?;
        binding.with_verified(encoded, now, |opened, _, context, _| {
            let header = opened.header();
            if header.kind != MailboxMessageKind::Reply
                || header.content_type == receipt_content_type()
            {
                return Err(ServiceError::Unauthorized);
            }
            let mut matching = self.entries.iter().filter_map(|(id, e)| {
                let route = e.header.reply_to?;
                (e.header.kind == MailboxMessageKind::Call
                    && header.tenant == e.header.tenant
                    && header.authorization_domain == e.header.authorization_domain
                    && header.sender == e.header.recipient
                    && header.recipient == e.header.sender
                    && header.correlation_id == e.header.correlation_id
                    && header.causation_id.as_bytes() == e.header.message_id.as_bytes()
                    && header.content_type == e.header.content_type
                    && header.capability_id == route.capability_id
                    && address(context) == route.address)
                    .then_some(*id)
            });
            let id = matching.next().ok_or(ServiceError::Unauthorized)?;
            if matching.next().is_some() {
                return Err(ServiceError::Conflict);
            }
            let entry = self.entries.get_mut(&id).ok_or(ServiceError::State)?;
            if now.monotonic_ms >= entry.deadline {
                return Err(ServiceError::Expired);
            }
            let commitment = request_commitment(header, opened.payload())?;
            if entry.reply_commitment.is_some_and(|old| old != commitment) {
                return Err(ServiceError::Conflict);
            }
            entry.reply_commitment = Some(commitment);
            entry.facts.application_replied = true;
            Ok(entry.handle)
        })
    }
    pub fn facts(
        &mut self,
        binding: &MailboxBinding,
        handle: &SendHandle,
        now: ServiceTime,
    ) -> Result<ReceiptFacts> {
        self.expire(now)?;
        binding.with_checked(now, |_, generation| {
            Ok(self.entry(binding, handle, generation)?.facts)
        })
    }
    fn entry(
        &mut self,
        binding: &MailboxBinding,
        handle: &SendHandle,
        generation: u64,
    ) -> Result<&mut Entry> {
        if handle.tracker != self.tracker
            || handle.binding != binding.binding_id()
            || handle.generation != generation
        {
            return Err(ServiceError::Unauthorized);
        }
        self.entries
            .get_mut(&handle.id)
            .filter(|e| e.handle == *handle)
            .ok_or(ServiceError::UnknownHandle)
    }
}
fn address(c: &OpenContext) -> MailboxAddress {
    MailboxAddress {
        tenant: c.expected_tenant,
        authorization_domain: c.expected_authorization_domain,
        mailbox: c.expected_mailbox,
        recipient: c.expected_recipient,
    }
}
fn address_from_header(h: &MailboxEnvelopeHeader) -> MailboxAddress {
    MailboxAddress {
        tenant: h.tenant,
        authorization_domain: h.authorization_domain,
        mailbox: h.mailbox,
        recipient: h.recipient,
    }
}
fn request_commitment(header: &MailboxEnvelopeHeader, payload: &[u8]) -> Result<[u8; 32]> {
    let mut hash = Sha256::new();
    hash.update(b"mvm.mailbox.local.immutable-request.v1\0");
    hash.update(serde_jcs::to_vec(header).map_err(|_| ServiceError::Malformed)?);
    hash.update((payload.len() as u64).to_le_bytes());
    hash.update(payload);
    Ok(hash.finalize().into())
}
fn outcome_rank(outcome: DeliveryOutcome) -> u8 {
    match outcome {
        DeliveryOutcome::AcceptedDurable => 3,
        DeliveryOutcome::AcceptedEphemeral => 2,
        DeliveryOutcome::Duplicate => 1,
        _ => 0,
    }
}

/// Trusted reverse-route configuration. It cannot derive authority from a
/// request's payload or manufacture its own destination grant.
pub struct ReceiptReturnConfig {
    binding: MailboxBinding,
    budget: HostBudget,
    receipt_key: VerifyingKey,
    last_monotonic: AtomicU64,
    issuance: crate::issuance::IssuancePolicy,
}
/// A bounded control slot owned before application admission can occur.
pub struct ReceiptPermit<'a> {
    returns: &'a ReceiptReturnConfig,
    source: &'a MailboxBinding,
    source_generation: u64,
    return_generation: u64,
    header: MailboxEnvelopeHeader,
    delivery: DeliveryId,
    ciphertext: [u8; 32],
    _charge: Reservation,
}
/// Canonical ciphertext with an exact header/envelope proof for transport
/// authorization. Repeated sends retain the same envelope/message identity.
pub struct PreparedReceipt<'a> {
    returns: &'a ReceiptReturnConfig,
    source: &'a MailboxBinding,
    source_generation: u64,
    return_generation: u64,
    prepared: PreparedMailboxEnvelope,
    authorized: AuthorizedMailboxFrame,
    deadline: u64,
    expiry: i64,
    _charge: Reservation,
}
#[derive(Debug)]
pub enum ReceiptSendError {
    Service(ServiceError),
    Transport(TransportError),
}
impl std::fmt::Display for ReceiptSendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ReceiptSendError {}

impl ReceiptReturnConfig {
    fn issuance_window(
        &self,
        config: &ServiceBindingConfig,
        now: ServiceTime,
    ) -> Result<crate::issuance::IssuedWindow> {
        self.issuance.window(
            now,
            now.unix_ms.checked_add(30_000).ok_or(ServiceError::Clock)?,
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
        )
    }
    /// The v1 reverse grant must pin exactly 4096 payload bytes and 16384
    /// queue-contribution bytes; narrower or wider grants are unsupported.
    pub fn new(
        binding: MailboxBinding,
        budget: HostBudget,
        expected_local_receipt_key: VerifyingKey,
    ) -> Result<Self> {
        Self::new_with_policy(
            binding,
            budget,
            expected_local_receipt_key,
            crate::issuance::IssuancePolicy::Exact,
        )
    }
    pub fn new_with_policy(
        binding: MailboxBinding,
        budget: HostBudget,
        expected_local_receipt_key: VerifyingKey,
        issuance: crate::issuance::IssuancePolicy,
    ) -> Result<Self> {
        issuance.validate()?;
        let grant = binding.capability_grant();
        if !budget.control_enabled()
            || grant.message_kinds != [MailboxMessageKind::Cast]
            || grant.content_types != [receipt_content_type()]
            || grant.max_payload_bytes != MAX_RECEIPT_PAYLOAD_BYTES as u32
            || grant.max_queue_contribution_bytes != MAX_RECEIPT_ENVELOPE_BYTES as u64
        {
            return Err(ServiceError::Unauthorized);
        }
        Ok(Self {
            binding,
            budget,
            receipt_key: expected_local_receipt_key,
            last_monotonic: AtomicU64::new(0),
            issuance,
        })
    }
    fn clock(&self, now: ServiceTime) -> Result<()> {
        self.last_monotonic
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |last| {
                (now.monotonic_ms >= last).then_some(now.monotonic_ms)
            })
            .map_err(|_| ServiceError::Clock)?;
        Ok(())
    }
    /// Verifies the original request and reserves independent receipt capacity
    /// before the caller admits any application work. No received route is used.
    /// Manual callers must use the same HostBudget owner for application
    /// admission; prefer `admit`, which enforces that ownership before mutation.
    pub fn reserve<'a>(
        &'a self,
        source: &'a MailboxBinding,
        encoded: &[u8],
        now: ServiceTime,
    ) -> Result<ReceiptPermit<'a>> {
        self.clock(now)?;
        if source.binding_id() == self.binding.binding_id() {
            return Err(ServiceError::Unauthorized);
        }
        source.with_verified(encoded, now, |opened, envelope, _, source_generation| {
            self.reserve_header(
                source,
                opened.header(),
                envelope.routing.delivery_id,
                envelope.ciphertext_commitment,
                source_generation,
                now,
            )
        })
    }
    /// Native integrated path: canonical authentication/decryption runs once.
    /// A failure after destination admission remains an uncertain observation,
    /// not a rollback of the accepted message.
    pub fn admit<'a>(
        &'a self,
        service: &mut LocalMailboxService,
        source: &'a MailboxBinding,
        encoded: &[u8],
        now: ServiceTime,
    ) -> Result<(SignedDestinationReceipt, PreparedReceipt<'a>)> {
        if !service.uses_budget(&self.budget) {
            return Err(ServiceError::Unauthorized);
        }
        self.clock(now)?;
        let (receipt, permit) = service.admit_for_control(source, encoded, now, |proof| {
            self.reserve_verified(source, proof, now)
        })?;
        let prepared = permit.seal(&receipt, now)?;
        Ok((receipt, prepared))
    }
    fn reserve_verified<'a>(
        &'a self,
        source: &'a MailboxBinding,
        proof: &Verified,
        now: ServiceTime,
    ) -> Result<ReceiptPermit<'a>> {
        if proof.binding_id() != source.binding_id()
            || proof.header().plaintext_len as usize != proof.payload().len()
            || address(proof.context()) != address_from_header(proof.header())
        {
            return Err(ServiceError::Unauthorized);
        }
        self.reserve_header(
            source,
            proof.header(),
            proof.envelope().routing.delivery_id,
            proof.envelope().ciphertext_commitment,
            proof.generation(),
            now,
        )
    }
    fn reserve_header<'a>(
        &'a self,
        source: &'a MailboxBinding,
        header: &MailboxEnvelopeHeader,
        delivery: DeliveryId,
        ciphertext: [u8; 32],
        source_generation: u64,
        now: ServiceTime,
    ) -> Result<ReceiptPermit<'a>> {
        if source.binding_id() == self.binding.binding_id()
            || header.content_type.name() == RECEIPT_CONTENT_TYPE
        {
            return Err(ServiceError::Unauthorized);
        }
        self.binding
            .with_sender(now, |config, context, return_generation| {
                if context.expected_tenant != header.tenant
                    || context.expected_authorization_domain != header.authorization_domain
                    || context.expected_sender != header.recipient
                    || context.expected_recipient != header.sender
                    || context.required_operation != MailboxOperation::Send
                    || context.accepted_content_types != [receipt_content_type()]
                {
                    return Err(ServiceError::Unauthorized);
                }
                self.issuance_window(config, now)?;
                let charge = self.budget.reserve_control(
                    context,
                    MAX_RECEIPT_ENVELOPE_BYTES as u64,
                    1,
                    0,
                )?;
                Ok(ReceiptPermit {
                    returns: self,
                    source,
                    source_generation,
                    return_generation,
                    header: header.clone(),
                    delivery,
                    ciphertext,
                    _charge: charge,
                })
            })
    }
}
impl<'a> ReceiptPermit<'a> {
    pub fn seal(
        self,
        receipt: &SignedDestinationReceipt,
        now: ServiceTime,
    ) -> Result<PreparedReceipt<'a>> {
        self.returns.clock(now)?;
        verify_receipt(receipt, &self.returns.receipt_key)
            .map_err(|_| ServiceError::Unauthorized)?;
        if receipt.receipt.address != address_from_header(&self.header)
            || receipt.receipt.message_id != self.header.message_id
            || receipt.receipt.delivery_id != self.delivery
            || receipt.receipt.ciphertext_commitment != self.ciphertext
            || receipt.receipt.execution_epoch != self.header.execution_epoch
            || receipt.receipt.observed_at_unix_ms < self.header.issued_at_unix_ms
            || receipt.receipt.observed_at_unix_ms
                > now.unix_ms.checked_add(5_000).ok_or(ServiceError::Clock)?
        {
            return Err(ServiceError::Unauthorized);
        }
        let payload = encode_destination_receipt(receipt)?;
        let (prepared, authorized, deadline, expiry) =
            self.source.with_checked(now, |_, generation| {
                if generation != self.source_generation {
                    return Err(ServiceError::Unauthorized);
                }
                self.returns
                    .binding
                    .with_sender(now, |config, context, generation| {
                        if generation != self.return_generation {
                            return Err(ServiceError::Unauthorized);
                        }
                        let window = self.returns.issuance_window(config, now)?;
                        let expiry = window.expires;
                        let remaining = u64::try_from(
                            expiry.checked_sub(now.unix_ms).ok_or(ServiceError::Clock)?,
                        )
                        .map_err(|_| ServiceError::Expired)?;
                        if remaining == 0 {
                            return Err(ServiceError::Expired);
                        }
                        let deadline = now
                            .monotonic_ms
                            .checked_add(remaining)
                            .ok_or(ServiceError::Clock)?;
                        let message = MessageId::from_bytes(rand::random());
                        let scope = message
                            .as_bytes()
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>();
                        let header = MailboxEnvelopeHeader {
                            protocol_version: ProtocolVersion::V1,
                            signature_algorithm: SignatureAlgorithm::ED25519,
                            tenant: context.expected_tenant,
                            authorization_domain: context.expected_authorization_domain,
                            sender: context.expected_sender,
                            recipient: context.expected_recipient,
                            mailbox: context.expected_mailbox,
                            sender_boot: config.expected_sender_boot,
                            execution_epoch: context.current_execution_epoch,
                            message_id: message,
                            correlation_id: self.header.correlation_id,
                            causation_id: CausationId::from_bytes(
                                *self.header.message_id.as_bytes(),
                            ),
                            kind: MailboxMessageKind::Cast,
                            reply_to: None,
                            ordering_scope: OrderingScope::parse(scope)
                                .map_err(|_| ServiceError::Malformed)?,
                            sequence: SequenceNumber::new(1)
                                .map_err(|_| ServiceError::Malformed)?,
                            issued_at_unix_ms: window.issued,
                            expires_at_unix_ms: expiry,
                            nonce: ReplayNonce::from_bytes(rand::random()),
                            content_type: receipt_content_type(),
                            capability_id: config.capability.grant.capability_id,
                            policy_epoch: context.current_policy_epoch,
                            encryption: EncryptionContext {
                                algorithm: EncryptionAlgorithm::AES_256_GCM,
                                key_id: config.encryption_key_id,
                            },
                            plaintext_len: payload.len() as u32,
                        };
                        let route = &config.binding.claims;
                        let prepared = prepare_mailbox_envelope(
                            SealRequest {
                                routing: MailboxRoutingFrame {
                                    protocol_version: ProtocolVersion::V1,
                                    authorization_domain_route_label: route
                                        .authorization_domain_route_label,
                                    destination_route_token: route.destination_route_token,
                                    delivery_id: DeliveryId::from_bytes(rand::random()),
                                    ciphertext_len: 0,
                                    padding: CiphertextPadding::Exact,
                                    expires_at_unix_ms: expiry,
                                    hop_limit: 8,
                                    transport_binding_id: route.binding_id,
                                },
                                header,
                                plaintext: &payload,
                            },
                            &config.encryption_key,
                            config
                                .signing_key
                                .as_ref()
                                .ok_or(ServiceError::Unauthorized)?,
                        )
                        .map_err(|_| ServiceError::Unauthorized)?;
                        let (_, authorized) = authorize(&prepared, config, context, now)?;
                        if authorized.encoded_len() > MAX_RECEIPT_ENVELOPE_BYTES {
                            return Err(ServiceError::Limit);
                        }
                        Ok((prepared, authorized, deadline, expiry))
                    })
            })?;
        Ok(PreparedReceipt {
            returns: self.returns,
            source: self.source,
            source_generation: self.source_generation,
            return_generation: self.return_generation,
            prepared,
            authorized,
            deadline,
            expiry,
            _charge: self._charge,
        })
    }
}
/// One charged send attempt. Reservation belongs to synchronous broker dispatch;
/// transport I/O consumes this non-cloneable permit after releasing dispatch.
/// Dropping it releases temporary queue credits, while the signed rate-history
/// identity remains charged for its original sixty-second window.
pub struct ReceiptSendPermit<'p, 'a> {
    receipt: &'p PreparedReceipt<'a>,
    reserved_at: u64,
    _rate_charge: Reservation,
}
impl<'a> PreparedReceipt<'a> {
    pub fn encoded_bytes(&self) -> &[u8] {
        self.authorized.encoded_bytes()
    }
    /// Reserve this attempt's signed control rate and queue capacity inside
    /// synchronous broker dispatch. A retry needs a fresh permit and charge.
    pub fn reserve_send(
        &self,
        cancel: &CancellationToken,
        now: ServiceTime,
    ) -> std::result::Result<ReceiptSendPermit<'_, 'a>, ReceiptSendError> {
        self.returns.clock(now).map_err(ReceiptSendError::Service)?;
        let rate_charge = self
            .source
            .with_checked(now, |_, generation| {
                self.check_send(generation, cancel, now)?;
                self.returns
                    .binding
                    .with_checked(now, |context, generation| {
                        if generation != self.return_generation {
                            return Err(ServiceError::Unauthorized);
                        }
                        self.returns.budget.reserve_control_send(
                            context,
                            self.authorized.encoded_len() as u64,
                            self.returns.binding.capability_grant(),
                            now,
                        )
                    })
            })
            .map_err(ReceiptSendError::Service)?;
        Ok(ReceiptSendPermit {
            receipt: self,
            reserved_at: now.monotonic_ms,
            _rate_charge: rate_charge,
        })
    }
    fn check_send(
        &self,
        source_generation: u64,
        cancel: &CancellationToken,
        now: ServiceTime,
    ) -> Result<()> {
        if source_generation != self.source_generation {
            return Err(ServiceError::Unauthorized);
        }
        if now.monotonic_ms >= self.deadline || now.unix_ms >= self.expiry {
            return Err(ServiceError::Expired);
        }
        if cancel.is_cancelled() {
            return Err(ServiceError::Cancelled);
        }
        Ok(())
    }
    /// Convenience path for standalone callers. Broker callers reserve within
    /// dispatch, release dispatch, then consume the permit during transport I/O.
    pub fn send<T: PeerTransport>(
        &self,
        transport: &T,
        cancel: &CancellationToken,
        now: ServiceTime,
    ) -> std::result::Result<usize, ReceiptSendError> {
        self.reserve_send(cancel, now)?.send(transport, cancel, now)
    }
}
impl ReceiptSendPermit<'_, '_> {
    /// Revalidate current source/return policy and canonical route at send time,
    /// without updating broker-global clocks or reserving rate capacity again.
    /// This consumes exactly one attempt; it does not bypass transport controls.
    pub fn send<T: PeerTransport>(
        self,
        transport: &T,
        cancel: &CancellationToken,
        now: ServiceTime,
    ) -> std::result::Result<usize, ReceiptSendError> {
        if now.monotonic_ms < self.reserved_at {
            return Err(ReceiptSendError::Service(ServiceError::Clock));
        }
        let receipt = self.receipt;
        let result = receipt.source.with_checked(now, |_, generation| {
            receipt.check_send(generation, cancel, now)?;
            receipt.returns.binding.send_prepared_at_generation(
                &receipt.prepared,
                receipt.return_generation,
                transport,
                cancel,
                now,
            )
        });
        result.map_err(ReceiptSendError::Service)
    }
}

fn authorize(
    prepared: &PreparedMailboxEnvelope,
    config: &ServiceBindingConfig,
    context: &OpenContext,
    now: ServiceTime,
) -> Result<(
    mvm_core::mailbox::transport::AuthorizedRoute,
    AuthorizedMailboxFrame,
)> {
    let route = authorize_route(
        &config.binding,
        &TransportBindingContext {
            expected_issuer: context.expected_capability_issuer,
            expected_address: address(context),
            current_execution_epoch: context.current_execution_epoch,
            current_policy_epoch: context.current_policy_epoch,
            minimum_assurance: config.minimum_assurance,
            now_unix_ms: now.unix_ms,
        },
        &config.authority_key,
    )
    .map_err(|_| ServiceError::Unauthorized)?;
    let frame = authorize_outbound(OutboundAuthorization {
        prepared,
        capability: &config.capability,
        context,
        capability_authority_key: &config.authority_key,
        route: &route,
    })
    .map_err(|_| ServiceError::Unauthorized)?;
    if frame.encoded_len() as u64 > config.capability.grant.max_queue_contribution_bytes {
        return Err(ServiceError::Limit);
    }
    Ok((route, frame))
}
