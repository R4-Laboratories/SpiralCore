#![cfg(feature = "live")]
mod common;
use ed25519_dalek::SigningKey;
use mvm_contract::protocol::mailbox::*;
use mvm_core::mailbox::{encode_envelope, sign_receipt};
use spiralcore_mvm_adapter::{
    DirectionConfig,
    issuance::IssuancePolicy,
    service::*,
    service_sender::{self, OrderingStream},
};

fn now() -> ServiceTime {
    ServiceTime {
        unix_ms: 1500,
        monotonic_ms: 100,
    }
}
fn binding(config: DirectionConfig) -> MailboxBinding {
    let recipient = config.policy.snapshot().unwrap().expected_recipient;
    MailboxBinding::new(
        ServiceBindingConfig::from_direction(config).unwrap(),
        Some(DestinationReceiptSigner {
            recipient,
            signing_key: SigningKey::from_bytes(&[24; 32]),
        }),
    )
    .unwrap()
}
struct Case {
    service: LocalMailboxService,
    incoming: MailboxBinding,
    outgoing: MailboxBinding,
    request: Vec<u8>,
    handle: DeliveryHandle,
    reply: mvm_core::mailbox::transport::PreparedMailboxEnvelope,
    reply_id: MessageId,
}
impl Case {
    fn new() -> Self {
        Self::with_reply_expiry(2000)
    }
    fn with_reply_expiry(expiry: i64) -> Self {
        let c = common::config(MailboxMessageKind::Call);
        let request = common::sealed_payload(&c, b"original question");
        let incoming = binding(c);
        let outgoing = binding(common::config(MailboxMessageKind::Reply));
        let mut service =
            LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
        assert_eq!(
            service
                .admit(&incoming, &request, now())
                .unwrap()
                .receipt
                .outcome,
            DeliveryOutcome::AcceptedEphemeral
        );
        let proof = service.receive(&incoming, now()).unwrap().unwrap();
        let handle = *proof.handle();
        let mut options = OrderingStream::new()
            .next_reply(&outgoing, &proof, now(), IssuancePolicy::Exact)
            .unwrap();
        options.expires_at_unix_ms = expiry;
        let reply_id = options.message_id;
        let reply = service
            .reply_prepared(
                &outgoing,
                &handle,
                service_sender::request(&outgoing, b"cached answer", &options, now()).unwrap(),
                now(),
            )
            .unwrap();
        Self {
            service,
            incoming,
            outgoing,
            request,
            handle,
            reply,
            reply_id,
        }
    }
}

#[test]
fn shorter_reply_monotonic_cap_survives_wall_rollback_on_both_retry_apis() {
    let mut case = Case::with_reply_expiry(1600);
    let rolled_back = ServiceTime {
        unix_ms: 1550,
        monotonic_ms: 200,
    };
    assert_eq!(
        case.service
            .retry_reply_prepared(&case.incoming, &case.request, rolled_back)
            .err(),
        Some(ServiceError::Expired)
    );
    assert_eq!(
        case.service
            .retry_reply_for_delivery_prepared(&case.incoming, &case.handle, rolled_back)
            .err(),
        Some(ServiceError::Expired)
    );
}

#[test]
fn admitted_handle_replays_the_exact_cached_wire_and_cleared_cache_is_none() {
    let mut case = Case::new();
    let first = encode_envelope(case.reply.envelope()).unwrap();
    let by_wire = case
        .service
        .retry_reply_prepared(&case.incoming, &case.request, now())
        .unwrap()
        .unwrap();
    let by_handle = case
        .service
        .retry_reply_for_delivery_prepared(&case.incoming, &case.handle, now())
        .unwrap()
        .unwrap();
    assert_eq!(encode_envelope(by_wire.envelope()).unwrap(), first);
    assert_eq!(encode_envelope(by_handle.envelope()).unwrap(), first);
    let envelope = case.reply.envelope();
    let config = common::config(MailboxMessageKind::Reply);
    let receipt = sign_receipt(
        DestinationReceipt {
            protocol_version: ProtocolVersion::V1,
            signature_algorithm: SignatureAlgorithm::ED25519,
            delivery_id: envelope.routing.delivery_id,
            message_id: case.reply_id,
            address: config.capability.grant.address,
            execution_epoch: config.capability.grant.minimum_execution_epoch,
            outcome: DeliveryOutcome::Duplicate,
            observed_at_unix_ms: 1500,
            ciphertext_commitment: envelope.ciphertext_commitment,
        },
        &SigningKey::from_bytes(&[24; 32]),
    )
    .unwrap();
    assert!(
        case.service
            .acknowledge_reply(
                &case.outgoing,
                &receipt,
                &SigningKey::from_bytes(&[24; 32]).verifying_key(),
                now()
            )
            .unwrap()
    );
    assert!(
        case.service
            .retry_reply_for_delivery_prepared(&case.incoming, &case.handle, now())
            .unwrap()
            .is_none()
    );
}

#[test]
fn foreign_service_binding_cast_and_changed_policy_never_select_a_cache() {
    let mut case = Case::new();
    let mut foreign =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    assert_eq!(
        foreign
            .retry_reply_for_delivery_prepared(&case.incoming, &case.handle, now())
            .err(),
        Some(ServiceError::Unauthorized)
    );
    let other = binding(common::config(MailboxMessageKind::Call));
    assert_eq!(
        case.service
            .retry_reply_for_delivery_prepared(&other, &case.handle, now())
            .err(),
        Some(ServiceError::Unauthorized)
    );
    let mut c = common::config(MailboxMessageKind::Cast);
    c.reply_to = None;
    let cast = common::sealed_payload(&c, b"not a Call");
    let cast_binding = binding(c);
    let mut cast_service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    cast_service.admit(&cast_binding, &cast, now()).unwrap();
    let proof = cast_service.receive(&cast_binding, now()).unwrap().unwrap();
    assert_eq!(
        cast_service
            .retry_reply_for_delivery_prepared(&cast_binding, proof.handle(), now())
            .err(),
        Some(ServiceError::Unauthorized)
    );
    case.incoming
        .replace_policy(
            common::config(MailboxMessageKind::Call)
                .policy
                .snapshot()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        case.service
            .retry_reply_for_delivery_prepared(&case.incoming, &case.handle, now())
            .err(),
        Some(ServiceError::Unauthorized)
    );
}

#[test]
fn original_call_expiry_outgoing_revocation_and_shutdown_refuse_retained_handle() {
    for mode in 0..4 {
        let mut case = Case::new();
        let time = match mode {
            0 => ServiceTime {
                unix_ms: 2000,
                monotonic_ms: 100,
            },
            1 => ServiceTime {
                unix_ms: 1500,
                monotonic_ms: 600,
            },
            _ => now(),
        };
        if mode == 2 {
            case.outgoing
                .replace_policy(
                    common::config(MailboxMessageKind::Reply)
                        .policy
                        .snapshot()
                        .unwrap(),
                )
                .unwrap();
        }
        if mode == 3 {
            case.service.shutdown(now()).unwrap();
        }
        assert_eq!(
            case.service
                .retry_reply_for_delivery_prepared(&case.incoming, &case.handle, time)
                .err(),
            Some(match mode {
                0 | 1 => ServiceError::Expired,
                2 => ServiceError::Unauthorized,
                _ => ServiceError::ShuttingDown,
            })
        );
    }
}
