//! One retained canonical Call identity, including partial local admission.
use ed25519_dalek::VerifyingKey;
use mvm_contract::protocol::mailbox::DeliveryOutcome;
use mvm_core::mailbox::{encode_envelope, transport::PreparedMailboxEnvelope};
use spiralcore_mvm_adapter::{
    receipt::{ReceiptTracker, SendHandle},
    service::{
        CallHandle, CallStatus, LocalMailboxService, MailboxBinding, ServiceError, ServiceTime,
    },
    service_sender::{self, MessageOptions},
};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum SessionError {
    Service(ServiceError),
    Busy,
    WitnessLimit,
    SendUncertain,
    StreamFenced,
    EventLimit,
    SendLimit,
    RemoteRefusal(DeliveryOutcome),
}
impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canonical session: {self:?}")
    }
}
impl std::error::Error for SessionError {}
impl From<ServiceError> for SessionError {
    fn from(error: ServiceError) -> Self {
        Self::Service(error)
    }
}
type Result<T> = std::result::Result<T, SessionError>;

/// One receiver lifetime, fixed at authenticated admission. Neither a wall
/// clock rollback nor replay can extend the original owner-monotonic bound.
pub(super) struct ExchangeDeadline {
    instant: std::time::Instant,
    expiry: i64,
    admitted_mono: u64,
    deadline_mono: u64,
}
impl ExchangeDeadline {
    pub(super) fn new(instant: std::time::Instant, expiry: i64, now: ServiceTime) -> Result<Self> {
        let remaining = expiry.checked_sub(now.unix_ms).ok_or(ServiceError::Clock)?;
        let remaining = u64::try_from(remaining)
            .ok()
            .filter(|value| *value > 0)
            .ok_or(ServiceError::Expired)?;
        let deadline_mono = now
            .monotonic_ms
            .checked_add(remaining)
            .ok_or(ServiceError::Clock)?;
        let signed_cap = std::time::Instant::now()
            .checked_add(std::time::Duration::from_millis(remaining))
            .ok_or(ServiceError::Clock)?;
        let result = Self {
            instant: instant.min(signed_cap),
            expiry,
            admitted_mono: now.monotonic_ms,
            deadline_mono,
        };
        result.check(now)?;
        Ok(result)
    }
    pub(super) fn instant(&self) -> std::time::Instant {
        self.instant
    }
    pub(super) fn check(&self, now: ServiceTime) -> Result<()> {
        if now.monotonic_ms < self.admitted_mono {
            return Err(ServiceError::Clock.into());
        }
        if std::time::Instant::now() >= self.instant
            || now.unix_ms >= self.expiry
            || now.monotonic_ms >= self.deadline_mono
        {
            return Err(ServiceError::Expired.into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ReplyAckState {
    #[default]
    None,
    Needed,
    InvokedUncertain,
    Sent,
}

/// Not Clone: only this producer owns the allocated FIFO position. Failed
/// admission does not release it or permit the caller to substitute another
/// payload. The existing service/tracker own retention and canonical authority.
pub(super) struct PendingCall {
    payload: Vec<u8>,
    options: MessageOptions,
    prepared: Option<PreparedMailboxEnvelope>,
    encoded: Option<Vec<u8>>,
    call: Option<CallHandle>,
    send: Option<SendHandle>,
    send_started: bool,
    send_invocations: u8,
    fenced: bool,
    deadline: Option<u64>,
    events_received: u8,
    receipt_outcome: Option<DeliveryOutcome>,
    reply_wire: Option<Vec<u8>>,
    reply_ack_state: ReplyAckState,
}
impl PendingCall {
    // The caller validates bounded Frame encoding before allocating options,
    // then immediately installs this value before any further fallible work.
    pub(super) fn new(payload: Vec<u8>, options: MessageOptions, now: ServiceTime) -> Self {
        let deadline = options
            .expires_at_unix_ms
            .checked_sub(now.unix_ms)
            .and_then(|remaining| u64::try_from(remaining).ok())
            .and_then(|remaining| now.monotonic_ms.checked_add(remaining));
        Self {
            payload,
            options,
            prepared: None,
            encoded: None,
            call: None,
            send: None,
            send_started: false,
            send_invocations: 0,
            fenced: deadline.is_none(),
            deadline,
            events_received: 0,
            receipt_outcome: None,
            reply_wire: None,
            reply_ack_state: ReplyAckState::None,
        }
    }
    pub(super) fn events_received(&self) -> u8 {
        self.events_received
    }
    pub(super) fn receive_event(&mut self) -> Result<()> {
        if self.events_received >= 16 {
            return Err(SessionError::EventLimit);
        }
        self.events_received += 1;
        Ok(())
    }
    pub(super) fn receipt_outcome(&self) -> Option<DeliveryOutcome> {
        self.receipt_outcome
    }
    pub(super) fn observe_receipt(&mut self, outcome: DeliveryOutcome) -> Result<()> {
        if !matches!(
            outcome,
            DeliveryOutcome::AcceptedEphemeral
                | DeliveryOutcome::AcceptedDurable
                | DeliveryOutcome::Duplicate
        ) {
            self.receipt_outcome = Some(outcome);
            return Err(SessionError::RemoteRefusal(outcome));
        }
        self.receipt_outcome = Some(match (self.receipt_outcome, outcome) {
            (Some(DeliveryOutcome::AcceptedDurable), _) => DeliveryOutcome::AcceptedDurable,
            (Some(DeliveryOutcome::AcceptedEphemeral), DeliveryOutcome::Duplicate) => {
                DeliveryOutcome::AcceptedEphemeral
            }
            _ => outcome,
        });
        Ok(())
    }
    pub(super) fn reply_wire(&self) -> Option<&[u8]> {
        self.reply_wire.as_deref()
    }
    pub(super) fn reply_observed(&self) -> bool {
        self.reply_wire().is_some()
    }
    /// Only the authenticated handle returned by the canonical tracker selects
    /// current state. The service retains the charged plaintext Reply payload.
    pub(super) fn observe_reply(&mut self, observed: SendHandle, wire: &[u8]) -> Result<bool> {
        if Some(observed) != self.send {
            return Ok(false);
        }
        if wire.is_empty() || wire.len() > spiralcore_mvm_adapter::MAX_ENVELOPE_BYTES {
            return Err(ServiceError::Limit.into());
        }
        self.reply_wire = Some(wire.to_vec());
        self.reply_ack_state = ReplyAckState::Needed;
        Ok(true)
    }
    pub(super) fn reply_ack_state(&self) -> ReplyAckState {
        self.reply_ack_state
    }
    pub(super) fn invoke_reply_ack(&mut self) -> Result<()> {
        if self.reply_ack_state != ReplyAckState::Needed {
            return Err(ServiceError::State.into());
        }
        self.reply_ack_state = ReplyAckState::InvokedUncertain;
        Ok(())
    }
    pub(super) fn sent_reply_ack(&mut self) -> Result<()> {
        if self.reply_ack_state != ReplyAckState::InvokedUncertain {
            return Err(ServiceError::State.into());
        }
        self.reply_ack_state = ReplyAckState::Sent;
        Ok(())
    }
    pub(super) fn ready_to_complete(&self) -> bool {
        matches!(
            self.receipt_outcome(),
            Some(
                DeliveryOutcome::AcceptedEphemeral
                    | DeliveryOutcome::AcceptedDurable
                    | DeliveryOutcome::Duplicate
            )
        ) && self.reply_observed()
            && self.reply_ack_state() == ReplyAckState::Sent
    }
    pub(super) fn matches(&self, payload: &[u8]) -> bool {
        self.payload == payload
    }
    pub(super) fn expires_at(&self) -> i64 {
        self.options.expires_at_unix_ms
    }
    pub(super) fn remaining(&self, now: ServiceTime) -> Result<u64> {
        let monotonic = self
            .deadline
            .ok_or(SessionError::StreamFenced)?
            .checked_sub(now.monotonic_ms)
            .filter(|remaining| *remaining > 0)
            .ok_or(ServiceError::Expired)?;
        let unix = self
            .expires_at()
            .checked_sub(now.unix_ms)
            .ok_or(ServiceError::Clock)?;
        let unix = u64::try_from(unix)
            .ok()
            .filter(|remaining| *remaining > 0)
            .ok_or(ServiceError::Expired)?;
        Ok(monotonic.min(unix))
    }
    pub(super) fn call_handle(&self) -> Option<CallHandle> {
        self.call
    }
    pub(super) fn send_handle(&self) -> Option<SendHandle> {
        self.send
    }
    pub(super) fn send_invocations(&self) -> u8 {
        self.send_invocations
    }
    #[cfg(test)]
    pub(super) fn send_started(&self) -> bool {
        self.send_started
    }
    pub(super) fn prepared(&self) -> Result<&PreparedMailboxEnvelope> {
        self.prepared.as_ref().ok_or(ServiceError::State.into())
    }
    pub(super) fn encoded(&self) -> Result<&[u8]> {
        self.encoded.as_deref().ok_or(ServiceError::State.into())
    }
    pub(super) fn register(
        &mut self,
        service: &mut LocalMailboxService,
        tracker: &mut ReceiptTracker,
        outgoing: &MailboxBinding,
        control: &MailboxBinding,
        peer: &VerifyingKey,
        now: ServiceTime,
    ) -> Result<()> {
        if self.send_started || self.fenced {
            // A retained terminal/uncertain operation must still maintain
            // resource expiry without forgetting its unresolved FIFO position.
            service.expire(now)?;
            tracker.expire(now)?;
            if self.fenced || self.remaining(now).is_err() {
                self.fenced = true;
                return Err(SessionError::StreamFenced);
            }
            return Err(SessionError::SendUncertain);
        }
        let result = self.register_inner(service, tracker, outgoing, control, peer, now);
        if let Err(error) = &result
            && !matches!(
                error,
                SessionError::Service(ServiceError::Full | ServiceError::Backpressured)
            )
        {
            self.fenced = true;
        }
        result
    }
    fn register_inner(
        &mut self,
        service: &mut LocalMailboxService,
        tracker: &mut ReceiptTracker,
        outgoing: &MailboxBinding,
        control: &MailboxBinding,
        peer: &VerifyingKey,
        now: ServiceTime,
    ) -> Result<()> {
        tracker.expire(now)?;
        if let Err(error) = self.remaining(now) {
            service.expire(now)?;
            return Err(error);
        }
        if self.prepared.is_none() {
            self.prepared = Some(service_sender::prepare(
                outgoing,
                &self.payload,
                &self.options,
                now,
            )?);
        }
        if self.encoded.is_none() {
            let bytes = encode_envelope(self.prepared()?.envelope())
                .map_err(|_| ServiceError::Malformed)?;
            if bytes.len() > spiralcore_mvm_adapter::MAX_ENVELOPE_BYTES {
                return Err(ServiceError::Limit.into());
            }
            self.encoded = Some(bytes);
        }
        if let Some(handle) = self.call {
            // Revalidate its current generation and deadline without allocating
            // a second Call or terminalizing the original partial transaction.
            match service.poll_call(outgoing, &handle, now)? {
                CallStatus::Pending => {}
                CallStatus::TimedOut => return Err(ServiceError::Expired.into()),
                CallStatus::Cancelled => return Err(ServiceError::Cancelled.into()),
                CallStatus::Unavailable => return Err(ServiceError::Unavailable.into()),
                CallStatus::ShuttingDown => return Err(ServiceError::ShuttingDown.into()),
                CallStatus::Replied(_) => return Err(ServiceError::Conflict.into()),
            }
        } else {
            self.call = Some(service.register_call(outgoing, self.encoded()?, now)?);
        }
        let send = tracker.register_send(outgoing, control, peer, self.encoded()?, now)?;
        if self.send.is_some_and(|old| old != send) {
            return Err(ServiceError::Conflict.into());
        }
        self.send = Some(send);
        // Policy replacement is independently synchronized by the binding.
        // Reconcile a retained Call after tracker registration, retaining the
        // new tracker handle even when this check fences the transaction.
        if !matches!(
            service.poll_call(outgoing, &self.call.ok_or(ServiceError::State)?, now)?,
            CallStatus::Pending
        ) {
            return Err(ServiceError::Conflict.into());
        }
        Ok(())
    }
    /// Mark uncertainty before entering transport. C7 never implicitly retries
    /// a possibly written request; two-peer event/retry reconciliation is C8.
    pub(super) fn start_send(&mut self) -> Result<()> {
        if self.fenced {
            return Err(SessionError::StreamFenced);
        }
        if self.send_started {
            return Err(SessionError::SendUncertain);
        }
        if self.call.is_none() || self.send.is_none() {
            return Err(ServiceError::State.into());
        }
        self.send_started = true;
        self.send_invocations = 1;
        Ok(())
    }

    fn check_retry(&self) -> Result<()> {
        if self.fenced {
            return Err(SessionError::StreamFenced);
        }
        if !self.send_started || self.call.is_none() || self.send.is_none() {
            return Err(ServiceError::State.into());
        }
        if self.send_invocations() >= 8 {
            return Err(SessionError::SendLimit);
        }
        Ok(())
    }

    /// Revalidate the retained attempt without polling away its charged Reply.
    /// The caller separately decides whether current facts require a resend.
    pub(super) fn prepare_retry(
        &mut self,
        service: &mut LocalMailboxService,
        tracker: &mut ReceiptTracker,
        outgoing: &MailboxBinding,
        control: &MailboxBinding,
        peer: &VerifyingKey,
        now: ServiceTime,
    ) -> Result<()> {
        service.expire(now)?;
        tracker.expire(now)?;
        self.check_retry()?;
        self.remaining(now)?;
        let observed = tracker.register_send(outgoing, control, peer, self.encoded()?, now)?;
        if Some(observed) != self.send {
            return Err(ServiceError::Conflict.into());
        }
        Ok(())
    }

    /// Burn an invocation before entering transport, including failed writes.
    pub(super) fn start_retry(&mut self) -> Result<()> {
        self.check_retry()?;
        self.send_invocations = self
            .send_invocations
            .checked_add(1)
            .ok_or(SessionError::SendLimit)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
