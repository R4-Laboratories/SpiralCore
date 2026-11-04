#![forbid(unsafe_code)]
//! Host-local owner access admission. Transport connectivity is never authority.
mod transport;
pub use transport::{AccessSocket, OwnerClient, WireCounters};
#[cfg(all(unix, feature = "native"))]
pub mod native;
#[cfg(all(test, unix, feature = "object-v1"))]
#[path = "native_object_tests.rs"]
mod native_object_tests;
#[cfg(all(test, unix, feature = "native"))]
#[path = "native_tests.rs"]
mod native_tests;
use chrono::{DateTime, Utc};
use ed25519_dalek::VerifyingKey;
use mvm_contract::{plan::VerbGrant, policy::security::AgentProfile};
use mvm_core::plan::{SignedExecutionPlan, verify_plan, verify_plan_id};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Error {
    Profile,
    Signature,
    Plan,
    Owner,
    Policy,
    Revoked,
    Epoch,
    Expired,
    Replay,
    Limit,
    Busy,
    Closed,
    Audit,
    State,
    Unsupported,
    Transport,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "access: {self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleBinding {
    pub instance: String,
    pub tenant: String,
    pub image_sha256: String,
    /// Local subtractive control state, never wire authorization or a MVM epoch.
    pub revocation_generation: u64,
    pub profile: AgentProfile,
}
struct ControlState {
    binding: LifecycleBinding,
    revoked: bool,
    current: Option<Arc<dyn Fn() -> Result<()> + Send + Sync>>,
}
/// Trusted installed-host lifecycle control, shared with its service. This is
/// not a remotely deserializable credential or a transport-derived identity.
#[derive(Clone)]
pub struct AccessControl(Arc<Mutex<ControlState>>);
impl AccessControl {
    pub fn new(binding: LifecycleBinding) -> Self {
        Self(Arc::new(Mutex::new(ControlState {
            binding,
            revoked: false,
            current: None,
        })))
    }
    pub fn with_current_check(
        binding: LifecycleBinding,
        current: Arc<dyn Fn() -> Result<()> + Send + Sync>,
    ) -> Self {
        Self(Arc::new(Mutex::new(ControlState {
            binding,
            revoked: false,
            current: Some(current),
        })))
    }
    pub fn revoke(&self) -> Result<()> {
        self.0.lock().map_err(|_| Error::Closed)?.revoked = true;
        Ok(())
    }
    pub fn transition(&self, binding: LifecycleBinding) -> Result<()> {
        self.0.lock().map_err(|_| Error::Closed)?.binding = binding;
        Ok(())
    }
    fn snapshot(&self) -> Result<LifecycleBinding> {
        let state = self.0.lock().map_err(|_| Error::Closed)?;
        if state.revoked {
            Err(Error::Revoked)
        } else {
            let binding = state.binding.clone();
            let current = state.current.clone();
            drop(state);
            if let Some(current) = current {
                current()?;
            }
            Ok(binding)
        }
    }
}
#[derive(Clone, Copy)]
pub struct Time {
    pub unix: DateTime<Utc>,
    pub monotonic: Instant,
}
pub trait Clock: Send + Sync {
    fn now(&self) -> Time;
}
impl<F: Fn() -> Time + Send + Sync> Clock for F {
    fn now(&self) -> Time {
        self()
    }
}

/// One trusted host/tenant allocation, injected into every access service.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub in_flight: usize,
    pub request_bytes: usize,
    pub reply_bytes: usize,
    /// Aggregate shared-core pair bytes; zero keeps object mapping disabled.
    pub codec_bytes: usize,
    /// Aggregate canonical JSON/encryption frame workspace for object mapping.
    pub wire_bytes: usize,
    pub requests_per_second: u32,
    pub replay_entries: usize,
    pub frames_per_request: u32,
    pub deadline: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            in_flight: 2,
            request_bytes: 128 * 1024,
            reply_bytes: 128 * 1024,
            codec_bytes: 0,
            wire_bytes: 0,
            requests_per_second: 32,
            replay_entries: 4096,
            frames_per_request: 16,
            deadline: Duration::from_secs(3),
        }
    }
}
struct BudgetState {
    limits: Limits,
    active: usize,
    bytes: usize,
    reply_bytes: usize,
    codec_bytes: usize,
    wire_bytes: usize,
    bucket: Option<Instant>,
    rate: u32,
    closed: bool,
}
#[derive(Clone)]
pub struct SharedBudget(Arc<Mutex<BudgetState>>);
impl SharedBudget {
    pub fn new(limits: Limits) -> Result<Self> {
        if limits.in_flight == 0
            || limits.in_flight > 1024
            || limits.request_bytes == 0
            || limits.reply_bytes == 0
            || limits.request_bytes > 64 * 1024 * 1024
            || limits.reply_bytes > 64 * 1024 * 1024
            || limits.codec_bytes > 64 * 1024 * 1024
            || limits.wire_bytes > 64 * 1024 * 1024
            || limits.requests_per_second == 0
            || limits.replay_entries == 0
            || limits.replay_entries > 65536
            || limits.frames_per_request == 0
            || limits.deadline.is_zero()
            || limits.deadline > Duration::from_secs(30)
        {
            return Err(Error::Limit);
        }
        Ok(Self(Arc::new(Mutex::new(BudgetState {
            limits,
            active: 0,
            bytes: 0,
            reply_bytes: 0,
            codec_bytes: 0,
            wire_bytes: 0,
            bucket: None,
            rate: 0,
            closed: false,
        }))))
    }
    fn reserve(&self, bytes: usize, now: Instant) -> Result<Charge> {
        let mut state = self.0.lock().map_err(|_| Error::Closed)?;
        if state.closed {
            return Err(Error::Closed);
        }
        if state.bucket.is_some_and(|last| now < last) {
            state.closed = true;
            return Err(Error::State);
        }
        if state
            .bucket
            .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(1))
        {
            state.bucket = Some(now);
            state.rate = 0;
        }
        let reply_bytes = state.limits.reply_bytes.min(64 * 1024);
        if state.active >= state.limits.in_flight
            || state
                .bytes
                .checked_add(bytes)
                .is_none_or(|n| n > state.limits.request_bytes)
            || state.rate >= state.limits.requests_per_second
            || state
                .reply_bytes
                .checked_add(reply_bytes)
                .is_none_or(|n| n > state.limits.reply_bytes)
        {
            return Err(Error::Busy);
        }
        state.active += 1;
        state.bytes += bytes;
        state.reply_bytes += reply_bytes;
        state.rate += 1;
        Ok(Charge {
            budget: self.clone(),
            bytes,
            reply_bytes,
        })
    }
    fn limits(&self) -> Result<Limits> {
        let state = self.0.lock().map_err(|_| Error::Closed)?;
        if state.closed {
            Err(Error::Closed)
        } else {
            Ok(state.limits)
        }
    }
    pub fn usage(&self) -> Result<(usize, usize)> {
        let state = self.0.lock().map_err(|_| Error::Closed)?;
        Ok((state.active, state.bytes))
    }
    #[cfg(all(unix, feature = "object-v1"))]
    fn reserve_object(&self, codec_bytes: usize, wire_bytes: usize) -> Result<ObjectCharge> {
        let mut state = self.0.lock().map_err(|_| Error::Closed)?;
        if state.closed {
            return Err(Error::Closed);
        }
        let codec = state
            .codec_bytes
            .checked_add(codec_bytes)
            .ok_or(Error::Limit)?;
        let wire = state
            .wire_bytes
            .checked_add(wire_bytes)
            .ok_or(Error::Limit)?;
        if codec > state.limits.codec_bytes || wire > state.limits.wire_bytes {
            return Err(Error::Busy);
        }
        state.codec_bytes = codec;
        state.wire_bytes = wire;
        Ok(ObjectCharge {
            budget: self.clone(),
            codec_bytes,
            wire_bytes,
        })
    }
    pub fn object_usage(&self) -> Result<(usize, usize)> {
        let state = self.0.lock().map_err(|_| Error::Closed)?;
        Ok((state.codec_bytes, state.wire_bytes))
    }
}
#[cfg(all(unix, feature = "object-v1"))]
struct ObjectCharge {
    budget: SharedBudget,
    codec_bytes: usize,
    wire_bytes: usize,
}
#[cfg(all(unix, feature = "object-v1"))]
impl Drop for ObjectCharge {
    fn drop(&mut self) {
        if let Ok(mut state) = self.budget.0.lock() {
            match (
                state.codec_bytes.checked_sub(self.codec_bytes),
                state.wire_bytes.checked_sub(self.wire_bytes),
            ) {
                (Some(codec), Some(wire)) => {
                    state.codec_bytes = codec;
                    state.wire_bytes = wire;
                }
                _ => state.closed = true,
            }
        }
    }
}
struct Charge {
    budget: SharedBudget,
    bytes: usize,
    reply_bytes: usize,
}
impl Drop for Charge {
    fn drop(&mut self) {
        if let Ok(mut state) = self.budget.0.lock() {
            match (
                state.active.checked_sub(1),
                state.bytes.checked_sub(self.bytes),
                state.reply_bytes.checked_sub(self.reply_bytes),
            ) {
                (Some(active), Some(bytes), Some(reply_bytes)) => {
                    state.active = active;
                    state.bytes = bytes;
                    state.reply_bytes = reply_bytes;
                }
                _ => state.closed = true,
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct LocalPolicy {
    owners: BTreeSet<String>,
    verbs: BTreeSet<String>,
}
impl LocalPolicy {
    pub fn new<O: Into<String>, V: Into<String>>(
        owners: impl IntoIterator<Item = O>,
        verbs: impl IntoIterator<Item = V>,
    ) -> Self {
        Self {
            owners: owners.into_iter().map(Into::into).collect(),
            verbs: verbs.into_iter().map(Into::into).collect(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum AuditKind {
    Granted,
    Request,
    Refused,
    Forward,
    Completed,
    Cancelled,
    Revoked,
    Shutdown,
}
#[derive(Clone, Debug, Serialize)]
pub struct AuditRecord {
    pub kind: AuditKind,
    pub reason: Option<Error>,
    pub request: [u8; 32],
    pub operation: String,
    pub sequence: u64,
    pub revocation_generation: u64,
    pub plan_id: String,
    pub plan_version: u32,
    pub plan_nonce: String,
    pub instance: String,
    pub tenant: String,
    pub bytes: usize,
}
pub trait Audit {
    fn append(&mut self, record: &AuditRecord) -> Result<()>;
}
impl<F: FnMut(&AuditRecord) -> Result<()>> Audit for F {
    fn append(&mut self, record: &AuditRecord) -> Result<()> {
        self(record)
    }
}
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counters {
    pub admitted: u64,
    pub forwarded_frames: u64,
    pub forwarded_bytes: u64,
    pub completed: u64,
    pub cancelled: u64,
}
pub struct RequestHandle {
    service: u64,
    id: [u8; 32],
    sequence: u64,
}
struct Pending {
    owner: String,
    verb: String,
    sequence: u64,
    deadline: Instant,
    unix_deadline: DateTime<Utc>,
    frames: u32,
    reply_bytes: usize,
    _charge: Charge,
}
static NEXT_SERVICE: AtomicU64 = AtomicU64::new(1);
pub struct Admission<'a> {
    pub signed: &'a SignedExecutionPlan,
    pub keys: &'a [(&'a str, &'a VerifyingKey)],
    pub grant: VerbGrant,
    pub host: &'a VerifyingKey,
    pub control: AccessControl,
    pub policy: LocalPolicy,
    pub budget: SharedBudget,
    pub clock: Arc<dyn Clock>,
}
pub struct AccessService<A: Audit> {
    id: u64,
    binding: LifecycleBinding,
    plan_id: mvm_contract::plan::PlanId,
    plan_version: u32,
    control: AccessControl,
    plan_verbs: BTreeSet<String>,
    grant: VerbGrant,
    policy: LocalPolicy,
    expiry: DateTime<Utc>,
    monotonic_expiry: Instant,
    last: Time,
    clock: Arc<dyn Clock>,
    budget: SharedBudget,
    audit: A,
    closed: bool,
    sequence: u64,
    seen: BTreeSet<[u8; 32]>,
    pending: BTreeMap<[u8; 32], Pending>,
    counters: Counters,
}
impl<A: Audit> AccessService<A> {
    pub fn admit(input: Admission<'_>, audit: A) -> Result<Self> {
        let plan = verify_plan(input.signed, input.keys).map_err(|_| Error::Signature)?;
        Self::admit_verified(plan, input, audit)
    }
    fn admit_verified(
        plan: mvm_core::plan::ExecutionPlan,
        input: Admission<'_>,
        audit: A,
    ) -> Result<Self> {
        let Admission {
            signed: _,
            keys: _,
            grant,
            host,
            control,
            policy,
            budget,
            clock,
        } = input;
        let binding = control.snapshot()?;
        let now = clock.now();
        verify_plan_id(&plan).map_err(|_| Error::Plan)?;
        mvm_core::plan::validity::check_window(&plan, now.unix).map_err(|_| Error::Expired)?;
        if binding.instance.is_empty()
            || binding.instance.len() > 128
            || binding.tenant != plan.tenant.0
            || binding.image_sha256 != plan.image.sha256
            || binding.revocation_generation == 0
        {
            return Err(Error::Plan);
        }
        grant
            .verify(host, &binding.instance, &plan.nonce, now.unix)
            .map_err(|_| Error::Signature)?;
        let expected = mvm_contract::grants::owner_access::OwnerAccessBinding::from_plan(
            &plan,
            &binding.instance,
        )
        .map_err(|_| Error::Policy)?;
        if grant.owner_access != expected {
            return Err(Error::Policy);
        }
        if grant.not_after > plan.valid_until || grant.not_after <= now.unix {
            return Err(Error::Expired);
        }
        budget.limits()?;
        if policy.owners.is_empty()
            || policy.owners.len() > 32
            || policy.verbs.len() > 32
            || policy.owners.iter().any(|v| v.is_empty() || v.len() > 128)
            || policy.verbs.iter().any(|v| !valid_verb(v))
        {
            return Err(Error::Policy);
        }
        let expiry = grant.not_after.min(plan.valid_until);
        let lifetime = (expiry - now.unix).to_std().map_err(|_| Error::Expired)?;
        let id = NEXT_SERVICE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| Error::Limit)?;
        let mut service = Self {
            id,
            binding,
            plan_id: plan.plan_id,
            plan_version: plan.plan_version,
            control,
            plan_verbs: plan
                .agent_verbs
                .unwrap_or_default()
                .into_iter()
                .map(|v| v.as_str().to_owned())
                .collect(),
            grant,
            policy,
            expiry,
            monotonic_expiry: now.monotonic.checked_add(lifetime).ok_or(Error::Expired)?,
            last: now,
            clock,
            budget,
            audit,
            closed: false,
            sequence: 0,
            seen: BTreeSet::new(),
            pending: BTreeMap::new(),
            counters: Counters::default(),
        };
        service.emit(AuditKind::Granted, None, [0; 32], "", 0, 0)?;
        Ok(service)
    }
    fn emit(
        &mut self,
        kind: AuditKind,
        reason: Option<Error>,
        request: [u8; 32],
        verb: &str,
        sequence: u64,
        bytes: usize,
    ) -> Result<()> {
        let record = AuditRecord {
            kind,
            reason,
            request,
            operation: verb.to_owned(),
            sequence,
            revocation_generation: self.binding.revocation_generation,
            plan_id: self.plan_id.0.clone(),
            plan_version: self.plan_version,
            plan_nonce: self.grant.plan_nonce.as_hex().to_owned(),
            instance: self.binding.instance.clone(),
            tenant: self.binding.tenant.clone(),
            bytes,
        };
        if self.audit.append(&record).is_err() {
            self.closed = true;
            self.pending.clear();
            return Err(Error::Audit);
        }
        Ok(())
    }
    fn current(&mut self) -> Result<Time> {
        if self.closed {
            return Err(Error::Closed);
        }
        let fresh = self.control.snapshot();
        let reason = match fresh {
            Ok(binding) if binding == self.binding => None,
            Ok(_) => Some(Error::Epoch),
            Err(error) => Some(error),
        };
        if let Some(error) = reason {
            self.closed = true;
            self.counters.cancelled += self.pending.len() as u64;
            self.pending.clear();
            return Err(error);
        }
        let now = self.clock.now();
        if now.monotonic < self.last.monotonic || now.unix < self.last.unix {
            self.closed = true;
            self.pending.clear();
            return Err(Error::State);
        }
        self.last = now;
        if now.unix >= self.expiry || now.monotonic >= self.monotonic_expiry {
            self.closed = true;
            self.pending.clear();
            return Err(Error::Expired);
        }
        self.budget.limits()?;
        Ok(now)
    }
    pub fn begin(
        &mut self,
        owner: &str,
        verb: &str,
        request: [u8; 32],
        bytes: usize,
    ) -> Result<RequestHandle> {
        let result = self.begin_inner(owner, verb, request, bytes);
        if let Err(reason) = result {
            let name = if valid_verb(verb) { verb } else { "invalid" };
            self.emit(
                AuditKind::Refused,
                Some(reason),
                digest(request),
                name,
                0,
                bytes,
            )?;
        }
        result
    }
    fn begin_typed(
        &mut self,
        owner: &str,
        request: &mvm_contract::protocol::owner_access::OwnerAccessRequest,
    ) -> Result<RequestHandle> {
        use mvm_contract::policy::security::AgentProfile;
        let result = (|| {
            if !matches!(
                self.binding.profile,
                AgentProfile::Dev | AgentProfile::SealedProd
            ) {
                return Err(Error::Profile);
            }
            let now = self.current()?;
            if request.owner != owner || !self.policy.owners.contains(owner) {
                return Err(Error::Owner);
            }
            if !self.policy.verbs.contains(&request.operation) {
                return Err(Error::Policy);
            }
            let binding = self.grant.owner_access.as_ref().ok_or(Error::Unsupported)?;
            let milliseconds =
                u64::try_from(now.unix.timestamp_millis()).map_err(|_| Error::Expired)?;
            let op = request
                .validate(binding, milliseconds)
                .map_err(|_| Error::Policy)?
                .clone();
            if request.sequence != self.sequence.checked_add(1).ok_or(Error::Limit)?
                || self.seen.contains(&request.request_id)
            {
                return Err(Error::Replay);
            }
            let limits = self.budget.limits()?;
            if self.seen.len() >= limits.replay_entries {
                return Err(Error::Limit);
            }
            if !self.pending.is_empty() {
                return Err(Error::Busy);
            }
            let charge = self
                .budget
                .reserve(op.max_request_bytes as usize, now.monotonic)?;
            let remaining =
                Duration::from_millis(request.deadline_unix_ms - milliseconds).min(limits.deadline);
            let deadline = now
                .monotonic
                .checked_add(remaining)
                .ok_or(Error::Expired)?
                .min(self.monotonic_expiry);
            let bytes = serde_json::to_vec(&request.payload)
                .map_err(|_| Error::Policy)?
                .len();
            self.emit(
                AuditKind::Request,
                None,
                digest(request.request_id),
                &request.operation,
                request.sequence,
                bytes,
            )?;
            self.sequence = request.sequence;
            self.seen.insert(request.request_id);
            self.pending.insert(
                request.request_id,
                Pending {
                    owner: owner.into(),
                    verb: request.operation.clone(),
                    sequence: request.sequence,
                    deadline,
                    unix_deadline: DateTime::from_timestamp_millis(
                        i64::try_from(request.deadline_unix_ms).map_err(|_| Error::Expired)?,
                    )
                    .ok_or(Error::Expired)?,
                    frames: 0,
                    reply_bytes: 0,
                    _charge: charge,
                },
            );
            self.counters.admitted += 1;
            Ok(RequestHandle {
                service: self.id,
                id: request.request_id,
                sequence: request.sequence,
            })
        })();
        if let Err(reason) = result {
            self.emit(
                AuditKind::Refused,
                Some(reason),
                digest(request.request_id),
                if valid_verb(&request.operation) {
                    &request.operation
                } else {
                    "invalid"
                },
                request.sequence,
                0,
            )?;
        }
        result
    }
    fn begin_inner(
        &mut self,
        owner: &str,
        verb: &str,
        request: [u8; 32],
        bytes: usize,
    ) -> Result<RequestHandle> {
        if self.binding.profile != AgentProfile::Dev {
            return Err(Error::Profile);
        }
        self.current()?;
        if !dev_verb(verb) {
            return Err(Error::Unsupported);
        }
        if !self.plan_verbs.contains(verb)
            || !self.grant.permits(verb)
            || self.grant.tool_mediation.is_some()
        {
            return Err(Error::Policy);
        }
        if !self.policy.owners.contains(owner) {
            return Err(Error::Owner);
        }
        if !self.policy.verbs.contains(verb) {
            return Err(Error::Policy);
        }
        let limits = self.budget.limits()?;
        if request == [0; 32] || self.seen.contains(&request) {
            return Err(Error::Replay);
        }
        if self.seen.len() >= limits.replay_entries
            || bytes > 64 * 1024
            || bytes > limits.request_bytes
        {
            return Err(Error::Limit);
        }
        if !self.pending.is_empty() {
            return Err(Error::Busy);
        }
        let charge = self.budget.reserve(bytes, self.last.monotonic)?;
        let sequence = self.sequence.checked_add(1).ok_or(Error::Limit)?;
        self.emit(
            AuditKind::Request,
            None,
            digest(request),
            verb,
            sequence,
            bytes,
        )?;
        self.sequence = sequence;
        self.seen.insert(request);
        let deadline = self
            .last
            .monotonic
            .checked_add(limits.deadline)
            .ok_or(Error::Expired)?
            .min(self.monotonic_expiry);
        self.pending.insert(
            request,
            Pending {
                owner: owner.to_owned(),
                verb: verb.to_owned(),
                sequence,
                deadline,
                unix_deadline: self
                    .last
                    .unix
                    .checked_add_signed(
                        chrono::Duration::from_std(limits.deadline).map_err(|_| Error::Expired)?,
                    )
                    .ok_or(Error::Expired)?
                    .min(self.expiry),
                frames: 0,
                reply_bytes: 0,
                _charge: charge,
            },
        );
        self.counters.admitted += 1;
        Ok(RequestHandle {
            service: self.id,
            id: request,
            sequence,
        })
    }
    fn pending(&self, handle: &RequestHandle) -> Result<&Pending> {
        if handle.service != self.id {
            return Err(Error::State);
        }
        self.pending
            .get(&handle.id)
            .filter(|p| p.sequence == handle.sequence)
            .ok_or(Error::State)
    }
    fn live_pending(&mut self, handle: &RequestHandle, now: Time) -> Result<()> {
        let pending = self.pending(handle)?;
        if now.monotonic >= pending.deadline || now.unix >= pending.unix_deadline {
            let (verb, sequence) = (pending.verb.clone(), pending.sequence);
            self.emit(
                AuditKind::Cancelled,
                Some(Error::Expired),
                digest(handle.id),
                &verb,
                sequence,
                0,
            )?;
            self.pending.remove(&handle.id);
            self.counters.cancelled += 1;
            return Err(Error::Expired);
        }
        Ok(())
    }
    /// The trusted transport closure writes exactly one canonical frame. An
    /// error is terminal; no retry/reconnect or new request identity is issued.
    pub fn forward(
        &mut self,
        handle: &RequestHandle,
        owner: &str,
        current: &LifecycleBinding,
        bytes: usize,
        write: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        let verb = self
            .pending(handle)
            .map(|pending| pending.verb.clone())
            .unwrap_or_else(|_| "unknown".into());
        let result = self.forward_inner(handle, owner, current, bytes, write);
        if let Err(reason) = result {
            self.emit(
                AuditKind::Refused,
                Some(reason),
                digest(handle.id),
                &verb,
                handle.sequence,
                bytes,
            )?;
        }
        result
    }
    fn forward_inner(
        &mut self,
        handle: &RequestHandle,
        owner: &str,
        current: &LifecycleBinding,
        bytes: usize,
        write: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        let now = self.current()?;
        if current != &self.binding {
            self.close(Error::Epoch)?;
            return Err(Error::Epoch);
        }
        self.live_pending(handle, now)?;
        let p = self.pending(handle)?;
        if p.owner != owner {
            return Err(Error::Owner);
        }
        if now.monotonic >= p.deadline {
            return Err(Error::Expired);
        }
        let limits = self.budget.limits()?;
        if p.frames >= limits.frames_per_request
            || p.reply_bytes
                .checked_add(bytes)
                .is_none_or(|n| n > limits.reply_bytes)
            || bytes > 64 * 1024
        {
            return Err(Error::Limit);
        }
        let (verb, sequence) = (p.verb.clone(), p.sequence);
        self.emit(
            AuditKind::Forward,
            None,
            digest(handle.id),
            &verb,
            sequence,
            bytes,
        )?;
        if let Err(error) = write() {
            self.closed = true;
            self.counters.cancelled += self.pending.len() as u64;
            self.pending.clear();
            return Err(error);
        }
        let p = self.pending.get_mut(&handle.id).ok_or(Error::State)?;
        p.frames += 1;
        p.reply_bytes += bytes;
        self.counters.forwarded_frames += 1;
        self.counters.forwarded_bytes += bytes as u64;
        Ok(())
    }
    pub fn complete(&mut self, handle: RequestHandle) -> Result<()> {
        let now = self.current()?;
        self.live_pending(&handle, now)?;
        let p = self.pending(&handle)?;
        let (verb, seq) = (p.verb.clone(), p.sequence);
        self.emit(AuditKind::Completed, None, digest(handle.id), &verb, seq, 0)?;
        self.pending.remove(&handle.id);
        self.counters.completed += 1;
        Ok(())
    }
    pub fn cancel(&mut self, handle: RequestHandle) -> Result<()> {
        let p = self.pending(&handle)?;
        let (verb, seq) = (p.verb.clone(), p.sequence);
        self.emit(AuditKind::Cancelled, None, digest(handle.id), &verb, seq, 0)?;
        self.pending.remove(&handle.id);
        self.counters.cancelled += 1;
        Ok(())
    }
    fn close(&mut self, reason: Error) -> Result<()> {
        self.closed = true;
        self.counters.cancelled += self.pending.len() as u64;
        self.pending.clear();
        self.emit(AuditKind::Revoked, Some(reason), [0; 32], "", 0, 0)
    }
    pub fn revoke(&mut self) -> Result<()> {
        self.control.revoke()?;
        self.close(Error::Revoked)
    }
    pub fn shutdown(&mut self) -> Result<()> {
        self.closed = true;
        self.counters.cancelled += self.pending.len() as u64;
        self.pending.clear();
        self.emit(AuditKind::Shutdown, None, [0; 32], "", 0, 0)
    }
    pub fn counters(&self) -> Counters {
        self.counters
    }
}
fn digest(id: [u8; 32]) -> [u8; 32] {
    Sha256::digest(id).into()
}
fn valid_verb(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 64
        && v.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn dev_verb(v: &str) -> bool {
    matches!(
        v,
        "exec"
            | "console-open"
            | "console-attach"
            | "console-detach"
            | "console-list"
            | "console-close"
            | "console-resize"
    )
}
