//! Verified prior-object reuse, independently measured from codec-context reuse.
use super::schema_selection::measure;
use super::schema_wan::{describe, observations, rows, METRICS_WAN};
use super::*;
use spiralcore_transfer::{schema_selection as warm_selection, wan::HttpClient, zstd_whole};
use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Read},
    net::TcpListener,
    process::{Child, Command, Stdio},
    time::Duration,
};

const ARMS_WARM: [&str; 2] = ["sparse", "whole"];
const ARMS_SELECTION: [&str; 3] = ["sparse", "selector", "whole"];
const ARTIFACT: &[u8] = include_bytes!(
    "../../../../docs/network/evidence/2026-10-30-schema-reuse/artifacts-corrected/schema.artifact"
);
fn selected_profile(p: &Value) -> bool {
    p["experiment"] == "warm-selection.v1"
}
fn prefix(p: &Value) -> &'static str {
    if selected_profile(p) {
        "warmselect"
    } else {
        "warm"
    }
}
fn arms(p: &Value) -> &'static [&'static str] {
    if selected_profile(p) {
        &ARMS_SELECTION
    } else {
        &ARMS_WARM
    }
}
fn pins() -> Value {
    let mut p = sources();
    p.as_object_mut().unwrap().remove("hardware_sha256");
    for (name, bytes) in [
        ("warm_harness", include_bytes!("warm_object.rs").as_slice()),
        (
            "measurement_helper",
            include_bytes!("schema_selection.rs").as_slice(),
        ),
        ("wan_helper", include_bytes!("schema_wan.rs").as_slice()),
        ("delta", include_bytes!("../../src/delta.rs").as_slice()),
        (
            "warm_selection",
            include_bytes!("../../src/schema_selection.rs").as_slice(),
        ),
        (
            "whole_zstd",
            include_bytes!("../../src/zstd_whole.rs").as_slice(),
        ),
        ("wan", include_bytes!("../../src/wan.rs").as_slice()),
        (
            "wan_server",
            include_bytes!("../../src/bin/spiralcore-wan.rs").as_slice(),
        ),
    ] {
        p[name] = json!(sha(bytes));
    }
    p
}
fn identity() -> Result<Value> {
    Ok(
        json!({"sources":pins(),"executable_sha256":sha(&fs::read(std::env::current_exe()?)?),
        "platform":format!("{}-{}",std::env::consts::ARCH,std::env::consts::OS),
        "hardware_sha256":cfg!(feature="hardware-sha256")}),
    )
}
fn fixture_seed(trial: usize, large: bool) -> u64 {
    let h = hash(
        b"spiralcore.warm-object.fixture.v1\0",
        &[&(trial as u64).to_le_bytes(), &[u8::from(large)]],
    );
    u64::from_le_bytes(h[..8].try_into().unwrap()).max(1)
}
fn fixture_pair(trial: usize, large: bool) -> Result<(Vec<u8>, Vec<u8>)> {
    if trial >= 30 {
        return Err("trial outside registration".into());
    }
    let mut state = fixture_seed(trial, large);
    let mut base = vec![0; if large { 16 << 20 } else { 1 << 20 }];
    // Exact historical xorshift64-v1 generator; new explicit trial/size seeds.
    for chunk in base.chunks_mut(8) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        chunk.copy_from_slice(&state.to_le_bytes()[..chunk.len()]);
    }
    let mut target = base.clone();
    let at = target.len() / 2;
    for b in &mut target[at..at + 32] {
        *b ^= 0xa5;
    }
    Ok((base, target))
}
fn metadata(bytes: &[u8]) -> Value {
    json!({"bytes":bytes.len(),"object":hex(&object_id(bytes)),"sha256":sha(bytes)})
}
fn order(trial: usize, large: bool) -> [&'static str; 2] {
    let mut shuffled: [bool; 30] = std::array::from_fn(|i| i < 15);
    let mut state = fixture_seed(30, large);
    for i in (1..shuffled.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        shuffled.swap(i, (state % (i as u64 + 1)) as usize);
    }
    if shuffled[trial] {
        ARMS_WARM
    } else {
        ["whole", "sparse"]
    }
}
fn scope_for(run: &str, trial: usize, large: bool, arm: &str, selection: bool) -> Id {
    hash(
        if selection {
            b"spiralcore.warm-selection.scope.v1\0"
        } else {
            b"spiralcore.warm-object.scope.v1\0"
        },
        &[
            run.as_bytes(),
            &(trial as u64).to_le_bytes(),
            &[u8::from(large)],
            arm.as_bytes(),
        ],
    )
}
fn plan(run: &str, shard: usize, source: &str, local: bool) -> Result<Value> {
    plan_for(run, shard, source, local, false)
}
fn selection_plan(run: &str, shard: usize, source: &str, local: bool) -> Result<Value> {
    plan_for(run, shard, source, local, true)
}
fn selection_order(trial: usize, large: bool) -> Result<[&'static str; 3]> {
    if trial >= 30 {
        return Err("trial outside registration".into());
    }
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let mut indices: [usize; 30] = std::array::from_fn(|i| i % 6);
    let mut state = fixture_seed(31, large);
    for i in (1..30).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        indices.swap(i, (state % (i as u64 + 1)) as usize);
    }
    Ok(permutations[indices[trial]].map(|i| ARMS_SELECTION[i]))
}
fn plan_for(run: &str, shard: usize, source: &str, local: bool, selection: bool) -> Result<Value> {
    if run.is_empty()
        || run.len() > 64
        || !run
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        || shard >= if selection { 6 } else { 3 }
        || (local && shard != 0)
        || source.len() != 40
        || !source
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err("invalid bounded registration".into());
    }
    let mut fixtures = Vec::new();
    let mut sessions = Vec::new();
    let mut scopes = Vec::new();
    let per_shard = if selection { 5 } else { 10 };
    let n = if local { 3 } else { per_shard };
    let arm_count = if selection { 3 } else { 2 };
    for trial in shard * per_shard..shard * per_shard + n {
        for large in [false, true] {
            let (base, target) = fixture_pair(trial, large)?;
            fixtures.push(json!({"trial":trial,"large":large,"seed":fixture_seed(trial,large),"base":metadata(&base),"target":metadata(&target)}));
            let ordered = if selection {
                selection_order(trial, large)?.to_vec()
            } else {
                order(trial, large).to_vec()
            };
            for arm in ordered {
                let s = hex(&scope_for(run, trial, large, arm, selection));
                scopes.push(s.clone());
                sessions.push(json!({"trial":trial,"large":large,"arm":arm,"scope":s}));
            }
        }
    }
    let retained = (n as u64) * 2 * arm_count * (17 << 20);
    let bound = retained + (40 << 20) + (16 << 20) + (n as u64) * 4 * arm_count * 4096;
    if bound > 768 << 20 {
        return Err("storage bound exceeded".into());
    }
    let mut p = json!({"schema":"spiralcore.wan.plan.v4","experiment":if selection{"warm-selection.v1"}else{"warm-object.v1"},"dose":if local{"local-qualification"}else{"wan"},
        "run":run,"shard":shard,"source_commit":source,"sources":pins(),"cold_graph_warming":false,"codec_cache_enabled":true,
        "generator":"xorshift64-v1/little-endian","edit":"midpoint32-xor-a5","warm_probe":false,
        "allowed_scopes":scopes,"sessions":sessions,"fixtures":fixtures,"seeds":n*2*arm_count as usize,"targets":n*2*arm_count as usize,"operations":n*4*arm_count as usize,
        "retained_payload_bytes":retained,"conservative_storage_bound":bound});
    if selection {
        p["artifact"] = metadata(ARTIFACT);
        p["schema_fits"] = json!(n * 2);
        p["training_samples"] = json!(512);
    }
    Ok(p)
}
fn read_plan(path: &Path, full: bool) -> Result<(Value, String)> {
    let raw = fs::read(path)?;
    let p: Value = serde_json::from_slice(&raw)?;
    if !matches!(
        p["experiment"].as_str(),
        Some("warm-object.v1" | "warm-selection.v1")
    ) || p["sources"] != pins()
        || !matches!(p["dose"].as_str(), Some("wan" | "local-qualification"))
    {
        return Err("plan identity changed".into());
    }
    if full
        && p != plan_for(
            p["run"].as_str().ok_or("run required")?,
            p["shard"].as_u64().ok_or("shard required")? as usize,
            p["source_commit"].as_str().ok_or("source required")?,
            p["dose"] == "local-qualification",
            selected_profile(&p),
        )?
    {
        return Err("plan changed".into());
    }
    Ok((p, sha(&raw)))
}
fn expected(p: &Value, trial: usize, large: bool) -> Result<&Value> {
    let matches = p["fixtures"]
        .as_array()
        .ok_or("fixtures missing")?
        .iter()
        .filter(|r| r["trial"] == trial && r["large"] == large)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err("fixture missing/duplicate".into());
    }
    Ok(matches[0])
}
fn check_identity(row: &Value, expected: &Value) -> Result<()> {
    for key in ["bytes", "object", "sha256"] {
        if row[key] != expected[key] {
            return Err("object identity differs".into());
        }
    }
    Ok(())
}
fn raw_cost(len: u64) -> u64 {
    368 + len + 133 * len.div_ceil(spiralcore_transfer::MAX_CHUNK as u64)
}
fn validate_operation(row: &Value, arm: &str, seed: bool, len: u64) -> Result<()> {
    if !ARMS_SELECTION.contains(&arm)
        || !row["error"].is_null()
        || numeric(row, "cpu_us")?
            != numeric(row, "sender_cpu_us")? + numeric(row, "receiver_cpu_us")?
        || numeric(row, "body_bytes")?
            != numeric(row, "request_body_bytes")? + numeric(row, "response_body_bytes")?
        || numeric(row, "exchange_wall_us")? > numeric(row, "wall_us")?
        || row["batches"] != 1
    {
        return Err("failed or inconsistent operation accounting".into());
    }
    if (seed || row["mode"] == "Raw")
        && (row["mode"] != "Raw" || row["body_bytes"] != raw_cost(len))
    {
        return Err("raw seed/fallback cost mismatch".into());
    }
    if !seed
        && (((arm == "sparse" || arm == "selector")
            && (row["mode"] != "SparseV1" || row["body_bytes"] != 600))
            || (arm == "whole" && !matches!(row["mode"].as_str(), Some("Raw" | "ZstdWholeV1"))))
    {
        return Err("update mode or sparse cost mismatch".into());
    }
    Ok(())
}
fn choice_json(choice: &warm_selection::WarmSelection) -> Value {
    json!({"selected_mode":format!("{:?}",choice.selected_mode),"needs_schema_seed":choice.needs_schema_seed,
        "candidates":choice.candidates.iter().map(|c|json!({"mode":format!("{:?}",c.mode),"target_http_body_bytes":c.target_http_body_bytes,"setup_http_body_bytes":c.setup_http_body_bytes,"total_http_body_bytes":c.total_http_body_bytes})).collect::<Vec<_>>(),
        "pruned":choice.pruned.iter().map(|c|json!({"mode":format!("{:?}",c.mode),"target_http_body_lower_bound":c.target_http_body_lower_bound,"setup_http_body_bytes":c.setup_http_body_bytes,"total_http_body_lower_bound":c.total_http_body_lower_bound})).collect::<Vec<_>>()})
}
fn validate_warm_choice(receipt: &Value, len: u64) -> Result<()> {
    let exact = receipt["candidates"]
        .as_array()
        .ok_or("exact candidates missing")?;
    let pruned = receipt["pruned"]
        .as_array()
        .ok_or("pruned candidates missing")?;
    let large = len == 16 << 20;
    if ![1 << 20, 16 << 20].contains(&len)
        || receipt["selected_mode"] != "SparseV1"
        || receipt["needs_schema_seed"] != false
        || exact.len() != if large { 2 } else { 3 }
        || pruned.len() != if large { 2 } else { 1 }
    {
        return Err("unexpected registered warm decision".into());
    }
    let modes = if large {
        vec!["Raw", "SparseV1"]
    } else {
        vec!["Raw", "ZstdWholeV1", "SparseV1"]
    };
    for (c, mode) in exact.iter().zip(modes) {
        let cost = c["target_http_body_bytes"]
            .as_u64()
            .ok_or("exact cost missing")?;
        if c["mode"] != mode
            || c["setup_http_body_bytes"] != 0
            || c["total_http_body_bytes"] != cost
            || cost < 600
            || (mode == "Raw" && cost != raw_cost(len))
            || (mode == "SparseV1" && cost != 600)
            || (mode == "ZstdWholeV1" && cost <= 600)
        {
            return Err("invalid evaluated cost or tie".into());
        }
    }
    let expected = if large {
        vec![
            ("ZstdWholeV1", 896, 0),
            ("SchemaZstdV1", 547, raw_cost(ARTIFACT.len() as u64)),
        ]
    } else {
        vec![("SchemaZstdV1", 547, raw_cost(ARTIFACT.len() as u64))]
    };
    for (c, (mode, bound, setup)) in pruned.iter().zip(expected) {
        if c["mode"] != mode
            || c["target_http_body_lower_bound"] != bound
            || c["setup_http_body_bytes"] != setup
            || c["total_http_body_lower_bound"] != bound + setup
            || bound + setup <= 600
        {
            return Err("invalid pruning bound or cold seed charge".into());
        }
    }
    Ok(())
}
enum Sender {
    Sparse,
    Whole(zstd_whole::PreparedEncoder),
    Selector(warm_selection::PreparedSelector, usize),
}
impl Sender {
    fn new(arm: &str) -> spiralcore_transfer::Result<Self> {
        match arm {
            "sparse" => Ok(Self::Sparse),
            "whole" => Ok(Self::Whole(zstd_whole::PreparedEncoder::new()?)),
            "selector" => {
                let training = training_records();
                let bytes = training.iter().map(Vec::len).sum();
                let refs = training.iter().map(Vec::as_slice).collect::<Vec<_>>();
                let artifact = schema::fit(&refs)?;
                if artifact != ARTIFACT {
                    return Err(spiralcore_transfer::Error::Integrity);
                }
                Ok(Self::Selector(
                    warm_selection::PreparedSelector::new(&artifact)?,
                    bytes,
                ))
            }
            _ => Err(spiralcore_transfer::Error::Unsupported),
        }
    }
    fn retained(&self) -> (usize, u64) {
        match self {
            Self::Sparse => (0, 0),
            Self::Whole(c) => (c.retained_bytes(), c.retained_charge()),
            Self::Selector(c, _) => (c.retained_bytes(), c.retained_charge()),
        }
    }
}
fn session(args: &[String]) -> Result<()> {
    let (p, digest) = read_plan(Path::new(&args[2]), false)?;
    let trial: usize = args[3].parse()?;
    let large = match args[4].as_str() {
        "0" => false,
        "1" => true,
        _ => return Err("invalid size".into()),
    };
    let arm = &args[5];
    let url = &args[6];
    let receiver = &args[7];
    wan::parse_id(receiver)?;
    let s = scope_for(
        p["run"].as_str().ok_or("run missing")?,
        trial,
        large,
        arm,
        selected_profile(&p),
    );
    if !p["sessions"]
        .as_array()
        .ok_or("sessions missing")?
        .iter()
        .any(|r| {
            r["trial"] == trial && r["large"] == large && r["arm"] == *arm && r["scope"] == hex(&s)
        })
    {
        return Err("unregistered session".into());
    }
    let (base, target) = fixture_pair(trial, large)?;
    let m = expected(&p, trial, large)?;
    check_identity(&metadata(&base), &m["base"])?;
    check_identity(&metadata(&target), &m["target"])?;
    let token =
        std::env::var("SPIRALCORE_WAN_TOKEN").map_err(|_| "SPIRALCORE_WAN_TOKEN required")?;
    let mut http = HttpClient::new(url, &token, s)?;
    let mut out = fresh(Path::new(&args[8]))?;
    let result = (|| -> Result<()> {
        write_row(
            &mut out,
            &json!({"event":"session_registration","identity":identity()?,"plan_sha256":digest,"trial":trial,"large":large,"arm":arm,"scope":hex(&s),"receiver_executable_sha256":receiver}),
        )?;
        let wall = Instant::now();
        let cpu = ProcessTime::now();
        let prepared = Sender::new(arm);
        let cpu_us = cpu.elapsed().as_micros();
        let wall_us = wall.elapsed().as_micros();
        let retained = prepared.as_ref().map(Sender::retained).unwrap_or((0, 0));
        let mut setup = json!({"event":"context_setup","cpu_us":cpu_us,"wall_us":wall_us,"retained_bytes":retained.0,"retained_charge":retained.1,"error":prepared.as_ref().err().map(ToString::to_string)});
        if selected_profile(&p) {
            setup["schema_fits"] = json!(usize::from(arm == "selector"));
            setup["artifact"] = if arm == "selector" {
                metadata(ARTIFACT)
            } else {
                Value::Null
            };
            setup["training_bytes"] = json!(if let Ok(Sender::Selector(_, bytes)) = &prepared {
                *bytes
            } else {
                0
            });
            setup["training_samples"] = json!(if arm == "selector" { 512 } else { 0 });
        }
        write_row(&mut out, &setup)?;
        let mut prepared = prepared?;
        for (seed, bytes) in [(true, base.as_slice()), (false, target.as_slice())] {
            let id = hash(
                b"spiralcore.warm-object.transfer.v1\0",
                &[&s, &[u8::from(seed)]],
            );
            let mut choice = None;
            let sent = measure(&mut http, |exchange| {
                if seed {
                    client::transfer_batched(bytes, Mode::Raw, None, id, false, exchange)
                } else {
                    match &mut prepared {
                        Sender::Sparse => {
                            client::transfer_sparse_batched(&base, bytes, id, false, exchange)
                        }
                        Sender::Whole(c) => client::transfer_prepared_zstd_whole_batched(
                            c, bytes, id, false, exchange,
                        ),
                        Sender::Selector(c, _) => {
                            let plan = c.prepare_warm(bytes, Some(&base), false, false)?;
                            choice = Some(plan.selection().clone());
                            if plan.selection().needs_schema_seed {
                                return Err(spiralcore_transfer::Error::Integrity);
                            }
                            client::transfer_warm_selection_batched(plan, id, exchange)
                        }
                    }
                }
            });
            let mut row = sent.row(if seed { "seed" } else { "target" }, None, bytes);
            if let Some(receipt) = choice {
                row["selection"] = choice_json(&receipt);
            }
            write_row(&mut out, &row)?;
            sent.result?;
            validate_operation(&row, arm, seed, bytes.len() as u64)?;
            if !seed && arm == "selector" {
                validate_warm_choice(&row["selection"], bytes.len() as u64)?;
            }
        }
        write_row(
            &mut out,
            &json!({"event":"session_complete","seeds":1,"targets":1,"status":"commit-acknowledged-reopen-pending"}),
        )
    })();
    if let Err(e) = result {
        write_row(&mut out, &json!({"event":"failure","error":e.to_string()}))?;
        return Err(e);
    }
    Ok(())
}
fn paired(plan_path: &Path, exe: &Path, url: &str, receiver: &str, out_path: &Path) -> Result<()> {
    let (p, digest) = read_plan(plan_path, true)?;
    let mut out = fresh(out_path)?;
    let result = (|| -> Result<()> {
        let output = Command::new(exe)
            .arg(format!("{}-identity", prefix(&p)))
            .output()?;
        let identity: Value = serde_json::from_slice(&output.stdout)?;
        if !output.status.success()
            || identity["sources"] != p["sources"]
            || identity["executable_sha256"] != sha(&fs::read(exe)?)
        {
            return Err("executable identity mismatch".into());
        }
        wan::parse_id(receiver)?;
        write_row(
            &mut out,
            &json!({"event":"registration","identity":identity,"source_commit":p["source_commit"],"plan_sha256":digest,"run":p["run"],"shard":p["shard"],"dose":p["dose"],"receiver_executable_sha256":receiver,"url":url}),
        )?;
        let dir = out_path.with_extension("sessions");
        fs::create_dir(&dir)?;
        for (index, s) in p["sessions"]
            .as_array()
            .ok_or("sessions missing")?
            .iter()
            .enumerate()
        {
            let path = dir.join(format!("{index:03}.jsonl"));
            let launched = Command::new(exe)
                .arg(format!("{}-session", prefix(&p)))
                .arg(plan_path)
                .arg(s["trial"].as_u64().ok_or("trial missing")?.to_string())
                .arg(if s["large"] == true { "1" } else { "0" })
                .arg(s["arm"].as_str().ok_or("arm missing")?)
                .arg(url)
                .arg(receiver)
                .arg(&path)
                .output();
            let output = match launched {
                Ok(o) => o,
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
            &json!({"event":"complete","sessions":p["sessions"].as_array().unwrap().len(),"operations":p["operations"]}),
        )
    })();
    if let Err(e) = result {
        write_row(&mut out, &json!({"event":"failure","error":e.to_string()}))?;
        return Err(e);
    }
    Ok(())
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
            return Err("stopped existing store required".into());
        }
        wan::parse_id(receiver)?;
        let log = fs::read(log_path)?;
        let observed = observations(&p, &log)?;
        write_row(
            &mut out,
            &json!({"event":"verification_registration","identity":identity()?,"plan_sha256":digest,"source_commit":p["source_commit"],"receiver_executable_sha256":receiver,"observation_sha256":sha(&log),"observations":observed}),
        )?;
        let mut endpoint = Endpoint::open_with_policy(
            cache,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )?;
        let mut exact = true;
        let mut checked = 0;
        for s in p["sessions"].as_array().ok_or("sessions missing")? {
            let scope = wan::parse_id(s["scope"].as_str().ok_or("scope missing")?)?;
            let trial = s["trial"].as_u64().ok_or("trial missing")? as usize;
            let large = s["large"].as_bool().ok_or("size missing")?;
            let (base, target) = fixture_pair(trial, large)?;
            for (phase, bytes) in [("seed", base), ("target", target)] {
                let wall = Instant::now();
                let cpu = ProcessTime::now();
                let actual = endpoint.read_object(&wan::context(scope), object_id(&bytes));
                let same = actual.as_ref().is_ok_and(|v| v == &bytes);
                let cpu_us = cpu.elapsed().as_micros();
                let wall_us = wall.elapsed().as_micros();
                exact &= same;
                checked += 1;
                let mut row = metadata(&bytes);
                let map = row.as_object_mut().unwrap();
                map.extend(json!({"event":"receiver_verification","scope":hex(&scope),"phase":phase,"exact":same,"actual_bytes":actual.as_ref().ok().map(Vec::len),"actual_sha256":actual.as_ref().ok().map(|v|sha(v)),"error":actual.err().map(|e|e.to_string()),"cpu_us":cpu_us,"wall_us":wall_us}).as_object().unwrap().clone());
                write_row(&mut out, &row)?;
            }
        }
        write_row(
            &mut out,
            &json!({"event":"verification_complete","checked":checked,"exact":exact}),
        )?;
        if !exact || p["operations"] != checked {
            return Err("independent reopen failed".into());
        }
        Ok(())
    })();
    if let Err(e) = result {
        write_row(&mut out, &json!({"event":"failure","error":e.to_string()}))?;
        return Err(e);
    }
    Ok(())
}
fn validate_receipts(
    p: &Value,
    digest: &str,
    raw: &[Value],
    verified: &[Value],
) -> Result<(Vec<Value>, Vec<Value>)> {
    let sessions = p["sessions"].as_array().ok_or("sessions missing")?;
    let count = sessions.len();
    if raw.len() != count + 2
        || verified.len() != count * 2 + 2
        || raw[0]["event"] != "registration"
        || raw[0]["plan_sha256"] != digest
        || raw[0]["source_commit"] != p["source_commit"]
        || raw[0]["run"] != p["run"]
        || raw[0]["shard"] != p["shard"]
        || raw[0]["dose"] != p["dose"]
        || raw[0]["identity"]["sources"] != p["sources"]
        || raw[count + 1]["event"] != "complete"
        || raw[count + 1]["sessions"] != count
        || raw[count + 1]["operations"] != count * 2
        || verified[0]["event"] != "verification_registration"
        || verified[0]["plan_sha256"] != digest
        || verified[0]["source_commit"] != p["source_commit"]
        || verified[0]["identity"]["sources"] != p["sources"]
        || verified[0]["receiver_executable_sha256"] != raw[0]["receiver_executable_sha256"]
        || verified[count * 2 + 1]["event"] != "verification_complete"
        || verified[count * 2 + 1]["checked"] != count * 2
        || verified[count * 2 + 1]["exact"] != true
    {
        return Err("incomplete/unbound shard".into());
    }
    for value in [
        &raw[0]["identity"]["executable_sha256"],
        &raw[0]["receiver_executable_sha256"],
        &verified[0]["identity"]["executable_sha256"],
        &verified[0]["observation_sha256"],
    ] {
        wan::parse_id(value.as_str().ok_or("hash missing")?)?;
    }
    let observed = verified[0]["observations"]
        .as_array()
        .ok_or("observations missing")?;
    observations(
        p,
        observed
            .iter()
            .map(serde_json::to_string)
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join("\n")
            .as_bytes(),
    )?;
    if observed.len() != count * 2 {
        return Err("receiver request count differs".into());
    }
    let mut operations = Vec::new();
    let mut totals = Vec::new();
    let mut preparations = 0u64;
    for (index, s) in sessions.iter().enumerate() {
        let outer = &raw[index + 1];
        if outer["event"] != "session" || outer["index"] != index || outer["session"] != *s {
            return Err("session order changed".into());
        }
        wan::parse_id(outer["raw_sha256"].as_str().ok_or("session hash missing")?)?;
        let r = outer["rows"].as_array().ok_or("session rows missing")?;
        if r.len() != 5
            || r[0]["event"] != "session_registration"
            || r[0]["identity"] != raw[0]["identity"]
            || r[0]["plan_sha256"] != digest
            || r[0]["receiver_executable_sha256"] != raw[0]["receiver_executable_sha256"]
            || r[1]["event"] != "context_setup"
            || !r[1]["error"].is_null()
            || r[4]["event"] != "session_complete"
            || r[4]["seeds"] != 1
            || r[4]["targets"] != 1
        {
            return Err("session incomplete or misbound".into());
        }
        for key in ["trial", "large", "arm", "scope"] {
            if r[0][key] != s[key] {
                return Err("session identity differs".into());
            }
        }
        let arm = s["arm"].as_str().ok_or("arm missing")?;
        let m = expected(
            p,
            s["trial"].as_u64().ok_or("trial missing")? as usize,
            s["large"].as_bool().ok_or("size missing")?,
        )?;
        if m["base"]["object"] == m["target"]["object"] {
            return Err("unchanged target".into());
        }
        let mut total = s.clone();
        total["event"] = json!("lifecycle");
        for key in METRICS_WAN {
            total[key] = json!(0.0);
        }
        for key in ["cpu_us", "wall_us", "retained_bytes", "retained_charge"] {
            numeric(&r[1], key)?;
        }
        if arm == "sparse" && (r[1]["retained_bytes"] != 0 || r[1]["retained_charge"] != 0) {
            return Err("unexpected sparse context".into());
        }
        if selected_profile(p) {
            let selector = arm == "selector";
            let training_bytes = if selector {
                training_records().iter().map(Vec::len).sum::<usize>()
            } else {
                0
            };
            if r[1]["schema_fits"] != usize::from(selector)
                || r[1]["training_bytes"] != training_bytes
                || r[1]["training_samples"] != if selector { 512 } else { 0 }
                || r[1]["artifact"]
                    != if selector {
                        metadata(ARTIFACT)
                    } else {
                        Value::Null
                    }
            {
                return Err("unpaid/unbound schema fitting".into());
            }
            total["schema_fits"] = r[1]["schema_fits"].clone();
            total["artifact"] = r[1]["artifact"].clone();
        }
        total["context_cpu_us"] = r[1]["cpu_us"].clone();
        total["context_wall_us"] = r[1]["wall_us"].clone();
        total["retained_bytes"] = r[1]["retained_bytes"].clone();
        total["retained_charge"] = r[1]["retained_charge"].clone();
        for key in ["cpu_us", "sender_cpu_us"] {
            total[key] = r[1]["cpu_us"].clone();
        }
        total["wall_us"] = r[1]["wall_us"].clone();
        for phase_index in 0..2 {
            let row = &r[phase_index + 2];
            let seed = phase_index == 0;
            let phase = if seed { "seed" } else { "target" };
            let expected = &m[if seed { "base" } else { "target" }];
            if row["event"] != "operation" || row["phase"] != phase || !row["position"].is_null() {
                return Err("seed/update order differs".into());
            }
            check_identity(row, expected)?;
            validate_operation(
                row,
                arm,
                seed,
                expected["bytes"].as_u64().ok_or("length missing")?,
            )?;
            if !seed && arm == "selector" {
                validate_warm_choice(
                    &row["selection"],
                    expected["bytes"].as_u64().ok_or("length missing")?,
                )?;
            } else if !row["selection"].is_null() {
                return Err("unexpected selection receipt".into());
            }
            let v = &verified[1 + index * 2 + phase_index];
            check_identity(v, expected)?;
            if v["event"] != "receiver_verification"
                || v["scope"] != s["scope"]
                || v["phase"] != phase
                || v["exact"] != true
                || v["actual_bytes"] != expected["bytes"]
                || v["actual_sha256"] != expected["sha256"]
                || !v["error"].is_null()
            {
                return Err("missing/failed exact readback".into());
            }
            let obs = &observed[index * 2 + phase_index];
            if !seed && row["mode"] == "ZstdWholeV1" {
                preparations += 1;
            }
            let c = &obs["codec_cache"];
            if obs["scope"] != s["scope"]
                || c["hits"] != 0
                || c["misses"] != preparations
                || c["preparations"] != preparations
                || c["evictions"] != preparations.saturating_sub(8)
                || c["entries"] != preparations.min(8)
                || c["resident_bytes"] != preparations.min(8) * (5 << 20)
            {
                return Err("cache history differs from observed modes".into());
            }
            let mut copy = row.clone();
            copy["raw_fallback"] = json!(!seed && arm == "whole" && row["mode"] == "Raw");
            for key in ["trial", "large", "arm", "scope"] {
                copy[key] = s[key].clone();
            }
            operations.push(copy);
            for key in METRICS_WAN {
                total[key] = json!(numeric(&total, key)? + numeric(row, key)?);
            }
        }
        totals.push(total);
    }
    Ok((operations, totals))
}
fn summary(paths: &[String], local: bool) -> Result<Value> {
    let first = read_plan(Path::new(paths.first().ok_or("plan required")?), false)?.0;
    let selection = selected_profile(&first);
    let arm_count = arms(&first).len();
    if paths.len()
        != if local {
            3
        } else if selection {
            18
        } else {
            9
        }
    {
        return Err("complete plan/raw/verify triples required".into());
    }
    let mut operations = Vec::new();
    let mut totals = Vec::new();
    let mut receipts = Vec::new();
    let mut registration = None;
    for (shard, chunk) in paths.chunks_exact(3).enumerate() {
        let (p, digest) = read_plan(Path::new(&chunk[0]), true)?;
        if p["shard"] != shard
            || (p["dose"] == "local-qualification") != local
            || selected_profile(&p) != selection
        {
            return Err("dose/shard differs".into());
        }
        let raw = fs::read(&chunk[1])?;
        let verified = fs::read(&chunk[2])?;
        let rr = rows(&raw)?;
        let vv = rows(&verified)?;
        let (oo, tt) = validate_receipts(&p, &digest, &rr, &vv)?;
        let reg = json!({"run":p["run"],"source_commit":p["source_commit"],"sources":p["sources"],"sender":rr[0]["identity"],"receiver":rr[0]["receiver_executable_sha256"]});
        if registration.as_ref().is_some_and(|r| r != &reg) {
            return Err("cross-shard identity changed".into());
        }
        registration = Some(reg);
        receipts.push(json!({"shard":shard,"plan_sha256":digest,"raw_sha256":sha(&raw),"verification_sha256":sha(&verified),"observation_sha256":vv[0]["observation_sha256"]}));
        operations.extend(oo);
        totals.extend(tt);
    }
    let n = if local { 3 } else { 30 };
    let mut cells = Vec::new();
    let mut comparisons = Vec::new();
    for large in [false, true] {
        for phase in ["seed", "target", "lifecycle"] {
            let input = if phase == "lifecycle" {
                &totals
            } else {
                &operations
            };
            for &arm in arms(&first) {
                let group = input
                    .iter()
                    .filter(|r| {
                        r["large"] == large
                            && r["arm"] == arm
                            && (phase == "lifecycle" || r["phase"] == phase)
                    })
                    .collect::<Vec<_>>();
                if group.len() != n {
                    return Err("incomplete cell".into());
                }
                cells.push(json!({"large":large,"bytes":if large{16<<20}else{1<<20},"phase":phase,"arm":arm,"trials":n,"metrics":describe(&group)?,
                    "selected_modes":group.iter().fold(serde_json::Map::new(),|mut m,r|{let k=r["mode"].as_str().unwrap_or("lifecycle");let old=m.get(k).and_then(Value::as_u64).unwrap_or(0);m.insert(k.to_string(),json!(old+1));m})}));
            }
            let pairs = if selection {
                vec![
                    ("selector", "whole"),
                    ("selector", "sparse"),
                    ("sparse", "whole"),
                ]
            } else {
                vec![("sparse", "whole")]
            };
            for (numerator, denominator) in pairs {
                let mut comparison = json!({"large":large,"phase":phase,"ratio":format!("{numerator}/{denominator}"),"paired_trials":n});
                for metric in [
                    "body_bytes",
                    "sender_cpu_us",
                    "receiver_cpu_us",
                    "cpu_us",
                    "wall_us",
                    "exchange_wall_us",
                ] {
                    let mut values = Vec::new();
                    for trial in 0..n {
                        let get = |arm: &str| -> Result<&Value> {
                            let found = input
                                .iter()
                                .filter(|r| {
                                    r["trial"] == trial
                                        && r["large"] == large
                                        && r["arm"] == arm
                                        && (phase == "lifecycle" || r["phase"] == phase)
                                })
                                .collect::<Vec<_>>();
                            if found.len() != 1 {
                                return Err("pair missing/duplicate".into());
                            }
                            Ok(found[0])
                        };
                        let a = get(numerator)?;
                        let b = get(denominator)?;
                        if phase != "lifecycle" {
                            check_identity(a, b)?;
                        }
                        let ratio = numeric(a, metric)? / numeric(b, metric)?;
                        if !ratio.is_finite() || ratio <= 0.0 {
                            return Err("nonpositive paired ratio".into());
                        }
                        values.push(ratio);
                    }
                    comparison[metric] = if local {
                        json!({"paired_trials":n,"geometric_mean":(values.iter().map(|v|v.ln()).sum::<f64>()/n as f64).exp(),"wins":values.iter().filter(|v|**v<1.0).count(),"ties":values.iter().filter(|v|**v==1.0).count(),"distribution":dist(values.clone()),"ratios":values})
                    } else {
                        schema_wan::ratios(values)?
                    };
                }
                comparisons.push(comparison);
            }
        }
    }
    if selection {
        for row in operations
            .iter()
            .filter(|r| r["arm"] == "selector" && r["phase"] == "target")
        {
            let paired = operations
                .iter()
                .filter(|r| {
                    r["trial"] == row["trial"]
                        && r["large"] == row["large"]
                        && r["phase"] == "target"
                })
                .collect::<Vec<_>>();
            let whole = paired
                .iter()
                .find(|r| r["arm"] == "whole")
                .ok_or("whole control missing")?;
            let sparse = paired
                .iter()
                .find(|r| r["arm"] == "sparse")
                .ok_or("sparse control missing")?;
            if row["body_bytes"] != sparse["body_bytes"] || row["mode"] != sparse["mode"] {
                return Err("selector differs from forced Sparse".into());
            }
            let receipt = &row["selection"];
            if row["large"] == false {
                let cost = receipt["candidates"][1]["target_http_body_bytes"]
                    .as_u64()
                    .ok_or("whole cost missing")?;
                if whole["body_bytes"] != cost.min(raw_cost(row["bytes"].as_u64().unwrap())) {
                    return Err("evaluated Whole cost differs from control".into());
                }
            } else if numeric(whole, "body_bytes")?
                < numeric(&receipt["pruned"][0], "total_http_body_lower_bound")?
            {
                return Err("pruned Whole bound exceeds control".into());
            }
        }
    }
    let mut result = json!({"schema":if selection{"spiralcore.warm-selection.summary.v1"}else{"spiralcore.warm-object.summary.v1"},"verdict":"EXACT_PAIRED_TRANSFERS_PASS","dose":if local{"local-qualification"}else{"wan"},"registration":registration,"receipts":receipts,
        "sessions":n*2*arm_count,"seed_operations":n*2*arm_count,"target_operations":n*2*arm_count,"exact_reopened_objects":n*4*arm_count,"discarded_observations":0,"cells":cells,"paired_comparisons":comparisons,"actual_lifecycles":totals,
        "metric_scope":"HTTP request+response body bytes; sender process CPU plus disjoint receiver handler CPU; target encoding-start through durable ACK wall; lifecycle is fitting/context+paid seed+update disjoint phase sum, excludes fixture generation, process launch and untimed verification; exchange wall is not pure wire time"});
    if selection {
        result["schema_fits"] = json!(n * 2);
        result["selection_decisions"]=json!(operations.iter().filter(|r|r["arm"]=="selector"&&r["phase"]=="target").map(|r|json!({"trial":r["trial"],"large":r["large"],"object":r["object"],"selection":r["selection"]})).collect::<Vec<_>>());
    }
    Ok(result)
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn local_receiver(plan_path: &Path, cache: &Path) -> Result<()> {
    let (p, _) = read_plan(plan_path, false)?;
    if p["dose"] != "local-qualification" || cache.exists() {
        return Err("fresh local receiver required".into());
    }
    let token = std::env::var("SPIRALCORE_WAN_TOKEN")?;
    let scopes = p["allowed_scopes"]
        .as_array()
        .ok_or("scopes missing")?
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
    wan::serve_observed(
        listener,
        endpoint,
        &token,
        &scopes,
        Duration::from_secs(300),
        p["operations"].as_u64().ok_or("operations missing")? as usize,
        |o| {
            let c = o.codec_cache;
            let row = json!({"event":"wan_request","request_ordinal":o.request_ordinal,"scope":hex(&o.scope),"http_status":o.http_status,"codec_cache":{"hits":c.hits,"misses":c.misses,"preparations":c.preparations,"evictions":c.evictions,"entries":c.entries,"resident_bytes":c.resident_bytes}});
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
fn local(exe: &Path, dir: &Path, run: &str, selection: bool) -> Result<()> {
    fs::create_dir(dir)?;
    let source = std::env::var(if selection {
        "SPIRALCORE_WARMSELECT_SOURCE"
    } else {
        "SPIRALCORE_WARM_SOURCE"
    })
    .map_err(|_| "source commit required")?;
    let p = plan_for(run, 0, &source, true, selection)?;
    let plan_path = dir.join("plan.json");
    write_row(&mut fresh(&plan_path)?, &p)?;
    let mut token = [0u8; 32];
    File::open("/dev/urandom")?.read_exact(&mut token)?;
    let token = hex(&token);
    let log_path = dir.join("receiver.jsonl");
    let cache = dir.join("cache");
    let receiver = sha(&fs::read(exe)?);
    let child = Command::new(exe)
        .arg(format!("{}-local-receiver", prefix(&p)))
        .arg(&plan_path)
        .arg(&cache)
        .env("SPIRALCORE_WAN_TOKEN", &token)
        .stdout(Stdio::piped())
        .stderr(Stdio::from(fresh(&log_path)?))
        .spawn()?;
    let mut child = ChildGuard(child);
    let mut ready = String::new();
    BufReader::new(child.0.stdout.take().ok_or("receiver stdout missing")?)
        .read_line(&mut ready)?;
    let url = ready
        .trim()
        .strip_prefix("READY ")
        .ok_or("receiver did not become ready")?;
    let paired_path = dir.join("paired.jsonl");
    let output = Command::new(exe)
        .arg(format!("{}-paired", prefix(&p)))
        .arg(&plan_path)
        .arg(exe)
        .arg(url)
        .arg(&receiver)
        .arg(&paired_path)
        .env("SPIRALCORE_WAN_TOKEN", &token)
        .output()?;
    save(&dir.join("driver.stdout"), &output.stdout)?;
    save(&dir.join("driver.stderr"), &output.stderr)?;
    write_row(
        &mut fresh(&dir.join("driver-status.json"))?,
        &json!({"success":output.status.success(),"exit_code":output.status.code()}),
    )?;
    if !output.status.success() {
        return Err("local driver failed; cache retained".into());
    }
    let status = child.0.wait()?;
    write_row(
        &mut fresh(&dir.join("receiver-status.json"))?,
        &json!({"success":status.success(),"exit_code":status.code()}),
    )?;
    if !status.success() {
        return Err("local receiver failed; cache retained".into());
    }
    let verification = dir.join("verification.jsonl");
    verify(&plan_path, &cache, &verification, &log_path, &receiver)?;
    let paths = [plan_path, paired_path, verification].map(|p| p.to_string_lossy().into_owned());
    write_row(
        &mut fresh(&dir.join("summary.json"))?,
        &summary(&paths, true)?,
    )?;
    fs::remove_dir_all(&cache)?;
    write_row(
        &mut fresh(&dir.join("cleanup.json"))?,
        &json!({"owned_store_removed":true,"only_after_exact_reopen":true,"objects":p["operations"]}),
    )
}
pub(super) fn command(args: &[String]) -> Result<()> {
    match args.get(1).map(String::as_str) {
        Some("warm-identity"|"warmselect-identity") if args.len()==2=>{println!("{}",identity()?);Ok(())},
        Some("warm-plan") if args.len()==5=>{let source=std::env::var("SPIRALCORE_WARM_SOURCE").map_err(|_|"SPIRALCORE_WARM_SOURCE required")?;write_row(&mut fresh(Path::new(&args[4]))?,&plan(&args[2],args[3].parse()?,&source,false)?)},
        Some("warmselect-plan") if args.len()==5=>{let source=std::env::var("SPIRALCORE_WARMSELECT_SOURCE").map_err(|_|"SPIRALCORE_WARMSELECT_SOURCE required")?;write_row(&mut fresh(Path::new(&args[4]))?,&selection_plan(&args[2],args[3].parse()?,&source,false)?)},
        Some("warm-session"|"warmselect-session") if args.len()==9=>session(args),
        Some("warm-paired"|"warmselect-paired") if args.len()==7=>paired(Path::new(&args[2]),Path::new(&args[3]),&args[4],&args[5],Path::new(&args[6])),
        Some("warm-verify") if args.len()==5=>verify(Path::new(&args[2]),Path::new(&args[3]),Path::new(&args[4]),Path::new(&std::env::var("SPIRALCORE_WARM_OBSERVATIONS").map_err(|_|"SPIRALCORE_WARM_OBSERVATIONS required")?),&std::env::var("SPIRALCORE_WARM_RECEIVER_SHA256").map_err(|_|"SPIRALCORE_WARM_RECEIVER_SHA256 required")?),
        Some("warmselect-verify") if args.len()==5=>verify(Path::new(&args[2]),Path::new(&args[3]),Path::new(&args[4]),Path::new(&std::env::var("SPIRALCORE_WARMSELECT_OBSERVATIONS").map_err(|_|"SPIRALCORE_WARMSELECT_OBSERVATIONS required")?),&std::env::var("SPIRALCORE_WARMSELECT_RECEIVER_SHA256").map_err(|_|"SPIRALCORE_WARMSELECT_RECEIVER_SHA256 required")?),
        Some("warm-summary") if args.len()==12=>write_row(&mut fresh(Path::new(&args[11]))?,&summary(&args[2..11],false)?),
        Some("warmselect-summary") if args.len()==21=>write_row(&mut fresh(Path::new(&args[20]))?,&summary(&args[2..20],false)?),
        Some("warm-local") if args.len()==5=>local(Path::new(&args[2]),Path::new(&args[3]),&args[4],false),
        Some("warmselect-local") if args.len()==5=>local(Path::new(&args[2]),Path::new(&args[3]),&args[4],true),
        Some("warm-local-receiver"|"warmselect-local-receiver") if args.len()==4=>local_receiver(Path::new(&args[2]),Path::new(&args[3])),
        _=>Err("usage: warm[-identity|-plan RUN SHARD NEW_PLAN|-session PLAN TRIAL SIZE ARM URL RECEIVER_SHA NEW_JSONL|-paired PLAN EXE URL RECEIVER_SHA NEW_JSONL|-verify PLAN STOPPED_CACHE NEW_JSONL|-summary THREE_PLAN_RAW_VERIFY_TRIPLES NEW_JSON|-local EXE NEW_DIRECTORY RUN]; warmselect-* has the same arguments except six summary triples".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn fabricated_choice(len: u64) -> Value {
        let mut exact = vec![
            json!({"mode":"Raw","target_http_body_bytes":raw_cost(len),"setup_http_body_bytes":0,"total_http_body_bytes":raw_cost(len)}),
        ];
        let mut pruned = Vec::new();
        if len == 1 << 20 {
            exact.push(json!({"mode":"ZstdWholeV1","target_http_body_bytes":raw_cost(len)+1,"setup_http_body_bytes":0,"total_http_body_bytes":raw_cost(len)+1}));
        } else {
            pruned.push(json!({"mode":"ZstdWholeV1","target_http_body_lower_bound":896,"setup_http_body_bytes":0,"total_http_body_lower_bound":896}));
        }
        exact.push(json!({"mode":"SparseV1","target_http_body_bytes":600,"setup_http_body_bytes":0,"total_http_body_bytes":600}));
        pruned.push(json!({"mode":"SchemaZstdV1","target_http_body_lower_bound":547,"setup_http_body_bytes":670,"total_http_body_lower_bound":1217}));
        json!({"selected_mode":"SparseV1","needs_schema_seed":false,"candidates":exact,"pruned":pruned})
    }

    #[test]
    fn warm_selection_actual_fit_and_consumed_plan_preserve_scoped_exact_bytes() {
        let Sender::Selector(mut encoder, training_bytes) = Sender::new("selector").unwrap() else {
            panic!("selector setup required")
        };
        assert!(training_bytes > 0);
        assert_eq!(encoder.artifact(), ARTIFACT);
        let root = temporary("selector");
        fs::create_dir(&root).unwrap();
        let cache = root.join("cache");
        let scope = [81; 32];
        let (base, target) = fixture_pair(0, false).unwrap();
        let mut channel = Channel::open(&cache, scope).unwrap();
        client::transfer_batched(&base, Mode::Raw, None, [82; 32], false, |f| {
            channel.exchange(f)
        })
        .unwrap();
        assert_eq!(
            channel
                .endpoint
                .read_object(&wan::context(scope), object_id(&target)),
            Err(spiralcore_transfer::Error::Unknown)
        );
        let plan = encoder
            .prepare_warm(&target, Some(&base), false, false)
            .unwrap();
        validate_warm_choice(&choice_json(plan.selection()), target.len() as u64).unwrap();
        let before = channel.metrics.request + channel.metrics.response;
        let sent = client::transfer_warm_selection_batched(plan, [83; 32], |f| channel.exchange(f))
            .unwrap();
        assert_eq!(sent.mode, Mode::SparseV1);
        assert_eq!(
            channel.metrics.request + channel.metrics.response - before,
            600
        );
        drop(channel);
        let mut reopened = Endpoint::open_with_policy(
            &cache,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        assert_eq!(
            reopened
                .read_object(&wan::context(scope), object_id(&target))
                .unwrap(),
            target
        );
        assert_eq!(
            reopened.read_object(&wan::context(scope), object_id(ARTIFACT)),
            Err(spiralcore_transfer::Error::Unknown)
        );
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn warm_selection_plan_has_six_bounded_shards_and_all_permutations() {
        let p = selection_plan("selection-qualification", 5, &"1".repeat(40), false).unwrap();
        assert_eq!(p["experiment"], "warm-selection.v1");
        assert_eq!(p["operations"], 60);
        assert_eq!(p["sessions"].as_array().unwrap().len(), 30);
        assert_eq!(p["retained_payload_bytes"], 510u64 << 20);
        assert_eq!(p["conservative_storage_bound"], (566u64 << 20) + 60 * 4096);
        let local = selection_plan("selection-local", 0, &"1".repeat(40), true).unwrap();
        assert_eq!(local["operations"], 36);
        for large in [false, true] {
            let mut counts = std::collections::BTreeMap::new();
            for trial in 0..30 {
                *counts
                    .entry(selection_order(trial, large).unwrap())
                    .or_insert(0) += 1;
            }
            assert_eq!(counts.len(), 6);
            assert!(counts.values().all(|n| *n == 5));
        }
    }

    #[test]
    fn warm_selection_receipt_rejects_fake_bounds_and_omitted_cold_setup() {
        let row = json!({"selected_mode":"SparseV1","needs_schema_seed":false,"candidates":[
            {"mode":"Raw","target_http_body_bytes":raw_cost(16<<20),"setup_http_body_bytes":0,"total_http_body_bytes":raw_cost(16<<20)},
            {"mode":"SparseV1","target_http_body_bytes":600,"setup_http_body_bytes":0,"total_http_body_bytes":600}],
            "pruned":[{"mode":"ZstdWholeV1","target_http_body_lower_bound":896,"setup_http_body_bytes":0,"total_http_body_lower_bound":896},
            {"mode":"SchemaZstdV1","target_http_body_lower_bound":547,"setup_http_body_bytes":670,"total_http_body_lower_bound":1217}]});
        validate_warm_choice(&row, 16 << 20).unwrap();
        let mut wrong = row.clone();
        wrong["pruned"][1]["setup_http_body_bytes"] = json!(0);
        assert!(validate_warm_choice(&wrong, 16 << 20).is_err());
        wrong = row.clone();
        wrong["pruned"][0]["target_http_body_lower_bound"] = json!(600);
        assert!(validate_warm_choice(&wrong, 16 << 20).is_err());
        wrong = row.clone();
        wrong["needs_schema_seed"] = json!(true);
        assert!(validate_warm_choice(&wrong, 16 << 20).is_err());
        assert!(validate_warm_choice(&row, 1 << 20).is_err());
    }

    fn temporary(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "spiralcore-warm-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn warm_randomized_pair_orders_are_reproducible_and_exactly_balanced() {
        for large in [false, true] {
            let a = (0..30).map(|t| order(t, large)).collect::<Vec<_>>();
            assert_eq!(a, (0..30).map(|t| order(t, large)).collect::<Vec<_>>());
            assert_eq!(a.iter().filter(|v| **v == ARMS_WARM).count(), 15);
            assert!(a.windows(2).any(|w| w[0] == w[1]));
        }
        let p = plan("bounded-plan", 1, &"1".repeat(40), false).unwrap();
        assert_eq!(p["operations"], 80);
        assert_eq!(p["sessions"].as_array().unwrap().len(), 40);
        assert_eq!(p["retained_payload_bytes"], 680u64 << 20);
        assert_eq!(p["conservative_storage_bound"], (736u64 << 20) + 80 * 4096);
        let scopes = p["allowed_scopes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(scopes.len(), 40);
    }

    #[test]
    fn warm_sparse_requires_scoped_base_then_commits_absent_target_exactly() {
        let root = temporary("scope");
        fs::create_dir(&root).unwrap();
        let cache = root.join("cache");
        let scope = [71; 32];
        let (base, target) = fixture_pair(0, false).unwrap();
        let mut channel = Channel::open(&cache, scope).unwrap();
        assert_eq!(
            channel
                .endpoint
                .read_object(&wan::context(scope), object_id(&target)),
            Err(spiralcore_transfer::Error::Unknown)
        );
        client::transfer_batched(&base, Mode::Raw, None, [72; 32], false, |f| {
            channel.exchange(f)
        })
        .unwrap();
        assert_eq!(
            channel
                .endpoint
                .read_object(&wan::context(scope), object_id(&base))
                .unwrap(),
            base
        );
        assert_eq!(
            channel
                .endpoint
                .read_object(&wan::context(scope), object_id(&target)),
            Err(spiralcore_transfer::Error::Unknown)
        );
        channel.scope = [73; 32];
        assert!(
            client::transfer_sparse_batched(&base, &target, [74; 32], false, |f| channel
                .exchange(f))
            .is_err()
        );
        assert_eq!(
            channel
                .endpoint
                .read_object(&wan::context(channel.scope), object_id(&target)),
            Err(spiralcore_transfer::Error::Unknown)
        );
        channel.scope = scope;
        let before = channel.metrics.request + channel.metrics.response;
        let stats = client::transfer_sparse_batched(&base, &target, [75; 32], false, |f| {
            channel.exchange(f)
        })
        .unwrap();
        assert_eq!(stats.mode, Mode::SparseV1);
        assert_eq!(
            channel.metrics.request + channel.metrics.response - before,
            600
        );
        drop(channel);
        let mut reopened = Endpoint::open_with_policy(
            &cache,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        assert_eq!(
            reopened
                .read_object(&wan::context(scope), object_id(&base))
                .unwrap(),
            base
        );
        assert_eq!(
            reopened
                .read_object(&wan::context(scope), object_id(&target))
                .unwrap(),
            target
        );
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }

    fn fabricated_receipts(p: &Value) -> (Vec<Value>, Vec<Value>) {
        let id = json!({"sources":p["sources"],"executable_sha256":"aa".repeat(32)});
        let receiver = "bb".repeat(32);
        let mut raw = vec![
            json!({"event":"registration","identity":id,"source_commit":p["source_commit"],"plan_sha256":"plan","run":p["run"],"shard":p["shard"],"dose":p["dose"],"receiver_executable_sha256":receiver}),
        ];
        let mut verified = vec![
            json!({"event":"verification_registration","identity":id,"source_commit":p["source_commit"],"plan_sha256":"plan","receiver_executable_sha256":receiver,"observation_sha256":"cc".repeat(32)}),
        ];
        let mut observations = Vec::new();
        for (index, s) in p["sessions"].as_array().unwrap().iter().enumerate() {
            let fixture = expected(
                p,
                s["trial"].as_u64().unwrap() as usize,
                s["large"].as_bool().unwrap(),
            )
            .unwrap();
            let mut registration = s.clone();
            registration.as_object_mut().unwrap().extend(json!({"event":"session_registration","identity":id,"plan_sha256":"plan","receiver_executable_sha256":receiver}).as_object().unwrap().clone());
            let mut rr = vec![
                registration,
                json!({"event":"context_setup","cpu_us":1,"wall_us":2,"retained_bytes":0,"retained_charge":0,"error":null}),
            ];
            if selected_profile(p) {
                let selector = s["arm"] == "selector";
                rr[1]["schema_fits"] = json!(usize::from(selector));
                rr[1]["training_samples"] = json!(if selector { 512 } else { 0 });
                rr[1]["training_bytes"] = json!(if selector {
                    training_records().iter().map(Vec::len).sum::<usize>()
                } else {
                    0
                });
                rr[1]["artifact"] = if selector {
                    metadata(ARTIFACT)
                } else {
                    Value::Null
                };
            }
            for seed in [true, false] {
                let meta = &fixture[if seed { "base" } else { "target" }];
                let sparse = !seed && (s["arm"] == "sparse" || s["arm"] == "selector");
                let cost = if sparse {
                    600
                } else {
                    raw_cost(meta["bytes"].as_u64().unwrap())
                };
                let phase = if seed { "seed" } else { "target" };
                let mut op = meta.clone();
                op.as_object_mut().unwrap().extend(json!({"event":"operation","phase":phase,"position":null,"error":null,"mode":if sparse{"SparseV1"}else{"Raw"},"body_bytes":cost,"request_body_bytes":cost-200,"response_body_bytes":200,"batches":1,"sender_cpu_us":10,"receiver_cpu_us":20,"cpu_us":30,"wall_us":50,"exchange_wall_us":40,"exchange_cpu_us":5}).as_object().unwrap().clone());
                if !seed && s["arm"] == "selector" {
                    op["selection"] = fabricated_choice(meta["bytes"].as_u64().unwrap());
                }
                rr.push(op);
                let mut v = meta.clone();
                v.as_object_mut().unwrap().extend(json!({"event":"receiver_verification","phase":phase,"scope":s["scope"],"exact":true,"actual_bytes":meta["bytes"],"actual_sha256":meta["sha256"],"error":null}).as_object().unwrap().clone());
                verified.push(v);
                observations.push(json!({"event":"wan_request","request_ordinal":observations.len()+1,"scope":s["scope"],"http_status":200,"codec_cache":{"hits":0,"misses":0,"preparations":0,"evictions":0,"entries":0,"resident_bytes":0}}));
            }
            rr.push(json!({"event":"session_complete","seeds":1,"targets":1}));
            raw.push(json!({"event":"session","index":index,"session":s,"rows":rr,"raw_sha256":"dd".repeat(32)}));
        }
        verified[0]["observations"] = json!(observations);
        raw.push(json!({"event":"complete","sessions":p["sessions"].as_array().unwrap().len(),"operations":p["operations"]}));
        verified
            .push(json!({"event":"verification_complete","checked":p["operations"],"exact":true}));
        (raw, verified)
    }

    #[test]
    fn warm_complete_receipts_reject_dose_scope_seed_and_cost_corruption() {
        let p = plan("fixture-qualification", 0, &"1".repeat(40), true).unwrap();
        let (raw, verified) = fabricated_receipts(&p);
        let (operations, totals) = validate_receipts(&p, "plan", &raw, &verified).unwrap();
        assert_eq!((operations.len(), totals.len()), (24, 12));
        assert_eq!(totals[0]["cpu_us"], 61.0);
        let mut wrong = raw.clone();
        wrong[0]["dose"] = json!("wan");
        assert!(validate_receipts(&p, "plan", &wrong, &verified).is_err());
        wrong = raw.clone();
        wrong[1]["rows"].as_array_mut().unwrap().remove(2);
        assert!(validate_receipts(&p, "plan", &wrong, &verified).is_err());
        wrong = raw.clone();
        wrong[1]["rows"][3]["body_bytes"] = json!(599);
        assert!(validate_receipts(&p, "plan", &wrong, &verified).is_err());
        let mut wrong_verification = verified.clone();
        wrong_verification[1]["scope"] = json!("ee".repeat(32));
        assert!(validate_receipts(&p, "plan", &raw, &wrong_verification).is_err());
        wrong_verification = verified;
        wrong_verification[2]["exact"] = json!(false);
        assert!(validate_receipts(&p, "plan", &raw, &wrong_verification).is_err());
    }

    #[test]
    fn warm_summary_accepts_complete_local_receipts_and_charges_setup_once() {
        let root = temporary("summary");
        fs::create_dir(&root).unwrap();
        let p = plan("summary-qualification", 0, &"1".repeat(40), true).unwrap();
        let pp = root.join("plan.json");
        write_row(&mut fresh(&pp).unwrap(), &p).unwrap();
        let digest = sha(&fs::read(&pp).unwrap());
        let (mut raw, mut verified) = fabricated_receipts(&p);
        raw[0]["plan_sha256"] = json!(digest);
        verified[0]["plan_sha256"] = json!(digest);
        for row in &mut raw {
            if row["event"] == "session" {
                row["rows"][0]["plan_sha256"] = json!(digest);
            }
        }
        let rp = root.join("raw.jsonl");
        let vp = root.join("verify.jsonl");
        let mut out = fresh(&rp).unwrap();
        for r in raw {
            write_row(&mut out, &r).unwrap();
        }
        let mut out = fresh(&vp).unwrap();
        for r in verified {
            write_row(&mut out, &r).unwrap();
        }
        let paths = [pp, rp, vp].map(|p| p.to_string_lossy().into_owned());
        let result = summary(&paths, true).unwrap();
        assert_eq!(result["exact_reopened_objects"], 24);
        assert_eq!(result["cells"].as_array().unwrap().len(), 12);
        assert_eq!(result["paired_comparisons"].as_array().unwrap().len(), 6);
        assert_eq!(result["actual_lifecycles"][0]["cpu_us"], 61.0);
        assert_eq!(result["actual_lifecycles"][0]["wall_us"], 102.0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn warm_selection_summary_joins_controls_and_rejects_unpaid_fit_and_decision_changes() {
        let root = temporary("selector-summary");
        fs::create_dir(&root).unwrap();
        let p = selection_plan("selection-summary", 0, &"1".repeat(40), true).unwrap();
        let pp = root.join("plan.json");
        write_row(&mut fresh(&pp).unwrap(), &p).unwrap();
        let digest = sha(&fs::read(&pp).unwrap());
        let (mut raw, mut verified) = fabricated_receipts(&p);
        validate_receipts(&p, "plan", &raw, &verified).unwrap();
        let selector = raw
            .iter()
            .position(|r| r["event"] == "session" && r["session"]["arm"] == "selector")
            .unwrap();
        let mut wrong = raw.clone();
        wrong[selector]["rows"][1]["schema_fits"] = json!(0);
        assert!(validate_receipts(&p, "plan", &wrong, &verified).is_err());
        wrong = raw.clone();
        wrong[selector]["rows"][3]["selection"] = Value::Null;
        assert!(validate_receipts(&p, "plan", &wrong, &verified).is_err());
        raw[0]["plan_sha256"] = json!(digest);
        verified[0]["plan_sha256"] = json!(digest);
        for r in &mut raw {
            if r["event"] == "session" {
                r["rows"][0]["plan_sha256"] = json!(digest);
            }
        }
        let rp = root.join("raw.jsonl");
        let vp = root.join("verify.jsonl");
        let mut out = fresh(&rp).unwrap();
        for r in raw {
            write_row(&mut out, &r).unwrap();
        }
        let mut out = fresh(&vp).unwrap();
        for r in verified {
            write_row(&mut out, &r).unwrap();
        }
        let paths = [pp, rp, vp].map(|p| p.to_string_lossy().into_owned());
        let result = summary(&paths, true).unwrap();
        assert_eq!(result["exact_reopened_objects"], 36);
        assert_eq!(result["schema_fits"], 6);
        assert_eq!(result["cells"].as_array().unwrap().len(), 18);
        assert_eq!(result["paired_comparisons"].as_array().unwrap().len(), 18);
        assert_eq!(result["selection_decisions"].as_array().unwrap().len(), 6);
        assert!(result["actual_lifecycles"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["cpu_us"] == 61.0 && r["wall_us"] == 102.0));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn warm_fixture_changes_exactly_32_midpoint_bytes_and_has_unique_identity() {
        for large in [false, true] {
            let (base, changed) = fixture_pair(7, large).unwrap();
            assert_eq!(base.len(), if large { 16 << 20 } else { 1 << 20 });
            let positions = base
                .iter()
                .zip(&changed)
                .enumerate()
                .filter_map(|(i, (a, b))| (a != b).then_some(i))
                .collect::<Vec<_>>();
            assert_eq!(
                positions,
                (base.len() / 2..base.len() / 2 + 32).collect::<Vec<_>>()
            );
            assert_ne!(object_id(&base), object_id(&changed));
            assert_eq!(fixture_pair(7, large).unwrap(), (base, changed));
        }
        assert!(fixture_pair(30, false).is_err());
    }

    #[test]
    fn warm_success_cost_requires_real_seed_and_changed_sparse_update() {
        let row = json!({"error":null,"mode":"SparseV1","body_bytes":600,
            "request_body_bytes":400,"response_body_bytes":200,"batches":1,
            "sender_cpu_us":10,"receiver_cpu_us":20,"cpu_us":30,
            "wall_us":40,"exchange_wall_us":30});
        validate_operation(&row, "sparse", false, 1 << 20).unwrap();
        let mut corrupted = row.clone();
        corrupted["body_bytes"] = json!(599);
        assert!(validate_operation(&corrupted, "sparse", false, 1 << 20).is_err());
        assert!(validate_operation(&row, "sparse", true, 1 << 20).is_err());
        corrupted = row;
        corrupted["mode"] = json!("Reuse");
        assert!(validate_operation(&corrupted, "sparse", false, 1 << 20).is_err());
    }
}
