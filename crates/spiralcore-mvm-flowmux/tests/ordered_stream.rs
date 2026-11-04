use spiralcore_mvm_flowmux::{
    Duplex, MappedStream, ObjectChannel, Role,
    scope::{PinnedScopePolicy, ScopeAuthority, ScopeFacts},
    tls::{PinnedIdentity, TlsCarrier},
};
use spiralcore_transfer::{
    resource_profile::ResourceProfile,
    v1::{Capabilities, ObjectEndpoint, ObjectMode, ObjectSender},
};
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    path::{Path, PathBuf},
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
fn block(index: u64) -> [u8; 16384] {
    let mut bytes = [0x61; 16384];
    bytes[..8].copy_from_slice(&index.to_be_bytes());
    bytes
}
fn cpu() -> Option<(u64, u64)> {
    let text = std::fs::read_to_string("/proc/self/stat").ok()?;
    let (_, fields) = text.rsplit_once(") ")?;
    let fields: Vec<_> = fields.split_whitespace().collect();
    Some((fields.get(11)?.parse().ok()?, fields.get(12)?.parse().ok()?))
}
fn memory(key: &str) -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|tail| tail.split_whitespace().next()?.parse().ok())
        })
}
fn files(root: &Path) -> (usize, u64) {
    let mut count = 0;
    let mut bytes = 0;
    for namespace in std::fs::read_dir(root).unwrap() {
        let namespace = namespace.unwrap();
        if !namespace.file_type().unwrap().is_dir() {
            continue;
        }
        for file in std::fs::read_dir(namespace.path()).unwrap() {
            let file = file.unwrap();
            if file.path().extension().is_some_and(|ext| ext == "object") {
                count += 1;
                bytes += file.metadata().unwrap().len();
            }
        }
    }
    (count, bytes)
}
#[test]
fn four_thousand_distinct_blocks_each_direction_keep_exact_bytes_and_bounded_retention() {
    const BLOCKS: u64 = 4096;
    let tick_rate = std::process::Command::new("getconf")
        .arg("CLK_TCK")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|value| value.trim().parse::<u64>().ok());
    let began = Instant::now();
    let cpu_before = cpu();
    let rss_before = memory("VmRSS:");
    let client_key = identity();
    let server_key = identity();
    let client_pin = client_key.spki_sha256();
    let server_pin = server_key.spki_sha256();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (server, _) = listener.accept().unwrap();
    let (left, right) = std::thread::scope(|scope| {
        let peer = scope.spawn(move || {
            TlsCarrier::accept(
                server,
                &server_key,
                client_pin,
                Instant::now() + Duration::from_secs(2),
            )
            .unwrap()
        });
        let left = TlsCarrier::connect(
            client,
            &client_key,
            server_pin,
            "peer.example",
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        (left, peer.join().unwrap())
    });
    let left_counters = left.try_clone().unwrap();
    let right_counters = right.try_clone().unwrap();
    let facts = ScopeFacts {
        object_profile: 2,
        policy_sha256: [1; 32],
        plan_id: "ordered-bulk-plan".into(),
        plan_version: 1,
        plan_nonce: [2; 16],
        bundle_sha256: [3; 32],
        tenant: "ordered-bulk-tenant".into(),
        instance: "ordered-bulk-instance".into(),
        session: "one-persistent-admitted-session".into(),
        guest_key: [4; 32],
        stream_id: 1,
        requested_host: "peer.example".into(),
        requested_port: 443,
        actual_peer: address,
        route: "peer".into(),
        initiator_spki: client_pin,
        acceptor_spki: server_pin,
        tls_exporter: left.exporter(),
    };
    let policy = PinnedScopePolicy::new(2).unwrap();
    let enrollment_until = Instant::now() + Duration::from_secs(120);
    let authorize = |role: Role| {
        let peer = if role == Role::Initiator {
            server_pin
        } else {
            client_pin
        };
        policy.enroll(&facts, peer, role, enrollment_until).unwrap();
        policy.authorize_scope(&facts, peer, role).unwrap()
    };
    let left_auth = authorize(Role::Initiator);
    let right_auth = authorize(Role::Acceptor);
    let stores = [
        Store(std::env::temp_dir().join(format!("ordered-stream-left-{}", std::process::id()))),
        Store(std::env::temp_dir().join(format!("ordered-stream-right-{}", std::process::id()))),
    ];
    let engine = |tls: TlsCarrier, authority: &ScopeAuthority, role: Role, root: &Path| {
        let caps = Capabilities::small_v1();
        let channel = ObjectChannel::from_authenticated(
            tls,
            authority,
            Duration::from_secs(2),
            role,
            caps,
            caps,
        )
        .unwrap();
        let profile = channel.profile();
        let sender = ObjectSender::new_with_resource_profile(
            ObjectMode::Whole,
            authority.context(),
            None,
            profile,
            ResourceProfile::SmallV1,
        )
        .unwrap();
        let endpoint = ObjectEndpoint::open_with_resource_profile(
            root,
            ResourceProfile::SmallV1.receiver_limits(),
            authority.clone(),
            profile,
            ResourceProfile::SmallV1,
        )
        .unwrap();
        let duplex =
            Duplex::spawn(channel, sender, endpoint, authority.context().clone(), role).unwrap();
        let observer = duplex.retention_observer();
        (MappedStream::from_duplex(duplex), observer)
    };
    let (left, left_state) = engine(left, &left_auth, Role::Initiator, &stores[0].0);
    let (right, right_state) = engine(right, &right_auth, Role::Acceptor, &stores[1].0);
    let setup_wall = began.elapsed();
    let cpu_setup = cpu();
    let rss_prepared = memory("VmRSS:");
    let run_until = Instant::now() + Duration::from_secs(90);
    std::thread::scope(|scope| {
        let writer = |mut stream: MappedStream,
                      observer: spiralcore_mvm_flowmux::RetentionObserver| {
            for index in 0..BLOCKS {
                assert!(Instant::now() < run_until, "original90s fixture deadline");
                stream.write_all(&block(index)).unwrap();
                let state = observer
                    .snapshot()
                    .unwrap()
                    .expect("actual ordered core snapshot");
                assert!(state.attempts <= 2, "{state:?}");
                assert!(state.publications <= 64, "{state:?}");
                assert!(state.cache_bytes <= 8 << 20, "{state:?}");
            }
            stream.shutdown(Shutdown::Write).unwrap();
        };
        let reader = |mut stream: MappedStream| {
            for index in 0..BLOCKS {
                let mut actual = [0; 16384];
                stream.read_exact(&mut actual).unwrap();
                assert_eq!(actual, block(index), "exact block{index}");
            }
            let mut byte = [0];
            assert_eq!(stream.read(&mut byte).unwrap(), 0);
        };
        let left_write = left.clone();
        let right_write = right.clone();
        let left_read = left.clone();
        let right_read = right.clone();
        let left_observer = left_state.clone();
        let right_observer = right_state.clone();
        let lw = scope.spawn(move || writer(left_write, right_observer));
        let rw = scope.spawn(move || writer(right_write, left_observer));
        let lr = scope.spawn(move || reader(left_read));
        let rr = scope.spawn(move || reader(right_read));
        lw.join().unwrap();
        rw.join().unwrap();
        lr.join().unwrap();
        rr.join().unwrap();
    });
    drop(left);
    drop(right);
    let a = left_counters.counters();
    let b = right_counters.counters();
    assert_eq!(a.sent, b.received);
    assert_eq!(a.received, b.sent);
    let ls = left_state.snapshot().unwrap().unwrap();
    let rs = right_state.snapshot().unwrap().unwrap();
    let lf = files(&stores[0].0);
    let rf = files(&stores[1].0);
    assert!(lf.0 <= 64 && rf.0 <= 64);
    assert!(ls.evictions > 0 && rs.evictions > 0);
    assert_eq!(lf.0, ls.publications);
    assert_eq!(rf.0, rs.publications);
    assert_eq!(lf.1, ls.cache_bytes);
    assert_eq!(rf.1, rs.cache_bytes);
    drop(left_counters);
    drop(right_counters);
    let paths = [stores[0].0.clone(), stores[1].0.clone()];
    drop(stores);
    assert!(paths.iter().all(|path| !path.exists()));
    let cpu_after = cpu();
    let lifecycle_wall = began.elapsed();
    let hwm = memory("VmHWM:");
    eprintln!(
        "ORDERED_BULK blocks_each_direction={BLOCKS} exact_bytes_each_direction={} actual_tls_a_sent={} actual_tls_a_received={} peer_sent={} peer_received={} setup_wall_us={} lifecycle_wall_us={} cpu_process_before={cpu_before:?} cpu_process_setup={cpu_setup:?} cpu_process_after={cpu_after:?} cpu_ticks_per_second={tick_rate:?} rss_before_kib={rss_before:?} rss_prepared_kib={rss_prepared:?} process_hwm_kib={hwm:?} core_pair_charge_each={} reader_stack_each={} left={ls:?} right={rs:?} files_left={lf:?} files_right={rf:?}",
        BLOCKS * 16384,
        a.sent,
        a.received,
        b.sent,
        b.received,
        setup_wall.as_micros(),
        lifecycle_wall.as_micros(),
        ResourceProfile::SmallV1.operation_charge().unwrap(),
        spiralcore_mvm_flowmux::READER_STACK_BYTES
    );
}
