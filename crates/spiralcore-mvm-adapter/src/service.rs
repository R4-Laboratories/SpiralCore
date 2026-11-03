//! Bounded ephemeral admission using canonical MVM envelopes and receipts.
//! Trusted local bindings own authority. This is not a durable mailbox journal.
mod ordering;
use crate::DirectionConfig;
use ed25519_dalek::{SigningKey, VerifyingKey};
use mvm_contract::protocol::mailbox::*;
use mvm_core::{
    crypto::aead::Key,
    mailbox::{
        self, OpenContext, OpenedMailboxEnvelope, ReplayIdentity, ReplayProtector, SealRequest,
        transport::{
            CancellationToken, ChannelLimits, OutboundAuthorization, PeerTransport,
            PreparedMailboxEnvelope, SignedTransportBinding, TransportAssurance,
            TransportBindingContext, TransportError, authorize_outbound, authorize_route,
            prepare_mailbox_envelope,
        },
    },
};
use ordering::{OrderingIndex, ScopeKey, scope_key};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
};

type Result<T> = std::result::Result<T, ServiceError>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceError {
    Unauthorized,
    Malformed,
    Limit,
    Full,
    Backpressured,
    Cancelled,
    Expired,
    Unavailable,
    ShuttingDown,
    Conflict,
    UnknownHandle,
    Clock,
    State,
}
impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ServiceError {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceTime {
    pub unix_ms: i64,
    pub monotonic_ms: u64,
}
#[derive(Debug, Clone, Copy)]
pub struct ScopeLimits {
    pub queued_bytes: u64,
    pub queued_messages: usize,
    pub active_calls: usize,
    pub identities: usize,
}
#[derive(Debug, Clone)]
pub struct ServiceLimits {
    pub host: ScopeLimits,
    pub tenant: ScopeLimits,
    pub mailbox: ScopeLimits,
    pub sender: ScopeLimits,
    pub retention_ms: u64,
    pub max_payload_bytes: usize,
    pub max_envelope_bytes: usize,
}
#[derive(Debug, Clone, Copy)]
pub struct ControlScope {
    pub queued_bytes: u64,
    pub queued_messages: usize,
    pub identities: usize,
}
#[derive(Debug, Clone, Copy)]
pub struct ControlReserve {
    pub host: ControlScope,
    pub tenant: ControlScope,
    pub mailbox: ControlScope,
    pub sender: ControlScope,
}
impl Default for ControlReserve {
    fn default() -> Self {
        Self {
            host: ControlScope {
                queued_bytes: 1 << 20,
                queued_messages: 256,
                identities: 1024,
            },
            tenant: ControlScope {
                queued_bytes: 512 << 10,
                queued_messages: 128,
                identities: 512,
            },
            mailbox: ControlScope {
                queued_bytes: 256 << 10,
                queued_messages: 64,
                identities: 256,
            },
            sender: ControlScope {
                queued_bytes: 128 << 10,
                queued_messages: 32,
                identities: 128,
            },
        }
    }
}
impl ControlScope {
    fn limit(self) -> ScopeLimits {
        ScopeLimits {
            queued_bytes: self.queued_bytes,
            queued_messages: self.queued_messages,
            active_calls: 0,
            identities: self.identities,
        }
    }
    fn data_limit(self, total: ScopeLimits) -> ScopeLimits {
        ScopeLimits {
            queued_bytes: total.queued_bytes - self.queued_bytes,
            queued_messages: total.queued_messages - self.queued_messages,
            active_calls: total.active_calls,
            identities: total.identities - self.identities,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum BudgetClass {
    Data,
    Control,
}
impl Default for ServiceLimits {
    fn default() -> Self {
        Self {
            host: ScopeLimits {
                queued_bytes: 16 << 20,
                queued_messages: 1024,
                active_calls: 64,
                identities: 4096,
            },
            tenant: ScopeLimits {
                queued_bytes: 8 << 20,
                queued_messages: 512,
                active_calls: 32,
                identities: 2048,
            },
            mailbox: ScopeLimits {
                queued_bytes: 4 << 20,
                queued_messages: 256,
                active_calls: 16,
                identities: 1024,
            },
            sender: ScopeLimits {
                queued_bytes: 2 << 20,
                queued_messages: 128,
                active_calls: 8,
                identities: 512,
            },
            retention_ms: 300_000,
            max_payload_bytes: 65_536,
            max_envelope_bytes: 262_144,
        }
    }
}
#[derive(Default, Clone, Copy)]
struct Usage {
    bytes: u64,
    messages: usize,
    calls: usize,
    identities: usize,
}
impl Usage {
    fn difference(self, x: Self) -> Result<Self> {
        Ok(Self {
            bytes: self.bytes.checked_sub(x.bytes).ok_or(ServiceError::State)?,
            messages: self
                .messages
                .checked_sub(x.messages)
                .ok_or(ServiceError::State)?,
            calls: self.calls.checked_sub(x.calls).ok_or(ServiceError::State)?,
            identities: self
                .identities
                .checked_sub(x.identities)
                .ok_or(ServiceError::State)?,
        })
    }
    fn plus(self, x: Self) -> Result<Self> {
        Ok(Self {
            bytes: self.bytes.checked_add(x.bytes).ok_or(ServiceError::Limit)?,
            messages: self
                .messages
                .checked_add(x.messages)
                .ok_or(ServiceError::Limit)?,
            calls: self.calls.checked_add(x.calls).ok_or(ServiceError::Limit)?,
            identities: self
                .identities
                .checked_add(x.identities)
                .ok_or(ServiceError::Limit)?,
        })
    }
    fn fits(self, l: ScopeLimits) -> bool {
        self.bytes <= l.queued_bytes
            && self.messages <= l.queued_messages
            && self.calls <= l.active_calls
            && self.identities <= l.identities
    }
    fn minus(&mut self, x: Self) {
        self.bytes -= x.bytes;
        self.messages -= x.messages;
        self.calls -= x.calls;
        self.identities -= x.identities;
    }
    fn empty(self) -> bool {
        self.bytes == 0 && self.messages == 0 && self.calls == 0 && self.identities == 0
    }
}
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct Scope {
    tenant: TenantId,
    address: MailboxAddress,
    sender: PrincipalId,
}
type SenderScope = (TenantId, AuthorizationDomainId, PrincipalId);
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct GrantScope {
    address: MailboxAddress,
    sender: PrincipalId,
    capability: CapabilityId,
}
impl GrantScope {
    fn new(scope: Scope, grant: &MailboxCapabilityGrant) -> Self {
        Self {
            address: scope.address,
            sender: scope.sender,
            capability: grant.capability_id,
        }
    }
}
impl Scope {
    fn sender_scope(self) -> SenderScope {
        (self.tenant, self.address.authorization_domain, self.sender)
    }
}
#[derive(Default)]
struct BudgetState {
    host: Usage,
    tenants: HashMap<TenantId, Usage>,
    mailboxes: HashMap<MailboxAddress, Usage>,
    senders: HashMap<SenderScope, Usage>,
    arrivals: HashMap<GrantScope, VecDeque<u64>>,
    grants: HashMap<GrantScope, Usage>,
    accepted: BTreeMap<MessageKey, AcceptedIndex>,
    last_monotonic_ms: Option<u64>,
    control: ClassUsage,
    control_rates: VecDeque<ControlRate>,
    owners: HashMap<MailboxAddress, Arc<Mutex<MailboxOwner>>>,
}
struct MailboxOwner {
    epoch: ExecutionEpoch,
    service: u64,
}
struct ControlRate {
    scope: Scope,
    expires: u64,
}
#[derive(Default)]
struct ClassUsage {
    host: Usage,
    tenants: HashMap<TenantId, Usage>,
    mailboxes: HashMap<MailboxAddress, Usage>,
    senders: HashMap<SenderScope, Usage>,
}
struct AcceptedIndex {
    commitment: [u8; 32],
    scope: Scope,
    retention: u64,
    _original_receipt: SignedDestinationReceipt,
}
enum AdmissionReservation {
    Reserved(Box<Reservation>),
    Duplicate,
    Conflict,
}
struct AdmissionInput<'a> {
    scope: Scope,
    delta: Usage,
    key: MessageKey,
    commitment: [u8; 32],
    retention: u64,
    receipt: SignedDestinationReceipt,
    grant: &'a MailboxCapabilityGrant,
    now: u64,
}
#[derive(Clone)]
pub struct HostBudget {
    inner: Arc<Mutex<BudgetState>>,
    limits: Arc<ServiceLimits>,
    control: Option<Arc<ControlReserve>>,
}
/// Aggregate diagnostic counters for the trusted broker owner. This does not
/// expose tenant identities, payloads or cache membership to remote senders.
#[derive(Debug, Clone)]
pub struct BudgetSnapshot {
    pub queued_bytes: u64,
    pub queued_messages: usize,
    pub active_calls: usize,
    pub identities: usize,
    pub owner_count: usize,
    pub limits: ServiceLimits,
    pub control: Option<ControlReserve>,
}
#[derive(Debug, Clone, Copy)]
pub struct ServiceSnapshot {
    pub accepted: usize,
    pub calls: usize,
    pub fifo_scopes: usize,
    pub fifo_entries: usize,
}
impl HostBudget {
    pub fn snapshot(&self) -> Result<BudgetSnapshot> {
        let state = self.inner.lock().map_err(|_| ServiceError::State)?;
        Ok(BudgetSnapshot {
            queued_bytes: state.host.bytes,
            queued_messages: state.host.messages,
            active_calls: state.host.calls,
            identities: state.host.identities,
            owner_count: state.owners.len(),
            limits: (*self.limits).clone(),
            control: self.control.as_deref().cloned(),
        })
    }
    pub(crate) fn same_owner(&self, other: &HostBudget) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
    fn current_epoch(&self, address: MailboxAddress) -> Result<Option<ExecutionEpoch>> {
        let owner = self
            .inner
            .lock()
            .map_err(|_| ServiceError::State)?
            .owners
            .get(&address)
            .cloned();
        owner
            .map(|o| o.lock().map(|o| o.epoch).map_err(|_| ServiceError::State))
            .transpose()
    }
    fn claim_owner(
        &self,
        scope: Scope,
        epoch: ExecutionEpoch,
        service: u64,
    ) -> Result<Arc<Mutex<MailboxOwner>>> {
        let owner = {
            let mut b = self.inner.lock().map_err(|_| ServiceError::State)?;
            if let Some(owner) = b.owners.get(&scope.address) {
                Arc::clone(owner)
            } else {
                self.reserve_locked(
                    &mut b,
                    scope,
                    Usage {
                        identities: 1,
                        ..Usage::default()
                    },
                    BudgetClass::Data,
                )?;
                let owner = Arc::new(Mutex::new(MailboxOwner { epoch, service }));
                b.owners.insert(scope.address, Arc::clone(&owner));
                owner
            }
        };
        {
            let mut state = owner.lock().map_err(|_| ServiceError::State)?;
            if epoch < state.epoch || (epoch == state.epoch && service != state.service) {
                return Err(ServiceError::Unavailable);
            }
            if epoch > state.epoch {
                state.epoch = epoch;
                state.service = service;
            }
        }
        Ok(owner)
    }
    fn duplicate(&self, key: MessageKey, commitment: [u8; 32]) -> Result<Option<DeliveryOutcome>> {
        let b = self.inner.lock().map_err(|_| ServiceError::State)?;
        Ok(b.accepted.get(&key).map(|old| {
            if old.commitment == commitment {
                DeliveryOutcome::Duplicate
            } else {
                DeliveryOutcome::Refused
            }
        }))
    }
    pub(crate) fn reserve_control_send(
        &self,
        context: &OpenContext,
        bytes: u64,
        grant: &MailboxCapabilityGrant,
        now: ServiceTime,
    ) -> Result<Reservation> {
        self.reserve_control_verified(context, bytes, grant, now)
    }
    pub(crate) fn reserve_control_verified(
        &self,
        context: &OpenContext,
        bytes: u64,
        grant: &MailboxCapabilityGrant,
        now: ServiceTime,
    ) -> Result<Reservation> {
        if !self.control_enabled() {
            return Err(ServiceError::Unavailable);
        }
        if bytes > crate::receipt::MAX_RECEIPT_ENVELOPE_BYTES as u64 {
            return Err(ServiceError::Limit);
        }
        if grant.message_kinds != [MailboxMessageKind::Cast]
            || grant.content_types != [crate::receipt::receipt_content_type()]
            || !grant.operations.allows(MailboxOperation::Send)
            || grant.max_payload_bytes as usize > crate::receipt::MAX_RECEIPT_PAYLOAD_BYTES
        {
            return Err(ServiceError::Unauthorized);
        }
        let scope = Scope {
            tenant: context.expected_tenant,
            address: address(context),
            sender: context.expected_sender,
        };
        if grant.address != scope.address || grant.subject != scope.sender {
            return Err(ServiceError::Unauthorized);
        }
        let expires = now
            .monotonic_ms
            .checked_add(60_000)
            .ok_or(ServiceError::Clock)?;
        let delta = Usage {
            bytes,
            messages: 1,
            identities: 1,
            calls: 0,
        };
        let mut b = self.inner.lock().map_err(|_| ServiceError::State)?;
        if b.last_monotonic_ms
            .is_some_and(|last| now.monotonic_ms < last)
        {
            return Err(ServiceError::Clock);
        }
        b.last_monotonic_ms = Some(now.monotonic_ms);
        expire_control_rates(&mut b, now.monotonic_ms);
        let (key, usage) = self.signed_limits(&b, scope, delta, grant)?;
        if b.arrivals
            .get(&key)
            .is_some_and(|arrivals| arrivals.len() >= grant.max_messages_per_minute as usize)
        {
            return Err(ServiceError::Backpressured);
        }
        self.reserve_locked(&mut b, scope, delta, BudgetClass::Control)?;
        if !usage.empty() {
            b.grants.insert(key, usage);
        }
        b.arrivals
            .entry(key)
            .or_default()
            .push_back(now.monotonic_ms);
        b.control_rates.push_back(ControlRate { scope, expires });
        Ok(Reservation {
            budget: self.clone(),
            scope,
            delta: Usage {
                identities: 0,
                ..delta
            },
            class: BudgetClass::Control,
            grant: Some(key),
        })
    }
    pub fn with_control(limits: ServiceLimits, reserve: ControlReserve) -> Result<Self> {
        let mut out = Self::new(limits)?;
        let valid = |r: ControlScope, l: ScopeLimits| {
            r.queued_bytes > 0
                && r.queued_messages > 0
                && r.identities > 0
                && r.queued_bytes < l.queued_bytes
                && r.queued_messages < l.queued_messages
                && r.identities < l.identities
        };
        let child = |a: ControlScope, b: ControlScope| {
            a.queued_bytes <= b.queued_bytes
                && a.queued_messages <= b.queued_messages
                && a.identities <= b.identities
        };
        if !valid(reserve.host, out.limits.host)
            || !valid(reserve.tenant, out.limits.tenant)
            || !valid(reserve.mailbox, out.limits.mailbox)
            || !valid(reserve.sender, out.limits.sender)
            || !child(reserve.sender, reserve.mailbox)
            || !child(reserve.mailbox, reserve.tenant)
            || !child(reserve.tenant, reserve.host)
        {
            return Err(ServiceError::Limit);
        }
        out.control = Some(Arc::new(reserve));
        Ok(out)
    }
    pub(crate) fn control_enabled(&self) -> bool {
        self.control.is_some()
    }
    pub(crate) fn reserve_control(
        &self,
        context: &OpenContext,
        bytes: u64,
        messages: usize,
        identities: usize,
    ) -> Result<Reservation> {
        if self.control.is_none() {
            return Err(ServiceError::Unavailable);
        }
        let scope = Scope {
            tenant: context.expected_tenant,
            address: address(context),
            sender: context.expected_sender,
        };
        let delta = Usage {
            bytes,
            messages,
            identities,
            calls: 0,
        };
        let mut b = self.inner.lock().map_err(|_| ServiceError::State)?;
        self.reserve_locked(&mut b, scope, delta, BudgetClass::Control)?;
        Ok(Reservation {
            budget: self.clone(),
            scope,
            delta,
            class: BudgetClass::Control,
            grant: None,
        })
    }
    pub fn new(limits: ServiceLimits) -> Result<Self> {
        let valid = |l: ScopeLimits| {
            l.queued_bytes > 0 && l.queued_messages > 0 && l.active_calls > 0 && l.identities > 0
        };
        let within = |a: ScopeLimits, b: ScopeLimits| {
            a.queued_bytes <= b.queued_bytes
                && a.queued_messages <= b.queued_messages
                && a.active_calls <= b.active_calls
                && a.identities <= b.identities
        };
        if ![limits.host, limits.tenant, limits.mailbox, limits.sender]
            .into_iter()
            .all(valid)
            || !within(limits.sender, limits.mailbox)
            || !within(limits.mailbox, limits.tenant)
            || !within(limits.tenant, limits.host)
            || limits.max_payload_bytes == 0
            || limits.max_payload_bytes > 65_536
            || limits.max_envelope_bytes == 0
            || limits.max_envelope_bytes > 262_144
            || limits.retention_ms == 0
            || limits.retention_ms > 300_000
        {
            return Err(ServiceError::Limit);
        }
        Ok(Self {
            inner: Arc::new(Mutex::new(BudgetState::default())),
            limits: Arc::new(limits),
            control: None,
        })
    }
    fn reserve(
        &self,
        scope: Scope,
        delta: Usage,
        grant: &MailboxCapabilityGrant,
    ) -> Result<Reservation> {
        let mut b = self.inner.lock().map_err(|_| ServiceError::State)?;
        let (grant_key, grant_usage) = self.signed_limits(&b, scope, delta, grant)?;
        self.reserve_locked(&mut b, scope, delta, BudgetClass::Data)?;
        if !grant_usage.empty() {
            b.grants.insert(grant_key, grant_usage);
        }
        Ok(Reservation {
            budget: self.clone(),
            scope,
            delta,
            class: BudgetClass::Data,
            grant: Some(grant_key),
        })
    }
    fn reserve_locked(
        &self,
        b: &mut BudgetState,
        scope: Scope,
        delta: Usage,
        class: BudgetClass,
    ) -> Result<()> {
        let h = b.host.plus(delta)?;
        let t = b
            .tenants
            .get(&scope.tenant)
            .copied()
            .unwrap_or_default()
            .plus(delta)?;
        let m = b
            .mailboxes
            .get(&scope.address)
            .copied()
            .unwrap_or_default()
            .plus(delta)?;
        let s = b
            .senders
            .get(&scope.sender_scope())
            .copied()
            .unwrap_or_default()
            .plus(delta)?;
        if !h.fits(self.limits.host)
            || !t.fits(self.limits.tenant)
            || !m.fits(self.limits.mailbox)
            || !s.fits(self.limits.sender)
        {
            return Err(ServiceError::Full);
        }
        if let Some(reserve) = self.control.as_deref() {
            let control = [
                b.control.host,
                b.control
                    .tenants
                    .get(&scope.tenant)
                    .copied()
                    .unwrap_or_default(),
                b.control
                    .mailboxes
                    .get(&scope.address)
                    .copied()
                    .unwrap_or_default(),
                b.control
                    .senders
                    .get(&scope.sender_scope())
                    .copied()
                    .unwrap_or_default(),
            ];
            let ceilings = [
                reserve.host,
                reserve.tenant,
                reserve.mailbox,
                reserve.sender,
            ];
            let totals = [
                self.limits.host,
                self.limits.tenant,
                self.limits.mailbox,
                self.limits.sender,
            ];
            let all = [h, t, m, s];
            for i in 0..4 {
                let fits = if class == BudgetClass::Control {
                    control[i].plus(delta)?.fits(ceilings[i].limit())
                } else {
                    all[i]
                        .difference(control[i])?
                        .fits(ceilings[i].data_limit(totals[i]))
                };
                if !fits {
                    return Err(ServiceError::Full);
                }
            }
            if class == BudgetClass::Control {
                b.control.host = control[0].plus(delta)?;
                b.control
                    .tenants
                    .insert(scope.tenant, control[1].plus(delta)?);
                b.control
                    .mailboxes
                    .insert(scope.address, control[2].plus(delta)?);
                b.control
                    .senders
                    .insert(scope.sender_scope(), control[3].plus(delta)?);
            }
        } else if class == BudgetClass::Control {
            return Err(ServiceError::Unavailable);
        }
        b.host = h;
        b.tenants.insert(scope.tenant, t);
        b.mailboxes.insert(scope.address, m);
        b.senders.insert(scope.sender_scope(), s);
        Ok(())
    }
    fn signed_limits(
        &self,
        b: &BudgetState,
        scope: Scope,
        delta: Usage,
        grant: &MailboxCapabilityGrant,
    ) -> Result<(GrantScope, Usage)> {
        let key = GrantScope::new(scope, grant);
        let mut grant_delta = delta;
        grant_delta.identities = 0;
        let next = b
            .grants
            .get(&key)
            .copied()
            .unwrap_or_default()
            .plus(grant_delta)?;
        if next.bytes > grant.max_queue_contribution_bytes
            || next.calls > grant.max_outstanding_calls as usize
        {
            return Err(ServiceError::Full);
        }
        Ok((key, next))
    }
    fn reserve_admission(&self, input: AdmissionInput<'_>) -> Result<AdmissionReservation> {
        let AdmissionInput {
            scope,
            mut delta,
            key,
            commitment,
            retention,
            receipt,
            grant,
            now,
        } = input;
        let retention = retention.max(now.checked_add(60_000).ok_or(ServiceError::Clock)?);
        let mut b = self.inner.lock().map_err(|_| ServiceError::State)?;
        if let Some(old) = b.accepted.get(&key) {
            return Ok(if old.commitment == commitment {
                AdmissionReservation::Duplicate
            } else {
                AdmissionReservation::Conflict
            });
        }
        let (grant_key, grant_usage) = self.signed_limits(&b, scope, delta, grant)?;
        if let Some(arrivals) = b.arrivals.get_mut(&grant_key) {
            while arrivals
                .front()
                .is_some_and(|t| now.saturating_sub(*t) >= 60_000)
            {
                arrivals.pop_front();
            }
        }
        if b.arrivals
            .get(&grant_key)
            .is_some_and(|arrivals| arrivals.len() >= grant.max_messages_per_minute as usize)
        {
            return Err(ServiceError::Backpressured);
        }
        self.reserve_locked(&mut b, scope, delta, BudgetClass::Data)?;
        if !grant_usage.empty() {
            b.grants.insert(grant_key, grant_usage);
        }
        b.accepted.insert(
            key,
            AcceptedIndex {
                commitment,
                scope,
                retention,
                _original_receipt: receipt,
            },
        );
        b.arrivals.entry(grant_key).or_default().push_back(now);
        // The shared owner retains the identity even if a service instance dies.
        delta.identities = 0;
        Ok(AdmissionReservation::Reserved(Box::new(Reservation {
            budget: self.clone(),
            scope,
            delta,
            class: BudgetClass::Data,
            grant: Some(grant_key),
        })))
    }
    fn expire(&self, now: ServiceTime) -> Result<usize> {
        let mut b = self.inner.lock().map_err(|_| ServiceError::State)?;
        expire_control_rates(&mut b, now.monotonic_ms);
        let expired: Vec<_> = b
            .accepted
            .iter()
            .filter_map(|(key, v)| (now.monotonic_ms >= v.retention).then_some(*key))
            .collect();
        for key in &expired {
            let record = b.accepted.remove(key).ok_or(ServiceError::State)?;
            release_usage(
                &mut b,
                record.scope,
                Usage {
                    identities: 1,
                    ..Usage::default()
                },
                BudgetClass::Data,
                None,
            );
        }
        b.arrivals.retain(|_, arrivals| {
            while arrivals
                .front()
                .is_some_and(|t| now.monotonic_ms.saturating_sub(*t) >= 60_000)
            {
                arrivals.pop_front();
            }
            !arrivals.is_empty()
        });
        Ok(expired.len())
    }
}
fn expire_control_rates(b: &mut BudgetState, now: u64) {
    while b.control_rates.front().is_some_and(|r| now >= r.expires) {
        if let Some(record) = b.control_rates.pop_front() {
            release_usage(
                b,
                record.scope,
                Usage {
                    identities: 1,
                    ..Usage::default()
                },
                BudgetClass::Control,
                None,
            );
        }
    }
    b.arrivals.retain(|_, arrivals| {
        while arrivals
            .front()
            .is_some_and(|time| now.saturating_sub(*time) >= 60_000)
        {
            arrivals.pop_front();
        }
        !arrivals.is_empty()
    });
}
fn release_usage(
    b: &mut BudgetState,
    scope: Scope,
    delta: Usage,
    class: BudgetClass,
    grant: Option<GrantScope>,
) {
    if let Some(grant) = grant
        && let Some(usage) = b.grants.get_mut(&grant)
    {
        let mut portion = delta;
        portion.identities = 0;
        usage.minus(portion);
        if usage.empty() {
            b.grants.remove(&grant);
        }
    }
    if class == BudgetClass::Control {
        b.control.host.minus(delta);
        if let Some(x) = b.control.tenants.get_mut(&scope.tenant) {
            x.minus(delta);
            if x.empty() {
                b.control.tenants.remove(&scope.tenant);
            }
        }
        if let Some(x) = b.control.mailboxes.get_mut(&scope.address) {
            x.minus(delta);
            if x.empty() {
                b.control.mailboxes.remove(&scope.address);
            }
        }
        let key = scope.sender_scope();
        if let Some(x) = b.control.senders.get_mut(&key) {
            x.minus(delta);
            if x.empty() {
                b.control.senders.remove(&key);
            }
        }
    }
    b.host.minus(delta);
    if let Some(x) = b.tenants.get_mut(&scope.tenant) {
        x.minus(delta);
        if x.empty() {
            b.tenants.remove(&scope.tenant);
        }
    }
    if let Some(x) = b.mailboxes.get_mut(&scope.address) {
        x.minus(delta);
        if x.empty() {
            b.mailboxes.remove(&scope.address);
        }
    }
    let key = scope.sender_scope();
    if let Some(x) = b.senders.get_mut(&key) {
        x.minus(delta);
        if x.empty() {
            b.senders.remove(&key);
        }
    }
}
pub(crate) struct Reservation {
    budget: HostBudget,
    scope: Scope,
    delta: Usage,
    class: BudgetClass,
    grant: Option<GrantScope>,
}
impl Reservation {
    fn release_calls(&mut self) -> Result<()> {
        if self.delta.calls > 0 {
            let mut b = self.budget.inner.lock().map_err(|_| ServiceError::State)?;
            release_usage(
                &mut b,
                self.scope,
                Usage {
                    calls: self.delta.calls,
                    ..Usage::default()
                },
                self.class,
                self.grant,
            );
            self.delta.calls = 0;
        }
        Ok(())
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        let Ok(mut b) = self.budget.inner.lock() else {
            return;
        };
        release_usage(&mut b, self.scope, self.delta, self.class, self.grant);
    }
}

pub struct ServiceBindingConfig {
    pub context: OpenContext,
    pub capability: SignedMailboxCapability,
    pub binding: SignedTransportBinding,
    pub sender_verifying_key: VerifyingKey,
    pub encryption_key: Key,
    pub authority_key: VerifyingKey,
    pub signing_key: Option<SigningKey>,
    pub kind: MailboxMessageKind,
    pub minimum_assurance: TransportAssurance,
    pub expected_sender_boot: BootId,
    pub encryption_key_id: [u8; 32],
}
impl ServiceBindingConfig {
    pub fn from_direction(c: DirectionConfig) -> Result<Self> {
        Ok(Self {
            context: c
                .policy
                .snapshot()
                .map_err(|_| ServiceError::Unauthorized)?,
            capability: c.capability,
            binding: c.binding,
            sender_verifying_key: c.sender_verifying_key,
            encryption_key: c.encryption_key,
            authority_key: c.authority_key,
            signing_key: c.signing_key,
            kind: c.kind,
            minimum_assurance: TransportAssurance::Development,
            expected_sender_boot: c.boot,
            encryption_key_id: c.key_id,
        })
    }
}
pub struct DestinationReceiptSigner {
    pub recipient: PrincipalId,
    pub signing_key: SigningKey,
}
struct BindingPolicy {
    context: OpenContext,
    generation: u64,
}
// All operations spanning two independently revocable bindings acquire them in
// immutable binding-id order. Retain these guards through the state transition.
struct PolicyPair<'a> {
    _first: MutexGuard<'a, BindingPolicy>,
    _second: Option<MutexGuard<'a, BindingPolicy>>,
    a_generation: u64,
    b_generation: u64,
}
fn policy_pair<'a>(
    a_id: u64,
    a: &'a Arc<Mutex<BindingPolicy>>,
    b_id: u64,
    b: &'a Arc<Mutex<BindingPolicy>>,
) -> Result<PolicyPair<'a>> {
    if Arc::ptr_eq(a, b) {
        let first = a.lock().map_err(|_| ServiceError::State)?;
        let generation = first.generation;
        return Ok(PolicyPair {
            _first: first,
            _second: None,
            a_generation: generation,
            b_generation: generation,
        });
    }
    let (first, second, forward) = if a_id < b_id {
        (a, b, true)
    } else {
        (b, a, false)
    };
    let first = first.lock().map_err(|_| ServiceError::State)?;
    let second = second.lock().map_err(|_| ServiceError::State)?;
    let (a_generation, b_generation) = if forward {
        (first.generation, second.generation)
    } else {
        (second.generation, first.generation)
    };
    Ok(PolicyPair {
        _first: first,
        _second: Some(second),
        a_generation,
        b_generation,
    })
}
pub struct MailboxBinding {
    config: ServiceBindingConfig,
    policy: Arc<Mutex<BindingPolicy>>,
    receipt_signer: Option<DestinationReceiptSigner>,
    id: u64,
}
static NEXT: AtomicU64 = AtomicU64::new(1);
fn fresh_id() -> Result<u64> {
    NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        value.checked_add(1)
    })
    .map_err(|_| ServiceError::Limit)
}
impl MailboxBinding {
    pub fn send_prepared(
        &self,
        prepared: &PreparedMailboxEnvelope,
        transport: &impl PeerTransport,
        cancel: &CancellationToken,
        now: ServiceTime,
    ) -> Result<usize> {
        self.send_prepared_inner(prepared, None, transport, cancel, now)
    }
    pub(crate) fn send_prepared_at_generation(
        &self,
        prepared: &PreparedMailboxEnvelope,
        generation: u64,
        transport: &impl PeerTransport,
        cancel: &CancellationToken,
        now: ServiceTime,
    ) -> Result<usize> {
        self.send_prepared_inner(prepared, Some(generation), transport, cancel, now)
    }
    fn send_prepared_inner(
        &self,
        prepared: &PreparedMailboxEnvelope,
        expected: Option<u64>,
        transport: &impl PeerTransport,
        cancel: &CancellationToken,
        now: ServiceTime,
    ) -> Result<usize> {
        if cancel.is_cancelled() {
            return Err(ServiceError::Cancelled);
        }
        let encoded =
            mailbox::encode_envelope(prepared.envelope()).map_err(|_| ServiceError::Limit)?;
        let verified = self.verify(&encoded, now)?;
        let policy = self.policy.lock().map_err(|_| ServiceError::State)?;
        if policy.generation != verified.generation
            || expected.is_some_and(|g| g != policy.generation)
        {
            return Err(ServiceError::Unauthorized);
        }
        let context = &verified.context;
        let route = authorize_route(
            &self.config.binding,
            &TransportBindingContext {
                expected_issuer: context.expected_capability_issuer,
                expected_address: address(context),
                current_execution_epoch: context.current_execution_epoch,
                current_policy_epoch: context.current_policy_epoch,
                minimum_assurance: self.config.minimum_assurance,
                now_unix_ms: now.unix_ms,
            },
            &self.config.authority_key,
        )
        .map_err(|_| ServiceError::Unauthorized)?;
        let authorized = authorize_outbound(OutboundAuthorization {
            prepared,
            capability: &self.config.capability,
            context,
            capability_authority_key: &self.config.authority_key,
            route: &route,
        })
        .map_err(|_| ServiceError::Unauthorized)?;
        let max = usize::try_from(self.config.capability.grant.max_queue_contribution_bytes)
            .map_err(|_| ServiceError::Limit)?
            .min(262_144);
        if authorized.encoded_len() > max {
            return Err(ServiceError::Limit);
        }
        let limits = ChannelLimits::new(max, max).map_err(map_transport)?;
        let mut channel = transport
            .connect(&route, limits, cancel)
            .map_err(map_transport)?;
        channel
            .try_send(&authorized, cancel)
            .map_err(map_transport)?;
        Ok(authorized.encoded_len())
    }
    fn deadlines(
        &self,
        header: &MailboxEnvelopeHeader,
        now: ServiceTime,
        retention_ms: u64,
    ) -> Result<(u64, u64)> {
        let (message_deadline, retention) = deadlines(header, now, retention_ms)?;
        let authorization_expiry = header
            .expires_at_unix_ms
            .min(self.config.capability.grant.expires_at_unix_ms)
            .min(self.config.binding.claims.expires_at_unix_ms);
        let remaining = u64::try_from(
            authorization_expiry
                .checked_sub(now.unix_ms)
                .ok_or(ServiceError::Clock)?,
        )
        .map_err(|_| ServiceError::Expired)?;
        let deadline = now
            .monotonic_ms
            .checked_add(remaining)
            .ok_or(ServiceError::Clock)?;
        Ok((message_deadline.min(deadline), retention))
    }
    pub(crate) fn with_sender<T>(
        &self,
        now: ServiceTime,
        f: impl FnOnce(&ServiceBindingConfig, &OpenContext, u64) -> Result<T>,
    ) -> Result<T> {
        let (context, generation) = self.checked(now)?;
        if self.config.signing_key.is_none() {
            return Err(ServiceError::Unauthorized);
        }
        let policy = self.policy.lock().map_err(|_| ServiceError::State)?;
        if policy.generation != generation {
            return Err(ServiceError::Unauthorized);
        }
        f(&self.config, &context, generation)
    }
    pub(crate) fn capability_grant(&self) -> &MailboxCapabilityGrant {
        &self.config.capability.grant
    }
    pub(crate) fn binding_id(&self) -> u64 {
        self.id
    }
    pub(crate) fn with_checked<T>(
        &self,
        now: ServiceTime,
        f: impl FnOnce(&OpenContext, u64) -> Result<T>,
    ) -> Result<T> {
        let (context, generation) = self.checked(now)?;
        let policy = self.policy.lock().map_err(|_| ServiceError::State)?;
        if policy.generation != generation {
            return Err(ServiceError::Unauthorized);
        }
        f(&context, generation)
    }
    pub(crate) fn with_verified<T>(
        &self,
        encoded: &[u8],
        now: ServiceTime,
        f: impl FnOnce(&OpenedMailboxEnvelope, &MailboxEnvelope, &OpenContext, u64) -> Result<T>,
    ) -> Result<T> {
        let verified = self.verify(encoded, now)?;
        let policy = self.policy.lock().map_err(|_| ServiceError::State)?;
        if policy.generation != verified.generation {
            return Err(ServiceError::Unauthorized);
        }
        f(
            &verified.opened,
            &verified.envelope,
            &verified.context,
            verified.generation,
        )
    }
    pub fn new(
        config: ServiceBindingConfig,
        receipt_signer: Option<DestinationReceiptSigner>,
    ) -> Result<Self> {
        if receipt_signer
            .as_ref()
            .is_some_and(|s| s.recipient != config.context.expected_recipient)
            || config
                .signing_key
                .as_ref()
                .is_some_and(|s| s.verifying_key() != config.sender_verifying_key)
        {
            return Err(ServiceError::Unauthorized);
        }
        let policy = BindingPolicy {
            context: config.context.clone(),
            generation: 0,
        };
        let out = Self {
            config,
            policy: Arc::new(Mutex::new(policy)),
            receipt_signer,
            id: fresh_id()?,
        };
        out.checked(ServiceTime {
            unix_ms: out.config.context.now_unix_ms,
            monotonic_ms: 0,
        })?;
        Ok(out)
    }
    pub fn replace_policy(&self, context: OpenContext) -> Result<()> {
        let mut p = self.policy.lock().map_err(|_| ServiceError::State)?;
        p.generation = p.generation.checked_add(1).ok_or(ServiceError::Limit)?;
        p.context = context;
        Ok(())
    }
    fn checked(&self, now: ServiceTime) -> Result<(OpenContext, u64)> {
        let p = self.policy.lock().map_err(|_| ServiceError::State)?;
        let mut c = p.context.clone();
        c.now_unix_ms = now.unix_ms;
        let g = &self.config.capability.grant;
        mailbox::verify_capability_signature(&self.config.capability, &self.config.authority_key)
            .map_err(|_| ServiceError::Unauthorized)?;
        let expected = if self.config.kind == MailboxMessageKind::Reply {
            MailboxOperation::Reply
        } else {
            MailboxOperation::Send
        };
        let address = address(&c);
        if c.required_operation != expected
            || c.acting_principal != c.expected_sender
            || g.subject != c.expected_sender
            || g.issuer != c.expected_capability_issuer
            || g.address != address
            || !g.operations.allows(expected)
            || g.message_kinds != [self.config.kind]
            || g.policy_epoch != c.current_policy_epoch
            || c.current_execution_epoch < g.minimum_execution_epoch
            || c.current_execution_epoch > g.maximum_execution_epoch
            || c.revoked_capabilities.contains(&g.revocation_id)
            || now.unix_ms < g.not_before_unix_ms
            || now.unix_ms >= g.expires_at_unix_ms
            || c.accepted_content_types != g.content_types
        {
            return Err(ServiceError::Unauthorized);
        }
        authorize_route(
            &self.config.binding,
            &TransportBindingContext {
                expected_issuer: c.expected_capability_issuer,
                expected_address: address,
                current_execution_epoch: c.current_execution_epoch,
                current_policy_epoch: c.current_policy_epoch,
                minimum_assurance: self.config.minimum_assurance,
                now_unix_ms: now.unix_ms,
            },
            &self.config.authority_key,
        )
        .map_err(|_| ServiceError::Unauthorized)?;
        if self
            .receipt_signer
            .as_ref()
            .is_some_and(|s| s.recipient != c.expected_recipient)
        {
            return Err(ServiceError::Unauthorized);
        }
        Ok((c, p.generation))
    }
    fn verify(&self, encoded: &[u8], now: ServiceTime) -> Result<Verified> {
        let (c, generation) = self.checked(now)?;
        if encoded.len() > 262_144 {
            return Err(ServiceError::Limit);
        }
        let envelope = mailbox::decode_envelope(encoded).map_err(|_| ServiceError::Malformed)?;
        let route = &self.config.binding.claims;
        if envelope.routing.transport_binding_id != route.binding_id
            || envelope.routing.authorization_domain_route_label
                != route.authorization_domain_route_label
            || envelope.routing.destination_route_token != route.destination_route_token
        {
            return Err(ServiceError::Unauthorized);
        }
        let mut replay = StagedReplay(None);
        let opened = mailbox::open_verified_envelope(
            &envelope,
            &self.config.capability,
            &c,
            &self.config.encryption_key,
            &self.config.sender_verifying_key,
            &self.config.authority_key,
            &mut replay,
        )
        .map_err(|_| ServiceError::Unauthorized)?;
        if opened.header().kind != self.config.kind
            || opened.header().sender_boot != self.config.expected_sender_boot
            || opened.header().encryption.key_id != self.config.encryption_key_id
        {
            return Err(ServiceError::Unauthorized);
        }
        let replay = replay.0.ok_or(ServiceError::State)?;
        let mut semantic =
            serde_jcs::to_vec(opened.header()).map_err(|_| ServiceError::Malformed)?;
        semantic.extend_from_slice(&(opened.payload().len() as u64).to_le_bytes());
        semantic.extend_from_slice(opened.payload());
        use sha2::{Digest, Sha256};
        let mut hash = Sha256::new();
        hash.update(b"mvm.mailbox.local-request.v1\0");
        hash.update(&semantic);
        let commitment = hash.finalize().into();
        Ok(Verified {
            opened,
            envelope,
            replay,
            commitment,
            generation,
            context: c,
            binding: self.id,
        })
    }
    fn receipt(
        &self,
        v: &Verified,
        outcome: DeliveryOutcome,
        now: ServiceTime,
    ) -> Result<SignedDestinationReceipt> {
        let signer = self
            .receipt_signer
            .as_ref()
            .ok_or(ServiceError::Unauthorized)?;
        mailbox::sign_receipt(
            DestinationReceipt {
                protocol_version: ProtocolVersion::V1,
                signature_algorithm: SignatureAlgorithm::ED25519,
                delivery_id: v.envelope.routing.delivery_id,
                message_id: v.opened.header().message_id,
                address: address(&v.context),
                execution_epoch: v.context.current_execution_epoch,
                outcome,
                observed_at_unix_ms: now.unix_ms,
                ciphertext_commitment: v.envelope.ciphertext_commitment,
            },
            &signer.signing_key,
        )
        .map_err(|_| ServiceError::Unauthorized)
    }
}
fn map_transport(error: TransportError) -> ServiceError {
    match error {
        TransportError::Cancelled => ServiceError::Cancelled,
        TransportError::Backpressured => ServiceError::Backpressured,
        TransportError::ShuttingDown => ServiceError::ShuttingDown,
        TransportError::InvalidLimits | TransportError::FrameTooLarge => ServiceError::Limit,
        TransportError::StateUnavailable => ServiceError::State,
        _ => ServiceError::Unavailable,
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
struct StagedReplay(Option<ReplayIdentity>);
impl ReplayProtector for StagedReplay {
    fn check_and_record(&mut self, id: ReplayIdentity) -> bool {
        if self.0.is_some() {
            return false;
        }
        self.0 = Some(id);
        true
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct MessageKey {
    tenant: TenantId,
    domain: AuthorizationDomainId,
    mailbox: MailboxId,
    recipient: PrincipalId,
    replay: ReplayIdentity,
}
impl MessageKey {
    fn new(a: MailboxAddress, replay: ReplayIdentity) -> Self {
        Self {
            tenant: a.tenant,
            domain: a.authorization_domain,
            mailbox: a.mailbox,
            recipient: a.recipient,
            replay,
        }
    }
}
pub(crate) struct Verified {
    opened: OpenedMailboxEnvelope,
    envelope: MailboxEnvelope,
    replay: ReplayIdentity,
    commitment: [u8; 32],
    generation: u64,
    context: OpenContext,
    binding: u64,
}
impl Verified {
    pub(crate) fn header(&self) -> &MailboxEnvelopeHeader {
        self.opened.header()
    }
    pub(crate) fn payload(&self) -> &[u8] {
        self.opened.payload()
    }
    pub(crate) fn envelope(&self) -> &MailboxEnvelope {
        &self.envelope
    }
    pub(crate) fn context(&self) -> &OpenContext {
        &self.context
    }
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
    pub(crate) fn binding_id(&self) -> u64 {
        self.binding
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeliveryHandle {
    service: u64,
    id: u64,
    binding: u64,
    generation: u64,
}
pub struct AdmittedMessage {
    opened: OpenedMailboxEnvelope,
    handle: DeliveryHandle,
    _queue: Reservation,
    deadline: u64,
    live: Arc<Mutex<bool>>,
    owner: Arc<Mutex<MailboxOwner>>,
}
impl AdmittedMessage {
    pub fn header(&self) -> &MailboxEnvelopeHeader {
        self.opened.header()
    }
    pub fn payload(&self) -> &[u8] {
        self.opened.payload()
    }
    pub fn handle(&self) -> &DeliveryHandle {
        &self.handle
    }
    /// Runs an adapter while the current policy generation is held unchanged.
    pub(crate) fn with_verified<T, E>(
        &self,
        binding: &MailboxBinding,
        now: ServiceTime,
        f: impl FnOnce(&OpenContext, &MailboxEnvelopeHeader, &[u8]) -> std::result::Result<T, E>,
    ) -> Result<std::result::Result<T, E>> {
        let live = self.live.lock().map_err(|_| ServiceError::State)?;
        if !*live {
            return Err(ServiceError::ShuttingDown);
        }
        let (context, generation) = binding.checked(now)?;
        let policy = binding.policy.lock().map_err(|_| ServiceError::State)?;
        if self.handle.binding != binding.id
            || self.handle.generation != generation
            || policy.generation != generation
        {
            return Err(ServiceError::Unauthorized);
        }
        if now.monotonic_ms >= self.deadline || now.unix_ms >= self.header().expires_at_unix_ms {
            return Err(ServiceError::Expired);
        }
        let owner = self.owner.lock().map_err(|_| ServiceError::State)?;
        if owner.epoch != self.header().execution_epoch || owner.service != self.handle.service {
            return Err(ServiceError::Unauthorized);
        }
        Ok(f(&context, self.header(), self.payload()))
    }
}
struct Accepted {
    commitment: [u8; 32],
    header: MailboxEnvelopeHeader,
    policy: Arc<Mutex<BindingPolicy>>,
    handle: DeliveryHandle,
    deadline: u64,
    retention: u64,
    message: Option<OpenedMailboxEnvelope>,
    queue: Option<Reservation>,
    active: Reservation,
    replied: bool,
    reply_cache: Option<CachedReply>,
    owner: Arc<Mutex<MailboxOwner>>,
    order_entry: Option<Reservation>,
    disposition: DeliveryOutcome,
}
struct CachedReply {
    prepared: PreparedMailboxEnvelope,
    header: MailboxEnvelopeHeader,
    binding: u64,
    generation: u64,
    policy: Arc<Mutex<BindingPolicy>>,
    _budget: Reservation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallStatus {
    Pending,
    Replied(Option<Vec<u8>>),
    TimedOut,
    Cancelled,
    Unavailable,
    ShuttingDown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallHandle {
    service: u64,
    id: u64,
    binding: u64,
    generation: u64,
}
struct PendingCall {
    header: MailboxEnvelopeHeader,
    commitment: [u8; 32],
    handle: CallHandle,
    deadline: u64,
    retention: u64,
    status: CallStatus,
    active: Reservation,
    reply_budget: Option<Reservation>,
    policy: Arc<Mutex<BindingPolicy>>,
}
pub struct LocalMailboxService {
    budget: HostBudget,
    id: u64,
    next: u64,
    accepted: BTreeMap<MessageKey, Accepted>,
    queue: VecDeque<MessageKey>,
    last: Option<ServiceTime>,
    shutting_down: bool,
    calls: BTreeMap<u64, PendingCall>,
    live: Arc<Mutex<bool>>,
    ordering: OrderingIndex,
    scope_charges: HashMap<ScopeKey, Reservation>,
}
impl LocalMailboxService {
    pub fn snapshot(&self) -> ServiceSnapshot {
        let (fifo_scopes, fifo_entries) = self.ordering.counts();
        ServiceSnapshot {
            accepted: self.accepted.len(),
            calls: self.calls.len(),
            fifo_scopes,
            fifo_entries,
        }
    }
    pub(crate) fn uses_budget(&self, budget: &HostBudget) -> bool {
        self.budget.same_owner(budget)
    }
    pub fn new(budget: HostBudget) -> Result<Self> {
        let ordering =
            OrderingIndex::new(budget.limits.host.identities, budget.limits.host.identities)?;
        Ok(Self {
            budget,
            id: fresh_id()?,
            next: 0,
            accepted: BTreeMap::new(),
            queue: VecDeque::new(),
            last: None,
            shutting_down: false,
            calls: BTreeMap::new(),
            live: Arc::new(Mutex::new(true)),
            ordering,
            scope_charges: HashMap::new(),
        })
    }
    fn clock(&mut self, now: ServiceTime) -> Result<()> {
        if now.unix_ms < 0 || self.last.is_some_and(|t| now.monotonic_ms < t.monotonic_ms) {
            return Err(ServiceError::Clock);
        }
        let mut budget = self.budget.inner.lock().map_err(|_| ServiceError::State)?;
        if budget
            .last_monotonic_ms
            .is_some_and(|old| now.monotonic_ms < old)
        {
            return Err(ServiceError::Clock);
        }
        budget.last_monotonic_ms = Some(now.monotonic_ms);
        self.last = Some(now);
        Ok(())
    }
    pub fn admit(
        &mut self,
        binding: &MailboxBinding,
        encoded: &[u8],
        now: ServiceTime,
    ) -> Result<SignedDestinationReceipt> {
        self.clock(now)?;
        if encoded.len() > self.budget.limits.max_envelope_bytes {
            return Err(ServiceError::Limit);
        }
        let v = binding.verify(encoded, now)?;
        let receipt = self.admit_verified(binding, v, encoded.len(), now)?;
        self.drain_ready_replies(now)?;
        Ok(receipt)
    }
    pub(crate) fn admit_for_control<T>(
        &mut self,
        binding: &MailboxBinding,
        encoded: &[u8],
        now: ServiceTime,
        reserve: impl FnOnce(&Verified) -> Result<T>,
    ) -> Result<(SignedDestinationReceipt, T)> {
        self.clock(now)?;
        if encoded.len() > self.budget.limits.max_envelope_bytes {
            return Err(ServiceError::Limit);
        }
        let verified = binding.verify(encoded, now)?;
        if verified.header().content_type.name() == crate::receipt::RECEIPT_CONTENT_TYPE {
            return Err(ServiceError::Unauthorized);
        }
        let control = {
            let policy = binding.policy.lock().map_err(|_| ServiceError::State)?;
            if policy.generation != verified.generation {
                return Err(ServiceError::Unauthorized);
            }
            reserve(&verified)?
        };
        let receipt = self.admit_verified(binding, verified, encoded.len(), now)?;
        self.drain_ready_replies(now)?;
        Ok((receipt, control))
    }
    fn admit_verified(
        &mut self,
        binding: &MailboxBinding,
        v: Verified,
        encoded_len: usize,
        now: ServiceTime,
    ) -> Result<SignedDestinationReceipt> {
        if v.opened.header().content_type.name() == crate::receipt::RECEIPT_CONTENT_TYPE {
            return Err(ServiceError::Unauthorized);
        }
        if v.opened.payload().len() > self.budget.limits.max_payload_bytes {
            return binding.receipt(&v, DeliveryOutcome::Refused, now);
        }
        let key = MessageKey::new(address(&v.context), v.replay);
        if self.shutting_down {
            return binding.receipt(&v, DeliveryOutcome::Unavailable, now);
        }
        let h = v.opened.header();
        let queued = h.kind != MailboxMessageKind::Reply || self.matching_call(h, now).is_some();
        let outcome = if queued {
            DeliveryOutcome::AcceptedEphemeral
        } else {
            DeliveryOutcome::Late
        };
        if self
            .budget
            .current_epoch(address(&v.context))?
            .is_some_and(|epoch| h.execution_epoch < epoch)
        {
            return Err(ServiceError::Unauthorized);
        }
        if let Some(outcome) = self.budget.duplicate(key, v.commitment)? {
            let policy = binding.policy.lock().map_err(|_| ServiceError::State)?;
            if policy.generation != v.generation {
                return Err(ServiceError::Unauthorized);
            }
            return binding.receipt(&v, outcome, now);
        }
        if let Some(old) = self.accepted.get(&key) {
            return binding.receipt(
                &v,
                if old.commitment != v.commitment {
                    DeliveryOutcome::Refused
                } else if now.monotonic_ms >= old.deadline {
                    DeliveryOutcome::Expired
                } else {
                    DeliveryOutcome::Duplicate
                },
                now,
            );
        }
        match self.ordering.check(h) {
            Ok(()) => {}
            Err(ServiceError::Conflict) => {
                return binding.receipt(&v, DeliveryOutcome::Refused, now);
            }
            Err(ServiceError::Full | ServiceError::Limit) => {
                return binding.receipt(&v, DeliveryOutcome::Full, now);
            }
            Err(error) => return Err(error),
        }
        let (deadline, retention) = binding.deadlines(h, now, self.budget.limits.retention_ms)?;
        let scope = Scope {
            tenant: h.tenant,
            address: address(&v.context),
            sender: h.sender,
        };
        let bytes = if queued { encoded_len as u64 } else { 0 };
        let owner = match self.budget.claim_owner(scope, h.execution_epoch, self.id) {
            Ok(owner) => owner,
            Err(ServiceError::Unavailable) => {
                return binding.receipt(&v, DeliveryOutcome::Unavailable, now);
            }
            Err(ServiceError::Full) => return binding.receipt(&v, DeliveryOutcome::Full, now),
            Err(e) => return Err(e),
        };
        let scope_charge = if !self.ordering.contains_scope(h) {
            match self.budget.reserve(
                scope,
                Usage {
                    identities: 1,
                    ..Usage::default()
                },
                &binding.config.capability.grant,
            ) {
                Ok(charge) => Some(charge),
                Err(ServiceError::Full) => return binding.receipt(&v, DeliveryOutcome::Full, now),
                Err(error) => return Err(error),
            }
        } else {
            None
        };
        let entry_charge = match self.budget.reserve(
            scope,
            Usage {
                identities: 1,
                ..Usage::default()
            },
            &binding.config.capability.grant,
        ) {
            Ok(charge) => charge,
            Err(ServiceError::Full) => return binding.receipt(&v, DeliveryOutcome::Full, now),
            Err(error) => return Err(error),
        };
        if bytes > binding.config.capability.grant.max_queue_contribution_bytes {
            return binding.receipt(&v, DeliveryOutcome::Full, now);
        }
        let reserve = Usage {
            bytes,
            messages: usize::from(queued),
            calls: usize::from(h.kind == MailboxMessageKind::Call),
            identities: 1,
        };
        let receipt = binding.receipt(&v, outcome, now)?;
        let next = self.next.checked_add(1).ok_or(ServiceError::Limit)?;
        let policy = binding.policy.lock().map_err(|_| ServiceError::State)?;
        if policy.generation != v.generation {
            return Err(ServiceError::Unauthorized);
        }
        let active_owner = owner.lock().map_err(|_| ServiceError::State)?;
        if active_owner.epoch != h.execution_epoch || active_owner.service != self.id {
            return Err(ServiceError::Unauthorized);
        }
        let mut reservation = match self.budget.reserve_admission(AdmissionInput {
            scope,
            delta: reserve,
            key,
            commitment: v.commitment,
            retention,
            receipt: receipt.clone(),
            grant: &binding.config.capability.grant,
            now: now.monotonic_ms,
        }) {
            Ok(AdmissionReservation::Reserved(x)) => *x,
            Ok(AdmissionReservation::Duplicate) => {
                return binding.receipt(&v, DeliveryOutcome::Duplicate, now);
            }
            Ok(AdmissionReservation::Conflict) => {
                return binding.receipt(&v, DeliveryOutcome::Refused, now);
            }
            Err(ServiceError::Full | ServiceError::Backpressured) => {
                return binding.receipt(&v, DeliveryOutcome::Full, now);
            }
            Err(e) => return Err(e),
        };
        let generation = policy.generation;
        self.ordering.insert(h)?;
        if let Some(charge) = scope_charge {
            self.scope_charges.insert(scope_key(h), charge);
        }
        self.next = next;
        let handle = DeliveryHandle {
            service: self.id,
            id: self.next,
            binding: binding.id,
            generation,
        };
        let queue = Reservation {
            budget: self.budget.clone(),
            scope,
            delta: Usage {
                bytes,
                messages: usize::from(queued),
                ..Usage::default()
            },
            class: BudgetClass::Data,
            grant: reservation.grant,
        };
        reservation.delta.bytes = 0;
        reservation.delta.messages = 0;
        let header = v.opened.header().clone();
        self.accepted.insert(
            key,
            Accepted {
                commitment: v.commitment,
                header: v.opened.header().clone(),
                policy: Arc::clone(&binding.policy),
                handle,
                deadline,
                retention,
                message: queued.then_some(v.opened),
                queue: queued.then_some(queue),
                active: reservation,
                replied: false,
                reply_cache: None,
                owner: Arc::clone(&owner),
                order_entry: Some(entry_charge),
                disposition: outcome,
            },
        );
        if queued {
            self.queue.push_back(key);
        } else {
            self.ordering.expire(&header)?;
        }
        self.release_retired_order_entries();
        Ok(receipt)
    }

    fn matching_call(&self, h: &MailboxEnvelopeHeader, now: ServiceTime) -> Option<u64> {
        self.calls.iter().find_map(|(id, p)| {
            let route = p.header.reply_to?;
            let current = p
                .policy
                .lock()
                .ok()
                .is_some_and(|policy| policy.generation == p.handle.generation);
            (current
                && p.status == CallStatus::Pending
                && now.monotonic_ms < p.deadline
                && now.unix_ms < p.header.expires_at_unix_ms
                && h.tenant == route.address.tenant
                && h.authorization_domain == route.address.authorization_domain
                && h.mailbox == route.address.mailbox
                && h.recipient == route.address.recipient
                && h.sender == p.header.recipient
                && h.recipient == p.header.sender
                && h.capability_id == route.capability_id
                && h.correlation_id == p.header.correlation_id
                && h.causation_id.as_bytes() == p.header.message_id.as_bytes())
            .then_some(*id)
        })
    }
    fn release_retired_order_entries(&mut self) {
        for entry in self.accepted.values_mut() {
            if entry.order_entry.is_some() && !self.ordering.contains_entry(&entry.header) {
                entry.order_entry.take();
            }
        }
    }
    fn drain_ready_replies(&mut self, now: ServiceTime) -> Result<usize> {
        let mut delivered = 0;
        let scan = self.queue.len().min(256);
        for _ in 0..scan {
            let key = self.queue.pop_front().ok_or(ServiceError::State)?;
            let ready = self.accepted.get(&key).is_some_and(|entry| {
                entry.header.kind == MailboxMessageKind::Reply
                    && self.ordering.is_ready(&entry.header)
            });
            if !ready {
                self.queue.push_back(key);
                continue;
            }
            let entry = self.accepted.get(&key).ok_or(ServiceError::State)?;
            let header = entry.header.clone();
            let incoming_policy = Arc::clone(&entry.policy);
            let incoming_id = entry.handle.binding;
            let incoming_generation = entry.handle.generation;
            let owner_state = Arc::clone(&entry.owner);
            let deadline = entry.deadline;
            let pending = self.matching_call(&header, now);
            let pending_policy = pending.map(|id| {
                let p = &self.calls[&id];
                (
                    id,
                    p.handle.binding,
                    p.handle.generation,
                    Arc::clone(&p.policy),
                )
            });
            // Find candidates without nested guards, then lock both in the same
            // immutable order used by reply preparation and cache replay.
            let policies = if let Some((_, id, _, policy)) = &pending_policy {
                Some(policy_pair(incoming_id, &incoming_policy, *id, policy)?)
            } else {
                None
            };
            let single = if policies.is_none() {
                Some(incoming_policy.lock().map_err(|_| ServiceError::State)?)
            } else {
                None
            };
            let owner = owner_state.lock().map_err(|_| ServiceError::State)?;
            let incoming_current = policies.as_ref().map_or_else(
                || {
                    single
                        .as_ref()
                        .is_some_and(|p| p.generation == incoming_generation)
                },
                |p| p.a_generation == incoming_generation,
            );
            let valid = incoming_current
                && owner.epoch == header.execution_epoch
                && owner.service == self.id
                && now.monotonic_ms < deadline
                && now.unix_ms < header.expires_at_unix_ms;
            let pending = pending_policy.as_ref().and_then(|(id, _, generation, _)| {
                let p = &self.calls[id];
                (valid
                    && policies
                        .as_ref()
                        .is_some_and(|pair| pair.b_generation == *generation)
                    && p.status == CallStatus::Pending
                    && now.monotonic_ms < p.deadline
                    && now.unix_ms < p.header.expires_at_unix_ms)
                    .then_some(*id)
            });
            let entry = self.accepted.get_mut(&key).ok_or(ServiceError::State)?;
            let payload = entry.message.take().ok_or(ServiceError::State)?;
            let queue = entry.queue.take().ok_or(ServiceError::State)?;
            self.ordering.mark_delivered(&header)?;
            entry.order_entry.take();
            if let Some(id) = pending {
                let call = self.calls.get_mut(&id).ok_or(ServiceError::State)?;
                call.status = CallStatus::Replied(Some(payload.into_payload()));
                call.active.release_calls()?;
                call.reply_budget = Some(queue);
                delivered += 1;
            } else {
                entry.disposition = DeliveryOutcome::Late;
            }
        }
        self.release_retired_order_entries();
        Ok(delivered)
    }

    pub fn receive(
        &mut self,
        binding: &MailboxBinding,
        now: ServiceTime,
    ) -> Result<Option<AdmittedMessage>> {
        self.clock(now)?;
        self.drain_ready_replies(now)?;
        let (_, generation) = binding.checked(now)?;
        let policy = binding.policy.lock().map_err(|_| ServiceError::State)?;
        if policy.generation != generation {
            return Err(ServiceError::Unauthorized);
        }
        if self.shutting_down {
            return Err(ServiceError::ShuttingDown);
        }
        let scan = self.queue.len().min(256);
        let mut ready = None;
        for _ in 0..scan {
            let key = self.queue.pop_front().ok_or(ServiceError::State)?;
            let take = self.accepted.get(&key).is_some_and(|entry| {
                entry.handle.binding == binding.id
                    && entry.header.kind != MailboxMessageKind::Reply
                    && self.ordering.is_ready(&entry.header)
            });
            if take {
                ready = Some(key);
                break;
            }
            self.queue.push_back(key);
        }
        let Some(key) = ready else {
            return if self.queue.len() > scan {
                Err(ServiceError::Backpressured)
            } else {
                Ok(None)
            };
        };
        let entry = self.accepted.get_mut(&key).ok_or(ServiceError::State)?;
        if generation != entry.handle.generation {
            return Err(ServiceError::Unauthorized);
        }
        let owner = entry.owner.lock().map_err(|_| ServiceError::State)?;
        if owner.epoch != entry.header.execution_epoch || owner.service != self.id {
            return Err(ServiceError::Unauthorized);
        }
        if now.monotonic_ms >= entry.deadline {
            return Err(ServiceError::Expired);
        }
        self.ordering.mark_delivered(&entry.header)?;
        entry.order_entry.take();
        Ok(Some(AdmittedMessage {
            opened: entry.message.take().ok_or(ServiceError::State)?,
            handle: entry.handle,
            _queue: entry.queue.take().ok_or(ServiceError::State)?,
            deadline: entry.deadline,
            live: Arc::clone(&self.live),
            owner: Arc::clone(&entry.owner),
        }))
    }

    pub fn register_call(
        &mut self,
        binding: &MailboxBinding,
        encoded: &[u8],
        now: ServiceTime,
    ) -> Result<CallHandle> {
        self.clock(now)?;
        if self.shutting_down {
            return Err(ServiceError::ShuttingDown);
        }
        self.expire(now)?;
        if encoded.len() > self.budget.limits.max_envelope_bytes {
            return Err(ServiceError::Limit);
        }
        let v = binding.verify(encoded, now)?;
        if v.opened.payload().len() > self.budget.limits.max_payload_bytes {
            return Err(ServiceError::Limit);
        }
        let header = v.opened.header();
        if header.kind != MailboxMessageKind::Call
            || binding.config.signing_key.is_none()
            || header.reply_to.is_none()
        {
            return Err(ServiceError::Unauthorized);
        }
        if let Some(old) = self.calls.values().find(|c| {
            c.header.tenant == header.tenant
                && c.header.authorization_domain == header.authorization_domain
                && c.header.sender == header.sender
                && c.header.execution_epoch == header.execution_epoch
                && c.header.correlation_id == header.correlation_id
        }) {
            return if old.commitment == v.commitment
                && old.status == CallStatus::Pending
                && old.handle.binding == binding.id
                && old.handle.generation == v.generation
            {
                Ok(old.handle)
            } else {
                Err(ServiceError::Conflict)
            };
        }
        let (deadline, retention) =
            binding.deadlines(header, now, self.budget.limits.retention_ms)?;
        let scope = Scope {
            tenant: header.tenant,
            address: address(&v.context),
            sender: header.sender,
        };
        let policy = binding.policy.lock().map_err(|_| ServiceError::State)?;
        if policy.generation != v.generation {
            return Err(ServiceError::Unauthorized);
        }
        let next = self.next.checked_add(1).ok_or(ServiceError::Limit)?;
        let active = self.budget.reserve(
            scope,
            Usage {
                calls: 1,
                identities: 1,
                ..Usage::default()
            },
            &binding.config.capability.grant,
        )?;
        self.next = next;
        let handle = CallHandle {
            service: self.id,
            id: self.next,
            binding: binding.id,
            generation: v.generation,
        };
        self.calls.insert(
            handle.id,
            PendingCall {
                header: header.clone(),
                commitment: v.commitment,
                handle,
                deadline,
                retention,
                status: CallStatus::Pending,
                active,
                reply_budget: None,
                policy: Arc::clone(&binding.policy),
            },
        );
        Ok(handle)
    }
    pub fn poll_call(
        &mut self,
        binding: &MailboxBinding,
        call: &CallHandle,
        now: ServiceTime,
    ) -> Result<CallStatus> {
        self.clock(now)?;
        self.expire(now)?;
        self.drain_ready_replies(now)?;
        let (_, generation) = binding.checked(now)?;
        let policy = binding.policy.lock().map_err(|_| ServiceError::State)?;
        if policy.generation != generation {
            return Err(ServiceError::Unauthorized);
        }
        let pending = self
            .calls
            .get_mut(&call.id)
            .ok_or(ServiceError::UnknownHandle)?;
        if call.service != self.id
            || call.binding != binding.id
            || call.generation != generation
            || pending.handle != *call
        {
            return Err(ServiceError::Unauthorized);
        }
        if let CallStatus::Replied(payload) = &mut pending.status {
            let out = payload.take();
            pending.reply_budget.take();
            return Ok(CallStatus::Replied(out));
        }
        Ok(pending.status.clone())
    }
    pub fn cancel(
        &mut self,
        binding: &MailboxBinding,
        call: &CallHandle,
        now: ServiceTime,
    ) -> Result<CallStatus> {
        self.clock(now)?;
        let (_, generation) = binding.checked(now)?;
        let policy = binding.policy.lock().map_err(|_| ServiceError::State)?;
        if policy.generation != generation {
            return Err(ServiceError::Unauthorized);
        }
        let pending = self
            .calls
            .get_mut(&call.id)
            .ok_or(ServiceError::UnknownHandle)?;
        if call.service != self.id
            || call.binding != binding.id
            || call.generation != generation
            || pending.handle != *call
        {
            return Err(ServiceError::Unauthorized);
        }
        if pending.status == CallStatus::Pending {
            pending.status = CallStatus::Cancelled;
            pending.active.release_calls()?;
        }
        drop(policy);
        self.poll_call(binding, call, now)
    }
    pub fn reply(
        &mut self,
        binding: &MailboxBinding,
        call: &DeliveryHandle,
        request: SealRequest<'_>,
        now: ServiceTime,
    ) -> Result<MailboxEnvelope> {
        self.reply_prepared(binding, call, request, now)
            .map(|prepared| prepared.envelope().clone())
    }
    pub fn reply_prepared(
        &mut self,
        binding: &MailboxBinding,
        call: &DeliveryHandle,
        request: SealRequest<'_>,
        now: ServiceTime,
    ) -> Result<PreparedMailboxEnvelope> {
        self.clock(now)?;
        self.expire(now)?;
        if self.shutting_down {
            return Err(ServiceError::ShuttingDown);
        }
        let entry = self
            .accepted
            .values_mut()
            .find(|a| a.handle == *call)
            .ok_or(ServiceError::UnknownHandle)?;
        if call.service != self.id || entry.header.kind != MailboxMessageKind::Call {
            return Err(ServiceError::Unauthorized);
        }
        if entry.replied {
            return Err(ServiceError::Conflict);
        }
        if now.monotonic_ms >= entry.deadline || now.unix_ms >= entry.header.expires_at_unix_ms {
            return Err(ServiceError::Expired);
        }
        let route = entry.header.reply_to.ok_or(ServiceError::Unauthorized)?;
        let header = &request.header;
        if binding.config.kind != MailboxMessageKind::Reply
            || header.kind != MailboxMessageKind::Reply
            || header.sender != entry.header.recipient
            || header.recipient != entry.header.sender
            || header.tenant != route.address.tenant
            || header.authorization_domain != route.address.authorization_domain
            || header.mailbox != route.address.mailbox
            || header.recipient != route.address.recipient
            || header.capability_id != route.capability_id
            || header.correlation_id != entry.header.correlation_id
            || header.causation_id.as_bytes() != entry.header.message_id.as_bytes()
            || header.expires_at_unix_ms > entry.header.expires_at_unix_ms
            || request.plaintext.len() > self.budget.limits.max_payload_bytes
        {
            return Err(ServiceError::Unauthorized);
        }
        let key = binding
            .config
            .signing_key
            .as_ref()
            .ok_or(ServiceError::Unauthorized)?;
        let prepared = prepare_mailbox_envelope(request, &binding.config.encryption_key, key)
            .map_err(|_| ServiceError::Malformed)?;
        let encoded =
            mailbox::encode_envelope(prepared.envelope()).map_err(|_| ServiceError::Limit)?;
        if encoded.len() > self.budget.limits.max_envelope_bytes {
            return Err(ServiceError::Limit);
        }
        // Canonical verification rechecks the exact independently granted reply.
        let verified = binding.verify(&encoded, now)?;
        let policies = policy_pair(call.binding, &entry.policy, binding.id, &binding.policy)?;
        let owner = entry.owner.lock().map_err(|_| ServiceError::State)?;
        if policies.a_generation != call.generation
            || policies.b_generation != verified.generation
            || owner.epoch != entry.header.execution_epoch
            || owner.service != self.id
        {
            return Err(ServiceError::Unauthorized);
        }
        let scope = Scope {
            tenant: verified.context.expected_tenant,
            address: address(&verified.context),
            sender: verified.context.expected_sender,
        };
        let reservation = self.budget.reserve(
            scope,
            Usage {
                bytes: encoded.len() as u64,
                messages: 1,
                ..Usage::default()
            },
            &binding.config.capability.grant,
        )?;
        entry.reply_cache = Some(CachedReply {
            prepared: prepared.clone(),
            header: verified.opened.header().clone(),
            binding: binding.id,
            generation: verified.generation,
            policy: Arc::clone(&binding.policy),
            _budget: reservation,
        });
        entry.replied = true;
        entry.active.release_calls()?;
        Ok(prepared)
    }
    pub fn retry_reply_prepared(
        &mut self,
        binding: &MailboxBinding,
        encoded_request: &[u8],
        now: ServiceTime,
    ) -> Result<Option<PreparedMailboxEnvelope>> {
        self.clock(now)?;
        self.expire(now)?;
        if self.shutting_down {
            return Err(ServiceError::ShuttingDown);
        }
        let verified = binding.verify(encoded_request, now)?;
        if verified.opened.header().kind != MailboxMessageKind::Call {
            return Err(ServiceError::Unauthorized);
        }
        let key = MessageKey::new(address(&verified.context), verified.replay);
        let original = self
            .budget
            .inner
            .lock()
            .map_err(|_| ServiceError::State)?
            .accepted
            .get(&key)
            .map(|a| a.commitment)
            .ok_or(ServiceError::UnknownHandle)?;
        if original != verified.commitment {
            return Err(ServiceError::Conflict);
        }
        let entry = self.accepted.get(&key).ok_or(ServiceError::Unavailable)?;
        let owner = entry.owner.lock().map_err(|_| ServiceError::State)?;
        if owner.epoch != entry.header.execution_epoch || owner.service != self.id {
            return Err(ServiceError::Unauthorized);
        }
        drop(owner);
        if verified.generation != entry.handle.generation || binding.id != entry.handle.binding {
            return Err(ServiceError::Unauthorized);
        }
        if now.monotonic_ms >= entry.deadline || now.unix_ms >= entry.header.expires_at_unix_ms {
            return Err(ServiceError::Expired);
        }
        let Some(cache) = &entry.reply_cache else {
            return Ok(None);
        };
        let policies = policy_pair(binding.id, &binding.policy, cache.binding, &cache.policy)?;
        let owner = entry.owner.lock().map_err(|_| ServiceError::State)?;
        if policies.a_generation != entry.handle.generation
            || policies.b_generation != cache.generation
            || owner.epoch != entry.header.execution_epoch
            || owner.service != self.id
        {
            return Err(ServiceError::Unauthorized);
        }
        if now.unix_ms >= cache.header.expires_at_unix_ms {
            return Err(ServiceError::Expired);
        }
        Ok(Some(cache.prepared.clone()))
    }
    pub fn acknowledge_reply(
        &mut self,
        binding: &MailboxBinding,
        receipt: &SignedDestinationReceipt,
        destination_key: &VerifyingKey,
        now: ServiceTime,
    ) -> Result<bool> {
        self.clock(now)?;
        let (context, generation) = binding.checked(now)?;
        mailbox::verify_receipt(receipt, destination_key)
            .map_err(|_| ServiceError::Unauthorized)?;
        if !matches!(
            receipt.receipt.outcome,
            DeliveryOutcome::AcceptedEphemeral
                | DeliveryOutcome::AcceptedDurable
                | DeliveryOutcome::Duplicate
        ) {
            return Ok(false);
        }
        let policy = binding.policy.lock().map_err(|_| ServiceError::State)?;
        if policy.generation != generation {
            return Err(ServiceError::Unauthorized);
        }
        let Some(entry) = self.accepted.values_mut().find(|a| {
            a.reply_cache
                .as_ref()
                .is_some_and(|c| c.header.message_id == receipt.receipt.message_id)
        }) else {
            return Ok(false);
        };
        let cache = entry.reply_cache.as_ref().ok_or(ServiceError::State)?;
        let e = cache.prepared.envelope();
        if cache.binding != binding.id
            || cache.generation != generation
            || receipt.receipt.address != address(&context)
            || receipt.receipt.execution_epoch != context.current_execution_epoch
            || receipt.receipt.delivery_id != e.routing.delivery_id
            || receipt.receipt.ciphertext_commitment != e.ciphertext_commitment
        {
            return Err(ServiceError::Unauthorized);
        }
        entry.reply_cache.take();
        Ok(true)
    }
    pub fn expire(&mut self, now: ServiceTime) -> Result<usize> {
        self.clock(now)?;
        let mut count = 0;
        for pending in self.calls.values_mut() {
            if (now.monotonic_ms >= pending.deadline
                || now.unix_ms >= pending.header.expires_at_unix_ms)
                && pending.status == CallStatus::Pending
            {
                pending.status = CallStatus::TimedOut;
                pending.active.release_calls()?;
                count += 1;
            }
        }
        for accepted in self.accepted.values_mut() {
            if now.monotonic_ms >= accepted.deadline
                || now.unix_ms >= accepted.header.expires_at_unix_ms
            {
                if accepted.message.is_some() && self.ordering.contains_entry(&accepted.header) {
                    self.ordering.expire(&accepted.header)?;
                    if accepted.disposition == DeliveryOutcome::AcceptedEphemeral {
                        accepted.disposition = DeliveryOutcome::Expired;
                    }
                }
                accepted.message.take();
                accepted.queue.take();
                accepted.reply_cache.take();
                accepted.active.release_calls()?;
            }
        }
        self.calls.retain(|_, c| now.monotonic_ms < c.retention);
        for accepted in self.accepted.values_mut() {
            if accepted.order_entry.is_some() && !self.ordering.contains_entry(&accepted.header) {
                accepted.order_entry.take();
            }
        }
        self.accepted
            .retain(|_, a| now.monotonic_ms < a.retention || a.order_entry.is_some());
        self.queue
            .retain(|key| self.accepted.get(key).is_some_and(|a| a.message.is_some()));
        count += self.budget.expire(now)?;
        Ok(count)
    }
    pub fn shutdown(&mut self, now: ServiceTime) -> Result<()> {
        self.clock(now)?;
        let mut live = self.live.lock().map_err(|_| ServiceError::State)?;
        *live = false;
        self.shutting_down = true;
        for c in self.calls.values_mut() {
            if c.status == CallStatus::Pending {
                c.status = CallStatus::ShuttingDown;
                c.active.release_calls()?;
            }
        }
        for a in self.accepted.values_mut() {
            a.message.take();
            a.queue.take();
            a.reply_cache.take();
            a.order_entry.take();
            a.active.release_calls()?;
        }
        self.queue.clear();
        self.scope_charges.clear();
        self.ordering = OrderingIndex::new(
            self.budget.limits.host.identities,
            self.budget.limits.host.identities,
        )?;
        Ok(())
    }
}
impl Drop for LocalMailboxService {
    fn drop(&mut self) {
        if let Ok(mut live) = self.live.lock() {
            *live = false;
        }
    }
}
fn deadlines(
    header: &MailboxEnvelopeHeader,
    now: ServiceTime,
    retention_ms: u64,
) -> Result<(u64, u64)> {
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
    let retention = deadline
        .checked_add(retention_ms)
        .ok_or(ServiceError::Clock)?;
    Ok((deadline, retention))
}
