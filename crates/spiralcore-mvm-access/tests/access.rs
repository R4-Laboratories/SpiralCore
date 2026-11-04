use chrono::{Duration, TimeZone, Utc};
use ed25519_dalek::{Signer, SigningKey};
use mvm_contract::{
    plan::{VerbGrant, VerbId},
    policy::security::AgentProfile,
};
use mvm_core::plan::{sign_plan, signing::test_support::sample_plan};
use spiralcore_mvm_access::{
    AccessControl, AccessService, Admission, AuditRecord, Error, LifecycleBinding, LocalPolicy,
    SharedBudget, Time,
};
use std::{sync::Arc, time::Instant};

#[derive(Clone, Default)]
struct Sink(
    Arc<std::sync::Mutex<Vec<AuditRecord>>>,
    Arc<std::sync::atomic::AtomicBool>,
);
impl spiralcore_mvm_access::Audit for Sink {
    fn append(&mut self, record: &AuditRecord) -> spiralcore_mvm_access::Result<()> {
        if self.1.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(Error::Audit);
        }
        self.0.lock().unwrap().push(record.clone());
        Ok(())
    }
}
struct Fixture {
    signed: mvm_core::plan::SignedExecutionPlan,
    grant: VerbGrant,
    plan_key: ed25519_dalek::VerifyingKey,
    host_key: ed25519_dalek::VerifyingKey,
    control: AccessControl,
    binding: LifecycleBinding,
    time: Arc<std::sync::Mutex<Time>>,
    sink: Sink,
}
impl Fixture {
    fn new() -> Self {
        let now = Utc.with_ymd_and_hms(2026, 11, 3, 12, 0, 0).unwrap();
        let plan_key = SigningKey::from_bytes(&[17; 32]);
        let host_key = SigningKey::from_bytes(&[18; 32]);
        let mut plan = sample_plan();
        plan.valid_from = now - Duration::seconds(1);
        plan.valid_until = now + Duration::seconds(60);
        plan.agent_verbs = Some(vec![VerbId::new("exec").unwrap()]);
        let signed = sign_plan(&plan, &plan_key, "fixture-plan-signer");
        let mut grant = VerbGrant {
            session_id: "fixture-instance".into(),
            plan_nonce: plan.nonce.clone(),
            not_after: now + Duration::seconds(30),
            verbs: vec![VerbId::new("exec").unwrap()],
            drive: None,
            tool_mediation: None,
            application_terminal: None,
            owner_access: None,
            sig: Vec::new(),
        };
        grant.sig = host_key.sign(&grant.signing_bytes()).to_bytes().to_vec();
        let binding = LifecycleBinding {
            instance: grant.session_id.clone(),
            tenant: plan.tenant.0.clone(),
            image_sha256: plan.image.sha256.clone(),
            revocation_generation: 1,
            profile: AgentProfile::Dev,
        };
        Self {
            signed,
            grant,
            plan_key: plan_key.verifying_key(),
            host_key: host_key.verifying_key(),
            control: AccessControl::new(binding.clone()),
            binding,
            time: Arc::new(std::sync::Mutex::new(Time {
                unix: now,
                monotonic: Instant::now(),
            })),
            sink: Sink::default(),
        }
    }
    fn service(&self, budget: SharedBudget) -> spiralcore_mvm_access::Result<AccessService<Sink>> {
        let time = self.time.clone();
        AccessService::admit(
            Admission {
                signed: &self.signed,
                keys: &[("fixture-plan-signer", &self.plan_key)],
                grant: self.grant.clone(),
                host: &self.host_key,
                control: self.control.clone(),
                policy: LocalPolicy::new(["local-owner"], ["exec"]),
                budget,
                clock: Arc::new(move || *time.lock().unwrap()),
            },
            self.sink.clone(),
        )
    }
}

#[test]
fn original_deadline_expiry_cancels_retained_charge_before_next_forward() {
    let f = Fixture::new();
    let budget = SharedBudget::new(Default::default()).unwrap();
    let mut service = f.service(budget.clone()).unwrap();
    let call = service.begin("local-owner", "exec", [4; 32], 7).unwrap();
    let mut time = f.time.lock().unwrap();
    time.monotonic += std::time::Duration::from_secs(3);
    time.unix += Duration::seconds(3);
    drop(time);
    let mut invoked = false;
    assert_eq!(
        service.forward(&call, "local-owner", &f.binding, 1, || {
            invoked = true;
            Ok(())
        }),
        Err(Error::Expired)
    );
    assert!(!invoked);
    assert_eq!(
        budget.usage().unwrap(),
        (0, 0),
        "expired work cannot pin the admitted host budget indefinitely"
    );
    assert_eq!(service.counters().completed, 0);
}

#[test]
fn genuine_dev_lifecycle_refuses_replay_foreign_handles_and_revocation() {
    let f = Fixture::new();
    let budget = SharedBudget::new(Default::default()).unwrap();
    let mut first = f.service(budget.clone()).unwrap();
    let mut second = f.service(budget.clone()).unwrap();
    assert!(matches!(
        first.begin("foreign-owner", "exec", [1; 32], 7),
        Err(Error::Owner)
    ));
    let call = first.begin("local-owner", "exec", [5; 32], 7).unwrap();
    assert_eq!(
        second.forward(&call, "local-owner", &f.binding, 1, || panic!(
            "foreign handle reached IO"
        )),
        Err(Error::State)
    );
    first
        .forward(&call, "local-owner", &f.binding, 7, || Ok(()))
        .unwrap();
    first.complete(call).unwrap();
    assert!(matches!(
        first.begin("local-owner", "exec", [5; 32], 7),
        Err(Error::Replay)
    ));
    let call = first.begin("local-owner", "exec", [6; 32], 7).unwrap();
    f.control.revoke().unwrap();
    assert_eq!(
        first.forward(&call, "local-owner", &f.binding, 1, || panic!(
            "revoked call reached IO"
        )),
        Err(Error::Revoked)
    );
    assert_eq!(budget.usage().unwrap(), (0, 0));
    assert_eq!(first.counters().completed, 1);
    assert_eq!(first.counters().forwarded_frames, 1);
    assert!(
        f.sink
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|record| record.reason == Some(Error::Revoked)),
        "a refusal before forwarding must still be audited"
    );
}

#[test]
fn identity_signature_expiry_epoch_and_audit_failures_refuse_without_io() {
    for mode in 0..7 {
        let mut f = Fixture::new();
        match mode {
            0 => f.signed.0.signature[0] ^= 1,
            1 => f.grant.sig[0] ^= 1,
            2 => {
                let mut binding = f.binding.clone();
                binding.tenant = "foreign".into();
                f.control.transition(binding).unwrap();
            }
            3 => {
                let mut binding = f.binding.clone();
                binding.instance = "foreign".into();
                f.control.transition(binding).unwrap();
            }
            4 => f.time.lock().unwrap().unix += Duration::seconds(60),
            5 => f.sink.1.store(true, std::sync::atomic::Ordering::SeqCst),
            _ => {
                let mut binding = f.binding.clone();
                binding.revocation_generation = 0;
                f.control.transition(binding).unwrap();
            }
        }
        let budget = SharedBudget::new(Default::default()).unwrap();
        assert!(f.service(budget.clone()).is_err());
        assert_eq!(budget.usage().unwrap(), (0, 0));
    }
    let f = Fixture::new();
    let budget = SharedBudget::new(Default::default()).unwrap();
    let mut service = f.service(budget.clone()).unwrap();
    let call = service.begin("local-owner", "exec", [7; 32], 7).unwrap();
    let mut changed = f.binding.clone();
    changed.revocation_generation = 2;
    f.control.transition(changed).unwrap();
    assert_eq!(
        service.forward(&call, "local-owner", &f.binding, 1, || panic!(
            "stale epoch IO"
        )),
        Err(Error::Epoch)
    );
    assert_eq!(budget.usage().unwrap(), (0, 0));
    let f = Fixture::new();
    let budget = SharedBudget::new(Default::default()).unwrap();
    let mut service = f.service(budget.clone()).unwrap();
    let call = service.begin("local-owner", "exec", [8; 32], 7).unwrap();
    f.sink.1.store(true, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        service.forward(&call, "local-owner", &f.binding, 1, || panic!(
            "unaudited IO"
        )),
        Err(Error::Audit)
    );
    assert_eq!(budget.usage().unwrap(), (0, 0));
}

#[test]
fn shared_reply_reservations_bound_two_services_before_forwarding() {
    let now = Utc.with_ymd_and_hms(2026, 11, 3, 12, 0, 0).unwrap();
    let plan_key = SigningKey::from_bytes(&[17; 32]);
    let host_key = SigningKey::from_bytes(&[18; 32]);
    let mut plan = sample_plan();
    plan.valid_from = now - Duration::seconds(1);
    plan.valid_until = now + Duration::seconds(60);
    plan.agent_verbs = Some(vec![VerbId::new("exec").unwrap()]);
    let signed = sign_plan(&plan, &plan_key, "fixture-plan-signer");
    let mut grant = VerbGrant {
        session_id: "fixture-instance".into(),
        plan_nonce: plan.nonce.clone(),
        not_after: now + Duration::seconds(30),
        verbs: vec![VerbId::new("exec").unwrap()],
        drive: None,
        tool_mediation: None,
        application_terminal: None,
        owner_access: None,
        sig: Vec::new(),
    };
    grant.sig = host_key.sign(&grant.signing_bytes()).to_bytes().to_vec();
    let binding = LifecycleBinding {
        instance: grant.session_id.clone(),
        tenant: plan.tenant.0.clone(),
        image_sha256: plan.image.sha256.clone(),
        revocation_generation: 1,
        profile: AgentProfile::Dev,
    };
    let limits = spiralcore_mvm_access::Limits {
        reply_bytes: 64 * 1024,
        ..Default::default()
    };
    let budget = SharedBudget::new(limits).unwrap();
    let clock: Arc<dyn spiralcore_mvm_access::Clock> = Arc::new(move || Time {
        unix: now,
        monotonic: Instant::now(),
    });
    let make = || {
        AccessService::admit(
            Admission {
                signed: &signed,
                keys: &[("fixture-plan-signer", &plan_key.verifying_key())],
                grant: grant.clone(),
                host: &host_key.verifying_key(),
                control: AccessControl::new(binding.clone()),
                policy: LocalPolicy::new(["local-owner"], ["exec"]),
                budget: budget.clone(),
                clock: clock.clone(),
            },
            |_: &AuditRecord| Ok(()),
        )
        .unwrap()
    };
    let (mut first, mut second) = (make(), make());
    let call = first.begin("local-owner", "exec", [2; 32], 7).unwrap();
    assert!(
        matches!(
            second.begin("local-owner", "exec", [3; 32], 7),
            Err(Error::Busy)
        ),
        "another service cannot oversubscribe the shared retained reply budget"
    );
    first.cancel(call).unwrap();
    let call = second.begin("local-owner", "exec", [3; 32], 7).unwrap();
    second.cancel(call).unwrap();
}

#[test]
fn sealed_profile_refuses_shell_even_with_genuine_signed_dev_grant() {
    let now = Utc.with_ymd_and_hms(2026, 11, 3, 12, 0, 0).unwrap();
    let plan_key = SigningKey::from_bytes(&[17; 32]);
    let host_key = SigningKey::from_bytes(&[18; 32]);
    let mut plan = sample_plan();
    plan.valid_from = now - Duration::seconds(1);
    plan.valid_until = now + Duration::seconds(60);
    plan.agent_verbs = Some(vec![VerbId::new("exec").unwrap()]);
    let signed = sign_plan(&plan, &plan_key, "fixture-plan-signer");
    let mut grant = VerbGrant {
        session_id: "fixture-instance".into(),
        plan_nonce: plan.nonce.clone(),
        not_after: now + Duration::seconds(30),
        verbs: vec![VerbId::new("exec").unwrap()],
        drive: None,
        tool_mediation: None,
        application_terminal: None,
        owner_access: None,
        sig: Vec::new(),
    };
    grant.sig = host_key.sign(&grant.signing_bytes()).to_bytes().to_vec();
    let binding = LifecycleBinding {
        instance: "fixture-instance".into(),
        tenant: plan.tenant.0.clone(),
        image_sha256: plan.image.sha256.clone(),
        revocation_generation: 1,
        profile: AgentProfile::SealedProd,
    };
    let policy = LocalPolicy::new(["local-owner"], ["exec"]);
    let mut audit = Vec::<AuditRecord>::new();
    let mut service = AccessService::admit(
        Admission {
            signed: &signed,
            keys: &[("fixture-plan-signer", &plan_key.verifying_key())],
            grant,
            host: &host_key.verifying_key(),
            control: AccessControl::new(binding),
            policy,
            budget: SharedBudget::new(Default::default()).unwrap(),
            clock: Arc::new(move || Time {
                unix: now,
                monotonic: Instant::now(),
            }),
        },
        |record: &AuditRecord| {
            audit.push(record.clone());
            Ok(())
        },
    )
    .unwrap();
    assert!(matches!(
        service.begin("local-owner", "exec", [1; 32], 7),
        Err(Error::Profile)
    ));
    assert_eq!(service.counters().admitted, 0);
    drop(service);
    let encoded = serde_json::to_string(&audit).unwrap();
    assert!(!encoded.contains("payload"));
    assert!(!encoded.contains("command"));
    assert!(
        audit
            .iter()
            .any(|record| record.reason == Some(Error::Profile))
    );
}
