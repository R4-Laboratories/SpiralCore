#![cfg(any(target_os = "linux", target_os = "macos"))]
//! Filesystem-only retirement of maximal private cache fixtures, not a transfer or MVM witness.
#[path = "support/requested_allocations.rs"]
pub mod allocations;
#[global_allocator]
static ALLOCATIONS: allocations::RequestedAllocations = allocations::RequestedAllocations;

use spiralcore_transfer::{
    resource_profile::ResourceProfile,
    v1::{Capabilities, ObjectEndpoint, OwnedLaneNamespaces, OwnedStoreLease, OwnedStorePolicy},
    wan,
};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::PathBuf,
    sync::{Arc, Barrier},
    time::Instant,
};

const STORES: usize = 2;
const PUBLICATIONS: usize = 64;
const WORKSPACE: usize = 1 << 20;

struct Fixture {
    root: PathBuf,
    lease: OwnedStoreLease,
    exact_readback_bytes: usize,
}
impl Fixture {
    fn new(worker: u8) -> Self {
        let root =
            std::env::temp_dir().join(format!("sc-verify-work-{}-{worker}", std::process::id()));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        fs::DirBuilder::new()
            .mode(0o700)
            .create(root.join("store"))
            .unwrap();
        let context = wan::context([40 + worker; 32]);
        let other = wan::context([60 + worker; 32]);
        let limits = ResourceProfile::SmallV1.receiver_limits();
        let policy = OwnedStorePolicy::small_v1(
            &limits,
            102,
            OwnedLaneNamespaces::Two(context.namespace().unwrap(), other.namespace().unwrap()),
            spiralcore_transfer::v1::OwnedWorkspaceFunding {
                heap_bytes: 1 << 20,
                existing_stack_bytes: 2 << 20,
            },
        )
        .unwrap();
        let lease = OwnedStoreLease::capture(
            Arc::new(File::open(&root).unwrap()),
            "store".as_ref(),
            Arc::new(File::open(root.join("store")).unwrap()),
            policy,
        )
        .unwrap();
        let caps = Capabilities::small_v1();
        drop(
            ObjectEndpoint::open_at_with_resource_profile(
                &lease,
                limits.clone(),
                wan::BenchAuthority,
                caps.negotiate(&caps).unwrap(),
                ResourceProfile::SmallV1,
            )
            .unwrap(),
        );
        let namespace = root
            .join("store")
            .join(spiralcore_transfer::hex(&context.namespace().unwrap()));
        let second = root
            .join("store")
            .join(spiralcore_transfer::hex(&other.namespace().unwrap()));
        for path in [&namespace, &second] {
            fs::DirBuilder::new().mode(0o700).create(path).unwrap();
        }
        let mut exact_readback_bytes = 0;
        for index in 0..PUBLICATIONS {
            let mut body = vec![0x51; ResourceProfile::SmallV1.max_artifact_len()];
            body[..8].copy_from_slice(&(index as u64).to_le_bytes());
            let end = body.len();
            body[end - 3..].fill(0);
            let id = spiralcore_transfer::object_id(&body);
            let path = namespace.join(format!("{}.object", spiralcore_transfer::hex(&id)));
            let mut record = Vec::new();
            record.extend_from_slice(b"SCPUB001");
            record.extend_from_slice(&(body.len() as u64).to_le_bytes());
            record.extend_from_slice(&id);
            record.push(0);
            record.extend_from_slice(&body);
            write(&path, &record);
            let actual = fs::read(&path).unwrap();
            assert_eq!(actual, record);
            exact_readback_bytes += actual.len();
        }
        for index in 0..limits.max_states {
            write(
                &namespace.join(format!(".stage-{}-{index}", "7".repeat(240))),
                b"retained stage\0",
            );
        }
        write(
            &root.join("store/.profile-123-456"),
            b"retained profile stage\0",
        );
        let entries = fs::read_dir(root.join("store"))
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                1 + if e.file_type().unwrap().is_dir() {
                    fs::read_dir(e.path()).unwrap().count()
                } else {
                    0
                }
            })
            .sum::<usize>();
        assert_eq!(entries, 101);
        Self {
            root,
            lease,
            exact_readback_bytes,
        }
    }
}
fn write(path: &std::path::Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn maximal_store_retirement_does_not_allocate_a_publication_body() {
    const CHILD: &str = "SC_VERIFICATION_WORKSPACE_CHILD";
    const NAME: &str = "maximal_store_retirement_does_not_allocate_a_publication_body";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--show-output", "--test-threads=1"])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        let rows: Vec<_> = stdout
            .lines()
            .filter(|l| l.starts_with("OWNED_VERIFICATION "))
            .collect();
        assert_eq!(rows.len(), 1);
        println!("{}", rows[0]);
        return;
    }
    let setup = Instant::now();
    let setup_cpu = cpu_time::ProcessTime::now();
    let fixtures: Vec<_> = (0..STORES).map(|n| Fixture::new(n as u8)).collect();
    let readback_bytes: usize = fixtures.iter().map(|f| f.exact_readback_bytes).sum();
    let setup_wall_ns = setup.elapsed().as_nanos();
    let setup_cpu_ns = setup_cpu.elapsed().as_nanos();
    let ready = Arc::new(Barrier::new(STORES + 1));
    let start = Arc::new(Barrier::new(STORES + 1));
    let ((costs, window_wall_ns, window_process_cpu_ns), largest, requested) =
        std::thread::scope(|scope| {
            let workers: Vec<_> = fixtures
                .into_iter()
                .map(|mut f| {
                    let ready = ready.clone();
                    let start = start.clone();
                    scope.spawn(move || {
                        ready.wait();
                        start.wait();
                        let clock = Instant::now();
                        let cpu = cpu_time::ThreadTime::now();
                        let owner = File::open(f.root.join("store/.owner")).unwrap();
                        let result = f.lease.retire_contents();
                        let costs = (clock.elapsed().as_nanos(), cpu.elapsed().as_nanos());
                        result.unwrap();
                        assert_eq!(fs::read_dir(f.root.join("store")).unwrap().count(), 0);
                        assert!(matches!(
                            owner.try_lock(),
                            Err(fs::TryLockError::WouldBlock)
                        ));
                        // The original lease still owns the writer lock until this worker's destruction.
                        costs
                    })
                })
                .collect();
            ready.wait();
            allocations::measure_requests(|| {
                let clock = Instant::now();
                let cpu = cpu_time::ProcessTime::now();
                start.wait();
                let costs = workers
                    .into_iter()
                    .map(|w| w.join().unwrap())
                    .collect::<Vec<_>>();
                (costs, clock.elapsed().as_nanos(), cpu.elapsed().as_nanos())
            })
        });
    assert!(allocations::valid());
    println!(
        "OWNED_VERIFICATION {}",
        serde_json::json!({"stores":STORES,"preloaded_objects_each":PUBLICATIONS,"body_bytes_each":ResourceProfile::SmallV1.max_artifact_len(),"entries_each":101,"setup_readback_bytes":readback_bytes,"setup_wall_ns":setup_wall_ns,"setup_process_cpu_ns":setup_cpu_ns,"retirement_wall_ns_each":costs.iter().map(|c|c.0).collect::<Vec<_>>(),"retirement_thread_cpu_ns_each":costs.iter().map(|c|c.1).collect::<Vec<_>>(),"window_wall_ns":window_wall_ns,"window_process_cpu_ns":window_process_cpu_ns,"largest_new_requested_allocation":largest,"cumulative_new_requested_bytes":requested,"declared_workspace_each":WORKSPACE,"allocation_scope":"all process requests while window open; retirement coordination/assertions/fixture destruction included; retained directory buffers predate window","retirement_timing_scope":"each worker opening the original owner probe through return from retire_contents; excludes later assertions/destruction","window_timing_scope":"before worker release through all joins; includes coordination/assertions/destruction","scope":"filesystem-only private cache fixtures; requested Rust payload only, not allocator overhead/direct libc/stacks/RSS; no network/MVM/WAN"})
    );
    assert!(
        largest <= 8192,
        "filesystem verification allocated a publication body: {largest}"
    );
    assert!(
        requested <= STORES * WORKSPACE,
        "retirement requests exceed declared aggregate workspace"
    );
}
