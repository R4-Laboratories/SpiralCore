//! Canonical mailbox service consumer shared by host and real guest campaigns.
use super::reconciliation::{
    ProcessingPhase, ProcessingTotals, RecoveryCounters, ReplyReplayRequests, RetryRequests,
    increment,
};
use super::session::{CompletedTail, ExchangeDeadline, PendingCall, ReplyAckState, SessionError};
use super::*;
use crate::config::Material;
use ed25519_dalek::{SigningKey, VerifyingKey};
use mvm_contract::protocol::mailbox::{DeliveryOutcome, ReplyRoute};
use mvm_core::mailbox::{decode_envelope, encode_envelope};
use spiralcore_mvm_adapter::{
    broker::BrokerContext,
    issuance::IssuancePolicy,
    receipt::{ReceiptObservation, ReceiptReturnConfig, ReceiptTracker},
    service::{
        CallStatus, ControlReserve, HostBudget, LocalMailboxService, MailboxBinding, ServiceError,
        ServiceLimits, ServiceTime,
    },
    service_object::ObjectMailbox,
    service_sender::{self, OrderingStream},
};

pub(super) struct Session {
    incoming: MailboxBinding,
    outgoing: MailboxBinding,
    control: MailboxBinding,
    returns: ReceiptReturnConfig,
    service: LocalMailboxService,
    tracker: ReceiptTracker,
    peer_key: VerifyingKey,
    normal_key: [u8; 32],
    control_key: [u8; 32],
    reply_to: Option<ReplyRoute>,
    broker: BrokerContext,
    ordering: OrderingStream,
    owner_capacity: Value,
    pub(super) calls: u64,
    context: Option<Context>,
    pending: Option<PendingCall>,
    completed_tail: Option<CompletedTail>,
    caller_fenced: bool,
    receiver_incomplete: bool,
    recovery: RecoveryCounters,
    receiver_processing: ProcessingTotals,
    receiver_phase: Option<ProcessingPhase>,
}
const MAX_CAMPAIGN_CALLS: u64 = 2048;
#[derive(Clone, Copy, PartialEq, Eq)]
enum AckTarget {
    Current,
    Tail,
    Historical,
}
const ISSUANCE_POLICY: IssuancePolicy = IssuancePolicy::Bounded {
    max_clock_ahead_ms: 5000,
};
fn campaign_limits() -> (ServiceLimits, ControlReserve) {
    // Native witness tier, separate from the reusable service's defaults.
    // 2048 Calls retain <=4096 control identities in any sender scope and
    // <=6144 aggregate (tracking + both receipt-rate histories).
    let mut limits = ServiceLimits::default();
    limits.host.identities = 65536;
    limits.tenant.identities = 32768;
    limits.mailbox.identities = 32768;
    limits.sender.identities = 16384;
    let mut reserve = ControlReserve::default();
    reserve.host.identities = 16384;
    reserve.tenant.identities = 8192;
    reserve.mailbox.identities = 8192;
    reserve.sender.identities = 4096;
    (limits, reserve)
}
fn campaign_budget() -> Result<HostBudget> {
    let (limits, reserve) = campaign_limits();
    Ok(HostBudget::with_control(limits, reserve)?)
}

pub(super) fn broker_context() -> Result<BrokerContext> {
    Ok(BrokerContext::new(campaign_budget()?))
}

// Application work uses the private admitted proof and endpoint-local limits.
// It must not mutate the aggregate service/tracker clock using this timestamp.
fn application_phase<T>(
    broker: &BrokerContext,
    work: impl FnOnce(ServiceTime) -> Result<T>,
) -> Result<T> {
    let now =
        broker.with_time::<_, spiralcore_mvm_adapter::service::ServiceError>(|_, now| Ok(now))?;
    work(now)
}

impl Session {
    pub(super) fn new(bootstrap: &Bootstrap, broker: &BrokerContext) -> Result<Self> {
        let profile = bootstrap
            .service
            .as_ref()
            .ok_or("missing service profile")?;
        if profile.profile != "canonical-ephemeral-v1" {
            return Err("unknown mailbox service profile".into());
        }
        let incoming = bootstrap
            .incoming
            .as_ref()
            .ok_or("missing incoming material")?;
        let outgoing = bootstrap
            .outgoing
            .as_ref()
            .ok_or("missing outgoing material")?;
        validate_materials(incoming, outgoing, profile)?;

        let local = SigningKey::from_bytes(&profile.local_receipt_signing);
        let peer_key = VerifyingKey::from_bytes(&profile.peer_receipt_verification)?;
        broker.with_time(|budget, _| -> Result<Self> {
            let snapshot = budget.snapshot()?;
            let owner_capacity = json!({
                "identity_limits": {
                    "host": snapshot.limits.host.identities,
                    "tenant": snapshot.limits.tenant.identities,
                    "mailbox": snapshot.limits.mailbox.identities,
                    "sender": snapshot.limits.sender.identities,
                },
                "control_identity_limits": snapshot.control.map(|control| json!({
                    "host": control.host.identities,
                    "tenant": control.tenant.identities,
                    "mailbox": control.mailbox.identities,
                    "sender": control.sender.identities,
                })),
            });
            Ok(Self {
                incoming: incoming.service_binding(Some(profile.local_receipt_signing))?,
                outgoing: outgoing.service_binding(None)?,
                control: profile.incoming_control.service_binding(None)?,
                returns: ReceiptReturnConfig::new_with_policy(
                    profile.outgoing_control.service_binding(None)?,
                    budget.clone(),
                    local.verifying_key(),
                    ISSUANCE_POLICY,
                )?,
                service: LocalMailboxService::new(budget.clone())?,
                tracker: ReceiptTracker::new(budget.clone(), 4096)?,
                peer_key,
                normal_key: incoming.key_id,
                control_key: profile.incoming_control.key_id,
                reply_to: outgoing.reply_to,
                broker: broker.clone(),
                ordering: OrderingStream::new(),
                owner_capacity,
                calls: 0,
                context: None,
                pending: None,
                completed_tail: None,
                caller_fenced: false,
                receiver_incomplete: false,
                recovery: RecoveryCounters::default(),
                receiver_processing: ProcessingTotals::default(),
                receiver_phase: None,
            })
        })
    }
    pub(super) fn report(&self) -> Value {
        json!({"session_report":"canonical-session-report/v2","application_calls":self.calls,
            "normal_envelopes_per_call":2,"normal_encrypted_destination_receipts_per_call":2,
            "normal_frames_per_call":4,"recovery":&self.recovery,
            "receiver_processing":&self.receiver_processing,
            "receiver_processing_domain":"post-frame process CPU and wall; includes unfinished work; excludes receive waits; co-located CPU intervals overlap",
            "profile":"canonical-ephemeral-v1","durability":"AcceptedEphemeral; object commit separately verified durable",
            "identity_tier":"broker-owned","witness_tier":"native-16m-v1",
            "identity_limits":self.owner_capacity["identity_limits"],"control_identity_limits":self.owner_capacity["control_identity_limits"],
            "max_application_calls":MAX_CAMPAIGN_CALLS,"pipeline":1,"ordering":"stable-scope-increasing-sequence",
            "broker_clock":"shared-serialized-dispatch","issuance_policy":{"mode":"bounded","max_clock_ahead_ms":5000,"max_issued_ttl_ms":30000,"max_remaining_ttl_ms":25000}})
    }
    pub(super) fn call<S: SocketIo + 'static>(
        &mut self,
        request: &Frame,
        stream: &StreamTransport<S>,
        cancel: &CancellationToken,
    ) -> Result<(Context, Frame)> {
        self.call_inner(request, stream, cancel, None, false)
    }

    pub(super) fn call_with_reconciliation<S: SocketIo + 'static>(
        &mut self,
        request: &Frame,
        stream: &StreamTransport<S>,
        cancel: &CancellationToken,
        retry_requests: RetryRequests,
    ) -> Result<(Context, Frame)> {
        self.call_inner(request, stream, cancel, Some(retry_requests), false)
    }

    pub(super) fn call_with_completed_tail<S: SocketIo + 'static>(
        &mut self,
        request: &Frame,
        stream: &StreamTransport<S>,
        cancel: &CancellationToken,
        retry_requests: Option<RetryRequests>,
    ) -> Result<(Context, Frame)> {
        let result = self.call_inner(request, stream, cancel, retry_requests, true);
        if result.is_err() {
            self.caller_fenced = true;
        }
        result
    }

    fn call_inner<S: SocketIo + 'static>(
        &mut self,
        request: &Frame,
        stream: &StreamTransport<S>,
        cancel: &CancellationToken,
        retry_requests: Option<RetryRequests>,
        retain_tail: bool,
    ) -> Result<(Context, Frame)> {
        if self.caller_fenced {
            return Err(SessionError::StreamFenced.into());
        }
        if self.calls >= MAX_CAMPAIGN_CALLS {
            return Err(SessionError::WitnessLimit.into());
        }
        if let Some(tail) = &self.completed_tail {
            let result = self.broker.with_time(|_, now| -> Result<()> {
                tail.lease.validate(&self.outgoing, &self.incoming, now)?;
                if Instant::now() >= tail.deadline || tail.ack != ReplyAckState::Sent {
                    return Err(SessionError::StreamFenced.into());
                }
                if tail.events >= 16 {
                    return Err(SessionError::EventLimit.into());
                }
                Ok(())
            });
            if let Err(error) = result {
                self.caller_fenced = true;
                return Err(error);
            }
        }
        // Validate bytes before consuming a FIFO position. Once allocated, the
        // pending identity survives every fallible registration/send/receive.
        let payload = request.encode()?;
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| !pending.matches(&payload))
        {
            self.broker
                .with_time::<_, spiralcore_mvm_adapter::service::ServiceError>(|_, now| {
                    self.service.expire(now)?;
                    self.tracker.expire(now)?;
                    Ok(())
                })?;
            return Err(SessionError::Busy.into());
        }
        let deadline = Instant::now() + CALL_TIMEOUT;
        let broker = self.broker.clone();
        let (context, call, send, deadline, send_time) =
            broker.with_time(|_, now| -> Result<_> {
                if self.pending.is_none() {
                    let expiry = now.unix_ms.checked_add(30_000).ok_or("clock overflow")?;
                    let mut options = self.ordering.next_for_binding(
                        &self.outgoing,
                        mvm_contract::protocol::mailbox::ContentType::parse(
                            spiralcore_mvm_adapter::CONTENT_TYPE,
                            1,
                        )?,
                        now,
                        expiry,
                        ISSUANCE_POLICY,
                    )?;
                    options.reply_to = self.reply_to;
                    self.pending = Some(PendingCall::new(payload, options, now));
                }
                let pending = self.pending.as_mut().ok_or(SessionError::StreamFenced)?;
                pending.register(
                    &mut self.service,
                    &mut self.tracker,
                    &self.outgoing,
                    &self.control,
                    &self.peer_key,
                    now,
                )?;
                let context = match &self.context {
                    Some(context) => context.clone(),
                    None => {
                        let context = service_sender::object_context(
                            &self.outgoing,
                            pending.prepared()?,
                            now,
                        )?;
                        self.context = Some(context.clone());
                        context
                    }
                };
                let remaining = pending.remaining(now)?;
                let deadline = deadline.min(Instant::now() + Duration::from_millis(remaining));
                let call = pending.call_handle().ok_or(SessionError::StreamFenced)?;
                let send = pending.send_handle().ok_or(SessionError::StreamFenced)?;
                pending.start_send()?;
                Ok((context, call, send, deadline, now))
            })?;
        // No broker lease spans potentially blocking transport I/O.
        self.service.send_call_prepared(
            &self.outgoing,
            &call,
            self.pending
                .as_ref()
                .ok_or(SessionError::StreamFenced)?
                .prepared()?,
            &stream.before(deadline),
            cancel,
            send_time,
        )?;
        let mut reply = None;
        while self
            .pending
            .as_ref()
            .ok_or(SessionError::StreamFenced)?
            .events_received()
            < 16
        {
            // The old tail and current Call share each complete incoming
            // event until authenticated progress retires the tail. Refuse
            // before reading beyond either original allowance.
            if self
                .completed_tail
                .as_ref()
                .is_some_and(|tail| tail.events >= 16)
            {
                return Err(SessionError::EventLimit.into());
            }
            let bytes = if let Some(requests) = &retry_requests {
                match stream.receive_or_idle(cancel, deadline, Duration::from_millis(250))? {
                    spiralcore_mvm_adapter::live::ReceiveOutcome::Frame(bytes) => bytes,
                    spiralcore_mvm_adapter::live::ReceiveOutcome::Idle => {
                        increment(&mut self.recovery.idle_outcomes)?;
                        let retry_time = broker.with_time(|_, now| -> Result<_> {
                            let pending =
                                self.pending.as_ref().ok_or(SessionError::StreamFenced)?;
                            if let Err(error) = pending.remaining(now) {
                                self.service.expire(now)?;
                                self.tracker.expire(now)?;
                                return Err(error.into());
                            }
                            if Instant::now() >= deadline {
                                return Err(SessionError::Service(ServiceError::Expired).into());
                            }
                            if pending.ready_to_complete() {
                                reply = self.poll_ready_reply(call, now, retain_tail, deadline)?;
                                return Ok(None);
                            }
                            if !requests.take_request() {
                                return Ok(None);
                            }
                            increment(&mut self.recovery.commands_consumed)?;
                            let pending =
                                self.pending.as_mut().ok_or(SessionError::StreamFenced)?;
                            pending.prepare_retry(
                                &mut self.service,
                                &mut self.tracker,
                                &self.outgoing,
                                &self.control,
                                &self.peer_key,
                                now,
                            )?;
                            Ok(Some(now))
                        })?;
                        if reply.is_some() {
                            break;
                        }
                        if retry_time.is_some() {
                            // Expiry maintenance/registration can consume time;
                            // sample again before burning the transport attempt.
                            let now = broker.with_time(|_, now| -> Result<_> {
                                self.pending
                                    .as_ref()
                                    .ok_or(SessionError::StreamFenced)?
                                    .remaining(now)?;
                                if Instant::now() >= deadline {
                                    return Err(SessionError::Service(ServiceError::Expired).into());
                                }
                                Ok(now)
                            })?;
                            self.pending
                                .as_mut()
                                .ok_or(SessionError::StreamFenced)?
                                .start_retry()?;
                            increment(&mut self.recovery.call_retry_invocations)?;
                            self.service.retry_call_prepared(
                                &self.outgoing,
                                &call,
                                self.pending
                                    .as_ref()
                                    .ok_or(SessionError::StreamFenced)?
                                    .prepared()?,
                                &stream.before(deadline),
                                cancel,
                                now,
                            )?;
                        }
                        continue;
                    }
                }
            } else {
                stream.receive(cancel, deadline)?
            };
            self.pending
                .as_mut()
                .ok_or(SessionError::StreamFenced)?
                .receive_event()?;
            if let Some(tail) = self.completed_tail.as_mut() {
                tail.receive_event()?;
                increment(&mut self.recovery.tail_events)?;
            }
            let mut owned_wire = Some(bytes);
            let envelope = decode_envelope(owned_wire.as_deref().ok_or(ServiceError::State)?)?;
            let acknowledgment = broker.with_time(|_, now| -> Result<_> {
                let bytes = owned_wire.as_deref().ok_or(ServiceError::State)?;
                if envelope.encryption.key_id == self.control_key {
                    self.tracker.observe_receipt(&self.control, bytes, now)?;
                    let receipt = match self.tracker.take_receipt(&self.outgoing, &send, now)? {
                        ReceiptObservation::Verified(receipt) => receipt,
                        // An authenticated duplicate or earlier retained
                        // observation need not provide a new current receipt.
                        ReceiptObservation::Pending => return Ok(None),
                        ReceiptObservation::Expired => {
                            return Err(SessionError::Service(ServiceError::Expired).into());
                        }
                    };
                    self.pending
                        .as_mut()
                        .ok_or(SessionError::StreamFenced)?
                        .observe_receipt(receipt.receipt.outcome)?;
                    if matches!(
                        receipt.receipt.outcome,
                        DeliveryOutcome::AcceptedEphemeral | DeliveryOutcome::AcceptedDurable
                    ) {
                        self.completed_tail.take();
                    }
                    Ok(None)
                } else if envelope.encryption.key_id == self.normal_key {
                    let tail_event = self
                        .completed_tail
                        .as_ref()
                        .is_some_and(|tail| tail.lease.wire() == bytes);
                    if tail_event {
                        let tail = self.completed_tail.as_ref().ok_or(ServiceError::State)?;
                        tail.lease.validate(&self.outgoing, &self.incoming, now)?;
                        if Instant::now() >= tail.deadline {
                            return Err(ServiceError::Expired.into());
                        }
                    }
                    let (accepted, ack) =
                        self.returns
                            .admit(&mut self.service, &self.incoming, bytes, now)?;
                    if tail_event {
                        let tail = self.completed_tail.as_mut().ok_or(ServiceError::State)?;
                        if accepted.receipt.outcome != DeliveryOutcome::Duplicate
                            || self.tracker.observe_registered_application_reply(
                                &self.incoming,
                                bytes,
                                now,
                            )? != Some(tail.send)
                        {
                            return Err(ServiceError::Unauthorized.into());
                        }
                        tail.ack = ReplyAckState::Needed;
                        return Ok(Some((ack, AckTarget::Tail)));
                    }
                    let current = match accepted.receipt.outcome {
                        // This authenticated negative disposition contributes
                        // no current facts, but still gets its actual receipt.
                        DeliveryOutcome::Late => false,
                        DeliveryOutcome::AcceptedEphemeral | DeliveryOutcome::Duplicate => {
                            match self.tracker.observe_registered_application_reply(
                                &self.incoming,
                                bytes,
                                now,
                            )? {
                                Some(observed) => {
                                    if self
                                        .completed_tail
                                        .as_ref()
                                        .is_some_and(|tail| tail.send == observed)
                                    {
                                        return Err(ServiceError::Conflict.into());
                                    }
                                    let pending =
                                        self.pending.as_mut().ok_or(SessionError::StreamFenced)?;
                                    let current = if retain_tail {
                                        pending.observe_reply_owned(observed, &mut owned_wire)?
                                    } else {
                                        pending.observe_reply(observed, bytes)?
                                    };
                                    if current {
                                        self.completed_tail.take();
                                    }
                                    current
                                }
                                // A previously Late identity can replay as
                                // Duplicate without a registered request.
                                None if accepted.receipt.outcome == DeliveryOutcome::Duplicate => {
                                    false
                                }
                                None => {
                                    return Err(
                                        SessionError::Service(ServiceError::Unauthorized).into()
                                    );
                                }
                            }
                        }
                        _ => return Err("Reply admission refused".into()),
                    };
                    Ok(Some((
                        ack,
                        if current {
                            AckTarget::Current
                        } else {
                            AckTarget::Historical
                        },
                    )))
                } else {
                    Err("unprovisioned incoming encryption key".into())
                }
            })?;
            if let Some((ack, target)) = acknowledgment {
                let (permit, now) = broker.with_time(|_, now| -> Result<_> {
                    if target == AckTarget::Tail {
                        self.completed_tail
                            .as_ref()
                            .ok_or(ServiceError::State)?
                            .lease
                            .validate(&self.outgoing, &self.incoming, now)?;
                    }
                    Ok((ack.reserve_send(cancel, now)?, now))
                })?;
                if target == AckTarget::Current {
                    self.pending
                        .as_mut()
                        .ok_or(SessionError::StreamFenced)?
                        .invoke_reply_ack()?;
                }
                let ack_deadline = if target == AckTarget::Tail {
                    let tail = self.completed_tail.as_mut().ok_or(ServiceError::State)?;
                    tail.invoke_ack()?;
                    increment(&mut self.recovery.tail_ack_invocations)?;
                    tail.deadline.min(deadline)
                } else {
                    deadline
                };
                permit.send(&stream.before(ack_deadline), cancel, now)?;
                if target == AckTarget::Current {
                    self.pending
                        .as_mut()
                        .ok_or(SessionError::StreamFenced)?
                        .sent_reply_ack()?;
                } else if target == AckTarget::Tail {
                    self.completed_tail.as_mut().ok_or(ServiceError::State)?.ack =
                        ReplyAckState::Sent;
                    increment(&mut self.recovery.tail_acks_written)?;
                }
            }
            if self
                .pending
                .as_ref()
                .ok_or(SessionError::StreamFenced)?
                .ready_to_complete()
            {
                // Observation readiness can precede FIFO delivery. Polling
                // Pending consumes nothing; keep receiving the missing event.
                reply = broker
                    .with_time(|_, now| self.poll_ready_reply(call, now, retain_tail, deadline))?;
                if reply.is_some() {
                    break;
                }
            }
        }
        let reply = reply.ok_or(SessionError::EventLimit)?;
        self.calls = self.calls.checked_add(1).ok_or("Call counter overflow")?;
        self.pending = None;
        Ok((context, reply))
    }
    fn poll_ready_reply(
        &mut self,
        call: spiralcore_mvm_adapter::service::CallHandle,
        now: ServiceTime,
        retain_tail: bool,
        deadline: Instant,
    ) -> Result<Option<Frame>> {
        self.pending
            .as_ref()
            .ok_or(SessionError::StreamFenced)?
            .remaining(now)?;
        if retain_tail {
            if self.completed_tail.is_some() {
                return Err(ServiceError::State.into());
            }
            let pending = self.pending.as_mut().ok_or(SessionError::StreamFenced)?;
            let send = pending.send_handle().ok_or(ServiceError::State)?;
            let events = pending.events_received();
            let ack_invocations = pending.reply_ack_invocations();
            let completed = match self.service.take_call_reply_with_tail(
                &self.outgoing,
                &self.incoming,
                &call,
                pending.reply_wire_mut(),
                now,
            ) {
                Ok(completed) => completed,
                Err(ServiceError::Backpressured) => return Ok(None),
                Err(error) => return Err(error.into()),
            };
            let reply = Frame::decode(&completed.payload)?;
            self.completed_tail = Some(CompletedTail {
                lease: completed.tail,
                send,
                deadline,
                events,
                ack_invocations,
                ack: ReplyAckState::Sent,
            });
            return Ok(Some(reply));
        }
        match self.service.poll_call(&self.outgoing, &call, now)? {
            CallStatus::Pending => Ok(None),
            CallStatus::Replied(Some(payload)) => Ok(Some(Frame::decode(&payload)?)),
            CallStatus::Replied(None) => Err(SessionError::Service(ServiceError::State).into()),
            CallStatus::TimedOut => Err(SessionError::Service(ServiceError::Expired).into()),
            CallStatus::Cancelled => Err(SessionError::Service(ServiceError::Cancelled).into()),
            CallStatus::Unavailable => Err(SessionError::Service(ServiceError::Unavailable).into()),
            CallStatus::ShuttingDown => {
                Err(SessionError::Service(ServiceError::ShuttingDown).into())
            }
        }
    }
    pub(super) fn drive_completed_tail<S: SocketIo + 'static>(
        &mut self,
        stream: &StreamTransport<S>,
        cancel: &CancellationToken,
    ) -> Result<TailDriveOutcome> {
        if self.caller_fenced {
            return Err(SessionError::StreamFenced.into());
        }
        if self.pending.is_some() {
            return Err(SessionError::Busy.into());
        }
        let result = self.drive_completed_tail_inner(stream, cancel);
        if result.is_err() {
            self.caller_fenced = true;
        }
        result
    }
    fn drive_completed_tail_inner<S: SocketIo + 'static>(
        &mut self,
        stream: &StreamTransport<S>,
        cancel: &CancellationToken,
    ) -> Result<TailDriveOutcome> {
        let Some(tail) = &self.completed_tail else {
            return Ok(TailDriveOutcome::NoTail);
        };
        let deadline = tail.deadline;
        let broker = self.broker.clone();
        loop {
            broker.with_time(|_, now| -> Result<()> {
                self.service.expire(now)?;
                self.tracker.expire(now)?;
                let tail = self.completed_tail.as_ref().ok_or(ServiceError::State)?;
                tail.lease.validate(&self.outgoing, &self.incoming, now)?;
                if Instant::now() >= deadline || tail.ack == ReplyAckState::InvokedUncertain {
                    return Err(ServiceError::Expired.into());
                }
                if tail.events >= 16 {
                    return Err(SessionError::EventLimit.into());
                }
                Ok(())
            })?;
            let bytes =
                match stream.receive_or_idle(cancel, deadline, Duration::from_millis(250))? {
                    spiralcore_mvm_adapter::live::ReceiveOutcome::Idle => {
                        increment(&mut self.recovery.tail_idle_outcomes)?;
                        return Ok(TailDriveOutcome::Idle);
                    }
                    spiralcore_mvm_adapter::live::ReceiveOutcome::Frame(bytes) => bytes,
                };
            self.completed_tail
                .as_mut()
                .ok_or(ServiceError::State)?
                .receive_event()?;
            increment(&mut self.recovery.tail_events)?;
            let envelope = decode_envelope(&bytes)?;
            if envelope.encryption.key_id == self.control_key {
                broker.with_time(|_, now| -> Result<()> {
                    let tail = self.completed_tail.as_ref().ok_or(ServiceError::State)?;
                    tail.lease.validate(&self.outgoing, &self.incoming, now)?;
                    self.tracker.observe_receipt(&self.control, &bytes, now)?;
                    self.tracker.take_receipt(&self.outgoing, &tail.send, now)?;
                    Ok(())
                })?;
                continue;
            }
            let ack = broker.with_time(|_, now| -> Result<_> {
                let tail = self.completed_tail.as_mut().ok_or(ServiceError::State)?;
                tail.lease.validate(&self.outgoing, &self.incoming, now)?;
                if envelope.encryption.key_id != self.normal_key || bytes != tail.lease.wire() {
                    return Err(ServiceError::Unauthorized.into());
                }
                let (accepted, ack) =
                    self.returns
                        .admit(&mut self.service, &self.incoming, &bytes, now)?;
                if accepted.receipt.outcome != DeliveryOutcome::Duplicate
                    || self.tracker.observe_registered_application_reply(
                        &self.incoming,
                        &bytes,
                        now,
                    )? != Some(tail.send)
                {
                    return Err(ServiceError::Unauthorized.into());
                }
                tail.ack = ReplyAckState::Needed;
                Ok(ack)
            })?;
            let (permit, now) = broker.with_time(|_, now| -> Result<_> {
                let tail = self.completed_tail.as_ref().ok_or(ServiceError::State)?;
                tail.lease.validate(&self.outgoing, &self.incoming, now)?;
                if Instant::now() >= deadline {
                    return Err(ServiceError::Expired.into());
                }
                Ok((ack.reserve_send(cancel, now)?, now))
            })?;
            self.completed_tail
                .as_mut()
                .ok_or(ServiceError::State)?
                .invoke_ack()?;
            increment(&mut self.recovery.tail_ack_invocations)?;
            permit.send(&stream.before(deadline), cancel, now)?;
            self.completed_tail.as_mut().ok_or(ServiceError::State)?.ack = ReplyAckState::Sent;
            increment(&mut self.recovery.tail_acks_written)?;
            return Ok(TailDriveOutcome::AckWritten);
        }
    }
    pub(super) fn handle<S: SocketIo + 'static>(
        &mut self,
        expected: Id,
        object: &mut ObjectMailbox,
        stream: &StreamTransport<S>,
        cancel: &CancellationToken,
    ) -> Result<Handled> {
        self.handle_with_replay_mode(expected, object, stream, cancel, None)
    }
    pub(super) fn handle_with_reply_replay<S: SocketIo + 'static>(
        &mut self,
        expected: Id,
        object: &mut ObjectMailbox,
        stream: &StreamTransport<S>,
        cancel: &CancellationToken,
        requests: ReplyReplayRequests,
    ) -> Result<Handled> {
        self.handle_with_replay_mode(expected, object, stream, cancel, Some(requests))
    }
    fn handle_with_replay_mode<S: SocketIo + 'static>(
        &mut self,
        expected: Id,
        object: &mut ObjectMailbox,
        stream: &StreamTransport<S>,
        cancel: &CancellationToken,
        requests: Option<ReplyReplayRequests>,
    ) -> Result<Handled> {
        let result = self.handle_inner(expected, object, stream, cancel, requests);
        if let Err(error) = self.receiver_processing.finish(&mut self.receiver_phase) {
            self.receiver_incomplete = true;
            return Err(error);
        }
        if result.is_err() {
            increment(&mut self.recovery.receiver_failed_invocations)?;
        }
        result
    }

    fn handle_inner<S: SocketIo + 'static>(
        &mut self,
        expected: Id,
        object: &mut ObjectMailbox,
        stream: &StreamTransport<S>,
        cancel: &CancellationToken,
        requests: Option<ReplyReplayRequests>,
    ) -> Result<Handled> {
        if self.calls >= MAX_CAMPAIGN_CALLS {
            return Err("native campaign Call limit".into());
        }
        if self.receiver_incomplete {
            return Err(SessionError::StreamFenced.into());
        }
        // This slice has no receiver resume: every incomplete invocation keeps
        // its canonical state and fences before a subsequent frame is read.
        self.receiver_incomplete = true;
        let deadline = Instant::now() + CALL_TIMEOUT;
        let broker = self.broker.clone();
        let mut events = 0_u8;
        let mut meter = Meter::default();
        let (receipt, proof, deadline) = loop {
            if events >= 16 {
                return Err(SessionError::EventLimit.into());
            }
            let bytes = stream.receive(cancel, deadline)?;
            events += 1;
            self.receiver_phase = Some((ProcessTime::now(), Instant::now()));
            let (receipt, proof, admitted_deadline) = broker.with_time(|_, now| -> Result<_> {
                if Instant::now() >= deadline {
                    return Err(ServiceError::Expired.into());
                }
                let envelope = decode_envelope(&bytes)?;
                if envelope.encryption.key_id == self.control_key {
                    self.tracker.observe_receipt(&self.control, &bytes, now)?;
                    increment(&mut self.recovery.historical_control_events)?;
                    return Ok((None, None, None));
                }
                if envelope.encryption.key_id != self.normal_key {
                    return Err("expected provisioned Call key".into());
                }
                // At the event ceiling, only an authenticated historical Call
                // may proceed: a fresh admission would have no room for its ACK.
                if events == 16 {
                    match self
                        .service
                        .retry_reply_prepared(&self.incoming, &bytes, now)
                    {
                        Ok(None) => {}
                        Ok(Some(_)) => return Err(ServiceError::State.into()),
                        Err(ServiceError::UnknownHandle) => {
                            return Err(SessionError::EventLimit.into());
                        }
                        Err(error) => return Err(error.into()),
                    }
                }
                // Inactive preamble: ordinary fresh admission verifies once.
                // The active ACK-wait loop below still prechecks cached identity.
                let (accepted, receipt) =
                    self.returns
                        .admit(&mut self.service, &self.incoming, &bytes, now)?;
                match accepted.receipt.outcome {
                    DeliveryOutcome::AcceptedEphemeral => {
                        if events == 16 {
                            return Err(SessionError::EventLimit.into());
                        }
                        let proof = self
                            .service
                            .receive(&self.incoming, now)?
                            .ok_or("missing admitted Call")?;
                        let cap = ExchangeDeadline::new(
                            deadline,
                            proof.header().expires_at_unix_ms,
                            now,
                        )?;
                        Ok((Some(receipt), Some(proof), Some(cap)))
                    }
                    DeliveryOutcome::Duplicate => {
                        // Canonical admission authenticated inner/outer expiry
                        // equality. The original-owner lookup below is separate.
                        let cap = ExchangeDeadline::new(
                            deadline,
                            envelope.routing.expires_at_unix_ms,
                            now,
                        )?;
                        Ok((Some(receipt), None, Some(cap)))
                    }
                    _ => Err("Call admission refused".into()),
                }
            })?;
            if let Some(proof) = proof {
                break (
                    receipt.ok_or(ServiceError::State)?,
                    proof,
                    admitted_deadline.ok_or(ServiceError::State)?,
                );
            }
            if let Some(receipt) = receipt {
                let cap = admitted_deadline.ok_or(ServiceError::State)?;
                let (permit, _) = broker.with_time(|_, now| -> Result<_> {
                    cap.check(now)?;
                    if self
                        .service
                        .retry_reply_prepared(&self.incoming, &bytes, now)?
                        .is_some()
                    {
                        return Err(ServiceError::State.into());
                    }
                    Ok((receipt.reserve_send(cancel, now)?, now))
                })?;
                let now = broker.with_time(|_, now| -> Result<_> {
                    cap.check(now)?;
                    if self
                        .service
                        .retry_reply_prepared(&self.incoming, &bytes, now)?
                        .is_some()
                    {
                        return Err(ServiceError::State.into());
                    }
                    Ok(now)
                })?;
                increment(&mut self.recovery.historical_receipt_invocations)?;
                permit.send(&stream.before(cap.instant()), cancel, now)?;
                increment(&mut self.recovery.historical_receipts_sent)?;
            }
            let processed = self.receiver_processing.finish(&mut self.receiver_phase)?;
            meter.add(processed.cpu_us, processed.wall_us)?;
        };
        let (permit, now) = broker.with_time(|_, now| -> Result<_> {
            deadline.check(now)?;
            Ok((receipt.reserve_send(cancel, now)?, now))
        })?;
        permit.send(&stream.before(deadline.instant()), cancel, now)?;
        let (reply, namespace) = application_phase(&broker, |now| {
            deadline.check(now)?;
            let frame = Frame::decode(proof.payload())?;
            if frame.transfer != expected {
                return Err("unexpected campaign transfer".into());
            }
            let namespace = object.context(&self.incoming, &proof, now)?.namespace()?;
            let reply = object.handle(&self.incoming, &proof, now)?;
            Ok((reply, namespace))
        })?;
        let (prepared, encoded, send) = broker.with_time(|_, now| -> Result<_> {
            deadline.check(now)?;
            let payload = reply.encode()?;
            let options = self
                .ordering
                .next_reply(&self.outgoing, &proof, now, ISSUANCE_POLICY)?;
            let prepared = self.service.reply_prepared(
                &self.outgoing,
                proof.handle(),
                service_sender::request(&self.outgoing, &payload, &options, now)?,
                now,
            )?;
            let encoded = encode_envelope(prepared.envelope())?;
            let send = self.tracker.register_send(
                &self.outgoing,
                &self.control,
                &self.peer_key,
                &encoded,
                now,
            )?;
            Ok((prepared, encoded, send))
        })?;
        let sent_at = broker.with_time(|_, now| -> Result<_> {
            deadline.check(now)?;
            Ok(now)
        })?;
        let mut sends = 1_u8;
        increment(&mut self.recovery.reply_send_invocations)?;
        self.outgoing.send_prepared(
            &prepared,
            &stream.before(deadline.instant()),
            cancel,
            sent_at,
        )?;
        // Receiver idle waits stay outside processing CPU/wall intervals.
        let processed = self.receiver_processing.finish(&mut self.receiver_phase)?;
        meter.add(processed.cpu_us, processed.wall_us)?;
        let mut acknowledged = false;
        // Preamble and initial Call share this invocation's cumulative bound.
        while events < 16 {
            let event = if let Some(requests) = &requests {
                match stream.receive_or_idle(
                    cancel,
                    deadline.instant(),
                    Duration::from_millis(250),
                )? {
                    spiralcore_mvm_adapter::live::ReceiveOutcome::Frame(bytes) => bytes,
                    spiralcore_mvm_adapter::live::ReceiveOutcome::Idle => {
                        broker.with_time(|_, now| -> Result<()> {
                            deadline.check(now)?;
                            self.service.expire(now)?;
                            self.tracker.expire(now)?;
                            Ok(())
                        })?;
                        if !requests.take_request() {
                            continue;
                        }
                        increment(&mut self.recovery.reply_replay_commands_consumed)?;
                        if sends >= 8 {
                            return Err(SessionError::SendLimit.into());
                        }
                        self.receiver_phase = Some((ProcessTime::now(), Instant::now()));
                        let cached = broker.with_time(|_, now| -> Result<_> {
                            deadline.check(now)?;
                            let cached = self
                                .service
                                .retry_reply_for_delivery_prepared(
                                    &self.incoming,
                                    proof.handle(),
                                    now,
                                )?
                                .ok_or(ServiceError::State)?;
                            let wire = encode_envelope(cached.envelope())?;
                            if wire != encoded
                                || self.tracker.register_send(
                                    &self.outgoing,
                                    &self.control,
                                    &self.peer_key,
                                    &wire,
                                    now,
                                )? != send
                            {
                                return Err(ServiceError::Conflict.into());
                            }
                            Ok(cached)
                        })?;
                        let now = broker.with_time(|_, now| -> Result<_> {
                            deadline.check(now)?;
                            let fresh = self
                                .service
                                .retry_reply_for_delivery_prepared(
                                    &self.incoming,
                                    proof.handle(),
                                    now,
                                )?
                                .ok_or(ServiceError::State)?;
                            if encode_envelope(fresh.envelope())? != encoded {
                                return Err(ServiceError::Conflict.into());
                            }
                            Ok(now)
                        })?;
                        sends += 1;
                        increment(&mut self.recovery.cached_reply_retry_invocations)?;
                        increment(&mut self.recovery.reply_send_invocations)?;
                        self.outgoing.send_prepared(
                            &cached,
                            &stream.before(deadline.instant()),
                            cancel,
                            now,
                        )?;
                        let processed =
                            self.receiver_processing.finish(&mut self.receiver_phase)?;
                        meter.add(processed.cpu_us, processed.wall_us)?;
                        continue;
                    }
                }
            } else {
                stream.receive(cancel, deadline.instant())?
            };
            events += 1;
            self.receiver_phase = Some((ProcessTime::now(), Instant::now()));
            let key = decode_envelope(&event)?.encryption.key_id;
            let repeat = broker.with_time(|_, now| -> Result<_> {
                deadline.check(now)?;
                if key == self.control_key {
                    self.tracker.observe_receipt(&self.control, &event, now)?;
                    match self.tracker.take_receipt(&self.outgoing, &send, now)? {
                        ReceiptObservation::Pending => return Ok(None),
                        ReceiptObservation::Expired => return Err(ServiceError::Expired.into()),
                        ReceiptObservation::Verified(receipt) => {
                            if !matches!(
                                receipt.receipt.outcome,
                                DeliveryOutcome::AcceptedEphemeral
                                    | DeliveryOutcome::AcceptedDurable
                                    | DeliveryOutcome::Duplicate
                            ) {
                                return Err(
                                    SessionError::RemoteRefusal(receipt.receipt.outcome).into()
                                );
                            }
                            if !self.service.acknowledge_reply(
                                &self.outgoing,
                                &receipt,
                                &self.peer_key,
                                now,
                            )? {
                                return Err(ServiceError::State.into());
                            }
                            acknowledged = true;
                            return Ok(None);
                        }
                    }
                }
                if key != self.normal_key {
                    return Err("unprovisioned incoming encryption key".into());
                }
                if sends >= 8 {
                    return Err(SessionError::SendLimit.into());
                }
                // Authenticate and identify the cached current Reply BEFORE
                // admitting a candidate: a new Call must not gain fresh state.
                let cached = self
                    .service
                    .retry_reply_prepared(&self.incoming, &event, now)?
                    .ok_or(ServiceError::State)?;
                if encode_envelope(cached.envelope())? != encoded {
                    return Err(ServiceError::Conflict.into());
                }
                let (accepted, receipt) =
                    self.returns
                        .admit(&mut self.service, &self.incoming, &event, now)?;
                if accepted.receipt.outcome != DeliveryOutcome::Duplicate {
                    return Err(ServiceError::State.into());
                }
                Ok(Some(receipt))
            })?;
            if let Some(receipt) = repeat {
                // Replay the Reply before the closing Call receipt. A caller
                // with an earlier Reply/ACK must ACK this replay before that
                // receipt can complete its still-missing admission observation.
                let (cached, now) = broker.with_time(|_, now| -> Result<_> {
                    deadline.check(now)?;
                    let cached = self
                        .service
                        .retry_reply_prepared(&self.incoming, &event, now)?
                        .ok_or(ServiceError::State)?;
                    let wire = encode_envelope(cached.envelope())?;
                    if wire != encoded
                        || self.tracker.register_send(
                            &self.outgoing,
                            &self.control,
                            &self.peer_key,
                            &wire,
                            now,
                        )? != send
                    {
                        return Err(ServiceError::Conflict.into());
                    }
                    Ok((cached, now))
                })?;
                sends += 1;
                increment(&mut self.recovery.cached_reply_retry_invocations)?;
                increment(&mut self.recovery.reply_send_invocations)?;
                self.outgoing.send_prepared(
                    &cached,
                    &stream.before(deadline.instant()),
                    cancel,
                    now,
                )?;
                // The write may wait: revalidate original ownership, both
                // bindings, exact cache and tracker before the closing receipt.
                let (permit, now) = broker.with_time(|_, now| -> Result<_> {
                    deadline.check(now)?;
                    let cached = self
                        .service
                        .retry_reply_prepared(&self.incoming, &event, now)?
                        .ok_or(ServiceError::State)?;
                    let wire = encode_envelope(cached.envelope())?;
                    if wire != encoded
                        || self.tracker.register_send(
                            &self.outgoing,
                            &self.control,
                            &self.peer_key,
                            &wire,
                            now,
                        )? != send
                    {
                        return Err(ServiceError::Conflict.into());
                    }
                    Ok((receipt.reserve_send(cancel, now)?, now))
                })?;
                permit.send(&stream.before(deadline.instant()), cancel, now)?;
            }
            let processed = self.receiver_processing.finish(&mut self.receiver_phase)?;
            meter.add(processed.cpu_us, processed.wall_us)?;
            if acknowledged {
                break;
            }
        }
        if !acknowledged {
            return Err(SessionError::EventLimit.into());
        }
        self.calls = self.calls.checked_add(1).ok_or("Call counter overflow")?;
        self.receiver_incomplete = false;
        Ok(Handled {
            reply,
            namespace,
            meter,
        })
    }
}
#[cfg(any(target_os = "linux", test))]
pub(super) fn validate_bootstrap(bootstrap: &Bootstrap) -> Result<()> {
    let profile = bootstrap
        .service
        .as_ref()
        .ok_or("missing service profile")?;
    if profile.profile != "canonical-ephemeral-v1" {
        return Err("unknown mailbox service profile".into());
    }
    validate_materials(
        bootstrap
            .incoming
            .as_ref()
            .ok_or("missing incoming material")?,
        bootstrap
            .outgoing
            .as_ref()
            .ok_or("missing outgoing material")?,
        profile,
    )
}
fn validate_materials(
    incoming: &Material,
    outgoing: &Material,
    profile: &crate::config::ServiceProvision,
) -> Result<()> {
    use mvm_contract::protocol::mailbox::MailboxMessageKind;
    for (normal, control, signing) in [
        (incoming, &profile.incoming_control, false),
        (outgoing, &profile.outgoing_control, true),
    ] {
        if control.kind != MailboxMessageKind::Cast
            || control.reply_to.is_some()
            || control.signing.is_some() != signing
            || control.capability.grant.content_types
                != [spiralcore_mvm_adapter::receipt::receipt_content_type()]
            || control.capability.grant.max_payload_bytes != 4096
            || control.capability.grant.max_queue_contribution_bytes != 16384
            || control.capability.grant.subject != normal.capability.grant.subject
            || control.capability.grant.address != normal.capability.grant.address
            || control.binding != normal.binding
            || control.key_id == normal.key_id
            || control.encryption == normal.encryption
        {
            return Err("invalid isolated receipt control provisioning".into());
        }
    }
    if incoming.signing.is_some()
        || outgoing.signing.is_none()
        || profile.incoming_control.key_id == profile.outgoing_control.key_id
    {
        return Err("invalid private service key partition".into());
    }
    for material in [outgoing, &profile.outgoing_control] {
        if SigningKey::from_bytes(material.signing.as_ref().ok_or("missing outgoing signer")?)
            .verifying_key()
            .to_bytes()
            != material.verification
        {
            return Err("service signer does not match configured identity".into());
        }
    }
    VerifyingKey::from_bytes(&profile.peer_receipt_verification)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Arm, Pair, Plan, Stage};

    fn pending_test_pair() -> Pair {
        Pair::provision(Plan {
            stage: Stage::Two,
            arm: Arm::Raw,
            size: 128,
            trial: 0,
            bandwidth_profile: Default::default(),
        })
        .unwrap()
    }

    #[test]
    fn real_caller_retains_the_partial_identity_and_refuses_a_substitute_frame() {
        use std::os::unix::net::UnixStream;
        let pair = pending_test_pair();
        let broker = broker_context().unwrap();
        let mut session = Session::new(&pair.sender, &broker).unwrap();
        broker
            .with_time::<_, spiralcore_mvm_adapter::service::ServiceError>(|budget, now| {
                session.tracker = ReceiptTracker::new(budget.clone(), 1)?;
                let mut options = service_sender::MessageOptions::fresh_for_binding(
                    &session.outgoing,
                    mvm_contract::protocol::mailbox::ContentType::parse(
                        spiralcore_mvm_adapter::CONTENT_TYPE,
                        1,
                    )
                    .map_err(|_| spiralcore_mvm_adapter::service::ServiceError::Malformed)?,
                    now,
                    now.unix_ms + 1000,
                    ISSUANCE_POLICY,
                )?;
                options.reply_to = session.reply_to;
                let old = service_sender::prepare(
                    &session.outgoing,
                    b"retained prior request",
                    &options,
                    now,
                )?;
                let wire = encode_envelope(old.envelope()).unwrap();
                session.tracker.register_send(
                    &session.outgoing,
                    &session.control,
                    &session.peer_key,
                    &wire,
                    now,
                )?;
                Ok(())
            })
            .unwrap();
        let (socket, _peer) = UnixStream::pair().unwrap();
        let stream = StreamTransport::new(
            socket,
            pair.sender
                .outgoing
                .as_ref()
                .unwrap()
                .binding
                .claims
                .route_id,
            CALL_TIMEOUT,
        )
        .unwrap();
        let cancel = CancellationToken::default();
        let frame = Frame {
            kind: 6,
            transfer: [17; 32],
            seq: 0,
            body: Vec::new(),
        };
        let error = session.call(&frame, &stream, &cancel).unwrap_err();
        assert!(matches!(
            error.downcast_ref::<SessionError>(),
            Some(SessionError::Service(
                spiralcore_mvm_adapter::service::ServiceError::Full
            ))
        ));
        let pending = session.pending.as_ref().unwrap();
        let original_wire = pending.encoded().unwrap().to_vec();
        let original_call = pending.call_handle().unwrap();
        assert_eq!(pending.prepared().unwrap().header().sequence.get(), 1);
        let mut substitute = frame.clone();
        substitute.seq = 1;
        let error = session.call(&substitute, &stream, &cancel).unwrap_err();
        assert_eq!(
            error.downcast_ref::<SessionError>(),
            Some(&SessionError::Busy)
        );
        assert!(session.call(&frame, &stream, &cancel).is_err());
        let pending = session.pending.as_ref().unwrap();
        assert_eq!(pending.call_handle(), Some(original_call));
        assert_eq!(pending.encoded().unwrap(), original_wire);
        let counters = stream.counters().unwrap();
        assert_eq!(counters.envelope_bytes_sent, 0);
        assert_eq!(counters.framing_bytes_sent, 0);
        assert_eq!(session.calls, 0);
        broker
            .with_time::<_, spiralcore_mvm_adapter::service::ServiceError>(|budget, _| {
                assert_eq!(budget.snapshot()?.active_calls, 1);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn invoked_send_failure_retains_identity_and_blocks_an_implicit_retransmission() {
        use std::os::unix::net::UnixStream;
        let pair = pending_test_pair();
        let broker = broker_context().unwrap();
        let mut session = Session::new(&pair.sender, &broker).unwrap();
        let (socket, _peer) = UnixStream::pair().unwrap();
        // Intentionally unready transport: its failure is not interpreted as
        // permission to reseal, allocate another sequence or reset the session.
        let stream = StreamTransport::new(
            socket,
            pair.sender
                .outgoing
                .as_ref()
                .unwrap()
                .binding
                .claims
                .route_id,
            CALL_TIMEOUT,
        )
        .unwrap();
        let frame = Frame {
            kind: 6,
            transfer: [18; 32],
            seq: 0,
            body: Vec::new(),
        };
        let cancel = CancellationToken::default();
        assert!(session.call(&frame, &stream, &cancel).is_err());
        let pending = session.pending.as_ref().unwrap();
        assert!(pending.send_started());
        let original = pending.encoded().unwrap().to_vec();
        let error = session.call(&frame, &stream, &cancel).unwrap_err();
        assert_eq!(
            error.downcast_ref::<SessionError>(),
            Some(&SessionError::SendUncertain)
        );
        assert_eq!(
            session.pending.as_ref().unwrap().encoded().unwrap(),
            original
        );
        assert_eq!(session.calls, 0);
    }
    #[test]
    fn independent_service_dispatch_progresses_while_application_phase_is_blocked() {
        use std::sync::mpsc;
        let broker = broker_context().unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        std::thread::scope(|scope| {
            let application_broker = &broker;
            let application = scope.spawn(move || {
                application_phase(application_broker, |_| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                })
            });
            entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            let control = scope.spawn(|| {
                let result = broker.with_time(|budget, now| {
                    let mut independent = LocalMailboxService::new(budget.clone())?;
                    independent.expire(now)
                });
                done_tx.send(result).unwrap();
            });
            // The deadline is a deadlock watchdog, not a latency assertion.
            // Always release and join both workers before asserting failure.
            let progress = done_rx.recv_timeout(Duration::from_secs(2));
            release_tx.send(()).unwrap();
            application.join().unwrap().unwrap();
            control.join().unwrap();
            assert!(
                progress.is_ok_and(|result| result.is_ok()),
                "application work retained global dispatch"
            );
        });
    }
    #[test]
    fn explicit_campaign_tier_reserves_worst_case_raw_control_and_data_identities() {
        let worst_raw_calls = 1 + 7 * ((16usize << 20).div_ceil(transfer::MAX_CHUNK) + 4);
        assert!(worst_raw_calls as u64 <= MAX_CAMPAIGN_CALLS);
        let (limits, reserve) = campaign_limits();
        let calls = MAX_CAMPAIGN_CALLS as usize;
        assert!(reserve.sender.identities >= 2 * calls);
        for capacity in [
            reserve.host.identities,
            reserve.tenant.identities,
            reserve.mailbox.identities,
        ] {
            assert!(capacity >= 3 * calls);
        }
        for (total, control) in [
            (limits.host, reserve.host),
            (limits.tenant, reserve.tenant),
            (limits.mailbox, reserve.mailbox),
            (limits.sender, reserve.sender),
        ] {
            assert!(total.identities - control.identities >= 3 * calls + 2);
        }
        campaign_budget().unwrap();
    }
}

#[cfg(test)]
mod pending_tests;

#[cfg(test)]
mod event_tests;

#[cfg(test)]
mod ack_tests;

#[cfg(test)]
mod limit_tests;

#[cfg(test)]
mod fifo_tests;

#[cfg(test)]
mod late_tests;

#[cfg(test)]
mod receiver_tests;

#[cfg(test)]
mod reconciliation_tests;

#[cfg(test)]
mod write_deadline_tests;

#[cfg(test)]
mod reconciliation_limits_tests;

#[cfg(all(test, unix))]
mod tail_limits_tests;

#[cfg(all(
    test,
    any(
        all(target_os = "linux", feature = "linux-host"),
        all(target_os = "macos", target_arch = "aarch64")
    )
))]
mod completed_tail_tests;
