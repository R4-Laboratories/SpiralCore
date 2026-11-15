#![cfg(any(target_os = "linux", target_os = "macos"))]
//! Explicit component funding declarations; these are not original MVM permits.
use spiralcore_transfer::{
    resource_profile::ResourceProfile,
    v1::{OwnedLaneNamespaces, OwnedStorePolicy, OwnedWorkspaceFunding},
    Error,
};
#[path = "support/requested_allocations.rs"]
pub mod allocations;
#[path = "support/native_stack.rs"]
mod native_stack;
#[global_allocator]
static ALLOCATOR: allocations::RequestedAllocations = allocations::RequestedAllocations;

use spiralcore_transfer::{
    endpoint::{Authority, Context},
    protocol::{Frame, Mode, Offer},
    v1::{
        Capabilities, ObjectEndpoint, OrderedBinding, OrderedLane, OrderedObjectEndpoint,
        OwnedStoreLease,
    },
    wan,
};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

// Qualify the exercised call chain below the existing reservation. A native
// pthread implementation may add stack space beyond Builder's request; observe
// it and keep the full reported extent within the unchanged 2 MiB ceiling.
const WORKER_STACK_REQUEST: usize = 1 << 20;

#[test]
fn filesystem_work_requires_its_exact_heap_and_existing_stack_before_capture() {
    let limits = ResourceProfile::SmallV1.receiver_limits();
    let lanes = OwnedLaneNamespaces::Two([1; 32], [2; 32]);
    let required = OwnedStorePolicy::workspace_requirements(&limits, lanes).unwrap();
    assert_eq!(required.directory_buffer_bytes, 32768);
    assert!(required.retained_requested_bytes > required.directory_buffer_bytes);
    assert!(required.required_heap_bytes <= 1 << 20);
    assert_eq!(required.required_existing_stack_bytes, 2 << 20);
    assert!(required.original_owner_memory_enforcement_required);
    let funded = OwnedWorkspaceFunding {
        heap_bytes: required.required_heap_bytes,
        existing_stack_bytes: required.required_existing_stack_bytes,
    };
    let policy = OwnedStorePolicy::small_v1(&limits, 102, lanes, funded).unwrap();
    assert_eq!(policy.workspace_funding(), funded);
    assert_eq!(policy.workspace_requirement(), required);
    for short in [
        OwnedWorkspaceFunding {
            heap_bytes: funded.heap_bytes - 1,
            ..funded
        },
        OwnedWorkspaceFunding {
            existing_stack_bytes: funded.existing_stack_bytes - 1,
            ..funded
        },
    ] {
        assert!(matches!(
            OwnedStorePolicy::small_v1(&limits, 102, lanes, short),
            Err(Error::Limit)
        ));
    }
    assert!(matches!(
        OwnedStorePolicy::small_v1(&limits, 101, lanes, funded),
        Err(Error::Limit)
    ));
}

struct Fixture {
    root: PathBuf,
    child: PathBuf,
    context: Context,
    other: Context,
    setup_readback_bytes: usize,
}
// The caller owns authenticated authority state. Compare the complete original
// synthetic context without wan::BenchAuthority's freshly allocated expected
// context on every check, so its scaffolding is not mistaken for store work.
struct FixtureAuthority<'a>(&'a Context);
impl Authority for FixtureAuthority<'_> {
    fn authorize(&self, context: &Context) -> spiralcore_transfer::Result<()> {
        if context == self.0 {
            Ok(())
        } else {
            Err(Error::Unauthorized)
        }
    }
}
impl Fixture {
    fn new(index: u8, leaf: &str) -> Self {
        let root = std::env::temp_dir().join(format!("sc-metadata-{}-{index}", std::process::id()));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let child = root.join(leaf);
        fs::DirBuilder::new().mode(0o700).create(&child).unwrap();
        let context = wan::context([80 + index; 32]);
        let other = wan::context([90 + index; 32]);
        let mut fixture = Self {
            root,
            child,
            context,
            other,
            setup_readback_bytes: 0,
        };
        let resources = ResourceProfile::SmallV1;
        let mut limits = resources.receiver_limits();
        limits.cache_bytes = 64 * 50;
        limits.namespace_bytes = limits.cache_bytes;
        let lanes = fixture.lanes();
        let lease = OwnedStoreLease::capture(
            Arc::new(File::open(&fixture.root).unwrap()),
            leaf.as_ref(),
            Arc::new(File::open(&fixture.child).unwrap()),
            OwnedStorePolicy::small_v1(
                &limits,
                102,
                lanes,
                OwnedWorkspaceFunding {
                    heap_bytes: 1 << 20,
                    existing_stack_bytes: 2 << 20,
                },
            )
            .unwrap(),
        )
        .unwrap();
        drop(fixture.open(&lease));
        drop(lease);
        let namespace = fixture.namespace();
        for ns in [&fixture.context, &fixture.other] {
            fs::DirBuilder::new()
                .mode(0o700)
                .create(
                    fixture
                        .child
                        .join(spiralcore_transfer::hex(&ns.namespace().unwrap())),
                )
                .unwrap();
        }
        for value in 0..64u8 {
            let body = [value];
            let id = spiralcore_transfer::object_id(&body);
            let mut record = Vec::new();
            record.extend_from_slice(b"SCPUB001");
            record.extend_from_slice(&1u64.to_le_bytes());
            record.extend_from_slice(&id);
            record.push(0);
            record.extend_from_slice(&body);
            let path = namespace.join(format!("{}.object", spiralcore_transfer::hex(&id)));
            private_write(&path, &record);
            assert_eq!(fs::read(path).unwrap(), record);
            fixture.setup_readback_bytes += record.len();
        }
        for index in 0..32 {
            private_write(
                &namespace.join(format!(".stage-{}-{index}", "7".repeat(240))),
                b"retained stage\0",
            );
        }
        private_write(
            &fixture.child.join(".profile-123-456"),
            b"retained profile stage\0",
        );
        assert_eq!(fixture.entries(), 101);
        fixture
    }
    fn namespace(&self) -> PathBuf {
        self.child
            .join(spiralcore_transfer::hex(&self.context.namespace().unwrap()))
    }
    fn lanes(&self) -> OwnedLaneNamespaces {
        OwnedLaneNamespaces::Two(
            self.context.namespace().unwrap(),
            self.other.namespace().unwrap(),
        )
    }
    fn open(&self, lease: &OwnedStoreLease) -> ObjectEndpoint<FixtureAuthority<'_>> {
        let caps = Capabilities::small_v1();
        let resources = ResourceProfile::SmallV1;
        let mut limits = resources.receiver_limits();
        limits.cache_bytes = 64 * 50;
        limits.namespace_bytes = limits.cache_bytes;
        // Capture's exact limits must match the endpoint's limits.
        ObjectEndpoint::open_at_with_resource_profile(
            lease,
            limits,
            FixtureAuthority(&self.context),
            caps.negotiate(&caps).unwrap(),
            resources,
        )
        .unwrap()
    }
    fn entries(&self) -> usize {
        fs::read_dir(&self.child)
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
    fn verify_records(&self, expected: usize) -> usize {
        let mut seen = 0;
        let mut bytes = 0;
        for entry in fs::read_dir(self.namespace()).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension() != Some("object".as_ref()) {
                continue;
            }
            let actual = fs::read(entry.path()).unwrap();
            assert_eq!(actual.len(), 50);
            let value = actual[49];
            assert!(value < 64);
            assert_eq!(&actual[..8], b"SCPUB001");
            assert_eq!(&actual[8..16], &1u64.to_le_bytes());
            assert_eq!(&actual[16..48], &spiralcore_transfer::object_id(&[value]));
            assert_eq!(actual[48], 0);
            assert_eq!(
                entry.file_name(),
                format!(
                    "{}.object",
                    spiralcore_transfer::hex(&spiralcore_transfer::object_id(&[value]))
                )
                .as_str()
            );
            seen += 1;
            bytes += actual.len();
        }
        assert_eq!(seen, expected);
        bytes
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn private_write(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
}
fn window_json(window: allocations::HeapWindow) -> serde_json::Value {
    serde_json::json!({"allocation_requests":window.count,"largest_requested":window.largest,"cumulative_requested":window.cumulative,"live_before":window.live_before,"peak_live_requested":window.peak_live,"live_after":window.live_after})
}

#[test]
fn metadata_heavy_stores_keep_their_preallocated_workspace_and_original_custody() {
    const NAME: &str =
        "metadata_heavy_stores_keep_their_preallocated_workspace_and_original_custody";
    const CHILD: &str = "SC_METADATA_WORKSPACE_CHILD";
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
            .filter(|line| line.starts_with("OWNED_WORKSPACE "))
            .collect();
        assert_eq!(rows.len(), 1);
        println!("{}", rows[0]);
        return;
    }
    std::thread::Builder::new()
        .stack_size(WORKER_STACK_REQUEST)
        .spawn(metadata_cases)
        .unwrap()
        .join()
        .unwrap();
}

fn metadata_cases() {
    let (actual_stack_bytes, stack_guard_bytes) = native_stack::current();
    assert!(actual_stack_bytes >= WORKER_STACK_REQUEST);
    assert!(actual_stack_bytes <= 2 << 20);
    let mut rows = Vec::new();
    for index in 0..2u8 {
        let lifecycle_clock = Instant::now();
        let lifecycle_cpu = cpu_time::ProcessTime::now();
        let leaf = if index == 0 {
            "store".to_owned()
        } else if cfg!(target_os = "macos") {
            "\u{0800}".repeat(128)
        } else {
            "x".repeat(255)
        };
        let setup_clock = Instant::now();
        let setup_cpu = cpu_time::ProcessTime::now();
        let fixture = Fixture::new(index, &leaf);
        let setup_wall = setup_clock.elapsed().as_nanos();
        let setup_cpu = setup_cpu.elapsed().as_nanos();
        let mut limits = ResourceProfile::SmallV1.receiver_limits();
        limits.cache_bytes = 64 * 50;
        limits.namespace_bytes = limits.cache_bytes;
        let required = OwnedStorePolicy::workspace_requirements(&limits, fixture.lanes()).unwrap();
        let funding = OwnedWorkspaceFunding {
            heap_bytes: required.required_heap_bytes,
            existing_stack_bytes: required.required_existing_stack_bytes,
        };
        let owner_before = fs::metadata(fixture.child.join(".owner")).unwrap().ino();
        for short in [
            OwnedWorkspaceFunding {
                heap_bytes: funding.heap_bytes - 1,
                ..funding
            },
            OwnedWorkspaceFunding {
                existing_stack_bytes: funding.existing_stack_bytes - 1,
                ..funding
            },
        ] {
            assert!(matches!(
                OwnedStorePolicy::small_v1(&limits, 102, fixture.lanes(), short),
                Err(Error::Limit)
            ));
            assert_eq!(fixture.entries(), 101);
            assert_eq!(fixture.verify_records(64), 3200);
            assert_eq!(
                fs::metadata(fixture.child.join(".owner")).unwrap().ino(),
                owner_before
            );
        }
        let policy = OwnedStorePolicy::small_v1(&limits, 102, fixture.lanes(), funding).unwrap();
        let parent = Arc::new(File::open(&fixture.root).unwrap());
        let child = Arc::new(File::open(&fixture.child).unwrap());
        let capture_clock = Instant::now();
        let capture_cpu = cpu_time::ProcessTime::now();
        let (lease, capture_heap) = allocations::measure_heap(|| {
            OwnedStoreLease::capture(parent, leaf.as_ref(), child, policy)
        });
        let capture_wall = capture_clock.elapsed().as_nanos();
        let capture_cpu = capture_cpu.elapsed().as_nanos();
        let mut lease = lease.unwrap();
        assert_eq!(
            capture_heap.count,
            usize::from(required.retained_allocation_count)
        );
        assert!(capture_heap.cumulative as u64 <= required.required_heap_bytes);
        assert!(
            capture_heap.live_after - capture_heap.live_before
                <= required.retained_requested_bytes as usize
        );
        assert_eq!(fixture.entries(), 101, "capture has no cleanup effects");
        let binding = OrderedBinding::new(
            &fixture.context,
            fixture.context.namespace().unwrap(),
            0,
            b"metadata.application\0",
            b"metadata.artifact\0",
        )
        .unwrap();
        let recover_clock = Instant::now();
        let recover_cpu = cpu_time::ProcessTime::now();
        let (endpoint, recover_heap) = allocations::measure_heap(|| fixture.open(&lease));
        let recover_wall = recover_clock.elapsed().as_nanos();
        let recover_cpu = recover_cpu.elapsed().as_nanos();
        assert_eq!(
            recover_heap.cumulative, 0,
            "recovery metadata is already allocated"
        );
        assert_eq!(fixture.entries(), 68);
        let index_clock = Instant::now();
        let index_cpu = cpu_time::ProcessTime::now();
        let (ordered, index_heap) = allocations::measure_heap(|| {
            OrderedObjectEndpoint::new(endpoint, binding.clone(), &fixture.context)
        });
        let index_wall = index_clock.elapsed().as_nanos();
        let index_cpu = index_cpu.elapsed().as_nanos();
        let mut ordered = ordered.unwrap();
        assert_eq!(ordered.snapshot(&fixture.context).unwrap().publications, 64);
        assert!(
            index_heap.largest <= 2,
            "only the one-byte payload verification uses temporary heap: {index_heap:?}"
        );
        assert!(index_heap.cumulative <= 64 * 2);
        let cached = Offer::cold(&[0], Mode::Raw);
        let probe = Frame {
            kind: 4,
            transfer: binding.transfer_id(OrderedLane::Artifact, 0),
            seq: 0,
            body: [
                cached.id().unwrap().as_slice(),
                cached.encode().unwrap().as_slice(),
            ]
            .concat(),
        };
        let pending = Offer::cold(&[200], Mode::Raw);
        let begin = Frame {
            kind: 1,
            transfer: binding.transfer_id(OrderedLane::Artifact, 1),
            seq: 0,
            body: pending.encode().unwrap(),
        };
        let request_bytes = probe.encode().unwrap().len() + begin.encode().unwrap().len();
        let plan_clock = Instant::now();
        let plan_cpu = cpu_time::ProcessTime::now();
        let ((probe_reply, begin_reply), plan_heap) = allocations::measure_heap(|| {
            let probe_reply = ordered
                .handle(OrderedLane::Artifact, 0, &fixture.context, &probe, 1)
                .unwrap();
            let begin_reply = ordered
                .handle(OrderedLane::Artifact, 1, &fixture.context, &begin, 2)
                .unwrap();
            (probe_reply, begin_reply)
        });
        let plan_wall = plan_clock.elapsed().as_nanos();
        let plan_cpu = plan_cpu.elapsed().as_nanos();
        assert_eq!(probe_reply.kind, 103);
        assert_eq!(begin_reply.kind, 101);
        let reply_bytes = probe_reply.encode().unwrap().len() + begin_reply.encode().unwrap().len();
        let snapshot = ordered.snapshot(&fixture.context).unwrap();
        assert_eq!((snapshot.publications, snapshot.evictions), (62, 2));
        let final_readback = fixture.verify_records(62);
        assert_eq!(
            ordered
                .read_object(&fixture.context, cached.object)
                .unwrap(),
            [0]
        );
        assert_eq!(
            lease.retire_contents(),
            Err(Error::Storage),
            "live endpoint/index retain original custody"
        );
        let owner = File::open(fixture.child.join(".owner")).unwrap();
        assert!(matches!(
            owner.try_lock(),
            Err(fs::TryLockError::WouldBlock)
        ));
        drop(ordered);
        let retire_clock = Instant::now();
        let retire_cpu = cpu_time::ProcessTime::now();
        let (retired, retire_heap) = allocations::measure_heap(|| lease.retire_contents());
        let retire_wall = retire_clock.elapsed().as_nanos();
        let retire_cpu = retire_cpu.elapsed().as_nanos();
        retired.unwrap();
        assert_eq!(
            retire_heap.cumulative, 0,
            "retirement metadata is already allocated"
        );
        assert!(matches!(
            owner.try_lock(),
            Err(fs::TryLockError::WouldBlock)
        ));
        assert_eq!(fs::read_dir(&fixture.child).unwrap().count(), 0);
        let release_clock = Instant::now();
        let release_cpu = cpu_time::ProcessTime::now();
        let ((), release_heap) = allocations::measure_heap(|| drop(lease));
        let release_wall = release_clock.elapsed().as_nanos();
        let release_cpu = release_cpu.elapsed().as_nanos();
        assert_eq!(release_heap.cumulative, 0);
        assert!(release_heap.live_before - release_heap.live_after >= 32768);
        assert!(
            owner.try_lock().is_ok(),
            "only final lease destruction releases the writer"
        );
        let lifecycle_wall = lifecycle_clock.elapsed().as_nanos();
        let lifecycle_cpu = lifecycle_cpu.elapsed().as_nanos();
        assert!(allocations::valid());
        let mut row = serde_json::json!({"component_bytes":leaf.len(),"publications":64,"body_bytes_each":1,"initial_entries":101,"post_recovery_entries":68,"setup_readback_bytes":fixture.setup_readback_bytes,"post_eviction_readback_bytes":final_readback,"evictions":2,"request_frame_bytes":request_bytes,"reply_frame_bytes":reply_bytes,"required_heap_bytes":required.required_heap_bytes,"required_existing_stack_bytes":required.required_existing_stack_bytes,"synchronization_bytes":required.synchronization_bytes,"capture_allocation_count":required.retained_allocation_count,"setup_wall_ns":setup_wall,"setup_process_cpu_ns":setup_cpu,"capture_wall_ns":capture_wall,"capture_process_cpu_ns":capture_cpu,"recovery_wall_ns":recover_wall,"recovery_process_cpu_ns":recover_cpu,"index_wall_ns":index_wall,"index_process_cpu_ns":index_cpu,"planning_wall_ns":plan_wall,"planning_process_cpu_ns":plan_cpu,"retirement_wall_ns":retire_wall,"retirement_process_cpu_ns":retire_cpu,"lease_release_wall_ns":release_wall,"lease_release_process_cpu_ns":release_cpu,"lifecycle_wall_ns":lifecycle_wall,"lifecycle_process_cpu_ns":lifecycle_cpu,"lifecycle_scope":"setup through verified retirement and final core-lease drop; excludes fixture-parent removal","sender_cpu_ns":null,"network_bytes":null,"capture_heap":window_json(capture_heap),"recovery_heap":window_json(recover_heap),"index_heap":window_json(index_heap),"planning_heap":window_json(plan_heap),"retirement_heap":window_json(retire_heap),"lease_release_heap":window_json(release_heap)});
        for (name, bytes) in [
            ("directory_buffer_bytes", required.directory_buffer_bytes),
            (
                "fixed_store_layout_bytes",
                required.fixed_store_layout_bytes,
            ),
            ("ordered_index_bytes", required.ordered_index_bytes),
            ("scan_record_bytes", required.scan_record_bytes),
            (
                "retained_requested_bytes",
                required.retained_requested_bytes,
            ),
            (
                "peak_temporary_requested_bytes",
                required.peak_temporary_requested_bytes,
            ),
        ] {
            row[name] = bytes.into();
        }
        row["original_owner_memory_enforcement_required"] =
            required.original_owner_memory_enforcement_required.into();
        rows.push(row);
    }
    println!(
        "OWNED_WORKSPACE {}",
        serde_json::json!({"stores":2,"requested_stack_bytes":WORKER_STACK_REQUEST,"actual_stack_bytes":actual_stack_bytes,"stack_guard_bytes":stack_guard_bytes,"cases":rows,"scope":"isolated serial host-process component; manually preloaded records and encoded frames, not network or original MVM permits; requested Rust payload, not rounded allocator/direct-libc/RSS; smaller witness-worker request and native reported extent within existing 2 MiB ceiling, not production pthread/alternate-stack qualification; original owner enforcement remains required"})
    );
}

#[test]
fn namespace_and_hex_keep_the_independent_legacy_byte_transcripts() {
    for block in 0..8u8 {
        let id = std::array::from_fn(|at| block * 32 + at as u8);
        let legacy: String = id.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(spiralcore_transfer::hex(&id), legacy);
    }
    for text in ["a".to_owned(), "z".repeat(256), "λ".repeat(128)] {
        let context = Context {
            tenant: text.clone(),
            domain: text.clone(),
            sender: text.clone(),
            recipient: text.clone(),
            mailbox: text.clone(),
            capability: text.clone(),
            authorization_scope: text,
            policy_epoch: u64::MAX,
            execution_epoch: 0,
        };
        let mut transcript = Vec::new();
        for value in [
            &context.tenant,
            &context.domain,
            &context.sender,
            &context.recipient,
            &context.mailbox,
            &context.capability,
            &context.authorization_scope,
        ] {
            transcript.extend_from_slice(&(value.len() as u16).to_le_bytes());
            transcript.extend_from_slice(value.as_bytes());
        }
        transcript.extend_from_slice(&context.policy_epoch.to_le_bytes());
        transcript.extend_from_slice(&context.execution_epoch.to_le_bytes());
        assert_eq!(
            context.namespace().unwrap(),
            spiralcore_transfer::hash(b"spiralcore.object.namespace.v1\0", &[&transcript])
        );
        let mut invalid = context.clone();
        invalid.mailbox.clear();
        assert_eq!(invalid.namespace(), Err(Error::Malformed));
        invalid.mailbox = "x".repeat(257);
        assert_eq!(invalid.namespace(), Err(Error::Malformed));
    }
}
