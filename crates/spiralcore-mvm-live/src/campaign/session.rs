//! One retained canonical Call identity, including partial local admission.
use ed25519_dalek::VerifyingKey;
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
    fenced: bool,
    deadline: Option<u64>,
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
            fenced: deadline.is_none(),
            deadline,
        }
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
        Ok(())
    }
}

#[cfg(test)]
mod tests;
