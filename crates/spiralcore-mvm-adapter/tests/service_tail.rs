#![cfg(feature = "live")]
mod common;
use ed25519_dalek::SigningKey;
use mvm_contract::protocol::mailbox::*;
use mvm_core::mailbox::{SealRequest, encode_envelope, seal_envelope};
use spiralcore_mvm_adapter::{DirectionConfig, service::*};

fn now() -> ServiceTime {
    ServiceTime {
        unix_ms: 1500,
        monotonic_ms: 100,
    }
}
fn wire(config: &DirectionConfig, payload: &[u8], id: u8, call: Option<u8>) -> Vec<u8> {
    let context = config.policy.snapshot().unwrap();
    let envelope = seal_envelope(
        SealRequest {
            routing: MailboxRoutingFrame {
                protocol_version: ProtocolVersion::V1,
                authorization_domain_route_label: config
                    .binding
                    .claims
                    .authorization_domain_route_label,
                destination_route_token: config.binding.claims.destination_route_token,
                delivery_id: DeliveryId::from_bytes(rand::random()),
                ciphertext_len: 0,
                padding: CiphertextPadding::Exact,
                expires_at_unix_ms: 2000,
                hop_limit: 8,
                transport_binding_id: config.binding.claims.binding_id,
            },
            header: MailboxEnvelopeHeader {
                protocol_version: ProtocolVersion::V1,
                signature_algorithm: SignatureAlgorithm::ED25519,
                tenant: context.expected_tenant,
                authorization_domain: context.expected_authorization_domain,
                sender: context.expected_sender,
                recipient: context.expected_recipient,
                mailbox: context.expected_mailbox,
                sender_boot: config.boot,
                execution_epoch: context.current_execution_epoch,
                message_id: MessageId::from_bytes([id; 32]),
                correlation_id: CorrelationId::from_bytes([call.unwrap_or(id); 32]),
                causation_id: CausationId::from_bytes([call.unwrap_or(0); 32]),
                kind: config.kind,
                reply_to: config.reply_to,
                ordering_scope: OrderingScope::parse(format!("tail-{id}")).unwrap(),
                sequence: SequenceNumber::new(1).unwrap(),
                issued_at_unix_ms: 1000,
                expires_at_unix_ms: 2000,
                nonce: ReplayNonce::from_bytes([id; 16]),
                content_type: config.capability.grant.content_types[0].clone(),
                capability_id: config.capability.grant.capability_id,
                policy_epoch: context.current_policy_epoch,
                encryption: EncryptionContext {
                    algorithm: EncryptionAlgorithm::AES_256_GCM,
                    key_id: config.key_id,
                },
                plaintext_len: payload.len() as u32,
            },
            plaintext: payload,
        },
        &config.encryption_key,
        config.signing_key.as_ref().unwrap(),
    )
    .unwrap();
    encode_envelope(&envelope).unwrap()
}
fn binding(config: DirectionConfig) -> MailboxBinding {
    let context = config.policy.snapshot().unwrap();
    MailboxBinding::new(
        ServiceBindingConfig::from_direction(config).unwrap(),
        Some(DestinationReceiptSigner {
            recipient: context.expected_recipient,
            signing_key: SigningKey::from_bytes(&[24; 32]),
        }),
    )
    .unwrap()
}
fn ready(
    limits: ServiceLimits,
) -> (
    HostBudget,
    LocalMailboxService,
    MailboxBinding,
    MailboxBinding,
    CallHandle,
    Vec<u8>,
) {
    ready_with(|_| limits)
}
fn ready_with(
    limits: impl FnOnce(usize) -> ServiceLimits,
) -> (
    HostBudget,
    LocalMailboxService,
    MailboxBinding,
    MailboxBinding,
    CallHandle,
    Vec<u8>,
) {
    ready_configured(limits, |_, _| {})
}
fn ready_configured(
    limits: impl FnOnce(usize) -> ServiceLimits,
    configure: impl FnOnce(&mut DirectionConfig, usize),
) -> (
    HostBudget,
    LocalMailboxService,
    MailboxBinding,
    MailboxBinding,
    CallHandle,
    Vec<u8>,
) {
    ready_budgeted(|w, _| limits(w), configure, HostBudget::new)
}
fn ready_budgeted(
    limits: impl FnOnce(usize, usize) -> ServiceLimits,
    configure: impl FnOnce(&mut DirectionConfig, usize),
    budget: impl FnOnce(ServiceLimits) -> Result<HostBudget, ServiceError>,
) -> (
    HostBudget,
    LocalMailboxService,
    MailboxBinding,
    MailboxBinding,
    CallHandle,
    Vec<u8>,
) {
    let c = common::config(MailboxMessageKind::Call);
    let request = wire(&c, b"question", 41, None);
    let caller = binding(c);
    let mut r = common::config(MailboxMessageKind::Reply);
    let reply = wire(&r, b"answer\0", 42, Some(41));
    configure(&mut r, reply.len());
    let replier = binding(r);
    let budget = budget(limits(reply.len(), reply.capacity())).unwrap();
    let mut service = LocalMailboxService::new(budget.clone()).unwrap();
    let call = service.register_call(&caller, &request, now()).unwrap();
    assert_eq!(
        service
            .admit(&replier, &reply, now())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    (budget, service, caller, replier, call, reply)
}

#[test]
fn another_service_shared_host_and_tenant_charge_blocks_tail_until_it_releases() {
    for tenant_only in [false, true] {
        let mut cast_config = common::config(MailboxMessageKind::Cast);
        cast_config.reply_to = None;
        let cast = wire(&cast_config, b"real competing queue", 71, None);
        let f = cast.len() as u64;
        let (budget, mut service, caller, replier, call, reply) = ready_budgeted(
            |w, c| {
                let mut limits = ServiceLimits::default();
                let cap = w as u64 + c as u64 + f - 1;
                if !tenant_only {
                    limits.host.queued_bytes = cap;
                }
                limits.tenant.queued_bytes = cap;
                limits.mailbox.queued_bytes = cap;
                limits.sender.queued_bytes = cap;
                limits
            },
            |_, _| {},
            HostBudget::new,
        );
        let cast_binding = binding(cast_config);
        let mut competitor = LocalMailboxService::new(budget.clone()).unwrap();
        assert_eq!(
            competitor
                .admit(&cast_binding, &cast, now())
                .unwrap()
                .receipt
                .outcome,
            DeliveryOutcome::AcceptedEphemeral
        );
        let mut wire = Some(reply);
        let w = wire.as_ref().unwrap().len() as u64;
        let retained_capacity = wire.as_ref().unwrap().capacity() as u64;
        assert_eq!(
            service
                .take_call_reply_with_tail(&caller, &replier, &call, &mut wire, now())
                .err(),
            Some(ServiceError::Full)
        );
        let refusal = budget.snapshot().unwrap().refusals.last.unwrap();
        assert_eq!(
            refusal.scope,
            if tenant_only {
                BudgetScope::Tenant
            } else {
                BudgetScope::Host
            }
        );
        assert_eq!(budget.snapshot().unwrap().queued_bytes, w + f);
        drop(competitor.receive(&cast_binding, now()).unwrap().unwrap());
        let done = service
            .take_call_reply_with_tail(&caller, &replier, &call, &mut wire, now())
            .unwrap();
        assert_eq!(done.payload, b"answer\0");
        assert_eq!(budget.snapshot().unwrap().queued_bytes, retained_capacity);
        drop(done);
        assert_eq!(budget.snapshot().unwrap().queued_bytes, 0);
    }
}

#[test]
fn mailbox_sender_and_signed_ceilings_preserve_unconsumed_reply() {
    for scope in [
        BudgetScope::Mailbox,
        BudgetScope::Sender,
        BudgetScope::Capability,
    ] {
        let (budget, mut service, caller, replier, call, reply) = ready_configured(
            |w| {
                let mut limits = ServiceLimits::default();
                if scope == BudgetScope::Mailbox {
                    limits.mailbox.queued_bytes = 2 * w as u64 - 1;
                }
                if scope != BudgetScope::Capability {
                    limits.sender.queued_bytes = 2 * w as u64 - 1;
                }
                limits
            },
            |config, w| {
                if scope == BudgetScope::Capability {
                    config.capability.grant.max_queue_contribution_bytes = 2 * w as u64 - 1;
                    config.capability = mvm_core::mailbox::sign_capability(
                        config.capability.grant.clone(),
                        &SigningKey::from_bytes(&[22; 32]),
                    )
                    .unwrap();
                }
            },
        );
        let mut wire = Some(reply);
        let w = wire.as_ref().unwrap().len() as u64;
        assert_eq!(
            service
                .take_call_reply_with_tail(&caller, &replier, &call, &mut wire, now())
                .err(),
            Some(ServiceError::Full)
        );
        assert_eq!(
            budget.snapshot().unwrap().refusals.last.unwrap().scope,
            scope
        );
        assert_eq!(budget.snapshot().unwrap().queued_bytes, w);
        assert_eq!(
            service.poll_call(&caller, &call, now()).unwrap(),
            CallStatus::Replied(Some(b"answer\0".to_vec()))
        );
    }
}

#[test]
fn altered_wire_and_foreign_handle_refuse_without_losing_original_result() {
    let (budget, mut service, caller, replier, call, reply) = ready(ServiceLimits::default());
    let duplicate = wire(
        &common::config(MailboxMessageKind::Reply),
        b"answer\0",
        42,
        Some(41),
    );
    assert_eq!(
        service
            .admit(&replier, &duplicate, now())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::Duplicate
    );
    assert_ne!(
        duplicate, reply,
        "fresh routing identity creates a distinct authenticated wire"
    );
    let mut candidate = Some(duplicate);
    assert_eq!(
        service
            .take_call_reply_with_tail(&caller, &replier, &call, &mut candidate, now())
            .err(),
        Some(ServiceError::Unauthorized)
    );
    let late = wire(
        &common::config(MailboxMessageKind::Reply),
        b"answer\0",
        48,
        Some(41),
    );
    assert_eq!(
        service
            .admit(&replier, &late, now())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::Late
    );
    let mut candidate = Some(late);
    assert_eq!(
        service
            .take_call_reply_with_tail(&caller, &replier, &call, &mut candidate, now())
            .err(),
        Some(ServiceError::Unauthorized)
    );
    let mut altered = reply.clone();
    altered[0] ^= 1;
    let mut candidate = Some(altered);
    assert_eq!(
        service
            .take_call_reply_with_tail(&caller, &replier, &call, &mut candidate, now())
            .err(),
        Some(ServiceError::Unauthorized)
    );
    let mut foreign =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let request = wire(
        &common::config(MailboxMessageKind::Call),
        b"question",
        41,
        None,
    );
    let handle = foreign.register_call(&caller, &request, now()).unwrap();
    let mut candidate = Some(reply);
    assert_eq!(
        service
            .take_call_reply_with_tail(&caller, &replier, &handle, &mut candidate, now())
            .err(),
        Some(ServiceError::Unauthorized)
    );
    assert!(budget.snapshot().unwrap().queued_bytes > 0);
    let done = service
        .take_call_reply_with_tail(&caller, &replier, &call, &mut candidate, now())
        .unwrap();
    assert_eq!(done.payload, b"answer\0");
    let mut second = Some(done.tail.wire().to_vec());
    assert_eq!(
        service
            .take_call_reply_with_tail(&caller, &replier, &call, &mut second, now())
            .err(),
        Some(ServiceError::Conflict)
    );
}

#[test]
fn tail_original_deadlines_and_generation_refuse_while_owned_bytes_stay_charged() {
    let (budget, mut service, caller, replier, call, reply) = ready(ServiceLimits::default());
    let retained_capacity = reply.capacity() as u64;
    let mut bytes = Some(reply);
    let done = service
        .take_call_reply_with_tail(&caller, &replier, &call, &mut bytes, now())
        .unwrap();
    assert_eq!(
        done.tail.validate(
            &caller,
            &replier,
            ServiceTime {
                unix_ms: 1500,
                monotonic_ms: 99
            }
        ),
        Err(ServiceError::Clock)
    );
    assert_eq!(
        done.tail.validate(
            &caller,
            &replier,
            ServiceTime {
                unix_ms: 2000,
                monotonic_ms: 100
            }
        ),
        Err(ServiceError::Expired)
    );
    assert_eq!(
        done.tail.validate(
            &caller,
            &replier,
            ServiceTime {
                unix_ms: 1500,
                monotonic_ms: 600
            }
        ),
        Err(ServiceError::Expired)
    );
    replier
        .replace_policy(
            common::config(MailboxMessageKind::Reply)
                .policy
                .snapshot()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        done.tail.validate(&caller, &replier, now()),
        Err(ServiceError::Unauthorized)
    );
    assert_eq!(budget.snapshot().unwrap().queued_bytes, retained_capacity);
    drop(done);
    assert_eq!(budget.snapshot().unwrap().queued_bytes, 0);
}

#[test]
fn data_reserve_and_queued_message_ceilings_charge_the_overlap() {
    for messages in [false, true] {
        let (budget, mut service, caller, replier, call, reply) = ready_budgeted(
            |w, c| {
                let mut limits = ServiceLimits::default();
                for scope in [
                    &mut limits.host,
                    &mut limits.tenant,
                    &mut limits.mailbox,
                    &mut limits.sender,
                ] {
                    if messages {
                        scope.queued_messages = 1;
                    } else {
                        scope.queued_bytes = w as u64 + c as u64 + 1;
                    }
                }
                limits
            },
            |_, _| {},
            |limits| {
                if messages {
                    return HostBudget::new(limits);
                }
                let scope = ControlScope {
                    queued_bytes: 2,
                    queued_messages: 1,
                    identities: 1,
                };
                HostBudget::with_control(
                    limits,
                    ControlReserve {
                        host: scope,
                        tenant: scope,
                        mailbox: scope,
                        sender: scope,
                    },
                )
            },
        );
        let mut wire = Some(reply);
        let before = budget.snapshot().unwrap();
        assert_eq!(
            service
                .take_call_reply_with_tail(&caller, &replier, &call, &mut wire, now())
                .err(),
            Some(ServiceError::Full)
        );
        let refusal = budget.snapshot().unwrap().refusals.last.unwrap();
        assert_eq!(
            refusal.resource,
            if messages {
                BudgetResource::QueuedMessages
            } else {
                BudgetResource::QueuedBytes
            }
        );
        assert_eq!(
            refusal.pool,
            if messages {
                BudgetPool::Total
            } else {
                BudgetPool::Data
            }
        );
        assert_eq!(budget.snapshot().unwrap().queued_bytes, before.queued_bytes);
        assert_eq!(
            service.poll_call(&caller, &call, now()).unwrap(),
            CallStatus::Replied(Some(b"answer\0".to_vec()))
        );
    }
}

#[test]
fn tail_memory_reservation_does_not_spend_a_second_signed_arrival() {
    let (_, mut service, caller, replier, call, reply) = ready_configured(
        |_| ServiceLimits::default(),
        |config, _| {
            config.capability.grant.max_messages_per_minute = 2;
            config.capability = mvm_core::mailbox::sign_capability(
                config.capability.grant.clone(),
                &SigningKey::from_bytes(&[22; 32]),
            )
            .unwrap();
        },
    );
    let mut bytes = Some(reply);
    let _tail = service
        .take_call_reply_with_tail(&caller, &replier, &call, &mut bytes, now())
        .unwrap();
    let request = wire(
        &common::config(MailboxMessageKind::Call),
        b"second",
        43,
        None,
    );
    let second = service.register_call(&caller, &request, now()).unwrap();
    let reply = wire(
        &common::config(MailboxMessageKind::Reply),
        b"second answer",
        44,
        Some(43),
    );
    assert_eq!(
        service
            .admit(&replier, &reply, now())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    assert_eq!(
        service.poll_call(&caller, &second, now()).unwrap(),
        CallStatus::Replied(Some(b"second answer".to_vec()))
    );
}

#[test]
fn pending_shutdown_expiry_and_owner_takeover_never_grant_tail_authority() {
    let (budget, mut service, caller, replier, call, reply) = ready(ServiceLimits::default());
    let request = wire(
        &common::config(MailboxMessageKind::Call),
        b"pending",
        45,
        None,
    );
    let pending = service.register_call(&caller, &request, now()).unwrap();
    let mut bytes = Some(reply);
    assert_eq!(
        service
            .take_call_reply_with_tail(&caller, &replier, &pending, &mut bytes, now())
            .err(),
        Some(ServiceError::Backpressured)
    );
    assert_eq!(
        service
            .take_call_reply_with_tail(
                &caller,
                &replier,
                &call,
                &mut bytes,
                ServiceTime {
                    unix_ms: 2000,
                    monotonic_ms: 100
                }
            )
            .err(),
        Some(ServiceError::Expired)
    );
    assert!(bytes.is_some());
    // Use a fresh case because canonical maintenance has observed the expiry.
    drop(service);
    let (_, mut service, caller, replier, call, reply) = ready(ServiceLimits::default());
    let mut bytes = Some(reply);
    service.shutdown(now()).unwrap();
    assert_eq!(
        service
            .take_call_reply_with_tail(&caller, &replier, &call, &mut bytes, now())
            .err(),
        Some(ServiceError::ShuttingDown)
    );
    assert!(bytes.is_some());
    // Actual higher-epoch owner admission fences a previously extracted tail.
    let (shared, mut service, caller, replier, call, reply) = ready(ServiceLimits::default());
    let mut bytes = Some(reply);
    let done = service
        .take_call_reply_with_tail(&caller, &replier, &call, &mut bytes, now())
        .unwrap();
    let mut next = common::config(MailboxMessageKind::Reply);
    let epoch = ExecutionEpoch::new(8).unwrap();
    let mut context = next.policy.snapshot().unwrap();
    context.current_execution_epoch = epoch;
    next.policy = spiralcore_mvm_adapter::Policy::new(context).unwrap();
    next.capability.grant.minimum_execution_epoch = epoch;
    next.capability.grant.maximum_execution_epoch = epoch;
    next.capability = mvm_core::mailbox::sign_capability(
        next.capability.grant.clone(),
        &SigningKey::from_bytes(&[22; 32]),
    )
    .unwrap();
    next.binding.claims.execution_epoch = epoch;
    next.binding = mvm_core::mailbox::transport::sign_transport_binding(
        next.binding.claims.clone(),
        &SigningKey::from_bytes(&[22; 32]),
    )
    .unwrap();
    let late = wire(&next, b"new owner", 46, Some(45));
    let next = binding(next);
    let mut successor = LocalMailboxService::new(shared.clone()).unwrap();
    assert_eq!(
        successor
            .admit(&next, &late, now())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::Late
    );
    assert_eq!(
        done.tail.validate(&caller, &replier, now()),
        Err(ServiceError::Unauthorized)
    );
    assert!(shared.snapshot().unwrap().queued_bytes > 0);
    drop(done);
    assert_eq!(shared.snapshot().unwrap().queued_bytes, 0);
    assert_eq!(budget.snapshot().unwrap().queued_bytes, 0);
}

#[test]
fn overlap_full_preserves_original_payload_wire_and_charge_before_strict_poll() {
    let (budget, mut service, caller, replier, call, reply) = ready_with(|w| {
        let mut limits = ServiceLimits::default();
        for scope in [
            &mut limits.host,
            &mut limits.tenant,
            &mut limits.mailbox,
            &mut limits.sender,
        ] {
            scope.queued_bytes = 2 * w as u64 - 1;
        }
        limits
    });
    assert!(reply.len() > b"answer\0".len());
    let w = reply.len() as u64;
    let mut wire = Some(reply);
    let ptr = wire.as_ref().unwrap().as_ptr();
    let before = budget.snapshot().unwrap();
    assert_eq!(before.queued_bytes, w);
    assert_eq!(
        service
            .take_call_reply_with_tail(&caller, &replier, &call, &mut wire, now())
            .err(),
        Some(ServiceError::Full)
    );
    assert_eq!(wire.as_ref().unwrap().as_ptr(), ptr);
    let after = budget.snapshot().unwrap();
    assert_eq!(
        (after.queued_bytes, after.queued_messages),
        (before.queued_bytes, before.queued_messages)
    );
    assert_eq!(
        service.poll_call(&caller, &call, now()).unwrap(),
        CallStatus::Replied(Some(b"answer\0".to_vec()))
    );
    assert_eq!(budget.snapshot().unwrap().queued_bytes, 0);
}

#[test]
fn overallocated_authenticated_wire_refuses_before_extraction_without_changing_allocation() {
    let (budget, mut service, caller, replier, call, reply) = ready_with(|w| {
        let mut limits = ServiceLimits::default();
        for scope in [
            &mut limits.host,
            &mut limits.tenant,
            &mut limits.mailbox,
            &mut limits.sender,
        ] {
            scope.queued_bytes = 3 * w as u64;
        }
        limits
    });
    let w = reply.len();
    let mut allocation = Vec::with_capacity(4 * w);
    allocation.extend_from_slice(&reply);
    let capacity = allocation.capacity();
    let pointer = allocation.as_ptr();
    let mut wire = Some(allocation);
    assert!(capacity > 3 * w);
    assert_eq!(
        service
            .take_call_reply_with_tail(&caller, &replier, &call, &mut wire, now())
            .err(),
        Some(ServiceError::Full)
    );
    assert_eq!(wire.as_ref().unwrap().as_ptr(), pointer);
    assert_eq!(wire.as_ref().unwrap().capacity(), capacity);
    assert_eq!(
        budget.snapshot().unwrap().refusals.last.unwrap().requested,
        capacity as u64
    );
    assert_eq!(budget.snapshot().unwrap().queued_bytes, w as u64);
    assert_eq!(
        service.poll_call(&caller, &call, now()).unwrap(),
        CallStatus::Replied(Some(b"answer\0".to_vec()))
    );
    assert_eq!(budget.snapshot().unwrap().queued_bytes, 0);
}

#[test]
fn extracted_tail_moves_exact_wire_and_stays_charged_after_service_drop_until_lease_drop() {
    let (budget, mut service, caller, replier, call, reply) = ready(ServiceLimits::default());
    let mut retained = Vec::with_capacity(2 * reply.len());
    retained.extend_from_slice(&reply);
    let reply = retained;
    let retained_capacity = reply.capacity() as u64;
    let ptr = reply.as_ptr();
    let mut wire = Some(reply);
    let completed = service
        .take_call_reply_with_tail(&caller, &replier, &call, &mut wire, now())
        .unwrap();
    assert!(wire.is_none());
    assert_eq!(completed.payload, b"answer\0");
    assert_eq!(completed.tail.wire().as_ptr(), ptr);
    assert_eq!(completed.tail.call(), &call);
    completed.tail.validate(&caller, &replier, now()).unwrap();
    assert_eq!(
        service.poll_call(&caller, &call, now()).unwrap(),
        CallStatus::Replied(None)
    );
    assert_eq!(budget.snapshot().unwrap().queued_bytes, retained_capacity);
    drop(service);
    assert_eq!(
        completed.tail.validate(&caller, &replier, now()),
        Err(ServiceError::ShuttingDown)
    );
    assert_eq!(
        (
            budget.snapshot().unwrap().queued_bytes,
            budget.snapshot().unwrap().queued_messages
        ),
        (retained_capacity, 1)
    );
    let moved = completed.tail;
    drop(moved);
    assert_eq!(
        (
            budget.snapshot().unwrap().queued_bytes,
            budget.snapshot().unwrap().queued_messages
        ),
        (0, 0)
    );
}
