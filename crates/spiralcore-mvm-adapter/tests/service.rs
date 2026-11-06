#![cfg(feature = "live")]
mod common;
use ed25519_dalek::SigningKey;
use mvm_contract::protocol::mailbox::*;
use mvm_core::mailbox::{SealRequest, encode_envelope, seal_envelope, verify_receipt};
use spiralcore_mvm_adapter::{
    DirectionConfig,
    service::{
        DestinationReceiptSigner, HostBudget, LocalMailboxService, MailboxBinding,
        ServiceBindingConfig, ServiceLimits, ServiceTime,
    },
};

fn time() -> ServiceTime {
    ServiceTime {
        unix_ms: 1_500,
        monotonic_ms: 100,
    }
}
fn config(kind: MailboxMessageKind) -> DirectionConfig {
    let mut c = common::config(kind);
    if kind == MailboxMessageKind::Cast {
        c.reply_to = None;
    }
    c
}
fn binding(c: DirectionConfig) -> MailboxBinding {
    let context = c.policy.snapshot().unwrap();
    let mut config = ServiceBindingConfig::from_direction(c).unwrap();
    if config.kind == MailboxMessageKind::Reply {
        config.context.required_operation = MailboxOperation::Reply;
        config.capability.grant.operations =
            CapabilityOperations::from_operations(&[MailboxOperation::Reply]);
        config.capability = mvm_core::mailbox::sign_capability(
            config.capability.grant,
            &SigningKey::from_bytes(&[22; 32]),
        )
        .unwrap();
    }
    MailboxBinding::new(
        config,
        Some(DestinationReceiptSigner {
            recipient: context.expected_recipient,
            signing_key: SigningKey::from_bytes(&[24; 32]),
        }),
    )
    .unwrap()
}
fn wire(c: &DirectionConfig, payload: &[u8], id: u8) -> Vec<u8> {
    wire_amend(c, payload, id, |_| {})
}
fn wire_amend(
    c: &DirectionConfig,
    payload: &[u8],
    id: u8,
    amend: impl FnOnce(&mut MailboxEnvelopeHeader),
) -> Vec<u8> {
    let context = c.policy.snapshot().unwrap();
    let mut header = MailboxEnvelopeHeader {
        protocol_version: ProtocolVersion::V1,
        signature_algorithm: SignatureAlgorithm::ED25519,
        tenant: context.expected_tenant,
        authorization_domain: context.expected_authorization_domain,
        sender: context.expected_sender,
        recipient: context.expected_recipient,
        mailbox: context.expected_mailbox,
        sender_boot: c.boot,
        execution_epoch: context.current_execution_epoch,
        message_id: MessageId::from_bytes([id; 32]),
        correlation_id: CorrelationId::from_bytes([id; 32]),
        causation_id: CausationId::from_bytes([0; 32]),
        kind: c.kind,
        reply_to: c.reply_to,
        ordering_scope: OrderingScope::parse(format!("service-test-{id}")).unwrap(),
        sequence: SequenceNumber::new(1).unwrap(),
        issued_at_unix_ms: 1000,
        expires_at_unix_ms: 2000,
        nonce: ReplayNonce::from_bytes([id; 16]),
        content_type: c.capability.grant.content_types[0].clone(),
        capability_id: c.capability.grant.capability_id,
        policy_epoch: context.current_policy_epoch,
        encryption: EncryptionContext {
            algorithm: EncryptionAlgorithm::AES_256_GCM,
            key_id: c.key_id,
        },
        plaintext_len: payload.len() as u32,
    };
    amend(&mut header);
    let routing = MailboxRoutingFrame {
        protocol_version: ProtocolVersion::V1,
        authorization_domain_route_label: c.binding.claims.authorization_domain_route_label,
        destination_route_token: c.binding.claims.destination_route_token,
        delivery_id: DeliveryId::from_bytes(rand::random()),
        ciphertext_len: 0,
        padding: CiphertextPadding::Exact,
        expires_at_unix_ms: 2000,
        hop_limit: 8,
        transport_binding_id: c.binding.claims.binding_id,
    };
    encode_envelope(
        &seal_envelope(
            SealRequest {
                routing,
                header,
                plaintext: payload,
            },
            &c.encryption_key,
            c.signing_key.as_ref().unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn canonical_cast_admission_signs_destination_receipt_and_owns_exact_bytes() {
    let c = config(MailboxMessageKind::Cast);
    let encoded = wire(&c, b"exact\0value\0", 1);
    let binding = binding(c);
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let receipt = service.admit(&binding, &encoded, time()).unwrap();
    verify_receipt(&receipt, &SigningKey::from_bytes(&[24; 32]).verifying_key()).unwrap();
    assert_eq!(receipt.receipt.outcome, DeliveryOutcome::AcceptedEphemeral);
    let message = service.receive(&binding, time()).unwrap().unwrap();
    assert_eq!(message.payload(), b"exact\0value\0");
    assert_eq!(message.header().kind, MailboxMessageKind::Cast);
    assert!(service.receive(&binding, time()).unwrap().is_none());
}

#[test]
fn authenticated_resealed_retry_deduplicates_across_shared_service_owners_and_conflicts_refuse() {
    let c = config(MailboxMessageKind::Cast);
    let first = wire(&c, b"immutable", 2);
    let second = wire(&c, b"immutable", 2);
    assert_ne!(first, second);
    let conflict = wire(&c, b"different", 2);
    let a = binding(c);
    let b = binding(config(MailboxMessageKind::Cast));
    let budget = HostBudget::new(ServiceLimits::default()).unwrap();
    let mut one = LocalMailboxService::new(budget.clone()).unwrap();
    let mut two = LocalMailboxService::new(budget).unwrap();
    assert_eq!(
        one.admit(&a, &first, time()).unwrap().receipt.outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    assert_eq!(
        two.admit(&b, &second, time()).unwrap().receipt.outcome,
        DeliveryOutcome::Duplicate
    );
    assert!(two.receive(&b, time()).unwrap().is_none());
    assert_eq!(
        two.admit(&b, &conflict, time()).unwrap().receipt.outcome,
        DeliveryOutcome::Refused
    );
    let received = one.receive(&a, time()).unwrap().unwrap();
    assert_eq!(received.payload(), b"immutable");
    drop(received);
    drop(one);
    assert_eq!(
        two.admit(&b, &second, time()).unwrap().receipt.outcome,
        DeliveryOutcome::Duplicate
    );
}

#[test]
fn full_is_not_acceptance_retry_after_capacity_returns_and_terminal_identity_stays() {
    let c = config(MailboxMessageKind::Cast);
    let a = wire(&c, b"one", 3);
    let b = wire(&c, b"two", 4);
    let binding = binding(c);
    let mut limits = ServiceLimits::default();
    for x in [
        &mut limits.host,
        &mut limits.tenant,
        &mut limits.mailbox,
        &mut limits.sender,
    ] {
        x.queued_messages = 1;
    }
    let mut service = LocalMailboxService::new(HostBudget::new(limits).unwrap()).unwrap();
    assert_eq!(
        service.admit(&binding, &a, time()).unwrap().receipt.outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    assert_eq!(
        service.admit(&binding, &b, time()).unwrap().receipt.outcome,
        DeliveryOutcome::Full
    );
    drop(service.receive(&binding, time()).unwrap().unwrap());
    assert_eq!(
        service.admit(&binding, &b, time()).unwrap().receipt.outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    assert_eq!(
        service.admit(&binding, &a, time()).unwrap().receipt.outcome,
        DeliveryOutcome::Duplicate
    );
}
#[test]
fn call_timeout_cancel_and_shutdown_are_distinct_terminal_observations() {
    use spiralcore_mvm_adapter::service::{CallStatus, ServiceError};
    let c = config(MailboxMessageKind::Call);
    let a = wire(&c, b"request", 5);
    let b = wire(&c, b"second", 6);
    let binding =
        MailboxBinding::new(ServiceBindingConfig::from_direction(c).unwrap(), None).unwrap();
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let call = service.register_call(&binding, &a, time()).unwrap();
    assert_eq!(
        service.poll_call(&binding, &call, time()).unwrap(),
        CallStatus::Pending
    );
    assert_eq!(
        service.cancel(&binding, &call, time()).unwrap(),
        CallStatus::Cancelled
    );
    assert!(matches!(
        service.register_call(&binding, &a, time()),
        Err(ServiceError::Conflict)
    ));
    let second = service.register_call(&binding, &b, time()).unwrap();
    let later = ServiceTime {
        unix_ms: 2001,
        monotonic_ms: 601,
    };
    service.expire(later).unwrap();
    assert_eq!(
        service.poll_call(&binding, &second, later).unwrap(),
        CallStatus::TimedOut
    );
    service.shutdown(later).unwrap();
    assert!(matches!(
        service.register_call(&binding, &b, later),
        Err(ServiceError::ShuttingDown)
    ));
}

#[test]
fn authenticated_reply_matches_exact_call_and_is_consumed_once_late_reply_stays_late() {
    use spiralcore_mvm_adapter::service::CallStatus;
    let call_config = config(MailboxMessageKind::Call);
    let request = wire(&call_config, b"call", 7);
    let caller = MailboxBinding::new(
        ServiceBindingConfig::from_direction(call_config).unwrap(),
        None,
    )
    .unwrap();
    let reply_config = config(MailboxMessageKind::Reply);
    let reply = wire_amend(&reply_config, b"answer\0", 8, |h| {
        h.correlation_id = CorrelationId::from_bytes([7; 32]);
        h.causation_id = CausationId::from_bytes([7; 32]);
    });
    let late = wire_amend(&reply_config, b"late", 9, |h| {
        h.correlation_id = CorrelationId::from_bytes([7; 32]);
        h.causation_id = CausationId::from_bytes([7; 32]);
    });
    let replier = binding(reply_config);
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let call = service.register_call(&caller, &request, time()).unwrap();
    assert_eq!(
        service
            .admit(&replier, &reply, time())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    assert!(
        service.receive(&replier, time()).unwrap().is_none(),
        "Replies belong to the call registry, never a new handler"
    );
    assert_eq!(
        service.poll_call(&caller, &call, time()).unwrap(),
        CallStatus::Replied(Some(b"answer\0".to_vec()))
    );
    assert_eq!(
        service.poll_call(&caller, &call, time()).unwrap(),
        CallStatus::Replied(None)
    );
    assert_eq!(
        service
            .admit(&replier, &reply, time())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::Duplicate
    );
    assert_eq!(
        service
            .admit(&replier, &late, time())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::Late
    );
}

fn reply_request<'a>(
    c: &DirectionConfig,
    payload: &'a [u8],
    call_id: u8,
    reply_id: u8,
) -> SealRequest<'a> {
    let encoded = wire_amend(c, payload, reply_id, |h| {
        h.correlation_id = CorrelationId::from_bytes([call_id; 32]);
        h.causation_id = CausationId::from_bytes([call_id; 32]);
    });
    let envelope = mvm_core::mailbox::decode_envelope(&encoded).unwrap();
    let opened = mvm_core::mailbox::open_verified_envelope(
        &envelope,
        &c.capability,
        &c.policy.snapshot().unwrap(),
        &c.encryption_key,
        &c.sender_verifying_key,
        &c.authority_key,
        &mut mvm_core::mailbox::InMemoryReplayProtector::default(),
    )
    .unwrap();
    SealRequest {
        routing: envelope.routing,
        header: opened.header().clone(),
        plaintext: payload,
    }
}
#[test]
fn reply_sealing_requires_admitted_call_and_exact_reply_authority() {
    use spiralcore_mvm_adapter::service::ServiceError;
    let c = config(MailboxMessageKind::Call);
    let encoded = wire(&c, b"question", 10);
    let inbound = binding(c);
    let reply_c = config(MailboxMessageKind::Reply);
    let bad = reply_request(&reply_c, b"bad", 11, 12);
    let good = reply_request(&reply_c, b"good", 10, 13);
    let duplicate = reply_request(&reply_c, b"again", 10, 14);
    let reply_binding = binding(reply_c);
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    service.admit(&inbound, &encoded, time()).unwrap();
    let message = service.receive(&inbound, time()).unwrap().unwrap();
    assert!(matches!(
        service.reply(&reply_binding, message.handle(), bad, time()),
        Err(ServiceError::Unauthorized)
    ));
    let sealed = service
        .reply(&reply_binding, message.handle(), good, time())
        .unwrap();
    assert!(sealed.ciphertext.len() > 4);
    assert!(matches!(
        service.reply(&reply_binding, message.handle(), duplicate, time()),
        Err(ServiceError::Conflict)
    ));
}

#[test]
fn sender_quota_is_shared_across_mailboxes_and_signed_rate_is_not_ignored() {
    let mut c = config(MailboxMessageKind::Cast);
    c.capability.grant.max_messages_per_minute = 1;
    c.capability =
        mvm_core::mailbox::sign_capability(c.capability.grant, &SigningKey::from_bytes(&[22; 32]))
            .unwrap();
    let first = wire(&c, b"first", 15);
    let second = wire(&c, b"second", 16);
    let b = binding(c);
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    assert_eq!(
        service.admit(&b, &first, time()).unwrap().receipt.outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    drop(service.receive(&b, time()).unwrap());
    assert_eq!(
        service.admit(&b, &second, time()).unwrap().receipt.outcome,
        DeliveryOutcome::Full
    );
}

#[test]
fn data_cannot_spend_the_immutable_control_reserve() {
    use spiralcore_mvm_adapter::service::{ControlReserve, ControlScope, ScopeLimits};
    let mut limits = ServiceLimits::default();
    let small = ScopeLimits {
        queued_bytes: 1 << 20,
        queued_messages: 2,
        active_calls: 2,
        identities: 32,
    };
    limits.host = small;
    limits.tenant = small;
    limits.mailbox = small;
    limits.sender = small;
    let control = ControlScope {
        queued_bytes: 4096,
        queued_messages: 1,
        identities: 4,
    };
    let reserved = ControlReserve {
        host: control,
        tenant: control,
        mailbox: control,
        sender: control,
    };
    let mut service =
        LocalMailboxService::new(HostBudget::with_control(limits, reserved).unwrap()).unwrap();
    let c = config(MailboxMessageKind::Cast);
    let a = wire(&c, b"a", 17);
    let b = wire(&c, b"b", 18);
    let binding = binding(c);
    assert_eq!(
        service.admit(&binding, &a, time()).unwrap().receipt.outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    assert_eq!(
        service.admit(&binding, &b, time()).unwrap().receipt.outcome,
        DeliveryOutcome::Full
    );
}

#[test]
fn narrowed_payload_limit_also_applies_to_authenticated_replies() {
    let c = config(MailboxMessageKind::Call);
    let request = wire(&c, b"call", 19);
    let caller =
        MailboxBinding::new(ServiceBindingConfig::from_direction(c).unwrap(), None).unwrap();
    let r = config(MailboxMessageKind::Reply);
    let reply = wire_amend(&r, b"12345", 20, |h| {
        h.correlation_id = CorrelationId::from_bytes([19; 32]);
        h.causation_id = CausationId::from_bytes([19; 32]);
    });
    let replier = binding(r);
    let limits = ServiceLimits {
        max_payload_bytes: 4,
        ..ServiceLimits::default()
    };
    let mut service = LocalMailboxService::new(HostBudget::new(limits).unwrap()).unwrap();
    service.register_call(&caller, &request, time()).unwrap();
    assert_eq!(
        service
            .admit(&replier, &reply, time())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::Refused
    );
}

#[test]
fn canonical_authentication_boot_key_and_live_revocation_gate_admission_and_reads() {
    use spiralcore_mvm_adapter::service::ServiceError;
    let c = config(MailboxMessageKind::Cast);
    let bad = [
        wire_amend(&c, b"secret", 21, |h| {
            h.recipient = PrincipalId::from_bytes([99; 32])
        }),
        wire_amend(&c, b"secret", 22, |h| {
            h.execution_epoch = ExecutionEpoch::new(8).unwrap()
        }),
        wire_amend(&c, b"secret", 23, |h| {
            h.sender_boot = BootId::from_bytes([99; 32])
        }),
        wire_amend(&c, b"secret", 24, |h| h.encryption.key_id = [99; 32]),
    ];
    let good = wire(&c, b"exact", 25);
    let initial = c.policy.snapshot().unwrap();
    let b = binding(c);
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    for encoded in bad {
        assert!(matches!(
            service.admit(&b, &encoded, time()),
            Err(ServiceError::Unauthorized)
        ));
    }
    assert!(matches!(
        service.admit(&b, b"not an envelope", time()),
        Err(ServiceError::Malformed)
    ));
    assert_eq!(
        service.admit(&b, &good, time()).unwrap().receipt.outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    let mut revoked = initial;
    revoked
        .revoked_capabilities
        .push(RevocationId::from_bytes([13; 32]));
    b.replace_policy(revoked).unwrap();
    assert!(matches!(
        service.receive(&b, time()),
        Err(ServiceError::Unauthorized)
    ));
    assert!(matches!(
        service.admit(&b, &good, time()),
        Err(ServiceError::Unauthorized)
    ));
}

#[test]
fn transport_binding_revocation_survives_context_replacement() {
    use spiralcore_mvm_adapter::service::ServiceError;

    let c = config(MailboxMessageKind::Cast);
    let binding_id = c.binding.claims.binding_id;
    let context = c.policy.snapshot().unwrap();
    let encoded = wire(&c, b"exact", 25);
    let b = binding(c);
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();

    for number in 0..1025u32 {
        let mut id = [0; 32];
        id[..4].copy_from_slice(&number.to_le_bytes());
        b.revoke_transport_binding(TransportBindingId::from_bytes(id))
            .unwrap();
    }
    assert_eq!(
        service.admit(&b, &encoded, time()).unwrap().receipt.outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    b.revoke_transport_binding(binding_id).unwrap();
    b.revoke_transport_binding(binding_id).unwrap();
    b.replace_policy(context).unwrap();
    assert!(matches!(
        service.receive(&b, time()),
        Err(ServiceError::Unauthorized)
    ));
    assert!(matches!(
        service.admit(&b, &encoded, time()),
        Err(ServiceError::Unauthorized)
    ));
}

#[test]
fn legacy_direction_conversion_refuses_to_drop_recorded_binding_revocations() {
    let c = config(MailboxMessageKind::Cast);
    c.policy
        .revoke_transport_binding(c.binding.claims.binding_id)
        .unwrap();
    assert!(matches!(
        ServiceBindingConfig::from_direction(c),
        Err(spiralcore_mvm_adapter::service::ServiceError::Unauthorized)
    ));
}

#[test]
fn lost_reply_retry_replays_same_prepared_envelope_without_second_handler_and_ack_releases_cache() {
    let c = config(MailboxMessageKind::Call);
    let request = wire(&c, b"question", 26);
    let original = binding(c);
    let r = config(MailboxMessageKind::Reply);
    let seal = reply_request(&r, b"answer", 26, 27);
    let reply_binding = binding(r);
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    service.admit(&original, &request, time()).unwrap();
    let msg = service.receive(&original, time()).unwrap().unwrap();
    let first = service
        .reply_prepared(&reply_binding, msg.handle(), seal, time())
        .unwrap();
    assert_eq!(
        service
            .admit(&original, &request, time())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::Duplicate
    );
    assert!(service.receive(&original, time()).unwrap().is_none());
    let retry = service
        .retry_reply_prepared(&original, &request, time())
        .unwrap()
        .unwrap();
    assert_eq!(
        encode_envelope(first.envelope()).unwrap(),
        encode_envelope(retry.envelope()).unwrap()
    );
    let e = first.envelope();
    let receipt = mvm_core::mailbox::sign_receipt(
        DestinationReceipt {
            protocol_version: ProtocolVersion::V1,
            signature_algorithm: SignatureAlgorithm::ED25519,
            delivery_id: e.routing.delivery_id,
            message_id: MessageId::from_bytes([27; 32]),
            address: MailboxAddress {
                tenant: TenantId::from_bytes([19; 32]),
                authorization_domain: AuthorizationDomainId::from_bytes([1; 32]),
                mailbox: MailboxId::from_bytes([14; 32]),
                recipient: PrincipalId::from_bytes([2; 32]),
            },
            execution_epoch: ExecutionEpoch::new(7).unwrap(),
            outcome: DeliveryOutcome::AcceptedEphemeral,
            observed_at_unix_ms: 1500,
            ciphertext_commitment: e.ciphertext_commitment,
        },
        &SigningKey::from_bytes(&[24; 32]),
    )
    .unwrap();
    assert!(
        service
            .acknowledge_reply(
                &reply_binding,
                &receipt,
                &SigningKey::from_bytes(&[24; 32]).verifying_key(),
                time()
            )
            .unwrap()
    );
    assert!(
        service
            .retry_reply_prepared(&original, &request, time())
            .unwrap()
            .is_none()
    );
    assert_eq!(
        service
            .admit(&original, &request, time())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::Duplicate
    );
}

#[test]
fn prepared_send_uses_canonical_authorization_and_cancelled_or_wrong_scope_sends_nothing() {
    use mvm_core::mailbox::transport::{
        CancellationToken, InMemoryPeerTransport, prepare_mailbox_envelope,
    };
    use spiralcore_mvm_adapter::service::ServiceError;
    let c = config(MailboxMessageKind::Call);
    let route = c.binding.claims.route_id;
    let prepared = prepare_mailbox_envelope(
        reply_request(&c, b"payload", 28, 29),
        &c.encryption_key,
        c.signing_key.as_ref().unwrap(),
    )
    .unwrap();
    let binding =
        MailboxBinding::new(ServiceBindingConfig::from_direction(c).unwrap(), None).unwrap();
    let transport = InMemoryPeerTransport::default();
    transport.register_route(route).unwrap();
    let cancel = CancellationToken::default();
    cancel.cancel();
    assert!(matches!(
        binding.send_prepared(&prepared, &transport, &cancel, time()),
        Err(ServiceError::Cancelled)
    ));
    assert!(transport.drain(route).unwrap().is_empty());
    let encoded = encode_envelope(prepared.envelope()).unwrap();
    assert_eq!(
        binding
            .send_prepared(&prepared, &transport, &CancellationToken::default(), time())
            .unwrap(),
        encoded.len()
    );
    assert_eq!(transport.drain(route).unwrap(), vec![encoded]);
}

#[test]
fn mailbox_epoch_has_one_owner_and_new_epoch_fences_previous_queue() {
    let c = config(MailboxMessageKind::Cast);
    let a = wire(&c, b"old", 30);
    let old = binding(c);
    let c = config(MailboxMessageKind::Cast);
    let b = wire(&c, b"second owner", 31);
    let other = binding(c);
    let budget = HostBudget::new(ServiceLimits::default()).unwrap();
    let mut first = LocalMailboxService::new(budget.clone()).unwrap();
    let mut second = LocalMailboxService::new(budget).unwrap();
    first.admit(&old, &a, time()).unwrap();
    assert_eq!(
        second.admit(&other, &b, time()).unwrap().receipt.outcome,
        DeliveryOutcome::Unavailable
    );
    let mut c = config(MailboxMessageKind::Cast);
    let mut context = c.policy.snapshot().unwrap();
    context.current_execution_epoch = ExecutionEpoch::new(8).unwrap();
    c.policy.replace(context).unwrap();
    c.boot = BootId::from_bytes([8; 32]);
    c.capability.grant.minimum_execution_epoch = ExecutionEpoch::new(8).unwrap();
    c.capability.grant.maximum_execution_epoch = ExecutionEpoch::new(8).unwrap();
    c.capability =
        mvm_core::mailbox::sign_capability(c.capability.grant, &SigningKey::from_bytes(&[22; 32]))
            .unwrap();
    c.binding.claims.execution_epoch = ExecutionEpoch::new(8).unwrap();
    c.binding = mvm_core::mailbox::transport::sign_transport_binding(
        c.binding.claims,
        &SigningKey::from_bytes(&[22; 32]),
    )
    .unwrap();
    let new_message = wire(&c, b"new", 32);
    let new_binding = binding(c);
    assert_eq!(
        second
            .admit(&new_binding, &new_message, time())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    assert!(first.receive(&old, time()).is_err());
    assert_eq!(
        second
            .receive(&new_binding, time())
            .unwrap()
            .unwrap()
            .payload(),
        b"new"
    );
}

#[test]
fn epoch_takeover_fences_retained_reply_handle_and_cached_reply() {
    use spiralcore_mvm_adapter::service::ServiceError;
    let c = config(MailboxMessageKind::Call);
    let request = wire(&c, b"question", 41);
    let pending = wire(&c, b"pending", 44);
    let old = binding(c);
    let budget = HostBudget::new(ServiceLimits::default()).unwrap();
    let mut first = LocalMailboxService::new(budget.clone()).unwrap();
    let mut second = LocalMailboxService::new(budget).unwrap();
    first.admit(&old, &request, time()).unwrap();
    let message = first.receive(&old, time()).unwrap().unwrap();
    let r = config(MailboxMessageKind::Reply);
    let seal = reply_request(&r, b"answer", 41, 42);
    let reply = binding(r);
    first
        .reply_prepared(&reply, message.handle(), seal, time())
        .unwrap();
    first.admit(&old, &pending, time()).unwrap();
    let pending_message = first.receive(&old, time()).unwrap().unwrap();
    let mut c = config(MailboxMessageKind::Call);
    let mut context = c.policy.snapshot().unwrap();
    context.current_execution_epoch = ExecutionEpoch::new(8).unwrap();
    c.policy.replace(context).unwrap();
    c.boot = BootId::from_bytes([8; 32]);
    c.capability.grant.minimum_execution_epoch = ExecutionEpoch::new(8).unwrap();
    c.capability.grant.maximum_execution_epoch = ExecutionEpoch::new(8).unwrap();
    c.capability =
        mvm_core::mailbox::sign_capability(c.capability.grant, &SigningKey::from_bytes(&[22; 32]))
            .unwrap();
    c.binding.claims.execution_epoch = ExecutionEpoch::new(8).unwrap();
    c.binding = mvm_core::mailbox::transport::sign_transport_binding(
        c.binding.claims,
        &SigningKey::from_bytes(&[22; 32]),
    )
    .unwrap();
    let next = wire(&c, b"new epoch", 43);
    let new = binding(c);
    second.admit(&new, &next, time()).unwrap();
    assert!(matches!(
        first.retry_reply_prepared(&old, &request, time()),
        Err(ServiceError::Unauthorized)
    ));
    let r = config(MailboxMessageKind::Reply);
    let seal = reply_request(&r, b"answer", 44, 45);
    assert!(matches!(
        first.reply_prepared(&reply, pending_message.handle(), seal, time()),
        Err(ServiceError::Unauthorized)
    ));
}

#[test]
fn fifo_gaps_block_only_their_scope_and_reused_sequence_is_refused() {
    let c = config(MailboxMessageKind::Cast);
    let future = wire_amend(&c, b"second", 33, |h| {
        h.ordering_scope = OrderingScope::parse("ordered").unwrap();
        h.sequence = SequenceNumber::new(2).unwrap();
    });
    let head = wire_amend(&c, b"first", 34, |h| {
        h.ordering_scope = OrderingScope::parse("ordered").unwrap();
        h.sequence = SequenceNumber::new(1).unwrap();
    });
    let independent = wire_amend(&c, b"independent", 35, |h| {
        h.ordering_scope = OrderingScope::parse("other").unwrap();
        h.sequence = SequenceNumber::new(1).unwrap();
    });
    let reused = wire_amend(&c, b"new id same seq", 36, |h| {
        h.ordering_scope = OrderingScope::parse("ordered").unwrap();
        h.sequence = SequenceNumber::new(1).unwrap();
    });
    let binding = binding(c);
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    assert_eq!(
        service
            .admit(&binding, &future, time())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    assert!(service.receive(&binding, time()).unwrap().is_none());
    service.admit(&binding, &independent, time()).unwrap();
    assert_eq!(
        service
            .receive(&binding, time())
            .unwrap()
            .unwrap()
            .payload(),
        b"independent"
    );
    service.admit(&binding, &head, time()).unwrap();
    assert_eq!(
        service
            .receive(&binding, time())
            .unwrap()
            .unwrap()
            .payload(),
        b"first"
    );
    assert_eq!(
        service
            .receive(&binding, time())
            .unwrap()
            .unwrap()
            .payload(),
        b"second"
    );
    assert_eq!(
        service
            .admit(&binding, &reused, time())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::Refused
    );
}

#[test]
fn reply_fifo_holds_future_response_until_the_known_scope_head_arrives() {
    use spiralcore_mvm_adapter::service::CallStatus;
    let c = config(MailboxMessageKind::Call);
    let first = wire(&c, b"first call", 37);
    let second = wire(&c, b"second call", 38);
    let caller =
        MailboxBinding::new(ServiceBindingConfig::from_direction(c).unwrap(), None).unwrap();
    let r = config(MailboxMessageKind::Reply);
    let future = wire_amend(&r, b"response two", 39, |h| {
        h.correlation_id = CorrelationId::from_bytes([38; 32]);
        h.causation_id = CausationId::from_bytes([38; 32]);
        h.ordering_scope = OrderingScope::parse("response-order").unwrap();
        h.sequence = SequenceNumber::new(2).unwrap();
    });
    let head = wire_amend(&r, b"response one", 40, |h| {
        h.correlation_id = CorrelationId::from_bytes([37; 32]);
        h.causation_id = CausationId::from_bytes([37; 32]);
        h.ordering_scope = OrderingScope::parse("response-order").unwrap();
        h.sequence = SequenceNumber::new(1).unwrap();
    });
    let replier = binding(r);
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let a = service.register_call(&caller, &first, time()).unwrap();
    let b = service.register_call(&caller, &second, time()).unwrap();
    service.admit(&replier, &future, time()).unwrap();
    assert_eq!(
        service.poll_call(&caller, &b, time()).unwrap(),
        CallStatus::Pending
    );
    service.admit(&replier, &head, time()).unwrap();
    assert_eq!(
        service.poll_call(&caller, &a, time()).unwrap(),
        CallStatus::Replied(Some(b"response one".to_vec()))
    );
    assert_eq!(
        service.poll_call(&caller, &b, time()).unwrap(),
        CallStatus::Replied(Some(b"response two".to_vec()))
    );
    assert!(service.receive(&replier, time()).unwrap().is_none());
}
