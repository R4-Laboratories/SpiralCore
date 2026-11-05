//! Charged ownership transfer for an explicitly retained, completed Reply wire.
use super::*;
use sha2::{Digest, Sha256};

/// One application result plus charged exact ciphertext custody. No send permit.
pub struct CompletedCallReply {
    pub payload: Vec<u8>,
    pub tail: CompletedReplyTail,
}

/// Non-Clone ownership of the exact authenticated Reply and its shared charge.
/// Expired or fenced bytes remain charged until this value is dropped.
pub struct CompletedReplyTail {
    wire: Vec<u8>,
    call: CallHandle,
    replier: u64,
    reply_generation: u64,
    call_deadline: u64,
    reply_deadline: u64,
    call_expiry: i64,
    reply_expiry: i64,
    created: ServiceTime,
    owner: Arc<Mutex<MailboxOwner>>,
    owner_epoch: ExecutionEpoch,
    live: Arc<Mutex<bool>>,
    // Bytes drop before their reservation is refunded.
    _reservation: Reservation,
}
impl CompletedReplyTail {
    /// Borrow retained ciphertext. This does not authorize sending or admission.
    pub fn wire(&self) -> &[u8] {
        &self.wire
    }
    pub fn call(&self) -> &CallHandle {
        &self.call
    }
    /// Fresh authority/lifetime observation; it never extends or releases custody.
    pub fn validate(
        &self,
        caller: &MailboxBinding,
        replier: &MailboxBinding,
        now: ServiceTime,
    ) -> Result<()> {
        if now.unix_ms < 0 || now.monotonic_ms < self.created.monotonic_ms {
            return Err(ServiceError::Clock);
        }
        if !*self.live.lock().map_err(|_| ServiceError::State)? {
            return Err(ServiceError::ShuttingDown);
        }
        let (_, call_generation) = caller.checked(now)?;
        let (_, reply_generation) = replier.checked(now)?;
        let policies = policy_pair(caller.id, &caller.policy, replier.id, &replier.policy)?;
        if caller.id != self.call.binding
            || replier.id != self.replier
            || call_generation != self.call.generation
            || reply_generation != self.reply_generation
            || policies.a_generation != call_generation
            || policies.b_generation != reply_generation
        {
            return Err(ServiceError::Unauthorized);
        }
        if now.monotonic_ms >= self.call_deadline
            || now.monotonic_ms >= self.reply_deadline
            || now.unix_ms >= self.call_expiry
            || now.unix_ms >= self.reply_expiry
        {
            return Err(ServiceError::Expired);
        }
        let owner = self.owner.lock().map_err(|_| ServiceError::State)?;
        if owner.epoch != self.owner_epoch || owner.service != self.call.service {
            return Err(ServiceError::Unauthorized);
        }
        Ok(())
    }
}

impl LocalMailboxService {
    /// Reserve conservative plaintext/ciphertext overlap before moving either.
    /// Full never consumes the result or the caller's wire. Strict poll is unchanged.
    pub fn take_call_reply_with_tail(
        &mut self,
        caller: &MailboxBinding,
        replier: &MailboxBinding,
        call: &CallHandle,
        wire: &mut Option<Vec<u8>>,
        now: ServiceTime,
    ) -> Result<CompletedCallReply> {
        self.clock(now)?;
        self.expire(now)?;
        self.drain_ready_replies(now)?;
        if self.shutting_down {
            return Err(ServiceError::ShuttingDown);
        }
        let (_, call_generation) = caller.checked(now)?;
        let (_, reply_generation) = replier.checked(now)?;
        let policies = policy_pair(caller.id, &caller.policy, replier.id, &replier.policy)?;
        let pending = self
            .calls
            .get(&call.id)
            .ok_or(ServiceError::UnknownHandle)?;
        if call.service != self.id
            || call.binding != caller.id
            || pending.handle != *call
            || call.generation != call_generation
            || policies.a_generation != call_generation
            || policies.b_generation != reply_generation
        {
            return Err(ServiceError::Unauthorized);
        }
        let payload = match &pending.status {
            CallStatus::Pending => return Err(ServiceError::Backpressured),
            CallStatus::Replied(Some(payload)) => payload,
            CallStatus::Replied(None) => return Err(ServiceError::Conflict),
            CallStatus::Cancelled => return Err(ServiceError::Cancelled),
            CallStatus::TimedOut => return Err(ServiceError::Expired),
            CallStatus::Unavailable => return Err(ServiceError::Unavailable),
            CallStatus::ShuttingDown => return Err(ServiceError::ShuttingDown),
        };
        let entry = self
            .accepted
            .get(&pending.reply_key.ok_or(ServiceError::State)?)
            .ok_or(ServiceError::State)?;
        if entry.handle.binding != replier.id
            || entry.handle.generation != reply_generation
            || entry.header.kind != MailboxMessageKind::Reply
        {
            return Err(ServiceError::Unauthorized);
        }
        if now.monotonic_ms >= pending.deadline
            || now.monotonic_ms >= entry.deadline
            || now.unix_ms >= pending.header.expires_at_unix_ms
            || now.unix_ms >= entry.header.expires_at_unix_ms
        {
            return Err(ServiceError::Expired);
        }
        let bytes = wire.as_ref().ok_or(ServiceError::State)?;
        if bytes.len() > self.budget.limits.max_envelope_bytes {
            return Err(ServiceError::Limit);
        }
        let identity = entry.reply_wire.ok_or(ServiceError::State)?;
        if bytes.len() != identity.len || <[u8; 32]>::from(Sha256::digest(bytes)) != identity.digest
        {
            return Err(ServiceError::Unauthorized);
        }
        let old = pending.reply_budget.as_ref().ok_or(ServiceError::State)?;
        if payload.len() > bytes.len()
            || old.delta.bytes != bytes.len() as u64
            || old.delta.messages != 1
            || old.class != BudgetClass::Data
            || !old.budget.same_owner(&self.budget)
            || old.grant != Some(GrantScope::new(old.scope, &replier.config.capability.grant))
        {
            return Err(ServiceError::State);
        }
        let owner_state = Arc::clone(&entry.owner);
        let owner = owner_state.lock().map_err(|_| ServiceError::State)?;
        if owner.epoch != entry.header.execution_epoch || owner.service != self.id {
            return Err(ServiceError::Unauthorized);
        }
        let reservation = self.budget.reserve(
            old.scope,
            Usage {
                bytes: u64::try_from(bytes.capacity()).map_err(|_| ServiceError::Limit)?,
                messages: 1,
                ..Usage::default()
            },
            &replier.config.capability.grant,
        )?;
        // Every fallible validation/reservation precedes these exclusive moves.
        let mut tail = CompletedReplyTail {
            wire: Vec::new(),
            call: *call,
            replier: replier.id,
            reply_generation,
            call_deadline: pending.deadline,
            reply_deadline: entry.deadline,
            call_expiry: pending.header.expires_at_unix_ms,
            reply_expiry: entry.header.expires_at_unix_ms,
            created: now,
            owner: Arc::clone(&owner_state),
            owner_epoch: entry.header.execution_epoch,
            live: Arc::clone(&self.live),
            _reservation: reservation,
        };
        let pending = self
            .calls
            .get_mut(&call.id)
            .expect("validated Call remains owned");
        let CallStatus::Replied(payload) = &mut pending.status else {
            unreachable!("validated Reply remains owned")
        };
        let payload = payload
            .take()
            .expect("validated Reply payload remains owned");
        tail.wire = wire.take().expect("validated wire remains owned");
        pending.reply_budget.take();
        Ok(CompletedCallReply { payload, tail })
    }
}
