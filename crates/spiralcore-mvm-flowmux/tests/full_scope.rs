use spiralcore_mvm_flowmux::scope::ScopeFacts;
fn facts() -> ScopeFacts {
    ScopeFacts {
        policy_sha256: [1; 32],
        plan_id: "actual-plan-id".into(),
        plan_version: 7,
        plan_nonce: [2; 16],
        bundle_sha256: [3; 32],
        tenant: "tenant".into(),
        instance: "instance".into(),
        session: "authenticated-network-flow-session".into(),
        guest_key: [4; 32],
        stream_id: 3,
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
fn full_actual_facts_bind_context_without_numeric_epoch_authority() {
    let actual = facts();
    let context = actual.context().unwrap();
    let binding = actual.binding().unwrap();
    assert_eq!(context.policy_epoch, 0);
    assert_eq!(context.execution_epoch, 0);
    assert_eq!(context.namespace().unwrap(), binding.flow);
    assert_eq!(binding.policy_sha256, [1; 32]);
    assert_eq!(
        actual.canonical_bytes().unwrap(),
        facts().canonical_bytes().unwrap()
    );
    let original = binding.scope_commitment;
    let mut changes = Vec::new();
    macro_rules! changed {
        ($field:ident,$value:expr) => {{
            let mut other = actual.clone();
            other.$field = $value;
            changes.push(other);
        }};
    }
    changed!(policy_sha256, [9; 32]);
    changed!(plan_id, "other-plan".into());
    changed!(plan_version, 8);
    changed!(plan_nonce, [9; 16]);
    changed!(bundle_sha256, [9; 32]);
    changed!(tenant, "other".into());
    changed!(instance, "other".into());
    changed!(session, "other".into());
    changed!(guest_key, [9; 32]);
    changed!(stream_id, 5);
    changed!(requested_host, "other.example".into());
    changed!(requested_port, 8443);
    changed!(actual_peer, "127.0.0.1:9443".parse().unwrap());
    changed!(route, "egress".into());
    changed!(initiator_spki, [9; 32]);
    changed!(acceptor_spki, [9; 32]);
    changed!(tls_exporter, [9; 32]);
    for other in changes {
        assert_ne!(other.binding().unwrap().scope_commitment, original);
        assert_ne!(
            other.context().unwrap().namespace().unwrap(),
            context.namespace().unwrap()
        );
    }
    let mut invalid = actual.clone();
    invalid.session.clear();
    assert!(invalid.binding().is_err());
    invalid = actual;
    invalid.plan_version = 0;
    assert!(invalid.context().is_err());
}
