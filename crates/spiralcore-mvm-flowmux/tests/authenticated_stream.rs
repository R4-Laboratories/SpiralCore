use spiralcore_mvm_flowmux::{
    Duplex, MappedStream, ObjectChannel, Role,
    scope::{PinnedScopePolicy, ScopeAuthority, ScopeFacts},
    tls::{PinnedIdentity, TlsCarrier},
};
use spiralcore_transfer::{
    endpoint::Limits,
    v1::{Capabilities, ObjectEndpoint, ObjectMode, ObjectSender},
};
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    path::PathBuf,
    time::{Duration, Instant},
};
struct Store(PathBuf);
impl Drop for Store {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn identity() -> PinnedIdentity {
    let key =
        ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    PinnedIdentity::from_pkcs8(key.as_ref().to_vec()).unwrap()
}
fn pair() -> (TlsCarrier, TlsCarrier, ScopeFacts) {
    let client = identity();
    let server = identity();
    let client_pin = client.spki_sha256();
    let server_pin = server.spki_sha256();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let actual_peer = listener.local_addr().unwrap();
    let raw = TcpStream::connect(actual_peer).unwrap();
    let (peer, _) = listener.accept().unwrap();
    let (left, right) = std::thread::scope(|scope| {
        let peer = scope.spawn(move || {
            TlsCarrier::accept(
                peer,
                &server,
                client_pin,
                Instant::now() + Duration::from_secs(2),
            )
            .unwrap()
        });
        let left = TlsCarrier::connect(
            raw,
            &client,
            server_pin,
            "peer.example",
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        (left, peer.join().unwrap())
    });
    let facts = ScopeFacts {
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
        actual_peer,
        route: "peer".into(),
        initiator_spki: client_pin,
        acceptor_spki: server_pin,
        tls_exporter: left.exporter(),
    };
    assert_eq!(left.exporter(), right.exporter());
    (left, right, facts)
}
fn authority(policy: &PinnedScopePolicy, facts: &ScopeFacts, role: Role) -> ScopeAuthority {
    let peer = match role {
        Role::Initiator => facts.acceptor_spki,
        Role::Acceptor => facts.initiator_spki,
    };
    policy
        .enroll(facts, peer, role, Instant::now() + Duration::from_secs(3))
        .unwrap();
    policy.authorize_scope(facts, peer, role).unwrap()
}
fn caps() -> Capabilities {
    let mut caps = Capabilities::full();
    caps.max_object_bytes = 65536;
    caps
}
fn duplex(socket: TlsCarrier, authority: &ScopeAuthority, role: Role, store: &Store) -> Duplex {
    let caps = caps();
    let channel = ObjectChannel::from_authenticated(
        socket,
        authority,
        Duration::from_secs(2),
        role,
        caps,
        caps,
    )
    .unwrap();
    let profile = channel.profile();
    let sender =
        ObjectSender::new_with_profile(ObjectMode::Whole, authority.context(), None, profile)
            .unwrap();
    let endpoint =
        ObjectEndpoint::open(&store.0, Limits::default(), authority.clone(), profile).unwrap();
    Duplex::spawn(channel, sender, endpoint, authority.context().clone(), role).unwrap()
}
#[test]
fn authenticated_object_stream_retains_bidirectional_fidelity_half_close_and_actual_tls_counts() {
    let (left, right, facts) = pair();
    let left_counts = left.try_clone().unwrap();
    let right_counts = right.try_clone().unwrap();
    let policy = PinnedScopePolicy::new(2).unwrap();
    let left_authority = authority(&policy, &facts, Role::Initiator);
    let right_authority = authority(&policy, &facts, Role::Acceptor);
    let stores = [
        Store(std::env::temp_dir().join(format!("flow-auth-left-{}", std::process::id()))),
        Store(std::env::temp_dir().join(format!("flow-auth-right-{}", std::process::id()))),
    ];
    let mut left =
        MappedStream::from_duplex(duplex(left, &left_authority, Role::Initiator, &stores[0]));
    let mut right =
        MappedStream::from_duplex(duplex(right, &right_authority, Role::Acceptor, &stores[1]));
    std::thread::scope(|scope| {
        let peer = scope.spawn(move || {
            let mut request = Vec::new();
            right.read_to_end(&mut request).unwrap();
            assert_eq!(request, vec![37; 20000]);
            right.write_all(&[9; 4096]).unwrap();
            right.shutdown(Shutdown::Write).unwrap();
        });
        left.write_all(&vec![37; 20000]).unwrap();
        left.shutdown(Shutdown::Write).unwrap();
        let mut response = Vec::new();
        left.read_to_end(&mut response).unwrap();
        assert_eq!(response, [9; 4096]);
        peer.join().unwrap();
    });
    let a = left_counts.counters();
    let b = right_counts.counters();
    assert_eq!(a.sent, b.received);
    assert_eq!(a.received, b.sent);
    assert!(a.sent > 0 && a.received > 0);
    println!(
        "TLS_FLOWMUX_COUNTERS logical_tx=20000 logical_rx=4096 cipher_tx={} cipher_rx={} peer_tx={} peer_rx={}",
        a.sent, a.received, b.sent, b.received
    );
}
#[test]
fn authenticated_scope_rejects_changed_exporter_foreign_session_and_reentry_after_revoke() {
    let (left, right, facts) = pair();
    let policy = PinnedScopePolicy::new(2).unwrap();
    let changed = ScopeFacts {
        tls_exporter: [8; 32],
        ..facts.clone()
    };
    let wrong = authority(&policy, &changed, Role::Initiator);
    assert!(
        ObjectChannel::from_authenticated(
            left.try_clone().unwrap(),
            &wrong,
            Duration::from_secs(1),
            Role::Initiator,
            caps(),
            caps()
        )
        .is_err()
    );
    let actual = authority(&policy, &facts, Role::Initiator);
    let mut channel = ObjectChannel::from_authenticated(
        left,
        &actual,
        Duration::from_secs(1),
        Role::Initiator,
        caps(),
        caps(),
    )
    .unwrap();
    policy.revoke_all().unwrap();
    assert!(actual.check().is_err());
    assert!(channel.half_close().is_err());
    drop(right);
}
#[test]
fn authenticated_direction_cannot_be_replaced_when_spawning_duplex() {
    let (left, _right, facts) = pair();
    let policy = PinnedScopePolicy::new(1).unwrap();
    let actual = authority(&policy, &facts, Role::Initiator);
    let caps = caps();
    let channel = ObjectChannel::from_authenticated(
        left,
        &actual,
        Duration::from_secs(1),
        Role::Initiator,
        caps,
        caps,
    )
    .unwrap();
    let profile = channel.profile();
    let sender =
        ObjectSender::new_with_profile(ObjectMode::Whole, actual.context(), None, profile).unwrap();
    let store = Store(std::env::temp_dir().join(format!("flow-auth-role-{}", std::process::id())));
    let endpoint =
        ObjectEndpoint::open(&store.0, Limits::default(), actual.clone(), profile).unwrap();
    assert!(
        Duplex::spawn(
            channel,
            sender,
            endpoint,
            actual.context().clone(),
            Role::Acceptor
        )
        .is_err(),
        "a trusted TLS initiator role must not be overwritten at duplex startup"
    );
}

#[test]
fn revocation_fences_already_buffered_application_bytes() {
    let (left, right, facts) = pair();
    let policy = PinnedScopePolicy::new(2).unwrap();
    let left_authority = authority(&policy, &facts, Role::Initiator);
    let right_authority = authority(&policy, &facts, Role::Acceptor);
    let stores = [
        Store(std::env::temp_dir().join(format!("flow-revoke-left-{}", std::process::id()))),
        Store(std::env::temp_dir().join(format!("flow-revoke-right-{}", std::process::id()))),
    ];
    let mut left =
        MappedStream::from_duplex(duplex(left, &left_authority, Role::Initiator, &stores[0]));
    let mut right =
        MappedStream::from_duplex(duplex(right, &right_authority, Role::Acceptor, &stores[1]));
    right.write_all(&[21; 1024]).unwrap();
    let mut byte = [0; 1];
    left.read_exact(&mut byte).unwrap();
    assert_eq!(byte, [21]);
    policy.revoke_all().unwrap();
    assert!(
        left.read(&mut byte).is_err(),
        "revoked scope must not release an already verified buffered suffix"
    );
    assert!(left.write(b"after revoke").is_err());
}
