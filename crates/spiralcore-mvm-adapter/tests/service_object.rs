#![cfg(feature = "live")]
mod common;

use ed25519_dalek::SigningKey;
use mvm_contract::protocol::mailbox::{
    CapabilityOperations, DeliveryOutcome, MailboxMessageKind, MailboxOperation, PrincipalId,
};
use spiralcore_mvm_adapter::{
    service::{
        CallStatus, DestinationReceiptSigner, HostBudget, LocalMailboxService, MailboxBinding,
        ServiceBindingConfig, ServiceError, ServiceLimits, ServiceTime,
    },
    service_object::ObjectMailbox,
    service_sender::{self, MessageOptions},
};
use spiralcore_transfer::{
    client,
    endpoint::{ColdGraphPolicy, Limits},
    object_id,
    protocol::Mode,
};

fn now() -> ServiceTime {
    ServiceTime {
        unix_ms: 1500,
        monotonic_ms: 1,
    }
}
fn binding() -> MailboxBinding {
    MailboxBinding::new(
        ServiceBindingConfig::from_direction(common::config(MailboxMessageKind::Call)).unwrap(),
        Some(DestinationReceiptSigner {
            recipient: PrincipalId::from_bytes([3; 32]),
            signing_key: SigningKey::from_bytes(&[57; 32]),
        }),
    )
    .unwrap()
}

fn reply_binding() -> MailboxBinding {
    let mut config =
        ServiceBindingConfig::from_direction(common::config(MailboxMessageKind::Reply)).unwrap();
    config.context.required_operation = MailboxOperation::Reply;
    config.capability.grant.operations =
        CapabilityOperations::from_operations(&[MailboxOperation::Reply]);
    config.capability = mvm_core::mailbox::sign_capability(
        config.capability.grant,
        &SigningKey::from_bytes(&[22; 32]),
    )
    .unwrap();
    MailboxBinding::new(
        config,
        Some(DestinationReceiptSigner {
            recipient: PrincipalId::from_bytes([2; 32]),
            signing_key: SigningKey::from_bytes(&[59; 32]),
        }),
    )
    .unwrap()
}

fn reply_request<'a>(
    binding: &MailboxBinding,
    request: &spiralcore_mvm_adapter::service::AdmittedMessage,
    payload: &'a [u8],
) -> mvm_core::mailbox::SealRequest<'a> {
    service_sender::request(
        binding,
        payload,
        &MessageOptions::for_reply(request, now()).unwrap(),
        now(),
    )
    .unwrap()
}
fn prepare_request(
    binding: &MailboxBinding,
    payload: &[u8],
    route: Option<mvm_contract::protocol::mailbox::ReplyRoute>,
) -> mvm_core::mailbox::transport::PreparedMailboxEnvelope {
    let mut options = MessageOptions::fresh(
        mvm_contract::protocol::mailbox::ContentType::parse("application/spiralcore-object", 1)
            .unwrap(),
        now(),
        2000,
    )
    .unwrap();
    options.reply_to = route;
    service_sender::prepare(binding, payload, &options, now()).unwrap()
}

fn request_wire(binding: &MailboxBinding, payload: &[u8]) -> Vec<u8> {
    let prepared = prepare_request(
        binding,
        payload,
        common::config(MailboxMessageKind::Call).reply_to,
    );
    mvm_core::mailbox::encode_envelope(prepared.envelope()).unwrap()
}

#[test]
fn admitted_object_calls_use_shared_core_and_exact_readback() {
    let expected = [common::random(131073, 51), vec![0; 17]].concat();
    check_exact_repeat(binding(), reply_binding(), expected);
}

#[test]
fn four_kib_raw_object_fits_original_4152_byte_call_and_88_byte_reply() {
    let limit = |kind, max_payload_bytes, recipient, key| {
        let mut cfg = ServiceBindingConfig::from_direction(common::config(kind)).unwrap();
        cfg.capability.grant.max_payload_bytes = max_payload_bytes;
        cfg.capability = mvm_core::mailbox::sign_capability(
            cfg.capability.grant,
            &SigningKey::from_bytes(&[22; 32]),
        )
        .unwrap();
        MailboxBinding::new(
            cfg,
            Some(DestinationReceiptSigner {
                recipient: PrincipalId::from_bytes([recipient; 32]),
                signing_key: SigningKey::from_bytes(&[key; 32]),
            }),
        )
        .unwrap()
    };
    let expected = [common::random(4079, 51), vec![0; 17]].concat();
    check_exact_repeat(
        limit(MailboxMessageKind::Call, 4152, 3, 57),
        limit(MailboxMessageKind::Reply, 88, 2, 59),
        expected,
    );
}

fn check_exact_repeat(binding: MailboxBinding, reverse: MailboxBinding, expected: Vec<u8>) {
    let store = common::Store::new();
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let mut caller =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let mut endpoint = ObjectMailbox::open(
        &store.0,
        Limits::default(),
        common::config(MailboxMessageKind::Call).policy,
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    endpoint.set_codec_cache_enabled(true).unwrap();
    let route = common::config(MailboxMessageKind::Call).reply_to;
    for (attempt, warm) in [false, true].into_iter().enumerate() {
        let mut exact = false;
        let mut forward_envelopes = 0;
        let mut reverse_envelopes = 0;
        let wall = std::time::Instant::now();
        let cpu = cpu_time::ThreadTime::now();
        let limits = service_sender::object_raw_frame_limits(
            &binding,
            &reverse,
            [&caller, &service],
            route.as_ref().unwrap(),
            now(),
        )
        .unwrap();
        let stats = client::transfer_raw_capped_batched(
            &expected,
            [73 + attempt as u8; 32],
            warm,
            limits,
            |frames| {
                let mut replies = Vec::new();
                for frame in frames {
                    let original = prepare_request(&binding, &frame.encode()?, route.clone());
                    let wire = mvm_core::mailbox::encode_envelope(original.envelope()).unwrap();
                    forward_envelopes += wire.len();
                    let call = caller.register_call(&binding, &wire, now()).unwrap();
                    let receipt = service.admit(&binding, &wire, now()).unwrap();
                    assert_eq!(receipt.receipt.outcome, DeliveryOutcome::AcceptedEphemeral);
                    mvm_core::mailbox::verify_receipt(
                        &receipt,
                        &SigningKey::from_bytes(&[57; 32]).verifying_key(),
                    )
                    .unwrap();
                    let message = service.receive(&binding, now()).unwrap().unwrap();
                    let receiver_context = endpoint.context(&binding, &message, now()).unwrap();
                    let prepared_scope =
                        service_sender::object_context(&binding, &original, now()).unwrap();
                    assert_eq!(receiver_context, prepared_scope);
                    let reply = endpoint.handle(&binding, &message, now()).unwrap();
                    if reply.kind == 103 {
                        assert_eq!(
                            endpoint
                                .read_object(&binding, &message, object_id(&expected), now())
                                .unwrap(),
                            expected
                        );
                        exact = true;
                    }
                    let payload = reply.encode()?;
                    let envelope = service
                        .reply(
                            &reverse,
                            message.handle(),
                            reply_request(&reverse, &message, &payload),
                            now(),
                        )
                        .unwrap();
                    let encoded = mvm_core::mailbox::encode_envelope(&envelope).unwrap();
                    reverse_envelopes += encoded.len();
                    let accepted = caller.admit(&reverse, &encoded, now()).unwrap();
                    assert_eq!(accepted.receipt.outcome, DeliveryOutcome::AcceptedEphemeral);
                    let CallStatus::Replied(Some(payload)) =
                        caller.poll_call(&binding, &call, now()).unwrap()
                    else {
                        panic!("canonical application reply missing")
                    };
                    let reply = spiralcore_transfer::protocol::Frame::decode(&payload)?;
                    let done = matches!(reply.kind, 103 | 127);
                    replies.push(reply);
                    if done {
                        break;
                    }
                }
                Ok(replies)
            },
        )
        .unwrap();
        assert_eq!(stats.object, object_id(&expected));
        assert!(exact);
        if warm {
            assert_eq!(
                stats.requests, 1,
                "repeat needs only the authenticated Offer"
            );
            assert_eq!(stats.data_bytes, 0);
        } else {
            assert_eq!(stats.data_bytes, expected.len() as u64);
        }
        eprintln!(
            "warm={warm} original={} object_request_bytes={} object_reply_bytes={} data_bytes={} forward_envelope_bytes={forward_envelopes} reverse_envelope_bytes={reverse_envelopes} thread_cpu_ns={} wall_ns={}",
            expected.len(),
            stats.request_bytes,
            stats.response_bytes,
            stats.data_bytes,
            cpu.elapsed().as_nanos(),
            wall.elapsed().as_nanos()
        );
    }
}

#[test]
fn shutdown_fences_a_dequeued_object_proof() {
    let store = common::Store::new();
    let binding = binding();
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let mut endpoint = ObjectMailbox::open(
        &store.0,
        Limits::default(),
        common::config(MailboxMessageKind::Call).policy,
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    let frame = spiralcore_transfer::protocol::Frame {
        kind: 1,
        transfer: [74; 32],
        seq: 0,
        body: spiralcore_transfer::protocol::Offer::cold(&[], Mode::Raw)
            .encode()
            .unwrap(),
    };
    let wire = common::sealed_payload(
        &common::config(MailboxMessageKind::Call),
        &frame.encode().unwrap(),
    );
    service.admit(&binding, &wire, now()).unwrap();
    let message = service.receive(&binding, now()).unwrap().unwrap();
    service.shutdown(now()).unwrap();
    assert!(endpoint.handle(&binding, &message, now()).is_err());
    assert!(
        endpoint
            .read_object(&binding, &message, object_id(&[]), now())
            .is_err()
    );
}

#[test]
fn expired_or_replaced_policy_cannot_reuse_an_object_proof() {
    for replace in [false, true] {
        let store = common::Store::new();
        let binding = binding();
        let mut service =
            LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
        let mut endpoint = ObjectMailbox::open(
            &store.0,
            Limits::default(),
            common::config(MailboxMessageKind::Call).policy,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        let frame = spiralcore_transfer::protocol::Frame {
            kind: 1,
            transfer: [75; 32],
            seq: 0,
            body: spiralcore_transfer::protocol::Offer::cold(&[], Mode::Raw)
                .encode()
                .unwrap(),
        };
        let config = common::config(MailboxMessageKind::Call);
        let wire = common::sealed_payload(&config, &frame.encode().unwrap());
        service.admit(&binding, &wire, now()).unwrap();
        let message = service.receive(&binding, now()).unwrap().unwrap();
        let time = if replace {
            binding
                .replace_policy(config.policy.snapshot().unwrap())
                .unwrap();
            now()
        } else {
            ServiceTime {
                unix_ms: 1999,
                monotonic_ms: 501,
            }
        };
        assert!(endpoint.handle(&binding, &message, time).is_err());
        assert!(
            endpoint
                .read_object(&binding, &message, object_id(&[]), time)
                .is_err()
        );
    }
}

fn assert_post_work_reply_is_fenced(replace_policy: bool) {
    let store = common::Store::new();
    let binding = binding();
    let reverse = reply_binding();
    let budget = HostBudget::new(ServiceLimits::default()).unwrap();
    let mut service = LocalMailboxService::new(budget.clone()).unwrap();
    let mut endpoint = ObjectMailbox::open(
        &store.0,
        Limits::default(),
        common::config(MailboxMessageKind::Call).policy,
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    let frame = spiralcore_transfer::protocol::Frame {
        kind: 1,
        transfer: [76; 32],
        seq: 0,
        body: spiralcore_transfer::protocol::Offer::cold(&[], Mode::Raw)
            .encode()
            .unwrap(),
    };
    let wire = request_wire(&binding, &frame.encode().unwrap());
    assert_eq!(
        service
            .admit(&binding, &wire, now())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    let message = service.receive(&binding, now()).unwrap().unwrap();
    let reply = endpoint.handle(&binding, &message, now()).unwrap();
    assert_ne!(
        reply.kind, 127,
        "object handling must succeed before fencing"
    );
    let payload = reply.encode().unwrap();
    // Build a valid Reply while both directions are authorized. Only the
    // post-work authority/deadline change should make its later commit fail.
    let request = reply_request(&reverse, &message, &payload);
    let before = budget.snapshot().unwrap();
    let (reply_time, expected_error) = if replace_policy {
        binding
            .replace_policy(
                common::config(MailboxMessageKind::Call)
                    .policy
                    .snapshot()
                    .unwrap(),
            )
            .unwrap();
        (now(), ServiceError::Unauthorized)
    } else {
        (
            ServiceTime {
                unix_ms: 2000,
                monotonic_ms: 501,
            },
            ServiceError::Expired,
        )
    };
    assert!(matches!(
        service.reply_prepared(&reverse, message.handle(), request, reply_time),
        Err(error) if error == expected_error
    ));
    let after = budget.snapshot().unwrap();
    assert_eq!(after.queued_bytes, before.queued_bytes);
    assert_eq!(after.queued_messages, before.queued_messages);
}

#[test]
fn incoming_policy_replaced_after_object_work_cannot_prepare_reply() {
    assert_post_work_reply_is_fenced(true);
}

#[test]
fn call_expiring_after_object_work_cannot_prepare_reply() {
    assert_post_work_reply_is_fenced(false);
}

#[test]
fn admitted_cast_is_not_an_object_protocol_call() {
    let store = common::Store::new();
    let mut config = common::config(MailboxMessageKind::Cast);
    config.reply_to = None;
    let frame = spiralcore_transfer::protocol::Frame {
        kind: 1,
        transfer: [76; 32],
        seq: 0,
        body: spiralcore_transfer::protocol::Offer::cold(&[], Mode::Raw)
            .encode()
            .unwrap(),
    };
    let wire = common::sealed_payload(&config, &frame.encode().unwrap());
    let binding = MailboxBinding::new(
        ServiceBindingConfig::from_direction(config).unwrap(),
        Some(DestinationReceiptSigner {
            recipient: PrincipalId::from_bytes([3; 32]),
            signing_key: SigningKey::from_bytes(&[57; 32]),
        }),
    )
    .unwrap();
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let mut endpoint = ObjectMailbox::open(
        &store.0,
        Limits::default(),
        common::config(MailboxMessageKind::Call).policy,
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    service.admit(&binding, &wire, now()).unwrap();
    let message = service.receive(&binding, now()).unwrap().unwrap();
    assert!(matches!(
        MessageOptions::for_reply(&message, now()),
        Err(ServiceError::Unauthorized)
    ));
    assert!(matches!(
        endpoint.handle(&binding, &message, now()),
        Err(ServiceError::Unauthorized)
    ));
    assert!(
        endpoint
            .read_object(&binding, &message, object_id(&[]), now())
            .is_err()
    );
}

#[test]
fn raw_budget_checks_both_original_grants_route_and_recipient_limits() {
    use mvm_contract::protocol::mailbox::CapabilityId;
    let call = binding();
    let reply = reply_binding();
    let local = |max_payload_bytes| {
        LocalMailboxService::new(
            HostBudget::new(ServiceLimits {
                max_payload_bytes,
                ..ServiceLimits::default()
            })
            .unwrap(),
        )
        .unwrap()
    };
    let b = local(1300);
    let a = local(90);
    let mut route = common::config(MailboxMessageKind::Call).reply_to.unwrap();
    let lookup = |time, route: &_| {
        service_sender::object_raw_frame_limits(&call, &reply, [&a, &b], route, time)
    };
    let limits = lookup(now(), &route).unwrap();
    assert_eq!((limits.request_bytes, limits.reply_bytes), (90, 90));
    assert!(matches!(
        lookup(
            ServiceTime {
                unix_ms: 3000,
                monotonic_ms: 1501
            },
            &route
        ),
        Err(ServiceError::Unauthorized)
    ));
    route.capability_id = CapabilityId::from_bytes([99; 32]);
    assert!(matches!(
        lookup(now(), &route),
        Err(ServiceError::Unauthorized)
    ));
}
