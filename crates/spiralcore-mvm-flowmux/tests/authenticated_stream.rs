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
        object_profile: 1,
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
#[test]
fn authenticated_enrollment_refuses_substituted_core_resource_profile_before_mapping() {
    let (left, _right, mut facts) = pair();
    facts.object_profile = 2;
    let policy = PinnedScopePolicy::new(1).unwrap();
    let enrolled = authority(&policy, &facts, Role::Initiator);
    assert!(matches!(
        ObjectChannel::from_authenticated(
            left,
            &enrolled,
            Duration::from_secs(2),
            Role::Initiator,
            caps(),
            caps()
        ),
        Err(spiralcore_mvm_flowmux::Error::PeerMismatch)
    ));
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

#[test]
fn explicit_artifact_deadline_is_not_refreshed_after_tls_or_partial_progress() {
    let (left, _right, facts) = pair();
    let counters = left.try_clone().unwrap();
    let policy = PinnedScopePolicy::new(1).unwrap();
    let actual = authority(&policy, &facts, Role::Initiator);
    let store =
        Store(std::env::temp_dir().join(format!("flow-artifact-timeout-{}", std::process::id())));
    let stream = MappedStream::from_duplex(duplex(left, &actual, Role::Initiator, &store));
    let before = counters.counters();
    let until = Instant::now() - Duration::from_millis(1);
    assert!(stream.seed_schema_until(until).is_err());
    assert_eq!(
        counters.counters().sent,
        before.sent,
        "expired publication writes no artifact bytes"
    );
    let (left, _right, facts) = pair();
    let policy = PinnedScopePolicy::new(1).unwrap();
    let actual = authority(&policy, &facts, Role::Initiator);
    let store =
        Store(std::env::temp_dir().join(format!("flow-artifact-partial-{}", std::process::id())));
    let stream = MappedStream::from_duplex(duplex(left, &actual, Role::Initiator, &store));
    let until = Instant::now() + Duration::from_millis(50);
    let start = Instant::now();
    assert!(stream.seed_base_until(vec![3; 1024], until).is_err());
    assert!(
        start.elapsed() < Duration::from_millis(500),
        "original50ms budget cannot refresh into the stream2s timeout"
    );
    assert!(
        stream
            .seed_checkpoint_until(Instant::now() + Duration::from_secs(1))
            .is_err(),
        "failed original attempt fences artifact reentry"
    );
}

#[test]
fn small_profile_sustains_more_than_128_distinct_verified_blocks_in_one_scope() {
    use spiralcore_transfer::resource_profile::ResourceProfile;
    const BLOCKS: usize = 129;
    let (left, right, mut facts) = pair();
    facts.object_profile = 2;
    let policy = PinnedScopePolicy::new(2).unwrap();
    let left_authority = authority(&policy, &facts, Role::Initiator);
    let right_authority = authority(&policy, &facts, Role::Acceptor);
    let stores = [
        Store(
            std::env::temp_dir().join(format!("flow-small-sustained-left-{}", std::process::id())),
        ),
        Store(
            std::env::temp_dir().join(format!("flow-small-sustained-right-{}", std::process::id())),
        ),
    ];
    let engine = |socket: TlsCarrier, auth: &ScopeAuthority, role: Role, store: &Store| {
        let caps = Capabilities::small_v1();
        let channel = ObjectChannel::from_authenticated(
            socket,
            auth,
            Duration::from_secs(2),
            role,
            caps,
            caps,
        )
        .unwrap();
        let profile = channel.profile();
        let sender = ObjectSender::new_with_resource_profile(
            ObjectMode::Whole,
            auth.context(),
            None,
            profile,
            ResourceProfile::SmallV1,
        )
        .unwrap();
        let receiver = ObjectEndpoint::open_with_resource_profile(
            &store.0,
            ResourceProfile::SmallV1.receiver_limits(),
            auth.clone(),
            profile,
            ResourceProfile::SmallV1,
        )
        .unwrap();
        MappedStream::from_duplex(
            Duplex::spawn(channel, sender, receiver, auth.context().clone(), role).unwrap(),
        )
    };
    let mut left = engine(left, &left_authority, Role::Initiator, &stores[0]);
    let mut right = engine(right, &right_authority, Role::Acceptor, &stores[1]);
    fn block(index: usize) -> [u8; 16384] {
        let mut bytes = [0x61; 16384];
        bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
        bytes
    }
    let (sent, failure, received) = std::thread::scope(|scope| {
        let reader = scope.spawn(move || {
            for index in 0..BLOCKS {
                let mut bytes = [0; 16384];
                if let Err(error) = right.read_exact(&mut bytes) {
                    return Err((index, error.to_string()));
                }
                assert_eq!(bytes, block(index), "distinct verified block{index}");
            }
            let mut byte = [0; 1];
            assert_eq!(
                right.read(&mut byte).unwrap(),
                0,
                "application FIN after all blocks"
            );
            Ok(BLOCKS)
        });
        let until = Instant::now() + Duration::from_secs(5);
        let mut sent = 0;
        let mut failure = None;
        for index in 0..BLOCKS {
            if Instant::now() >= until {
                failure = Some("fixture original5s watchdog".into());
                break;
            }
            if let Err(error) = left.write_all(&block(index)) {
                failure = Some(error.to_string());
                break;
            }
            sent += 1;
        }
        let _ = left.shutdown(if failure.is_some() {
            Shutdown::Both
        } else {
            Shutdown::Write
        });
        (sent, failure, reader.join().unwrap())
    });
    eprintln!(
        "SMALL_SUSTAINED expected_blocks={BLOCKS} sent_verified_blocks={sent} receive={received:?} failure={failure:?}"
    );
    assert_eq!(
        sent, BLOCKS,
        "bounded persistent scope stopped: {failure:?}, receiver={received:?}"
    );
    assert_eq!(received.unwrap(), BLOCKS);
}

#[test]
fn fatal_cancel_refuses_verified_buffered_bytes_while_clean_half_close_still_drains() {
    for mode in 0..3 {
        let (left, right, facts) = pair();
        let closed_probe = left.try_clone().unwrap();
        let policy = PinnedScopePolicy::new(2).unwrap();
        let left_authority = authority(&policy, &facts, Role::Initiator);
        let right_authority = authority(&policy, &facts, Role::Acceptor);
        let stores = [
            Store(
                std::env::temp_dir()
                    .join(format!("flow-buffer-left-{mode}-{}", std::process::id())),
            ),
            Store(
                std::env::temp_dir()
                    .join(format!("flow-buffer-right-{mode}-{}", std::process::id())),
            ),
        ];
        let mut left =
            MappedStream::from_duplex(duplex(left, &left_authority, Role::Initiator, &stores[0]));
        let mut right =
            MappedStream::from_duplex(duplex(right, &right_authority, Role::Acceptor, &stores[1]));
        let payload = b"verified-before-fatal-cancel";
        right.write_all(payload).unwrap();
        let mut first = [0; 1];
        left.read_exact(&mut first).unwrap();
        assert_eq!(first, [payload[0]]);
        left_authority.check().unwrap();
        right_authority.check().unwrap();
        if mode != 1 {
            right.shutdown(Shutdown::Write).unwrap();
            left.shutdown(Shutdown::Write).unwrap();
            assert_eq!(right.read(&mut [0; 1]).unwrap(), 0);
            // Observe actual shared TLS shutdown, rather than guessing when
            // the clean worker completed. Cloning never creates another FD.
            let until = Instant::now() + Duration::from_secs(1);
            while closed_probe.try_clone().is_ok() {
                assert!(
                    Instant::now() < until,
                    "clean duplex must close its physical carrier"
                );
                std::thread::yield_now();
            }
        }
        if mode == 0 {
            assert!(left.write(b"pre-IO refusal after local FIN").is_err());
            let mut rest = Vec::new();
            left.read_to_end(&mut rest).unwrap();
            assert_eq!(rest, payload[1..]);
            assert_eq!(left.read(&mut [0; 1]).unwrap(), 0);
        } else {
            // mode2 cancels only AFTER actual graceful I/O closure; the
            // fatal delivery fence must still become effective monotonically.
            left.shutdown(Shutdown::Both).unwrap();
            left_authority.check().unwrap();
            let mut output = [0; 5];
            assert_eq!(
                left.read(&mut output)
                    .expect_err(
                        "fatal cancellation must not deliver an already verified buffered suffix"
                    )
                    .kind(),
                std::io::ErrorKind::BrokenPipe
            );
            assert_eq!(output, [0; 5]);
            let mut clone = left.clone();
            assert_eq!(
                clone.read(&mut output).unwrap_err().kind(),
                std::io::ErrorKind::BrokenPipe
            );
        }
        left_authority.check().unwrap();
    }
    // Joining a positively completed worker is observation, not cancellation.
    // Its already verified application queue remains readable under live scope.
    let (left, right, facts) = pair();
    let policy = PinnedScopePolicy::new(2).unwrap();
    let left_authority = authority(&policy, &facts, Role::Initiator);
    let right_authority = authority(&policy, &facts, Role::Acceptor);
    let stores = [
        Store(std::env::temp_dir().join(format!("flow-joined-left-{}", std::process::id()))),
        Store(std::env::temp_dir().join(format!("flow-joined-right-{}", std::process::id()))),
    ];
    let (mut left_tx, mut left_rx, left_guard) =
        duplex(left, &left_authority, Role::Initiator, &stores[0]).split();
    let (mut right_tx, mut right_rx, right_guard) =
        duplex(right, &right_authority, Role::Acceptor, &stores[1]).split();
    let payload = b"queued-before-successful-join".to_vec();
    right_tx.send_block(payload.clone()).unwrap();
    left_tx.half_close().unwrap();
    right_tx.half_close().unwrap();
    let left_counts = left_guard.join().unwrap();
    let right_counts = right_guard.join().unwrap();
    assert_eq!(left_counts.0.wire_bytes, right_counts.1.wire_bytes);
    assert_eq!(left_counts.1.wire_bytes, right_counts.0.wire_bytes);
    left_authority.check().unwrap();
    assert_eq!(
        left_rx.receive().unwrap(),
        spiralcore_mvm_flowmux::Event::Data(payload)
    );
    assert_eq!(
        left_rx.receive().unwrap(),
        spiralcore_mvm_flowmux::Event::HalfClose
    );
    assert_eq!(
        right_rx.receive().unwrap(),
        spiralcore_mvm_flowmux::Event::HalfClose
    );
}

fn assert_fatal_transport(error: spiralcore_mvm_flowmux::Error) {
    use spiralcore_mvm_flowmux::Error;
    match error {
        Error::Truncated | Error::Closed | Error::Cancelled => {}
        Error::Io(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::NotConnected
            ) => {}
        other => panic!("expected fatal physical truncation, observed {other:?}"),
    }
}

#[test]
fn tls_close_without_authenticated_remote_application_fin_remains_fatal() {
    let (left, right, facts) = pair();
    let abrupt = right.try_clone().unwrap();
    let policy = PinnedScopePolicy::new(2).unwrap();
    let left_authority = authority(&policy, &facts, Role::Initiator);
    let right_authority = authority(&policy, &facts, Role::Acceptor);
    let stores = [
        Store(std::env::temp_dir().join(format!("flow-missing-fin-left-{}", std::process::id()))),
        Store(std::env::temp_dir().join(format!("flow-missing-fin-right-{}", std::process::id()))),
    ];
    let (mut left_tx, mut left_rx, left_guard) =
        duplex(left, &left_authority, Role::Initiator, &stores[0]).split();
    let (_right_tx, mut right_rx, right_guard) =
        duplex(right, &right_authority, Role::Acceptor, &stores[1]).split();
    left_tx.half_close().unwrap();
    assert_eq!(
        right_rx.receive().unwrap(),
        spiralcore_mvm_flowmux::Event::HalfClose
    );
    // The local FIN is real; the remote application FIN was never sent.
    abrupt.shutdown(Shutdown::Both).unwrap();
    assert_fatal_transport(left_guard.join().unwrap_err());
    assert!(matches!(
        left_rx.receive(),
        Err(spiralcore_mvm_flowmux::Error::Cancelled)
    ));
    assert_fatal_transport(right_guard.join().unwrap_err());
    left_authority.check().unwrap();
}

#[test]
fn tls_close_with_canonical_publication_reply_pending_remains_fatal() {
    let (left, right, facts) = pair();
    let mut abrupt = right.try_clone().unwrap();
    abrupt
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let policy = PinnedScopePolicy::new(2).unwrap();
    let left_authority = authority(&policy, &facts, Role::Initiator);
    let right_authority = authority(&policy, &facts, Role::Acceptor);
    let stores = [
        Store(std::env::temp_dir().join(format!("flow-pending-left-{}", std::process::id()))),
        Store(std::env::temp_dir().join(format!("flow-pending-right-{}", std::process::id()))),
    ];
    let (mut left_tx, mut left_rx, left_guard) =
        duplex(left, &left_authority, Role::Initiator, &stores[0]).split();
    let mut channel = ObjectChannel::from_authenticated(
        right,
        &right_authority,
        Duration::from_secs(2),
        Role::Acceptor,
        caps(),
        caps(),
    )
    .unwrap();
    let mut endpoint = ObjectEndpoint::open(
        &stores[1].0,
        Limits::default(),
        right_authority.clone(),
        channel.profile(),
    )
    .unwrap();
    std::thread::scope(|scope| {
        let sending = scope.spawn(move || left_tx.send_block(vec![37; 1024]));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let spiralcore_mvm_flowmux::ServeEvent::Handled { request, reply, .. } = channel
            .serve_once(&mut endpoint, right_authority.context(), now)
            .unwrap()
        else {
            panic!("canonical offer expected")
        };
        assert_eq!((request.kind, reply.kind), (1, 101));
        // Observe the actual next canonical request without consuming it or
        // sending Published. Local application FIN is deliberately absent.
        let mut byte = [0u8; 1];
        assert_eq!(abrupt.peek(&mut byte).unwrap(), 1);
        abrupt.shutdown(Shutdown::Both).unwrap();
        assert!(
            sending.join().unwrap().is_err(),
            "withheld Published became successful transfer"
        );
    });
    assert_fatal_transport(left_guard.join().unwrap_err());
    assert!(matches!(
        left_rx.receive(),
        Err(spiralcore_mvm_flowmux::Error::Cancelled)
    ));
    left_authority.check().unwrap();
}
