#![cfg(feature = "live")]
mod common;

use ed25519_dalek::SigningKey;
use mvm_contract::protocol::mailbox::*;
use mvm_core::mailbox::{sign_receipt, verify_receipt};
use spiralcore_mvm_adapter::{
    receipt::{
        MAX_RECEIPT_PAYLOAD_BYTES, ReceiptControlDisposition, ReceiptObservation,
        ReceiptReturnConfig, ReceiptTracker, decode_destination_receipt,
        encode_destination_receipt, receipt_content_type,
    },
    service::{
        ControlReserve, HostBudget, MailboxBinding, ServiceBindingConfig, ServiceError,
        ServiceLimits, ServiceTime,
    },
};

fn now() -> ServiceTime {
    ServiceTime {
        unix_ms: 1_500,
        monotonic_ms: 100,
    }
}
fn budget() -> HostBudget {
    HostBudget::with_control(ServiceLimits::default(), ControlReserve::default()).unwrap()
}
fn config(kind: MailboxMessageKind, control: bool) -> ServiceBindingConfig {
    let mut c = ServiceBindingConfig::from_direction(common::config(if control {
        MailboxMessageKind::Reply
    } else {
        kind
    }))
    .unwrap();
    c.kind = kind;
    c.context.required_operation = if kind == MailboxMessageKind::Reply {
        MailboxOperation::Reply
    } else {
        MailboxOperation::Send
    };
    c.capability.grant.operations =
        CapabilityOperations::from_operations(&[c.context.required_operation]);
    c.capability.grant.message_kinds = vec![kind];
    if control {
        c.context.accepted_content_types = vec![receipt_content_type()];
        c.capability.grant.content_types = vec![receipt_content_type()];
        c.capability.grant.max_payload_bytes = 4_096;
        c.capability.grant.max_queue_contribution_bytes = 16_384;
    }
    c.capability =
        mvm_core::mailbox::sign_capability(c.capability.grant, &SigningKey::from_bytes(&[22; 32]))
            .unwrap();
    c
}
struct Wire {
    bytes: Vec<u8>,
    header: MailboxEnvelopeHeader,
}
fn seal(
    c: &ServiceBindingConfig,
    payload: &[u8],
    message: u8,
    delivery: u8,
    causation: u8,
) -> Wire {
    use mvm_core::mailbox::{SealRequest, encode_envelope, seal_envelope};
    let ctx = &c.context;
    let header = MailboxEnvelopeHeader {
        protocol_version: ProtocolVersion::V1,
        signature_algorithm: SignatureAlgorithm::ED25519,
        tenant: ctx.expected_tenant,
        authorization_domain: ctx.expected_authorization_domain,
        sender: ctx.expected_sender,
        recipient: ctx.expected_recipient,
        mailbox: ctx.expected_mailbox,
        sender_boot: c.expected_sender_boot,
        execution_epoch: ctx.current_execution_epoch,
        message_id: MessageId::from_bytes([message; 32]),
        correlation_id: CorrelationId::from_bytes([90; 32]),
        causation_id: CausationId::from_bytes([causation; 32]),
        kind: c.kind,
        reply_to: if c.kind == MailboxMessageKind::Call {
            Some(ReplyRoute {
                address: MailboxAddress {
                    tenant: ctx.expected_tenant,
                    authorization_domain: ctx.expected_authorization_domain,
                    mailbox: MailboxId::from_bytes([14; 32]),
                    recipient: ctx.expected_sender,
                },
                capability_id: CapabilityId::from_bytes([11; 32]),
            })
        } else {
            None
        },
        ordering_scope: OrderingScope::parse("receipt-test").unwrap(),
        sequence: SequenceNumber::new(message as u64).unwrap(),
        issued_at_unix_ms: 1_000,
        expires_at_unix_ms: 2_000,
        nonce: ReplayNonce::from_bytes([message; 16]),
        content_type: c.capability.grant.content_types[0].clone(),
        capability_id: c.capability.grant.capability_id,
        policy_epoch: ctx.current_policy_epoch,
        encryption: EncryptionContext {
            algorithm: EncryptionAlgorithm::AES_256_GCM,
            key_id: c.encryption_key_id,
        },
        plaintext_len: payload.len() as u32,
    };
    let route = &c.binding.claims;
    let envelope = seal_envelope(
        SealRequest {
            routing: MailboxRoutingFrame {
                protocol_version: ProtocolVersion::V1,
                authorization_domain_route_label: route.authorization_domain_route_label,
                destination_route_token: route.destination_route_token,
                delivery_id: DeliveryId::from_bytes([delivery; 32]),
                ciphertext_len: 0,
                padding: CiphertextPadding::Exact,
                expires_at_unix_ms: 2_000,
                hop_limit: 8,
                transport_binding_id: route.binding_id,
            },
            header: header.clone(),
            plaintext: payload,
        },
        &c.encryption_key,
        c.signing_key.as_ref().unwrap(),
    )
    .unwrap();
    Wire {
        bytes: encode_envelope(&envelope).unwrap(),
        header,
    }
}
fn for_request(request: &Wire, outcome: DeliveryOutcome) -> SignedDestinationReceipt {
    let envelope = mvm_core::mailbox::decode_envelope(&request.bytes).unwrap();
    let mut receipt = signed_receipt().receipt;
    receipt.delivery_id = envelope.routing.delivery_id;
    receipt.message_id = request.header.message_id;
    receipt.address = MailboxAddress {
        tenant: request.header.tenant,
        authorization_domain: request.header.authorization_domain,
        mailbox: request.header.mailbox,
        recipient: request.header.recipient,
    };
    receipt.ciphertext_commitment = envelope.ciphertext_commitment;
    receipt.outcome = outcome;
    sign_receipt(receipt, &SigningKey::from_bytes(&[24; 32])).unwrap()
}

fn signed_receipt() -> SignedDestinationReceipt {
    sign_receipt(
        DestinationReceipt {
            protocol_version: ProtocolVersion::V1,
            signature_algorithm: SignatureAlgorithm::ED25519,
            delivery_id: DeliveryId::from_bytes([1; 32]),
            message_id: MessageId::from_bytes([2; 32]),
            address: MailboxAddress {
                tenant: TenantId::from_bytes([19; 32]),
                authorization_domain: AuthorizationDomainId::from_bytes([1; 32]),
                mailbox: MailboxId::from_bytes([4; 32]),
                recipient: PrincipalId::from_bytes([3; 32]),
            },
            execution_epoch: ExecutionEpoch::new(7).unwrap(),
            outcome: DeliveryOutcome::AcceptedEphemeral,
            observed_at_unix_ms: 1_500,
            ciphertext_commitment: [8; 32],
        },
        &SigningKey::from_bytes(&[24; 32]),
    )
    .unwrap()
}

#[test]
fn canonical_receipt_codec_preserves_existing_signature_and_rejects_malleable_input() {
    let signed = signed_receipt();
    let encoded = encode_destination_receipt(&signed).unwrap();
    assert!(encoded.len() <= MAX_RECEIPT_PAYLOAD_BYTES);
    assert_eq!(
        receipt_content_type(),
        ContentType::parse("application/vnd.mvm.mailbox.receipt", 1).unwrap()
    );
    let decoded = decode_destination_receipt(&encoded).unwrap();
    assert_eq!(decoded, signed);
    verify_receipt(&decoded, &SigningKey::from_bytes(&[24; 32]).verifying_key()).unwrap();
    assert_eq!(
        decode_destination_receipt(&[encoded.as_slice(), b" "].concat()),
        Err(ServiceError::Malformed)
    );
    assert_eq!(
        decode_destination_receipt(&vec![b' '; MAX_RECEIPT_PAYLOAD_BYTES + 1]),
        Err(ServiceError::Limit)
    );
    let mut value = serde_json::to_value(&signed).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("unexpected".into(), serde_json::json!(true));
    assert_eq!(
        decode_destination_receipt(&serde_jcs::to_vec(&value).unwrap()),
        Err(ServiceError::Malformed)
    );
    let mut malformed = signed.clone();
    malformed.signature.pop();
    assert_eq!(
        encode_destination_receipt(&malformed),
        Err(ServiceError::Malformed)
    );
}

#[test]
fn encrypted_receipts_match_attempts_and_do_not_recurse_or_lose_positive_facts() {
    let request_config = config(MailboxMessageKind::Call, false);
    let control_config = config(MailboxMessageKind::Cast, true);
    let request = seal(&request_config, b"exact object request", 1, 1, 0);
    let request_binding =
        MailboxBinding::new(config(MailboxMessageKind::Call, false), None).unwrap();
    let control_binding =
        MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap();
    let key = SigningKey::from_bytes(&[24; 32]).verifying_key();
    let mut tracker = ReceiptTracker::new(budget(), 4).unwrap();
    let handle = tracker
        .register_send(
            &request_binding,
            &control_binding,
            &key,
            &request.bytes,
            now(),
        )
        .unwrap();
    assert_eq!(
        tracker
            .take_receipt(&request_binding, &handle, now())
            .unwrap(),
        ReceiptObservation::Pending
    );
    let full = for_request(&request, DeliveryOutcome::Full);
    let full_wire = seal(
        &control_config,
        &encode_destination_receipt(&full).unwrap(),
        20,
        20,
        1,
    );
    assert_eq!(
        tracker
            .observe_receipt(&control_binding, &full_wire.bytes, now())
            .unwrap(),
        ReceiptControlDisposition::Observed
    );
    assert_eq!(
        tracker
            .take_receipt(&request_binding, &handle, now())
            .unwrap(),
        ReceiptObservation::Verified(Box::new(full.clone()))
    );
    let retry = seal(&request_config, b"exact object request", 1, 2, 0);
    assert_eq!(
        tracker
            .register_send(
                &request_binding,
                &control_binding,
                &key,
                &retry.bytes,
                now()
            )
            .unwrap(),
        handle
    );
    let accepted = for_request(&retry, DeliveryOutcome::AcceptedEphemeral);
    let accepted_wire = seal(
        &control_config,
        &encode_destination_receipt(&accepted).unwrap(),
        21,
        21,
        1,
    );
    assert!(
        !accepted_wire
            .bytes
            .windows(b"application/vnd.mvm.mailbox.receipt".len())
            .any(|w| w == b"application/vnd.mvm.mailbox.receipt")
    );
    tracker
        .observe_receipt(&control_binding, &accepted_wire.bytes, now())
        .unwrap();
    tracker
        .observe_receipt(&control_binding, &full_wire.bytes, now())
        .unwrap();
    assert!(
        tracker
            .facts(&request_binding, &handle, now())
            .unwrap()
            .accepted
    );
    assert_eq!(
        tracker
            .take_receipt(&request_binding, &handle, now())
            .unwrap(),
        ReceiptObservation::Verified(Box::new(accepted))
    );
    assert_eq!(
        tracker
            .observe_receipt(&control_binding, &accepted_wire.bytes, now())
            .unwrap(),
        ReceiptControlDisposition::Duplicate
    );
    assert_eq!(
        tracker
            .take_receipt(&request_binding, &handle, now())
            .unwrap(),
        ReceiptObservation::Pending
    );
}

#[test]
fn receipt_tracker_rejects_wrong_signature_commitment_causation_and_policy_generation() {
    let request_config = config(MailboxMessageKind::Call, false);
    let control_config = config(MailboxMessageKind::Cast, true);
    let request = seal(&request_config, b"request", 1, 1, 0);
    let request_binding =
        MailboxBinding::new(config(MailboxMessageKind::Call, false), None).unwrap();
    let control_binding =
        MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap();
    let mut tracker = ReceiptTracker::new(budget(), 2).unwrap();
    let handle = tracker
        .register_send(
            &request_binding,
            &control_binding,
            &SigningKey::from_bytes(&[24; 32]).verifying_key(),
            &request.bytes,
            now(),
        )
        .unwrap();
    let signed = for_request(&request, DeliveryOutcome::AcceptedEphemeral);
    let mut bad = signed.clone();
    bad.signature[0] ^= 1;
    let wire = seal(
        &control_config,
        &encode_destination_receipt(&bad).unwrap(),
        20,
        20,
        1,
    );
    assert!(
        tracker
            .observe_receipt(&control_binding, &wire.bytes, now())
            .is_err()
    );
    let mut bad = signed.receipt.clone();
    bad.ciphertext_commitment[0] ^= 1;
    let bad = sign_receipt(bad, &SigningKey::from_bytes(&[24; 32])).unwrap();
    let wire = seal(
        &control_config,
        &encode_destination_receipt(&bad).unwrap(),
        21,
        21,
        1,
    );
    assert!(
        tracker
            .observe_receipt(&control_binding, &wire.bytes, now())
            .is_err()
    );
    let wire = seal(
        &control_config,
        &encode_destination_receipt(&signed).unwrap(),
        22,
        22,
        9,
    );
    assert!(
        tracker
            .observe_receipt(&control_binding, &wire.bytes, now())
            .is_err()
    );
    let context = request_config.context.clone();
    request_binding.replace_policy(context).unwrap();
    assert!(
        tracker
            .take_receipt(&request_binding, &handle, now())
            .is_err()
    );
}

#[test]
fn application_reply_and_admission_are_independent_and_late_replies_cannot_complete() {
    let request_config = config(MailboxMessageKind::Call, false);
    let control_config = config(MailboxMessageKind::Cast, true);
    let request = seal(&request_config, b"request", 1, 1, 0);
    let request_binding =
        MailboxBinding::new(config(MailboxMessageKind::Call, false), None).unwrap();
    let control_binding =
        MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap();
    let reply_binding =
        MailboxBinding::new(config(MailboxMessageKind::Reply, false), None).unwrap();
    let mut tracker = ReceiptTracker::new(budget(), 2).unwrap();
    let handle = tracker
        .register_send(
            &request_binding,
            &control_binding,
            &SigningKey::from_bytes(&[24; 32]).verifying_key(),
            &request.bytes,
            now(),
        )
        .unwrap();
    let reply = seal(
        &config(MailboxMessageKind::Reply, false),
        b"exact result",
        30,
        30,
        1,
    );
    assert_eq!(
        tracker
            .observe_application_reply(&reply_binding, &reply.bytes, now())
            .unwrap(),
        handle
    );
    let facts = tracker.facts(&request_binding, &handle, now()).unwrap();
    assert!(facts.application_replied);
    assert!(!facts.accepted);
    let signed = for_request(&request, DeliveryOutcome::AcceptedEphemeral);
    let wire = seal(
        &control_config,
        &encode_destination_receipt(&signed).unwrap(),
        20,
        20,
        1,
    );
    tracker
        .observe_receipt(&control_binding, &wire.bytes, now())
        .unwrap();
    assert!(
        tracker
            .facts(&request_binding, &handle, now())
            .unwrap()
            .accepted
    );
    assert_eq!(
        tracker
            .observe_application_reply(&reply_binding, &reply.bytes, now())
            .unwrap(),
        handle
    );
    let mut other = ReceiptTracker::new(budget(), 2).unwrap();
    other
        .register_send(
            &request_binding,
            &control_binding,
            &SigningKey::from_bytes(&[24; 32]).verifying_key(),
            &request.bytes,
            now(),
        )
        .unwrap();
    let late = ServiceTime {
        unix_ms: 1_900,
        monotonic_ms: 601,
    };
    assert_eq!(
        other.observe_application_reply(&reply_binding, &reply.bytes, late),
        Err(ServiceError::Expired)
    );
}

#[test]
fn receipt_return_reserves_before_admission_and_sends_only_canonical_encrypted_cast() {
    use mvm_core::mailbox::transport::{CancellationToken, InMemoryPeerTransport};
    let request = seal(
        &config(MailboxMessageKind::Call, false),
        b"request",
        1,
        1,
        0,
    );
    let source_binding =
        MailboxBinding::new(config(MailboxMessageKind::Call, false), None).unwrap();
    let return_binding = MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap();
    let route = config(MailboxMessageKind::Cast, true)
        .binding
        .claims
        .route_id;
    let returns = ReceiptReturnConfig::new(
        return_binding,
        budget(),
        SigningKey::from_bytes(&[24; 32]).verifying_key(),
    )
    .unwrap();
    let permit = returns
        .reserve(&source_binding, &request.bytes, now())
        .unwrap();
    let prepared = permit
        .seal(
            &for_request(&request, DeliveryOutcome::AcceptedEphemeral),
            now(),
        )
        .unwrap();
    let transport = InMemoryPeerTransport::default();
    transport.register_route(route).unwrap();
    let count = prepared
        .send(&transport, &CancellationToken::default(), now())
        .unwrap();
    let wire = transport.drain(route).unwrap().pop().unwrap();
    assert_eq!(count, wire.len());
    assert_eq!(wire, prepared.encoded_bytes());
    assert!(!wire.windows(20).any(|w| w == b"accepted_ephemeral\"}"));
    let receiver = MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap();
    let request_binding =
        MailboxBinding::new(config(MailboxMessageKind::Call, false), None).unwrap();
    let mut tracker = ReceiptTracker::new(budget(), 1).unwrap();
    let handle = tracker
        .register_send(
            &request_binding,
            &receiver,
            &SigningKey::from_bytes(&[24; 32]).verifying_key(),
            &request.bytes,
            now(),
        )
        .unwrap();
    tracker.observe_receipt(&receiver, &wire, now()).unwrap();
    assert!(
        tracker
            .facts(&request_binding, &handle, now())
            .unwrap()
            .accepted
    );
    // Retrying a prepared send reuses its exact envelope and message identity.
    prepared
        .send(&transport, &CancellationToken::default(), now())
        .unwrap();
    let wire2 = transport.drain(route).unwrap().pop().unwrap();
    assert_eq!(wire, wire2);
    assert_eq!(
        tracker.observe_receipt(&receiver, &wire2, now()).unwrap(),
        ReceiptControlDisposition::Duplicate
    );
    // Receipt control is infrastructure only: it can never ask for another receipt.
    assert!(returns.reserve(&receiver, &wire, now()).is_err());
}

#[test]
fn attempts_and_shared_identity_quotas_fail_closed_and_release_owned_capacity() {
    let source = config(MailboxMessageKind::Cast, false);
    let original = MailboxBinding::new(config(MailboxMessageKind::Cast, false), None).unwrap();
    let control = MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap();
    let key = SigningKey::from_bytes(&[24; 32]).verifying_key();
    assert!(matches!(
        ReceiptTracker::new(HostBudget::new(ServiceLimits::default()).unwrap(), 1),
        Err(ServiceError::Limit)
    ));
    let mut reserve = ControlReserve::default();
    reserve.sender.identities = 1;
    let shared = HostBudget::with_control(ServiceLimits::default(), reserve).unwrap();
    let mut first = ReceiptTracker::new(shared.clone(), 4).unwrap();
    let mut second = ReceiptTracker::new(shared.clone(), 4).unwrap();
    for delivery in 1..=8 {
        first
            .register_send(
                &original,
                &control,
                &key,
                &seal(&source, b"same", 1, delivery, 0).bytes,
                now(),
            )
            .unwrap();
    }
    assert_eq!(
        first.register_send(
            &original,
            &control,
            &key,
            &seal(&source, b"same", 1, 9, 0).bytes,
            now()
        ),
        Err(ServiceError::Limit)
    );
    let another = seal(&source, b"another", 2, 2, 0);
    assert_eq!(
        second.register_send(&original, &control, &key, &another.bytes, now()),
        Err(ServiceError::Full)
    );
    {
        use spiralcore_mvm_adapter::service::{BudgetPool, BudgetResource, BudgetScope};
        let refusal = shared.snapshot().unwrap().refusals.last.unwrap();
        assert_eq!(refusal.scope, BudgetScope::Sender);
        assert_eq!(refusal.pool, BudgetPool::Control);
        assert_eq!(refusal.resource, BudgetResource::Identities);
        assert_eq!(
            (refusal.current, refusal.requested, refusal.limit),
            (1, 1, 1)
        );
        assert_eq!(first.snapshot().retained_entries, 1);
        assert_eq!(first.snapshot().attempts, 8);
        assert_eq!(first.snapshot().next_expiry_monotonic_ms, Some(300_600));
        assert_eq!(second.snapshot().retained_entries, 0);
    }
    drop(first);
    second
        .register_send(&original, &control, &key, &another.bytes, now())
        .unwrap();
}

#[test]
fn tracker_local_capacity_diagnostic_preserves_state_and_expires_at_exact_retention() {
    use spiralcore_mvm_adapter::service::{BudgetPool, BudgetResource, BudgetScope};
    let source = config(MailboxMessageKind::Cast, false);
    let original = MailboxBinding::new(config(MailboxMessageKind::Cast, false), None).unwrap();
    let control = MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap();
    let shared = budget();
    let mut tracker = ReceiptTracker::new(shared.clone(), 1).unwrap();
    let key = SigningKey::from_bytes(&[24; 32]).verifying_key();
    let first = seal(&source, b"first\0\0", 1, 1, 0);
    let second = seal(&source, b"second\0\0", 2, 2, 0);
    let handle = tracker
        .register_send(&original, &control, &key, &first.bytes, now())
        .unwrap();
    let before = shared.snapshot().unwrap();
    assert_eq!(
        tracker.register_send(&original, &control, &key, &second.bytes, now()),
        Err(ServiceError::Full)
    );
    let snapshot = tracker.snapshot();
    assert_eq!(
        (
            snapshot.retained_entries,
            snapshot.request_indices,
            snapshot.attempts,
            snapshot.max_pending
        ),
        (1, 1, 1, 1)
    );
    let reason = shared.snapshot().unwrap().refusals.last.unwrap();
    assert_eq!(
        (reason.scope, reason.pool, reason.resource),
        (
            BudgetScope::Tracker,
            BudgetPool::Local,
            BudgetResource::TrackerEntries
        )
    );
    assert_eq!((reason.current, reason.requested, reason.limit), (1, 1, 1));
    assert_eq!(shared.snapshot().unwrap().identities, before.identities);
    // Retry of the exact send remains idempotent even at the local ceiling.
    assert_eq!(
        tracker
            .register_send(&original, &control, &key, &first.bytes, now())
            .unwrap(),
        handle
    );
    let expiry = snapshot.next_expiry_monotonic_ms.unwrap();
    let mut later = now();
    later.monotonic_ms = expiry - 1;
    assert_eq!(tracker.expire(later).unwrap(), 0);
    assert_eq!(tracker.snapshot().retained_entries, 1);
    later.monotonic_ms = expiry;
    assert_eq!(tracker.expire(later).unwrap(), 1);
    assert_eq!(tracker.snapshot().retained_entries, 0);
    assert_eq!(shared.snapshot().unwrap().identities, 0);
    assert!(tracker.facts(&original, &handle, later).is_err());
    // A new authenticated request can use released capacity without rebuilding tracker.
    // Keep synthetic wall time in its signed validity interval; monotonic time
    // independently drives retention here. Real-time qualification is separate.
    tracker
        .register_send(&original, &control, &key, &second.bytes, later)
        .unwrap();
    assert_eq!(tracker.snapshot().retained_entries, 1);
}

#[test]
fn full_receipt_consumer_queue_can_retry_after_consumption_without_false_duplicate() {
    let source = config(MailboxMessageKind::Cast, false);
    let original = MailboxBinding::new(config(MailboxMessageKind::Cast, false), None).unwrap();
    let control = MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap();
    let control_config = config(MailboxMessageKind::Cast, true);
    let key = SigningKey::from_bytes(&[24; 32]).verifying_key();
    let mut reserve = ControlReserve::default();
    reserve.sender.queued_messages = 1;
    let shared = HostBudget::with_control(ServiceLimits::default(), reserve).unwrap();
    let mut tracker = ReceiptTracker::new(shared, 4).unwrap();
    let first = seal(&source, b"first", 1, 1, 0);
    let second = seal(&source, b"second", 2, 2, 0);
    let h1 = tracker
        .register_send(&original, &control, &key, &first.bytes, now())
        .unwrap();
    let h2 = tracker
        .register_send(&original, &control, &key, &second.bytes, now())
        .unwrap();
    let a = seal(
        &control_config,
        &encode_destination_receipt(&for_request(&first, DeliveryOutcome::AcceptedEphemeral))
            .unwrap(),
        21,
        21,
        1,
    );
    let b = seal(
        &control_config,
        &encode_destination_receipt(&for_request(&second, DeliveryOutcome::AcceptedEphemeral))
            .unwrap(),
        22,
        22,
        2,
    );
    tracker.observe_receipt(&control, &a.bytes, now()).unwrap();
    assert_eq!(
        tracker.observe_receipt(&control, &b.bytes, now()),
        Err(ServiceError::Full)
    );
    assert!(!tracker.facts(&original, &h2, now()).unwrap().accepted);
    assert!(matches!(
        tracker.take_receipt(&original, &h1, now()).unwrap(),
        ReceiptObservation::Verified(_)
    ));
    assert_eq!(
        tracker.observe_receipt(&control, &b.bytes, now()).unwrap(),
        ReceiptControlDisposition::Observed
    );
    assert!(tracker.facts(&original, &h2, now()).unwrap().accepted);
}

#[test]
fn ordinary_reply_has_destination_receipt_without_reply_of_reply() {
    let source = config(MailboxMessageKind::Reply, false);
    let original = MailboxBinding::new(config(MailboxMessageKind::Reply, false), None).unwrap();
    let mut incoming = config(MailboxMessageKind::Cast, false);
    incoming.context.accepted_content_types = vec![receipt_content_type()];
    incoming.capability.grant.content_types = vec![receipt_content_type()];
    incoming.capability.grant.max_payload_bytes = 4_096;
    incoming.capability.grant.max_queue_contribution_bytes = 16_384;
    incoming.capability = mvm_core::mailbox::sign_capability(
        incoming.capability.grant,
        &SigningKey::from_bytes(&[22; 32]),
    )
    .unwrap();
    let wire = seal(&source, b"application reply", 30, 30, 1);
    let control_wire_config = config(MailboxMessageKind::Cast, false);
    let mut control_wire_config = ServiceBindingConfig {
        context: incoming.context.clone(),
        capability: incoming.capability.clone(),
        ..control_wire_config
    };
    control_wire_config.capability = incoming.capability.clone();
    let control = MailboxBinding::new(incoming, None).unwrap();
    let key = SigningKey::from_bytes(&[21; 32]);
    let mut tracker = ReceiptTracker::new(budget(), 2).unwrap();
    let handle = tracker
        .register_send(
            &original,
            &control,
            &key.verifying_key(),
            &wire.bytes,
            now(),
        )
        .unwrap();
    let signed = sign_receipt(
        for_request(&wire, DeliveryOutcome::AcceptedEphemeral).receipt,
        &key,
    )
    .unwrap();
    let carrier = seal(
        &control_wire_config,
        &encode_destination_receipt(&signed).unwrap(),
        31,
        31,
        30,
    );
    tracker
        .observe_receipt(&control, &carrier.bytes, now())
        .unwrap();
    let facts = tracker.facts(&original, &handle, now()).unwrap();
    assert!(facts.accepted);
    assert!(!facts.application_replied);
}

#[test]
fn prepared_receipt_refuses_backward_clock_and_policy_change_before_transport() {
    use mvm_core::mailbox::transport::{CancellationToken, InMemoryPeerTransport};
    let request = seal(
        &config(MailboxMessageKind::Call, false),
        b"request",
        1,
        1,
        0,
    );
    let source = MailboxBinding::new(config(MailboxMessageKind::Call, false), None).unwrap();
    let route = config(MailboxMessageKind::Cast, true)
        .binding
        .claims
        .route_id;
    let returns = ReceiptReturnConfig::new(
        MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap(),
        budget(),
        SigningKey::from_bytes(&[24; 32]).verifying_key(),
    )
    .unwrap();
    let signed = for_request(&request, DeliveryOutcome::AcceptedEphemeral);
    let mut backward = now();
    backward.monotonic_ms -= 1;
    assert!(matches!(
        returns
            .reserve(&source, &request.bytes, now())
            .unwrap()
            .seal(&signed, backward),
        Err(ServiceError::Clock)
    ));
    let prepared = returns
        .reserve(&source, &request.bytes, now())
        .unwrap()
        .seal(&signed, now())
        .unwrap();
    let transport = InMemoryPeerTransport::default();
    transport.register_route(route).unwrap();
    assert!(matches!(
        prepared.send(&transport, &CancellationToken::default(), backward),
        Err(spiralcore_mvm_adapter::receipt::ReceiptSendError::Service(
            ServiceError::Clock
        ))
    ));
    source
        .replace_policy(config(MailboxMessageKind::Call, false).context)
        .unwrap();
    assert!(
        prepared
            .send(&transport, &CancellationToken::default(), now())
            .is_err()
    );
    assert!(transport.drain(route).unwrap().is_empty());
}

#[test]
fn signed_call_limit_is_reported_separately_from_host_quota() {
    use spiralcore_mvm_adapter::service::{
        BudgetPool, BudgetResource, BudgetScope, LocalMailboxService,
    };
    use spiralcore_mvm_adapter::service_sender::{self, MessageOptions};
    let mut source = config(MailboxMessageKind::Call, false);
    source.capability.grant.max_outstanding_calls = 1;
    source.capability = mvm_core::mailbox::sign_capability(
        source.capability.grant,
        &SigningKey::from_bytes(&[22; 32]),
    )
    .unwrap();
    let content_type = source.capability.grant.content_types[0].clone();
    let reply_to = seal(&source, b"route fixture", 1, 1, 1).header.reply_to;
    let producer = MailboxBinding::new(source, None).unwrap();
    let shared = budget();
    let mut service = LocalMailboxService::new(shared.clone()).unwrap();
    let request = |payload: &[u8]| {
        let mut options = MessageOptions::fresh(content_type.clone(), now(), 2000).unwrap();
        options.reply_to = reply_to;
        let prepared = service_sender::prepare(&producer, payload, &options, now()).unwrap();
        mvm_core::mailbox::encode_envelope(prepared.envelope()).unwrap()
    };
    let first = request(b"first");
    let second = request(b"second");
    service.register_call(&producer, &first, now()).unwrap();
    // Synthetic fixture signs one active Call per grant; the host allows more.
    assert_eq!(
        service.register_call(&producer, &second, now()),
        Err(ServiceError::Full)
    );
    let reason = shared.snapshot().unwrap().refusals.last.unwrap();
    assert_eq!(
        (reason.scope, reason.pool, reason.resource),
        (
            BudgetScope::Capability,
            BudgetPool::Signed,
            BudgetResource::ActiveCalls
        )
    );
    assert_eq!((reason.current, reason.requested, reason.limit), (1, 1, 1));
    assert_eq!(shared.snapshot().unwrap().active_calls, 1);
}

#[test]
fn generic_application_admission_cannot_acknowledge_reserved_receipt_control() {
    use spiralcore_mvm_adapter::service::{DestinationReceiptSigner, LocalMailboxService};
    let control_config = config(MailboxMessageKind::Cast, true);
    let control = MailboxBinding::new(
        config(MailboxMessageKind::Cast, true),
        Some(DestinationReceiptSigner {
            recipient: control_config.context.expected_recipient,
            signing_key: SigningKey::from_bytes(&[21; 32]),
        }),
    )
    .unwrap();
    let wire = seal(
        &control_config,
        &encode_destination_receipt(&signed_receipt()).unwrap(),
        22,
        22,
        1,
    );
    let mut service = LocalMailboxService::new(budget()).unwrap();
    assert_eq!(
        service.admit(&control, &wire.bytes, now()),
        Err(ServiceError::Unauthorized)
    );
}

#[test]
fn integrated_admission_owns_receipt_credit_before_accepting_and_retries_same_message() {
    use spiralcore_mvm_adapter::service::{DestinationReceiptSigner, LocalMailboxService};
    let source_config = config(MailboxMessageKind::Cast, false);
    let source = MailboxBinding::new(
        config(MailboxMessageKind::Cast, false),
        Some(DestinationReceiptSigner {
            recipient: source_config.context.expected_recipient,
            signing_key: SigningKey::from_bytes(&[24; 32]),
        }),
    )
    .unwrap();
    let request = seal(&source_config, b"application value", 1, 1, 0);
    let mut reserve = ControlReserve::default();
    reserve.sender.queued_messages = 1;
    let shared = HostBudget::with_control(ServiceLimits::default(), reserve).unwrap();
    let returns = ReceiptReturnConfig::new(
        MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap(),
        shared.clone(),
        SigningKey::from_bytes(&[24; 32]).verifying_key(),
    )
    .unwrap();
    let mut service = LocalMailboxService::new(shared).unwrap();
    let occupied = returns.reserve(&source, &request.bytes, now()).unwrap();
    assert!(matches!(
        returns.admit(&mut service, &source, &request.bytes, now()),
        Err(ServiceError::Full)
    ));
    assert!(service.receive(&source, now()).unwrap().is_none());
    drop(occupied);
    let (receipt, _prepared) = returns
        .admit(&mut service, &source, &request.bytes, now())
        .unwrap();
    assert_eq!(receipt.receipt.outcome, DeliveryOutcome::AcceptedEphemeral);
    assert_eq!(
        service.receive(&source, now()).unwrap().unwrap().payload(),
        b"application value"
    );
}

#[test]
fn delayed_negative_same_attempt_cannot_republish_consumed_positive_receipt() {
    let source = config(MailboxMessageKind::Cast, false);
    let request = seal(&source, b"request", 1, 1, 0);
    let original = MailboxBinding::new(config(MailboxMessageKind::Cast, false), None).unwrap();
    let control = MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap();
    let control_config = config(MailboxMessageKind::Cast, true);
    let mut tracker = ReceiptTracker::new(budget(), 1).unwrap();
    let handle = tracker
        .register_send(
            &original,
            &control,
            &SigningKey::from_bytes(&[24; 32]).verifying_key(),
            &request.bytes,
            now(),
        )
        .unwrap();
    let full = seal(
        &control_config,
        &encode_destination_receipt(&for_request(&request, DeliveryOutcome::Full)).unwrap(),
        20,
        20,
        1,
    );
    let accepted = seal(
        &control_config,
        &encode_destination_receipt(&for_request(&request, DeliveryOutcome::AcceptedEphemeral))
            .unwrap(),
        21,
        21,
        1,
    );
    tracker
        .observe_receipt(&control, &full.bytes, now())
        .unwrap();
    tracker
        .observe_receipt(&control, &accepted.bytes, now())
        .unwrap();
    assert!(matches!(
        tracker.take_receipt(&original, &handle, now()).unwrap(),
        ReceiptObservation::Verified(_)
    ));
    tracker
        .observe_receipt(&control, &full.bytes, now())
        .unwrap();
    assert_eq!(
        tracker
            .observe_receipt(&control, &accepted.bytes, now())
            .unwrap(),
        ReceiptControlDisposition::Duplicate
    );
    assert_eq!(
        tracker.take_receipt(&original, &handle, now()).unwrap(),
        ReceiptObservation::Pending
    );
}

#[test]
fn receipt_replay_and_prepared_resend_obey_signed_control_rate_without_ack_loops() {
    use mvm_core::mailbox::transport::{CancellationToken, InMemoryPeerTransport};
    let source_cfg = config(MailboxMessageKind::Cast, false);
    let source = MailboxBinding::new(config(MailboxMessageKind::Cast, false), None).unwrap();
    let request = seal(&source_cfg, b"request", 1, 1, 0);
    let mut control_cfg = config(MailboxMessageKind::Cast, true);
    control_cfg.capability.grant.max_messages_per_minute = 1;
    control_cfg.capability = mvm_core::mailbox::sign_capability(
        control_cfg.capability.grant,
        &SigningKey::from_bytes(&[22; 32]),
    )
    .unwrap();
    let wire = seal(
        &control_cfg,
        &encode_destination_receipt(&for_request(&request, DeliveryOutcome::AcceptedEphemeral))
            .unwrap(),
        20,
        20,
        1,
    );
    let route = control_cfg.binding.claims.route_id;
    let mut return_cfg = config(MailboxMessageKind::Cast, true);
    return_cfg.capability = control_cfg.capability.clone();
    let control = MailboxBinding::new(control_cfg, None).unwrap();
    let shared = budget();
    let mut tracker = ReceiptTracker::new(shared.clone(), 1).unwrap();
    let handle = tracker
        .register_send(
            &source,
            &control,
            &SigningKey::from_bytes(&[24; 32]).verifying_key(),
            &request.bytes,
            now(),
        )
        .unwrap();
    tracker
        .observe_receipt(&control, &wire.bytes, now())
        .unwrap();
    tracker.take_receipt(&source, &handle, now()).unwrap();
    assert_eq!(
        tracker.observe_receipt(&control, &wire.bytes, now()),
        Err(ServiceError::Backpressured)
    );
    {
        use spiralcore_mvm_adapter::service::{BudgetPool, BudgetResource, BudgetScope};
        let reason = shared.snapshot().unwrap().refusals.last.unwrap();
        assert_eq!(
            (reason.scope, reason.pool, reason.resource),
            (
                BudgetScope::Capability,
                BudgetPool::Signed,
                BudgetResource::MessagesPerMinute
            )
        );
        assert_eq!((reason.current, reason.requested, reason.limit), (1, 1, 1));
    }
    let returns = ReceiptReturnConfig::new(
        MailboxBinding::new(return_cfg, None).unwrap(),
        budget(),
        SigningKey::from_bytes(&[24; 32]).verifying_key(),
    )
    .unwrap();
    let prepared = returns
        .reserve(&source, &request.bytes, now())
        .unwrap()
        .seal(
            &for_request(&request, DeliveryOutcome::AcceptedEphemeral),
            now(),
        )
        .unwrap();
    let transport = InMemoryPeerTransport::default();
    transport.register_route(route).unwrap();
    prepared
        .send(&transport, &CancellationToken::default(), now())
        .unwrap();
    transport.drain(route).unwrap();
    assert!(matches!(
        prepared.send(&transport, &CancellationToken::default(), now()),
        Err(spiralcore_mvm_adapter::receipt::ReceiptSendError::Service(
            ServiceError::Backpressured
        ))
    ));
    assert!(transport.drain(route).unwrap().is_empty());
    // Consumed positive receipts retain their terminal identity until expiry.
    let retained = tracker.snapshot();
    assert_eq!(retained.accepted_entries, 1);
    assert_eq!(retained.pending_receipts, 0);
    let boundary = retained.next_expiry_monotonic_ms.unwrap();
    let mut later = now();
    later.monotonic_ms = boundary - 1;
    assert_eq!(tracker.expire(later).unwrap(), 0);
    later.monotonic_ms = boundary;
    assert_eq!(tracker.expire(later).unwrap(), 1);
    assert_eq!(tracker.snapshot().retained_entries, 0);
}

#[test]
fn receipt_return_rejects_undersized_reverse_grants_before_application_admission() {
    use spiralcore_mvm_adapter::service::{DestinationReceiptSigner, LocalMailboxService};
    let source_config = config(MailboxMessageKind::Cast, false);
    let source = MailboxBinding::new(
        config(MailboxMessageKind::Cast, false),
        Some(DestinationReceiptSigner {
            recipient: source_config.context.expected_recipient,
            signing_key: SigningKey::from_bytes(&[24; 32]),
        }),
    )
    .unwrap();
    let mut service = LocalMailboxService::new(budget()).unwrap();
    for undersized_payload in [true, false] {
        let mut reverse = config(MailboxMessageKind::Cast, true);
        if undersized_payload {
            reverse.capability.grant.max_payload_bytes = 1;
        } else {
            reverse.capability.grant.max_queue_contribution_bytes = 1;
        }
        reverse.capability = mvm_core::mailbox::sign_capability(
            reverse.capability.grant,
            &SigningKey::from_bytes(&[22; 32]),
        )
        .unwrap();
        let returns = ReceiptReturnConfig::new(
            MailboxBinding::new(reverse, None).unwrap(),
            budget(),
            SigningKey::from_bytes(&[24; 32]).verifying_key(),
        );
        assert!(
            matches!(returns, Err(ServiceError::Unauthorized)),
            "undersized reverse receipt grant must fail at configuration time"
        );
        assert!(service.receive(&source, now()).unwrap().is_none());
    }
}

#[test]
fn integrated_receipt_admission_rejects_a_different_budget_owner_before_queue_mutation() {
    use spiralcore_mvm_adapter::service::{DestinationReceiptSigner, LocalMailboxService};
    let source_config = config(MailboxMessageKind::Cast, false);
    let source = MailboxBinding::new(
        config(MailboxMessageKind::Cast, false),
        Some(DestinationReceiptSigner {
            recipient: source_config.context.expected_recipient,
            signing_key: SigningKey::from_bytes(&[24; 32]),
        }),
    )
    .unwrap();
    let request = seal(&source_config, b"application value", 1, 1, 0);
    let returns = ReceiptReturnConfig::new(
        MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap(),
        budget(),
        SigningKey::from_bytes(&[24; 32]).verifying_key(),
    )
    .unwrap();
    let mut service = LocalMailboxService::new(budget()).unwrap();
    assert!(matches!(
        returns.admit(&mut service, &source, &request.bytes, now()),
        Err(ServiceError::Unauthorized)
    ));
    assert!(service.receive(&source, now()).unwrap().is_none());
    // Rejection did not consume replay/dedup state or admit the application.
    let receipt = service.admit(&source, &request.bytes, now()).unwrap();
    assert_eq!(receipt.receipt.outcome, DeliveryOutcome::AcceptedEphemeral);
}

// Keep the existing signed fixture and canonical envelope format; change only
// the correlation value before re-signing a Reply with its real fixture key.
fn reply_with_correlation(payload: &[u8], correlation: u8, causation: u8) -> Wire {
    use mvm_core::mailbox::{SealRequest, decode_envelope, encode_envelope, seal_envelope};
    let c = config(MailboxMessageKind::Reply, false);
    let mut reply = seal(&c, payload, 30, 30, causation);
    reply.header.correlation_id = CorrelationId::from_bytes([correlation; 32]);
    let original = decode_envelope(&reply.bytes).unwrap();
    let envelope = seal_envelope(
        SealRequest {
            routing: original.routing,
            header: reply.header.clone(),
            plaintext: payload,
        },
        &c.encryption_key,
        c.signing_key.as_ref().unwrap(),
    )
    .unwrap();
    reply.bytes = encode_envelope(&envelope).unwrap();
    reply
}

#[test]
fn registered_reply_observer_matches_before_partial_candidates_without_inventing_acceptance() {
    let request_binding =
        MailboxBinding::new(config(MailboxMessageKind::Call, false), None).unwrap();
    let control = MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap();
    let replies = MailboxBinding::new(config(MailboxMessageKind::Reply, false), None).unwrap();
    let mut tracker = ReceiptTracker::new(budget(), 4).unwrap();
    let key = SigningKey::from_bytes(&[24; 32]).verifying_key();
    let first = seal(&config(MailboxMessageKind::Call, false), b"first", 1, 1, 0);
    let other = seal(&config(MailboxMessageKind::Call, false), b"other", 2, 2, 0);
    let first_handle = tracker
        .register_send(&request_binding, &control, &key, &first.bytes, now())
        .unwrap();
    let other_handle = tracker
        .register_send(&request_binding, &control, &key, &other.bytes, now())
        .unwrap();
    // Both requests share correlation 90. The second partial candidate must not
    // veto the complete match for the first request's causation/message ID.
    let reply = seal(
        &config(MailboxMessageKind::Reply, false),
        b"result",
        30,
        30,
        1,
    );
    assert_eq!(
        tracker.observe_registered_application_reply(&replies, &reply.bytes, now()),
        Ok(Some(first_handle))
    );
    assert_eq!(
        tracker.observe_application_reply(&replies, &reply.bytes, now()),
        Ok(first_handle)
    );
    let facts = tracker
        .facts(&request_binding, &first_handle, now())
        .unwrap();
    assert!(facts.application_replied);
    assert!(!facts.accepted);
    let other_facts = tracker
        .facts(&request_binding, &other_handle, now())
        .unwrap();
    assert!(!other_facts.application_replied && !other_facts.accepted);
    assert_eq!(
        tracker
            .take_receipt(&request_binding, &first_handle, now())
            .unwrap(),
        ReceiptObservation::Pending
    );
}

#[test]
fn registered_reply_observer_unknown_is_none_without_retained_state_and_strict_stays_unauthorized()
{
    let request_binding =
        MailboxBinding::new(config(MailboxMessageKind::Call, false), None).unwrap();
    let control = MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap();
    let replies = MailboxBinding::new(config(MailboxMessageKind::Reply, false), None).unwrap();
    let shared = budget();
    let mut tracker = ReceiptTracker::new(shared.clone(), 2).unwrap();
    let request = seal(&config(MailboxMessageKind::Call, false), b"known", 1, 1, 0);
    let handle = tracker
        .register_send(
            &request_binding,
            &control,
            &SigningKey::from_bytes(&[24; 32]).verifying_key(),
            &request.bytes,
            now(),
        )
        .unwrap();
    let unknown = reply_with_correlation(b"unknown", 91, 99);
    let before = tracker.snapshot();
    let budget_before = shared.snapshot().unwrap();
    assert_eq!(
        tracker.observe_registered_application_reply(&replies, &unknown.bytes, now()),
        Ok(None)
    );
    assert_eq!(
        tracker.observe_application_reply(&replies, &unknown.bytes, now()),
        Err(ServiceError::Unauthorized)
    );
    assert_eq!(tracker.snapshot(), before);
    let after = shared.snapshot().unwrap();
    assert_eq!(
        (
            after.identities,
            after.queued_bytes,
            after.queued_messages,
            after.active_calls,
            after.owner_count,
            after.refusals
        ),
        (
            budget_before.identities,
            budget_before.queued_bytes,
            budget_before.queued_messages,
            budget_before.active_calls,
            budget_before.owner_count,
            budget_before.refusals
        ),
    );
    let facts = tracker.facts(&request_binding, &handle, now()).unwrap();
    assert!(!facts.application_replied && !facts.accepted);
}

#[test]
fn registered_reply_observer_known_correlation_or_causation_mismatch_is_not_unknown() {
    let request_binding =
        MailboxBinding::new(config(MailboxMessageKind::Call, false), None).unwrap();
    let control = MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap();
    let replies = MailboxBinding::new(config(MailboxMessageKind::Reply, false), None).unwrap();
    let mut tracker = ReceiptTracker::new(budget(), 2).unwrap();
    let request = seal(&config(MailboxMessageKind::Call, false), b"known", 1, 1, 0);
    let handle = tracker
        .register_send(
            &request_binding,
            &control,
            &SigningKey::from_bytes(&[24; 32]).verifying_key(),
            &request.bytes,
            now(),
        )
        .unwrap();
    for (correlation, causation) in [(90, 99), (91, 1)] {
        let mismatch = reply_with_correlation(b"mismatched", correlation, causation);
        assert_eq!(
            tracker.observe_registered_application_reply(&replies, &mismatch.bytes, now()),
            Err(ServiceError::Unauthorized),
            "known correlation OR causation must not become None: {correlation}/{causation}",
        );
    }
    assert!(
        !tracker
            .facts(&request_binding, &handle, now())
            .unwrap()
            .application_replied
    );
}

#[test]
fn registered_reply_observer_preserves_expiry_commitment_and_authentication_errors() {
    for mode in ["expired", "commitment", "tampered-unknown"] {
        let request_binding =
            MailboxBinding::new(config(MailboxMessageKind::Call, false), None).unwrap();
        let control = MailboxBinding::new(config(MailboxMessageKind::Cast, true), None).unwrap();
        let replies = MailboxBinding::new(config(MailboxMessageKind::Reply, false), None).unwrap();
        let mut tracker = ReceiptTracker::new(budget(), 2).unwrap();
        let request = seal(&config(MailboxMessageKind::Call, false), b"known", 1, 1, 0);
        let handle = tracker
            .register_send(
                &request_binding,
                &control,
                &SigningKey::from_bytes(&[24; 32]).verifying_key(),
                &request.bytes,
                now(),
            )
            .unwrap();
        let first = seal(
            &config(MailboxMessageKind::Reply, false),
            b"first",
            30,
            30,
            1,
        );
        let (wire, time, expected) = match mode {
            "expired" => (
                first.bytes,
                ServiceTime {
                    unix_ms: 1_900,
                    monotonic_ms: 601,
                },
                ServiceError::Expired,
            ),
            "commitment" => {
                assert_eq!(
                    tracker.observe_registered_application_reply(&replies, &first.bytes, now()),
                    Ok(Some(handle))
                );
                let changed = seal(
                    &config(MailboxMessageKind::Reply, false),
                    b"other",
                    30,
                    30,
                    1,
                );
                (changed.bytes, now(), ServiceError::Conflict)
            }
            "tampered-unknown" => {
                let unknown = reply_with_correlation(b"unknown", 91, 99);
                let mut envelope = mvm_core::mailbox::decode_envelope(&unknown.bytes).unwrap();
                envelope.ciphertext[0] ^= 1;
                (
                    mvm_core::mailbox::encode_envelope(&envelope).unwrap(),
                    now(),
                    ServiceError::Unauthorized,
                )
            }
            _ => unreachable!(),
        };
        assert_eq!(
            tracker.observe_registered_application_reply(&replies, &wire, time),
            Err(expected),
            "{mode}"
        );
    }
}
