#![cfg(feature = "live")]
mod common;

use ed25519_dalek::SigningKey;
use mvm_contract::protocol::mailbox::{
    CapabilityOperations, DeliveryOutcome, MailboxMessageKind, MailboxOperation, PrincipalId,
};
use spiralcore_mvm_adapter::{
    service::{
        CallStatus, DestinationReceiptSigner, HostBudget, LocalMailboxService, MailboxBinding,
        ServiceBindingConfig, ServiceLimits, ServiceTime,
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
fn request_wire(binding: &MailboxBinding, payload: &[u8]) -> Vec<u8> {
    let mut options = MessageOptions::fresh(
        mvm_contract::protocol::mailbox::ContentType::parse("application/spiralcore-object", 1)
            .unwrap(),
        now(),
        2000,
    )
    .unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    mvm_core::mailbox::encode_envelope(
        service_sender::prepare(binding, payload, &options, now())
            .unwrap()
            .envelope(),
    )
    .unwrap()
}

#[test]
fn admitted_object_calls_use_shared_core_and_exact_readback() {
    let store = common::Store::new();
    let binding = binding();
    let mut service =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let mut caller =
        LocalMailboxService::new(HostBudget::new(ServiceLimits::default()).unwrap()).unwrap();
    let reverse = reply_binding();
    let mut endpoint = ObjectMailbox::open(
        &store.0,
        Limits::default(),
        common::config(MailboxMessageKind::Call).policy,
        ColdGraphPolicy::Disabled,
    )
    .unwrap();
    endpoint.set_codec_cache_enabled(true).unwrap();
    let expected = [common::random(131073, 51), vec![0; 17]].concat();
    let mut exact = false;
    let stats = client::transfer_batched(&expected, Mode::Raw, None, [73; 32], false, |frames| {
        let mut replies = Vec::new();
        for frame in frames {
            let wire = request_wire(&binding, &frame.encode()?);
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
            let prepared_scope = service_sender::object_context(
                &binding,
                &service_sender::prepare(
                    &binding,
                    &frame.encode()?,
                    &{
                        let mut options = MessageOptions::fresh(
                            mvm_contract::protocol::mailbox::ContentType::parse(
                                "application/spiralcore-object",
                                1,
                            )
                            .unwrap(),
                            now(),
                            2000,
                        )
                        .unwrap();
                        options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
                        options
                    },
                    now(),
                )
                .unwrap(),
                now(),
            )
            .unwrap();
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
            let accepted = caller
                .admit(
                    &reverse,
                    &mvm_core::mailbox::encode_envelope(&envelope).unwrap(),
                    now(),
                )
                .unwrap();
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
    })
    .unwrap();
    assert_eq!(stats.object, object_id(&expected));
    assert!(exact);
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
    assert!(endpoint.handle(&binding, &message, now()).is_err());
    assert!(
        endpoint
            .read_object(&binding, &message, object_id(&[]), now())
            .is_err()
    );
}
