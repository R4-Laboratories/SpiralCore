#![cfg(any(target_os = "linux", target_os = "macos"))]

//! Populated-store qualification, not a native MVM or allocator-resident bound.
#[path = "support/requested_allocations.rs"]
pub mod allocations;
#[global_allocator]
static ALLOCATIONS: allocations::RequestedAllocations = allocations::RequestedAllocations;

use sha2::{Digest, Sha256};
use spiralcore_transfer::{
    client,
    endpoint::Context,
    protocol::Mode,
    resource_profile::ResourceProfile,
    v1::{
        Capabilities, ObjectEndpoint, OrderedBinding, OrderedLane, OrderedObjectEndpoint,
        OwnedLaneNamespaces, OwnedStoreLease, OwnedStorePolicy,
    },
    wan, Error,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::PathBuf,
    sync::{mpsc, Arc},
    time::Instant,
};

const STORES: usize = 2;
// Existing installed MVM611a3c76 workspace quote; no permit or authority is created here.
const WORKSPACE: u64 = 1 << 20;
const BODY: usize = 16 * 1024;
const APP: &[u8] = b"spiralcore.allocation.application.v1\0";
const ART: &[u8] = b"spiralcore.allocation.artifact.v1\0";

fn executable_sha256() -> String {
    let mut file = File::open(std::env::current_exe().unwrap()).unwrap();
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 32768];
    loop {
        let count = file.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    spiralcore_transfer::hex(&hash.finalize().into())
}

struct Store {
    root: PathBuf,
    context: Context,
    other: Context,
}
impl Store {
    fn new(worker: u8, phase: u8) -> Self {
        let root = std::env::temp_dir().join(format!(
            "sc-owned-allocation-{}-{worker}-{phase}",
            std::process::id()
        ));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        fs::DirBuilder::new()
            .mode(0o700)
            .create(root.join("store"))
            .unwrap();
        Self {
            root,
            context: wan::context([40 + worker; 32]),
            other: wan::context([60 + worker; 32]),
        }
    }
    fn capture(&self) -> OwnedStoreLease {
        let limits = ResourceProfile::SmallV1.receiver_limits();
        OwnedStoreLease::capture(
            Arc::new(File::open(&self.root).unwrap()),
            "store".as_ref(),
            Arc::new(File::open(self.root.join("store")).unwrap()),
            OwnedStorePolicy::small_v1(
                &limits,
                102,
                OwnedLaneNamespaces::Two(
                    self.context.namespace().unwrap(),
                    self.other.namespace().unwrap(),
                ),
                spiralcore_transfer::v1::OwnedWorkspaceFunding {
                    heap_bytes: 1 << 20,
                    existing_stack_bytes: 2 << 20,
                },
            )
            .unwrap(),
        )
        .unwrap()
    }
    fn open(&self, lease: &OwnedStoreLease) -> ObjectEndpoint<wan::BenchAuthority> {
        let caps = Capabilities::small_v1();
        ObjectEndpoint::open_at_with_resource_profile(
            lease,
            ResourceProfile::SmallV1.receiver_limits(),
            wan::BenchAuthority,
            caps.negotiate(&caps).unwrap(),
            ResourceProfile::SmallV1,
        )
        .unwrap()
    }
    fn ordered(
        &self,
        endpoint: ObjectEndpoint<wan::BenchAuthority>,
    ) -> (OrderedBinding, OrderedObjectEndpoint<wan::BenchAuthority>) {
        let binding = OrderedBinding::new(
            &self.context,
            self.context.namespace().unwrap(),
            0,
            APP,
            ART,
        )
        .unwrap();
        let endpoint =
            OrderedObjectEndpoint::new(endpoint, binding.clone(), &self.context).unwrap();
        (binding, endpoint)
    }
    fn namespace(&self) -> PathBuf {
        self.root
            .join("store")
            .join(spiralcore_transfer::hex(&self.context.namespace().unwrap()))
    }
    fn stage_full_tree(&self) -> Vec<PathBuf> {
        let limits = ResourceProfile::SmallV1.receiver_limits();
        assert_eq!((limits.max_publications, limits.max_states), (64, 32));
        let other = self
            .root
            .join("store")
            .join(spiralcore_transfer::hex(&self.other.namespace().unwrap()));
        fs::DirBuilder::new().mode(0o700).create(other).unwrap();
        let mut paths = Vec::new();
        for index in 0..limits.max_states {
            // Valid long stage names exercise repeated iterator refills without growing a list.
            paths.push(
                self.namespace()
                    .join(format!(".stage-{}-{index}", "7".repeat(240))),
            );
        }
        paths.push(self.root.join("store/.profile-123-456"));
        for path in &paths {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)
                .unwrap();
            file.write_all(b"retained recovery stage\0").unwrap();
        }
        assert_eq!(self.entries(), 101);
        paths
    }
    fn entries(&self) -> usize {
        fs::read_dir(self.root.join("store"))
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                1 + if entry.file_type().unwrap().is_dir() {
                    fs::read_dir(entry.path()).unwrap().count()
                } else {
                    0
                }
            })
            .sum()
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn body(index: usize) -> Vec<u8> {
    let mut bytes = vec![0x51; BODY];
    bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
    bytes[BODY - 3..].fill(0);
    bytes
}
#[derive(Default)]
struct Costs {
    application: u64,
    setup_cpu_ns: u128,
    setup_wall_ns: u128,
    request: u64,
    reply: u64,
    sender_cpu_ns: u128,
    receiver_cpu_ns: u128,
    readback_cpu_ns: u128,
    recovery_cpu_ns: u128,
    recovery_wall_ns: u128,
    retirement_cpu_ns: u128,
    retirement_wall_ns: u128,
    refusal_cpu_ns: u128,
    refusal_wall_ns: u128,
}
fn populate(store: &Store, costs: &mut Costs) -> OwnedStoreLease {
    let setup_cpu = cpu_time::ThreadTime::now();
    let setup_clock = Instant::now();
    let lease = store.capture();
    let (binding, mut endpoint) = store.ordered(store.open(&lease));
    costs.setup_cpu_ns += setup_cpu.elapsed().as_nanos();
    costs.setup_wall_ns += setup_clock.elapsed().as_nanos();
    for index in 0..64 {
        let bytes = body(index);
        let cpu = cpu_time::ThreadTime::now();
        let mut receive = 0;
        let stats = client::transfer_batched(
            &bytes,
            Mode::Raw,
            None,
            binding.transfer_id(OrderedLane::Application, index as u64),
            false,
            |frames| {
                let cpu = cpu_time::ThreadTime::now();
                let replies = frames
                    .iter()
                    .map(|frame| {
                        endpoint.handle(
                            OrderedLane::Application,
                            index as u64,
                            &store.context,
                            frame,
                            0,
                        )
                    })
                    .collect();
                receive += cpu.elapsed().as_nanos();
                replies
            },
        )
        .unwrap();
        costs.sender_cpu_ns += cpu.elapsed().as_nanos().checked_sub(receive).unwrap();
        costs.receiver_cpu_ns += receive;
        let cpu = cpu_time::ThreadTime::now();
        assert_eq!(
            endpoint.read_object(&store.context, stats.object).unwrap(),
            bytes
        );
        costs.readback_cpu_ns += cpu.elapsed().as_nanos();
        costs.application += bytes.len() as u64;
        costs.request += stats.request_bytes;
        costs.reply += stats.response_bytes;
    }
    assert_eq!(endpoint.snapshot(&store.context).unwrap().publications, 64);
    drop(endpoint);
    lease
}

struct Gate {
    notify: mpsc::SyncSender<u8>,
    wait: mpsc::Receiver<u8>,
}
impl Gate {
    fn meet(&self, phase: u8) {
        self.notify.try_send(phase).unwrap();
        assert_eq!(
            self.wait
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap(),
            phase
        );
    }
}

fn run_store(worker: u8, gate: Gate) -> Costs {
    let mut costs = Costs::default();
    {
        let store = Store::new(worker, 0);
        let lease = populate(&store, &mut costs);
        let stages = store.stage_full_tree();
        // Stop the previous core owner; recovery captures the same private directory anew.
        drop(lease);
        gate.meet(0);
        let recovery_cpu = cpu_time::ThreadTime::now();
        let recovery_clock = Instant::now();
        let mut lease = store.capture();
        let endpoint = store.open(&lease);
        assert!(stages.iter().all(|path| !path.exists()));
        assert_eq!(store.entries(), 68);
        let (_, mut endpoint) = store.ordered(endpoint);
        costs.recovery_cpu_ns += recovery_cpu.elapsed().as_nanos();
        costs.recovery_wall_ns += recovery_clock.elapsed().as_nanos();
        assert_eq!(endpoint.snapshot(&store.context).unwrap().publications, 64);
        for index in 0..64 {
            let bytes = body(index);
            assert_eq!(
                endpoint
                    .read_object(&store.context, spiralcore_transfer::object_id(&bytes))
                    .unwrap(),
                bytes
            );
        }
        drop(endpoint);
        let owner = File::open(store.root.join("store/.owner")).unwrap();
        gate.meet(1);
        let retire_cpu = cpu_time::ThreadTime::now();
        let retire_clock = Instant::now();
        lease.retire_contents().unwrap();
        costs.retirement_cpu_ns += retire_cpu.elapsed().as_nanos();
        costs.retirement_wall_ns += retire_clock.elapsed().as_nanos();
        assert_eq!(fs::read_dir(store.root.join("store")).unwrap().count(), 0);
        assert!(matches!(
            owner.try_lock(),
            Err(fs::TryLockError::WouldBlock)
        ));
        drop(lease);
        owner.try_lock().unwrap();
    }
    {
        let store = Store::new(worker, 1);
        let mut lease = populate(&store, &mut costs);
        let stages = store.stage_full_tree();
        let profile = store.root.join("store/.spiralcore-profile-v1");
        let profile_bytes = fs::read(&profile).unwrap();
        let saved: Vec<_> = (0..64)
            .map(|index| {
                let id = spiralcore_transfer::object_id(&body(index));
                let path = store
                    .namespace()
                    .join(format!("{}.object", spiralcore_transfer::hex(&id)));
                let bytes = fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect();
        let overflow = store.namespace().join(".stage-123-987");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&overflow)
            .unwrap();
        assert_eq!(store.entries(), 102);
        let owner = File::open(store.root.join("store/.owner")).unwrap();
        gate.meet(2);
        let refusal_cpu = cpu_time::ThreadTime::now();
        let refusal_clock = Instant::now();
        assert_eq!(lease.retire_contents(), Err(Error::Limit));
        costs.refusal_cpu_ns += refusal_cpu.elapsed().as_nanos();
        costs.refusal_wall_ns += refusal_clock.elapsed().as_nanos();
        assert!(matches!(
            owner.try_lock(),
            Err(fs::TryLockError::WouldBlock)
        ));
        for path in stages {
            assert_eq!(fs::read(path).unwrap(), b"retained recovery stage\0");
        }
        assert_eq!(fs::read(profile).unwrap(), profile_bytes);
        for (path, bytes) in saved {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
        assert!(overflow.exists());
        assert!(fs::read(overflow).unwrap().is_empty());
        assert_eq!(owner.metadata().unwrap().len(), 0);
        let held = owner.metadata().unwrap();
        let named = fs::metadata(store.root.join("store/.owner")).unwrap();
        assert_eq!((held.dev(), held.ino()), (named.dev(), named.ino()));
        assert_eq!(store.entries(), 102);
        // This failed lease is held until this fixture ends; no refund is asserted.
        drop(lease);
    }
    costs
}

#[test]
fn populated_owned_stores_recover_and_refuse_overload_with_requested_heap_accounting() {
    const CHILD: &str = "SC_OWNED_ALLOCATION_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "populated_owned_stores_recover_and_refuse_overload_with_requested_heap_accounting",
                "--show-output",
                "--test-threads=1",
            ])
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
        assert!(stdout.contains("test result: ok. 1 passed; 0 failed"));
        assert!(stdout.contains("OWNED_ALLOCATION "));
        let rows: Vec<_> = stdout
            .lines()
            .filter(|line| line.starts_with("OWNED_ALLOCATION "))
            .collect();
        assert_eq!(rows.len(), 1);
        println!("{}", rows[0]);
        return;
    }
    let executable_before = executable_sha256();
    let witness_source = spiralcore_transfer::hex(
        &Sha256::digest(include_bytes!("owned_store_allocation.rs")).into(),
    );
    let allocator_source = spiralcore_transfer::hex(
        &Sha256::digest(include_bytes!("support/requested_allocations.rs")).into(),
    );
    let before = allocations::snapshot();
    let clock = Instant::now();
    let cpu = cpu_time::ProcessTime::now();
    let (a, a_rx) = mpsc::sync_channel(1);
    let (b, b_rx) = mpsc::sync_channel(1);
    let gates = [
        Gate {
            notify: a,
            wait: b_rx,
        },
        Gate {
            notify: b,
            wait: a_rx,
        },
    ];
    let results = std::thread::scope(|scope| {
        let workers: Vec<_> = gates
            .into_iter()
            .enumerate()
            .map(|(worker, gate)| scope.spawn(move || run_store(worker as u8, gate)))
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join())
            .collect::<Vec<_>>()
    });
    let costs: Vec<_> = results.into_iter().map(Result::unwrap).collect();
    let wall_ns = clock.elapsed().as_nanos();
    let process_cpu_ns = cpu.elapsed().as_nanos();
    let measured = allocations::snapshot();
    assert!(
        allocations::valid(),
        "allocation counters overflowed or underflowed"
    );
    let operation = ResourceProfile::SmallV1.operation_charge().unwrap();
    let funded = (operation + WORKSPACE) * STORES as u64;
    assert!(
        (measured.1 as u64) <= funded,
        "requested Rust heap exceeded quoted aggregate"
    );
    let executable_after = executable_sha256();
    assert_eq!(executable_before, executable_after);
    let row = serde_json::json!({"stores":STORES,"publications_per_store":64,"recovery_stages_per_store":32,
        "executable_sha256_before":executable_before,"executable_sha256_after":executable_after,
        "witness_source_sha256":witness_source,"allocator_source_sha256":allocator_source,
        "admitted_entry_count":101,"overflow_entry_count":102,"declared_workspace_per_store":WORKSPACE,
        "declared_operation_per_store":operation,"aggregate_declared_bytes":funded,
        "counter_bookkeeping_valid":true,
        "rust_live_before":before.0,"rust_live_after":measured.0,"rust_peak_requested":measured.1,
        "rust_cumulative_requested":measured.2,"wall_ns":wall_ns,"process_cpu_ns":process_cpu_ns,
        "application_bytes":costs.iter().map(|c|c.application).sum::<u64>(),
        "request_frame_bytes":costs.iter().map(|c|c.request).sum::<u64>(),
        "reply_frame_bytes":costs.iter().map(|c|c.reply).sum::<u64>(),
        "setup_core_cpu_ns":costs.iter().map(|c|c.setup_cpu_ns).sum::<u128>(),
        "setup_core_wall_ns_sum":costs.iter().map(|c|c.setup_wall_ns).sum::<u128>(),
        "sender_outside_dispatch_cpu_ns":costs.iter().map(|c|c.sender_cpu_ns).sum::<u128>(),
        "receiver_dispatch_cpu_ns":costs.iter().map(|c|c.receiver_cpu_ns).sum::<u128>(),
        "initial_readback_cpu_ns":costs.iter().map(|c|c.readback_cpu_ns).sum::<u128>(),
        "recovery_cpu_ns":costs.iter().map(|c|c.recovery_cpu_ns).sum::<u128>(),
        "recovery_wall_ns_sum":costs.iter().map(|c|c.recovery_wall_ns).sum::<u128>(),
        "retirement_cpu_ns":costs.iter().map(|c|c.retirement_cpu_ns).sum::<u128>(),
        "retirement_wall_ns_sum":costs.iter().map(|c|c.retirement_wall_ns).sum::<u128>(),
        "overflow_refusal_cpu_ns":costs.iter().map(|c|c.refusal_cpu_ns).sum::<u128>(),
        "overflow_refusal_wall_ns_sum":costs.iter().map(|c|c.refusal_wall_ns).sum::<u128>(),
        "exact_readbacks":true,"retirement_overload_preserved_all_bytes":true,"original_lock_retained_on_refusal":true,
        "scope":"instrumented host process; requested Rust allocations only; no direct libc/allocator overhead/native stacks/RSS/MVM/WAN bound"});
    println!("OWNED_ALLOCATION {row}");
}
