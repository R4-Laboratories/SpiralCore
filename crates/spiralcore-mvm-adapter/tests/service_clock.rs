#![cfg(feature = "live")]
mod common;
#[path = "common/service.rs"]
mod fixture;

use mvm_contract::protocol::mailbox::*;
use mvm_core::mailbox::{self, InMemoryReplayProtector, MailboxError};
use spiralcore_mvm_adapter::{
    issuance::IssuancePolicy,
    receipt::{ReceiptObservation, ReceiptReturnConfig, ReceiptTracker},
    service::{ControlReserve, HostBudget, LocalMailboxService, ServiceLimits, ServiceTime},
    service_sender::{self, MessageOptions},
};

fn guest() -> ServiceTime {
    ServiceTime {
        unix_ms: 1100,
        ..fixture::time()
    }
}
fn host() -> ServiceTime {
    ServiceTime {
        unix_ms: 1900,
        monotonic_ms: 200,
    }
}
fn budget() -> HostBudget {
    HostBudget::with_control(ServiceLimits::default(), ControlReserve::default()).unwrap()
}

#[test]
fn canonical_verifier_refuses_host_issued_header_ahead_of_guest_clock() {
    let config = common::config(MailboxMessageKind::Call);
    let bytes =
        common::sealed_payload_at(&config, b"public clock regression", host().unix_ms, 2000);
    let envelope = mailbox::decode_envelope(&bytes).unwrap();
    let mut context = config.policy.snapshot().unwrap();
    context.now_unix_ms = guest().unix_ms;
    assert!(matches!(
        mailbox::open_verified_envelope(
            &envelope,
            &config.capability,
            &context,
            &config.encryption_key,
            &config.sender_verifying_key,
            &config.authority_key,
            &mut InMemoryReplayProtector::default(),
        ),
        Err(MailboxError::IssuedInFuture)
    ));
}

#[test]
fn bounded_native_clock_lag_allows_receipt_without_relaxing_receive_time_checks() {
    let outgoing = fixture::binding(MailboxMessageKind::Call, true, false, true);
    let incoming = fixture::binding(MailboxMessageKind::Call, true, false, false);
    let control = fixture::binding(MailboxMessageKind::Cast, false, true, false);
    let owner = budget();
    let mut service = LocalMailboxService::new(owner.clone()).unwrap();
    let returns = ReceiptReturnConfig::new_with_policy(
        fixture::binding(MailboxMessageKind::Cast, false, true, true),
        owner,
        fixture::receipt_key(true).verifying_key(),
        IssuancePolicy::bounded(800).unwrap(),
    )
    .unwrap();
    let mut options = MessageOptions::fresh(fixture::content_type(), guest(), 2000).unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    let prepared =
        service_sender::prepare(&outgoing, b"public synthetic request", &options, guest()).unwrap();
    let encoded = mailbox::encode_envelope(prepared.envelope()).unwrap();
    let mut tracker = ReceiptTracker::new(budget(), 4).unwrap();
    let send = tracker
        .register_send(
            &outgoing,
            &control,
            &fixture::receipt_key(true).verifying_key(),
            &encoded,
            guest(),
        )
        .unwrap();
    let (_, receipt) = returns
        .admit(&mut service, &incoming, &encoded, host())
        .unwrap();
    assert!(matches!(
        tracker.observe_receipt(
            &control,
            receipt.encoded_bytes(),
            ServiceTime {
                unix_ms: 1099,
                ..guest()
            },
        ),
        Err(spiralcore_mvm_adapter::service::ServiceError::Unauthorized)
    ));
    tracker
        .observe_receipt(&control, receipt.encoded_bytes(), guest())
        .unwrap();
    assert!(matches!(
        tracker.take_receipt(&outgoing, &send, guest()).unwrap(),
        ReceiptObservation::Verified(_)
    ));

    // Complete the independent application Reply and its encrypted admission
    // receipt with the same 800 ms skew; neither receipt replaces that Reply.
    let source_owner = budget();
    let mut caller = LocalMailboxService::new(source_owner.clone()).unwrap();
    let call = caller.register_call(&outgoing, &encoded, guest()).unwrap();
    let proof = service.receive(&incoming, host()).unwrap().unwrap();
    let reply_out = fixture::binding(MailboxMessageKind::Reply, false, false, true);
    let reply_in = fixture::binding(MailboxMessageKind::Reply, false, false, false);
    let options = MessageOptions::for_reply_with_policy(
        &reply_out,
        &proof,
        host(),
        IssuancePolicy::bounded(800).unwrap(),
    )
    .unwrap();
    let reply = service
        .reply_prepared(
            &reply_out,
            proof.handle(),
            service_sender::request(&reply_out, b"exact clock-skew reply\0\0", &options, host())
                .unwrap(),
            host(),
        )
        .unwrap();
    let reply_wire = mailbox::encode_envelope(reply.envelope()).unwrap();
    let reply_control = fixture::binding(MailboxMessageKind::Cast, true, true, false);
    let mut host_tracker = ReceiptTracker::new(budget(), 4).unwrap();
    let reply_send = host_tracker
        .register_send(
            &reply_out,
            &reply_control,
            &fixture::receipt_key(false).verifying_key(),
            &reply_wire,
            host(),
        )
        .unwrap();
    let source_returns = ReceiptReturnConfig::new(
        fixture::binding(MailboxMessageKind::Cast, true, true, true),
        source_owner,
        fixture::receipt_key(false).verifying_key(),
    )
    .unwrap();
    let (_, reply_receipt) = source_returns
        .admit(&mut caller, &reply_in, &reply_wire, guest())
        .unwrap();
    tracker
        .observe_application_reply(&reply_in, &reply_wire, guest())
        .unwrap();
    assert_eq!(
        caller.poll_call(&outgoing, &call, guest()).unwrap(),
        spiralcore_mvm_adapter::service::CallStatus::Replied(Some(
            b"exact clock-skew reply\0\0".to_vec()
        ))
    );
    host_tracker
        .observe_receipt(&reply_control, reply_receipt.encoded_bytes(), host())
        .unwrap();
    let ReceiptObservation::Verified(ack) = host_tracker
        .take_receipt(&reply_out, &reply_send, host())
        .unwrap()
    else {
        panic!("missing authenticated Reply receipt")
    };
    assert!(
        service
            .acknowledge_reply(
                &reply_out,
                &ack,
                &fixture::receipt_key(false).verifying_key(),
                host()
            )
            .unwrap()
    );
    assert!(
        service
            .retry_reply_prepared(&incoming, &encoded, host())
            .unwrap()
            .is_none()
    );
}

#[test]
fn bounded_normal_issuance_is_stable_on_retry_and_keeps_destination_time_strict() {
    let outgoing = fixture::binding(MailboxMessageKind::Call, true, false, true);
    let incoming = fixture::binding(MailboxMessageKind::Call, true, false, false);
    let mut options = MessageOptions::fresh_for_binding(
        &outgoing,
        fixture::content_type(),
        host(),
        2000,
        IssuancePolicy::bounded(800).unwrap(),
    )
    .unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    assert_eq!(options.issued_at_unix_ms, guest().unix_ms);
    assert_eq!(options.expires_at_unix_ms, 2000);
    let first =
        service_sender::prepare(&outgoing, b"stable public retry", &options, host()).unwrap();
    let later = ServiceTime {
        unix_ms: 1950,
        monotonic_ms: 250,
    };
    let retry =
        service_sender::prepare(&outgoing, b"stable public retry", &options, later).unwrap();
    assert_eq!(first.header(), retry.header());
    let mut service = LocalMailboxService::new(budget()).unwrap();
    let wire = mailbox::encode_envelope(first.envelope()).unwrap();
    assert_eq!(
        service
            .admit(&incoming, &wire, guest())
            .unwrap()
            .receipt
            .outcome,
        DeliveryOutcome::AcceptedEphemeral
    );
    let mut expired = LocalMailboxService::new(budget()).unwrap();
    assert!(
        expired
            .admit(
                &incoming,
                &wire,
                ServiceTime {
                    unix_ms: 2000,
                    ..guest()
                }
            )
            .is_err()
    );
}

#[test]
fn young_reverse_clock_window_refuses_before_application_admission() {
    use spiralcore_mvm_adapter::service::ServiceError;
    let outgoing = fixture::binding(MailboxMessageKind::Call, true, false, true);
    let incoming = fixture::binding(MailboxMessageKind::Call, true, false, false);
    let owner = budget();
    let mut service = LocalMailboxService::new(owner.clone()).unwrap();
    let returns = ReceiptReturnConfig::new_with_policy(
        fixture::binding(MailboxMessageKind::Cast, false, true, true),
        owner,
        fixture::receipt_key(true).verifying_key(),
        IssuancePolicy::bounded(1001).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        MessageOptions::fresh_for_binding(
            &outgoing,
            fixture::content_type(),
            host(),
            2000,
            IssuancePolicy::bounded(1001).unwrap()
        ),
        Err(ServiceError::Unauthorized)
    ));
    let mut options = MessageOptions::fresh(fixture::content_type(), guest(), 2000).unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    let prepared =
        service_sender::prepare(&outgoing, b"must not be admitted", &options, guest()).unwrap();
    let encoded = mailbox::encode_envelope(prepared.envelope()).unwrap();
    assert!(matches!(
        returns.admit(&mut service, &incoming, &encoded, host()),
        Err(ServiceError::Unauthorized)
    ));
    assert!(service.receive(&incoming, host()).unwrap().is_none());
}
