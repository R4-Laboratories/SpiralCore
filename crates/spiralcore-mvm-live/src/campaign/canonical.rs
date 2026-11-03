//! Canonical mailbox service consumer shared by host and real guest campaigns.
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
        CallStatus, ControlReserve, HostBudget, LocalMailboxService, MailboxBinding, ServiceLimits,
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
}
const MAX_CAMPAIGN_CALLS: u64 = 2048;
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
            })
        })
    }
    pub(super) fn report(&self) -> Value {
        json!({"application_calls":self.calls,"normal_envelopes_per_call":2,"encrypted_destination_receipts_per_call":2,"frames_per_call":4,"profile":"canonical-ephemeral-v1","durability":"AcceptedEphemeral; object commit separately verified durable","identity_tier":"broker-owned","witness_tier":"native-16m-v1","identity_limits":self.owner_capacity["identity_limits"],"control_identity_limits":self.owner_capacity["control_identity_limits"],"max_application_calls":MAX_CAMPAIGN_CALLS,"pipeline":1,"ordering":"stable-scope-increasing-sequence","broker_clock":"shared-serialized-dispatch","issuance_policy":{"mode":"bounded","max_clock_ahead_ms":5000,"max_issued_ttl_ms":30000,"max_remaining_ttl_ms":25000}})
    }
    pub(super) fn call<S: SocketIo + 'static>(
        &mut self,
        request: &Frame,
        stream: &StreamTransport<S>,
        cancel: &CancellationToken,
    ) -> Result<(Context, Frame)> {
        if self.calls >= MAX_CAMPAIGN_CALLS {
            return Err("native campaign Call limit".into());
        }
        let deadline = Instant::now() + CALL_TIMEOUT;
        let broker = self.broker.clone();
        let (prepared, context, call, send, deadline, send_time) =
            broker.with_time(|_, now| -> Result<_> {
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
                let prepared =
                    service_sender::prepare(&self.outgoing, &request.encode()?, &options, now)?;
                let encoded = encode_envelope(prepared.envelope())?;
                let context = match &self.context {
                    Some(context) => context.clone(),
                    None => {
                        let context =
                            service_sender::object_context(&self.outgoing, &prepared, now)?;
                        self.context = Some(context.clone());
                        context
                    }
                };
                let remaining = u64::try_from(
                    options
                        .expires_at_unix_ms
                        .checked_sub(now.unix_ms)
                        .ok_or("Call deadline overflow")?,
                )
                .map_err(|_| "Call already expired")?;
                let deadline = deadline.min(Instant::now() + Duration::from_millis(remaining));
                let call = self.service.register_call(&self.outgoing, &encoded, now)?;
                let send = self.tracker.register_send(
                    &self.outgoing,
                    &self.control,
                    &self.peer_key,
                    &encoded,
                    now,
                )?;
                Ok((prepared, context, call, send, deadline, now))
            })?;
        // No broker lease spans potentially blocking transport I/O.
        self.outgoing
            .send_prepared(&prepared, stream, cancel, send_time)?;
        let mut admitted = false;
        let mut reply = None;
        for _ in 0..2 {
            let bytes = stream.receive(cancel, deadline)?;
            let envelope = decode_envelope(&bytes)?;
            let acknowledgment = broker.with_time(|_, now| -> Result<_> {
                if envelope.encryption.key_id == self.control_key {
                    self.tracker.observe_receipt(&self.control, &bytes, now)?;
                    let ReceiptObservation::Verified(receipt) =
                        self.tracker.take_receipt(&self.outgoing, &send, now)?
                    else {
                        return Err("missing authenticated Call admission".into());
                    };
                    if !matches!(
                        receipt.receipt.outcome,
                        DeliveryOutcome::AcceptedEphemeral | DeliveryOutcome::Duplicate
                    ) {
                        return Err("Call admission refused".into());
                    }
                    admitted = true;
                    Ok(None)
                } else if envelope.encryption.key_id == self.normal_key {
                    let (accepted, ack) =
                        self.returns
                            .admit(&mut self.service, &self.incoming, &bytes, now)?;
                    if accepted.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
                        return Err("Reply admission refused".into());
                    }
                    self.tracker
                        .observe_application_reply(&self.incoming, &bytes, now)?;
                    let CallStatus::Replied(Some(payload)) =
                        self.service.poll_call(&self.outgoing, &call, now)?
                    else {
                        return Err("Reply did not complete admitted Call".into());
                    };
                    reply = Some(Frame::decode(&payload)?);
                    Ok(Some((ack, now)))
                } else {
                    Err("unprovisioned incoming encryption key".into())
                }
            })?;
            if let Some((ack, _)) = acknowledgment {
                let (permit, now) = broker.with_time(|_, now| -> Result<_> {
                    Ok((ack.reserve_send(cancel, now)?, now))
                })?;
                permit.send(stream, cancel, now)?;
            }
        }
        if !admitted {
            return Err("missing Call destination receipt".into());
        }
        self.calls = self.calls.checked_add(1).ok_or("Call counter overflow")?;
        Ok((context, reply.ok_or("missing application Reply")?))
    }
    pub(super) fn handle<S: SocketIo + 'static>(
        &mut self,
        expected: Id,
        object: &mut ObjectMailbox,
        stream: &StreamTransport<S>,
        cancel: &CancellationToken,
    ) -> Result<Handled> {
        if self.calls >= MAX_CAMPAIGN_CALLS {
            return Err("native campaign Call limit".into());
        }
        let bytes = stream.receive(cancel, Instant::now() + CALL_TIMEOUT)?;
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        let broker = self.broker.clone();
        let (receipt, proof) = broker.with_time(|_, now| -> Result<_> {
            if decode_envelope(&bytes)?.encryption.key_id != self.normal_key {
                return Err("expected provisioned Call key".into());
            }
            let (accepted, receipt) =
                self.returns
                    .admit(&mut self.service, &self.incoming, &bytes, now)?;
            if accepted.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
                return Err("Call admission refused".into());
            }
            let proof = self
                .service
                .receive(&self.incoming, now)?
                .ok_or("missing admitted Call")?;
            Ok((receipt, proof))
        })?;
        let (permit, now) = broker
            .with_time(|_, now| -> Result<_> { Ok((receipt.reserve_send(cancel, now)?, now)) })?;
        permit.send(stream, cancel, now)?;
        let (reply, namespace, prepared, send, sent_at) =
            broker.with_time(|_, now| -> Result<_> {
                let frame = Frame::decode(proof.payload())?;
                if frame.transfer != expected {
                    return Err("unexpected campaign transfer".into());
                }
                let namespace = object.context(&self.incoming, &proof, now)?.namespace()?;
                let reply = object.handle(&self.incoming, &proof, now)?;
                let payload = reply.encode()?;
                let options =
                    self.ordering
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
                Ok((reply, namespace, prepared, send, now))
            })?;
        self.outgoing
            .send_prepared(&prepared, stream, cancel, sent_at)?;
        // Strict single-Call sender sends the Reply receipt before its next Call.
        // Receiver idle wait is kept out of receiver processing CPU/wall metrics.
        let mut meter = Meter {
            cpu_us: cpu.elapsed().as_micros(),
            wall_us: wall.elapsed().as_micros(),
        };
        let remaining = broker.with_time(|_, now| -> Result<_> {
            u64::try_from(
                prepared
                    .header()
                    .expires_at_unix_ms
                    .checked_sub(now.unix_ms)
                    .ok_or("Reply deadline overflow")?,
            )
            .map_err(|_| "Reply already expired".into())
        })?;
        let ack = stream.receive(
            cancel,
            Instant::now() + Duration::from_millis(remaining).min(CALL_TIMEOUT),
        )?;
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        broker.with_time(|_, now| -> Result<_> {
            if decode_envelope(&ack)?.encryption.key_id != self.control_key {
                return Err("expected provisioned Reply receipt key".into());
            }
            self.tracker.observe_receipt(&self.control, &ack, now)?;
            let ReceiptObservation::Verified(receipt) =
                self.tracker.take_receipt(&self.outgoing, &send, now)?
            else {
                return Err("missing authenticated Reply admission".into());
            };
            if !self
                .service
                .acknowledge_reply(&self.outgoing, &receipt, &self.peer_key, now)?
            {
                return Err("Reply acknowledgment did not release cache".into());
            }
            Ok(())
        })?;
        meter.add(cpu.elapsed().as_micros(), wall.elapsed().as_micros())?;
        self.calls = self.calls.checked_add(1).ok_or("Call counter overflow")?;
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
