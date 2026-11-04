use spiralcore_mvm_flowmux::{
    Role,
    scope::{PinnedScopePolicy, ScopeFacts},
};
use spiralcore_transfer::endpoint::Authority;
use std::time::{Duration, Instant};
fn facts() -> ScopeFacts {
    ScopeFacts {
        policy_sha256: [1; 32],
        plan_id: "actual-plan".into(),
        plan_version: 1,
        plan_nonce: [2; 16],
        bundle_sha256: [3; 32],
        tenant: "tenant".into(),
        instance: "instance".into(),
        session: "session".into(),
        guest_key: [4; 32],
        stream_id: 1,
        requested_host: "peer.example".into(),
        requested_port: 443,
        actual_peer: "127.0.0.1:8443".parse().unwrap(),
        route: "peer".into(),
        initiator_spki: [5; 32],
        acceptor_spki: [6; 32],
        tls_exporter: [7; 32],
    }
}
#[test]
fn explicit_enrollment_refuses_unknown_foreign_changed_expired_and_revoked_scopes() {
    let policy = PinnedScopePolicy::new(2).unwrap();
    let facts = facts();
    let until = Instant::now() + Duration::from_secs(2);
    assert!(
        policy
            .authorize_scope(&facts, [5; 32], Role::Acceptor)
            .is_err()
    );
    policy
        .enroll(&facts, [5; 32], Role::Acceptor, until)
        .unwrap();
    let authority = policy
        .authorize_scope(&facts, [5; 32], Role::Acceptor)
        .unwrap();
    let context = facts.context().unwrap();
    authority.authorize(&context).unwrap();
    assert!(
        policy
            .authorize_scope(&facts, [6; 32], Role::Acceptor)
            .is_err()
    );
    assert!(
        policy
            .authorize_scope(&facts, [5; 32], Role::Initiator)
            .is_err()
    );
    for changed in [
        ScopeFacts {
            tenant: "foreign".into(),
            ..facts.clone()
        },
        ScopeFacts {
            plan_nonce: [9; 16],
            ..facts.clone()
        },
        ScopeFacts {
            plan_id: "foreign".into(),
            ..facts.clone()
        },
    ] {
        assert!(
            policy
                .authorize_scope(&changed, [5; 32], Role::Acceptor)
                .is_err()
        );
        assert!(authority.authorize(&changed.context().unwrap()).is_err());
    }
    assert!(
        policy
            .enroll(
                &facts,
                [5; 32],
                Role::Acceptor,
                Instant::now() - Duration::from_millis(1)
            )
            .is_err()
    );
    policy.revoke_all().unwrap();
    assert!(authority.authorize(&context).is_err());
    assert!(
        policy
            .authorize_scope(&facts, [5; 32], Role::Acceptor)
            .is_err()
    );
    policy
        .enroll(
            &facts,
            [5; 32],
            Role::Acceptor,
            Instant::now() + Duration::from_millis(20),
        )
        .unwrap();
    let expiring = policy
        .authorize_scope(&facts, [5; 32], Role::Acceptor)
        .unwrap();
    std::thread::sleep(Duration::from_millis(30));
    assert!(expiring.authorize(&context).is_err());
}

#[test]
fn bounded_enrollment_and_replacement_never_revive_an_old_authority() {
    let policy = PinnedScopePolicy::new(1).unwrap();
    let facts = facts();
    let until = Instant::now() + Duration::from_secs(2);
    policy
        .enroll(&facts, [5; 32], Role::Acceptor, until)
        .unwrap();
    let original = policy
        .authorize_scope(&facts, [5; 32], Role::Acceptor)
        .unwrap();
    let other = ScopeFacts {
        stream_id: 3,
        ..facts.clone()
    };
    assert!(
        policy
            .enroll(&other, [5; 32], Role::Acceptor, until)
            .is_err()
    );
    policy
        .enroll(&facts, [5; 32], Role::Acceptor, until)
        .unwrap();
    assert!(original.check().is_err());
    let renewed = policy
        .authorize_scope(&facts, [5; 32], Role::Acceptor)
        .unwrap();
    renewed.check().unwrap();
    policy.revoke_all().unwrap();
    policy
        .enroll(&facts, [5; 32], Role::Acceptor, until)
        .unwrap();
    assert!(original.check().is_err());
    assert!(renewed.check().is_err());
}
