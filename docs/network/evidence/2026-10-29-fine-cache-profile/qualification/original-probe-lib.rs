use cpu_time::ProcessTime;
use serde::{Deserialize, Serialize};
use spiralcore_transfer::{
    delta,
    endpoint::{Authority, Context, Endpoint, Limits},
    hash, hex, object_id,
    protocol::{Frame, Mode, Offer},
    scan_profile::{self, ScanProfile},
    Id, MAX_CHUNK,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::Instant,
};
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const POPULATIONS: [usize; 4] = [0, 64, 256, 768];
const OWNED: &[u8] = b"SpiralCore receiver diagnostic owned directory v1\n";
pub const SOURCE_REVISION: &str = match option_env!("SPIRAL_PROBE_LIBRARY_REVISION") {
    Some(s) => s,
    None => "unrecorded",
};
pub const SOURCE_SHA256: &str = match option_env!("SPIRAL_PROBE_LIBRARY_SHA256") {
    Some(s) => s,
    None => "unrecorded",
};

pub fn fixture(seed: u64) -> (Vec<u8>, Vec<u8>) {
    let mut s = seed.max(1);
    let base: Vec<_> = (0..1 << 20)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s as u8
        })
        .collect();
    let mut target = base.clone();
    for b in &mut target[524288..524320] {
        *b ^= 0xa5;
    }
    (base, target)
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Cost {
    pub wall_ns: u64,
    pub cpu_ns: u64,
    pub frames: usize,
}
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Stages {
    pub begin: Cost,
    pub data: Cost,
    pub finish: Cost,
    pub total: Cost,
}
impl Stages {
    pub fn add(&mut self, kind: u8, cost: Cost) -> std::result::Result<(), &'static str> {
        let old = match kind {
            1 => self.begin,
            2 => self.data,
            3 => self.finish,
            _ => return Err("unexpected stage"),
        };
        let stage = old.plus(cost)?;
        let total = self.total.plus(cost)?;
        match kind {
            1 => self.begin = stage,
            2 => self.data = stage,
            3 => self.finish = stage,
            _ => unreachable!(),
        }
        self.total = total;
        Ok(())
    }
}
impl Cost {
    fn plus(self, other: Self) -> std::result::Result<Self, &'static str> {
        Ok(Self {
            wall_ns: self
                .wall_ns
                .checked_add(other.wall_ns)
                .ok_or("time overflow")?,
            cpu_ns: self
                .cpu_ns
                .checked_add(other.cpu_ns)
                .ok_or("time overflow")?,
            frames: self
                .frames
                .checked_add(other.frames)
                .ok_or("frame overflow")?,
        })
    }
}
fn elapsed(cpu: ProcessTime, wall: Instant, frames: usize) -> Cost {
    let wall_ns = wall
        .elapsed()
        .as_nanos()
        .try_into()
        .expect("bounded diagnostic duration");
    let cpu_ns = cpu
        .elapsed()
        .as_nanos()
        .try_into()
        .expect("bounded diagnostic duration");
    Cost {
        wall_ns,
        cpu_ns,
        frames,
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Arm {
    pub sparse: bool,
    pub tiny: bool,
}
impl Arm {
    pub fn label(self) -> String {
        format!(
            "{}-{}",
            if self.sparse { "sparse" } else { "raw" },
            if self.tiny {
                "node-budget-1"
            } else {
                "node-budget-262144"
            }
        )
    }
    pub fn limits(self) -> Limits {
        Limits {
            max_nodes: if self.tiny { 1 } else { 262144 },
            graph_cache_bytes: 0,
            graph_cache_entries: 0,
            ..Limits::default()
        }
    }
}
pub fn ordered_arms(block: usize, population: usize) -> [Arm; 4] {
    let all = [
        Arm {
            sparse: false,
            tiny: false,
        },
        Arm {
            sparse: true,
            tiny: false,
        },
        Arm {
            sparse: false,
            tiny: true,
        },
        Arm {
            sparse: true,
            tiny: true,
        },
    ];
    let offset = (block
        + POPULATIONS
            .iter()
            .position(|n| *n == population)
            .unwrap_or(0))
        % 4;
    std::array::from_fn(|i| all[(i + offset) % 4])
}
pub fn context(name: &str) -> Context {
    Context {
        tenant: "public-diagnostic".into(),
        domain: "local".into(),
        sender: "sender".into(),
        recipient: "receiver".into(),
        mailbox: name.into(),
        capability: "synthetic-not-a-network-capability".into(),
        authorization_scope: "objects".into(),
        policy_epoch: 1,
        execution_epoch: 1,
    }
}
pub struct Policy;
impl Authority for Policy {
    fn authorize(&self, c: &Context) -> spiralcore_transfer::Result<()> {
        let expected = context(&c.mailbox);
        if *c == expected
            && (c.mailbox == "active"
                || c.mailbox
                    .strip_prefix("background-")
                    .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())))
        {
            Ok(())
        } else {
            Err(spiralcore_transfer::Error::Unauthorized)
        }
    }
}
pub fn frames(base: &[u8], target: &[u8], sparse: bool, transfer: Id) -> Result<Vec<Frame>> {
    let offer = if sparse {
        Offer {
            mode: Mode::SparseV1,
            len: target.len() as u64,
            object: object_id(target),
            manifest: None,
            base: Some(object_id(base)),
        }
    } else {
        Offer::cold(target, Mode::Raw)
    };
    let bodies = if sparse {
        delta::encode(base, target)?
            .ok_or("fixture unexpectedly selects raw fallback")?
            .frames
    } else {
        target
            .chunks(MAX_CHUNK)
            .map(|c| {
                let mut b = Vec::with_capacity(c.len() + 5);
                b.push(0);
                b.extend_from_slice(&(c.len() as u32).to_le_bytes());
                b.extend_from_slice(c);
                b
            })
            .collect()
    };
    let mut result = Vec::with_capacity(bodies.len() + 2);
    result.push(Frame {
        kind: 1,
        transfer,
        seq: 0,
        body: offer.encode()?,
    });
    for body in bodies {
        result.push(Frame {
            kind: 2,
            transfer,
            seq: result.len() as u64,
            body,
        });
    }
    result.push(Frame {
        kind: 3,
        transfer,
        seq: result.len() as u64,
        body: Vec::new(),
    });
    Ok(result)
}
fn check_reply(f: &Frame, r: &Frame, id: Id, len: usize, accepted: &mut u64) -> Result<()> {
    if r.transfer != f.transfer || r.seq != f.seq {
        return Err("reply identity mismatch".into());
    }
    match f.kind {
        1 if r.kind == 101 => {
            let offer = Offer::decode(&f.body)?;
            let mut expected = vec![offer.mode as u8, u8::from(offer.base.is_some())];
            expected.extend_from_slice(&1u64.to_le_bytes());
            if f.seq != 0
                || *accepted != 0
                || offer.object != id
                || offer.len != len as u64
                || r.body != expected
            {
                return Err("invalid Ready body/mode/base pin/next sequence".into());
            }
            Ok(())
        }
        2 if r.kind == 102 => {
            let expanded =
                u32::from_le_bytes(f.body.get(1..5).ok_or("short Data body")?.try_into()?) as u64;
            let next = accepted
                .checked_add(expanded)
                .ok_or("accepted count overflow")?;
            if expanded == 0 || next > len as u64 || r.body.as_slice() != next.to_le_bytes() {
                return Err("invalid Accepted body or cumulative expanded count".into());
            }
            *accepted = next;
            Ok(())
        }
        3 if r.kind == 103 && r.body == id && *accepted == len as u64 => Ok(()),
        _ => Err(format!(
            "unexpected reply {} to stage {}; no Begin cache hits permitted",
            r.kind, f.kind
        )
        .into()),
    }
}
fn send(
    ep: &mut Endpoint<Policy>,
    c: &Context,
    bytes: &[u8],
    transfer: Id,
    now: u64,
) -> Result<()> {
    let id = object_id(bytes);
    let mut accepted = 0;
    for f in frames(&[], bytes, false, transfer)? {
        let r = ep.handle(c, &f, now)?;
        check_reply(&f, &r, id, bytes.len(), &mut accepted)?;
    }
    Ok(())
}
fn mkdir(path: &Path) -> Result<()> {
    let mut d = fs::DirBuilder::new();
    d.mode(0o700).create(path)?;
    File::open(path)?.sync_all()?;
    if let Some(p) = path.parent() {
        File::open(p)?.sync_all()?;
    }
    Ok(())
}
fn output(path: &Path) -> Result<File> {
    Ok(OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?)
}
pub fn append(f: &mut File, value: &impl Serialize) -> Result<()> {
    serde_json::to_writer(&mut *f, value)?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    Ok(())
}
fn owned(root: &Path) -> Result<()> {
    if fs::symlink_metadata(root)?.file_type().is_symlink()
        || fs::read(root.join(".probe-owned"))? != OWNED
    {
        return Err("not an owned diagnostic directory".into());
    }
    Ok(())
}
fn canonical_hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Inventory {
    pub namespaces: usize,
    pub publications: usize,
    pub publication_bytes: u64,
    pub active_publications: usize,
    pub active_bytes: u64,
    pub manifests: usize,
}
pub fn inventory(root: &Path, active: Id) -> Result<Inventory> {
    let mut result = Inventory::default();
    let active = hex(&active);
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF8 diagnostic entry")?;
        if name == ".owner" || name == ".spiralcore-profile-v1" {
            continue;
        }
        if !entry.file_type()?.is_dir() || !canonical_hex(&name) {
            return Err("unexpected diagnostic namespace".into());
        }
        result.namespaces += 1;
        for file in fs::read_dir(entry.path())? {
            let file = file?;
            let fname = file
                .file_name()
                .into_string()
                .map_err(|_| "non-UTF8 publication")?;
            if !file.file_type()?.is_file()
                || !fname.strip_suffix(".object").is_some_and(canonical_hex)
            {
                return Err("unexpected publication/staging entry".into());
            }
            let mut h = [0u8; 49];
            File::open(file.path())?.read_exact(&mut h)?;
            if &h[..8] != b"SCPUB001" || h[48] > 1 {
                return Err("invalid publication header".into());
            }
            let len = u64::from_le_bytes(h[8..16].try_into()?);
            let actual = file.metadata()?.len();
            if actual != len + 49 + 32 * u64::from(h[48]) {
                return Err("invalid publication length".into());
            }
            result.publications += 1;
            result.publication_bytes += actual;
            result.manifests += usize::from(h[48]);
            if name == active {
                result.active_publications += 1;
                result.active_bytes += actual;
            }
        }
    }
    Ok(result)
}
pub fn prepare(root: &Path) -> Result<()> {
    mkdir(root)?;
    let mut marker = output(&root.join(".probe-owned"))?;
    marker.write_all(OWNED)?;
    marker.sync_all()?;
    mkdir(&root.join("templates"))?;
    mkdir(&root.join("working"))?;
    let mut log = output(&root.join("setup.jsonl"))?;
    let (base, _) = fixture(42);
    let active = context("active");
    for n in POPULATIONS {
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        let dir = root.join("templates").join(n.to_string());
        let mut ep = Endpoint::open(
            &dir,
            Limits {
                max_states: 2048,
                graph_cache_entries: 0,
                graph_cache_bytes: 0,
                ..Limits::default()
            },
            Policy,
        )?;
        for i in 0..n {
            let data = hash(
                b"spiralcore.receiver.background.v1\0",
                &[&(i as u64).to_le_bytes()],
            );
            let c = context(&format!("background-{}", i / 2));
            send(
                &mut ep,
                &c,
                &data,
                hash(b"background-transfer", &[&(i as u64).to_le_bytes()]),
                i as u64,
            )?;
            if ep.read_object(&c, object_id(&data))? != data {
                return Err("background mismatch".into());
            }
        }
        send(&mut ep, &active, &base, [0xaa; 32], n as u64)?;
        if ep.read_object(&active, object_id(&base))? != base {
            return Err("base mismatch".into());
        }
        drop(ep);
        File::open(&dir)?.sync_all()?;
        let cost = elapsed(cpu, wall, 0);
        let inv = inventory(&dir, active.namespace()?)?;
        if inv.publications != n + 1
            || inv.namespaces != n / 2 + 1
            || inv.active_publications != 1
            || inv.manifests != n + 1
        {
            return Err("template population mismatch".into());
        }
        append(
            &mut log,
            &serde_json::json!({"kind":"template","population":n,"cost":cost,"inventory":inv,"base_object":hex(&object_id(&base)),"base_bytes":base.len(),"policy":"setup default graph budget; no graph retention; all publication files synchronized by Endpoint API"}),
        )?;
    }
    File::open(root)?.sync_all()?;
    Ok(())
}
fn clone_synced(source: &Path, target: &Path) -> Result<()> {
    if !fs::symlink_metadata(source)?.is_dir() {
        return Err("template is not a directory".into());
    }
    mkdir(target)?;
    for e in fs::read_dir(source)? {
        let e = e?;
        let dest = target.join(e.file_name());
        let ty = e.file_type()?;
        if ty.is_dir() {
            clone_synced(&e.path(), &dest)?;
        } else if ty.is_file() {
            let mut input = File::open(e.path())?;
            let mut out = output(&dest)?;
            std::io::copy(&mut input, &mut out)?;
            out.sync_all()?;
        } else {
            return Err("nonregular template entry".into());
        }
    }
    File::open(target)?.sync_all()?;
    File::open(target.parent().ok_or("clone parent")?)?.sync_all()?;
    Ok(())
}
#[derive(Debug, Serialize)]
pub struct FrameSample {
    pub kind: u8,
    pub sequence: u64,
    pub request_bytes: usize,
    pub reply_kind: Option<u8>,
    pub reply_bytes: Option<usize>,
    pub reply_body_prefix: Option<Vec<u8>>,
    pub reply_body_truncated: bool,
    pub cost: Cost,
    pub disk_usage: ScanProfile,
}
fn check_scan_profile(kind: u8, p: ScanProfile, before: &Inventory) -> Result<()> {
    if kind == 2 {
        if p != ScanProfile::default() {
            return Err("unexpected Data quota scan".into());
        }
        return Ok(());
    }
    if !matches!(kind, 1 | 3)
        || p.overflow
        || p.last_error.is_some()
        || p.last_error_stage.is_some()
        || (p.calls, p.successes, p.errors) != (1, 1, 0)
        || p.root_read_dir_calls != 1
        || p.root_entries != before.namespaces as u64 + 2
        || p.root_file_type_calls != p.root_entries
        || p.namespace_read_dir_calls != before.namespaces as u64
        || p.namespace_entries != before.publications as u64
        || p.object_file_type_calls != p.namespace_entries
        || p.metadata_calls != p.namespace_entries
        || p.metadata_successes != p.metadata_calls
        || p.namespace_comparisons != p.metadata_calls
        || p.objects_counted != p.metadata_calls
        || p.returned_objects != before.publications as u64
        || p.returned_scoped_bytes != before.active_bytes
        || p.returned_all_bytes != before.publication_bytes
    {
        return Err("quota scan counters differ from exact pre-publication inventory".into());
    }
    Ok(())
}
#[derive(Debug, Serialize)]
pub struct Observation {
    pub kind: &'static str,
    pub variant: String,
    pub block: usize,
    pub population: usize,
    pub arm: Arm,
    pub arm_label: String,
    pub base_object: String,
    pub target_object: String,
    pub object_bytes: usize,
    pub active_namespace: String,
    pub status: String,
    pub error: Option<String>,
    pub error_stage: String,
    pub work_cache: PathBuf,
    pub clone_and_sync: Cost,
    pub endpoint_open: Cost,
    pub frame_preparation: Cost,
    pub handler_stages: Stages,
    pub dispatch_including_instrumentation: Cost,
    pub local_operation_total: Cost,
    pub close_reopen_verify: Cost,
    pub before: Option<Inventory>,
    pub after: Option<Inventory>,
    pub target_manifest_present: Option<bool>,
    pub graph_cache_after: Option<serde_json::Value>,
    pub frame_samples: Vec<FrameSample>,
}
pub fn observe(
    root: &Path,
    variant: &str,
    block: usize,
    n: usize,
    arm: Arm,
    base: &[u8],
    target: &[u8],
) -> Observation {
    let active = context("active");
    let ns = active.namespace().expect("constant context");
    let work = root.join("working").join(format!(
        "{variant}-{}-{block}-{n}-{}",
        std::process::id(),
        arm.label()
    ));
    let target_id = object_id(target);
    let mut row = Observation {
        kind: "observation",
        variant: variant.into(),
        block,
        population: n,
        arm,
        arm_label: arm.label(),
        base_object: hex(&object_id(base)),
        target_object: hex(&target_id),
        object_bytes: target.len(),
        active_namespace: hex(&ns),
        status: "failed".into(),
        error: None,
        error_stage: "ownership".into(),
        work_cache: work.clone(),
        clone_and_sync: Cost::default(),
        endpoint_open: Cost::default(),
        frame_preparation: Cost::default(),
        handler_stages: Stages::default(),
        dispatch_including_instrumentation: Cost::default(),
        local_operation_total: Cost::default(),
        close_reopen_verify: Cost::default(),
        before: None,
        after: None,
        target_manifest_present: None,
        graph_cache_after: None,
        frame_samples: Vec::new(),
    };
    let result = (|| -> Result<()> {
        owned(root)?;
        row.error_stage = "clone_sync".into();
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        let cloned = clone_synced(&root.join("templates").join(n.to_string()), &work);
        row.clone_and_sync = elapsed(cpu, wall, 0);
        cloned?;
        row.error_stage = "inventory_before".into();
        let inv = inventory(&work, ns)?;
        if inv.publications != n + 1 || inv.namespaces != n / 2 + 1 || inv.active_publications != 1
        {
            return Err("unexpected before inventory".into());
        }
        row.before = Some(inv);
        row.error_stage = "open".into();
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        let opened = Endpoint::open(&work, arm.limits(), Policy);
        row.endpoint_open = elapsed(cpu, wall, 0);
        let mut ep = opened?;
        row.error_stage = "frame_preparation".into();
        let operation_cpu = ProcessTime::now();
        let operation_wall = Instant::now();
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        let transfer = hash(
            b"spiralcore.receiver.diagnostic.transfer.v1\0",
            &[
                variant.as_bytes(),
                &(block as u64).to_le_bytes(),
                &(n as u64).to_le_bytes(),
                row.arm_label.as_bytes(),
            ],
        );
        let requests = frames(base, target, arm.sparse, transfer);
        row.frame_preparation = elapsed(cpu, wall, 0);
        let requests = match requests {
            Ok(r) => r,
            Err(e) => {
                row.local_operation_total = elapsed(operation_cpu, operation_wall, 0);
                return Err(e);
            }
        };
        row.frame_samples = Vec::with_capacity(requests.len());
        row.error_stage = "handler".into();
        let dispatch_cpu = ProcessTime::now();
        let dispatch_wall = Instant::now();
        let mut accepted = 0;
        let handled = (|| -> Result<()> {
            for f in &requests {
                scan_profile::reset();
                let cpu = ProcessTime::now();
                let wall = Instant::now();
                let result = ep.handle(&active, f, 0);
                let cost = elapsed(cpu, wall, 1);
                let disk_usage = scan_profile::take();
                row.handler_stages.add(f.kind, cost)?;
                row.frame_samples.push(FrameSample {
                    kind: f.kind,
                    sequence: f.seq,
                    request_bytes: f.body.len() + 56,
                    reply_kind: result.as_ref().ok().map(|r| r.kind),
                    reply_bytes: result.as_ref().ok().map(|r| r.body.len() + 56),
                    reply_body_prefix: result
                        .as_ref()
                        .ok()
                        .map(|r| r.body[..r.body.len().min(256)].to_vec()),
                    reply_body_truncated: result.as_ref().is_ok_and(|r| r.body.len() > 256),
                    cost,
                    disk_usage,
                });
                let reply = result?;
                check_scan_profile(
                    f.kind,
                    disk_usage,
                    row.before.as_ref().ok_or("missing inventory")?,
                )?;
                if disk_usage.cpu_ns > cost.cpu_ns || disk_usage.wall_ns > cost.wall_ns {
                    return Err("scan clocks exceed containing handler clocks".into());
                }
                check_reply(f, &reply, target_id, target.len(), &mut accepted)?;
            }
            Ok(())
        })();
        row.dispatch_including_instrumentation =
            elapsed(dispatch_cpu, dispatch_wall, row.frame_samples.len());
        drop(requests);
        row.local_operation_total = elapsed(operation_cpu, operation_wall, row.frame_samples.len());
        handled?;
        let stats = ep.graph_cache_stats();
        row.graph_cache_after = Some(
            serde_json::json!({"hits":stats.hits,"misses":stats.misses,"evictions":stats.evictions,"entries":stats.entries,"resident_bytes":stats.resident_bytes}),
        );
        if stats.entries != 0 || stats.resident_bytes != 0 {
            return Err("retained graph in retention-disabled arm".into());
        }
        row.error_stage = "close_reopen_verify".into();
        let cpu = ProcessTime::now();
        let wall = Instant::now();
        drop(ep);
        let mut reopened = Endpoint::open(&work, arm.limits(), Policy)?;
        if reopened.read_object(&active, object_id(target))? != target
            || reopened.read_object(&active, object_id(base))? != base
        {
            return Err("reopened bytes differ".into());
        }
        drop(reopened);
        row.close_reopen_verify = elapsed(cpu, wall, 0);
        row.error_stage = "inventory_after".into();
        let inv = inventory(&work, ns)?;
        if inv.publications != n + 2 || inv.namespaces != n / 2 + 1 || inv.active_publications != 2
        {
            return Err("unexpected after inventory".into());
        }
        let mut h = [0u8; 49];
        File::open(
            work.join(hex(&ns))
                .join(format!("{}.object", hex(&object_id(target)))),
        )?
        .read_exact(&mut h)?;
        let ready = h[48] == 1;
        row.target_manifest_present = Some(ready);
        if ready != (!arm.sparse && !arm.tiny) {
            return Err("unexpected target DAG readiness".into());
        }
        row.after = Some(inv);
        row.error_stage = "cleanup".into();
        fs::remove_dir_all(&work)?;
        File::open(root.join("working"))?.sync_all()?;
        row.status = "exact".into();
        row.error_stage.clear();
        Ok(())
    })();
    if let Err(e) = result {
        row.error = Some(e.to_string());
    }
    row
}
pub fn run_block(root: &Path, out: &Path, variant: &str, block: usize) -> Result<()> {
    if block >= 2 || variant != "diagnostic" {
        return Err("only diagnostic blocks 0 and 1 registered".into());
    }
    owned(root)?;
    let mut log = output(out)?;
    let cpu = ProcessTime::now();
    let wall = Instant::now();
    let (base, target) = fixture(42);
    let generation = elapsed(cpu, wall, 0);
    use sha2::{Digest, Sha256};
    let exe = std::env::current_exe()?;
    let exe_hash = hex(&Sha256::digest(fs::read(exe)?).into());
    append(
        &mut log,
        &serde_json::json!({"kind":"metadata","schema":"spiralcore.cache-scan-probe.v1","library_revision":SOURCE_REVISION,"library_source_sha256":SOURCE_SHA256,"executable_sha256":exe_hash,"variant":variant,"block":block,"fixture_generator":"xorshift64-low-byte-per-step-v1","fixture_seed":42,"distinct_from_wan_fixture":true,"fixture_generation":generation,"policies":{"normal_nodes":262144,"tiny_nodes":1,"graph_cache_entries":0,"graph_cache_bytes":0},"timing":"local_operation_total.cpu_ns includes preparation, instrumented handlers, profile reset/take and checks, validated Published and request-buffer teardown. Handler cost includes snapshot scan instrumentation; disk_usage clocks are a subset, never summed again. Profile reset and take are outside handler clocks. No per-entry clock, output or allocation is added. Counts are Rust API call sites, not OS syscall counts.","boundaries":"private template clone and every file/directory sync precede open/timing; fixture generation, clone/sync, Endpoint open, and exact reopened comparison are separate. OS page caches uncontrolled; no network or energy. Tiny node budget changes graph admission/reservation and readiness; not a pure warming toggle. Sparse always skips graph warming; both policies disable graph retention. Instrumentation changes code generation and adds counter/timer overhead: this is attribution only, not a production speed comparison.","ordering":"Two complete blocks 0 then 1; population order and four-arm order rotate by block as in the prior helper. All 32 observations retained; no retry or filtering."}),
    )?;
    for i in 0..4 {
        let n = POPULATIONS[(i + block) % 4];
        for arm in ordered_arms(block, n) {
            let row = observe(root, variant, block, n, arm, &base, &target);
            append(&mut log, &row)?;
            if row.status != "exact" {
                return Err(format!(
                    "failed observation retained at {}: {:?}",
                    out.display(),
                    row.error
                )
                .into());
            }
        }
    }
    File::open(out.parent().ok_or("output parent")?)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Seek, SeekFrom},
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    #[test]
    fn profile_validation_rejects_extra_calls_and_wrong_inventory() {
        let inv = Inventory {
            namespaces: 1,
            publications: 1,
            publication_bytes: 100,
            active_publications: 1,
            active_bytes: 100,
            manifests: 1,
        };
        let valid = ScanProfile {
            calls: 1,
            successes: 1,
            root_read_dir_calls: 1,
            root_entries: 3,
            root_file_type_calls: 3,
            namespace_read_dir_calls: 1,
            namespace_entries: 1,
            object_file_type_calls: 1,
            metadata_calls: 1,
            metadata_successes: 1,
            namespace_comparisons: 1,
            objects_counted: 1,
            returned_scoped_bytes: 100,
            returned_all_bytes: 100,
            returned_objects: 1,
            ..Default::default()
        };
        assert!(check_scan_profile(1, valid, &inv).is_ok());
        assert!(check_scan_profile(3, valid, &inv).is_ok());
        assert!(check_scan_profile(2, ScanProfile::default(), &inv).is_ok());
        assert!(check_scan_profile(2, valid, &inv).is_err());
        assert!(check_scan_profile(1, ScanProfile { calls: 2, ..valid }, &inv).is_err());
        assert!(check_scan_profile(
            3,
            ScanProfile {
                returned_all_bytes: 99,
                ..valid
            },
            &inv
        )
        .is_err());
        assert!(check_scan_profile(
            1,
            ScanProfile {
                last_error: Some(9),
                ..valid
            },
            &inv
        )
        .is_err());
        assert!(check_scan_profile(
            1,
            ScanProfile {
                overflow: true,
                ..valid
            },
            &inv
        )
        .is_err());
    }
    #[test]
    fn ready_rejects_wrong_mode_pin_sequence_and_trailing_body() {
        let (_, target) = fixture(42);
        let ff = frames(&[], &target, false, [4; 32]).unwrap();
        let f = &ff[0];
        let mut ready = vec![0, 0];
        ready.extend_from_slice(&1u64.to_le_bytes());
        assert!(check_reply(
            f,
            &f.reply(101, ready.clone()),
            object_id(&target),
            target.len(),
            &mut 0
        )
        .is_ok());
        for bad in [
            vec![],
            vec![0; 9],
            {
                let mut x = ready.clone();
                x.push(0);
                x
            },
            {
                let mut x = ready.clone();
                x[0] = 3;
                x
            },
            {
                let mut x = ready.clone();
                x[1] = 1;
                x
            },
            {
                let mut x = ready.clone();
                x[2] = 2;
                x
            },
        ] {
            assert!(check_reply(
                f,
                &f.reply(101, bad),
                object_id(&target),
                target.len(),
                &mut 0
            )
            .is_err());
        }
    }
    #[test]
    fn accepted_rejects_wrong_expanded_count_and_trailing_body() {
        let (_, target) = fixture(42);
        let ff = frames(&[], &target, false, [4; 32]).unwrap();
        let f = &ff[1];
        for bad in [vec![], vec![0; 8], vec![0; 9], 1u64.to_le_bytes().to_vec()] {
            let mut count = 0;
            assert!(check_reply(
                f,
                &f.reply(102, bad),
                object_id(&target),
                target.len(),
                &mut count
            )
            .is_err());
            assert_eq!(count, 0);
        }
        let mut accepted = 0;
        let body = (MAX_CHUNK as u64).to_le_bytes().to_vec();
        check_reply(
            f,
            &f.reply(102, body.clone()),
            object_id(&target),
            target.len(),
            &mut accepted,
        )
        .unwrap();
        assert_eq!(accepted, MAX_CHUNK as u64);
        assert!(check_reply(
            &ff[2],
            &ff[2].reply(102, body),
            object_id(&target),
            target.len(),
            &mut accepted
        )
        .is_err());
        assert_eq!(accepted, MAX_CHUNK as u64);
    }
    struct Temp(PathBuf);
    impl Temp {
        fn new(base: &[u8]) -> Self {
            let root = std::env::temp_dir().join(format!(
                "spiral-receiver-probe-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            mkdir(&root).unwrap();
            let mut marker = output(&root.join(".probe-owned")).unwrap();
            marker.write_all(OWNED).unwrap();
            marker.sync_all().unwrap();
            mkdir(&root.join("templates")).unwrap();
            mkdir(&root.join("working")).unwrap();
            let mut ep = Endpoint::open(
                &root.join("templates/0"),
                Arm {
                    sparse: false,
                    tiny: false,
                }
                .limits(),
                Policy,
            )
            .unwrap();
            send(&mut ep, &context("active"), base, [8; 32], 0).unwrap();
            drop(ep);
            Self(root)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn fixture_changes_exactly_32_bytes_and_is_reproducible() {
        let (base, target) = fixture(42);
        assert_eq!(base.len(), 1 << 20);
        assert_eq!(target.len(), base.len());
        assert_eq!(base.iter().zip(&target).filter(|(a, b)| a != b).count(), 32);
        assert_eq!(fixture(42), (base, target));
        assert_ne!(fixture(43).0, fixture(42).0);
    }
    #[test]
    fn stage_accounting_is_exact_and_rejects_unknown_or_overflow() {
        let mut s = Stages::default();
        for (kind, wall, cpu) in [(1, 9, 3), (2, 10, 4), (2, 13, 7), (3, 19, 5)] {
            s.add(
                kind,
                Cost {
                    wall_ns: wall,
                    cpu_ns: cpu,
                    frames: 1,
                },
            )
            .unwrap();
        }
        assert_eq!(
            (
                s.begin.wall_ns,
                s.data.wall_ns,
                s.finish.wall_ns,
                s.total.wall_ns
            ),
            (9, 23, 19, 51)
        );
        assert_eq!((s.total.cpu_ns, s.total.frames), (19, 4));
        assert!(s.add(99, Cost::default()).is_err());
        let before = serde_json::to_string(&s).unwrap();
        assert!(s
            .add(
                2,
                Cost {
                    wall_ns: u64::MAX,
                    cpu_ns: 0,
                    frames: 1
                }
            )
            .is_err());
        assert_eq!(serde_json::to_string(&s).unwrap(), before);
    }
    #[test]
    fn request_and_order_fixtures_have_exact_expected_shapes() {
        let (base, target) = fixture(42);
        for sparse in [false, true] {
            let ff = frames(&base, &target, sparse, [2; 32]).unwrap();
            assert_eq!(ff.len(), if sparse { 3 } else { 20 });
            assert_eq!(ff[0].kind, 1);
            assert_eq!(ff.last().unwrap().kind, 3);
            for (i, f) in ff.iter().enumerate() {
                assert_eq!(f.seq, i as u64);
                assert_eq!(f.encode().unwrap().len(), f.body.len() + 56);
            }
            let offer = Offer::decode(&ff[0].body).unwrap();
            assert_eq!(offer.object, object_id(&target));
            assert_eq!(
                offer.base,
                if sparse { Some(object_id(&base)) } else { None }
            );
        }
        for n in POPULATIONS {
            let mut counts = std::collections::BTreeMap::new();
            for block in 0..8 {
                for (position, arm) in ordered_arms(block, n).iter().enumerate() {
                    *counts.entry((arm.label(), position)).or_insert(0) += 1;
                }
            }
            assert_eq!(counts.len(), 16);
            assert!(counts.values().all(|n| *n == 2));
        }
    }
    #[test]
    fn actual_api_receipts_match_stages_readiness_and_reopened_bytes() {
        let (base, target) = fixture(42);
        let temp = Temp::new(&base);
        for arm in ordered_arms(0, 0) {
            let r = observe(&temp.0, "baseline", 0, 0, arm, &base, &target);
            assert_eq!(r.status, "exact", "{:?}", r.error);
            assert!(r.error.is_none());
            assert_eq!(r.before.as_ref().unwrap().publications, 1);
            assert_eq!(r.after.as_ref().unwrap().publications, 2);
            assert_eq!(r.target_manifest_present, Some(!arm.sparse && !arm.tiny));
            assert_eq!(r.handler_stages.begin.frames, 1);
            assert_eq!(r.handler_stages.finish.frames, 1);
            assert_eq!(
                r.handler_stages.data.frames,
                if arm.sparse { 1 } else { 18 }
            );
            let mut aggregate = Stages::default();
            for f in &r.frame_samples {
                aggregate.add(f.kind, f.cost).unwrap();
                let p = f.disk_usage;
                if f.kind == 2 {
                    assert_eq!(p, ScanProfile::default());
                } else {
                    assert_eq!((p.calls, p.successes, p.errors), (1, 1, 0));
                    assert_eq!(
                        (p.root_entries, p.namespace_entries, p.metadata_calls),
                        (3, 1, 1)
                    );
                    let inv = r.before.as_ref().unwrap();
                    assert_eq!(
                        (
                            p.returned_scoped_bytes,
                            p.returned_all_bytes,
                            p.returned_objects
                        ),
                        (
                            inv.active_bytes,
                            inv.publication_bytes,
                            inv.publications as u64
                        )
                    );
                    assert!(p.wall_ns <= f.cost.wall_ns);
                }
            }
            assert_eq!(
                serde_json::to_string(&aggregate).unwrap(),
                serde_json::to_string(&r.handler_stages).unwrap()
            );
            assert!(r.local_operation_total.wall_ns >= r.handler_stages.total.wall_ns);
            assert_eq!(r.graph_cache_after.as_ref().unwrap()["entries"], 0);
            assert!(!r.work_cache.exists());
        }
    }
    #[test]
    fn corrupted_base_keeps_failed_handler_receipt_and_owned_cache() {
        let (base, target) = fixture(42);
        let temp = Temp::new(&base);
        let c = context("active");
        let path = temp
            .0
            .join("templates/0")
            .join(hex(&c.namespace().unwrap()))
            .join(format!("{}.object", hex(&object_id(&base))));
        let mut file = OpenOptions::new().write(true).open(path).unwrap();
        file.seek(SeekFrom::Start(81)).unwrap();
        file.write_all(&[base[0] ^ 0xff]).unwrap();
        file.sync_all().unwrap();
        let r = observe(
            &temp.0,
            "baseline",
            0,
            0,
            Arm {
                sparse: true,
                tiny: true,
            },
            &base,
            &target,
        );
        assert_eq!(r.status, "failed");
        assert_eq!(r.error_stage, "handler");
        assert!(r.error.as_ref().unwrap().contains("Integrity"));
        assert_eq!(r.handler_stages.begin.frames, 1);
        assert_eq!(r.handler_stages.total.frames, 1);
        assert_eq!(r.frame_samples[0].reply_kind, None);
        assert!(r.work_cache.is_dir());
        assert_eq!(
            inventory(&r.work_cache, c.namespace().unwrap())
                .unwrap()
                .publications,
            1
        );
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["status"], "failed");
        assert!(json["error"].is_string());
    }
}
