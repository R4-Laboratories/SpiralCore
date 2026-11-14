#[cfg(feature = "witness-stall")]
#[path = "ordered_stream/stall.rs"]
mod stall;
use cpu_time::ProcessTime;
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
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
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
// Only fixed scalar counters are retained. The product's SmallV1 charge is unchanged.
// Timing totals overlap across nested operations and the two directions.
struct WitnessReport<'a> {
    began: Instant,
    process_start: Option<ProcessTime>,
    setup_wall: Duration,
    setup_cpu: Option<Duration>,
    tls: [&'a TlsCarrier; 2],
    progress: Arc<[AtomicU64; 4]>,
    #[cfg(feature = "witness-timing")]
    retention: [&'a spiralcore_mvm_flowmux::RetentionObserver; 2],
    #[cfg(feature = "witness-timing")]
    timing: [spiralcore_mvm_flowmux::timing::TimingObserver; 4],
    success: bool,
}
impl Drop for WitnessReport<'_> {
    fn drop(&mut self) {
        let cpu = self
            .process_start
            .and_then(|start| start.try_elapsed().ok());
        let progress = self.progress.each_ref().map(|v| v.load(Ordering::Relaxed));
        let traffic = self.tls.map(|tls| tls.counters());
        // Ignore a closed diagnostic output stream; never mask the original failure.
        let _ = writeln!(
            std::io::stderr(),
            "ORDERED_WITNESS success={} witness_timing={} snapshot=concurrent wall_us={} original_gate_us=240000000 setup_wall_us={} setup_process_cpu_us={:?} process_cpu_to_snapshot_us={:?} progress_write_ack_left_right_read_left_right={progress:?} actual_tls={traffic:?} scopes=unavailable_nonblocking scope_status_at_existing_failure_assertion=see_original_error rss_kib={:?} hwm_kib={:?}",
            self.success,
            cfg!(feature = "witness-timing"),
            self.began.elapsed().as_micros(),
            self.setup_wall.as_micros(),
            self.setup_cpu.map(|v| v.as_micros()),
            cpu.map(|v| v.as_micros()),
            memory("VmRSS:"),
            memory("VmHWM:")
        );
        #[cfg(feature = "witness-timing")]
        {
            use spiralcore_mvm_flowmux::timing::{Snapshot, TimingObserver};
            // Four owners (TLS pair and duplex pair). Sixteen observer handles
            // include four new wait-wrapper handles. Those four are already in
            // the synchronization metadata below; this inventory is nonadditive.
            // Allocator bookkeeping is not measurable through std and is excluded.
            let owner_bytes = 4 * TimingObserver::owned_bytes();
            let handles_bytes = 16 * std::mem::size_of::<TimingObserver>();
            let handles_overlap = 4 * std::mem::size_of::<TimingObserver>();
            let synchronization_bytes =
                2 * TlsCarrier::ownership_wait_bytes() + 2 * Duplex::ownership_wait_bytes();
            let snapshot_bytes = std::mem::size_of::<Snapshot>();
            let _ = writeln!(
                std::io::stderr(),
                "ORDERED_TIMING_STORAGE outside_small_v1=true fixed_owner_heap_requested_bytes={owner_bytes} observer_handles_bytes={handles_bytes} ownership_wait_additional_requested_bytes={synchronization_bytes} observer_handle_overlap_with_wait_metadata_bytes={handles_overlap} component_inventory_nonadditive=true one_snapshot_value_bytes={snapshot_bytes} allocator_bookkeeping=excluded fixed_progress_bytes={} retention_nonblocking={:?}",
                std::mem::size_of::<[AtomicU64; 4]>() + 2 * std::mem::size_of::<usize>(),
                self.retention.map(|r| r.try_snapshot())
            );
            let _ = writeln!(
                std::io::stderr(),
                "ORDERED_COMMIT_SUBSTAGES scope=process_aggregate_both_endpoints outside_small_v1=true lifetime=process fixed_static_bytes={} overlapping_not_additive=true wall_and_cpu_maxima_independent=true {:?}",
                spiralcore_transfer::endpoint::commit_timing::fixed_static_bytes(),
                spiralcore_transfer::endpoint::commit_timing::snapshot()
            );
            // Format one snapshot at a time; no event collection or variable buffer.
            for (slot, observer) in self.timing.iter().enumerate() {
                let _ = writeln!(
                    std::io::stderr(),
                    "ORDERED_TIMING slot={slot} order=tls_left,tls_right,duplex_left,duplex_right snapshot=concurrent overlapping_elapsed_not_cpu {:?}",
                    observer.snapshot()
                );
            }
        }
    }
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
    let process_start = ProcessTime::try_now().ok();
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
    #[cfg(feature = "witness-timing")]
    let tls_timing = [left.timing_observer(), right.timing_observer()];
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
    let enrollment_until = began + Duration::from_secs(270);
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
    #[cfg(feature = "witness-timing")]
    eprintln!("ORDERED_STORAGE_ROOT temp={}", std::env::temp_dir().display());
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
        Duplex::spawn(channel, sender, endpoint, authority.context().clone(), role).unwrap()
    };
    let left = engine(left, &left_auth, Role::Initiator, &stores[0].0);
    let right = engine(right, &right_auth, Role::Acceptor, &stores[1].0);
    let left_state = left.retention_observer();
    let right_state = right.retention_observer();
    #[cfg(feature = "witness-timing")]
    let duplex_timing = [left.timing_observer(), right.timing_observer()];
    #[cfg(feature = "witness-stall")]
    let stall_observers = [left.stall_observer(), right.stall_observer()];
    let left = MappedStream::from_duplex(left);
    let right = MappedStream::from_duplex(right);
    let setup_wall = began.elapsed();
    let cpu_setup = cpu();
    let rss_prepared = memory("VmRSS:");
    let progress = Arc::new(std::array::from_fn(|_| AtomicU64::new(0)));
    let mut report = WitnessReport {
        began,
        process_start,
        setup_wall,
        setup_cpu: process_start.and_then(|start| start.try_elapsed().ok()),
        tls: [&left_counters, &right_counters],
        progress: progress.clone(),
        #[cfg(feature = "witness-timing")]
        retention: [&left_state, &right_state],
        #[cfg(feature = "witness-timing")]
        timing: [
            tls_timing[0].clone(),
            tls_timing[1].clone(),
            duplex_timing[0].clone(),
            duplex_timing[1].clone(),
        ],
        success: false,
    };
    // Drop temporary observer arrays; the report retains the four fixed clones.
    #[cfg(feature = "witness-timing")]
    {
        drop(tls_timing);
        drop(duplex_timing);
    }
    let run_until = began + Duration::from_secs(240);
    std::thread::scope(|scope| {
        #[cfg(feature = "witness-stall")]
        let _sampler_guard = {
            let (complete, stop_sampler) = std::sync::mpsc::sync_channel(0);
            scope.spawn(|| {
                stall::watch(
                    stop_sampler,
                    stall_observers,
                    [&left_counters, &right_counters],
                    &progress,
                    began,
                    BLOCKS,
                    "ordered-stream",
                )
            });
            complete
        };
        let writer = |mut stream: MappedStream,
                      direction: usize,
                      observer: spiralcore_mvm_flowmux::RetentionObserver| {
            for index in 0..BLOCKS {
                assert!(
                    Instant::now() < run_until,
                    "original240s fixture deadline at block{index}; retained={:?}; left_scope={:?}; right_scope={:?}",
                    observer.snapshot(),
                    left_auth.check(),
                    right_auth.check()
                );
                stream.write_all(&block(index)).unwrap_or_else(|error| {
                    eprintln!(
                        "ORDERED_FIRST_FAILURE role={direction} operation=write local={:?}",
                        stream.first_failure()
                    );
                    panic!(
                        "block{index}: {error}; retained={:?}; left_scope={:?}; right_scope={:?}",
                        observer.snapshot(),
                        left_auth.check(),
                        right_auth.check()
                    )
                });
                progress[direction].store(index + 1, Ordering::Relaxed);
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
        let reader = |mut stream: MappedStream, direction: usize| {
            for index in 0..BLOCKS {
                let mut actual = [0; 16384];
                stream.read_exact(&mut actual).unwrap_or_else(|error| {
                    panic!("block{index} read: {error}; ORDERED_FIRST_FAILURE role={direction} operation=read local={:?}", stream.first_failure());
                });
                assert_eq!(actual, block(index), "exact block{index}");
                progress[2 + direction].store(index + 1, Ordering::Relaxed);
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
        let lw = scope.spawn(move || writer(left_write, 0, right_observer));
        let rw = scope.spawn(move || writer(right_write, 1, left_observer));
        let lr = scope.spawn(move || reader(left_read, 0));
        let rr = scope.spawn(move || reader(right_read, 1));
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
    report.success = true;
    drop(report);
    drop(left_counters);
    drop(right_counters);
    let paths = [stores[0].0.clone(), stores[1].0.clone()];
    drop(stores);
    assert!(paths.iter().all(|path| !path.exists()));
    let cpu_after = cpu();
    let lifecycle_wall = began.elapsed();
    let lifecycle_cpu = process_start
        .and_then(|start| start.try_elapsed().ok())
        .map(|v| v.as_micros());
    let hwm = memory("VmHWM:");
    eprintln!(
        "ORDERED_BULK blocks_each_direction={BLOCKS} exact_bytes_each_direction={} actual_tls_a_sent={} actual_tls_a_received={} peer_sent={} peer_received={} setup_wall_us={} lifecycle_wall_us={} cpu_process_before={cpu_before:?} cpu_process_setup={cpu_setup:?} cpu_process_after={cpu_after:?} lifecycle_process_cpu_us={lifecycle_cpu:?} cpu_ticks_per_second={tick_rate:?} rss_before_kib={rss_before:?} rss_prepared_kib={rss_prepared:?} process_hwm_kib={hwm:?} core_pair_charge_each={} reader_stack_each={} left={ls:?} right={rs:?} files_left={lf:?} files_right={rf:?}",
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
