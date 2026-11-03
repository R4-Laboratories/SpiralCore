#![cfg(feature = "live")]
mod common;

use ed25519_dalek::SigningKey;
use mvm_contract::protocol::mailbox::*;
use spiralcore_mvm_adapter::{
    Policy,
    broker::BrokerContext,
    service::{
        HostBudget, LocalMailboxService, MailboxBinding, ServiceBindingConfig, ServiceError,
        ServiceLimits,
    },
    service_sender::{self, MessageOptions},
};
use std::{
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

fn valid_binding(now: i64, identity: u8) -> MailboxBinding {
    let mut c = common::config(MailboxMessageKind::Call);
    let mut context = c.policy.snapshot().unwrap();
    context.expected_tenant = TenantId::from_bytes([identity; 32]);
    context.expected_mailbox = MailboxId::from_bytes([identity; 32]);
    context.expected_sender = PrincipalId::from_bytes([identity; 32]);
    context.acting_principal = context.expected_sender;
    context.now_unix_ms = now;
    c.policy = Policy::new(context.clone()).unwrap();
    c.capability.grant.subject = context.expected_sender;
    c.capability.grant.address.tenant = context.expected_tenant;
    c.capability.grant.address.mailbox = context.expected_mailbox;
    c.capability.grant.not_before_unix_ms = now - 1_000;
    c.capability.grant.expires_at_unix_ms = now + 60_000;
    c.capability =
        mvm_core::mailbox::sign_capability(c.capability.grant, &SigningKey::from_bytes(&[22; 32]))
            .unwrap();
    c.binding.claims.address.tenant = context.expected_tenant;
    c.binding.claims.address.mailbox = context.expected_mailbox;
    c.reply_to.as_mut().unwrap().address.tenant = context.expected_tenant;
    c.binding.claims.not_before_unix_ms = now - 1_000;
    c.binding.claims.expires_at_unix_ms = now + 60_000;
    c.binding = mvm_core::mailbox::transport::sign_transport_binding(
        c.binding.claims,
        &SigningKey::from_bytes(&[22; 32]),
    )
    .unwrap();
    MailboxBinding::new(ServiceBindingConfig::from_direction(c).unwrap(), None).unwrap()
}

#[test]
fn cloned_broker_sessions_share_aggregate_quota_and_released_credits() {
    let mut limits = ServiceLimits::default();
    limits.host.active_calls = 1;
    limits.tenant.active_calls = 1;
    limits.mailbox.active_calls = 1;
    limits.sender.active_calls = 1;
    let broker = BrokerContext::new(HostBudget::new(limits).unwrap());
    let other = broker.clone();
    let mut first = broker
        .with_time(|budget, _| LocalMailboxService::new(budget.clone()))
        .unwrap();
    let mut second = other
        .with_time(|budget, _| LocalMailboxService::new(budget.clone()))
        .unwrap();
    let (a, b, wa, wb) = broker
        .with_time::<_, ServiceError>(|_, now| {
            let a = valid_binding(now.unix_ms, 51);
            let b = valid_binding(now.unix_ms, 52);
            let encode = |binding: &MailboxBinding, identity: u8| {
                let mut options = MessageOptions::fresh(
                    ContentType::parse("application/spiralcore-object", 1).unwrap(),
                    now,
                    now.unix_ms + 10_000,
                )
                .unwrap();
                let mut reply = common::config(MailboxMessageKind::Call).reply_to.unwrap();
                reply.address.tenant = TenantId::from_bytes([identity; 32]);
                options.reply_to = Some(reply);
                mvm_core::mailbox::encode_envelope(
                    service_sender::prepare(binding, b"exact\0bytes", &options, now)
                        .unwrap()
                        .envelope(),
                )
                .unwrap()
            };
            let wa = encode(&a, 51);
            let wb = encode(&b, 52);
            Ok((a, b, wa, wb))
        })
        .unwrap();
    let call = broker
        .with_time(|_, now| first.register_call(&a, &wa, now))
        .unwrap();
    assert!(matches!(
        other.with_time(|_, now| second.register_call(&b, &wb, now)),
        Err(ServiceError::Full)
    ));
    broker
        .with_time::<_, ServiceError>(|budget, _| {
            use spiralcore_mvm_adapter::service::{BudgetPool, BudgetResource, BudgetScope};
            let snapshot = budget.snapshot()?;
            let reason = snapshot.refusals.last.unwrap();
            assert_eq!(snapshot.refusals.total, 1);
            assert_eq!(reason.scope, BudgetScope::Host);
            assert_eq!(reason.pool, BudgetPool::Total);
            assert_eq!(reason.resource, BudgetResource::ActiveCalls);
            assert_eq!((reason.current, reason.requested, reason.limit), (1, 1, 1));
            assert_eq!(snapshot.active_calls, 1);
            Ok(())
        })
        .unwrap();
    broker
        .with_time(|_, now| first.cancel(&a, &call, now))
        .unwrap();
    other
        .with_time(|_, now| second.register_call(&b, &wb, now))
        .unwrap();
}

#[test]
fn dispatch_samples_shared_clock_after_waiting_for_other_sessions() {
    let broker = BrokerContext::new(HostBudget::new(ServiceLimits::default()).unwrap());
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let broker = broker.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                let mut service = broker
                    .with_time(|budget, _| LocalMailboxService::new(budget.clone()))
                    .unwrap();
                barrier.wait();
                let mut last = 0;
                for _ in 0..16 {
                    broker
                        .with_time::<_, ServiceError>(|_, now| {
                            assert!(now.monotonic_ms >= last);
                            last = now.monotonic_ms;
                            thread::sleep(Duration::from_millis(1));
                            service.expire(now)?;
                            Ok(())
                        })
                        .unwrap();
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
}

#[test]
fn dispatch_returns_action_error_without_poisoning_future_work() {
    let broker = BrokerContext::new(HostBudget::new(ServiceLimits::default()).unwrap());
    assert_eq!(
        broker.with_time::<(), ServiceError>(|_, _| Err(ServiceError::Cancelled)),
        Err(ServiceError::Cancelled)
    );
    broker
        .with_time::<_, ServiceError>(|_, now| {
            assert!(now.unix_ms > 0);
            Ok(())
        })
        .unwrap();
}
