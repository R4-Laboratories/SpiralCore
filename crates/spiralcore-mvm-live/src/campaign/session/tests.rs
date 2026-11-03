use super::*;
use crate::config::{Arm, Pair, Plan, Stage, now_ms};
use ed25519_dalek::VerifyingKey;
use spiralcore_mvm_adapter::{
    issuance::IssuancePolicy,
    receipt::ReceiptTracker,
    service::{
        ControlReserve, HostBudget, LocalMailboxService, MailboxBinding, ServiceError,
        ServiceLimits, ServiceTime,
    },
    service_sender::{self, MessageOptions, OrderingStream},
};

fn fixture() -> (Pair, MailboxBinding, MailboxBinding, VerifyingKey) {
    let pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 128,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    let outgoing = pair
        .sender
        .outgoing
        .as_ref()
        .unwrap()
        .service_binding(None)
        .unwrap();
    let profile = pair.sender.service.as_ref().unwrap();
    let control = profile.incoming_control.service_binding(None).unwrap();
    let peer = VerifyingKey::from_bytes(&profile.peer_receipt_verification).unwrap();
    (pair, outgoing, control, peer)
}
fn pending(binding: &MailboxBinding, pair: &Pair, now: ServiceTime) -> PendingCall {
    let mut stream = OrderingStream::new();
    let mut options = stream
        .next_for_binding(
            binding,
            mvm_contract::protocol::mailbox::ContentType::parse(
                spiralcore_mvm_adapter::CONTENT_TYPE,
                1,
            )
            .unwrap(),
            now,
            now.unix_ms + 30_000,
            IssuancePolicy::Bounded {
                max_clock_ahead_ms: 5000,
            },
        )
        .unwrap();
    options.reply_to = pair.sender.outgoing.as_ref().unwrap().reply_to;
    PendingCall::new(b"fixed\0request\0".to_vec(), options, now)
}
fn budget() -> HostBudget {
    HostBudget::with_control(ServiceLimits::default(), ControlReserve::default()).unwrap()
}

#[test]
fn tracker_refusal_retains_partial_call_and_exact_sealed_request_until_capacity_returns() {
    let (pair, outgoing, control, peer) = fixture();
    let start = ServiceTime {
        unix_ms: now_ms().unwrap(),
        monotonic_ms: 0,
    };
    let owner = budget();
    let mut tracker = ReceiptTracker::new(owner.clone(), 1).unwrap();
    let mut options = MessageOptions::fresh(
        mvm_contract::protocol::mailbox::ContentType::parse(
            spiralcore_mvm_adapter::CONTENT_TYPE,
            1,
        )
        .unwrap(),
        start,
        start.unix_ms + 1000,
    )
    .unwrap();
    options.reply_to = pair.sender.outgoing.as_ref().unwrap().reply_to;
    let old = service_sender::prepare(&outgoing, b"old", &options, start).unwrap();
    let old_wire = mvm_core::mailbox::encode_envelope(old.envelope()).unwrap();
    tracker
        .register_send(&outgoing, &control, &peer, &old_wire, start)
        .unwrap();
    let now = ServiceTime {
        unix_ms: start.unix_ms + 300_999,
        monotonic_ms: 300_999,
    };
    let mut call = pending(&outgoing, &pair, now);
    let mut service = LocalMailboxService::new(owner.clone()).unwrap();
    assert!(matches!(
        call.register(&mut service, &mut tracker, &outgoing, &control, &peer, now),
        Err(SessionError::Service(ServiceError::Full))
    ));
    let first_handle = call.call_handle().unwrap();
    assert!(call.send_handle().is_none());
    assert!(!call.send_started());
    let wire = call.encoded().unwrap().to_vec();
    let header = call.prepared().unwrap().header().clone();
    let after = ServiceTime {
        unix_ms: now.unix_ms + 1,
        monotonic_ms: now.monotonic_ms + 1,
    };
    call.register(
        &mut service,
        &mut tracker,
        &outgoing,
        &control,
        &peer,
        after,
    )
    .unwrap();
    assert_eq!(call.call_handle(), Some(first_handle));
    assert!(call.send_handle().is_some());
    assert_eq!(call.encoded().unwrap(), wire);
    assert_eq!(call.prepared().unwrap().header(), &header);
    assert_eq!(tracker.snapshot().retained_entries, 1);
    assert_eq!(owner.snapshot().unwrap().active_calls, 1);
    outgoing
        .replace_policy(pair.sender.outgoing.as_ref().unwrap().context().unwrap())
        .unwrap();
    assert!(matches!(
        call.register(
            &mut service,
            &mut tracker,
            &outgoing,
            &control,
            &peer,
            after
        ),
        Err(SessionError::Service(ServiceError::Unauthorized))
    ));
    assert_eq!(call.encoded().unwrap(), wire);
    let expired = ServiceTime {
        unix_ms: start.unix_ms + 626_000,
        monotonic_ms: 626_000,
    };
    assert!(matches!(
        call.register(
            &mut service,
            &mut tracker,
            &outgoing,
            &control,
            &peer,
            expired
        ),
        Err(SessionError::StreamFenced)
    ));
    assert_eq!(owner.snapshot().unwrap().active_calls, 0);
    assert_eq!(tracker.snapshot().retained_entries, 0);
    assert_eq!(call.encoded().unwrap(), wire);
}

#[test]
fn call_refusal_does_not_register_tracker_or_replace_the_prepared_identity() {
    let (pair, outgoing, control, peer) = fixture();
    let now = ServiceTime {
        unix_ms: now_ms().unwrap(),
        monotonic_ms: 0,
    };
    let mut limits = ServiceLimits::default();
    for scope in [
        &mut limits.host,
        &mut limits.tenant,
        &mut limits.mailbox,
        &mut limits.sender,
    ] {
        scope.active_calls = 1;
    }
    let owner = HostBudget::with_control(limits, ControlReserve::default()).unwrap();
    let mut service = LocalMailboxService::new(owner.clone()).unwrap();
    let mut tracker = ReceiptTracker::new(owner.clone(), 4).unwrap();
    let mut first = pending(&outgoing, &pair, now);
    first
        .register(&mut service, &mut tracker, &outgoing, &control, &peer, now)
        .unwrap();
    let mut second = pending(&outgoing, &pair, now);
    assert!(matches!(
        second.register(&mut service, &mut tracker, &outgoing, &control, &peer, now),
        Err(SessionError::Service(ServiceError::Full))
    ));
    let wire = second.encoded().unwrap().to_vec();
    assert!(second.call_handle().is_none());
    assert!(second.send_handle().is_none());
    assert_eq!(tracker.snapshot().retained_entries, 1);
    service
        .cancel(&outgoing, &first.call_handle().unwrap(), now)
        .unwrap();
    second
        .register(&mut service, &mut tracker, &outgoing, &control, &peer, now)
        .unwrap();
    assert_eq!(second.encoded().unwrap(), wire);
    assert_eq!(owner.snapshot().unwrap().active_calls, 1);
}

#[test]
fn pre_send_refusal_cannot_extend_original_lifetime_when_wall_clock_rolls_back() {
    let (pair, outgoing, control, peer) = fixture();
    let now = ServiceTime {
        unix_ms: now_ms().unwrap(),
        monotonic_ms: 0,
    };
    let mut limits = ServiceLimits::default();
    for scope in [
        &mut limits.host,
        &mut limits.tenant,
        &mut limits.mailbox,
        &mut limits.sender,
    ] {
        scope.active_calls = 1;
    }
    let owner = HostBudget::with_control(limits, ControlReserve::default()).unwrap();
    let mut service = LocalMailboxService::new(owner.clone()).unwrap();
    let mut tracker = ReceiptTracker::new(owner.clone(), 4).unwrap();
    let mut blocker = pending(&outgoing, &pair, now);
    blocker
        .register(&mut service, &mut tracker, &outgoing, &control, &peer, now)
        .unwrap();
    let mut request = pending(&outgoing, &pair, now);
    assert!(matches!(
        request.register(&mut service, &mut tracker, &outgoing, &control, &peer, now),
        Err(SessionError::Service(ServiceError::Full))
    ));
    let original = request.encoded().unwrap().to_vec();
    let after = ServiceTime {
        unix_ms: now.unix_ms,
        monotonic_ms: 25_000,
    };
    assert!(
        matches!(
            request.register(
                &mut service,
                &mut tracker,
                &outgoing,
                &control,
                &peer,
                after
            ),
            Err(SessionError::Service(ServiceError::Expired))
        ),
        "allocated request lifetime must not restart on retry"
    );
    assert_eq!(owner.snapshot().unwrap().active_calls, 0);
    assert_eq!(request.encoded().unwrap(), original);
    assert!(request.call_handle().is_none());
}

#[test]
fn starting_send_keeps_identity_but_blocks_implicit_registration_or_replacement() {
    let (pair, outgoing, control, peer) = fixture();
    let now = ServiceTime {
        unix_ms: now_ms().unwrap(),
        monotonic_ms: 0,
    };
    let owner = budget();
    let mut service = LocalMailboxService::new(owner.clone()).unwrap();
    let mut tracker = ReceiptTracker::new(owner.clone(), 4).unwrap();
    let mut call = pending(&outgoing, &pair, now);
    assert!(call.matches(b"fixed\0request\0"));
    assert!(!call.matches(b"different"));
    assert!(call.start_send().is_err());
    call.register(&mut service, &mut tracker, &outgoing, &control, &peer, now)
        .unwrap();
    let wire = call.encoded().unwrap().to_vec();
    call.start_send().unwrap();
    assert!(call.send_started());
    assert!(matches!(
        call.register(&mut service, &mut tracker, &outgoing, &control, &peer, now),
        Err(SessionError::SendUncertain)
    ));
    assert_eq!(call.encoded().unwrap(), wire);
    let expired = ServiceTime {
        unix_ms: now.unix_ms + 325_000,
        monotonic_ms: 325_000,
    };
    assert!(matches!(
        call.register(
            &mut service,
            &mut tracker,
            &outgoing,
            &control,
            &peer,
            expired
        ),
        Err(SessionError::StreamFenced)
    ));
    assert_eq!(owner.snapshot().unwrap().active_calls, 0);
    assert_eq!(tracker.snapshot().retained_entries, 0);
    assert_eq!(call.encoded().unwrap(), wire);
}
