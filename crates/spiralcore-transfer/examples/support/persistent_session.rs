//! Once-fitted, acknowledged rolling sessions; historical harnesses stay separate.
use super::*;
use super::{
    schema_selection::{measure, Measured},
    schema_wan::{describe, observations, rows, METRICS_WAN},
    warm_object as warm,
};
use spiralcore_transfer::{
    endpoint::Context, schema_selection::PreparedSelector, session::WarmSession, wan::HttpClient,
    zstd_whole,
};
use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Read},
    net::TcpListener,
    process::{Command, Stdio},
    time::Duration,
};

const ARMS: [&str; 3] = ["fresh-selector", "persistent-selector", "whole"];
const MIXED_ARMS: [&str; 3] = ["baseline-selector", "candidate-selector", "whole"];
fn mixed(p: &Value) -> bool {
    p["experiment"] == "unrelated-cpu.v1"
}
fn baseline_pins(mut p: Value) -> Value {
    for (key, value) in [
        (
            "delta",
            "734caf02ac51a29d6181fc46e35baafd8de0002c26eddce15355b1ef78478ef7",
        ),
        (
            "schema",
            "034e0424cbf7b95a83e8579b7b7d75b584758f0eb514a00b51840a202c993359",
        ),
        (
            "cargo_manifest",
            "fdc1715045fc3d914ca04a4c41a4750e778ce3eef82ce391bcf82c553648739b",
        ),
        (
            "lockfile",
            "6cf795bc12285eea4083d64da4a3b7264d675a95b45d545751eec38708d87896",
        ),
    ] {
        p[key] = json!(value);
    }
    p
}
fn role_pins<'a>(p: &'a Value, arm: &str) -> &'a Value {
    if mixed(p) && arm == "baseline-selector" {
        &p["source_profiles"]["baseline"]
    } else {
        &p["sources"]
    }
}
fn prefix(p: &Value) -> &'static str {
    if mixed(p) {
        "unrelated"
    } else {
        "persistent"
    }
}
const ARTIFACT: &[u8] = include_bytes!(
    "../../../../docs/network/evidence/2026-10-30-schema-reuse/artifacts-corrected/schema.artifact"
);
const EXTERNAL: u64 = 64 << 20;
const SCRATCH: u64 = 128 << 20;
fn identity() -> Result<Value> {
    let mut id = warm::identity()?;
    id["sources"]["persistent_harness"] = json!(sha(include_bytes!("persistent_session.rs")));
    id["sources"]["session"] = json!(sha(include_bytes!("../../src/session.rs")));
    Ok(id)
}
fn pins() -> Value {
    let mut p = warm::pins();
    for (name, bytes) in [
        (
            "persistent_harness",
            include_bytes!("persistent_session.rs").as_slice(),
        ),
        ("session", include_bytes!("../../src/session.rs").as_slice()),
    ] {
        p[name] = json!(sha(bytes));
    }
    p
}
fn random_bytes(mut state: u64, len: usize) -> Vec<u8> {
    let mut bytes = vec![0; len];
    for chunk in bytes.chunks_mut(8) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        chunk.copy_from_slice(&state.to_le_bytes()[..chunk.len()]);
    }
    bytes
}
fn seed(trial: usize, large: bool) -> Vec<u8> {
    random_bytes(
        warm::fixture_seed(trial, large),
        if large { 16 << 20 } else { 1 << 20 },
    )
}
pub(super) fn persistent_next(
    trial: usize,
    large: bool,
    position: usize,
    base: &[u8],
) -> Result<Vec<u8>> {
    if trial >= 10
        || !(1..=10).contains(&position)
        || base.len() != if large { 16 << 20 } else { 1 << 20 }
    {
        return Err("invalid rolling fixture".into());
    }
    if position == 5 {
        let h = hash(
            b"spiralcore.persistent-session.escape.v1\0",
            &[&(trial as u64).to_le_bytes(), &[u8::from(large)]],
        );
        return Ok(random_bytes(
            u64::from_le_bytes(h[..8].try_into().unwrap()).max(1),
            base.len(),
        ));
    }
    let mut target = base.to_vec();
    let at = base.len() / 2
        + (if position < 5 {
            position - 1
        } else {
            position - 6
        }) * 64;
    for b in &mut target[at..at + 32] {
        *b ^= 0xa5;
    }
    Ok(target)
}
fn mixed_next(trial: usize, large: bool, position: usize, base: &[u8]) -> Result<Vec<u8>> {
    if trial >= 10
        || !(1..=10).contains(&position)
        || base.len() != if large { 16 << 20 } else { 1 << 20 }
    {
        return Err("invalid mixed fixture".into());
    }
    if position == 2 {
        return persistent_next(trial, large, 5, base);
    }
    if position == 4 {
        return Ok(vec![0; base.len()]);
    }
    if (6..=9).contains(&position) {
        let family = match position {
            8 => "variants",
            9 => "drift",
            _ => "normal",
        };
        let key = if position == 9 {
            b"\"message\"".as_slice()
        } else {
            b"\"note\"".as_slice()
        };
        let mut out = Vec::with_capacity(base.len());
        let extra = b"held-out persistent telemetry value ".repeat(8);
        let mut index = 0;
        while out.len() < base.len() {
            let row = record(false, index, trial * 10 + position, family);
            let key_at = row
                .windows(key.len())
                .position(|v| v == key)
                .ok_or("fixture key missing")?
                + key.len();
            let quote = key_at
                + row[key_at..]
                    .iter()
                    .position(|b| *b == b'"')
                    .ok_or("fixture value missing")?
                + 1;
            for part in [&row[..quote], extra.as_slice(), &row[quote..]] {
                let n = part.len().min(base.len() - out.len());
                out.extend_from_slice(&part[..n]);
            }
            index += 1;
        }
        return Ok(out);
    }
    let mut out = base.to_vec();
    let middle = out.len() / 2;
    for b in &mut out[middle..middle + 32] {
        *b ^= 0xa5;
    }
    Ok(out)
}
fn next_for(p: &Value, trial: usize, large: bool, position: usize, base: &[u8]) -> Result<Vec<u8>> {
    if mixed(p) {
        mixed_next(trial, large, position, base)
    } else {
        persistent_next(trial, large, position, base)
    }
}
fn mixed_kind(position: usize) -> &'static str {
    match position {
        1 | 3 => "localized-random",
        2 => "unrelated-replacement",
        4 => "zero",
        5 => "localized-zero",
        6 | 7 => "structured-normal",
        8 => "structured-variants",
        9 => "schema-drift",
        10 => "localized-drift",
        _ => "invalid",
    }
}
fn order(trial: usize, large: bool) -> [&'static str; 3] {
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let mut indices = [0, 1, 2, 3, 4, 5];
    let mut state = warm::fixture_seed(40 + trial / 6, large);
    for i in (1..6).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        indices.swap(i, (state % (i as u64 + 1)) as usize);
    }
    permutations[indices[trial % 6]].map(|i| ARMS[i])
}
fn scope(run: &str, trial: usize, large: bool, arm: &str) -> Id {
    hash(
        b"spiralcore.persistent-session.scope.v1\0",
        &[
            run.as_bytes(),
            &(trial as u64).to_le_bytes(),
            &[u8::from(large)],
            arm.as_bytes(),
        ],
    )
}
pub(super) fn persistent_plan(run: &str, shard: usize, source: &str, local: bool) -> Result<Value> {
    make_plan(run, shard, source, local, false)
}
fn unrelated_plan(run: &str, shard: usize, source: &str, local: bool) -> Result<Value> {
    make_plan(run, shard, source, local, true)
}
fn make_plan(
    run: &str,
    shard: usize,
    source: &str,
    local: bool,
    mixed_profile: bool,
) -> Result<Value> {
    if run.is_empty()
        || run.len() > 64
        || !run
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || shard >= if local { 10 } else { 6 }
        || source.len() != 40
        || !source
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("invalid persistent registration".into());
    }
    let mut fixtures = Vec::new();
    let mut sessions = Vec::new();
    let mut scopes = Vec::new();
    for large in [false, true] {
        let mut base = seed(shard, large);
        let initial = warm::metadata(&base);
        let mut ids = BTreeSet::from([object_id(&base)]);
        let mut targets = Vec::new();
        for position in 1..=10 {
            base = if mixed_profile {
                mixed_next(shard, large, position, &base)?
            } else {
                persistent_next(shard, large, position, &base)?
            };
            if !ids.insert(object_id(&base)) {
                return Err("duplicate target fixture".into());
            }
            let mut target_meta = json!({"position":position,"kind":if mixed_profile {mixed_kind(position)} else if position==5{"escape"}else{"localized"},"expected":warm::metadata(&base)});
            if mixed_profile {
                target_meta["records"] = json!(
                    base.iter().filter(|b| **b == b'\n').count()
                        + usize::from(base.last() != Some(&b'\n'))
                );
            }
            targets.push(target_meta);
        }
        fixtures.push(json!({"trial":shard,"large":large,"base":initial,"targets":targets}));
        for original_arm in order(shard, large) {
            let arm = if mixed_profile {
                MIXED_ARMS[ARMS.iter().position(|a| *a == original_arm).unwrap()]
            } else {
                original_arm
            };
            let s = hex(&scope(run, shard, large, arm));
            sessions.push(json!({"trial":shard,"large":large,"arm":arm,"scope":s}));
            scopes.push(s);
        }
    }
    let mut p = json!({"schema":"spiralcore.wan.plan.v4","experiment":"persistent-session.v1","dose":if local{"local-comparison"}else{"wan-pilot"},"run":run,"shard":shard,"source_commit":source,"sources":pins(),"sessions":sessions,"allowed_scopes":scopes,"fixtures":fixtures,
        "cold_graph_warming":false,"codec_cache_enabled":true,"http_keep_alive":{"max_requests_per_connection":32,"idle_timeout_ms":500},"warm_probe":false,
        "seeds":6,"targets":60,"operations":66,"schema_fits":4,"updates_per_session":10,"artifact":warm::metadata(ARTIFACT),"training_samples":512,
        "context_constructions":{"fresh-selector":20,"persistent-selector":2,"whole":2},"retained_payload_bytes":561u64<<20,"conservative_storage_bound":(617u64<<20)+66*4096});
    if mixed_profile {
        p["experiment"] = json!("unrelated-cpu.v1");
        p["operations"] = json!(70);
        p["schema_seeds"] = json!(4);
        p["schema_publication_before_position"] = json!(6);
        p["source_profiles"] = json!({"baseline":baseline_pins(pins()),"candidate":pins(),"baseline_revision":"56e15db5c77fe2ecdc03476d8e09664cd4213fd1"});
        p["context_constructions"] =
            json!({"baseline-selector":2,"candidate-selector":2,"whole":2});
        p["conservative_storage_bound"] =
            json!((617u64 << 20) + 70 * 4096 + 4 * ARTIFACT.len() as u64);
    }
    Ok(p)
}
fn read_plan(path: &Path, full: bool) -> Result<(Value, String)> {
    let bytes = fs::read(path)?;
    let p: Value = serde_json::from_slice(&bytes)?;
    if (!mixed(&p) && p["experiment"] != "persistent-session.v1")
        || (p["sources"] != pins()
            && !(mixed(&p) && !full && p["source_profiles"]["baseline"] == pins()))
        || mixed(&p)
            && (p["source_profiles"]["baseline"] != baseline_pins(p["sources"].clone())
                || p["source_profiles"]["candidate"] != p["sources"])
        || !matches!(p["dose"].as_str(), Some("local-comparison" | "wan-pilot"))
    {
        return Err("persistent source/experiment mismatch".into());
    }
    if full
        && p != make_plan(
            p["run"].as_str().ok_or("run missing")?,
            p["shard"].as_u64().ok_or("shard missing")? as usize,
            p["source_commit"].as_str().ok_or("source missing")?,
            p["dose"] == "local-comparison",
            mixed(&p),
        )?
    {
        return Err("persistent plan changed".into());
    }
    Ok((p, sha(&bytes)))
}
pub(super) fn persistent_peak_charge(retained: u64, target: usize, external: usize) -> Result<u64> {
    let total = retained
        .checked_add(target as u64)
        .and_then(|n| n.checked_add(SCRATCH))
        .and_then(|n| n.checked_add(EXTERNAL))
        .ok_or("sender charge overflow")?;
    if target > 16 << 20 || external as u64 > EXTERNAL || total > 512 << 20 {
        return Err("shared sender budget exceeded".into());
    }
    Ok(total)
}
pub(super) fn persistent_totals(
    setup: &Value,
    seed: &Value,
    updates: &[Value],
    generation: &[Value],
    outer: &Value,
) -> Result<Value> {
    if updates.len() != 10 || generation.len() != 10 || seed["phase"] != "seed" {
        return Err("incomplete persistent session".into());
    }
    let mut cpu = numeric(setup, "cpu_us")?;
    let mut wall = numeric(setup, "wall_us")?;
    let mut receiver = 0.0;
    let mut body = 0.0;
    let mut gen_cpu = 0.0;
    let mut gen_wall = 0.0;
    for (i, (op, g)) in updates.iter().zip(generation).enumerate() {
        if op["position"] != i + 1 || g["position"] != i + 1 || op["phase"] != "target" {
            return Err("rolling position changed".into());
        }
        cpu += numeric(op, "sender_cpu_us")?;
        wall += numeric(op, "wall_us")?;
        gen_cpu += numeric(g, "cpu_us")?;
        gen_wall += numeric(g, "wall_us")?;
    }
    for op in std::iter::once(seed).chain(updates) {
        if numeric(op, "cpu_us")? != numeric(op, "sender_cpu_us")? + numeric(op, "receiver_cpu_us")?
            || numeric(op, "body_bytes")?
                != numeric(op, "request_body_bytes")? + numeric(op, "response_body_bytes")?
            || numeric(op, "exchange_wall_us")? > numeric(op, "wall_us")?
        {
            return Err("operation accounting differs".into());
        }
        receiver += numeric(op, "receiver_cpu_us")?;
        body += numeric(op, "body_bytes")?;
    }
    let paid = cpu + numeric(seed, "sender_cpu_us")?;
    let paid_wall = wall + numeric(seed, "wall_us")?;
    if numeric(outer, "cpu_us")? + 1.0 < paid + gen_cpu
        || numeric(outer, "wall_us")? + 1.0 < paid_wall + gen_wall
    {
        return Err("outer interval excludes charged phases".into());
    }
    Ok(
        json!({"setup_and_updates_sender_cpu_us":cpu,"setup_and_updates_wall_us":wall,"paid_lifecycle_sender_cpu_us":paid,"paid_lifecycle_cpu_us":paid+receiver,"paid_lifecycle_wall_us":paid_wall,
        "generation_cpu_us":gen_cpu,"generation_wall_us":gen_wall,"outer_sender_cpu_us":numeric(outer,"cpu_us")?,"outer_cpu_subtotal_us":numeric(outer,"cpu_us")?+receiver,"outer_wall_us":numeric(outer,"wall_us")?,"receiver_cpu_us":receiver,"body_bytes":body}),
    )
}
fn session_totals(
    setup: &Value,
    seed: &Value,
    schema_seed: Option<&Value>,
    updates: &[Value],
    generation: &[Value],
    outer: &Value,
) -> Result<Value> {
    let mut paid_setup = setup.clone();
    if let Some(s) = schema_seed {
        if s["phase"] != "schema_seed"
            || s["mode"] != "Raw"
            || s["body_bytes"] != warm::raw_cost(ARTIFACT.len() as u64)
            || numeric(s, "cpu_us")?
                != numeric(s, "sender_cpu_us")? + numeric(s, "receiver_cpu_us")?
            || numeric(s, "body_bytes")?
                != numeric(s, "request_body_bytes")? + numeric(s, "response_body_bytes")?
        {
            return Err("schema publication accounting differs".into());
        }
        paid_setup["cpu_us"] = json!(numeric(setup, "cpu_us")? + numeric(s, "sender_cpu_us")?);
        paid_setup["wall_us"] = json!(numeric(setup, "wall_us")? + numeric(s, "wall_us")?);
    }
    let mut result = persistent_totals(&paid_setup, seed, updates, generation, outer)?;
    if let Some(s) = schema_seed {
        for key in [
            "paid_lifecycle_cpu_us",
            "outer_cpu_subtotal_us",
            "receiver_cpu_us",
        ] {
            result[key] = json!(numeric(&result, key)? + numeric(s, "receiver_cpu_us")?);
        }
        result["body_bytes"] = json!(numeric(&result, "body_bytes")? + numeric(s, "body_bytes")?);
    }
    Ok(result)
}

enum Sender {
    Fresh {
        artifact: Vec<u8>,
        base: Option<Vec<u8>>,
    },
    Persistent(WarmSession),
    Whole {
        encoder: zstd_whole::PreparedEncoder,
        base: Option<Vec<u8>>,
    },
}
#[derive(Default)]
struct SetupPhases {
    generation: (u128, u128),
    fit: (u128, u128),
    constructor: (u128, u128),
    training_capacity: usize,
    training_bytes: usize,
}
impl Sender {
    fn new(arm: &str, c: &Context, stages: &mut SetupPhases) -> Result<Self> {
        if arm == "whole" {
            let w = Instant::now();
            let t = ProcessTime::now();
            let encoder = zstd_whole::PreparedEncoder::new()?;
            stages.constructor = (t.elapsed().as_micros(), w.elapsed().as_micros());
            return Ok(Self::Whole {
                encoder,
                base: None,
            });
        }
        let w = Instant::now();
        let t = ProcessTime::now();
        let training = training_records();
        let refs = training.iter().map(Vec::as_slice).collect::<Vec<_>>();
        stages.generation = (t.elapsed().as_micros(), w.elapsed().as_micros());
        stages.training_bytes = training.iter().map(Vec::len).sum();
        stages.training_capacity = training.capacity() * std::mem::size_of::<Vec<u8>>()
            + training.iter().map(Vec::capacity).sum::<usize>()
            + refs.capacity() * std::mem::size_of::<&[u8]>();
        if stages.training_capacity > 56 << 20 {
            return Err("training exceeds external allowance".into());
        }
        let w = Instant::now();
        let t = ProcessTime::now();
        let artifact = schema::fit(&refs)?;
        stages.fit = (t.elapsed().as_micros(), w.elapsed().as_micros());
        if artifact != ARTIFACT {
            return Err("training artifact changed".into());
        }
        match arm {
            "fresh-selector" => Ok(Self::Fresh {
                artifact,
                base: None,
            }),
            "persistent-selector" | "baseline-selector" | "candidate-selector" => {
                let w = Instant::now();
                let t = ProcessTime::now();
                let session = WarmSession::new(c, &artifact)?;
                stages.constructor = (t.elapsed().as_micros(), w.elapsed().as_micros());
                Ok(Self::Persistent(session))
            }
            _ => Err("unknown session arm".into()),
        }
    }
    fn base(&self) -> Result<&[u8]> {
        match self {
            Self::Persistent(s) => s.base_bytes(),
            Self::Fresh { base, .. } | Self::Whole { base, .. } => base.as_deref(),
        }
        .ok_or_else(|| "acknowledged base missing".into())
    }
    fn retained(&self) -> (usize, u64) {
        match self {
            Self::Fresh { artifact, base } => {
                let n = artifact.capacity() + base.as_ref().map_or(0, Vec::capacity);
                (n, n as u64 + 4096)
            }
            Self::Persistent(s) => (s.retained_bytes(), s.retained_charge()),
            Self::Whole { encoder, base } => {
                let n = base.as_ref().map_or(0, Vec::capacity);
                (
                    encoder.retained_bytes() + n,
                    encoder.retained_charge() + n as u64 + 4096,
                )
            }
        }
    }
}

fn expected(p: &Value, large: bool) -> Result<&Value> {
    p["fixtures"]
        .as_array()
        .ok_or("fixtures missing")?
        .iter()
        .find(|f| f["large"] == large)
        .ok_or_else(|| "fixture missing".into())
}
fn choice_json(c: &spiralcore_transfer::schema_selection::WarmSelection) -> Value {
    json!({"selected_mode":format!("{:?}",c.selected_mode),"needs_schema_seed":c.needs_schema_seed,
        "candidates":c.candidates.iter().map(|x|json!({"mode":format!("{:?}",x.mode),"target_http_body_bytes":x.target_http_body_bytes,"setup_http_body_bytes":x.setup_http_body_bytes,"total_http_body_bytes":x.total_http_body_bytes})).collect::<Vec<_>>(),
        "pruned":c.pruned.iter().map(|x|json!({"mode":format!("{:?}",x.mode),"target_http_body_lower_bound":x.target_http_body_lower_bound,"setup_http_body_bytes":x.setup_http_body_bytes,"total_http_body_lower_bound":x.total_http_body_lower_bound})).collect::<Vec<_>>()})
}
fn frame_capacity(frames: &Vec<Frame>) -> usize {
    frames.capacity() * std::mem::size_of::<Frame>()
        + frames.iter().map(|f| f.body.capacity()).sum::<usize>()
}
struct Buffered {
    measured: Measured,
    generation: (u128, u128),
    constructor: (u128, u128),
    choice: Option<spiralcore_transfer::schema_selection::WarmSelection>,
    frames: Vec<Frame>,
    retained: (usize, u64),
    peak: u64,
    input_capacity: usize,
}
fn session(args: &[String]) -> Result<()> {
    let (p, digest) = read_plan(Path::new(&args[2]), false)?;
    let trial: usize = args[3].parse()?;
    let large = match args[4].as_str() {
        "0" => false,
        "1" => true,
        _ => return Err("invalid size".into()),
    };
    let arm = args[5].as_str();
    let receiver = &args[7];
    wan::parse_id(receiver)?;
    let s = scope(p["run"].as_str().ok_or("run missing")?, trial, large, arm);
    let label = json!({"trial":trial,"large":large,"arm":arm,"scope":hex(&s)});
    if !p["sessions"]
        .as_array()
        .ok_or("sessions missing")?
        .contains(&label)
    {
        return Err("unregistered session".into());
    }
    let meta = expected(&p, large)?;
    let mut expected_ids = vec![wan::parse_id(
        meta["base"]["object"].as_str().ok_or("base id")?,
    )?];
    for t in meta["targets"].as_array().ok_or("targets missing")? {
        expected_ids.push(wan::parse_id(
            t["expected"]["object"].as_str().ok_or("target id")?,
        )?);
    }
    if expected_ids.len() != 11 || expected_ids.iter().collect::<BTreeSet<_>>().len() != 11 {
        return Err("distinct canonical targets required".into());
    }
    let own = identity()?;
    if own["sources"] != *role_pins(&p, arm) {
        return Err("worker role/source mismatch".into());
    }
    let token = std::env::var("SPIRALCORE_WAN_TOKEN")?;
    let mut out = fresh(Path::new(&args[8]))?;
    write_row(
        &mut out,
        &json!({"event":"session_registration","identity":own,"plan_sha256":digest,"session":label,"receiver_executable_sha256":receiver}),
    )?;
    let result = (|| -> Result<()> {
        let w = Instant::now();
        let c = ProcessTime::now();
        let initial = seed(trial, large);
        let initial_time = (c.elapsed().as_micros(), w.elapsed().as_micros());
        if warm::metadata(&initial) != meta["base"] {
            return Err("initial fixture identity changed".into());
        }
        // Fixed metadata reserve covers plan/JSON trees, deferred rows and small
        // owned objects. The only large external allocation is captured winning
        // frames; it is capacity-counted, capped and never duplicated in timers.
        let metadata_reserve = 8usize << 20;
        let mut buffered = Vec::with_capacity(10);
        let context = wan::context(s);
        let outer_w = Instant::now();
        let outer_c = ProcessTime::now();
        let mut stages = SetupPhases::default();
        let w = Instant::now();
        let c = ProcessTime::now();
        let mut http = HttpClient::new(&args[6], &token, s)?;
        let sender_result = Sender::new(arm, &context, &mut stages);
        let setup_time = (c.elapsed().as_micros(), w.elapsed().as_micros());
        let mut sender = match sender_result {
            Ok(s) => s,
            Err(e) => {
                write_row(
                    &mut out,
                    &json!({"event":"context_setup","cpu_us":setup_time.0,"wall_us":setup_time.1,"error":e.to_string()}),
                )?;
                return Err(e);
            }
        };
        let setup_retained = sender.retained();
        let seed_peak =
            persistent_peak_charge(setup_retained.1, initial.capacity(), metadata_reserve)?;
        let seed_id = hash(b"spiralcore.persistent-session.transfer.v1\0", &[&s, &[0]]);
        let seeded = measure(&mut http, |exchange| match &mut sender {
            Sender::Persistent(session) => session.seed_base(&context, initial, seed_id, exchange),
            Sender::Fresh { base, .. } | Sender::Whole { base, .. } => {
                let stats =
                    client::transfer_batched(&initial, Mode::Raw, None, seed_id, false, exchange)?;
                if stats.object != expected_ids[0] {
                    return Err(spiralcore_transfer::Error::Integrity);
                }
                *base = Some(initial);
                Ok(stats)
            }
        });
        let mut failure = seeded.result.as_ref().err().map(ToString::to_string);
        if seeded
            .result
            .as_ref()
            .is_ok_and(|r| r.object != expected_ids[0])
        {
            failure = Some("seed ACK identity changed".into());
        }
        let mut captured = 0usize;
        let mut published = None;
        if failure.is_none() {
            for (position, expected_id) in expected_ids.iter().enumerate().skip(1) {
                if mixed(&p) && arm != "whole" && position == 6 {
                    let id = hash(b"spiralcore.persistent-session.transfer.v1\0", &[&s, &[11]]);
                    let measured = measure(&mut http, |exchange| match &mut sender {
                        Sender::Persistent(session) => session.seed_schema(&context, id, exchange),
                        _ => Err(spiralcore_transfer::Error::Sequence),
                    });
                    if let Err(e) = &measured.result {
                        failure = Some(e.to_string());
                    }
                    if measured.result.as_ref().is_ok_and(|r| {
                        hex(&r.object) != p["artifact"]["object"].as_str().unwrap_or("")
                    }) {
                        failure = Some("artifact ACK identity changed".into());
                    }
                    published = Some(measured);
                    if failure.is_some() {
                        break;
                    }
                }
                let w = Instant::now();
                let c = ProcessTime::now();
                let target_result = next_for(&p, trial, large, position, sender.base()?);
                let gen_time = (c.elapsed().as_micros(), w.elapsed().as_micros());
                let target = target_result?;
                let capacity = target.capacity();
                let mut ctor = (0, 0);
                let mut choice = None;
                let mut frames = Vec::new();
                let retained = sender.retained();
                let mut peak =
                    persistent_peak_charge(retained.1, capacity, metadata_reserve + captured)?;
                let id = hash(
                    b"spiralcore.persistent-session.transfer.v1\0",
                    &[&s, &[position as u8]],
                );
                let measured = measure(&mut http, |exchange| {
                    let mut observed = |batch: &[Frame]| {
                        if arm != "whole" {
                            let incoming = batch
                                .iter()
                                .map(|f| f.body.len() + std::mem::size_of::<Frame>())
                                .sum::<usize>();
                            if captured + frame_capacity(&frames) + incoming > 24 << 20 {
                                return Err(spiralcore_transfer::Error::Limit);
                            }
                            frames.extend_from_slice(batch);
                        }
                        exchange(batch)
                    };
                    match &mut sender {
                        Sender::Fresh { artifact, base } => {
                            let w = Instant::now();
                            let c = ProcessTime::now();
                            let mut encoder = PreparedSelector::new(artifact)?;
                            ctor = (c.elapsed().as_micros(), w.elapsed().as_micros());
                            peak = persistent_peak_charge(
                                encoder.retained_charge() + retained.1,
                                capacity,
                                metadata_reserve + captured,
                            )
                            .map_err(|_| spiralcore_transfer::Error::Limit)?;
                            let prepared =
                                encoder.prepare_warm(&target, base.as_deref(), false, false)?;
                            if prepared.selection().needs_schema_seed {
                                return Err(spiralcore_transfer::Error::Missing);
                            }
                            choice = Some(prepared.selection().clone());
                            let stats = client::transfer_warm_selection_batched(
                                prepared,
                                id,
                                &mut observed,
                            )?;
                            if stats.object != *expected_id {
                                return Err(spiralcore_transfer::Error::Integrity);
                            }
                            *base = Some(target);
                            Ok(stats)
                        }
                        Sender::Persistent(session) => {
                            let stats = session.send(&context, target, id, false, &mut observed)?;
                            choice = Some(stats.selection);
                            Ok(stats.transfer)
                        }
                        Sender::Whole { encoder, base } => {
                            let stats = client::transfer_prepared_zstd_whole_batched(
                                encoder,
                                &target,
                                id,
                                false,
                                &mut observed,
                            )?;
                            if stats.object != *expected_id {
                                return Err(spiralcore_transfer::Error::Integrity);
                            }
                            *base = Some(target);
                            Ok(stats)
                        }
                    }
                });
                if measured
                    .result
                    .as_ref()
                    .is_ok_and(|r| r.object != *expected_id)
                {
                    failure = Some("target ACK identity changed".into());
                }
                if let Err(e) = &measured.result {
                    failure = Some(e.to_string());
                }
                captured += frame_capacity(&frames);
                buffered.push(Buffered {
                    measured,
                    generation: gen_time,
                    constructor: ctor,
                    choice,
                    frames,
                    retained: sender.retained(),
                    peak,
                    input_capacity: capacity,
                });
                if failure.is_some() {
                    break;
                }
            }
        }
        let outer =
            json!({"cpu_us":outer_c.elapsed().as_micros(),"wall_us":outer_w.elapsed().as_micros()});
        // Serialization and transcript hashing start only after the outer clock.
        let setup = json!({"event":"context_setup","cpu_us":setup_time.0,"wall_us":setup_time.1,"retained_bytes":setup_retained.0,"retained_charge":setup_retained.1,"schema_fits":usize::from(arm!="whole"),"training_samples":if arm=="whole"{0}else{512},"training_bytes":stages.training_bytes,"training_capacity":stages.training_capacity,"training_generation_cpu_us":stages.generation.0,"training_generation_wall_us":stages.generation.1,"fit_cpu_us":stages.fit.0,"fit_wall_us":stages.fit.1,"constructor_cpu_us":stages.constructor.0,"constructor_wall_us":stages.constructor.1,"artifact":if arm=="whole"{Value::Null}else{p["artifact"].clone()},"constructors":usize::from(arm!="fresh-selector"),"error":Value::Null});
        write_row(
            &mut out,
            &json!({"event":"initial_generation","cpu_us":initial_time.0,"wall_us":initial_time.1}),
        )?;
        write_row(&mut out, &setup)?;
        let mut seed_row = seeded.row_with_metadata("seed", None, &meta["base"]);
        seed_row["peak_charge"] = json!(seed_peak);
        write_row(&mut out, &seed_row)?;
        let mut targets = Vec::new();
        let mut generations = Vec::new();
        let schema_row = published.as_ref().map(|m| {
            let mut r = m.row_with_metadata("schema_seed", None, &p["artifact"]);
            r["peak_charge"] = json!(seed_peak);
            r
        });
        let mut peak = seed_peak;
        for (i, b) in buffered.iter().enumerate() {
            if i == 5 {
                if let Some(r) = &schema_row {
                    write_row(&mut out, r)?;
                }
            }
            let g = json!({"event":"generation","position":i+1,"cpu_us":b.generation.0,"wall_us":b.generation.1,"input_capacity":b.input_capacity});
            write_row(&mut out, &g)?;
            generations.push(g);
            let mut r = b.measured.row_with_metadata(
                "target",
                Some(i + 1),
                &meta["targets"][i]["expected"],
            );
            r["kind"] = meta["targets"][i]["kind"].clone();
            r["selection"] = b.choice.as_ref().map(choice_json).unwrap_or(Value::Null);
            r["constructor_cpu_us"] = json!(b.constructor.0);
            r["constructor_wall_us"] = json!(b.constructor.1);
            r["constructors"] = json!(usize::from(arm == "fresh-selector"));
            r["normalized_frames_sha256"] = if arm == "whole" {
                Value::Null
            } else {
                json!(warm::normalized_frames(&b.frames)?)
            };
            r["retained_bytes"] = json!(b.retained.0);
            r["retained_charge"] = json!(b.retained.1);
            r["peak_charge"] = json!(b.peak);
            peak = peak.max(b.peak);
            write_row(&mut out, &r)?;
            targets.push(r);
        }
        if buffered.len() < 6 {
            if let Some(r) = &schema_row {
                write_row(&mut out, r)?;
            }
        }
        write_row(
            &mut out,
            &json!({"event":"session_outer","cpu_us":outer["cpu_us"],"wall_us":outer["wall_us"],"captured_frame_capacity":captured,"external_metadata_reserve":metadata_reserve,"external_capacity_bound":metadata_reserve+captured,"peak_charge":peak,"retained_bytes":sender.retained().0,"retained_charge":sender.retained().1}),
        )?;
        if let Some(e) = failure {
            return Err(e.into());
        }
        let totals = session_totals(
            &setup,
            &seed_row,
            schema_row.as_ref(),
            &targets,
            &generations,
            &outer,
        )?;
        write_row(
            &mut out,
            &json!({"event":"session_complete","seeds":1,"targets":10,"totals":totals}),
        )?;
        Ok(())
    })();
    if let Err(e) = result {
        write_row(&mut out, &json!({"event":"failure","error":e.to_string()}))?;
        return Err(e);
    }
    Ok(())
}

fn paired(
    plan_path: &Path,
    exe: &Path,
    baseline: Option<&Path>,
    url: &str,
    receiver: &str,
    out_path: &Path,
) -> Result<()> {
    let (p, digest) = read_plan(plan_path, true)?;
    let mut out = fresh(out_path)?;
    let result = (|| -> Result<()> {
        wan::parse_id(receiver)?;
        let found = Command::new(exe)
            .arg(format!("{}-identity", prefix(&p)))
            .output()?;
        let id: Value = serde_json::from_slice(&found.stdout)?;
        if !found.status.success()
            || id["sources"] != p["sources"]
            || id["executable_sha256"] != sha(&fs::read(exe)?)
        {
            return Err("worker executable/source identity mismatch".into());
        }
        let baseline_id = if mixed(&p) {
            let path = baseline.ok_or("baseline executable missing")?;
            let output = Command::new(path).arg("unrelated-identity").output()?;
            let v: Value = serde_json::from_slice(&output.stdout)?;
            if !output.status.success()
                || v["sources"] != *role_pins(&p, "baseline-selector")
                || v["executable_sha256"] != sha(&fs::read(path)?)
            {
                return Err("baseline executable/source mismatch".into());
            }
            Some(v)
        } else {
            None
        };
        let mut registration = json!({"event":"registration","identity":id,"plan_sha256":digest,"source_commit":p["source_commit"],"run":p["run"],"shard":p["shard"],"dose":p["dose"],"receiver_executable_sha256":receiver,"url":url});
        if let Some(id) = baseline_id {
            registration["baseline_identity"] = id;
        }
        write_row(&mut out, &registration)?;
        let dir = out_path.with_extension("sessions");
        fs::create_dir(&dir)?;
        for (index, s) in p["sessions"]
            .as_array()
            .ok_or("sessions missing")?
            .iter()
            .enumerate()
        {
            let path = dir.join(format!("{index:02}.jsonl"));
            let worker = if s["arm"] == "baseline-selector" {
                baseline.ok_or("baseline missing")?
            } else {
                exe
            };
            let launched = Command::new(worker)
                .arg(format!("{}-session", prefix(&p)))
                .arg(plan_path)
                .arg(s["trial"].as_u64().ok_or("trial")?.to_string())
                .arg(if s["large"] == true { "1" } else { "0" })
                .arg(s["arm"].as_str().ok_or("arm")?)
                .arg(url)
                .arg(receiver)
                .arg(&path)
                .output();
            let output = match launched {
                Ok(v) => v,
                Err(e) => {
                    write_row(
                        &mut out,
                        &json!({"event":"spawn_failure","index":index,"error":e.to_string()}),
                    )?;
                    return Err(e.into());
                }
            };
            let raw = fs::read(&path).unwrap_or_default();
            let parsed = rows(&raw);
            if !output.status.success() || parsed.is_err() {
                write_row(
                    &mut out,
                    &json!({"event":"session_failure","index":index,"session":s,"exit_code":output.status.code(),"stdout":String::from_utf8_lossy(&output.stdout),"stderr":String::from_utf8_lossy(&output.stderr),"raw":String::from_utf8_lossy(&raw),"raw_sha256":sha(&raw),"parse_error":parsed.err().map(|e|e.to_string())}),
                )?;
                return Err("session failed; no retry".into());
            }
            write_row(
                &mut out,
                &json!({"event":"session","index":index,"session":s,"rows":parsed?,"raw_sha256":sha(&raw)}),
            )?;
            fs::remove_file(path)?;
        }
        fs::remove_dir(dir)?;
        write_row(
            &mut out,
            &json!({"event":"complete","sessions":6,"operations":p["operations"]}),
        )
    })();
    if let Err(e) = result {
        write_row(&mut out, &json!({"event":"failure","error":e.to_string()}))?;
        return Err(e);
    }
    Ok(())
}
fn observed_requests(p: &Value, bytes: &[u8]) -> Result<Vec<Value>> {
    let observed = observations(p, bytes)?;
    if observed.len() as u64 != p["operations"].as_u64().ok_or("operation count")? {
        return Err("admitted request observation count differs".into());
    }
    let mut connection = 0;
    let mut in_connection = 0;
    for r in &observed {
        let c = r["connection_ordinal"]
            .as_u64()
            .filter(|v| *v > 0)
            .ok_or("connection ordinal missing")?;
        let n = r["request_on_connection"]
            .as_u64()
            .filter(|v| *v > 0 && *v <= 32)
            .ok_or("connection request ordinal missing")?;
        if c < connection || c == connection && n <= in_connection || c > connection && n != 1 {
            return Err("connection observations inconsistent".into());
        }
        connection = c;
        in_connection = n;
    }
    Ok(observed)
}
fn verify(
    plan_path: &Path,
    cache: &Path,
    out_path: &Path,
    log_path: &Path,
    receiver: &str,
) -> Result<()> {
    let (p, digest) = read_plan(plan_path, true)?;
    let mut out = fresh(out_path)?;
    let result = (|| -> Result<()> {
        if !cache.is_dir() {
            return Err("stopped existing receiver store required".into());
        }
        wan::parse_id(receiver)?;
        let log = fs::read(log_path)?;
        let observed = observed_requests(&p, &log)?;
        write_row(
            &mut out,
            &json!({"event":"verification_registration","identity":identity()?,"source_commit":p["source_commit"],"plan_sha256":digest,"receiver_executable_sha256":receiver,"observation_sha256":sha(&log),"observations":observed}),
        )?;
        let mut endpoint = Endpoint::open_with_policy(
            cache,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )?;
        let mut exact = true;
        let mut checked = 0;
        for s in p["sessions"].as_array().ok_or("sessions")? {
            let trial = s["trial"].as_u64().ok_or("trial")? as usize;
            let large = s["large"].as_bool().ok_or("size")?;
            let ctx = wan::context(wan::parse_id(s["scope"].as_str().ok_or("scope")?)?);
            let mut bytes = seed(trial, large);
            let publication = mixed(&p) && s["arm"] != "whole";
            for slot in 0..(11 + usize::from(publication)) {
                let artifact = publication && slot == 6;
                let position = slot - usize::from(publication && slot >= 6);
                if position > 0 && !artifact {
                    bytes = next_for(&p, trial, large, position, &bytes)?;
                }
                let expected_bytes = if artifact { ARTIFACT } else { bytes.as_slice() };
                let meta = warm::metadata(expected_bytes);
                let w = Instant::now();
                let c = ProcessTime::now();
                let actual = endpoint.read_object(&ctx, object_id(expected_bytes));
                let same = actual
                    .as_ref()
                    .is_ok_and(|v| v.as_slice() == expected_bytes);
                let cpu = c.elapsed().as_micros();
                let wall = w.elapsed().as_micros();
                exact &= same;
                checked += 1;
                write_row(
                    &mut out,
                    &json!({"event":"receiver_verification","scope":s["scope"],"phase":if artifact{"schema_seed"}else if position==0{"seed"}else{"target"},"position":if position==0||artifact{Value::Null}else{json!(position)},"expected":meta,"exact":same,"actual_bytes":actual.as_ref().ok().map(Vec::len),"actual_sha256":actual.as_ref().ok().map(|v|sha(v)),"cpu_us":cpu,"wall_us":wall,"error":actual.err().map(|e|e.to_string())}),
                )?;
            }
        }
        write_row(
            &mut out,
            &json!({"event":"verification_complete","checked":checked,"exact":exact}),
        )?;
        if !exact || checked != p["operations"].as_u64().ok_or("operations")? {
            return Err("stopped-store exactness failed".into());
        }
        Ok(())
    })();
    if let Err(e) = result {
        write_row(&mut out, &json!({"event":"failure","error":e.to_string()}))?;
        return Err(e);
    }
    Ok(())
}
fn validate_selection(
    c: &Value,
    mode: &Value,
    body: u64,
    len: u64,
    localized: bool,
    schema_ready: bool,
) -> Result<()> {
    if c["needs_schema_seed"] != false || c["selected_mode"] != *mode {
        return Err("selection mode/readiness mismatch".into());
    }
    let candidates = c["candidates"]
        .as_array()
        .ok_or("candidate receipt required")?;
    let pruned = c["pruned"].as_array().ok_or("pruning receipt required")?;
    let priority = ["Raw", "ZstdWholeV1", "SparseV1", "SchemaZstdV1"];
    let mut seen = BTreeSet::new();
    let mut best = None;
    let mut last = 0;
    for (index, v) in candidates.iter().enumerate() {
        let name = v["mode"].as_str().ok_or("candidate mode missing")?;
        let rank = priority
            .iter()
            .position(|m| *m == name)
            .ok_or("candidate mode invalid")?;
        if !seen.insert(name) || index > 0 && rank <= last {
            return Err("candidate ordering/duplication".into());
        }
        last = rank;
        let target = v["target_http_body_bytes"]
            .as_u64()
            .filter(|v| *v > 0)
            .ok_or("candidate cost missing")?;
        let setup = if name == "SchemaZstdV1" && !schema_ready {
            warm::raw_cost(ARTIFACT.len() as u64)
        } else {
            0
        };
        let total = target.checked_add(setup).ok_or("cost overflow")?;
        if v["setup_http_body_bytes"] != setup
            || v["total_http_body_bytes"] != total
            || name == "Raw" && target != warm::raw_cost(len)
        {
            return Err("candidate cost inconsistent".into());
        }
        if best.is_none_or(|(_, cost)| total < cost) {
            best = Some((name, total));
        }
    }
    let (selected, cost) = best.ok_or("no candidate")?;
    if mode != selected || cost != body {
        return Err("nonminimum selected bytes".into());
    }
    for v in pruned {
        let name = v["mode"].as_str().ok_or("pruned mode missing")?;
        if !priority.contains(&name) || !seen.insert(name) {
            return Err("pruned mode duplicated".into());
        }
        let target = v["target_http_body_lower_bound"]
            .as_u64()
            .filter(|v| *v > 0)
            .ok_or("lower bound missing")?;
        let setup = if name == "SchemaZstdV1" && !schema_ready {
            warm::raw_cost(ARTIFACT.len() as u64)
        } else {
            0
        };
        let total = target.checked_add(setup).ok_or("lower bound overflow")?;
        if v["setup_http_body_bytes"] != setup
            || v["total_http_body_lower_bound"] != total
            || total < cost
        {
            return Err("invalid pruning cost".into());
        }
    }
    // Sparse may be unavailable on the unrelated escape. Raw is mandatory.
    if !seen.contains("Raw")
        || !seen.contains("ZstdWholeV1")
        || !seen.contains("SchemaZstdV1")
        || localized && (mode != "SparseV1" || body != 600)
    {
        return Err("required candidate or localized result changed".into());
    }
    Ok(())
}
fn validate_receipts(
    p: &Value,
    digest: &str,
    raw: &[Value],
    verified: &[Value],
) -> Result<(Vec<Value>, Vec<Value>)> {
    let op_count = p["operations"].as_u64().ok_or("operations missing")? as usize;
    if raw.len() != 8
        || verified.len() != op_count + 2
        || raw[0]["event"] != "registration"
        || raw[0]["plan_sha256"] != digest
        || raw[0]["source_commit"] != p["source_commit"]
        || raw[0]["run"] != p["run"]
        || raw[0]["shard"] != p["shard"]
        || raw[0]["dose"] != p["dose"]
        || raw[0]["identity"]["sources"] != p["sources"]
        || raw[7] != json!({"event":"complete","sessions":6,"operations":op_count})
        || verified[0]["event"] != "verification_registration"
        || verified[0]["plan_sha256"] != digest
        || verified[0]["source_commit"] != p["source_commit"]
        || verified[0]["identity"]["sources"] != p["sources"]
        || verified[0]["receiver_executable_sha256"] != raw[0]["receiver_executable_sha256"]
        || verified[op_count + 1]
            != json!({"event":"verification_complete","checked":op_count,"exact":true})
    {
        return Err("incomplete or unbound persistent shard".into());
    }
    if mixed(p) {
        if raw[0]["baseline_identity"]["sources"] != *role_pins(p, "baseline-selector") {
            return Err("baseline identity absent/changed".into());
        }
        wan::parse_id(
            raw[0]["baseline_identity"]["executable_sha256"]
                .as_str()
                .ok_or("baseline executable pin")?,
        )?;
    }
    for h in [
        &raw[0]["identity"]["executable_sha256"],
        &raw[0]["receiver_executable_sha256"],
        &verified[0]["identity"]["executable_sha256"],
        &verified[0]["observation_sha256"],
    ] {
        wan::parse_id(h.as_str().ok_or("hash missing")?)?;
    }
    let observed = verified[0]["observations"]
        .as_array()
        .ok_or("observations missing")?;
    observed_requests(
        p,
        observed
            .iter()
            .map(serde_json::to_string)
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join("\n")
            .as_bytes(),
    )?;
    let mut operations = Vec::new();
    let mut totals = Vec::new();
    let mut cursor = 0;
    for (index, s) in p["sessions"]
        .as_array()
        .ok_or("sessions")?
        .iter()
        .enumerate()
    {
        let r = &raw[index + 1];
        if r["event"] != "session" || r["index"] != index || r["session"] != *s {
            return Err("session order differs".into());
        }
        wan::parse_id(r["raw_sha256"].as_str().ok_or("session hash")?)?;
        let r = r["rows"].as_array().ok_or("session rows")?;
        let arm = s["arm"].as_str().ok_or("arm")?;
        let publication = mixed(p) && arm != "whole";
        let extra = usize::from(publication);
        let outer = 24 + extra;
        let complete = 25 + extra;
        let source_identity = if arm == "baseline-selector" {
            &raw[0]["baseline_identity"]
        } else {
            &raw[0]["identity"]
        };
        if r.len() != 26 + extra
            || r[0]["event"] != "session_registration"
            || r[0]["session"] != *s
            || r[0]["identity"] != *source_identity
            || r[0]["plan_sha256"] != digest
            || r[0]["receiver_executable_sha256"] != raw[0]["receiver_executable_sha256"]
            || r[1]["event"] != "initial_generation"
            || r[2]["event"] != "context_setup"
            || !r[2]["error"].is_null()
            || r[outer]["event"] != "session_outer"
            || r[complete]["event"] != "session_complete"
            || r[complete]["seeds"] != 1
            || r[complete]["targets"] != 10
        {
            return Err("incomplete worker rows".into());
        }
        let is_selector = arm != "whole";
        let meta = expected(p, s["large"].as_bool().ok_or("size")?)?;
        if r[2]["schema_fits"] != usize::from(is_selector)
            || r[2]["training_samples"] != if is_selector { 512 } else { 0 }
            || r[2]["constructors"] != usize::from(arm != "fresh-selector")
            || r[2]["artifact"]
                != if is_selector {
                    p["artifact"].clone()
                } else {
                    Value::Null
                }
        {
            return Err("once-fitted setup changed".into());
        }
        for key in ["cpu_us", "wall_us"] {
            numeric(&r[1], key)?;
            numeric(&r[2], key)?;
            numeric(&r[outer], key)?;
        }
        if numeric(&r[outer], "external_capacity_bound")? > EXTERNAL as f64
            || numeric(&r[outer], "peak_charge")? > (512u64 << 20) as f64
        {
            return Err("sender budget exceeded".into());
        }
        let mut target_rows = Vec::new();
        let mut generations = Vec::new();
        let mut connections = BTreeSet::new();
        for slot in 0..(11 + extra) {
            let artifact = publication && slot == 6;
            let position = slot - usize::from(publication && slot >= 6);
            let op = &r[if artifact {
                14
            } else if position == 0 {
                3
            } else {
                3 + 2 * position + usize::from(publication && position >= 6)
            }];
            let m = if artifact {
                &p["artifact"]
            } else if position == 0 {
                &meta["base"]
            } else {
                &meta["targets"][position - 1]["expected"]
            };
            let phase = if artifact {
                "schema_seed"
            } else if position == 0 {
                "seed"
            } else {
                "target"
            };
            let pos = if position == 0 || artifact {
                Value::Null
            } else {
                json!(position)
            };
            if op["event"] != "operation"
                || op["phase"] != phase
                || op["position"] != pos
                || !op["error"].is_null()
                || op["bytes"] != m["bytes"]
                || op["object"] != m["object"]
                || op["sha256"] != m["sha256"]
                || op["batches"] != 1
            {
                return Err("operation identity/count/error differs".into());
            }
            for key in METRICS_WAN {
                numeric(op, key)?;
            }
            if numeric(op, "peak_charge")? > (512u64 << 20) as f64 {
                return Err("operation sender charge".into());
            }
            let check = &verified[1 + cursor];
            let obs = &observed[cursor];
            cursor += 1;
            if check["event"] != "receiver_verification"
                || check["scope"] != s["scope"]
                || check["phase"] != phase
                || check["position"] != pos
                || check["expected"] != *m
                || check["exact"] != true
                || check["actual_bytes"] != m["bytes"]
                || check["actual_sha256"] != m["sha256"]
                || !check["error"].is_null()
                || obs["scope"] != s["scope"]
            {
                return Err("exact stopped-store comparison/observation mismatch".into());
            }
            connections.insert(obs["connection_ordinal"].as_u64().ok_or("connection")?);
            if position == 0 || artifact {
                if op["mode"] != "Raw"
                    || op["body_bytes"] != warm::raw_cost(m["bytes"].as_u64().ok_or("length")?)
                {
                    return Err("seed raw identity/cost changed".into());
                }
            } else {
                let g = &r[2 + 2 * position + usize::from(publication && position >= 6)];
                if g["event"] != "generation"
                    || g["position"] != position
                    || g["input_capacity"] != m["bytes"]
                    || op["kind"] != meta["targets"][position - 1]["kind"]
                    || op["constructors"] != usize::from(arm == "fresh-selector")
                {
                    return Err("generation/kind/constructor count differs".into());
                }
                if numeric(op, "constructor_cpu_us")? > numeric(op, "sender_cpu_us")?
                    || numeric(op, "constructor_wall_us")? > numeric(op, "wall_us")?
                {
                    return Err("constructor interval exceeds operation".into());
                }
                if is_selector {
                    validate_selection(
                        &op["selection"],
                        &op["mode"],
                        op["body_bytes"].as_u64().ok_or("body")?,
                        m["bytes"].as_u64().ok_or("len")?,
                        if mixed(p) {
                            matches!(position, 1 | 3)
                        } else {
                            position != 5
                        },
                        publication && position >= 6,
                    )?;
                    wan::parse_id(
                        op["normalized_frames_sha256"]
                            .as_str()
                            .ok_or("normalized transcript")?,
                    )?;
                } else if !op["selection"].is_null()
                    || !op["normalized_frames_sha256"].is_null()
                    || !matches!(op["mode"].as_str(), Some("Raw" | "ZstdWholeV1"))
                {
                    return Err("Whole control receipt differs".into());
                }
                target_rows.push(op.clone());
                generations.push(g.clone());
            }
            let mut tagged = op.clone();
            tagged
                .as_object_mut()
                .unwrap()
                .extend(s.as_object().unwrap().clone());
            operations.push(tagged);
        }
        let calculated = session_totals(
            &r[2],
            &r[3],
            if publication { Some(&r[14]) } else { None },
            &target_rows,
            &generations,
            &r[outer],
        )?;
        if calculated != r[complete]["totals"] {
            return Err("cumulative phase accounting corrupted".into());
        }
        let mut t = s.clone();
        t["totals"] = calculated;
        t["setup"] = r[2].clone();
        t["initial_generation"] = r[1].clone();
        t["connections"] = json!(connections.len());
        t["reused_connections"] = json!(connections.len() < 11 + extra);
        t["memory"] = r[outer].clone();
        totals.push(t);
    }
    for large in [false, true] {
        for position in 1..=10 {
            let get = |arm: &str| {
                operations
                    .iter()
                    .find(|r| r["large"] == large && r["arm"] == arm && r["position"] == position)
                    .ok_or("paired operation missing")
            };
            let a = get(if mixed(p) {
                "baseline-selector"
            } else {
                "fresh-selector"
            })?;
            let b = get(if mixed(p) {
                "candidate-selector"
            } else {
                "persistent-selector"
            })?;
            for key in [
                "bytes",
                "object",
                "sha256",
                "mode",
                "body_bytes",
                "request_body_bytes",
                "response_body_bytes",
                "selection",
                "normalized_frames_sha256",
            ] {
                if a[key] != b[key] {
                    return Err(
                        format!("fresh/persistent exact winning parity differs: {key}").into(),
                    );
                }
            }
        }
    }
    Ok((operations, totals))
}

fn paired_ratios(values: Vec<f64>) -> Result<Value> {
    if values.is_empty() || values.iter().any(|v| !v.is_finite() || *v <= 0.0) {
        return Err("positive finite paired ratios required".into());
    }
    let geometric_mean = (values.iter().map(|v| v.ln()).sum::<f64>() / values.len() as f64).exp();
    Ok(
        json!({"pairs":values.len(),"candidate_wins":values.iter().filter(|v|**v<1.0).count(),"geometric_mean":geometric_mean,"distribution":dist(values.clone()),"ratios":values}),
    )
}
fn unrelated_cpu_gates(replacement: &[f64], warm: &[f64], lifecycle: &[f64]) -> Result<Value> {
    if [replacement, warm, lifecycle]
        .iter()
        .any(|v| v.len() != 10 || v.iter().any(|n| !n.is_finite() || *n <= 0.0))
    {
        return Err("ten finite positive paired ratios required".into());
    }
    let replacement = paired_ratios(replacement.to_vec())?;
    let warm = paired_ratios(warm.to_vec())?;
    let lifecycle = paired_ratios(lifecycle.to_vec())?;
    let a = numeric(&replacement["distribution"], "median")? <= 0.95;
    let b = numeric(&warm["distribution"], "median")? <= 1.05;
    let c = numeric(&lifecycle["distribution"], "median")? <= 1.05;
    Ok(
        json!({"replacement":replacement,"warm":warm,"lifecycle":lifecycle,"replacement_threshold":0.95,"warm_threshold":1.05,"lifecycle_threshold":1.05,"replacement_passed":a,"warm_passed":b,"lifecycle_passed":c,"passed":a&&b&&c}),
    )
}
fn aggregate(operations: &[Value], totals: &[Value], count: usize, local: bool) -> Result<Value> {
    let is_mixed = totals.iter().any(|s| s["arm"] == "candidate-selector");
    let candidate = if is_mixed {
        "candidate-selector"
    } else {
        "persistent-selector"
    };
    let baseline = if is_mixed {
        "baseline-selector"
    } else {
        "fresh-selector"
    };
    if operations.len() != count * if is_mixed { 70 } else { 66 } || totals.len() != count * 6 {
        return Err("campaign operation count differs".into());
    }
    let mut groups = Vec::new();
    let mut lifecycle = Vec::new();
    let mut pairs = Vec::new();
    let mut gates = Vec::new();
    for large in [false, true] {
        for arm in if is_mixed { MIXED_ARMS } else { ARMS } {
            let mut views = [
                "seed",
                "first-use",
                "later-use",
                "localized",
                "escape",
                "all-updates",
            ]
            .map(str::to_owned)
            .to_vec();
            if is_mixed {
                if arm != "whole" {
                    views.push("schema_seed".into());
                }
                views.extend((1..=10).map(|p| format!("position-{p}")));
            }
            for view in views {
                let selected = operations
                    .iter()
                    .filter(|r| {
                        r["large"] == large
                            && r["arm"] == arm
                            && match view.as_str() {
                                "seed" => r["phase"] == "seed",
                                "schema_seed" => r["phase"] == "schema_seed",
                                "first-use" => r["position"] == 1,
                                "later-use" => r["position"].as_u64().is_some_and(|p| p >= 2),
                                "localized" => {
                                    r["phase"] == "target"
                                        && if is_mixed {
                                            r["position"] == 1 || r["position"] == 3
                                        } else {
                                            r["position"] != 5
                                        }
                                }
                                "escape" => r["position"] == if is_mixed { 2 } else { 5 },
                                v if v.starts_with("position-") => {
                                    r["position"].as_u64() == v[9..].parse::<u64>().ok()
                                }
                                _ => r["phase"] == "target",
                            }
                    })
                    .collect::<Vec<_>>();
                let mut d = describe(&selected)?;
                d["large"] = json!(large);
                d["arm"] = json!(arm);
                d["view"] = json!(view);
                groups.push(d);
            }
            let group = totals
                .iter()
                .filter(|r| r["large"] == large && r["arm"] == arm)
                .collect::<Vec<_>>();
            if group.len() != count {
                return Err("lifecycle pairs incomplete".into());
            }
            let mut d = json!({"large":large,"arm":arm,"pairs":count});
            for key in group[0]["totals"].as_object().ok_or("totals")?.keys() {
                d[key] = dist(
                    group
                        .iter()
                        .map(|r| numeric(&r["totals"], key))
                        .collect::<Result<Vec<_>>>()?,
                );
            }
            for key in ["cpu_us", "wall_us", "retained_bytes", "retained_charge"] {
                d[format!("setup_{key}")] = dist(
                    group
                        .iter()
                        .map(|r| numeric(&r["setup"], key))
                        .collect::<Result<Vec<_>>>()?,
                );
            }
            for key in [
                "peak_charge",
                "retained_bytes",
                "captured_frame_capacity",
                "external_capacity_bound",
            ] {
                d[format!("memory_{key}")] = dist(
                    group
                        .iter()
                        .map(|r| numeric(&r["memory"], key))
                        .collect::<Result<Vec<_>>>()?,
                );
            }
            d["observed_connections"] = dist(
                group
                    .iter()
                    .map(|r| numeric(r, "connections"))
                    .collect::<Result<Vec<_>>>()?,
            );
            lifecycle.push(d);
        }
        for denominator in [baseline, "whole"] {
            for metric in [
                "setup_and_updates_sender_cpu_us",
                "setup_and_updates_wall_us",
                "paid_lifecycle_cpu_us",
                "paid_lifecycle_wall_us",
                "outer_cpu_subtotal_us",
                "outer_wall_us",
                "body_bytes",
            ] {
                let mut values = Vec::new();
                for trial in 0..count {
                    let get = |arm: &str| {
                        totals
                            .iter()
                            .find(|r| r["trial"] == trial && r["large"] == large && r["arm"] == arm)
                            .ok_or("paired trial missing")
                    };
                    values.push(
                        numeric(&get(candidate)?["totals"], metric)?
                            / numeric(&get(denominator)?["totals"], metric)?,
                    );
                }
                let mut row = paired_ratios(values)?;
                row["large"] = json!(large);
                row["metric"] = json!(metric);
                row["denominator"] = json!(denominator);
                if !is_mixed
                    && denominator == "fresh-selector"
                    && metric == "setup_and_updates_sender_cpu_us"
                {
                    let median = numeric(&row["distribution"], "median")?;
                    gates.push(json!({"large":large,"pairs":count,"threshold":1.05,"median_ratio":median,"passed":median<=1.05,"registered_gate":local}));
                }
                pairs.push(row);
            }
        }
        if is_mixed {
            let mut replacement = Vec::new();
            let mut warm = Vec::new();
            let mut lifecycle = Vec::new();
            for trial in 0..count {
                let total = |arm| -> Result<f64> {
                    numeric(
                        &totals
                            .iter()
                            .find(|r| r["trial"] == trial && r["large"] == large && r["arm"] == arm)
                            .ok_or("paired lifecycle missing")?["totals"],
                        "setup_and_updates_sender_cpu_us",
                    )
                };
                let op = |arm, position| -> Result<&Value> {
                    operations
                        .iter()
                        .find(|r| {
                            r["trial"] == trial
                                && r["large"] == large
                                && r["arm"] == arm
                                && r["position"] == position
                                && r["phase"] == "target"
                        })
                        .ok_or_else(|| "paired target missing".into())
                };
                replacement.push(
                    numeric(op(candidate, 2)?, "sender_cpu_us")?
                        / numeric(op(baseline, 2)?, "sender_cpu_us")?,
                );
                warm.push(
                    (numeric(op(candidate, 1)?, "sender_cpu_us")?
                        + numeric(op(candidate, 3)?, "sender_cpu_us")?)
                        / (numeric(op(baseline, 1)?, "sender_cpu_us")?
                            + numeric(op(baseline, 3)?, "sender_cpu_us")?),
                );
                lifecycle.push(total(candidate)? / total(baseline)?);
            }
            let mut gate = if local {
                unrelated_cpu_gates(&replacement, &warm, &lifecycle)?
            } else {
                json!({"replacement":paired_ratios(replacement)?,"warm":paired_ratios(warm)?,"lifecycle":paired_ratios(lifecycle)?,"passed":Value::Null})
            };
            gate["large"] = json!(large);
            gate["registered_gate"] = json!(local);
            gates.push(gate);
            for denominator in [baseline, "whole"] {
                for position in 1..=10 {
                    for metric in [
                        "sender_cpu_us",
                        "cpu_us",
                        "wall_us",
                        "exchange_wall_us",
                        "body_bytes",
                    ] {
                        let mut values = Vec::new();
                        for trial in 0..count {
                            let get = |arm| {
                                operations
                                    .iter()
                                    .find(|r| {
                                        r["trial"] == trial
                                            && r["large"] == large
                                            && r["arm"] == arm
                                            && r["position"] == position
                                            && r["phase"] == "target"
                                    })
                                    .ok_or("paired position missing")
                            };
                            values.push(
                                numeric(get(candidate)?, metric)?
                                    / numeric(get(denominator)?, metric)?,
                            );
                        }
                        let mut row = paired_ratios(values)?;
                        row["large"] = json!(large);
                        row["position"] = json!(position);
                        row["denominator"] = json!(denominator);
                        row["metric"] = json!(metric);
                        pairs.push(row);
                    }
                }
            }
        }
    }
    let advance = local && count == 10 && gates.iter().all(|v| v["passed"] == true);
    let mut result = json!({"status":"EXACT_PAIRED_TRANSFERS_PASS","profile":"persistent-session.v1","dose":if local{"local-comparison"}else{"six-pair-wan-pilot"},"paired_trials_per_size":count,"operations":operations.len(),"seeds":count*6,"targets":count*60,"sessions":totals.len(),"schema_fits":count*4,"discarded":0,"metric_scope":"HTTP bodies in both directions; sender process CPU plus disjoint receiver handler CPU; exchange is a nested interval; outer includes fixture generation; paid lifecycle includes the equal-state seed that Whole does not require","target_distributions":groups,"lifecycle_distributions":lifecycle,"paired_lifecycle_ratios":pairs,"local_gates":gates,"advance_to_wan":advance,"session_totals":totals});
    if is_mixed {
        result["profile"] = json!("unrelated-cpu.v1");
        result["schema_seeds"] = json!(count * 4);
        result["metric_scope"]=json!("HTTP bodies both directions; disjoint sender and receiver CPU; nested exchange; outer includes generation and all fit/context/base/artifact costs. setup_and_updates includes actual schema publication. Whole equal-state base seed is unnecessary for its codec.");
    }
    Ok(result)
}
fn summary(paths: &[String], local: bool) -> Result<Value> {
    let count = if local { 10 } else { 6 };
    if paths.len() != count * 3 {
        return Err("complete registered shard triples required".into());
    }
    let mut ops = Vec::new();
    let mut totals = Vec::new();
    let mut bindings = Vec::new();
    let mut common = None;
    for (index, triplet) in paths.chunks_exact(3).enumerate() {
        let (p, digest) = read_plan(Path::new(&triplet[0]), true)?;
        if p["shard"] != index
            || p["dose"]
                != if local {
                    "local-comparison"
                } else {
                    "wan-pilot"
                }
        {
            return Err("shard order/dose differs".into());
        }
        let bytes = fs::read(&triplet[1])?;
        let checks = fs::read(&triplet[2])?;
        let raw = rows(&bytes)?;
        let verified = rows(&checks)?;
        let mut binding = json!({"run":p["run"],"source_commit":p["source_commit"],"identity":raw.first().ok_or("empty observations")?["identity"],"receiver_executable_sha256":raw[0]["receiver_executable_sha256"]});
        if mixed(&p) {
            binding["baseline_identity"] = raw[0]["baseline_identity"].clone();
            binding["source_profiles"] = p["source_profiles"].clone();
        }
        if common.as_ref().is_some_and(|c| *c != binding) {
            return Err("campaign source/executable changed between shards".into());
        }
        common = Some(binding);
        let (o, t) = validate_receipts(&p, &digest, &raw, &verified)?;
        ops.extend(o);
        totals.extend(t);
        bindings.push(json!({"shard":index,"plan_sha256":digest,"raw_sha256":sha(&bytes),"verification_sha256":sha(&checks)}));
    }
    let mut result = aggregate(&ops, &totals, count, local)?;
    result["binding"] = common.ok_or("no binding")?;
    result["shards"] = json!(bindings);
    Ok(result)
}
fn local_receiver(plan_path: &Path, cache: &Path) -> Result<()> {
    let (p, _) = read_plan(plan_path, false)?;
    if p["dose"] != "local-comparison" || cache.exists() {
        return Err("fresh local receiver required".into());
    }
    let token = std::env::var("SPIRALCORE_WAN_TOKEN")?;
    let scopes = p["allowed_scopes"]
        .as_array()
        .ok_or("scopes")?
        .iter()
        .map(|s| wan::parse_id(s.as_str().unwrap_or("")))
        .collect::<spiralcore_transfer::Result<BTreeSet<_>>>()?;
    let mut endpoint = Endpoint::open_with_policy(
        cache,
        wan::limits(),
        BenchAuthority,
        ColdGraphPolicy::Disabled,
    )?;
    endpoint.set_codec_cache_enabled(true)?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    println!("READY http://{}", listener.local_addr()?);
    std::io::stdout().flush()?;
    wan::serve_observed_configured(
        listener,
        endpoint,
        &token,
        &scopes,
        wan::ServeConfig {
            duration: Duration::from_secs(600),
            max_requests: p["operations"].as_u64().ok_or("operations")? as usize,
            connection_policy: wan::ConnectionPolicy::KeepAlive {
                max_requests_per_connection: 32,
                idle_timeout: Duration::from_millis(500),
            },
        },
        |o| {
            let c = o.codec_cache;
            let row = json!({"event":"wan_request","request_ordinal":o.request_ordinal,"connection_ordinal":o.connection_ordinal,"request_on_connection":o.request_on_connection,"scope":hex(&o.scope),"http_status":o.http_status,"codec_cache":{"hits":c.hits,"misses":c.misses,"preparations":c.preparations,"evictions":c.evictions,"entries":c.entries,"resident_bytes":c.resident_bytes}});
            let mut out = std::io::stderr().lock();
            serde_json::to_writer(&mut out, &row)
                .map_err(|_| spiralcore_transfer::Error::Storage)?;
            writeln!(out)?;
            out.flush()?;
            Ok(())
        },
    )?;
    Ok(())
}
fn disk_floor(path: &Path) -> Result<()> {
    let output = Command::new("df").arg("-Pk").arg(path).output()?;
    let text = std::str::from_utf8(&output.stdout)?;
    let available = text
        .lines()
        .last()
        .and_then(|line| line.split_whitespace().nth(3))
        .ok_or("disk capacity unavailable")?
        .parse::<u64>()?;
    if !output.status.success() || available < 4 * 1024 * 1024 {
        return Err("less than 4 GiB free before store launch".into());
    }
    Ok(())
}
fn local_shard(exe: &Path, baseline: Option<&Path>, dir: &Path, p: &Value) -> Result<[String; 3]> {
    disk_floor(dir.parent().ok_or("parent missing")?)?;
    fs::create_dir(dir)?;
    let plan_path = dir.join("plan.json");
    write_row(&mut fresh(&plan_path)?, p)?;
    let mut token = [0u8; 32];
    File::open("/dev/urandom")?.read_exact(&mut token)?;
    let token = hex(&token);
    let log = dir.join("receiver.jsonl");
    let cache = dir.join("cache");
    let receiver = sha(&fs::read(exe)?);
    let child = Command::new(exe)
        .arg(format!("{}-local-receiver", prefix(p)))
        .arg(&plan_path)
        .arg(&cache)
        .env("SPIRALCORE_WAN_TOKEN", &token)
        .stdout(Stdio::piped())
        .stderr(Stdio::from(fresh(&log)?))
        .spawn()?;
    let mut child = warm::ChildGuard(child);
    let mut ready = String::new();
    BufReader::new(child.0.stdout.take().ok_or("stdout missing")?).read_line(&mut ready)?;
    let url = ready
        .trim()
        .strip_prefix("READY ")
        .ok_or("receiver not ready")?;
    let raw = dir.join("paired.jsonl");
    let mut command = Command::new(exe);
    command.arg(format!("{}-paired", prefix(p))).arg(&plan_path);
    if mixed(p) {
        command.arg(baseline.ok_or("baseline path missing")?);
    }
    let output = command
        .arg(exe)
        .arg(url)
        .arg(&receiver)
        .arg(&raw)
        .env("SPIRALCORE_WAN_TOKEN", &token)
        .output()?;
    save(&dir.join("driver.stdout"), &output.stdout)?;
    save(&dir.join("driver.stderr"), &output.stderr)?;
    write_row(
        &mut fresh(&dir.join("driver-status.json"))?,
        &json!({"success":output.status.success(),"exit_code":output.status.code()}),
    )?;
    if !output.status.success() {
        return Err("local session failed; store preserved".into());
    }
    let status = child.0.wait()?;
    write_row(
        &mut fresh(&dir.join("receiver-status.json"))?,
        &json!({"success":status.success(),"exit_code":status.code()}),
    )?;
    if !status.success() {
        return Err("receiver failed; store preserved".into());
    }
    let verified = dir.join("verification.jsonl");
    verify(&plan_path, &cache, &verified, &log, &receiver)?;
    validate_receipts(
        p,
        &sha(&fs::read(&plan_path)?),
        &rows(&fs::read(&raw)?)?,
        &rows(&fs::read(&verified)?)?,
    )?;
    fs::remove_dir_all(&cache)?;
    write_row(
        &mut fresh(&dir.join("cleanup.json"))?,
        &json!({"owned_store_removed":true,"only_after_exact_reopen":true,"objects":p["operations"]}),
    )?;
    Ok([plan_path, raw, verified].map(|p| p.to_string_lossy().into_owned()))
}
fn local(exe: &Path, baseline: Option<&Path>, dir: &Path, run: &str) -> Result<()> {
    let source = std::env::var(if baseline.is_some() {
        "SPIRALCORE_UNRELATED_SOURCE"
    } else {
        "SPIRALCORE_PERSISTENT_SOURCE"
    })?;
    fs::create_dir(dir)?;
    let mut paths = Vec::new();
    let result = (|| -> Result<()> {
        for shard in 0..10 {
            let p = make_plan(run, shard, &source, true, baseline.is_some())?;
            paths.extend(local_shard(
                exe,
                baseline,
                &dir.join(format!("shard-{shard}")),
                &p,
            )?);
        }
        write_row(
            &mut fresh(&dir.join("summary.json"))?,
            &summary(&paths, true)?,
        )
    })();
    if let Err(e) = result {
        write_row(
            &mut fresh(&dir.join("failure.json"))?,
            &json!({"error":e.to_string(),"completed_shards":paths.len()/3,"no_retry":true}),
        )?;
        return Err(e);
    }
    Ok(())
}
pub(super) fn command(args: &[String]) -> Result<()> {
    match args.get(1).map(String::as_str){
        Some("persistent-identity") if args.len()==2=>{println!("{}",identity()?);Ok(())},
        Some("persistent-plan") if args.len()==5=>write_row(&mut fresh(Path::new(&args[4]))?,&persistent_plan(&args[2],args[3].parse()?,&std::env::var("SPIRALCORE_PERSISTENT_SOURCE")?,false)?),
        Some("persistent-session") if args.len()==9=>session(args),
        Some("persistent-paired") if args.len()==7=>paired(Path::new(&args[2]),Path::new(&args[3]),None,&args[4],&args[5],Path::new(&args[6])),
        Some("persistent-verify") if args.len()==5=>verify(Path::new(&args[2]),Path::new(&args[3]),Path::new(&args[4]),Path::new(&std::env::var("SPIRALCORE_PERSISTENT_OBSERVATIONS")?),&std::env::var("SPIRALCORE_PERSISTENT_RECEIVER_SHA256")?),
        Some("persistent-summary") if args.len()==21=>write_row(&mut fresh(Path::new(&args[20]))?,&summary(&args[2..20],false)?),
        Some("persistent-local-summary") if args.len()==33=>write_row(&mut fresh(Path::new(&args[32]))?,&summary(&args[2..32],true)?),
        Some("persistent-local") if args.len()==5=>local(Path::new(&args[2]),None,Path::new(&args[3]),&args[4]),
        Some("persistent-local-receiver") if args.len()==4=>local_receiver(Path::new(&args[2]),Path::new(&args[3])),
        Some("unrelated-identity") if args.len()==2=>{println!("{}",identity()?);Ok(())},
        Some("unrelated-plan") if args.len()==5=>write_row(&mut fresh(Path::new(&args[4]))?,&unrelated_plan(&args[2],args[3].parse()?,&std::env::var("SPIRALCORE_UNRELATED_SOURCE")?,false)?),
        Some("unrelated-session") if args.len()==9=>session(args),
        Some("unrelated-paired") if args.len()==8=>paired(Path::new(&args[2]),Path::new(&args[4]),Some(Path::new(&args[3])),&args[5],&args[6],Path::new(&args[7])),
        Some("unrelated-verify") if args.len()==5=>verify(Path::new(&args[2]),Path::new(&args[3]),Path::new(&args[4]),Path::new(&std::env::var("SPIRALCORE_UNRELATED_OBSERVATIONS")?),&std::env::var("SPIRALCORE_UNRELATED_RECEIVER_SHA256")?),
        Some("unrelated-summary") if args.len()==21=>write_row(&mut fresh(Path::new(&args[20]))?,&summary(&args[2..20],false)?),
        Some("unrelated-local-summary") if args.len()==33=>write_row(&mut fresh(Path::new(&args[32]))?,&summary(&args[2..32],true)?),
        Some("unrelated-local") if args.len()==6=>local(Path::new(&args[3]),Some(Path::new(&args[2])),Path::new(&args[4]),&args[5]),
        Some("unrelated-local-receiver") if args.len()==4=>local_receiver(Path::new(&args[2]),Path::new(&args[3])),
        _=>Err("usage: persistent-identity | persistent-plan RUN SHARD NEW_PLAN | persistent-paired PLAN EXE URL RECEIVER_SHA NEW_JSONL | persistent-verify PLAN STOPPED_CACHE NEW_JSONL | persistent-summary SIX_PLAN_RAW_VERIFY_TRIPLES NEW_JSON | persistent-local EXE NEW_DIRECTORY RUN | persistent-local-summary TEN_PLAN_RAW_VERIFY_TRIPLES NEW_JSON".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unrelated_sequence_keeps_exact_warm_edits_and_held_out_shapes() {
        let mut base = seed(0, false);
        let mut ids = BTreeSet::from([object_id(&base)]);
        for position in 1..=10 {
            let target = mixed_next(0, false, position, &base).unwrap();
            assert_eq!(target.len(), 1 << 20);
            assert!(ids.insert(object_id(&target)));
            if matches!(position, 1 | 3 | 5 | 10) {
                assert_eq!(target.iter().zip(&base).filter(|(a, b)| a != b).count(), 32);
                let middle = target.len() / 2;
                assert!(target[middle..middle + 32]
                    .iter()
                    .zip(&base[middle..middle + 32])
                    .all(|(a, b)| a ^ b == 0xa5));
            }
            if position == 4 {
                assert!(target.iter().all(|b| *b == 0));
            }
            if (6..=9).contains(&position) {
                assert!(target
                    .windows(b"held-out persistent telemetry value ".len())
                    .any(|w| w == b"held-out persistent telemetry value "));
                assert!(target.iter().filter(|b| **b == b'\n').count() < 65_536);
            }
            base = target;
        }
        let large = mixed_next(0, true, 6, &vec![0; 16 << 20]).unwrap();
        assert_eq!(large.len(), 16 << 20);
        assert!(large.iter().filter(|b| **b == b'\n').count() < 65_536);
        assert!(mixed_next(10, false, 1, &base).is_err());
    }
    #[test]
    fn unrelated_plan_charges_real_artifact_publication_and_fixed_dose() {
        let plan = unrelated_plan("mixed-red", 0, &"1".repeat(40), true).unwrap();
        assert_eq!(plan["sessions"].as_array().unwrap().len(), 6);
        assert_eq!(plan["targets"], 60);
        assert_eq!(plan["seeds"], 6);
        assert_eq!(plan["schema_seeds"], 4);
        assert_eq!(plan["operations"], 70);
        assert_eq!(plan["schema_fits"], 4);
        assert_eq!(plan["schema_publication_before_position"], 6);
        assert!(plan["conservative_storage_bound"].as_u64().unwrap() < 768 << 20);
        let arms = plan["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["arm"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            arms,
            BTreeSet::from(["baseline-selector", "candidate-selector", "whole"])
        );
    }
    #[test]
    fn unrelated_cpu_gates_preserve_negative_controls_and_require_all_pairs() {
        let good = unrelated_cpu_gates(&[0.9; 10], &[1.0; 10], &[1.0; 10]).unwrap();
        assert_eq!(good["passed"], true);
        let slower = unrelated_cpu_gates(&[0.9; 10], &[1.06; 10], &[1.0; 10]).unwrap();
        assert_eq!(slower["passed"], false);
        assert_eq!(slower["replacement_passed"], true);
        assert_eq!(slower["warm_passed"], false);
        assert!(unrelated_cpu_gates(&[0.9; 9], &[1.0; 10], &[1.0; 10]).is_err());
        assert!(unrelated_cpu_gates(&[f64::NAN; 10], &[1.0; 10], &[1.0; 10]).is_err());
    }
    // Complete synthetic receipts qualify strict joins/accounting; they are not
    // performance data and do not substitute for stopped-store byte comparison.
    fn receipts(p: &Value) -> (Vec<Value>, Vec<Value>) {
        let digest = "d".repeat(64);
        let id = json!({"sources":p["sources"],"executable_sha256":"a".repeat(64)});
        let receiver = "b".repeat(64);
        let mut raw = vec![
            json!({"event":"registration","identity":id,"source_commit":p["source_commit"],"plan_sha256":digest,"run":p["run"],"shard":p["shard"],"dose":p["dose"],"receiver_executable_sha256":receiver}),
        ];
        let baseline_id =
            json!({"sources":role_pins(p,"baseline-selector"),"executable_sha256":"9".repeat(64)});
        if mixed(p) {
            raw[0]["baseline_identity"] = baseline_id.clone();
        }
        let mut verified = vec![
            json!({"event":"verification_registration","identity":id,"source_commit":p["source_commit"],"plan_sha256":digest,"receiver_executable_sha256":receiver,"observation_sha256":"c".repeat(64)}),
        ];
        let mut obs = Vec::new();
        for (index, s) in p["sessions"].as_array().unwrap().iter().enumerate() {
            let selector = s["arm"] != "whole";
            let fresh = s["arm"] == "fresh-selector";
            let publication = mixed(p) && selector;
            let own_id = if s["arm"] == "baseline-selector" {
                &baseline_id
            } else {
                &id
            };
            let m = expected(p, s["large"].as_bool().unwrap()).unwrap();
            let setup = json!({"event":"context_setup","cpu_us":10,"wall_us":10,"retained_bytes":0,"retained_charge":4096,"schema_fits":usize::from(selector),"training_samples":if selector{512}else{0},"artifact":if selector{p["artifact"].clone()}else{Value::Null},"constructors":usize::from(!fresh),"error":Value::Null});
            let mut r = vec![
                json!({"event":"session_registration","identity":own_id,"plan_sha256":digest,"session":s,"receiver_executable_sha256":receiver}),
                json!({"event":"initial_generation","cpu_us":1,"wall_us":1}),
                setup.clone(),
            ];
            let mut update = Vec::new();
            let mut generation = Vec::new();
            let mut seed_row = Value::Null;
            let mut schema_row = None;
            for slot in 0..(11 + usize::from(publication)) {
                let artifact = publication && slot == 6;
                let position = slot - usize::from(publication && slot >= 6);
                let expected = if artifact {
                    &p["artifact"]
                } else if position == 0 {
                    &m["base"]
                } else {
                    &m["targets"][position - 1]["expected"]
                };
                let len = expected["bytes"].as_u64().unwrap();
                let localized = !artifact
                    && if mixed(p) {
                        matches!(position, 1 | 3)
                    } else {
                        position != 0 && position != 5
                    };
                let rawcost = warm::raw_cost(len);
                let sparse = selector && localized;
                let body = if sparse { 600 } else { rawcost };
                let phase = if artifact {
                    "schema_seed"
                } else if position == 0 {
                    "seed"
                } else {
                    "target"
                };
                let pos = if position == 0 || artifact {
                    Value::Null
                } else {
                    json!(position)
                };
                let mut op = json!({"event":"operation","phase":phase,"position":pos,"bytes":len,"object":expected["object"],"sha256":expected["sha256"],"sender_cpu_us":30,"receiver_cpu_us":3,"cpu_us":33,"wall_us":50,"exchange_cpu_us":5,"exchange_wall_us":40,"receiver_wall_us":4,"request_body_bytes":body-100,"response_body_bytes":100,"body_bytes":body,"batches":1,"mode":if sparse{"SparseV1"}else{"Raw"},"error":Value::Null,"peak_charge":400u64<<20});
                if position > 0 && !artifact {
                    let g = json!({"event":"generation","position":position,"cpu_us":5,"wall_us":5,"input_capacity":len});
                    r.push(g.clone());
                    generation.push(g);
                    op["kind"] = m["targets"][position - 1]["kind"].clone();
                    op["constructors"] = json!(usize::from(fresh));
                    op["constructor_cpu_us"] = json!(usize::from(fresh));
                    op["constructor_wall_us"] = json!(usize::from(fresh));
                    op["retained_bytes"] = json!(len);
                    op["retained_charge"] = json!(len + 4096);
                    if selector {
                        let setup = if publication && position >= 6 { 0 } else { 670 };
                        let raw = json!({"mode":"Raw","target_http_body_bytes":rawcost,"setup_http_body_bytes":0,"total_http_body_bytes":rawcost});
                        op["selection"] = if sparse {
                            json!({"selected_mode":"SparseV1","needs_schema_seed":false,"candidates":[raw,{"mode":"SparseV1","target_http_body_bytes":600,"setup_http_body_bytes":0,"total_http_body_bytes":600}],"pruned":[{"mode":"ZstdWholeV1","target_http_body_lower_bound":800,"setup_http_body_bytes":0,"total_http_body_lower_bound":800},{"mode":"SchemaZstdV1","target_http_body_lower_bound":547,"setup_http_body_bytes":setup,"total_http_body_lower_bound":547+setup}]})
                        } else {
                            json!({"selected_mode":"Raw","needs_schema_seed":false,"candidates":[raw,{"mode":"ZstdWholeV1","target_http_body_bytes":rawcost+1,"setup_http_body_bytes":0,"total_http_body_bytes":rawcost+1},{"mode":"SchemaZstdV1","target_http_body_bytes":rawcost+1,"setup_http_body_bytes":setup,"total_http_body_bytes":rawcost+1+setup}],"pruned":[]})
                        };
                        op["normalized_frames_sha256"] = json!("e".repeat(64));
                    } else {
                        op["selection"] = Value::Null;
                        op["normalized_frames_sha256"] = Value::Null;
                    }
                    update.push(op.clone());
                } else if artifact {
                    schema_row = Some(op.clone());
                } else {
                    seed_row = op.clone();
                }
                r.push(op);
                verified.push(json!({"event":"receiver_verification","scope":s["scope"],"phase":phase,"position":pos,"expected":expected,"exact":true,"actual_bytes":len,"actual_sha256":expected["sha256"],"error":Value::Null}));
                obs.push(json!({"event":"wan_request","request_ordinal":obs.len()+1,"connection_ordinal":index+1,"request_on_connection":slot+1,"scope":s["scope"],"http_status":200,"codec_cache":{"hits":0,"misses":0,"preparations":0,"evictions":0,"entries":0,"resident_bytes":0}}));
            }
            let outer = json!({"event":"session_outer","cpu_us":if mixed(p){440}else{400},"wall_us":700,"captured_frame_capacity":0,"external_metadata_reserve":8<<20,"external_capacity_bound":8<<20,"peak_charge":400u64<<20,"retained_bytes":0,"retained_charge":4096});
            let total = session_totals(
                &setup,
                &seed_row,
                schema_row.as_ref(),
                &update,
                &generation,
                &outer,
            )
            .unwrap();
            r.push(outer);
            r.push(json!({"event":"session_complete","seeds":1,"targets":10,"totals":total}));
            raw.push(json!({"event":"session","index":index,"session":s,"rows":r,"raw_sha256":"f".repeat(64)}));
        }
        raw.push(json!({"event":"complete","sessions":6,"operations":p["operations"]}));
        verified[0]["observations"] = json!(obs);
        verified
            .push(json!({"event":"verification_complete","checked":p["operations"],"exact":true}));
        (raw, verified)
    }
    #[test]
    fn unrelated_complete_receipts_require_paid_publication_roles_and_negative_gates() {
        let p = unrelated_plan("mixed-receipt", 0, &"1".repeat(40), true).unwrap();
        let (raw, checked) = receipts(&p);
        let (ops, totals) = validate_receipts(&p, &"d".repeat(64), &raw, &checked).unwrap();
        assert_eq!(ops.len(), 70);
        let index = p["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .position(|r| r["arm"] == "candidate-selector")
            .unwrap()
            + 1;
        let mut wrong = raw.clone();
        wrong[index]["rows"].as_array_mut().unwrap().remove(14);
        assert!(validate_receipts(&p, &"d".repeat(64), &wrong, &checked).is_err());
        let mut wrong = raw.clone();
        wrong[index]["rows"][14]["body_bytes"] = json!(0);
        assert!(validate_receipts(&p, &"d".repeat(64), &wrong, &checked).is_err());
        let mut wrong = raw.clone();
        wrong[0]["baseline_identity"]["sources"]["schema"] = json!("0".repeat(64));
        assert!(validate_receipts(&p, &"d".repeat(64), &wrong, &checked).is_err());
        let mut wrong = raw.clone();
        wrong[index]["rows"][16]["normalized_frames_sha256"] = json!("0".repeat(64));
        assert!(validate_receipts(&p, &"d".repeat(64), &wrong, &checked).is_err());
        let mut all_ops = Vec::new();
        let mut all_totals = Vec::new();
        for trial in 0..10 {
            for mut r in ops.clone() {
                r["trial"] = json!(trial);
                all_ops.push(r);
            }
            for mut r in totals.clone() {
                r["trial"] = json!(trial);
                all_totals.push(r);
            }
        }
        let result = aggregate(&all_ops, &all_totals, 10, true).unwrap();
        assert_eq!(result["operations"], 700);
        assert_eq!(result["advance_to_wan"], false);
        for r in &mut all_ops {
            if r["arm"] == "candidate-selector" && r["position"] == 2 {
                r["sender_cpu_us"] = json!(27);
            }
        }
        assert_eq!(
            aggregate(&all_ops, &all_totals, 10, true).unwrap()["advance_to_wan"],
            true
        );
        assert!(aggregate(&all_ops[..699], &all_totals, 10, true).is_err());
    }
    #[test]
    fn unrelated_mixed_session_publishes_schema_and_reopens_all_exact_objects() {
        let path = std::env::temp_dir().join(format!(
            "spiral-mixed-qualification-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let ctx = wan::context([197; 32]);
        let mut channel = Channel::open(&path, [197; 32]).unwrap();
        let mut session = WarmSession::new(&ctx, ARTIFACT).unwrap();
        session
            .seed_base(&ctx, seed(0, false), [1; 32], |f| channel.exchange(f))
            .unwrap();
        for position in 1..=10 {
            if position == 6 {
                session
                    .seed_schema(&ctx, [12; 32], |f| channel.exchange(f))
                    .unwrap();
                assert!(session.schema_ready());
            }
            let target = mixed_next(0, false, position, session.base_bytes().unwrap()).unwrap();
            let before = channel.metrics.request + channel.metrics.response;
            let stats = session
                .send(&ctx, target, [position as u8 + 1; 32], false, |f| {
                    channel.exchange(f)
                })
                .unwrap();
            if matches!(position, 1 | 3) {
                assert_eq!(stats.transfer.mode, Mode::SparseV1);
                assert_eq!(
                    channel.metrics.request + channel.metrics.response - before,
                    600
                );
            }
            assert!(!stats.selection.needs_schema_seed);
        }
        drop(session);
        drop(channel);
        let mut endpoint = Endpoint::open_with_policy(
            &path,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        let mut bytes = seed(0, false);
        for position in 0..=10 {
            if position > 0 {
                bytes = mixed_next(0, false, position, &bytes).unwrap();
            }
            assert_eq!(
                endpoint.read_object(&ctx, object_id(&bytes)).unwrap(),
                bytes
            );
        }
        assert_eq!(
            endpoint.read_object(&ctx, object_id(ARTIFACT)).unwrap(),
            ARTIFACT
        );
        drop(endpoint);
        fs::remove_dir_all(path).unwrap();
    }
    #[test]
    fn persistent_complete_receipts_reject_missing_fits_costs_and_fidelity() {
        let p = persistent_plan("receipt-qualification", 0, &"1".repeat(40), true).unwrap();
        let (raw, checks) = receipts(&p);
        let digest = "d".repeat(64);
        let (ops, totals) = validate_receipts(&p, &digest, &raw, &checks).unwrap();
        assert_eq!(ops.len(), 66);
        assert_eq!(totals.len(), 6);
        let mut wrong = raw.clone();
        wrong[1]["rows"][2]["schema_fits"] = json!(9);
        assert!(validate_receipts(&p, &digest, &wrong, &checks).is_err());
        let mut wrong = raw.clone();
        wrong[1]["rows"].as_array_mut().unwrap().remove(8);
        assert!(validate_receipts(&p, &digest, &wrong, &checks).is_err());
        let mut wrong = checks.clone();
        wrong[8]["actual_sha256"] = json!("0".repeat(64));
        assert!(validate_receipts(&p, &digest, &raw, &wrong).is_err());
        let mut wrong = checks.clone();
        wrong[0]["observations"][1]["request_on_connection"] = json!(1);
        assert!(validate_receipts(&p, &digest, &raw, &wrong).is_err());
        let index = p["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .position(|s| s["arm"] == "persistent-selector")
            .unwrap()
            + 1;
        let mut wrong = raw.clone();
        wrong[index]["rows"][5]["normalized_frames_sha256"] = json!("0".repeat(64));
        assert!(validate_receipts(&p, &digest, &wrong, &checks).is_err());
        let mut wrong = raw.clone();
        wrong[index]["rows"][5]["selection"]["candidates"][1]["total_http_body_bytes"] = json!(599);
        assert!(validate_receipts(&p, &digest, &wrong, &checks).is_err());
    }
    #[test]
    fn persistent_campaign_summary_keeps_unfavorable_gate() {
        let p = persistent_plan("summary-qualification", 0, &"1".repeat(40), true).unwrap();
        let (r, v) = receipts(&p);
        let (mut ops, mut totals) = validate_receipts(&p, &"d".repeat(64), &r, &v).unwrap();
        let original_ops = ops.clone();
        let original_totals = totals.clone();
        for trial in 1..10 {
            for mut op in original_ops.clone() {
                op["trial"] = json!(trial);
                ops.push(op);
            }
            for mut t in original_totals.clone() {
                t["trial"] = json!(trial);
                totals.push(t);
            }
        }
        assert_eq!(
            aggregate(&ops, &totals, 10, true).unwrap()["advance_to_wan"],
            true
        );
        for t in &mut totals {
            if t["arm"] == "persistent-selector" && t["large"] == false {
                t["totals"]["setup_and_updates_sender_cpu_us"] = json!(500.0);
            }
        }
        let summary = aggregate(&ops, &totals, 10, true).unwrap();
        assert_eq!(summary["advance_to_wan"], false);
        assert_eq!(summary["operations"], 660);
        assert_eq!(summary["local_gates"][1]["passed"], true);
        assert!(aggregate(&ops[..659], &totals, 10, true).is_err());
    }
    #[test]
    fn persistent_rolling_objects_survive_real_serialized_store_reopen() {
        let path = std::env::temp_dir().join(format!(
            "spiral-persistent-qualification-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let scope = [99; 32];
        let ctx = wan::context(scope);
        let mut channel = Channel::open(&path, scope).unwrap();
        let mut session = WarmSession::new(&ctx, ARTIFACT).unwrap();
        let base = seed(0, false);
        session
            .seed_base(&ctx, base, [1; 32], |f| channel.exchange(f))
            .unwrap();
        for position in 1..=10 {
            let next = persistent_next(0, false, position, session.base_bytes().unwrap()).unwrap();
            let before = channel.metrics.request + channel.metrics.response;
            let stats = session
                .send(&ctx, next, [position as u8 + 1; 32], false, |f| {
                    channel.exchange(f)
                })
                .unwrap();
            if position != 5 {
                assert_eq!(stats.transfer.mode, Mode::SparseV1);
                assert_eq!(
                    channel.metrics.request + channel.metrics.response - before,
                    600
                );
            }
            assert_eq!(session.base_id(), Some(stats.transfer.object));
        }
        drop(session);
        drop(channel);
        let mut endpoint = Endpoint::open_with_policy(
            &path,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        let mut bytes = seed(0, false);
        for position in 0..=10 {
            if position > 0 {
                bytes = persistent_next(0, false, position, &bytes).unwrap();
            }
            assert_eq!(
                endpoint.read_object(&ctx, object_id(&bytes)).unwrap(),
                bytes
            );
        }
        drop(endpoint);
        fs::remove_dir_all(path).unwrap();
    }
}
