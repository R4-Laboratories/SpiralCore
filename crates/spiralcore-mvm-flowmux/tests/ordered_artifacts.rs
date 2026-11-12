use spiralcore_mvm_flowmux::{
    Duplex, MappedStream, ObjectChannel, Role,
    scope::{PinnedScopePolicy, ScopeFacts},
    tls::{PinnedIdentity, TlsCarrier},
};
use spiralcore_transfer::{
    resource_profile::ResourceProfile,
    session::SelectionPolicy,
    shared_runtime::{OutputKind, Recipe, State},
    shared_transfer::PreparedPrediction,
    v1::{Capabilities, ObjectEndpoint, ObjectMode, ObjectSender},
};
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    time::{Duration, Instant},
};
fn identity() -> PinnedIdentity {
    let key =
        ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    PinnedIdentity::from_pkcs8(key.as_ref().to_vec()).unwrap()
}
struct Store(std::path::PathBuf);
impl Drop for Store {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn ordered_small_warm_runtime_and_combined_keep_verified_setup_pinned_through_eviction() {
    for mode in [ObjectMode::Warm, ObjectMode::Runtime, ObjectMode::Combined] {
        let client_key = identity();
        let server_key = identity();
        let cp = client_key.spki_sha256();
        let sp = server_key.spki_sha256();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let client = TcpStream::connect(addr).unwrap();
        let (server, _) = listener.accept().unwrap();
        let (left, right) = std::thread::scope(|scope| {
            let peer = scope.spawn(move || {
                TlsCarrier::accept(
                    server,
                    &server_key,
                    cp,
                    Instant::now() + Duration::from_secs(2),
                )
                .unwrap()
            });
            let client = TlsCarrier::connect(
                client,
                &client_key,
                sp,
                "peer.example",
                Instant::now() + Duration::from_secs(2),
            )
            .unwrap();
            (client, peer.join().unwrap())
        });
        let facts = ScopeFacts {
            object_profile: 2,
            policy_sha256: [1; 32],
            plan_id: "ordered-math-plan".into(),
            plan_version: 1,
            plan_nonce: [2; 16],
            bundle_sha256: [3; 32],
            tenant: "tenant".into(),
            instance: format!("ordered-{mode:?}"),
            session: "one-fixed-session".into(),
            guest_key: [4; 32],
            stream_id: 1,
            requested_host: "peer.example".into(),
            requested_port: 443,
            actual_peer: addr,
            route: "peer".into(),
            initiator_spki: cp,
            acceptor_spki: sp,
            tls_exporter: left.exporter(),
        };
        let policy = PinnedScopePolicy::new(2).unwrap();
        let until = Instant::now() + Duration::from_secs(20);
        policy.enroll(&facts, sp, Role::Initiator, until).unwrap();
        policy.enroll(&facts, cp, Role::Acceptor, until).unwrap();
        let la = policy.authorize_scope(&facts, sp, Role::Initiator).unwrap();
        let ra = policy.authorize_scope(&facts, cp, Role::Acceptor).unwrap();
        let caps = Capabilities::small_v1();
        let profile = caps.negotiate(&caps).unwrap();
        let checkpoint = State::seeded(32).unwrap().encode().unwrap();
        let schema = spiralcore_transfer::schema::fit(&[b"{\"value\":0}\n"]).unwrap();
        let prediction = PreparedPrediction::new_with_resource_profile(
            &checkpoint,
            &Recipe {
                steps: 16,
                payload: 1.25,
                output: OutputKind::Trace,
            },
            ResourceProfile::SmallV1,
        )
        .unwrap();
        let original = prediction.prediction().to_vec();
        assert!((8..=16384).contains(&original.len()));
        let sender = match mode {
            ObjectMode::Warm => ObjectSender::new_with_resource_profile(
                mode,
                la.context(),
                Some(&schema),
                profile,
                ResourceProfile::SmallV1,
            ),
            ObjectMode::Runtime => ObjectSender::with_runtime_resource_profile(
                la.context(),
                prediction,
                profile,
                ResourceProfile::SmallV1,
            ),
            ObjectMode::Combined => ObjectSender::with_combined_resource_profile(
                la.context(),
                prediction,
                &schema,
                SelectionPolicy::Exhaustive,
                profile,
                ResourceProfile::SmallV1,
            ),
            _ => unreachable!(),
        }
        .unwrap();
        let stores = [
            Store(
                std::env::temp_dir()
                    .join(format!("ordered-math-left-{mode:?}-{}", std::process::id())),
            ),
            Store(std::env::temp_dir().join(format!(
                "ordered-math-right-{mode:?}-{}",
                std::process::id()
            ))),
        ];
        let left_endpoint = ObjectEndpoint::open_with_resource_profile(
            &stores[0].0,
            ResourceProfile::SmallV1.receiver_limits(),
            la.clone(),
            profile,
            ResourceProfile::SmallV1,
        )
        .unwrap();
        let right_endpoint = ObjectEndpoint::open_with_resource_profile(
            &stores[1].0,
            ResourceProfile::SmallV1.receiver_limits(),
            ra.clone(),
            profile,
            ResourceProfile::SmallV1,
        )
        .unwrap();
        let right_sender = ObjectSender::new_with_resource_profile(
            ObjectMode::Whole,
            ra.context(),
            None,
            profile,
            ResourceProfile::SmallV1,
        )
        .unwrap();
        let left_channel = ObjectChannel::from_authenticated(
            left,
            &la,
            Duration::from_secs(2),
            Role::Initiator,
            caps,
            caps,
        )
        .unwrap();
        let right_channel = ObjectChannel::from_authenticated(
            right,
            &ra,
            Duration::from_secs(2),
            Role::Acceptor,
            caps,
            caps,
        )
        .unwrap();
        let left = Duplex::spawn(
            left_channel,
            sender,
            left_endpoint,
            la.context().clone(),
            Role::Initiator,
        )
        .unwrap();
        let right = Duplex::spawn(
            right_channel,
            right_sender,
            right_endpoint,
            ra.context().clone(),
            Role::Acceptor,
        )
        .unwrap();
        let observed = right.retention_observer();
        let mut left = MappedStream::from_duplex(left);
        let mut right = MappedStream::from_duplex(right);
        let setup_until = Instant::now() + Duration::from_secs(2);
        if matches!(mode, ObjectMode::Runtime | ObjectMode::Combined) {
            left.seed_checkpoint_until(setup_until).unwrap();
        }
        if matches!(mode, ObjectMode::Warm | ObjectMode::Combined) {
            left.seed_schema_until(setup_until).unwrap();
        }
        let expected = original.clone();
        let size = original.len();
        std::thread::scope(|scope| {
            let peer = scope.spawn(move || {
                for index in 0..129u64 {
                    let mut actual = vec![0; size];
                    right.read_exact(&mut actual).unwrap();
                    let mut target = expected.clone();
                    target[..8].copy_from_slice(&index.to_be_bytes());
                    assert_eq!(actual, target, "artifact bytes never become application");
                    right.write_all(&actual).unwrap();
                }
                let mut byte = [0];
                assert_eq!(right.read(&mut byte).unwrap(), 0);
                right.shutdown(Shutdown::Write).unwrap();
            });
            for index in 0..129u64 {
                let mut target = original.clone();
                target[..8].copy_from_slice(&index.to_be_bytes());
                left.write_all(&target).unwrap();
                let mut echoed = vec![0; size];
                left.read_exact(&mut echoed).unwrap();
                assert_eq!(echoed, target);
            }
            left.shutdown(Shutdown::Write).unwrap();
            let mut byte = [0];
            assert_eq!(left.read(&mut byte).unwrap(), 0);
            peer.join().unwrap();
        });
        let stats = observed.snapshot().unwrap().unwrap();
        assert_eq!(
            stats.pinned,
            if mode == ObjectMode::Combined { 2 } else { 1 }
        );
        assert!(stats.evictions > 0);
        assert!(stats.publications <= 64);
        assert!(stats.attempts <= 2);
        let namespace = stores[1]
            .0
            .join(spiralcore_transfer::hex(&ra.context().namespace().unwrap()));
        if matches!(mode, ObjectMode::Runtime | ObjectMode::Combined) {
            assert!(
                namespace
                    .join(format!(
                        "{}.object",
                        spiralcore_transfer::hex(&spiralcore_transfer::object_id(&checkpoint))
                    ))
                    .exists()
            );
        }
        if matches!(mode, ObjectMode::Warm | ObjectMode::Combined) {
            assert!(
                namespace
                    .join(format!(
                        "{}.object",
                        spiralcore_transfer::hex(&spiralcore_transfer::object_id(&schema))
                    ))
                    .exists()
            );
        }
        eprintln!(
            "ORDERED_MATH mode={mode:?} exact_blocks_each_direction=129 block_bytes={size} state={stats:?}"
        );
        drop(left);
    }
}
