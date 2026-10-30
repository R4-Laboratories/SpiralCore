//! Local CPU experiment. Compile this identical source against the preserved
//! baseline and candidate. Only the candidate enables `--cfg replay_cpu_prepared`.
#![allow(unexpected_cfgs)]
use cpu_time::ProcessTime;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spiralcore_transfer::{
    client,
    endpoint::{ColdGraphPolicy, Endpoint},
    hash, hex, object_id,
    protocol::{Frame, Mode},
    shared_runtime::{self, OutputKind, Recipe, State},
    wan::{self, BenchAuthority},
    Id,
};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
    process::Command,
    time::Instant,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const ARMS: [&str; 6] = ["baseline", "packed", "prepared", "raw", "zstd", "adaptive"];
const CASES: [&str; 3] = ["generated", "edited", "unrelated"];

fn sha(b: &[u8]) -> String {
    hex(&Sha256::digest(b).into())
}
fn next(random: &mut u64) -> u64 {
    *random ^= *random << 13;
    *random ^= *random >> 7;
    *random ^= *random << 17;
    *random
}
fn target_for(case: &str, prediction: &[u8], seed: u64) -> Result<Vec<u8>> {
    if prediction.len() < 18 {
        return Err("prediction too short".into());
    }
    let mut target = prediction.to_vec();
    let mut random = if seed == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        seed
    };
    match case {
        "generated" => (),
        "edited" => target[prediction.len() / 2] ^= 1,
        "unrelated" => target.iter_mut().for_each(|b| *b = next(&mut random) as u8),
        _ => return Err("unknown fixture".into()),
    }
    Ok(target)
}
fn arm_order(width: usize, trial: usize) -> [&'static str; 6] {
    let mut arms = ARMS;
    let mut random = 0xc810_53ca_f645_1885 ^ (width as u64).rotate_left(27) ^ trial as u64;
    for i in (1..arms.len()).rev() {
        arms.swap(i, next(&mut random) as usize % (i + 1));
    }
    arms
}
fn emit(row: &Value) -> Result<()> {
    let mut out = std::io::stdout().lock();
    serde_json::to_writer(&mut out, row)?;
    writeln!(out)?;
    out.flush()?;
    Ok(())
}
fn pins() -> Value {
    json!({"harness":sha(include_bytes!("replay_cpu.rs")),
        "runtime":sha(include_bytes!("../src/shared_runtime.rs")),
        "transfer":sha(include_bytes!("../src/shared_transfer.rs")),
        "client":sha(include_bytes!("../src/client.rs")),
        "endpoint":sha(include_bytes!("../src/endpoint.rs")),
        "selection":sha(include_bytes!("../src/selection.rs")),
        "lockfile":sha(include_bytes!("../Cargo.lock")),
        "prepared_api":cfg!(replay_cpu_prepared)})
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
        for f in &decoded {
            let reply = self
                .endpoint
                .handle(&wan::context(self.scope), f, 0)
                .unwrap_or_else(|e| f.reply(127, (e as u16).to_le_bytes().to_vec()));
            let terminal = matches!(reply.kind, 103 | 127);
            replies.push(reply);
            if terminal {
                break;
            }
        }
        let response = wan::encode_batch(&replies)?;
        self.metrics.receiver_cpu += receiver_cpu.elapsed().as_micros();
        self.metrics.receiver_wall += receiver_wall.elapsed().as_micros();
        // SCWREP01 contains the same frames plus two u64 receiver metrics.
        self.metrics.response += response.len() as u64 + 16;
        let decoded = wan::decode_batch(&response);
        self.metrics.exchange_cpu += cpu.elapsed().as_micros();
        self.metrics.exchange_wall += wall.elapsed().as_micros();
        decoded
    }
}

#[cfg(replay_cpu_prepared)]
type Prepared = spiralcore_transfer::shared_transfer::PreparedPrediction;
#[cfg(not(replay_cpu_prepared))]
struct Prepared;

struct Fixture {
    checkpoint: Vec<u8>,
    prediction: Vec<u8>,
    recipe: Recipe,
}
fn operation(
    fixture: &Fixture,
    target: &[u8],
    prepared: Option<&Prepared>,
    arm: &str,
    phase: &str,
    scope: Id,
    cache: &Path,
) -> Result<Value> {
    let bytes = if phase == "seed" {
        &fixture.checkpoint
    } else {
        target
    };
    let id = hash(
        b"spiralcore.replay-cpu.operation.v1\0",
        &[&scope, phase.as_bytes()],
    );
    let mut channel = Channel::open(cache, scope)?;
    let mut selected_cost = None;
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let result = (|| -> spiralcore_transfer::Result<client::Stats> {
        let exchange = |frames: &[Frame]| channel.exchange(frames);
        if phase == "seed" || arm == "raw" {
            return client::transfer_batched(bytes, Mode::Raw, None, id, false, exchange);
        }
        if arm == "zstd" {
            return client::transfer_batched(bytes, Mode::Zstd, None, id, false, exchange);
        }
        let result = match arm {
            "adaptive" => client::transfer_adaptive_batched(
                bytes,
                Some(&fixture.checkpoint),
                id,
                false,
                exchange,
            ),
            "baseline" | "packed" => client::transfer_runtime_batched(
                &fixture.checkpoint,
                bytes,
                &fixture.recipe,
                id,
                false,
                exchange,
            ),
            "prepared" => {
                #[cfg(replay_cpu_prepared)]
                {
                    client::transfer_prepared_runtime_batched(
                        prepared.ok_or(spiralcore_transfer::Error::Malformed)?,
                        bytes,
                        id,
                        false,
                        exchange,
                    )
                }
                #[cfg(not(replay_cpu_prepared))]
                {
                    let _ = prepared;
                    Err(spiralcore_transfer::Error::Unsupported)
                }
            }
            _ => Err(spiralcore_transfer::Error::Malformed),
        }?;
        selected_cost = Some(result.selection.expected_http_body_bytes);
        Ok(result.transfer)
    })();
    let cpu_us = cpu.elapsed().as_micros();
    let wall_us = wall.elapsed().as_micros();
    let m = channel.metrics;
    drop(channel.endpoint);
    let mut error = result.as_ref().err().map(ToString::to_string);
    let check_wall = Instant::now();
    let check_cpu = ProcessTime::now();
    let mut exact = false;
    if result.is_ok() {
        let mut endpoint = Endpoint::open_with_policy(
            cache,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )?;
        exact = endpoint.read_object(&wan::context(scope), object_id(bytes))? == *bytes;
        if !exact {
            error = Some("independent reopened bytes differ".into());
        }
    }
    if selected_cost.is_some_and(|n| n != m.request + m.response) {
        error = Some("selected body cost differs from serialized exchange".into());
    }
    Ok(
        json!({"event":"operation", "phase":phase, "bytes":bytes.len(), "object":hex(&object_id(bytes)),
        "cpu_us":cpu_us,"wall_us":wall_us,"exchange_cpu_us":m.exchange_cpu,
        "exchange_wall_us":m.exchange_wall,"receiver_cpu_us":m.receiver_cpu,
        "receiver_wall_us":m.receiver_wall,"request_body_bytes":m.request,
        "response_body_bytes":m.response,"body_bytes":m.request+m.response,"batches":m.batches,
        "selected":result.as_ref().ok().map(|s| format!("{:?}",s.mode)),"selected_cost":selected_cost,
        "exact_reopen":exact,"verification_cpu_us":check_cpu.elapsed().as_micros(),
        "verification_wall_us":check_wall.elapsed().as_micros(),"error":error}),
    )
}

fn worker(arm: &str, width: usize, trial: usize, cache: &Path) -> Result<()> {
    if !ARMS.contains(&arm) || ![48, 4096].contains(&width) || trial >= 10 {
        return Err("invalid worker configuration".into());
    }
    if arm == "prepared" && !cfg!(replay_cpu_prepared) {
        return Err("baseline binary has no prepared API".into());
    }
    fs::create_dir(cache)?;
    let steps = if width == 48 { 243 } else { 4096 };
    let checkpoint = State::seeded(width)?.encode()?;
    let recipe = Recipe {
        steps,
        payload: 1.0,
        output: OutputKind::Trace,
    };
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let prediction = shared_runtime::replay(&checkpoint, &recipe, shared_runtime::MAX_OUTPUT)?;
    let replay_cpu_us = cpu.elapsed().as_micros();
    let replay_wall_us = wall.elapsed().as_micros();
    let expected = if width == 48 { 142738 } else { 14851384 };
    if prediction.len() != expected {
        return Err("registered actual output size changed".into());
    }
    let fixture = Fixture {
        checkpoint,
        prediction,
        recipe,
    };
    let seed = hash(
        b"spiralcore.runtime.measurement.fixture.v1\0",
        &[
            &(width as u64).to_le_bytes(),
            &steps.to_le_bytes(),
            &(trial as u64).to_le_bytes(),
        ],
    );
    let seed = u64::from_le_bytes(seed[..8].try_into()?);
    emit(
        &json!({"event":"worker", "arm":arm,"width":width,"steps":steps,"trial":trial,
        "source":pins(),"executable_sha256":sha(&fs::read(std::env::current_exe()?)?),
        "checkpoint_sha256":sha(&fixture.checkpoint),"prediction_sha256":sha(&fixture.prediction),
        "checkpoint_object":hex(&object_id(&fixture.checkpoint)),"prediction_object":hex(&object_id(&fixture.prediction)),
        "checkpoint_bytes":fixture.checkpoint.len(),"prediction_bytes":fixture.prediction.len(),
        "replay_cpu_us":replay_cpu_us,"replay_wall_us":replay_wall_us,"seed":seed}),
    )?;
    let prepared: Option<Prepared>;
    #[cfg(replay_cpu_prepared)]
    {
        prepared = if arm == "prepared" {
            let wall = Instant::now();
            let cpu = ProcessTime::now();
            let p = Prepared::new(&fixture.checkpoint, &fixture.recipe)?;
            let cpu_us = cpu.elapsed().as_micros();
            let wall_us = wall.elapsed().as_micros();
            if p.prediction() != fixture.prediction || p.checkpoint() != fixture.checkpoint {
                return Err("prepared prediction changed bytes".into());
            }
            emit(
                &json!({"event":"preparation","arm":arm,"width":width,"trial":trial,
            "cpu_us":cpu_us,"wall_us":wall_us,"retained_bytes":p.retained_bytes(),"actual_uses":3}),
            )?;
            Some(p)
        } else {
            None
        };
    }
    #[cfg(not(replay_cpu_prepared))]
    {
        prepared = None;
    }
    for case in CASES {
        let target = target_for(case, &fixture.prediction, seed)?;
        let scope = hash(
            b"spiralcore.replay-cpu.scope.v1\0",
            &[
                arm.as_bytes(),
                case.as_bytes(),
                &(width as u64).to_le_bytes(),
                &(trial as u64).to_le_bytes(),
            ],
        );
        let dir = cache.join(case);
        for phase in ["seed", "target"] {
            let mut row = operation(
                &fixture,
                &target,
                prepared.as_ref(),
                arm,
                phase,
                scope,
                &dir,
            )?;
            row["arm"] = json!(arm);
            row["case"] = json!(case);
            row["width"] = json!(width);
            row["trial"] = json!(trial);
            emit(&row)?;
            if !row["error"].is_null() || row["exact_reopen"] != true {
                return Err("operation failed; cache retained".into());
            }
        }
        fs::remove_dir_all(&dir)?;
    }
    #[cfg(replay_cpu_prepared)]
    drop(prepared);
    fs::remove_dir(cache)?;
    emit(
        &json!({"event":"cleanup","arm":arm,"width":width,"trial":trial,"owned_cache_removed":true}),
    )
}

fn write_row(out: &mut File, row: &Value) -> Result<()> {
    serde_json::to_writer(&mut *out, row)?;
    writeln!(out)?;
    out.flush()?;
    Ok(())
}
fn run(baseline: &Path, candidate: &Path, out: &Path, cache: &Path, profile: bool) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(out)?;
    fs::create_dir(cache)?;
    write_row(
        &mut file,
        &json!({"event":"registration","profile":profile,"trials":if profile {3}else{10},
        "baseline_executable_sha256":sha(&fs::read(baseline)?),"candidate_executable_sha256":sha(&fs::read(candidate)?),
        "driver_source":pins(),"sizes":[142738,14851384],"cases":CASES,"arms":ARMS,
        "scope":"single-process serialized durable local mechanism; body equivalent, not HTTP or WAN", "discarded_warmups":0}),
    )?;
    for trial in 0..if profile { 3 } else { 10 } {
        for width in [48, 4096] {
            let order = if profile {
                vec!["baseline"]
            } else {
                arm_order(width, trial).to_vec()
            };
            for arm in order {
                let exe = if arm == "baseline" {
                    baseline
                } else {
                    candidate
                };
                let dir = cache.join(format!("{width}-{trial}-{arm}"));
                let result = Command::new(exe)
                    .args(["worker", arm, &width.to_string(), &trial.to_string()])
                    .arg(&dir)
                    .output();
                let output = match result {
                    Ok(output) => output,
                    Err(e) => {
                        write_row(
                            &mut file,
                            &json!({"event":"worker_spawn_failure","arm":arm,"width":width,"trial":trial,"error":e.to_string()}),
                        )?;
                        return Err(e.into());
                    }
                };
                let parsed = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(serde_json::from_str::<Value>)
                    .collect::<std::result::Result<Vec<_>, _>>();
                if !output.status.success()
                    || parsed.is_err()
                    || parsed.as_ref().is_ok_and(|rows| rows.is_empty())
                {
                    write_row(
                        &mut file,
                        &json!({"event":"worker_failure","arm":arm,"width":width,"trial":trial,"exit":output.status.code(),
                        "stdout":String::from_utf8_lossy(&output.stdout),"stderr":String::from_utf8_lossy(&output.stderr)}),
                    )?;
                    return Err("worker failed; outputs and cache retained".into());
                }
                for row in parsed? {
                    write_row(&mut file, &row)?;
                }
                if !output.stderr.is_empty() {
                    write_row(
                        &mut file,
                        &json!({"event":"worker_stderr","arm":arm,"width":width,"trial":trial,"text":String::from_utf8_lossy(&output.stderr)}),
                    )?;
                }
            }
        }
    }
    fs::remove_dir(cache)?;
    write_row(
        &mut file,
        &json!({"event":"complete","owned_cache_removed":true}),
    )
}
fn distribution(mut values: Vec<f64>) -> Value {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n == 0 {
        return Value::Null;
    }
    let median = (values[(n - 1) / 2] + values[n / 2]) / 2.0;
    json!({"n":n,"min":values[0],"median":median,"p95":values[(95*n).div_ceil(100)-1],"max":values[n-1]})
}
fn one<'a>(
    rows: &'a [Value],
    event: &str,
    width: u64,
    trial: u64,
    arm: &str,
    case: Option<&str>,
    phase: Option<&str>,
) -> Result<&'a Value> {
    let mut matches = rows.iter().filter(|r| {
        r["event"] == event
            && r["width"] == width
            && r["trial"] == trial
            && r["arm"] == arm
            && case.is_none_or(|v| r["case"] == v)
            && phase.is_none_or(|v| r["phase"] == v)
    });
    let row = matches.next().ok_or("missing paired row")?;
    if matches.next().is_some() {
        return Err("duplicate paired row".into());
    }
    Ok(row)
}
fn number(row: &Value, key: &str) -> Result<f64> {
    row[key]
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or_else(|| format!("invalid {key}").into())
}
fn summarize(rows: &[Value]) -> Result<Value> {
    let count = |event| rows.iter().filter(|r| r["event"] == event).count();
    if count("registration") != 1
        || count("complete") != 1
        || count("worker") != 120
        || count("preparation") != 20
        || count("operation") != 720
        || count("cleanup") != 120
        || rows.len() != 982
    {
        return Err("campaign must contain exactly 120 workers,20 preparations,720 operations,120 cleanups and two bookends".into());
    }
    let registration = &rows[0];
    if registration["event"] != "registration"
        || registration["profile"] != false
        || rows.last().unwrap()["event"] != "complete"
    {
        return Err("invalid campaign bookends".into());
    }
    for width in [48, 4096] {
        for trial in 0..10 {
            let base = one(rows, "worker", width, trial, "baseline", None, None)?;
            for arm in ARMS {
                let worker = one(rows, "worker", width, trial, arm, None, None)?;
                for key in [
                    "checkpoint_sha256",
                    "prediction_sha256",
                    "checkpoint_object",
                    "prediction_object",
                    "checkpoint_bytes",
                    "prediction_bytes",
                    "seed",
                ] {
                    if worker[key] != base[key] {
                        return Err(format!("paired {key} differs").into());
                    }
                }
                let expected_exe = if arm == "baseline" {
                    &registration["baseline_executable_sha256"]
                } else {
                    &registration["candidate_executable_sha256"]
                };
                if &worker["executable_sha256"] != expected_exe {
                    return Err("worker executable pin differs".into());
                }
                if arm == "baseline" {
                    if worker["source"]["runtime"]
                        != "c956c80f88e401751f594b7540c587e6213b8fe33e70a776d663c7259992bbb8"
                        || worker["source"]
                            != one(rows, "worker", 48, 0, "baseline", None, None)?["source"]
                    {
                        return Err("baseline source differs".into());
                    }
                } else if worker["source"] != registration["driver_source"] {
                    return Err("candidate source differs from driver".into());
                }
                if one(rows, "cleanup", width, trial, arm, None, None)?["owned_cache_removed"]
                    != true
                {
                    return Err("cleanup missing".into());
                }
                for case in CASES {
                    for phase in ["seed", "target"] {
                        let row = one(
                            rows,
                            "operation",
                            width,
                            trial,
                            arm,
                            Some(case),
                            Some(phase),
                        )?;
                        let paired = one(
                            rows,
                            "operation",
                            width,
                            trial,
                            "baseline",
                            Some(case),
                            Some(phase),
                        )?;
                        if row["object"] != paired["object"]
                            || row["bytes"] != paired["bytes"]
                            || row["exact_reopen"] != true
                            || !row["error"].is_null()
                        {
                            return Err("fidelity or pairing failed".into());
                        }
                        if phase == "seed" && row["object"] != worker["checkpoint_object"] {
                            return Err("checkpoint object differs".into());
                        }
                        if phase == "target"
                            && case == "generated"
                            && row["object"] != worker["prediction_object"]
                        {
                            return Err("generated trace differs".into());
                        }
                        if ["baseline", "packed", "prepared"].contains(&arm)
                            && row["body_bytes"] != paired["body_bytes"]
                        {
                            return Err("runtime paired byte cost differs".into());
                        }
                        if number(row, "body_bytes")?
                            != number(row, "request_body_bytes")?
                                + number(row, "response_body_bytes")?
                        {
                            return Err("body sum differs".into());
                        }
                        if !row["selected_cost"].is_null()
                            && row["selected_cost"] != row["body_bytes"]
                        {
                            return Err("selection cost differs".into());
                        }
                        if number(row, "cpu_us")? < number(row, "exchange_cpu_us")?
                            || number(row, "exchange_cpu_us")? < number(row, "receiver_cpu_us")?
                            || number(row, "wall_us")? < number(row, "exchange_wall_us")?
                            || number(row, "exchange_wall_us")? < number(row, "receiver_wall_us")?
                        {
                            return Err("nested timer scope invalid".into());
                        }
                    }
                }
            }
            let prep = one(rows, "preparation", width, trial, "prepared", None, None)?;
            if prep["actual_uses"] != 3
                || number(prep, "retained_bytes")?
                    != number(base, "prediction_bytes")? + number(base, "checkpoint_bytes")? + 69.0
            {
                return Err("preparation retention/use count differs".into());
            }
        }
    }
    let mut cells = Vec::new();
    let mut amortized = Vec::new();
    let mut replay = Vec::new();
    for width in [48, 4096] {
        for arm in ARMS {
            let workers = (0..10)
                .map(|t| one(rows, "worker", width, t, arm, None, None))
                .collect::<Result<Vec<_>>>()?;
            replay.push(json!({"width":width,"arm":arm,
                "cpu_us":distribution(workers.iter().map(|r| number(r,"replay_cpu_us")).collect::<Result<Vec<_>>>()?),
                "wall_us":distribution(workers.iter().map(|r| number(r,"replay_wall_us")).collect::<Result<Vec<_>>>()?)}));
            for case in CASES {
                let mut cell = json!({"width":width,"case":case,"arm":arm});
                let group = (0..10)
                    .map(|t| one(rows, "operation", width, t, arm, Some(case), Some("target")))
                    .collect::<Result<Vec<_>>>()?;
                for metric in [
                    "body_bytes",
                    "cpu_us",
                    "wall_us",
                    "exchange_cpu_us",
                    "exchange_wall_us",
                    "receiver_cpu_us",
                    "receiver_wall_us",
                ] {
                    cell[metric] = distribution(
                        group
                            .iter()
                            .map(|r| number(r, metric))
                            .collect::<Result<Vec<_>>>()?,
                    );
                }
                let mut ratios = Vec::new();
                let mut cold_cpu = Vec::new();
                let mut cold_wall = Vec::new();
                let mut practical_cpu = Vec::new();
                let mut practical_wall = Vec::new();
                let mut practical_body = Vec::new();
                for trial in 0..10 {
                    let row = group[trial as usize];
                    let base = one(
                        rows,
                        "operation",
                        width,
                        trial,
                        "baseline",
                        Some(case),
                        Some("target"),
                    )?;
                    ratios.push(number(base, "cpu_us")? / number(row, "cpu_us")?);
                    let seed = one(
                        rows,
                        "operation",
                        width,
                        trial,
                        arm,
                        Some(case),
                        Some("seed"),
                    )?;
                    let need_seed = !matches!(arm, "raw" | "zstd");
                    let mut practical_cpu_us = number(row, "cpu_us")?
                        + if need_seed {
                            number(seed, "cpu_us")?
                        } else {
                            0.0
                        };
                    let mut practical_wall_us = number(row, "wall_us")?
                        + if need_seed {
                            number(seed, "wall_us")?
                        } else {
                            0.0
                        };
                    if arm == "prepared" {
                        let prep = one(rows, "preparation", width, trial, arm, None, None)?;
                        cold_cpu.push(number(prep, "cpu_us")? + number(row, "cpu_us")?);
                        cold_wall.push(number(prep, "wall_us")? + number(row, "wall_us")?);
                        practical_cpu_us += number(prep, "cpu_us")?;
                        practical_wall_us += number(prep, "wall_us")?;
                    }
                    practical_cpu.push(practical_cpu_us);
                    practical_wall.push(practical_wall_us);
                    practical_body.push(
                        number(row, "body_bytes")?
                            + if need_seed {
                                number(seed, "body_bytes")?
                            } else {
                                0.0
                            },
                    );
                }
                cell["paired_baseline_cpu_speedup"] = distribution(ratios.clone());
                cell["paired_baseline_cpu_speedup_geomean"] =
                    json!((ratios.iter().map(|v| v.ln()).sum::<f64>() / 10.0).exp());
                cell["practical_cold_cpu_us"] = distribution(practical_cpu);
                cell["practical_cold_wall_us"] = distribution(practical_wall);
                cell["practical_cold_body_bytes"] = distribution(practical_body);
                if arm == "prepared" {
                    cell["preparation_plus_target_cpu_us"] = distribution(cold_cpu);
                    cell["preparation_plus_target_wall_us"] = distribution(cold_wall);
                }
                cells.push(cell);
            }
        }
        let mut per_use_cpu = Vec::new();
        let mut per_use_wall = Vec::new();
        let mut prep_cpu = Vec::new();
        let mut prep_wall = Vec::new();
        let mut retention = Vec::new();
        for trial in 0..10 {
            let prep = one(rows, "preparation", width, trial, "prepared", None, None)?;
            let mut cpu = number(prep, "cpu_us")?;
            let mut wall = number(prep, "wall_us")?;
            prep_cpu.push(cpu);
            prep_wall.push(wall);
            retention.push(number(prep, "retained_bytes")?);
            for case in CASES {
                let row = one(
                    rows,
                    "operation",
                    width,
                    trial,
                    "prepared",
                    Some(case),
                    Some("target"),
                )?;
                cpu += number(row, "cpu_us")?;
                wall += number(row, "wall_us")?;
            }
            per_use_cpu.push(cpu / 3.0);
            per_use_wall.push(wall / 3.0);
        }
        amortized.push(json!({"width":width,"actual_uses":3,"preparation_cpu_us":distribution(prep_cpu),"preparation_wall_us":distribution(prep_wall),
            "retained_bytes":distribution(retention),"preparation_plus_three_targets_cpu_us_per_use":distribution(per_use_cpu),"preparation_plus_three_targets_wall_us_per_use":distribution(per_use_wall)}));
    }
    Ok(
        json!({"schema":"spiralcore.replay-cpu.summary.v1","verdict":"PAIRED_EXACTNESS_AND_BODY_COST_PASS","target_operations":360,"seed_operations":360,
        "workers":120,"discarded_observations":0,"replay":replay,"cells":cells,"preparation_and_actual_reuse":amortized}),
    )
}
fn command() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("worker") if args.len()==6 => worker(&args[2], args[3].parse()?, args[4].parse()?, Path::new(&args[5])),
        Some(mode @ ("run"|"profile")) if args.len()==6 => run(Path::new(&args[2]),Path::new(&args[3]),Path::new(&args[4]),Path::new(&args[5]),mode=="profile"),
        Some("summarize") if args.len()==4 => {
            let raw=fs::read(&args[2])?;
            let rows=std::str::from_utf8(&raw)?.lines().map(serde_json::from_str::<Value>).collect::<std::result::Result<Vec<_>,_>>()?;
            let mut result=summarize(&rows)?; result["raw_sha256"]=json!(sha(&raw));
            let mut out=OpenOptions::new().write(true).create_new(true).open(&args[3])?;
            write_row(&mut out,&result)
        },
        _ => Err("usage: replay_cpu worker ARM WIDTH TRIAL NEW_CACHE | run/profile BASELINE_EXE CANDIDATE_EXE NEW_JSONL NEW_CACHE".into()),
    }
}
fn main() {
    if let Err(e) = command() {
        let _ = emit(&json!({"event":"failure","error":e.to_string()}));
        eprintln!("{e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixtures_change_exactly_one_bit_or_replace_every_byte_deterministically() {
        let input = vec![0x55; 137];
        assert_eq!(target_for("generated", &input, 7).unwrap(), input);
        let edited = target_for("edited", &input, 7).unwrap();
        assert_eq!(edited.len(), input.len());
        assert_eq!(
            edited
                .iter()
                .zip(&input)
                .map(|(a, b)| (a ^ b).count_ones())
                .sum::<u32>(),
            1
        );
        let unrelated = target_for("unrelated", &input, 7).unwrap();
        assert_eq!(unrelated.len(), input.len());
        assert_ne!(unrelated, input);
        assert_eq!(unrelated, target_for("unrelated", &input, 7).unwrap());
        assert!(target_for("invalid", &input, 7).is_err());
    }
    #[test]
    fn registered_order_is_repeatable_complete_and_varies_across_trials() {
        let mut first = arm_order(48, 0);
        assert_eq!(first, arm_order(48, 0));
        assert_ne!(first, arm_order(48, 1));
        first.sort();
        assert_eq!(
            first,
            ["adaptive", "baseline", "packed", "prepared", "raw", "zstd"]
        );
    }
    #[test]
    fn summary_rejects_missing_trials_and_quantiles_retain_extremes() {
        assert!(summarize(&[]).is_err());
        assert_eq!(distribution(vec![10.0, 1.0, 4.0, 2.0])["median"], 3.0);
        assert_eq!(distribution(vec![10.0, 1.0, 4.0, 2.0])["p95"], 10.0);
    }
}
