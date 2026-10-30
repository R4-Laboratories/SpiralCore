//! Frozen-training, held-out, exact-byte local transfer prototype.
use cpu_time::ProcessTime;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spiralcore_transfer::{
    client,
    endpoint::{ColdGraphPolicy, Endpoint},
    hash, hex, object_id,
    protocol::{Frame, Mode},
    schema,
    wan::{self, BenchAuthority},
    zstd_dictionary, Id,
};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
    time::Instant,
};

#[path = "support/schema_reuse.rs"]
mod schema_reuse;
#[path = "support/schema_sender.rs"]
mod schema_sender;

fn sha(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes).into())
}
fn sources() -> Value {
    json!({"harness":sha(include_bytes!("schema_fitting.rs")),
    "schema":sha(include_bytes!("../src/schema.rs")),"dictionary":sha(include_bytes!("../src/zstd_dictionary.rs")),
    "client":sha(include_bytes!("../src/client.rs")),"endpoint":sha(include_bytes!("../src/endpoint.rs")),
    "protocol":sha(include_bytes!("../src/protocol.rs")),"selection":sha(include_bytes!("../src/selection.rs")),
        "fitted_transfer":sha(include_bytes!("../src/fitted_transfer.rs")),"prefix":sha(include_bytes!("../src/zstd_prefix.rs")),
        "lockfile":sha(include_bytes!("../Cargo.lock")),"hardware_sha256":cfg!(feature="hardware-sha256")})
}
fn fresh(path: &Path) -> Result<File> {
    Ok(OpenOptions::new().write(true).create_new(true).open(path)?)
}
fn write_row(out: &mut File, row: &Value) -> Result<()> {
    serde_json::to_writer(&mut *out, row)?;
    writeln!(out)?;
    out.flush()?;
    Ok(())
}
fn save(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut out = fresh(path)?;
    out.write_all(bytes)?;
    out.sync_all()?;
    Ok(())
}

fn plan() -> Result<Value> {
    let training = training_records();
    let concatenated = training.concat();
    let mut targets = Vec::new();
    for trial in 0..10 {
        for large in [false, true] {
            for family in FAMILIES {
                let target = fixture(family, large, trial)?;
                targets.push(json!({"trial":trial,"large":large,"family":family,"bytes":target.bytes.len(),
            "records":target.records,"sha256":sha(&target.bytes),"object":hex(&object_id(&target.bytes))}));
            }
        }
    }
    Ok(
        json!({"schema":"spiralcore.schema-fitting.plan.v1","sources":sources(),"training_samples":training.len(),
        "training_bytes":concatenated.len(),"training_sha256":sha(&concatenated),"training_object":hex(&object_id(&concatenated)),
        "training_sample_sha256":training.iter().map(|r|sha(r)).collect::<Vec<_>>(),"dictionary_budget":8192,
        "trials":10,"families":FAMILIES,"arms":ARMS,"targets":targets}),
    )
}
#[derive(Default)]
struct Metrics {
    request: u64,
    response: u64,
    exchange_cpu: u128,
    exchange_wall: u128,
    receiver_cpu: u128,
    receiver_wall: u128,
    batches: usize,
}
struct Channel {
    endpoint: Endpoint<BenchAuthority>,
    scope: Id,
    metrics: Metrics,
    wire: Option<Vec<Vec<u8>>>,
}
impl Channel {
    fn open(path: &Path, scope: Id) -> Result<Self> {
        Ok(Self {
            endpoint: Endpoint::open_with_policy(
                path,
                wan::limits(),
                BenchAuthority,
                ColdGraphPolicy::Disabled,
            )?,
            scope,
            metrics: Metrics::default(),
            wire: None,
        })
    }
    fn exchange(&mut self, frames: &[Frame]) -> spiralcore_transfer::Result<Vec<Frame>> {
        let wall = Instant::now();
        let cpu = ProcessTime::now();
        let request = wan::encode_batch(frames)?;
        self.metrics.request += request.len() as u64;
        self.metrics.batches += 1;
        let receiver_wall = Instant::now();
        let receiver_cpu = ProcessTime::now();
        let decoded = wan::decode_batch(&request)?;
        let mut replies = Vec::new();
        for frame in &decoded {
            let reply = self
                .endpoint
                .handle(&wan::context(self.scope), frame, 0)
                .unwrap_or_else(|e| frame.reply(127, (e as u16).to_le_bytes().to_vec()));
            let terminal = matches!(reply.kind, 103 | 127);
            replies.push(reply);
            if terminal {
                break;
            }
        }
        let response = wan::encode_batch(&replies)?;
        self.metrics.receiver_cpu += receiver_cpu.elapsed().as_micros();
        self.metrics.receiver_wall += receiver_wall.elapsed().as_micros();
        self.metrics.response += response.len() as u64 + 16;
        let decoded = wan::decode_batch(&response);
        if let Some(wire) = &mut self.wire {
            wire.push(request);
            wire.push(response);
        }
        self.metrics.exchange_cpu += cpu.elapsed().as_micros();
        self.metrics.exchange_wall += wall.elapsed().as_micros();
        decoded
    }
}
fn operation(
    bytes: &[u8],
    artifact: Option<&[u8]>,
    arm: &str,
    seed: bool,
    scope: Id,
    path: &Path,
) -> Result<Value> {
    let mut channel = Channel::open(path, scope)?;
    let mut expected_cost = None;
    let transfer = hash(
        b"spiralcore.schema-fitting.transfer.v1\0",
        &[&scope, if seed { b"seed" } else { b"target" }],
    );
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let result = (|| -> spiralcore_transfer::Result<client::Stats> {
        let exchange = |frames: &[Frame]| channel.exchange(frames);
        if seed || arm == "raw" {
            return client::transfer_batched(bytes, Mode::Raw, None, transfer, false, exchange);
        }
        match arm {
            "zstd" => client::transfer_batched(bytes, Mode::Zstd, None, transfer, false, exchange),
            "adaptive" => {
                let result =
                    client::transfer_adaptive_batched(bytes, artifact, transfer, false, exchange)?;
                expected_cost = Some(result.selection.expected_http_body_bytes);
                Ok(result.transfer)
            }
            "schema" => client::transfer_schema_batched(
                artifact.ok_or(spiralcore_transfer::Error::Malformed)?,
                bytes,
                transfer,
                false,
                exchange,
            ),
            "dictionary" => client::transfer_zstd_dictionary_batched(
                artifact.ok_or(spiralcore_transfer::Error::Malformed)?,
                bytes,
                transfer,
                false,
                exchange,
            ),
            _ => Err(spiralcore_transfer::Error::Unsupported),
        }
    })();
    let cpu_us = cpu.elapsed().as_micros();
    let wall_us = wall.elapsed().as_micros();
    let metrics = channel.metrics;
    drop(channel.endpoint);
    let mut error = result.as_ref().err().map(ToString::to_string);
    let mut exact = false;
    let check_wall = Instant::now();
    let check_cpu = ProcessTime::now();
    if result.is_ok() {
        let mut endpoint = Endpoint::open_with_policy(
            path,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )?;
        exact = endpoint.read_object(&wan::context(scope), object_id(bytes))? == bytes;
        if !exact {
            error = Some("reopened bytes differ".into());
        }
    }
    if expected_cost.is_some_and(|n| n != metrics.request + metrics.response) {
        error = Some("adaptive complete-body cost differs".into());
    }
    Ok(
        json!({"event":"operation","phase":if seed {"seed"}else{"target"},"bytes":bytes.len(),"object":hex(&object_id(bytes)),
        "cpu_us":cpu_us,"wall_us":wall_us,"sender_cpu_us":cpu_us.saturating_sub(metrics.exchange_cpu),
        "exchange_cpu_us":metrics.exchange_cpu,"exchange_wall_us":metrics.exchange_wall,
        "receiver_cpu_us":metrics.receiver_cpu,"receiver_wall_us":metrics.receiver_wall,
        "request_body_bytes":metrics.request,"response_body_bytes":metrics.response,"body_bytes":metrics.request+metrics.response,"batches":metrics.batches,
        "mode":result.as_ref().ok().map(|s|format!("{:?}",s.mode)),"raw_fallback":!seed&&matches!(arm,"schema"|"dictionary")&&result.as_ref().is_ok_and(|s|s.mode==Mode::Raw),
        "exact_reopen":exact,"verification_cpu_us":check_cpu.elapsed().as_micros(),"verification_wall_us":check_wall.elapsed().as_micros(),"error":error}),
    )
}
fn arm_order(trial: usize, large: bool, family: usize) -> [&'static str; 5] {
    let mut arms = ARMS;
    let mut state = 0x4d19_672e_19c7_9a81u64
        ^ (trial as u64).rotate_left(9)
        ^ (family as u64).rotate_left(29)
        ^ u64::from(large);
    for i in (1..arms.len()).rev() {
        arms.swap(i, next(&mut state) as usize % (i + 1));
    }
    arms
}
fn run(
    plan_path: &Path,
    source: &str,
    out_path: &Path,
    cache: &Path,
    artifacts: &Path,
) -> Result<()> {
    if source.len() != 40
        || !source
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("invalid source commit".into());
    }
    let plan_bytes = fs::read(plan_path)?;
    let registered: Value = serde_json::from_slice(&plan_bytes)?;
    // Full fixture/manifest comparison occurs before fitting or transfer timing.
    if registered != plan()? {
        return Err("fixture/source plan changed".into());
    }
    let mut out = fresh(out_path)?;
    fs::create_dir(cache)?;
    fs::create_dir(artifacts)?;
    write_row(
        &mut out,
        &json!({"event":"registration","source_commit":source,"sources":sources(),"plan_sha256":sha(&plan_bytes),
        "executable_sha256":sha(&fs::read(std::env::current_exe()?)?),"target_operations":400,"seed_operations":240,
        "training_object":registered["training_object"],"training_bytes":registered["training_bytes"],
        "platform":format!("{}-{}",std::env::consts::ARCH,std::env::consts::OS),"scope":"local serialized durable transfer; body equivalent; sender and receiver CPU counted once"}),
    )?;
    let training = training_records();
    let training_bytes = training.concat();
    let samples: Vec<&[u8]> = training.iter().map(Vec::as_slice).collect();
    save(&artifacts.join("training.jsonl"), &training_bytes)?;
    let mut schema_artifact = Vec::new();
    let mut dictionary_artifact = Vec::new();
    for name in ["schema", "dictionary"] {
        let wall = Instant::now();
        let cpu = ProcessTime::now();
        let fitted = if name == "schema" {
            schema::fit(&samples)
        } else {
            zstd_dictionary::train(&samples, 8192)
        };
        let cpu_us = cpu.elapsed().as_micros();
        let wall_us = wall.elapsed().as_micros();
        match fitted {
            Ok(bytes) => {
                write_row(
                    &mut out,
                    &json!({"event":"fit","arm":name,"cpu_us":cpu_us,"wall_us":wall_us,"bytes":bytes.len(),"sha256":sha(&bytes),"object":hex(&object_id(&bytes)),"training_sha256":sha(&training_bytes),"actual_target_uses":80,"scope":"fit or training plus artifact serialization"}),
                )?;
                save(&artifacts.join(format!("{name}.artifact")), &bytes)?;
                if name == "schema" {
                    schema_artifact = bytes;
                } else {
                    dictionary_artifact = bytes;
                }
            }
            Err(e) => {
                write_row(
                    &mut out,
                    &json!({"event":"fit_failure","arm":name,"cpu_us":cpu_us,"wall_us":wall_us,"error":e.to_string()}),
                )?;
                return Err(e.into());
            }
        }
    }
    for trial in 0..10 {
        for large in [false, true] {
            for (family_index, family) in FAMILIES.into_iter().enumerate() {
                let target = fixture(family, large, trial)?;
                for arm in arm_order(trial, large, family_index) {
                    let artifact = match arm {
                        "adaptive" => Some(training_bytes.as_slice()),
                        "schema" => Some(schema_artifact.as_slice()),
                        "dictionary" => Some(dictionary_artifact.as_slice()),
                        _ => None,
                    };
                    let scope = hash(
                        b"spiralcore.schema-fitting.scope.v1\0",
                        &[
                            arm.as_bytes(),
                            family.as_bytes(),
                            &(trial as u64).to_le_bytes(),
                            &[u8::from(large)],
                        ],
                    );
                    let dir = cache.join(hex(&scope));
                    for seed in [true, false] {
                        if seed && artifact.is_none() {
                            continue;
                        }
                        let bytes = if seed {
                            artifact.unwrap()
                        } else {
                            &target.bytes
                        };
                        let result = operation(bytes, artifact, arm, seed, scope, &dir);
                        let mut row = match result {
                            Ok(row) => row,
                            Err(e) => {
                                write_row(
                                    &mut out,
                                    &json!({"event":"operation_failure","arm":arm,"trial":trial,"large":large,"family":family,"seed":seed,"error":e.to_string(),"cache_preserved":true}),
                                )?;
                                return Err(e);
                            }
                        };
                        row["arm"] = json!(arm);
                        row["family"] = json!(family);
                        row["trial"] = json!(trial);
                        row["large"] = json!(large);
                        row["target_records"] = json!(target.records);
                        write_row(&mut out, &row)?;
                        if row["exact_reopen"] != true || !row["error"].is_null() {
                            return Err("operation failed; cache retained".into());
                        }
                    }
                    fs::remove_dir_all(&dir)?;
                    write_row(
                        &mut out,
                        &json!({"event":"cleanup","arm":arm,"family":family,"trial":trial,"large":large,"owned_store_removed":true}),
                    )?;
                }
            }
        }
    }
    fs::remove_dir(cache)?;
    write_row(
        &mut out,
        &json!({"event":"complete","target_operations":400,"seed_operations":240,"owned_stores_removed":400,"schema_uses":80,"dictionary_uses":80,"adaptive_base_uses":80}),
    )
}
fn dist(mut values: Vec<f64>) -> Value {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n == 0 {
        return Value::Null;
    }
    json!({"n":n,"min":values[0],"median":(values[(n-1)/2]+values[n/2])/2.0,"p95":values[(95*n).div_ceil(100)-1],"max":values[n-1]})
}
fn numeric(row: &Value, key: &str) -> Result<f64> {
    row[key]
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or_else(|| format!("invalid {key}").into())
}
fn only<'a>(
    rows: &'a [Value],
    arm: &str,
    family: &str,
    large: bool,
    trial: usize,
    phase: &str,
) -> Result<&'a Value> {
    let mut found = rows.iter().filter(|r| {
        r["event"] == "operation"
            && r["arm"] == arm
            && r["family"] == family
            && r["large"] == large
            && r["trial"] == trial
            && r["phase"] == phase
    });
    let row = found.next().ok_or("missing paired operation")?;
    if found.next().is_some() {
        return Err("duplicate paired operation".into());
    }
    Ok(row)
}
fn summarize(rows: &[Value]) -> Result<Value> {
    let count = |event| rows.iter().filter(|r| r["event"] == event).count();
    if rows.len() != 1044
        || count("registration") != 1
        || count("fit") != 2
        || count("operation") != 640
        || count("cleanup") != 400
        || count("complete") != 1
    {
        return Err("registered campaign counts differ".into());
    }
    let mut cells = Vec::new();
    let mut totals = Vec::new();
    for arm in ARMS {
        let fit = rows.iter().find(|r| r["event"] == "fit" && r["arm"] == arm);
        let fit_cpu = fit
            .map(|r| numeric(r, "cpu_us"))
            .transpose()?
            .unwrap_or(0.0);
        let fit_wall = fit
            .map(|r| numeric(r, "wall_us"))
            .transpose()?
            .unwrap_or(0.0);
        let mut total_cpu = fit_cpu;
        let mut total_wall = fit_wall;
        let mut total_body = 0.0;
        for large in [false, true] {
            for family in FAMILIES {
                let mut group = Vec::new();
                let mut cold_cpu = Vec::new();
                let mut cold_wall = Vec::new();
                let mut cold_body = Vec::new();
                let mut ratios = Vec::new();
                for trial in 0..10 {
                    let row = only(rows, arm, family, large, trial, "target")?;
                    let raw = only(rows, "raw", family, large, trial, "target")?;
                    if row["object"] != raw["object"]
                        || row["bytes"] != raw["bytes"]
                        || row["exact_reopen"] != true
                        || !row["error"].is_null()
                    {
                        return Err("paired fidelity failure".into());
                    }
                    if numeric(row, "body_bytes")?
                        != numeric(row, "request_body_bytes")?
                            + numeric(row, "response_body_bytes")?
                        || numeric(row, "cpu_us")? < numeric(row, "exchange_cpu_us")?
                        || numeric(row, "exchange_cpu_us")? < numeric(row, "receiver_cpu_us")?
                    {
                        return Err("byte/timer accounting failure".into());
                    }
                    let mut cpu = numeric(row, "cpu_us")?;
                    let mut wall = numeric(row, "wall_us")?;
                    let mut body = numeric(row, "body_bytes")?;
                    if matches!(arm, "adaptive" | "schema" | "dictionary") {
                        let seed = only(rows, arm, family, large, trial, "seed")?;
                        if seed["exact_reopen"] != true || !seed["error"].is_null() {
                            return Err("seed verification failed".into());
                        }
                        if let Some(model) = fit {
                            if seed["object"] != model["object"] || seed["bytes"] != model["bytes"]
                            {
                                return Err("artifact seed differs".into());
                            }
                        }
                        cpu += numeric(seed, "cpu_us")?;
                        wall += numeric(seed, "wall_us")?;
                        body += numeric(seed, "body_bytes")?;
                    }
                    total_cpu += cpu;
                    total_wall += wall;
                    total_body += body;
                    cold_cpu.push(cpu + fit_cpu);
                    cold_wall.push(wall + fit_wall);
                    cold_body.push(body);
                    ratios.push(numeric(raw, "cpu_us")? / numeric(row, "cpu_us")?);
                    group.push(row);
                }
                let mut cell = json!({"arm":arm,"family":family,"large":large,"raw_fallbacks":group.iter().filter(|r|r["raw_fallback"]==true).count()});
                for metric in [
                    "bytes",
                    "body_bytes",
                    "cpu_us",
                    "sender_cpu_us",
                    "wall_us",
                    "exchange_cpu_us",
                    "exchange_wall_us",
                    "receiver_cpu_us",
                    "receiver_wall_us",
                ] {
                    cell[metric] = dist(
                        group
                            .iter()
                            .map(|r| numeric(r, metric))
                            .collect::<Result<Vec<_>>>()?,
                    );
                }
                cell["raw_cpu_speedup_geomean"] =
                    json!((ratios.iter().map(|v| v.ln()).sum::<f64>() / 10.0).exp());
                cell["cold_fit_seed_target_cpu_us"] = dist(cold_cpu);
                cell["cold_fit_seed_target_wall_us"] = dist(cold_wall);
                cell["cold_seed_target_body_bytes"] = dist(cold_body);
                cells.push(cell);
            }
        }
        let retained = if arm == "adaptive" {
            rows[0]["training_bytes"].clone()
        } else {
            fit.map(|r| r["bytes"].clone()).unwrap_or(json!(0))
        };
        totals.push(json!({"arm":arm,"actual_target_uses":80,"fit_cpu_us":fit_cpu,"fit_wall_us":fit_wall,"owned_artifact_bytes":retained,
            "actual_campaign_cpu_us":total_cpu,"actual_campaign_wall_us":total_wall,"actual_campaign_body_bytes":total_body,
            "cpu_us_per_use_including_fit_and_actual_seeds":total_cpu/80.0,"wall_us_per_use_including_fit_and_actual_seeds":total_wall/80.0,"body_bytes_per_use_including_actual_seeds":total_body/80.0}));
    }
    if rows
        .iter()
        .filter(|r| r["event"] == "cleanup")
        .any(|r| r["owned_store_removed"] != true)
    {
        return Err("cleanup failed".into());
    }
    Ok(
        json!({"schema":"spiralcore.schema-fitting.summary.v1","verdict":"EXACT_PAIRED_CAMPAIGN_PASS","target_operations":400,"seed_operations":240,"discarded_observations":0,"fits":rows.iter().filter(|r|r["event"]=="fit").collect::<Vec<_>>(),"cells":cells,"actual_reuse_totals":totals}),
    )
}
fn command() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s.starts_with("reuse-")) {
        return schema_reuse::command(&args);
    }
    if args.get(1).is_some_and(|s| s.starts_with("sender-")) {
        return schema_sender::command(&args);
    }
    match args.get(1).map(String::as_str) {
        Some("plan") if args.len()==3=>{let mut out=fresh(Path::new(&args[2]))?;write_row(&mut out,&plan()?)},
        Some("run") if args.len()==7=>run(Path::new(&args[2]),&args[3],Path::new(&args[4]),Path::new(&args[5]),Path::new(&args[6])),
        Some("summarize") if args.len()==4=>{let raw=fs::read(&args[2])?;let rows=std::str::from_utf8(&raw)?.lines().map(serde_json::from_str::<Value>).collect::<std::result::Result<Vec<_>,_>>()?;let mut summary=summarize(&rows)?;summary["raw_sha256"]=json!(sha(&raw));write_row(&mut fresh(Path::new(&args[3]))?,&summary)},
        _=>Err("usage: schema_fitting plan NEW_PLAN | run PLAN SOURCE_COMMIT NEW_JSONL NEW_CACHE NEW_ARTIFACT_DIR | summarize JSONL NEW_SUMMARY".into())
    }
}
fn main() {
    if let Err(error) = command() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const FAMILIES: [&str; 4] = ["normal", "variants", "drift", "unrelated"];
const ARMS: [&str; 5] = ["raw", "zstd", "adaptive", "schema", "dictionary"];
struct Fixture {
    bytes: Vec<u8>,
    records: usize,
}
fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}
fn record(training: bool, index: usize, trial: usize, family: &str) -> Vec<u8> {
    let mut random = (if training {
        0x529d_12ae_063f_8ab1u64
    } else {
        0xb80e_9da1_39a7_c62fu64
    }) ^ (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (trial as u64).wrapping_mul(0xd1b5_4a32_d192_ed03);
    let value = next(&mut random);
    let seq = if training {
        index
    } else {
        1_000_000 + trial * 10_000 + index
    };
    if family == "drift" {
        let message = format!("{:016x}", next(&mut random)).repeat(6 + (value as usize % 7));
        return format!(r#"{{"event":"status-v2","origin":"edge-{}","counter":{},"epoch":{},"reading":"{}e-3","quality":null,"message":"{}","schema":2}}
"#,value%197,seq,1_800_000_000u64+seq as u64,value%100000,message).into_bytes();
    }
    let mut fields: Vec<(&str, String)> = vec![
        ("type", r#""telemetry""#.into()),
        ("site", format!(r#""site-{:02}""#, value % 8)),
        (
            "node",
            format!(
                r#""node-{:06}""#,
                if training {
                    value % 512
                } else {
                    50_000 + value % 512
                }
            ),
        ),
        ("seq", seq.to_string()),
        ("tick", (1_700_000_000u64 + seq as u64).to_string()),
        (
            "temp",
            format!("{}.{:02}", 18 + value % 20, next(&mut random) % 100),
        ),
        ("ok", (!value.is_multiple_of(3)).to_string()),
        (
            "note",
            format!(
                r#""{}""#,
                [
                    "normal operating sample",
                    "scheduled sensor report",
                    "steady observation",
                    "periodic edge reading"
                ][value as usize % 4]
            ),
        ),
        (
            "status",
            format!(r#""{}""#, ["nominal", "idle", "warm"][value as usize % 3]),
        ),
        ("revision", (1 + value % 3).to_string()),
        ("units", r#""C""#.into()),
        ("trace", format!(r#""{:016x}""#, next(&mut random))),
    ];
    let variant = (index + trial) % 4;
    if family == "variants" {
        match variant {
            0 => (),
            1 => fields.rotate_left(4),
            2 => {
                fields.retain(|(key, _)| *key != "units");
                fields.push(("calibration", r#""v2""#.into()));
            }
            _ => {
                fields.iter_mut().find(|(k, _)| *k == "temp").unwrap().1 =
                    format!("{}.{:02}e+0", 18 + value % 20, value % 100);
                fields.iter_mut().find(|(k, _)| *k == "note").unwrap().1 =
                    r#""line\nword\u0020end\/""#.into();
            }
        }
    }
    let spaced = family == "variants" && variant == 0;
    let body = fields
        .into_iter()
        .map(|(key, value)| format!("\"{key}\"{}{value}", if spaced { ": " } else { ":" }))
        .collect::<Vec<_>>()
        .join(if spaced { ", " } else { "," });
    format!(
        "{{{}{body}{}}}{}",
        if spaced { " " } else { "" },
        if spaced { " " } else { "" },
        if spaced { "\r\n" } else { "\n" }
    )
    .into_bytes()
}
fn training_records() -> Vec<Vec<u8>> {
    (0..512).map(|i| record(true, i, 0, "normal")).collect()
}
fn fixture(family: &str, large: bool, trial: usize) -> Result<Fixture> {
    if !FAMILIES.contains(&family) || trial >= 100 {
        return Err("invalid fixture configuration".into());
    }
    let mut bytes = Vec::new();
    let mut records = 0;
    loop {
        bytes.extend(record(
            false,
            records,
            trial,
            if family == "unrelated" {
                "normal"
            } else {
                family
            },
        ));
        records += 1;
        if !large || bytes.len() >= 1_048_576 {
            break;
        }
    }
    if family == "unrelated" {
        let mut state =
            0x6f51_bdad_47f0_1489u64 ^ (trial as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        for byte in &mut bytes {
            *byte = next(&mut state) as u8;
        }
        records = 0;
    }
    Ok(Fixture { bytes, records })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn training_is_frozen_and_disjoint_from_held_out_records() {
        let training = training_records();
        assert_eq!(training.len(), 512);
        assert_eq!(training, training_records());
        assert!(training.iter().all(|r| r.ends_with(b"\n")));
        assert!(training.iter().all(|r| serde_json::from_slice::<Value>(r)
            .unwrap()
            .as_object()
            .unwrap()
            .len()
            == 12));
        for trial in 0..10 {
            let target = fixture("normal", false, trial).unwrap();
            assert!(!training.contains(&target.bytes));
            assert_eq!(target.records, 1);
            for family in ["normal", "variants", "drift"] {
                assert!(serde_json::from_slice::<Value>(
                    &fixture(family, false, trial).unwrap().bytes
                )
                .unwrap()
                .is_object());
            }
        }
    }
    #[test]
    fn whole_record_batches_and_binary_controls_preserve_actual_lengths() {
        for family in ["normal", "variants", "drift"] {
            let target = fixture(family, true, 2).unwrap();
            assert!(target.bytes.len() >= 1_048_576 && target.bytes.len() < 1_050_000);
            assert_eq!(
                target.records,
                target.bytes.iter().filter(|b| **b == b'\n').count()
            );
            assert!(target.bytes.ends_with(b"\n"));
        }
        let normal = fixture("normal", true, 2).unwrap();
        let unrelated = fixture("unrelated", true, 2).unwrap();
        assert_eq!(normal.bytes.len(), unrelated.bytes.len());
        assert_ne!(normal.bytes, unrelated.bytes);
        assert!(unrelated.bytes.contains(&0));
        assert_eq!(
            unrelated.bytes,
            fixture("unrelated", true, 2).unwrap().bytes
        );
        assert!(fixture("unknown", false, 0).is_err());
    }
}
