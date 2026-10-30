//! Reproducible shared-runtime transfer measurements; see the registered contract.
use cpu_time::ProcessTime;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spiralcore_transfer::{
    client,
    endpoint::{ColdGraphPolicy, Endpoint},
    hash, hex, object_id,
    protocol::{Frame, Mode},
    shared_runtime::{self, OutputKind, Recipe, State},
    wan::{self, BenchAuthority, HttpClient},
    Id,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};

type BoxResult<T> = Result<T, Box<dyn std::error::Error>>;
const ARMS: [&str; 4] = ["raw", "zstd", "adaptive", "runtime"];
const CASES: [&str; 5] = ["generated", "edited", "append", "truncate", "unrelated"];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    source: String,
    run_id: String,
    widths: Vec<usize>,
    steps: Vec<u32>,
    cases: Vec<String>,
    trials: usize,
    transport: String,
}

impl Config {
    fn validate(&self) -> BoxResult<()> {
        fn unique<T: Ord>(v: &[T]) -> bool {
            v.iter().collect::<BTreeSet<_>>().len() == v.len()
        }
        if self.source.len() != 40
            || !self
                .source
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.run_id.is_empty()
            || self.run_id.len() > 64
            || !self
                .run_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !(1..=10).contains(&self.trials)
            || self.widths.is_empty()
            || self.widths.len() > 8
            || !unique(&self.widths)
            || self.widths.iter().any(|n| !(1..=4096).contains(n))
            || self.steps.is_empty()
            || self.steps.len() > 8
            || !unique(&self.steps)
            || self.steps.iter().any(|n| !(1..=4096).contains(n))
            || self.cases.is_empty()
            || !unique(&self.cases)
            || self.cases.iter().any(|s| !CASES.contains(&s.as_str()))
            || !["local-mechanism", "remote-http"].contains(&self.transport.as_str())
        {
            return Err("invalid bounded measurement configuration".into());
        }
        let scopes =
            self.widths.len() * self.steps.len() * self.cases.len() * self.trials * ARMS.len();
        if scopes > 4096 {
            return Err("too many scopes".into());
        }
        // Before generating anything, bound a remote shard's cumulative objects
        // conservatively. Local runs remove each owned arm's cache after verification.
        if self.transport == "remote-http"
            && (scopes * (shared_runtime::MAX_OUTPUT + shared_runtime::MAX_CHECKPOINT) > 768 << 20
                || scopes * 2 > 1800)
        {
            return Err(
                "remote shard exceeds conservative cache/state budget; split the plan".into(),
            );
        }
        Ok(())
    }
}

fn sha(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes).into())
}

fn source_files() -> Value {
    json!({
        "harness":sha(include_bytes!("shared_runtime.rs")),
        "runtime":sha(include_bytes!("../src/shared_runtime.rs")),
        "runtime_transfer":sha(include_bytes!("../src/shared_transfer.rs")),
        "client":sha(include_bytes!("../src/client.rs")),
        "selection":sha(include_bytes!("../src/selection.rs")),
        "endpoint":sha(include_bytes!("../src/endpoint.rs")),
        "protocol":sha(include_bytes!("../src/protocol.rs")),
        "wan":sha(include_bytes!("../src/wan.rs")),
        "lockfile":sha(include_bytes!("../Cargo.lock"))
    })
}

struct Fixture {
    checkpoint: Vec<u8>,
    prediction: Vec<u8>,
    target: Vec<u8>,
    recipe: Recipe,
}
impl Fixture {
    fn new(width: usize, steps: u32, case: &str, trial: usize) -> BoxResult<Self> {
        let checkpoint = State::seeded(width)?.encode()?;
        let recipe = Recipe {
            steps,
            payload: 1.0,
            output: OutputKind::Trace,
        };
        let prediction = shared_runtime::replay(&checkpoint, &recipe, shared_runtime::MAX_OUTPUT)?;
        let seed = hash(
            b"spiralcore.runtime.measurement.fixture.v1\0",
            &[
                &(width as u64).to_le_bytes(),
                &steps.to_le_bytes(),
                &(trial as u64).to_le_bytes(),
            ],
        );
        let target = target_for(case, &prediction, u64::from_le_bytes(seed[..8].try_into()?))?;
        if target.len() > client::MAX_BATCHED_OBJECT {
            return Err("actual target exceeds transfer limit".into());
        }
        Ok(Self {
            checkpoint,
            prediction,
            target,
            recipe,
        })
    }
    fn identity(&self, width: usize, steps: u32, case: &str, trial: usize) -> Value {
        json!({"width":width,"steps":steps,"case":case,"trial":trial,
            "checkpoint_bytes":self.checkpoint.len(),"checkpoint_sha256":sha(&self.checkpoint),
            "checkpoint_object":hex(&object_id(&self.checkpoint)),
            "prediction_bytes":self.prediction.len(),"prediction_sha256":sha(&self.prediction),
            "target_bytes":self.target.len(),"target_sha256":sha(&self.target),"target_object":hex(&object_id(&self.target)),
            "recipe_hex":hex_bytes(&self.recipe.encode().expect("constructed valid recipe"))})
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn scope_for(config: &Config, width: usize, steps: u32, case: &str, trial: usize, arm: &str) -> Id {
    hash(
        b"spiralcore.runtime.measurement.scope.v1\0",
        &[
            config.run_id.as_bytes(),
            config.source.as_bytes(),
            &(width as u64).to_le_bytes(),
            &steps.to_le_bytes(),
            case.as_bytes(),
            &(trial as u64).to_le_bytes(),
            arm.as_bytes(),
        ],
    )
}

fn all_scopes(config: &Config) -> Vec<String> {
    let mut scopes = Vec::new();
    for &width in &config.widths {
        for &steps in &config.steps {
            for case in &config.cases {
                for trial in 0..config.trials {
                    for arm in ARMS {
                        scopes.push(hex(&scope_for(config, width, steps, case, trial, arm)));
                    }
                }
            }
        }
    }
    scopes
}

fn make_plan(config: &Config) -> BoxResult<Value> {
    config.validate()?;
    let mut fixtures = Vec::new();
    for &width in &config.widths {
        for &steps in &config.steps {
            for case in &config.cases {
                for trial in 0..config.trials {
                    fixtures.push(
                        Fixture::new(width, steps, case, trial)?
                            .identity(width, steps, case, trial),
                    );
                }
            }
        }
    }
    let scopes = all_scopes(config);
    Ok(
        json!({"schema":"spiralcore.wan.plan.v4","experiment":"spiralcore.shared-runtime.v1",
        "cold_graph_warming":false,"config":config,"source_files":source_files(),
        "expected_operations":scopes.len()*2,"allowed_scopes":scopes,"fixtures":fixtures,
        "methods":ARMS,"seed_mode":"raw","warm_probe":false,
        "interpretation":"recipe traces are favorable generated workloads; all controls and losing candidate CPU are retained"}),
    )
}

enum Destination {
    Local(PathBuf),
    Remote { url: String, token: String },
}

#[derive(Default, Clone, Copy)]
struct Metrics {
    request: u64,
    response: u64,
    batches: u64,
    receiver_cpu_us: u128,
    receiver_wall_us: u128,
    exchange_cpu_us: u128,
    exchange_wall_us: u128,
}
struct Channel {
    local: Option<Endpoint<BenchAuthority>>,
    remote: Option<HttpClient>,
    scope: Id,
    metrics: Metrics,
}
impl Channel {
    fn open(destination: &Destination, scope: Id) -> BoxResult<Self> {
        let (local, remote) = match destination {
            Destination::Local(path) => (
                Some(Endpoint::open_with_policy(
                    path,
                    wan::limits(),
                    BenchAuthority,
                    ColdGraphPolicy::Disabled,
                )?),
                None,
            ),
            Destination::Remote { url, token } => (None, Some(HttpClient::new(url, token, scope)?)),
        };
        Ok(Self {
            local,
            remote,
            scope,
            metrics: Metrics::default(),
        })
    }
    fn exchange(&mut self, frames: &[Frame]) -> spiralcore_transfer::Result<Vec<Frame>> {
        let wall = Instant::now();
        let cpu = ProcessTime::now();
        let result = if let Some(client) = self.remote.as_mut() {
            let result = client.exchange(frames);
            let m = client.metrics();
            self.metrics.request = m.request_body_bytes;
            self.metrics.response = m.response_body_bytes;
            self.metrics.batches = m.http_requests;
            self.metrics.receiver_cpu_us = m.receiver_cpu_micros as u128;
            self.metrics.receiver_wall_us = m.receiver_wall_micros as u128;
            result
        } else {
            self.local_exchange(frames)
        };
        self.metrics.exchange_cpu_us += cpu.elapsed().as_micros();
        self.metrics.exchange_wall_us += wall.elapsed().as_micros();
        result
    }
    fn local_exchange(&mut self, frames: &[Frame]) -> spiralcore_transfer::Result<Vec<Frame>> {
        let request = wan::encode_batch(frames)?;
        self.metrics.request += request.len() as u64;
        self.metrics.batches += 1;
        let wall = Instant::now();
        let cpu = ProcessTime::now();
        let decoded = wan::decode_batch(&request)?;
        let endpoint = self
            .local
            .as_mut()
            .ok_or(spiralcore_transfer::Error::Storage)?;
        let mut replies = Vec::new();
        for frame in &decoded {
            let reply = endpoint
                .handle(&wan::context(self.scope), frame, 0)
                .unwrap_or_else(|error| frame.reply(127, (error as u16).to_le_bytes().to_vec()));
            let terminal = matches!(reply.kind, 103 | 127);
            replies.push(reply);
            if terminal {
                break;
            }
        }
        let response = wan::encode_batch(&replies)?;
        // Actual SCWREP01 adds two u64 metrics to the otherwise equivalent batch.
        self.metrics.response += response.len() as u64 + 16;
        self.metrics.receiver_cpu_us += cpu.elapsed().as_micros();
        self.metrics.receiver_wall_us += wall.elapsed().as_micros();
        wan::decode_batch(&response)
    }
}

fn operation(
    destination: &Destination,
    scope: Id,
    fixture: &Fixture,
    arm: &str,
    phase: &str,
) -> BoxResult<Value> {
    let bytes = if phase == "seed" {
        &fixture.checkpoint
    } else {
        &fixture.target
    };
    let id = hash(
        b"spiralcore.runtime.measurement.operation.v1\0",
        &[&scope, phase.as_bytes()],
    );
    let mut channel = Channel::open(destination, scope)?;
    let mut selection = None;
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let result = (|| -> spiralcore_transfer::Result<client::Stats> {
        let exchange = |frames: &[Frame]| channel.exchange(frames);
        if phase == "seed" {
            return client::transfer_batched(bytes, Mode::Raw, None, id, false, exchange);
        }
        match arm {
            "raw" => client::transfer_batched(bytes, Mode::Raw, None, id, false, exchange),
            "zstd" => client::transfer_batched(bytes, Mode::Zstd, None, id, false, exchange),
            "adaptive" => {
                let result = client::transfer_adaptive_batched(
                    bytes,
                    Some(&fixture.checkpoint),
                    id,
                    false,
                    exchange,
                )?;
                selection = Some(result.selection);
                Ok(result.transfer)
            }
            "runtime" => {
                let result = client::transfer_runtime_batched(
                    &fixture.checkpoint,
                    bytes,
                    &fixture.recipe,
                    id,
                    false,
                    exchange,
                )?;
                selection = Some(result.selection);
                Ok(result.transfer)
            }
            _ => Err(spiralcore_transfer::Error::Malformed),
        }
    })();
    let operation_cpu_us = cpu.elapsed().as_micros();
    let operation_wall_us = wall.elapsed().as_micros();
    let m = channel.metrics;
    drop(channel);
    let mut error = result.as_ref().err().map(ToString::to_string);
    let mut verification = "not-committed";
    let check_wall = Instant::now();
    let check_cpu = ProcessTime::now();
    if let Ok(stats) = &result {
        if stats.object != object_id(bytes) {
            error = Some("acknowledged identity differs from target".into());
        }
        match destination {
            Destination::Local(path) => {
                let check = (|| -> BoxResult<()> {
                    let mut endpoint = Endpoint::open_with_policy(
                        path,
                        wan::limits(),
                        BenchAuthority,
                        ColdGraphPolicy::Disabled,
                    )?;
                    if endpoint.read_object(&wan::context(scope), stats.object)? != *bytes {
                        return Err("exact bytes differ after reopen".into());
                    }
                    Ok(())
                })();
                match check {
                    Ok(()) => verification = "exact-after-reopen",
                    Err(e) => error = Some(e.to_string()),
                }
            }
            Destination::Remote { .. } => verification = "commit-ack-only-pending-receiver-reopen",
        }
    }
    let verification_cpu_us = check_cpu.elapsed().as_micros();
    let verification_wall_us = check_wall.elapsed().as_micros();
    let body = m.request + m.response;
    if selection
        .as_ref()
        .is_some_and(|s| s.expected_http_body_bytes != body)
    {
        error = Some("selected projected cost differs from actual successful body bytes".into());
    }
    let selected = selection.map(|s| json!({"selected_mode":s.selected as u8,
        "expected_http_body_bytes":s.expected_http_body_bytes,"exact_repeat_probe":s.exact_repeat_probe,
        "candidates":s.candidates.iter().map(|c|json!({"mode":c.mode as u8,"http_body_bytes":c.http_body_bytes})).collect::<Vec<_>>() }));
    let stats = result.as_ref().ok();
    Ok(
        json!({"event":"operation","phase":phase,"arm":arm,"scope":hex(&scope),
        "status":if error.is_none(){"ok"}else{"error"},"error":error,"verification":verification,
        "object":hex(&object_id(bytes)),"object_bytes":bytes.len(),"target_sha256":sha(bytes),
        "actual_mode":stats.map(|s|s.mode as u8),"protocol_request_bytes":stats.map(|s|s.request_bytes),
        "protocol_response_bytes":stats.map(|s|s.response_bytes),"body_bytes":body,
        "request_body_bytes":m.request,"response_body_bytes":m.response,"batches":m.batches,
        "operation_cpu_us":operation_cpu_us,"operation_wall_us":operation_wall_us,
        "receiver_cpu_us":m.receiver_cpu_us,"receiver_wall_us":m.receiver_wall_us,
        "exchange_cpu_us":m.exchange_cpu_us,"exchange_wall_us":m.exchange_wall_us,
        "verification_cpu_us":verification_cpu_us,"verification_wall_us":verification_wall_us,
        "cpu_scope":if matches!(destination,Destination::Local(_)){"single-process-combined-sender-receiver"}else{"sender-process"},
        "body_scope":if matches!(destination,Destination::Local(_)){"serialized-http-body-equivalent"}else{"actual-http-bodies-excluding-headers-tls-network"},
        "selection":selected}),
    )
}

fn new_output(path: &Path) -> BoxResult<File> {
    Ok(OpenOptions::new().write(true).create_new(true).open(path)?)
}
fn write_row(file: &mut File, row: &Value) -> BoxResult<()> {
    serde_json::to_writer(&mut *file, row)?;
    file.write_all(b"\n")?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}
fn validate_fixture(
    expected: &Value,
    fixture: &Fixture,
    width: usize,
    steps: u32,
    case: &str,
    trial: usize,
) -> BoxResult<()> {
    if *expected != fixture.identity(width, steps, case, trial) {
        return Err("fixture bytes differ from registered plan".into());
    }
    Ok(())
}
fn arm_order(trial: usize, case_index: usize) -> [&'static str; 4] {
    std::array::from_fn(|i| ARMS[(i + trial + case_index) % ARMS.len()])
}
fn file_sha(path: &Path) -> BoxResult<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex(&hasher.finalize().into()))
}
fn load_plan(path: &Path) -> BoxResult<(Value, Config)> {
    let plan: Value = serde_json::from_slice(&fs::read(path)?)?;
    let config: Config = serde_json::from_value(plan["config"].clone())?;
    config.validate()?;
    let scopes = all_scopes(&config);
    if plan["schema"] != "spiralcore.wan.plan.v4"
        || plan["experiment"] != "spiralcore.shared-runtime.v1"
        || plan["cold_graph_warming"] != false
        || plan["source_files"] != source_files()
        || plan["allowed_scopes"] != json!(scopes)
        || plan["expected_operations"] != json!(scopes.len() * 2)
        || plan["methods"] != json!(ARMS)
        || plan["seed_mode"] != "raw"
        || plan["warm_probe"] != false
        || plan["fixtures"]
            .as_array()
            .is_none_or(|v| v.len() != scopes.len() / ARMS.len())
    {
        return Err("plan schema/source/scope/operation binding mismatch".into());
    }
    Ok((plan, config))
}

fn planned_fixture<'a>(
    plan: &'a Value,
    width: usize,
    steps: u32,
    case: &str,
    trial: usize,
) -> BoxResult<&'a Value> {
    let matches = plan["fixtures"]
        .as_array()
        .ok_or("missing fixtures")?
        .iter()
        .filter(|f| {
            f["width"] == width && f["steps"] == steps && f["case"] == case && f["trial"] == trial
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err("missing or duplicate planned fixture".into());
    }
    Ok(matches[0])
}

fn run_campaign(
    plan: &Value,
    config: &Config,
    destination: &Destination,
    out: &mut File,
) -> BoxResult<()> {
    if let Destination::Local(root) = destination {
        fs::create_dir(root)?;
    }
    let mut count = 0;
    for &width in &config.widths {
        for &steps in &config.steps {
            for (case_index, case) in config.cases.iter().enumerate() {
                for trial in 0..config.trials {
                    let pair = format!("w{width}-s{steps}-{case}-t{trial}");
                    write_row(out, &json!({"event":"generation_started","pair":pair}))?;
                    let generation_wall = Instant::now();
                    let generation_cpu = ProcessTime::now();
                    let fixture = Fixture::new(width, steps, case, trial)?;
                    let generation_cpu_us = generation_cpu.elapsed().as_micros();
                    let generation_wall_us = generation_wall.elapsed().as_micros();
                    validate_fixture(
                        planned_fixture(plan, width, steps, case, trial)?,
                        &fixture,
                        width,
                        steps,
                        case,
                        trial,
                    )?;
                    write_row(
                        out,
                        &json!({"event":"generation","pair":pair,"fixture":fixture.identity(width,steps,case,trial),
                "generation_cpu_us":generation_cpu_us,"generation_wall_us":generation_wall_us,
                "scope":"seeded state creation + replay + target edit; hash metadata excluded; once per paired input"}),
                    )?;
                    for (order, arm) in arm_order(trial, case_index).into_iter().enumerate() {
                        let scope = scope_for(config, width, steps, case, trial, arm);
                        let local = match destination {
                            Destination::Local(root) => {
                                Some(Destination::Local(root.join(hex(&scope))))
                            }
                            _ => None,
                        };
                        let arm_destination = local.as_ref().unwrap_or(destination);
                        for phase in ["seed", "target"] {
                            write_row(
                                out,
                                &json!({"event":"operation_started","pair":pair,"arm":arm,"phase":phase,"scope":hex(&scope),"order":order}),
                            )?;
                            let mut row = match operation(
                                arm_destination,
                                scope,
                                &fixture,
                                arm,
                                phase,
                            ) {
                                Ok(row) => row,
                                Err(e) => {
                                    json!({"event":"operation","status":"error","pair":pair,"arm":arm,"phase":phase,
                            "scope":hex(&scope),"error":e.to_string(),"verification":"not-committed","body_bytes":null,
                            "metric_status":"channel preparation failed before transfer timers"})
                                }
                            };
                            row["pair"] = json!(pair);
                            row["order"] = json!(order);
                            row["width"] = json!(width);
                            row["steps"] = json!(steps);
                            row["case"] = json!(case);
                            row["trial"] = json!(trial);
                            write_row(out, &row)?;
                            count += 1;
                            if row["status"] != "ok" {
                                return Err("operation failed; retained rows and owned cache for inspection".into());
                            }
                        }
                        if let Destination::Local(path) = arm_destination {
                            match fs::remove_dir_all(path) {
                                Ok(()) => write_row(
                                    out,
                                    &json!({"event":"cleanup","scope":hex(&scope),"status":"owned-arm-cache-removed-after-exact-verification"}),
                                )?,
                                Err(e) => {
                                    write_row(
                                        out,
                                        &json!({"event":"cleanup","scope":hex(&scope),"status":"error","error":e.to_string()}),
                                    )?;
                                    return Err(e.into());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if plan["expected_operations"] != json!(count) {
        return Err("operation count mismatch".into());
    }
    write_row(
        out,
        &json!({"event":"complete","operations":count,"status":"ok",
        "remote_verification_pending":matches!(destination,Destination::Remote{..})}),
    )
}

fn verify_receiver(plan: &Value, config: &Config, cache: &Path, out: &mut File) -> BoxResult<()> {
    if config.transport != "remote-http" {
        return Err("receiver verification requires a remote plan".into());
    }
    if !cache.is_dir() {
        return Err("existing stopped receiver cache required".into());
    }
    let mut endpoint = Endpoint::open_with_policy(
        cache,
        wan::limits(),
        BenchAuthority,
        ColdGraphPolicy::Disabled,
    )?;
    let mut failed = false;
    let mut checked = 0;
    for &width in &config.widths {
        for &steps in &config.steps {
            for case in &config.cases {
                for trial in 0..config.trials {
                    let fixture = Fixture::new(width, steps, case, trial)?;
                    validate_fixture(
                        planned_fixture(plan, width, steps, case, trial)?,
                        &fixture,
                        width,
                        steps,
                        case,
                        trial,
                    )?;
                    for arm in ARMS {
                        let scope = scope_for(config, width, steps, case, trial, arm);
                        for (phase, bytes) in
                            [("seed", &fixture.checkpoint), ("target", &fixture.target)]
                        {
                            let read = endpoint.read_object(&wan::context(scope), object_id(bytes));
                            let exact = read.as_ref().is_ok_and(|v| v == bytes);
                            failed |= !exact;
                            checked += 1;
                            write_row(
                                out,
                                &json!({"event":"receiver_verification","scope":hex(&scope),"phase":phase,
                    "status":if exact{"ok"}else{"error"},"verification":"independent-exact-read-after-server-stop",
                    "expected_object":hex(&object_id(bytes)),"expected_sha256":sha(bytes),"expected_bytes":bytes.len(),
                    "actual_sha256":read.as_ref().ok().map(|v|sha(v)),"actual_bytes":read.as_ref().ok().map(Vec::len),
                    "error":read.err().map(|e|e.to_string())}),
                            )?;
                        }
                    }
                }
            }
        }
    }
    write_row(
        out,
        &json!({"event":"verification_complete","checked":checked,"status":if failed{"error"}else{"ok"}}),
    )?;
    if failed {
        return Err("one or more receiver objects failed exact reopen verification".into());
    }
    Ok(())
}

fn take(options: &mut BTreeMap<String, String>, key: &str) -> BoxResult<String> {
    options
        .remove(key)
        .ok_or_else(|| format!("missing {key}").into())
}
fn no_options(options: &BTreeMap<String, String>) -> BoxResult<()> {
    if !options.is_empty() {
        return Err("unknown option".into());
    }
    Ok(())
}
fn command() -> BoxResult<()> {
    let mut args = std::env::args().skip(1);
    let command = args
        .next()
        .ok_or("command required: plan, local, remote, verify")?;
    let mut options = BTreeMap::new();
    while let Some(key) = args.next() {
        let value = args.next().ok_or("missing option value")?;
        if options.insert(key, value).is_some() {
            return Err("duplicate option".into());
        }
    }
    let output_path = PathBuf::from(take(&mut options, "--out")?);
    if command == "plan" {
        let config = Config {
            source: take(&mut options, "--source")?,
            run_id: take(&mut options, "--run-id")?,
            widths: take(&mut options, "--widths")?
                .split(',')
                .map(str::parse)
                .collect::<Result<_, _>>()?,
            steps: take(&mut options, "--steps")?
                .split(',')
                .map(str::parse)
                .collect::<Result<_, _>>()?,
            cases: take(&mut options, "--cases")?
                .split(',')
                .map(str::to_string)
                .collect(),
            trials: take(&mut options, "--trials")?.parse()?,
            transport: take(&mut options, "--transport")?,
        };
        no_options(&options)?;
        config.validate()?;
        let mut out = new_output(&output_path)?;
        let result = make_plan(&config);
        match result {
            Ok(plan) => write_row(&mut out, &plan),
            Err(e) => {
                write_row(
                    &mut out,
                    &json!({"event":"plan_failure","error":e.to_string(),"config":config}),
                )?;
                Err(e)
            }
        }
    } else {
        let plan_path = PathBuf::from(take(&mut options, "--plan")?);
        let (plan, config) = load_plan(&plan_path)?;
        let mut receiver_exe = None;
        let destination = match command.as_str() {
            "local" | "verify" => Destination::Local(PathBuf::from(take(&mut options, "--cache")?)),
            "remote" => {
                let url = take(&mut options, "--url")?;
                let pin = take(&mut options, "--receiver-exe-sha256")?;
                wan::parse_id(&pin)?;
                receiver_exe = Some(pin);
                let token = std::env::var("SPIRALCORE_WAN_TOKEN")
                    .map_err(|_| "SPIRALCORE_WAN_TOKEN required")?;
                // Validate origin and credential without exposing either credential bytes or headers.
                HttpClient::new(&url, &token, [0; 32])?;
                Destination::Remote { url, token }
            }
            _ => return Err("command required: plan, local, remote, verify".into()),
        };
        no_options(&options)?;
        if command == "local" && config.transport != "local-mechanism"
            || command == "remote" && config.transport != "remote-http"
        {
            return Err("plan transport differs from execution command".into());
        }
        let mut out = new_output(&output_path)?;
        write_row(
            &mut out,
            &json!({"event":"run","experiment":"spiralcore.shared-runtime.v1","command":command,
            "plan_sha256":file_sha(&plan_path)?,"source":config.source,"source_files":source_files(),
            "executable_sha256":file_sha(&std::env::current_exe()?)?,"receiver_executable_sha256_operator_supplied":receiver_exe,
            "config":config,"architecture":std::env::consts::ARCH,"os":std::env::consts::OS,
            "timing":"input-ready through durable commit acknowledgement, including all codec candidates; generation, endpoint open, verification, JSONL writes and cleanup separate",
            "cpu":"local operation already includes receiver; remote operation is sender only; receiver metric covers adapter decode/handler/encode, not complete remote host or relay CPU",
            "exchange":"exchange time is a nested subset including serialization and request waiting; it is not pure byte-on-line transmission time"}),
        )?;
        let result = if command == "verify" {
            let Destination::Local(cache) = destination else {
                unreachable!()
            };
            verify_receiver(&plan, &config, &cache, &mut out)
        } else {
            run_campaign(&plan, &config, &destination, &mut out)
        };
        if let Err(e) = &result {
            write_row(
                &mut out,
                &json!({"event":"failure","status":"error","error":e.to_string()}),
            )?;
        }
        result
    }
}

fn main() {
    if let Err(error) = command() {
        eprintln!("SHARED_RUNTIME_MEASUREMENT_ERROR {error}");
        std::process::exit(2);
    }
}

fn target_for(case: &str, prediction: &[u8], seed: u64) -> Result<Vec<u8>, &'static str> {
    if prediction.len() < 18 {
        return Err("prediction too short");
    }
    let mut target = prediction.to_vec();
    let mut random = if seed == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        seed
    };
    let mut next = || {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        random as u8
    };
    match case {
        "generated" => (),
        "edited" => target[prediction.len() / 2] ^= 1,
        "append" => target.extend((0..32).map(|_| next())),
        "truncate" => target.truncate(prediction.len() - 17),
        "unrelated" => target.iter_mut().for_each(|byte| *byte = next()),
        _ => return Err("unknown fixture case"),
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_http_ack_is_separate_from_stopped_receiver_exact_verification() {
        let root =
            std::env::temp_dir().join(format!("shared-runtime-http-test-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let cache = root.join("cache");
        let config = Config {
            source: "1".repeat(40),
            run_id: "http-qualification".into(),
            widths: vec![48],
            steps: vec![27],
            cases: vec!["edited".into()],
            trials: 1,
            transport: "remote-http".into(),
        };
        let plan = make_plan(&config).unwrap();
        let fixture = Fixture::new(48, 27, "edited", 0).unwrap();
        let scopes = all_scopes(&config)
            .iter()
            .map(|s| wan::parse_id(s).unwrap())
            .collect();
        let endpoint = Endpoint::open_with_policy(
            &cache,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        // Public, synthetic test credential; never used by a deployed server.
        const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let server = std::thread::spawn(move || {
            wan::serve(
                listener,
                endpoint,
                TOKEN,
                &scopes,
                std::time::Duration::from_secs(15),
                8,
            )
            .unwrap()
        });
        let destination = Destination::Remote {
            url,
            token: TOKEN.into(),
        };
        for arm in ARMS {
            let scope = scope_for(&config, 48, 27, "edited", 0, arm);
            for phase in ["seed", "target"] {
                let row = operation(&destination, scope, &fixture, arm, phase).unwrap();
                assert_eq!(row["status"], "ok", "{row}");
                assert_eq!(
                    row["verification"],
                    "commit-ack-only-pending-receiver-reopen"
                );
                assert_eq!(
                    row["body_scope"],
                    "actual-http-bodies-excluding-headers-tls-network"
                );
                if phase == "target" && ["adaptive", "runtime"].contains(&arm) {
                    assert_eq!(
                        row["body_bytes"],
                        row["selection"]["expected_http_body_bytes"]
                    );
                }
            }
        }
        server.join().unwrap();
        let verified_path = root.join("verified.jsonl");
        let mut out = new_output(&verified_path).unwrap();
        verify_receiver(&plan, &config, &cache, &mut out).unwrap();
        drop(out);
        let rows: Vec<Value> = fs::read_to_string(&verified_path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(rows.len(), 9);
        assert_eq!(rows[8]["checked"], 8);
        assert!(rows.iter().all(|row| row["status"] == "ok"));
        let mut wrong_scope = config;
        wrong_scope.run_id = "different-run".into();
        let rejected_path = root.join("rejected.jsonl");
        let mut out = new_output(&rejected_path).unwrap();
        assert!(verify_receiver(&plan, &wrong_scope, &cache, &mut out).is_err());
        drop(out);
        assert_eq!(
            fs::read_to_string(&rejected_path).unwrap().lines().count(),
            9
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn output_creation_never_overwrites_and_fixture_binding_rejects_changes() {
        let path =
            std::env::temp_dir().join(format!("shared-runtime-output-test-{}", std::process::id()));
        let mut output = new_output(&path).unwrap();
        write_row(&mut output, &json!({"retained":true})).unwrap();
        drop(output);
        assert!(new_output(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"retained\":true}\n");
        fs::remove_file(path).unwrap();
        let fixture = Fixture::new(48, 3, "edited", 0).unwrap();
        let identity = fixture.identity(48, 3, "edited", 0);
        assert!(validate_fixture(&identity, &fixture, 48, 3, "edited", 0).is_ok());
        let mut wrong = identity;
        wrong["target_sha256"] = json!("00".repeat(32));
        assert!(validate_fixture(&wrong, &fixture, 48, 3, "edited", 0).is_err());
        assert_eq!(arm_order(0, 0), ["raw", "zstd", "adaptive", "runtime"]);
        for trial in 0..10 {
            assert_eq!(
                arm_order(trial, 0)
                    .into_iter()
                    .collect::<BTreeSet<_>>()
                    .len(),
                4
            );
        }
    }

    #[test]
    fn serialized_runtime_transfer_cost_and_exact_reopen_are_measured() {
        let root = std::env::temp_dir().join(format!(
            "shared-runtime-example-test-{}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let fixture = Fixture::new(48, 27, "edited", 0).unwrap();
        let destination = Destination::Local(root.join("cache"));
        let seed = operation(&destination, [81; 32], &fixture, "runtime", "seed").unwrap();
        assert_eq!(seed["status"], "ok");
        assert_eq!(seed["verification"], "exact-after-reopen");
        let target = operation(&destination, [81; 32], &fixture, "runtime", "target").unwrap();
        assert_eq!(target["status"], "ok", "{target}");
        assert_eq!(target["target_sha256"], sha(&fixture.target));
        assert_eq!(target["verification"], "exact-after-reopen");
        assert_eq!(
            target["body_bytes"],
            target["selection"]["expected_http_body_bytes"]
        );
        assert_eq!(
            target["body_bytes"].as_u64().unwrap(),
            target["request_body_bytes"].as_u64().unwrap()
                + target["response_body_bytes"].as_u64().unwrap()
        );
        let missing = operation(&destination, [82; 32], &fixture, "runtime", "target").unwrap();
        assert_eq!(missing["status"], "error");
        assert!(missing["request_body_bytes"].as_u64().unwrap() > 0);
        assert_eq!(missing["verification"], "not-committed");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fixture_edits_keep_every_unmodified_byte_and_bind_trial() {
        let prediction: Vec<u8> = (0..255).collect();
        let edited = target_for("edited", &prediction, 7).unwrap();
        assert_eq!(edited.len(), prediction.len());
        assert_eq!(
            edited
                .iter()
                .zip(&prediction)
                .filter(|(a, b)| a != b)
                .count(),
            1
        );
        let appended = target_for("append", &prediction, 7).unwrap();
        assert_eq!(&appended[..prediction.len()], prediction);
        assert_eq!(appended.len(), prediction.len() + 32);
        assert_eq!(
            target_for("truncate", &prediction, 7).unwrap(),
            prediction[..238]
        );
        assert_eq!(target_for("generated", &prediction, 7).unwrap(), prediction);
        let unrelated = target_for("unrelated", &prediction, 7).unwrap();
        assert_eq!(unrelated.len(), prediction.len());
        assert_eq!(unrelated, target_for("unrelated", &prediction, 7).unwrap());
        assert_ne!(unrelated, target_for("unrelated", &prediction, 8).unwrap());
        assert_ne!(
            target_for("unrelated", &prediction, 8).unwrap(),
            target_for("unrelated", &prediction, 9).unwrap()
        );
        assert!(target_for("unknown", &prediction, 7).is_err());
    }

    #[test]
    fn plan_binds_unique_scopes_and_actual_fixture_sizes() {
        let config = Config {
            source: "1".repeat(40),
            run_id: "qualification".into(),
            widths: vec![48],
            steps: vec![3],
            cases: vec!["generated".into()],
            trials: 2,
            transport: "local-mechanism".into(),
        };
        let plan = make_plan(&config).unwrap();
        let scopes = plan["allowed_scopes"].as_array().unwrap();
        assert_eq!(scopes.len(), 8);
        assert_eq!(
            scopes
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            8
        );
        assert_eq!(plan["expected_operations"], 16);
        assert_eq!(plan["fixtures"].as_array().unwrap().len(), 2);
        let fixture = Fixture::new(48, 3, "generated", 0).unwrap();
        assert_eq!(plan["fixtures"][0]["target_bytes"], fixture.target.len());
        assert_eq!(plan["fixtures"][0]["target_sha256"], sha(&fixture.target));
        let mut duplicate = config.clone();
        duplicate.widths.push(48);
        assert!(make_plan(&duplicate).is_err());
        let mut too_large = config;
        too_large.transport = "remote-http".into();
        too_large.widths = vec![4096];
        too_large.steps = vec![4096];
        too_large.cases = vec!["generated".into(), "unrelated".into()];
        too_large.trials = 10;
        assert!(make_plan(&too_large).is_err());
    }
}
