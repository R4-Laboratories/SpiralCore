//! Bounded public-fixture experiment; execution alone is not a performance claim.
use cpu_time::ProcessTime;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spiralcore_transfer::{
    client,
    dag::Graph,
    endpoint::Endpoint,
    hash, hex, object_id,
    prepared::PreparedObject,
    protocol::{Frame, Mode},
    wan, Error, Id,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const MIB: usize = 1 << 20;
static TRANSFER_NONCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Task {
    id: String,
    pair: String,
    trial: u32,
    case: String,
    size: usize,
    arm: String,
    scope: String,
    fixture_seed: u64,
    seed_base: bool,
    warm_probe: bool,
    cache_state: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    suite: Option<String>,
    run: String,
    trials: u32,
    fixture_version: String,
    retained_payload_bytes: u64,
    allowed_scopes: Vec<String>,
    tasks: Vec<Task>,
    limitations: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metrics {
    fixture_generation_wall_micros: u64,
    fixture_generation_cpu_micros: u64,
    wall_micros: u64,
    sender_cpu_micros: u64,
    base_prepare_wall_micros: u64,
    base_prepare_cpu_micros: u64,
    receiver_cpu_micros: u64,
    receiver_wall_micros: u64,
    request_body_bytes: u64,
    response_body_bytes: u64,
    application_request_bytes: Option<u64>,
    application_response_bytes: Option<u64>,
    data_bytes: Option<u64>,
    records_sent: Option<usize>,
    http_requests: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    preparation: Option<Preparation>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Preparation {
    retained_base_reused: bool,
    base_resident_charge_bytes: Option<u64>,
    target_prepare_wall_micros: u64,
    target_prepare_cpu_micros: u64,
    target_resident_charge_bytes: Option<u64>,
    promotion_performed: bool,
    selected_mode: Option<u8>,
}
impl Metrics {
    fn body_bytes(&self) -> u64 {
        self.request_body_bytes
            .saturating_add(self.response_body_bytes)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    suite: Option<String>,
    run: String,
    plan_trials: u32,
    task: Task,
    phase: String,
    object: String,
    object_bytes: usize,
    status: String,
    error: Option<String>,
    unix_start_nanos: String,
    unix_end_nanos: String,
    metrics: Metrics,
    source_revision: String,
    executable_sha256: String,
    os: String,
    architecture: String,
}
fn rng(x: &mut u64) -> u64 {
    *x ^= *x << 13;
    *x ^= *x >> 7;
    *x ^= *x << 17;
    *x
}
fn base_fixture(size: usize, seed: u64, structured: bool) -> Vec<u8> {
    let mut x = seed.max(1);
    let mut b = vec![0; size];
    for chunk in b.chunks_mut(8) {
        let value = rng(&mut x).to_le_bytes();
        chunk.copy_from_slice(&value[..chunk.len()]);
    }
    if structured {
        // Repeated public 4 KiB records with sixteen variants of an eight-byte field.
        let template = b[..4096.min(size)].to_vec();
        for (i, chunk) in b.chunks_mut(4096).enumerate() {
            chunk.copy_from_slice(&template[..chunk.len()]);
            if chunk.len() >= 8 {
                chunk[..8].copy_from_slice(&(i as u64 % 16).to_le_bytes());
            }
        }
    }
    b
}
fn fixture(task: &Task, phase: &str) -> Vec<u8> {
    let mut b = base_fixture(task.size, task.fixture_seed, task.case == "cold_structured");
    if phase == "seed" {
        return b;
    }
    match task.case.as_str() {
        "localized_32" => {
            for x in &mut b[task.size / 2..task.size / 2 + 32] {
                *x ^= 0xa5;
            }
        }
        "scattered_32x32" => {
            for i in 0..32 {
                let at = i * task.size / 32;
                for x in &mut b[at..at + 32] {
                    *x ^= 0xa5;
                }
            }
        }
        "prefix_1" => b.insert(0, 0x5a),
        _ => {}
    }
    b
}
fn make_plan(run: &str, trials: u32) -> Result<Plan> {
    let run_id = wan::parse_id(run)?;
    if !(1..=30).contains(&trials) {
        return Err("--trials must be between 1 and 30".into());
    }
    let mut tasks = Vec::new();
    let mut retained_payload_bytes = 0u64;
    for (size, count, cases) in [
        (
            MIB,
            trials,
            vec![
                "cold_random",
                "cold_structured",
                "repeat",
                "localized_32",
                "scattered_32x32",
                "prefix_1",
            ],
        ),
        (16 * MIB, 1, vec!["cold_random", "repeat"]),
    ] {
        for case in cases {
            for trial in 0..count {
                let pair = format!("{size}-{case}-{trial:02}");
                let seed_id = hash(
                    b"spiralcore.bench.fixture.v1\0",
                    &[&run_id, &(size as u64).to_le_bytes(), &trial.to_le_bytes()],
                );
                let fixture_seed = u64::from_le_bytes(seed_id[..8].try_into()?).max(1);
                let mut arms = vec!["raw_full", "zstd_full"];
                if case == "repeat" {
                    arms.push("raw_cas");
                }
                if size == MIB {
                    arms.push("dag");
                }
                // Three-arm blocks cover all six orders; four-arm blocks balance positions.
                let permutations: &[&[usize]] = match arms.len() {
                    2 => &[&[0, 1], &[1, 0]],
                    3 => &[
                        &[0, 1, 2],
                        &[0, 2, 1],
                        &[1, 0, 2],
                        &[1, 2, 0],
                        &[2, 0, 1],
                        &[2, 1, 0],
                    ],
                    _ => &[&[0, 1, 2, 3], &[1, 2, 3, 0], &[2, 3, 0, 1], &[3, 0, 1, 2]],
                };
                let offset = hash(
                    b"spiralcore.bench.order.v1\0",
                    &[&run_id, case.as_bytes(), &(size as u64).to_le_bytes()],
                )[0] as usize;
                for &i in permutations[(offset + trial as usize) % permutations.len()] {
                    let arm = arms[i];
                    let id = format!("{pair}-{arm}");
                    let scope = hex(&hash(
                        b"spiralcore.bench.scope.v1\0",
                        &[&run_id, id.as_bytes()],
                    ));
                    let mutation = matches!(case, "localized_32" | "scattered_32x32" | "prefix_1");
                    let seed_base =
                        mutation || (case == "repeat" && matches!(arm, "raw_cas" | "dag"));
                    retained_payload_bytes += (size + usize::from(case == "prefix_1")) as u64;
                    if seed_base && case != "repeat" {
                        retained_payload_bytes += size as u64;
                    }
                    tasks.push(Task {
                        id,
                        pair: pair.clone(),
                        trial,
                        case: case.into(),
                        size,
                        arm: arm.into(),
                        scope,
                        fixture_seed,
                        seed_base,
                        warm_probe: case == "repeat" && (arm == "dag" || arm == "raw_cas"),
                        cache_state: if !seed_base {
                            "target_absent"
                        } else if case == "repeat" {
                            "target_present"
                        } else {
                            "base_present_target_absent"
                        }
                        .into(),
                    });
                }
            }
        }
    }
    let publications = tasks
        .iter()
        .map(|t| 1 + usize::from(t.seed_base && t.case != "repeat"))
        .sum::<usize>();
    if retained_payload_bytes + publications as u64 * 4096 >= 1 << 30 {
        return Err(
            "plan exceeds cache budget including 4 KiB per-publication metadata allowance".into(),
        );
    }
    Ok(Plan {
        schema: "spiralcore.wan.plan.v1".into(), suite: None, run: hex(&run_id), trials, fixture_version: "xorshift64-v1".into(),
        retained_payload_bytes, allowed_scopes: tasks.iter().map(|t| t.scope.clone()).collect(), tasks,
        limitations: vec![
            "16 MiB random DAG exceeds the current node cap; only raw/zstd and raw-CAS smoke checks are planned".into(),
            "Application cache state is controlled; OS page/disk caches are not flushed".into(),
            "Repeat full-copy controls have empty target scopes; warm raw-CAS and DAG share the general object-cache shortcut".into(),
            "Localized edits change 32 bytes; scattered edits change 1024 bytes, not an equal edit budget".into(),
            "Receiver CPU covers decode/handle/encode; relay/cloudflared CPU and socket wait are excluded".into(),
            "HTTP body counts are not packet/link bytes; failed-upload counts are attempted body bytes".into(),
            "Transfer timing starts with generated in-memory inputs before graph/codec preparation; fixture generation is measured separately".into(),
            "Arms use seeded counterbalanced paired order, not an independent random shuffle".into(),
            "Begin cache hits are valid only for repeat raw-CAS/DAG targets; any hit during a seed, cold upload, or modified-target upload invalidates that operation".into(),
        ],
    })
}
fn selected_plan(run: &str, trials: u32, suite: Option<&str>) -> Result<Plan> {
    let Some(suite) = suite else {
        return make_plan(run, trials);
    };
    let cases: &[&str] = match suite {
        "efficient-warm" => &["repeat", "localized_32", "scattered_32x32"],
        "efficient-prefix" => &["prefix_1"],
        _ => return Err("--suite must be efficient-warm or efficient-prefix".into()),
    };
    let run_id = wan::parse_id(run)?;
    if !(1..=30).contains(&trials) {
        return Err("--trials must be between 1 and 30".into());
    }
    let mut tasks = Vec::new();
    let mut retained_payload_bytes = 0;
    for &case in cases {
        let mut arms = vec!["raw_full", "zstd_full"];
        if case == "repeat" {
            arms.push("raw_cas");
        }
        arms.push("dag");
        if case != "prefix_1" {
            arms.push("prepared");
        }
        arms.push("sparse");
        let offset = hash(
            b"spiralcore.bench.order.v2\0",
            &[&run_id, suite.as_bytes(), case.as_bytes()],
        )[0] as usize
            % arms.len();
        for trial in 0..trials {
            let pair = format!("{MIB}-{case}-{trial:02}");
            // Match v1 fixtures for a given run and trial; suite-specific scopes
            // isolate the experiment even if both suites use the same run ID.
            let seed_id = hash(
                b"spiralcore.bench.fixture.v1\0",
                &[&run_id, &(MIB as u64).to_le_bytes(), &trial.to_le_bytes()],
            );
            let fixture_seed = u64::from_le_bytes(seed_id[..8].try_into()?).max(1);
            // Cyclic counterbalancing gives every arm each position equally
            // often for 5/6-arm 30-trial blocks (within one for four arms).
            for position in 0..arms.len() {
                let arm = arms[(offset + trial as usize + position) % arms.len()];
                let id = format!("{pair}-{arm}");
                let scope = hex(&hash(
                    b"spiralcore.bench.scope.v2\0",
                    &[&run_id, suite.as_bytes(), id.as_bytes()],
                ));
                let warm_probe = case == "repeat" && !matches!(arm, "raw_full" | "zstd_full");
                let seed_base = case != "repeat" || warm_probe;
                retained_payload_bytes += (MIB + usize::from(case == "prefix_1")) as u64;
                if seed_base && case != "repeat" {
                    retained_payload_bytes += MIB as u64;
                }
                tasks.push(Task {
                    id,
                    pair: pair.clone(),
                    trial,
                    case: case.into(),
                    size: MIB,
                    arm: arm.into(),
                    scope,
                    fixture_seed,
                    seed_base,
                    warm_probe,
                    cache_state: if !seed_base {
                        "target_absent"
                    } else if case == "repeat" {
                        "target_present"
                    } else {
                        "base_present_target_absent"
                    }
                    .into(),
                });
            }
        }
    }
    let publications = tasks
        .iter()
        .map(|t| 1 + usize::from(t.seed_base && t.case != "repeat"))
        .sum::<usize>();
    if retained_payload_bytes + publications as u64 * 4096 >= 1 << 30 {
        return Err("efficiency suite exceeds the isolated 1 GiB cache budget".into());
    }
    Ok(Plan {
        schema: "spiralcore.wan.plan.v2".into(), suite: Some(suite.into()),
        run: hex(&run_id), trials, fixture_version: "xorshift64-v1".into(), retained_payload_bytes,
        allowed_scopes: tasks.iter().map(|t| t.scope.clone()).collect(), tasks,
        limitations: vec![
            "Each suite requires a fresh isolated receiver cache; suites are not combined under one cache quota".into(),
            "Application cache state is controlled; OS page/disk caches are not flushed".into(),
            "The DAG arm uses the current executable's full-fold path, not the historical pre-optimization binary".into(),
            "Prepared base construction is charged inside the seed timer; the same owned state survives into its immediately following transfer".into(),
            "Prepared target comparison/hash/closure work and terminal retained-state teardown are timed; this one-update experiment performs no successor promotion".into(),
            "Resident charges are conservative owned-state accounting, not process RSS or preparation peak memory; shared graphs may be counted twice".into(),
            "Sparse encoding, comparison and fallback selection are timed; sparse is an aligned/prefix/suffix recipe, not content-defined chunking".into(),
            "Repeat full-copy controls have empty target scopes; raw-CAS, DAG, prepared and sparse share the general object-cache shortcut".into(),
            "Localized edits change 32 bytes; scattered edits change 1024 bytes, not an equal edit budget".into(),
            "Transfer timing starts with generated in-memory inputs before graph/codec preparation; fixture generation is measured separately".into(),
            "Arms use seeded cyclic counterbalanced paired order, not an independent random shuffle or balanced carryover design".into(),
            "Begin cache hits are valid only for seeded repeat targets; hits during seeds or modified-target uploads invalidate those operations".into(),
            "Receiver CPU covers decode/handle/encode; relay/cloudflared CPU and socket wait are excluded; CPU is not an energy measurement".into(),
            "HTTP body counts are not packet/link bytes; failed-upload counts are attempted body bytes".into(),
        ],
    })
}
fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut b = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut b)?;
    if b.len() as u64 > limit {
        return Err("input exceeds bounded file size".into());
    }
    Ok(b)
}
fn read_plan(path: &Path) -> Result<Plan> {
    let p: Plan = serde_json::from_slice(&read_limited(path, 2 << 20)?)?;
    if p != selected_plan(&p.run, p.trials, p.suite.as_deref())? {
        return Err("plan differs from canonical generated plan".into());
    }
    Ok(p)
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
    serde_json::to_writer_pretty(&mut f, value)?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    Ok(())
}
fn micros(x: u128) -> u64 {
    x.min(u64::MAX as u128) as u64
}
fn unix_nanos() -> Result<String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string())
}
fn fresh_transfer_id(scope: Id, phase: &str, unix_start_nanos: &str) -> Id {
    hash(
        b"spiralcore.bench.transfer.v2\0",
        &[
            &scope,
            phase.as_bytes(),
            unix_start_nanos.as_bytes(),
            &std::process::id().to_le_bytes(),
            &TRANSFER_NONCE.fetch_add(1, Ordering::Relaxed).to_le_bytes(),
        ],
    )
}
fn unexpected_begin_cache_hit(task: &Task, phase: &str, sent: &[Frame], replies: &[Frame]) -> bool {
    let allowed = phase == "transfer"
        && task.case == "repeat"
        && matches!(task.arm.as_str(), "raw_cas" | "dag" | "prepared" | "sparse");
    !allowed
        && sent
            .iter()
            .zip(replies)
            .any(|(request, reply)| request.kind == 1 && reply.kind == 103)
}
fn run_phase(
    plan: &Plan,
    task: &Task,
    phase: &str,
    url: &str,
    token: &str,
    binary: &str,
    revision: &str,
) -> Result<Row> {
    let fixture_wall = Instant::now();
    let fixture_cpu = ProcessTime::now();
    let bytes = fixture(task, phase);
    let base_bytes =
        (phase == "transfer" && task.seed_base && task.arm == "dag").then(|| fixture(task, "seed"));
    let fixture_generation_cpu_micros = micros(fixture_cpu.elapsed().as_micros());
    let fixture_generation_wall_micros = micros(fixture_wall.elapsed().as_micros());
    let scope = wan::parse_id(&task.scope)?;
    let unix_start_nanos = unix_nanos()?;
    let transfer_id = fresh_transfer_id(scope, phase, &unix_start_nanos);
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let mut http = wan::HttpClient::new(url, token, scope)?;
    let base_wall = Instant::now();
    let base_cpu = ProcessTime::now();
    let base = base_bytes
        .as_ref()
        .map(|b| Graph::fold(b, 262144))
        .transpose();
    let base_prepare_cpu_micros = micros(base_cpu.elapsed().as_micros());
    let base_prepare_wall_micros = micros(base_wall.elapsed().as_micros());
    let mode = if phase == "seed" {
        Mode::Raw
    } else {
        match task.arm.as_str() {
            "dag" => Mode::Dag,
            "zstd_full" => Mode::Zstd,
            _ => Mode::Raw,
        }
    };
    let mut unexpected_cache_hit = false;
    let outcome = base.and_then(|g| {
        client::transfer_batched(
            &bytes,
            mode,
            g.as_ref(),
            transfer_id,
            phase == "transfer" && task.warm_probe,
            |frames| {
                let replies = http.exchange(frames)?;
                if unexpected_begin_cache_hit(task, phase, frames, &replies) {
                    unexpected_cache_hit = true;
                    return Err(Error::Integrity);
                }
                Ok(replies)
            },
        )
    });
    let sender_cpu_micros = micros(cpu.elapsed().as_micros());
    let wall_micros = micros(wall.elapsed().as_micros());
    let unix_end_nanos = unix_nanos()?;
    let m = http.metrics();
    let stats = outcome.as_ref().ok();
    let metrics = Metrics {
        fixture_generation_wall_micros,
        fixture_generation_cpu_micros,
        wall_micros,
        sender_cpu_micros,
        base_prepare_wall_micros,
        base_prepare_cpu_micros,
        receiver_cpu_micros: m.receiver_cpu_micros,
        receiver_wall_micros: m.receiver_wall_micros,
        request_body_bytes: m.request_body_bytes,
        response_body_bytes: m.response_body_bytes,
        application_request_bytes: stats.map(|s| s.request_bytes),
        application_response_bytes: stats.map(|s| s.response_bytes),
        data_bytes: stats.map(|s| s.data_bytes),
        records_sent: stats.map(|s| s.records_sent),
        http_requests: m.http_requests,
        preparation: None,
    };
    Ok(Row {
        schema: "spiralcore.wan.row.v1".into(),
        suite: None,
        run: plan.run.clone(),
        plan_trials: plan.trials,
        task: task.clone(),
        phase: phase.into(),
        object: hex(&object_id(&bytes)),
        object_bytes: bytes.len(),
        status: if outcome.is_ok() {
            "published"
        } else {
            "failed"
        }
        .into(),
        error: outcome.err().map(|e| {
            if unexpected_cache_hit {
                "unexpected_begin_cache_hit".into()
            } else {
                e.to_string()
            }
        }),
        unix_start_nanos,
        unix_end_nanos,
        metrics,
        source_revision: revision.into(),
        executable_sha256: binary.into(),
        os: env::consts::OS.into(),
        architecture: env::consts::ARCH.into(),
    })
}
#[derive(Default)]
struct Retained {
    prepared: Option<PreparedObject>,
    sparse: Option<Vec<u8>>,
}
struct Execution<'a> {
    url: &'a str,
    token: &'a str,
    binary: &'a str,
    revision: &'a str,
}
fn run_efficiency_phase(
    plan: &Plan,
    task: &Task,
    phase: &str,
    execution: &Execution<'_>,
    retained: &mut Retained,
) -> Result<Row> {
    let fixture_wall = Instant::now();
    let fixture_cpu = ProcessTime::now();
    let bytes = fixture(task, phase);
    let base_bytes =
        (phase == "transfer" && task.seed_base && task.arm == "dag").then(|| fixture(task, "seed"));
    let fixture_generation_cpu_micros = micros(fixture_cpu.elapsed().as_micros());
    let fixture_generation_wall_micros = micros(fixture_wall.elapsed().as_micros());
    let scope = wan::parse_id(&task.scope)?;
    let unix_start_nanos = unix_nanos()?;
    let transfer_id = fresh_transfer_id(scope, phase, &unix_start_nanos);
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let mut http = wan::HttpClient::new(execution.url, execution.token, scope)?;
    let mut preparation = Preparation::default();
    let mut base_prepare_wall_micros = 0;
    let mut base_prepare_cpu_micros = 0;
    let mut unexpected_cache_hit = false;
    let outcome = (|| -> spiralcore_transfer::Result<client::Stats> {
        if phase == "seed" && task.arm == "prepared" {
            let start = Instant::now();
            let cpu = ProcessTime::now();
            let prepared = PreparedObject::new(&bytes, 262144);
            base_prepare_cpu_micros = micros(cpu.elapsed().as_micros());
            base_prepare_wall_micros = micros(start.elapsed().as_micros());
            let prepared = prepared?;
            preparation.base_resident_charge_bytes = Some(prepared.resident_charge());
            retained.prepared = Some(prepared);
        }
        let graph = if let Some(base) = &base_bytes {
            let start = Instant::now();
            let cpu = ProcessTime::now();
            let graph = Graph::fold(base, 262144);
            base_prepare_cpu_micros = micros(cpu.elapsed().as_micros());
            base_prepare_wall_micros = micros(start.elapsed().as_micros());
            Some(graph?)
        } else {
            None
        };
        let exchange = |frames: &[Frame]| {
            let replies = http.exchange(frames)?;
            if unexpected_begin_cache_hit(task, phase, frames, &replies) {
                unexpected_cache_hit = true;
                return Err(Error::Integrity);
            }
            Ok(replies)
        };
        if phase == "transfer" && task.arm == "prepared" {
            let base = retained.prepared.as_ref().ok_or(Error::Missing)?;
            preparation.retained_base_reused = true;
            preparation.base_resident_charge_bytes = Some(base.resident_charge());
            let start = Instant::now();
            let cpu = ProcessTime::now();
            let delta = base.prepare_delta(&bytes);
            preparation.target_prepare_cpu_micros = micros(cpu.elapsed().as_micros());
            preparation.target_prepare_wall_micros = micros(start.elapsed().as_micros());
            let delta = delta?;
            preparation.target_resident_charge_bytes = Some(delta.resident_charge());
            // This experiment ends after one durable update; no successor graph
            // is needed. Delta destruction remains inside the transfer timer.
            client::transfer_prepared_batched(&delta, transfer_id, task.warm_probe, exchange)
        } else if phase == "transfer" && task.arm == "sparse" {
            let base = retained.sparse.as_deref().ok_or(Error::Missing)?;
            preparation.retained_base_reused = true;
            preparation.base_resident_charge_bytes = Some(base.len() as u64);
            client::transfer_sparse_batched(base, &bytes, transfer_id, task.warm_probe, exchange)
        } else {
            if phase == "seed" && task.arm == "sparse" {
                // Move this already-generated buffer into per-task ownership
                // after producing the receipt; no copy or hidden index build.
                preparation.base_resident_charge_bytes = Some(bytes.len() as u64);
            }
            let mode = match (phase, task.arm.as_str()) {
                ("transfer", "dag") => Mode::Dag,
                ("transfer", "zstd_full") => Mode::Zstd,
                _ => Mode::Raw,
            };
            client::transfer_batched(
                &bytes,
                mode,
                graph.as_ref(),
                transfer_id,
                phase == "transfer" && task.warm_probe,
                exchange,
            )
        }
    })();
    if phase == "transfer" {
        // No successor is retained by this one-update experiment. Charge the
        // terminal cleanup rather than moving its CPU outside all receipts.
        drop(std::mem::take(retained));
    }
    let m = http.metrics();
    drop(http);
    let sender_cpu_micros = micros(cpu.elapsed().as_micros());
    let wall_micros = micros(wall.elapsed().as_micros());
    let unix_end_nanos = unix_nanos()?;
    let stats = outcome.as_ref().ok();
    preparation.selected_mode = stats.map(|s| s.mode as u8);
    let row = Row {
        schema: "spiralcore.wan.row.v2".into(),
        suite: plan.suite.clone(),
        run: plan.run.clone(),
        plan_trials: plan.trials,
        task: task.clone(),
        phase: phase.into(),
        object: hex(&object_id(&bytes)),
        object_bytes: bytes.len(),
        status: if outcome.is_ok() {
            "published"
        } else {
            "failed"
        }
        .into(),
        error: outcome.as_ref().err().map(|e| {
            if unexpected_cache_hit {
                "unexpected_begin_cache_hit".into()
            } else {
                e.to_string()
            }
        }),
        unix_start_nanos,
        unix_end_nanos,
        metrics: Metrics {
            fixture_generation_wall_micros,
            fixture_generation_cpu_micros,
            wall_micros,
            sender_cpu_micros,
            base_prepare_wall_micros,
            base_prepare_cpu_micros,
            receiver_cpu_micros: m.receiver_cpu_micros,
            receiver_wall_micros: m.receiver_wall_micros,
            request_body_bytes: m.request_body_bytes,
            response_body_bytes: m.response_body_bytes,
            application_request_bytes: stats.map(|s| s.request_bytes),
            application_response_bytes: stats.map(|s| s.response_bytes),
            data_bytes: stats.map(|s| s.data_bytes),
            records_sent: stats.map(|s| s.records_sent),
            http_requests: m.http_requests,
            preparation: Some(preparation),
        },
        source_revision: execution.revision.into(),
        executable_sha256: execution.binary.into(),
        os: env::consts::OS.into(),
        architecture: env::consts::ARCH.into(),
    };
    if phase == "seed" && task.arm == "sparse" && row.status == "published" {
        retained.sparse = Some(bytes);
    }
    Ok(row)
}
fn run(plan: &Plan, url: &str, output: &Path) -> Result<()> {
    let token = env::var("SPIRALCORE_WAN_TOKEN").map_err(|_| "SPIRALCORE_WAN_TOKEN is required")?;
    let binary = hex(&Sha256::digest(read_limited(&env::current_exe()?, 256 << 20)?).into());
    let revision = env::var("SPIRALCORE_SOURCE_REV").unwrap_or_else(|_| "unrecorded".into());
    wan::HttpClient::new(url, &token, wan::parse_id(&plan.allowed_scopes[0])?)?;
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    let mut failed = 0;
    let execution = Execution {
        url,
        token: &token,
        binary: &binary,
        revision: &revision,
    };
    for task in &plan.tasks {
        // State belongs to one arm/trial only. Seed and transfer are adjacent,
        // and all sender state is dropped before the next arm starts.
        let mut retained = Retained::default();
        let phases: &[&str] = if task.seed_base {
            &["seed", "transfer"]
        } else {
            &["transfer"]
        };
        for phase in phases {
            let row = if plan.suite.is_some() {
                run_efficiency_phase(plan, task, phase, &execution, &mut retained)?
            } else {
                run_phase(plan, task, phase, url, &token, &binary, &revision)?
            };
            if row.status != "published" {
                failed += 1;
            }
            serde_json::to_writer(&mut f, &row)?;
            f.write_all(b"\n")?;
            f.flush()?;
            eprintln!("{} {} {}", task.id, phase, row.status);
            // Missing target after failed seed is detected by verification, never treated as warm success.
            if row.status != "published" && *phase == "seed" {
                break;
            }
        }
    }
    f.sync_all()?;
    if failed != 0 {
        return Err(format!("{failed} failed transfers; receipts retained").into());
    }
    Ok(())
}
fn read_rows(path: &Path) -> Result<Vec<Row>> {
    let bytes = read_limited(path, 8 << 20)?;
    let mut rows = Vec::new();
    let mut keys = BTreeSet::new();
    for line in BufReader::new(bytes.as_slice()).lines() {
        let row: Row = serde_json::from_str(&line?)?;
        if !valid_row_profile(&row)
            || !matches!(row.phase.as_str(), "seed" | "transfer")
            || !matches!(row.status.as_str(), "published" | "failed")
            || (row.status == "published") != row.error.is_none()
        {
            return Err("invalid receipt schema, phase or outcome".into());
        }
        if !keys.insert((row.run.clone(), row.task.id.clone(), row.phase.clone())) {
            return Err("duplicate receipt".into());
        }
        rows.push(row);
    }
    if rows.is_empty() || rows.len() > 2048 {
        return Err("receipt count outside 1..=2048".into());
    }
    Ok(rows)
}
fn valid_row_profile(row: &Row) -> bool {
    match (&row.suite, &row.metrics.preparation) {
        (None, None) => row.schema == "spiralcore.wan.row.v1",
        (Some(suite), Some(p)) => {
            row.schema == "spiralcore.wan.row.v2"
                && matches!(suite.as_str(), "efficient-warm" | "efficient-prefix")
                && !p.promotion_performed
                // A failed operation may stop before reaching retained state.
                && (row.status != "published" || p.retained_base_reused ==
                    (row.phase == "transfer" && matches!(row.task.arm.as_str(), "prepared" | "sparse")))
                && (row.status != "published" || p.selected_mode.is_some_and(|mode| match (row.phase.as_str(), row.task.arm.as_str(), row.task.case.as_str()) {
                    ("seed", _, _) | (_, "raw_full" | "raw_cas", _) | (_, "prepared", "repeat") => mode == 0,
                    (_, "zstd_full", _) => mode == 1,
                    (_, "dag" | "prepared", _) => mode == 2,
                    (_, "sparse", "repeat") => mode == 0,
                    (_, "sparse", _) => mode == 0 || mode == 3,
                    _ => false,
                }))
        }
        _ => false,
    }
}
fn verify(plan: &Plan, rows: &[Row], cache: &Path) -> Result<Value> {
    if receipt_plan(rows)? != *plan {
        return Err("receipts describe a different plan".into());
    }
    if !cache.is_dir() {
        return Err("existing cache directory required".into());
    }
    let tasks: BTreeMap<_, _> = plan.tasks.iter().map(|t| (t.id.as_str(), t)).collect();
    let expected: usize = plan
        .tasks
        .iter()
        .map(|t| 1 + usize::from(t.seed_base))
        .sum();
    if rows.len() != expected {
        return Err(format!("expected {expected} receipts, found {}", rows.len()).into());
    }
    let mut endpoint = Endpoint::open(cache, wan::limits(), wan::BenchAuthority)?;
    let mut verified = 0u64;
    let mut bytes_verified = 0u64;
    for row in rows {
        let Some(task) = tasks.get(row.task.id.as_str()) else {
            return Err("unknown receipt task".into());
        };
        if row.run != plan.run
            || row.task != **task
            || (row.phase == "seed" && !task.seed_base)
            || row.status != "published"
        {
            return Err("receipt does not match successful planned operation".into());
        }
        let expected = fixture(task, &row.phase);
        let id = object_id(&expected);
        if row.object != hex(&id) || row.object_bytes != expected.len() {
            return Err("receipt object differs from fixture".into());
        }
        if endpoint.read_object(&wan::context(wan::parse_id(&task.scope)?), id)? != expected {
            return Err("persisted reconstruction differs from fixture".into());
        }
        verified += 1;
        bytes_verified += expected.len() as u64;
    }
    let mut report = json!({"schema":"spiralcore.wan.verification.v1","run":plan.run,"status":"exact","receipts_verified":verified,"bytes_compared":bytes_verified,"comparison":"byte-for-byte after reopening receiver cache","qualification_outside_transfer_timing":true});
    if let Some(suite) = &plan.suite {
        report["schema"] = "spiralcore.wan.verification.v2".into();
        report["suite"] = suite.clone().into();
    }
    Ok(report)
}
fn percentile(v: &[f64], n: usize, d: usize) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    let mut v = v.to_vec();
    v.sort_by(f64::total_cmp);
    Some(v[(v.len() * n).div_ceil(d).max(1) - 1])
}
fn distribution(v: &[f64]) -> Value {
    json!({"n":v.len(),"median":percentile(v,1,2),"p95":percentile(v,95,100),"mean":if v.is_empty(){None}else{Some(v.iter().sum::<f64>()/v.len() as f64)}})
}
fn ratios(pairs: &[(f64, f64)]) -> Value {
    let r: Vec<_> = pairs
        .iter()
        .filter(|(_, c)| *c > 0.0)
        .map(|(b, c)| b / c)
        .collect();
    json!({"n":r.len(),"baseline_over_candidate":distribution(&r),"geometric_mean":if r.is_empty()||r.iter().any(|v|*v<=0.0){None}else{Some((r.iter().map(|v|v.ln()).sum::<f64>()/r.len() as f64).exp())},"interpretation":"greater than one favors candidate; paired operations only"})
}
fn summarize(rows: &[Row]) -> Result<Value> {
    let plan = receipt_plan(rows)?;
    let run = &rows[0].run;
    if rows.iter().any(|r| &r.run != run) {
        return Err("cannot combine different runs".into());
    }
    let mut groups: BTreeMap<(usize, &str, &str), Vec<&Row>> = BTreeMap::new();
    let seeds: BTreeMap<_, _> = rows
        .iter()
        .filter(|r| r.phase == "seed")
        .map(|r| (r.task.id.as_str(), r))
        .collect();
    let targets: BTreeMap<_, _> = rows
        .iter()
        .filter(|r| r.phase == "transfer")
        .map(|r| ((r.task.pair.as_str(), r.task.arm.as_str()), r))
        .collect();
    for r in rows.iter().filter(|r| r.phase == "transfer") {
        groups
            .entry((r.task.size, &r.task.case, &r.task.arm))
            .or_default()
            .push(r);
    }
    let mut summaries = Vec::new();
    for ((size, case, arm), group) in groups {
        let ok: Vec<_> = group
            .iter()
            .copied()
            .filter(|r| r.status == "published")
            .collect();
        let metric =
            |f: fn(&Metrics) -> u64| ok.iter().map(|r| f(&r.metrics) as f64).collect::<Vec<_>>();
        let cumulative = |f: fn(&Metrics) -> u64| {
            ok.iter()
                .map(|r| {
                    f(&r.metrics)
                        .saturating_add(seeds.get(r.task.id.as_str()).map_or(0, |s| f(&s.metrics)))
                        as f64
                })
                .collect::<Vec<_>>()
        };
        let mut wall = Vec::new();
        let mut body = Vec::new();
        let mut total = Vec::new();
        let mut break_even = Vec::new();
        for r in &ok {
            let Some(b) = targets.get(&(r.task.pair.as_str(), "raw_full")) else {
                continue;
            };
            if b.status != "published" {
                continue;
            }
            wall.push((b.metrics.wall_micros as f64, r.metrics.wall_micros as f64));
            body.push((b.metrics.body_bytes() as f64, r.metrics.body_bytes() as f64));
            let seed = seeds
                .get(r.task.id.as_str())
                .map_or(0, |s| s.metrics.body_bytes());
            let bseed = seeds
                .get(b.task.id.as_str())
                .map_or(0, |s| s.metrics.body_bytes());
            total.push((
                b.metrics.body_bytes().saturating_add(bseed) as f64,
                r.metrics.body_bytes().saturating_add(seed) as f64,
            ));
            if b.metrics.body_bytes() > r.metrics.body_bytes() {
                break_even.push(
                    (seed.saturating_sub(bseed) as f64
                        / (b.metrics.body_bytes() - r.metrics.body_bytes()) as f64)
                        .ceil(),
                );
            }
        }
        let mut summary = json!({"size":size,"case":case,"arm":arm,"attempted":group.len(),"published":ok.len(),"failed":group.len()-ok.len(),
            "transfer_wall_micros":distribution(&metric(|m|m.wall_micros)),"sender_cpu_micros":distribution(&metric(|m|m.sender_cpu_micros)),"receiver_cpu_micros":distribution(&metric(|m|m.receiver_cpu_micros)),
            "http_body_bytes_both_directions":distribution(&metric(Metrics::body_bytes)),
            "seed_plus_transfer_wall_micros":distribution(&cumulative(|m|m.wall_micros)),"seed_plus_transfer_sender_cpu_micros":distribution(&cumulative(|m|m.sender_cpu_micros)),"seed_plus_transfer_receiver_cpu_micros":distribution(&cumulative(|m|m.receiver_cpu_micros)),"seed_plus_transfer_http_body_bytes":distribution(&cumulative(Metrics::body_bytes)),
            "paired_raw_over_candidate_wall":ratios(&wall),"paired_raw_over_candidate_body_bytes":ratios(&body),"paired_seed_plus_transfer_body_bytes":ratios(&total),"additional_seed_cost_break_even_repetitions":distribution(&break_even)});
        if plan.suite.is_some() {
            summary["base_prepare_cpu_micros"] =
                distribution(&metric(|m| m.base_prepare_cpu_micros));
            summary["base_prepare_wall_micros"] =
                distribution(&metric(|m| m.base_prepare_wall_micros));
            let prep_metric = |f: fn(&Preparation) -> u64| {
                ok.iter()
                    .filter_map(|r| r.metrics.preparation.as_ref().map(|p| f(p) as f64))
                    .collect::<Vec<_>>()
            };
            summary["target_prepare_cpu_micros"] =
                distribution(&prep_metric(|p| p.target_prepare_cpu_micros));
            summary["target_prepare_wall_micros"] =
                distribution(&prep_metric(|p| p.target_prepare_wall_micros));
            for (name, baseline) in [
                ("raw", "raw_full"),
                ("zstd", "zstd_full"),
                ("raw_cas", "raw_cas"),
                ("dag", "dag"),
            ] {
                for (label, get_metric) in [
                    ("wall", (|m: &Metrics| m.wall_micros) as fn(&Metrics) -> u64),
                    ("sender_cpu", |m: &Metrics| m.sender_cpu_micros),
                    ("receiver_cpu", |m: &Metrics| m.receiver_cpu_micros),
                    ("body_bytes", Metrics::body_bytes),
                ] {
                    let pairs: Vec<_> = ok
                        .iter()
                        .filter_map(|r| {
                            let b = targets.get(&(r.task.pair.as_str(), baseline))?;
                            (b.status == "published").then_some((
                                get_metric(&b.metrics) as f64,
                                get_metric(&r.metrics) as f64,
                            ))
                        })
                        .collect();
                    summary[format!("paired_{name}_over_candidate_{label}")] = ratios(&pairs);
                    let pairs: Vec<_> = ok
                        .iter()
                        .filter_map(|r| {
                            let b = targets.get(&(r.task.pair.as_str(), baseline))?;
                            let cost = |row: &Row| {
                                get_metric(&row.metrics).saturating_add(
                                    seeds
                                        .get(row.task.id.as_str())
                                        .map_or(0, |s| get_metric(&s.metrics)),
                                ) as f64
                            };
                            (b.status == "published").then_some((cost(b), cost(r)))
                        })
                        .collect();
                    summary[format!("paired_seed_plus_transfer_{name}_over_candidate_{label}")] =
                        ratios(&pairs);
                }
            }
            summary["seed_base_prepare_cpu_micros"] = distribution(
                &ok.iter()
                    .filter_map(|r| {
                        seeds
                            .get(r.task.id.as_str())
                            .map(|s| s.metrics.base_prepare_cpu_micros as f64)
                    })
                    .collect::<Vec<_>>(),
            );
            summary["seed_base_prepare_wall_micros"] = distribution(
                &ok.iter()
                    .filter_map(|r| {
                        seeds
                            .get(r.task.id.as_str())
                            .map(|s| s.metrics.base_prepare_wall_micros as f64)
                    })
                    .collect::<Vec<_>>(),
            );
            let mut selected_modes: BTreeMap<u8, usize> = BTreeMap::new();
            for mode in ok
                .iter()
                .filter_map(|r| r.metrics.preparation.as_ref()?.selected_mode)
            {
                *selected_modes.entry(mode).or_default() += 1;
            }
            summary["selected_wire_mode_counts"] = serde_json::to_value(selected_modes)?;
            summary["base_resident_charge_bytes"] = distribution(
                &ok.iter()
                    .filter_map(|r| {
                        r.metrics
                            .preparation
                            .as_ref()?
                            .base_resident_charge_bytes
                            .map(|n| n as f64)
                    })
                    .collect::<Vec<_>>(),
            );
            summary["target_resident_charge_bytes"] = distribution(
                &ok.iter()
                    .filter_map(|r| {
                        r.metrics
                            .preparation
                            .as_ref()?
                            .target_resident_charge_bytes
                            .map(|n| n as f64)
                    })
                    .collect::<Vec<_>>(),
            );
            summary["promotion_performed"] = false.into();
        }
        summaries.push(summary);
    }
    let expected_rows: usize = plan
        .tasks
        .iter()
        .map(|t| 1 + usize::from(t.seed_base))
        .sum();
    let mut report = json!({"schema":"spiralcore.wan.summary.v1","run":run,"plan_trials":plan.trials,"rows":rows.len(),"expected_rows":expected_rows,"missing_rows":expected_rows.saturating_sub(rows.len()),"failed_rows":rows.iter().filter(|r|r.status!="published").count(),"groups":summaries,"limitations":[
        "Published receipts require separate local byte-for-byte qualification before exactness is claimed",
        "Body bytes exclude HTTP headers, TLS, TCP/IP and tunnel overhead; packet evidence is joined separately",
        "Failed rows contain attempted upload body bytes and completely received response bodies, not proven link bytes",
        "No WAN speed claim follows from running this summarizer",
        "Break-even extrapolates constant per-transfer savings; it is not a measured sequence",
        "16 MiB cases are one-trial smoke checks, not 30-trial performance qualification",
        "Receiver process CPU excludes relay/cloudflared and socket wait; sender CPU includes HTTP/TLS",
        "Failed pairs remain in receipts but are excluded from ratios; inspect failures before interpreting success-only statistics"]});
    if let Some(suite) = &plan.suite {
        report["schema"] = "spiralcore.wan.summary.v2".into();
        report["suite"] = suite.clone().into();
        report["quantiles"] =
            "nearest rank: sorted[ceil(n*p)-1], including lower middle for even-n median".into();
        let limitations = report["limitations"].as_array_mut().unwrap();
        limitations.retain(|v| !v.as_str().is_some_and(|s| s.starts_with("16 MiB")));
        limitations.extend(plan.limitations.iter().cloned().map(Value::String));
    }
    Ok(report)
}
fn receipt_plan(rows: &[Row]) -> Result<Plan> {
    let first = rows.first().ok_or("no receipts")?;
    let plan = selected_plan(&first.run, first.plan_trials, first.suite.as_deref())?;
    let tasks: BTreeMap<_, _> = plan.tasks.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut seen = BTreeSet::new();
    for row in rows {
        let task = tasks
            .get(row.task.id.as_str())
            .ok_or("receipt task is not in plan")?;
        if !valid_row_profile(row)
            || row.suite != plan.suite
            || row.run != plan.run
            || row.plan_trials != plan.trials
            || row.source_revision != first.source_revision
            || row.executable_sha256 != first.executable_sha256
            || row.os != first.os
            || row.architecture != first.architecture
            || row.task != **task
            || (row.phase == "seed" && !task.seed_base)
            || !matches!(row.phase.as_str(), "seed" | "transfer")
            || !seen.insert((row.task.id.as_str(), row.phase.as_str()))
        {
            return Err(
                "receipt metadata differs from canonical plan or repeats an operation".into(),
            );
        }
        let expected_len =
            task.size + usize::from(row.phase == "transfer" && task.case == "prefix_1");
        if row.object_bytes != expected_len {
            return Err("receipt fixture length differs from the plan".into());
        }
        wan::parse_id(&row.object)?;
        wan::parse_id(&row.executable_sha256)?;
        if row.unix_start_nanos.parse::<u128>()? > row.unix_end_nanos.parse::<u128>()?
            || row
                .metrics
                .request_body_bytes
                .checked_add(row.metrics.response_body_bytes)
                .is_none()
        {
            return Err("invalid receipt timing or byte count".into());
        }
    }
    for row in rows
        .iter()
        .filter(|r| r.phase == "transfer" && r.task.seed_base && r.status == "published")
    {
        if !rows
            .iter()
            .any(|s| s.task.id == row.task.id && s.phase == "seed" && s.status == "published")
        {
            return Err("successful warm transfer lacks a successful seed receipt".into());
        }
    }
    Ok(plan)
}
fn arguments() -> Result<(String, BTreeMap<String, String>)> {
    let mut args = env::args().skip(1);
    let command = args
        .next()
        .ok_or("expected plan, run, verify or summarize")?;
    let allowed: &[&str] = match command.as_str() {
        "plan" => &["--trials", "--run", "--out", "--suite"],
        "run" => &["--plan", "--url", "--out"],
        "verify" => &["--plan", "--rows", "--cache"],
        "summarize" => &["--rows", "--out"],
        _ => return Err("unknown command".into()),
    };
    let mut opts = BTreeMap::new();
    while let Some(key) = args.next() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unknown option {key}").into());
        }
        let value = args.next().ok_or("option requires value")?;
        if opts.insert(key, value).is_some() {
            return Err("duplicate option".into());
        }
    }
    Ok((command, opts))
}
fn execute() -> Result<()> {
    let (command, opts) = arguments()?;
    let get = |key: &str| {
        opts.get(key)
            .map(String::as_str)
            .ok_or_else(|| format!("missing {key}"))
    };
    match command.as_str() {
        "plan" => write_json(
            Path::new(get("--out")?),
            &selected_plan(
                get("--run")?,
                opts.get("--trials").map_or(Ok(30), |s| s.parse::<u32>())?,
                opts.get("--suite").map(String::as_str),
            )?,
        ),
        "run" => run(
            &read_plan(Path::new(get("--plan")?))?,
            get("--url")?,
            Path::new(get("--out")?),
        ),
        "verify" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&verify(
                    &read_plan(Path::new(get("--plan")?))?,
                    &read_rows(Path::new(get("--rows")?))?,
                    Path::new(get("--cache")?)
                )?)?
            );
            Ok(())
        }
        "summarize" => write_json(
            Path::new(get("--out")?),
            &summarize(&read_rows(Path::new(get("--rows")?))?)?,
        ),
        _ => unreachable!(),
    }
}
fn main() {
    if let Err(e) = execute() {
        eprintln!("benchmark: {e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transfer_identity_is_fresh_and_cache_hit_policy_covers_every_arm() {
        assert_ne!(
            fresh_transfer_id([1; 32], "transfer", "1"),
            fresh_transfer_id([1; 32], "transfer", "1")
        );
        let plans = [
            selected_plan(&"01".repeat(32), 1, None).unwrap(),
            selected_plan(&"01".repeat(32), 1, Some("efficient-warm")).unwrap(),
            selected_plan(&"01".repeat(32), 1, Some("efficient-prefix")).unwrap(),
        ];
        let begin = Frame {
            kind: 1,
            transfer: [2; 32],
            seq: 0,
            body: Vec::new(),
        };
        let hit = begin.reply(103, vec![0; 32]);
        for task in plans.iter().flat_map(|p| &p.tasks) {
            assert!(unexpected_begin_cache_hit(
                task,
                "seed",
                std::slice::from_ref(&begin),
                std::slice::from_ref(&hit)
            ));
            assert_eq!(
                unexpected_begin_cache_hit(
                    task,
                    "transfer",
                    std::slice::from_ref(&begin),
                    std::slice::from_ref(&hit)
                ),
                !task.warm_probe
            );
            assert!(!unexpected_begin_cache_hit(
                task,
                "transfer",
                std::slice::from_ref(&begin),
                &[begin.reply(101, Vec::new())]
            ));
        }
    }
    #[test]
    fn rerun_against_populated_receiver_is_a_retained_failure() {
        let p = make_plan(&"01".repeat(32), 1).unwrap();
        let task = p
            .tasks
            .iter()
            .find(|t| t.size == MIB && t.case == "cold_random" && t.arm == "raw_full")
            .unwrap();
        let root = env::temp_dir().join(format!(
            "spiral-bench-rerun-{}-{}",
            std::process::id(),
            unix_nanos().unwrap()
        ));
        let endpoint = Endpoint::open(&root, wan::limits(), wan::BenchAuthority).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let scope = wan::parse_id(&task.scope).unwrap();
        let token = "ab".repeat(32);
        let server_token = token.clone();
        let server = std::thread::spawn(move || {
            wan::serve(
                listener,
                endpoint,
                &server_token,
                &BTreeSet::from([scope]),
                std::time::Duration::from_secs(30),
                2,
            )
        });
        let first =
            run_phase(&p, task, "transfer", &url, &token, &"00".repeat(32), "test").unwrap();
        let second =
            run_phase(&p, task, "transfer", &url, &token, &"00".repeat(32), "test").unwrap();
        server.join().unwrap().unwrap();
        std::fs::remove_dir_all(root).unwrap();
        assert_eq!(first.status, "published");
        assert_eq!(second.status, "failed");
        assert_eq!(second.error.as_deref(), Some("unexpected_begin_cache_hit"));
        assert!(second.metrics.request_body_bytes > 0);
        let report = summarize(&[second]).unwrap();
        assert_eq!(report["failed_rows"], 1);
        assert_eq!(report["groups"][0]["published"], 0);
        assert_eq!(
            report["groups"][0]["paired_raw_over_candidate_wall"]["n"],
            0
        );
    }
    #[test]
    fn fixture_mutations_are_exact_and_reproducible() {
        assert_eq!(
            base_fixture(8, 1, false),
            [0x41, 0x20, 0x82, 0x40, 0, 0, 0, 0]
        );
        let p = make_plan(&"01".repeat(32), 1).unwrap();
        for t in p.tasks.iter().filter(|t| t.arm == "dag") {
            let b = fixture(t, "seed");
            let changed = fixture(t, "transfer");
            assert_eq!(changed, fixture(t, "transfer"));
            match t.case.as_str() {
                "localized_32" => {
                    assert_eq!(b.iter().zip(&changed).filter(|(a, b)| a != b).count(), 32)
                }
                "scattered_32x32" => {
                    assert_eq!(b.iter().zip(&changed).filter(|(a, b)| a != b).count(), 1024)
                }
                "prefix_1" => {
                    assert_eq!(changed.len(), b.len() + 1);
                    assert_eq!(changed[1..], b);
                }
                _ => assert_eq!(b, changed),
            }
        }
    }
    #[test]
    fn paired_ratios_are_not_ratios_of_unpaired_means() {
        let v = ratios(&[(100.0, 10.0), (2.0, 1.0)]);
        assert_eq!(v["n"], 2);
        assert_eq!(v["baseline_over_candidate"]["mean"], 6.0);
        assert!((v["geometric_mean"].as_f64().unwrap() - 20f64.sqrt()).abs() < 1e-10);
        assert_eq!(ratios(&[(1.0, 0.0)])["n"], 0);
    }
    fn row(plan: &Plan, task: &Task, phase: &str, body: u64) -> Row {
        Row {
            schema: "spiralcore.wan.row.v1".into(),
            suite: None,
            run: plan.run.clone(),
            plan_trials: plan.trials,
            task: task.clone(),
            phase: phase.into(),
            object: "00".repeat(32),
            object_bytes: task.size,
            status: "published".into(),
            error: None,
            unix_start_nanos: "1".into(),
            unix_end_nanos: "2".into(),
            metrics: Metrics {
                request_body_bytes: body,
                wall_micros: body,
                ..Metrics::default()
            },
            source_revision: "fixture".into(),
            executable_sha256: "00".repeat(32),
            os: "fixture".into(),
            architecture: "fixture".into(),
        }
    }
    #[test]
    fn summary_includes_seed_cost_and_rejects_missing_seed_or_wrong_pair() {
        let p = make_plan(&"01".repeat(32), 1).unwrap();
        let raw = p
            .tasks
            .iter()
            .find(|t| t.size == MIB && t.case == "repeat" && t.arm == "raw_full")
            .unwrap();
        let dag = p
            .tasks
            .iter()
            .find(|t| t.size == MIB && t.case == "repeat" && t.arm == "dag")
            .unwrap();
        let mut rows = vec![
            row(&p, raw, "transfer", 100),
            row(&p, dag, "seed", 10),
            row(&p, dag, "transfer", 20),
        ];
        let report = summarize(&rows).unwrap();
        let group = report["groups"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["arm"] == "dag")
            .unwrap();
        assert!(
            (group["paired_raw_over_candidate_body_bytes"]["geometric_mean"]
                .as_f64()
                .unwrap()
                - 5.0)
                .abs()
                < 1e-10
        );
        assert_eq!(group["seed_plus_transfer_http_body_bytes"]["mean"], 30.0);
        assert_eq!(
            group["additional_seed_cost_break_even_repetitions"]["mean"],
            1.0
        );
        assert_eq!(report["missing_rows"], 33);
        rows[2].task.pair = "unpaired".into();
        assert!(summarize(&rows).is_err());
        rows[2].task = dag.clone();
        rows.remove(1);
        assert!(summarize(&rows).is_err());
    }
}
