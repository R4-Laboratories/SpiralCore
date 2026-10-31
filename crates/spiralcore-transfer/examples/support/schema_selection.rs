//! Equally configured whole-zstd control and setup-aware byte-first selection.
use super::schema_wan::{describe, observations, ratios, rows, METRICS_WAN};
use super::*;
use spiralcore_transfer::{
    schema_selection as selection, schema_zstd,
    wan::{HttpClient, WireMetrics},
    zstd_whole,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    process::Command,
};

const ARMS_SELECTION: [&str; 3] = ["schema", "whole", "selector"];
const SEQUENCE: [&str; 10] = [
    "normal",
    "normal",
    "normal",
    "normal",
    "variants",
    "variants",
    "drift",
    "drift",
    "unrelated",
    "unrelated",
];
const ARTIFACT: &[u8] = include_bytes!(
    "../../../../docs/network/evidence/2026-10-30-schema-reuse/artifacts-corrected/schema.artifact"
);

fn pins() -> Value {
    let mut p = sources();
    p.as_object_mut().unwrap().remove("hardware_sha256");
    for (name, bytes) in [
        (
            "selection_harness",
            include_bytes!("schema_selection.rs").as_slice(),
        ),
        ("wan_harness", include_bytes!("schema_wan.rs").as_slice()),
        (
            "schema_selection",
            include_bytes!("../../src/schema_selection.rs").as_slice(),
        ),
        (
            "whole_zstd",
            include_bytes!("../../src/zstd_whole.rs").as_slice(),
        ),
        (
            "schema_zstd",
            include_bytes!("../../src/schema_zstd.rs").as_slice(),
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
        json!({"sources":pins(),"executable_sha256":sha(&fs::read(std::env::current_exe()?)?),"platform":format!("{}-{}",std::env::consts::ARCH,std::env::consts::OS),"hardware_sha256":cfg!(feature="hardware-sha256")}),
    )
}
fn scope(run: &str, trial: usize, large: bool, arm: &str) -> Id {
    hash(
        b"spiralcore.schema-selection.scope.v1\0",
        &[
            run.as_bytes(),
            &(trial as u64).to_le_bytes(),
            &[u8::from(large)],
            arm.as_bytes(),
        ],
    )
}
fn order(trial: usize, large: bool) -> Result<[&'static str; 3]> {
    Ok(schema_wan::order(trial, large)?.map(|a| match a {
        "baseline" => "schema",
        "candidate" => "whole",
        _ => "selector",
    }))
}
fn plan(run: &str, shard: usize, source: &str) -> Result<Value> {
    if run.is_empty()
        || run.len() > 64
        || !run
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        || shard >= 3
        || source.len() != 40
        || !source
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err("invalid bounded registration".into());
    }
    let mut targets = Vec::new();
    let mut sessions = Vec::new();
    let mut scopes = Vec::new();
    let mut retained = 0u64;
    for trial in shard * 10..shard * 10 + 10 {
        for large in [false, true] {
            let mut distinct = BTreeSet::new();
            for (position, family) in SEQUENCE.iter().enumerate() {
                let t = fixture(family, large, trial * 10 + position)?;
                let id = hex(&object_id(&t.bytes));
                if !distinct.insert(id.clone()) {
                    return Err("duplicate object".into());
                }
                retained += 3 * t.bytes.len() as u64;
                targets.push(json!({"trial":trial,"large":large,"position":position,"family":family,"bytes":t.bytes.len(),"object":id,"sha256":sha(&t.bytes)}));
            }
            for arm in order(trial, large)? {
                let s = hex(&scope(run, trial, large, arm));
                scopes.push(s.clone());
                sessions.push(json!({"trial":trial,"large":large,"arm":arm,"scope":s}));
            }
        }
    }
    retained += 40 * ARTIFACT.len() as u64;
    let bound = retained + 640 * 4096 + (40 << 20) + (16 << 20);
    if bound > 768 << 20 {
        return Err("storage bound".into());
    }
    Ok(
        json!({"schema":"spiralcore.wan.plan.v4","experiment":"schema-selection.v1","run":run,"shard":shard,"source_commit":source,"sources":pins(),"cold_graph_warming":false,"codec_cache_enabled":true,
        "allowed_scopes":scopes,"sessions":sessions,"targets":targets,"target_operations":600,"mandatory_seeds":20,"maximum_seeds":40,"maximum_operations":640,"sender_context_setups":60,
        "artifact_bytes":ARTIFACT.len(),"artifact_sha256":sha(ARTIFACT),"artifact_object":hex(&object_id(ARTIFACT)),"retained_payload_bound":retained,"conservative_storage_bound":bound,"sequence":SEQUENCE}),
    )
}
fn read_plan(path: &Path, full: bool) -> Result<(Value, String)> {
    let raw = fs::read(path)?;
    let p: Value = serde_json::from_slice(&raw)?;
    if p["experiment"] != "schema-selection.v1"
        || p["sources"] != pins()
        || p["artifact_sha256"] != sha(ARTIFACT)
        || p["maximum_operations"] != 640
    {
        return Err("plan source/artifact mismatch".into());
    }
    if full
        && p != plan(
            p["run"].as_str().ok_or("run required")?,
            p["shard"].as_u64().ok_or("shard required")? as usize,
            p["source_commit"].as_str().ok_or("source required")?,
        )?
    {
        return Err("plan changed".into());
    }
    Ok((p, sha(&raw)))
}
fn target(p: &Value, trial: usize, large: bool, position: usize) -> Result<&Value> {
    let found = p["targets"]
        .as_array()
        .ok_or("targets required")?
        .iter()
        .filter(|r| r["trial"] == trial && r["large"] == large && r["position"] == position)
        .collect::<Vec<_>>();
    if found.len() != 1 {
        return Err("fixture count mismatch".into());
    }
    Ok(found[0])
}
fn check_bytes(m: &Value, bytes: &[u8]) -> Result<()> {
    if m["bytes"] != bytes.len()
        || m["sha256"] != sha(bytes)
        || m["object"] != hex(&object_id(bytes))
    {
        return Err("fixture bytes changed".into());
    }
    Ok(())
}

fn validate_choice(row: &Value, ready: bool) -> Result<()> {
    let candidates = row["candidates"].as_array().ok_or("three costs required")?;
    if candidates.len() != 3 {
        return Err("exactly three candidates required".into());
    }
    let modes = ["Raw", "ZstdWholeV1", "SchemaZstdV1"];
    let mut best = 0;
    let mut minimum = u64::MAX;
    for (index, mode) in modes.iter().enumerate() {
        let candidate = &candidates[index];
        let target = candidate["target_http_body_bytes"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("positive target cost required")?;
        let setup = if index == 2 && !ready {
            raw_body_cost(ARTIFACT.len() as u64)
        } else {
            0
        };
        let total = target.checked_add(setup).ok_or("candidate cost overflow")?;
        if candidate["mode"] != *mode
            || candidate["setup_http_body_bytes"] != setup
            || candidate["total_http_body_bytes"] != total
        {
            return Err("candidate mode/setup/total mismatch".into());
        }
        if total < minimum {
            minimum = total;
            best = index;
        }
    }
    if row["selected_mode"] != modes[best] || row["needs_schema_seed"] != (best == 2 && !ready) {
        return Err("nonminimum choice or wrong seed requirement".into());
    }
    Ok(())
}
fn raw_body_cost(len: u64) -> u64 {
    368 + len + 133 * len.div_ceil(spiralcore_transfer::MAX_CHUNK as u64)
}

pub(super) struct Measured {
    pub(super) result: spiralcore_transfer::Result<client::Stats>,
    sender_cpu: u128,
    wall: u128,
    exchange_cpu: u128,
    exchange_wall: u128,
    before: WireMetrics,
    after: WireMetrics,
}
pub(super) fn measure<F>(http: &mut HttpClient, call: F) -> Measured
where
    F: FnOnce(
        &mut dyn FnMut(&[Frame]) -> spiralcore_transfer::Result<Vec<Frame>>,
    ) -> spiralcore_transfer::Result<client::Stats>,
{
    let before = http.metrics();
    let mut exchange_cpu = 0;
    let mut exchange_wall = 0;
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let result = call(&mut |frames| {
        let w = Instant::now();
        let c = ProcessTime::now();
        let r = http.exchange(frames);
        exchange_cpu += c.elapsed().as_micros();
        exchange_wall += w.elapsed().as_micros();
        r
    });
    let sender_cpu = cpu.elapsed().as_micros();
    let wall = wall.elapsed().as_micros();
    let after = http.metrics();
    Measured {
        result,
        sender_cpu,
        wall,
        exchange_cpu,
        exchange_wall,
        before,
        after,
    }
}
impl Measured {
    pub(super) fn row(&self, phase: &str, position: Option<usize>, bytes: &[u8]) -> Value {
        self.row_with_metadata(
            phase,
            position,
            &json!({"bytes":bytes.len(),"object":hex(&object_id(bytes)),"sha256":sha(bytes)}),
        )
    }
    pub(super) fn row_with_metadata(
        &self,
        phase: &str,
        position: Option<usize>,
        expected: &Value,
    ) -> Value {
        let request = self.after.request_body_bytes - self.before.request_body_bytes;
        let response = self.after.response_body_bytes - self.before.response_body_bytes;
        let receiver = self.after.receiver_cpu_micros - self.before.receiver_cpu_micros;
        json!({"event":"operation","phase":phase,"position":position,"bytes":expected["bytes"],"object":expected["object"],"sha256":expected["sha256"],"sender_cpu_us":self.sender_cpu,"receiver_cpu_us":receiver,"cpu_us":self.sender_cpu+receiver as u128,"wall_us":self.wall,
            "exchange_cpu_us":self.exchange_cpu,"exchange_wall_us":self.exchange_wall,"receiver_wall_us":self.after.receiver_wall_micros-self.before.receiver_wall_micros,"request_body_bytes":request,"response_body_bytes":response,"body_bytes":request+response,"batches":self.after.http_requests-self.before.http_requests,
            "mode":self.result.as_ref().ok().map(|s|format!("{:?}",s.mode)),"error":self.result.as_ref().err().map(ToString::to_string),"verification":"commit-acknowledged-independent-reopen-pending"})
    }
}
enum Sender {
    Schema(schema_zstd::PreparedEncoder),
    Whole(zstd_whole::PreparedEncoder),
    Selector(selection::PreparedSelector),
}
impl Sender {
    fn new(arm: &str) -> Result<Self> {
        match arm {
            "schema" => Ok(Self::Schema(schema_zstd::PreparedEncoder::new(ARTIFACT)?)),
            "whole" => Ok(Self::Whole(zstd_whole::PreparedEncoder::new()?)),
            "selector" => Ok(Self::Selector(selection::PreparedSelector::new(ARTIFACT)?)),
            _ => Err("invalid arm".into()),
        }
    }
    fn retained(&self) -> (usize, u64) {
        match self {
            Self::Schema(p) => (p.retained_bytes(), p.retained_charge()),
            Self::Whole(p) => (p.retained_bytes(), p.retained_charge()),
            Self::Selector(p) => (p.retained_bytes(), p.retained_charge()),
        }
    }
}
fn transfer_id(scope: Id, phase: &str, position: Option<usize>) -> Id {
    hash(
        b"spiralcore.schema-selection.transfer.v1\0",
        &[
            &scope,
            phase.as_bytes(),
            &(position.map(|p| p as u64).unwrap_or(u64::MAX)).to_le_bytes(),
        ],
    )
}
fn receipt(r: &selection::Selection) -> Value {
    json!({"candidates":r.candidates.iter().map(|c|json!({"mode":format!("{:?}",c.mode),"target_http_body_bytes":c.target_http_body_bytes,"setup_http_body_bytes":c.setup_http_body_bytes,"total_http_body_bytes":c.total_http_body_bytes})).collect::<Vec<_>>(),"selected_mode":format!("{:?}",r.selected_mode),"needs_schema_seed":r.needs_schema_seed})
}
#[allow(clippy::too_many_arguments)]
fn session(
    p: &Value,
    digest: &str,
    trial: usize,
    large: bool,
    arm: &str,
    url: &str,
    token: &str,
    receiver: &str,
    out: &mut File,
) -> Result<()> {
    wan::parse_id(receiver)?;
    let scope = scope(p["run"].as_str().ok_or("run required")?, trial, large, arm);
    let scopehex = hex(&scope);
    if !p["sessions"]
        .as_array()
        .ok_or("sessions missing")?
        .iter()
        .any(|s| {
            s["trial"] == trial && s["large"] == large && s["arm"] == arm && s["scope"] == scopehex
        })
    {
        return Err("unregistered session".into());
    }
    let targets = SEQUENCE
        .iter()
        .enumerate()
        .map(|(position, family)| {
            let t = fixture(family, large, trial * 10 + position)?;
            check_bytes(target(p, trial, large, position)?, &t.bytes)?;
            Ok(t.bytes)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut http = HttpClient::new(url, token, scope)?;
    write_row(
        out,
        &json!({"event":"session_registration","identity":identity()?,"source_commit":p["source_commit"],"plan_sha256":digest,"trial":trial,"large":large,"arm":arm,"scope":scopehex,"receiver_executable_sha256":receiver,"url":url}),
    )?;
    let w = Instant::now();
    let c = ProcessTime::now();
    let sender = Sender::new(arm);
    let cpu = c.elapsed().as_micros();
    let wall = w.elapsed().as_micros();
    write_row(
        out,
        &json!({"event":"context_setup","cpu_us":cpu,"wall_us":wall,"error":sender.as_ref().err().map(ToString::to_string)}),
    )?;
    let mut sender = sender?;
    let mut ready = false;
    if arm == "schema" {
        let seed = measure(&mut http, |exchange| {
            client::transfer_batched(
                ARTIFACT,
                Mode::Raw,
                None,
                transfer_id(scope, "seed", None),
                false,
                exchange,
            )
        });
        write_row(out, &seed.row("seed", None, ARTIFACT))?;
        seed.result?;
        ready = true;
    }
    for (position, bytes) in targets.iter().enumerate() {
        let before = http.metrics();
        let outer_wall = Instant::now();
        let outer_cpu = ProcessTime::now();
        let mut prep_cpu = 0;
        let mut prep_wall = 0;
        let mut choice = None;
        let mut seeded = None;
        let was_ready = ready;
        let sent = match &mut sender {
            Sender::Schema(p) => measure(&mut http, |exchange| {
                client::transfer_prepared_schema_zstd_batched(
                    p,
                    bytes,
                    transfer_id(scope, "target", Some(position)),
                    false,
                    exchange,
                )
            }),
            Sender::Whole(p) => measure(&mut http, |exchange| {
                client::transfer_prepared_zstd_whole_batched(
                    p,
                    bytes,
                    transfer_id(scope, "target", Some(position)),
                    false,
                    exchange,
                )
            }),
            Sender::Selector(p) => {
                let w = Instant::now();
                let c = ProcessTime::now();
                let plan = p.prepare(bytes, ready, false);
                prep_cpu = c.elapsed().as_micros();
                prep_wall = w.elapsed().as_micros();
                let plan = match plan {
                    Ok(p) => p,
                    Err(e) => {
                        write_row(
                            out,
                            &json!({"event":"preparation_failure","position":position,"cpu_us":prep_cpu,"wall_us":prep_wall,"error":e.to_string()}),
                        )?;
                        return Err(e.into());
                    }
                };
                let selection = plan.selection().clone();
                if selection.needs_schema_seed {
                    let seed = measure(&mut http, |exchange| {
                        client::transfer_batched(
                            ARTIFACT,
                            Mode::Raw,
                            None,
                            transfer_id(scope, "seed", Some(position)),
                            false,
                            exchange,
                        )
                    });
                    if seed.result.is_err() {
                        write_row(
                            out,
                            &json!({"event":"selection_preparation","position":position,"cpu_us":prep_cpu,"wall_us":prep_wall,"schema_ready_before":was_ready,"receipt":receipt(&selection)}),
                        )?;
                        write_row(out, &seed.row("seed", Some(position), ARTIFACT))?;
                        return Err("selected seed failed; no retry".into());
                    }
                    seeded = Some(seed);
                    ready = true;
                }
                // Fixed-size receipt clone is retained; JSON/logging occurs only after ACK.
                choice = Some(selection);
                measure(&mut http, |exchange| {
                    client::transfer_schema_selection_batched(
                        plan,
                        transfer_id(scope, "target", Some(position)),
                        exchange,
                    )
                })
            }
        };
        let sender_cpu = outer_cpu.elapsed().as_micros();
        let wall = outer_wall.elapsed().as_micros();
        let after = http.metrics();
        if let Some(choice) = choice {
            let r = receipt(&choice);
            write_row(
                out,
                &json!({"event":"selection_preparation","position":position,"bytes":bytes.len(),"object":hex(&object_id(bytes)),"sha256":sha(bytes),"schema_ready_before":was_ready,"cpu_us":prep_cpu,"wall_us":prep_wall,"receipt":r}),
            )?;
        }
        if let Some(seed) = &seeded {
            write_row(out, &seed.row("seed", Some(position), ARTIFACT))?;
        }
        let target_row = sent.row("target", Some(position), bytes);
        write_row(out, &target_row)?;
        let (retained, charge) = sender.retained();
        let request = after.request_body_bytes - before.request_body_bytes;
        let response = after.response_body_bytes - before.response_body_bytes;
        let receiver_cpu = after.receiver_cpu_micros - before.receiver_cpu_micros;
        write_row(
            out,
            &json!({"event":"charged_target","position":position,"family":SEQUENCE[position],"bytes":bytes.len(),"object":hex(&object_id(bytes)),"sha256":sha(bytes),"mode":target_row["mode"],"raw_fallback":target_row["mode"]=="Raw", "seeded":seeded.is_some(),"sender_cpu_us":sender_cpu,"receiver_cpu_us":receiver_cpu,"cpu_us":sender_cpu+receiver_cpu as u128,"wall_us":wall,
            "phase_wall_sum_us":prep_wall+sent.wall+seeded.as_ref().map_or(0,|s|s.wall),"phase_sender_cpu_sum_us":prep_cpu+sent.sender_cpu+seeded.as_ref().map_or(0,|s|s.sender_cpu),
            "exchange_wall_us":sent.exchange_wall+seeded.as_ref().map_or(0,|s|s.exchange_wall),"exchange_cpu_us":sent.exchange_cpu+seeded.as_ref().map_or(0,|s|s.exchange_cpu),"request_body_bytes":request,"response_body_bytes":response,"body_bytes":request+response,"batches":after.http_requests-before.http_requests,
            "sender_retained_bytes":retained,"sender_retained_charge":charge,"error":target_row["error"]}),
        )?;
        sent.result?;
    }
    write_row(
        out,
        &json!({"event":"session_complete","targets":10,"artifact_seeded":ready,"status":"commit-acknowledged-reopen-pending"}),
    )
}
fn paired(plan_path: &Path, exe: &Path, url: &str, receiver: &str, out_path: &Path) -> Result<()> {
    let (p, digest) = read_plan(plan_path, true)?;
    let mut out = fresh(out_path)?;
    let result = (|| -> Result<()> {
        let output = Command::new(exe).arg("selection-identity").output()?;
        let id: Value = serde_json::from_slice(&output.stdout)?;
        if !output.status.success()
            || id["sources"] != p["sources"]
            || id["executable_sha256"] != sha(&fs::read(exe)?)
        {
            return Err("executable/source identity mismatch".into());
        }
        wan::parse_id(receiver)?;
        write_row(
            &mut out,
            &json!({"event":"registration","identity":id,"source_commit":p["source_commit"],"plan_sha256":digest,"run":p["run"],"shard":p["shard"],"receiver_executable_sha256":receiver,"url":url}),
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
            let result = Command::new(exe)
                .arg("selection-session")
                .arg(plan_path)
                .arg(s["trial"].as_u64().ok_or("trial missing")?.to_string())
                .arg(if s["large"] == true { "1" } else { "0" })
                .arg(s["arm"].as_str().ok_or("arm missing")?)
                .arg(url)
                .arg(receiver)
                .arg(&path)
                .output();
            let output = match result {
                Ok(o) => o,
                Err(e) => {
                    write_row(
                        &mut out,
                        &json!({"event":"spawn_failure","index":index,"session":s,"error":e.to_string()}),
                    )?;
                    return Err(e.into());
                }
            };
            let bytes = fs::read(&path).unwrap_or_default();
            let parsed = rows(&bytes);
            if !output.status.success() || parsed.is_err() {
                write_row(
                    &mut out,
                    &json!({"event":"session_failure","index":index,"session":s,"exit_code":output.status.code(),"stdout":String::from_utf8_lossy(&output.stdout),"stderr":String::from_utf8_lossy(&output.stderr),"raw":String::from_utf8_lossy(&bytes),"raw_sha256":sha(&bytes),"parse_error":parsed.err().map(|e|e.to_string())}),
                )?;
                return Err("session failed; no retry".into());
            }
            write_row(
                &mut out,
                &json!({"event":"session","index":index,"session":s,"rows":parsed?,"raw_sha256":sha(&bytes)}),
            )?;
            fs::remove_file(path)?;
        }
        fs::remove_dir(dir)?;
        write_row(
            &mut out,
            &json!({"event":"complete","targets":600,"sessions":60}),
        )
    })();
    if let Err(e) = result {
        write_row(&mut out, &json!({"event":"failure","error":e.to_string()}))?;
        return Err(e);
    }
    Ok(())
}
fn verify(plan_path: &Path, cache: &Path, out_path: &Path) -> Result<()> {
    let (p, digest) = read_plan(plan_path, true)?;
    let mut out = fresh(out_path)?;
    let result = (|| -> Result<()> {
        if !cache.is_dir() {
            return Err("stopped existing store required".into());
        }
        let log = fs::read(
            std::env::var("SPIRALCORE_SELECTION_OBSERVATIONS")
                .map_err(|_| "SPIRALCORE_SELECTION_OBSERVATIONS required")?,
        )?;
        let observed = observations(&p, &log)?;
        write_row(
            &mut out,
            &json!({"event":"verification_registration","plan_sha256":digest,"identity":identity()?,"source_commit":p["source_commit"],"observation_sha256":sha(&log),"observations":observed}),
        )?;
        let mut endpoint = Endpoint::open_with_policy(
            cache,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )?;
        let mut failed = false;
        let mut checked = 0;
        let mut present = 0;
        for s in p["sessions"].as_array().ok_or("sessions missing")? {
            let scope = wan::parse_id(s["scope"].as_str().ok_or("scope missing")?)?;
            let trial = s["trial"].as_u64().ok_or("trial missing")? as usize;
            let large = s["large"].as_bool().ok_or("size missing")?;
            for n in usize::from(s["arm"] == "whole")..11 {
                let position = if n == 0 { None } else { Some(n - 1) };
                let bytes = if let Some(pos) = position {
                    let t = fixture(SEQUENCE[pos], large, trial * 10 + pos)?;
                    check_bytes(target(&p, trial, large, pos)?, &t.bytes)?;
                    t.bytes
                } else {
                    ARTIFACT.to_vec()
                };
                let w = Instant::now();
                let c = ProcessTime::now();
                let actual = endpoint.read_object(&wan::context(scope), object_id(&bytes));
                let exact = actual.as_ref().is_ok_and(|v| v == &bytes);
                let absent = matches!(&actual, Err(spiralcore_transfer::Error::Unknown));
                let optional = position.is_none() && s["arm"] == "selector";
                let accepted = exact || (optional && absent);
                let cpu = c.elapsed().as_micros();
                let wall = w.elapsed().as_micros();
                failed |= !accepted;
                checked += 1;
                present += usize::from(exact);
                write_row(
                    &mut out,
                    &json!({"event":"receiver_verification","scope":hex(&scope),"position":position,"phase":if n==0{"artifact"}else{"target"},"object":hex(&object_id(&bytes)),"sha256":sha(&bytes),"bytes":bytes.len(),"exact":exact,"absent":absent,"optional":optional,"accepted":accepted,"actual_bytes":actual.as_ref().ok().map(Vec::len),"actual_sha256":actual.as_ref().ok().map(|v|sha(v)),"cpu_us":cpu,"wall_us":wall,"error":actual.err().map(|e|e.to_string())}),
                )?;
            }
        }
        write_row(
            &mut out,
            &json!({"event":"verification_complete","checks":checked,"exact_objects":present,"accepted":!failed}),
        )?;
        if failed || checked != 640 {
            return Err("independent verification failed".into());
        }
        Ok(())
    })();
    if let Err(e) = result {
        write_row(&mut out, &json!({"event":"failure","error":e.to_string()}))?;
        return Err(e);
    }
    Ok(())
}
fn accounting(row: &Value) -> Result<()> {
    if !row["error"].is_null()
        || numeric(row, "cpu_us")?
            != numeric(row, "sender_cpu_us")? + numeric(row, "receiver_cpu_us")?
        || numeric(row, "body_bytes")?
            != numeric(row, "request_body_bytes")? + numeric(row, "response_body_bytes")?
        || numeric(row, "exchange_wall_us")? > numeric(row, "wall_us")?
    {
        return Err("failed or inconsistent operation accounting".into());
    }
    Ok(())
}
fn candidate<'a>(receipt: &'a Value, mode: &str) -> Result<&'a Value> {
    let found = receipt["candidates"]
        .as_array()
        .ok_or("candidate costs required")?
        .iter()
        .filter(|c| c["mode"] == mode)
        .collect::<Vec<_>>();
    if found.len() != 1 {
        return Err("candidate missing/duplicated".into());
    }
    Ok(found[0])
}
fn find_verification<'a>(
    verified: &'a [Value],
    scope: &Value,
    position: Option<usize>,
) -> Result<&'a Value> {
    let found = verified
        .iter()
        .filter(|r| {
            r["event"] == "receiver_verification"
                && r["scope"] == *scope
                && r["position"] == json!(position)
        })
        .collect::<Vec<_>>();
    if found.len() != 1 {
        return Err("missing/duplicate readback".into());
    }
    Ok(found[0])
}
fn validate_receipts(
    p: &Value,
    digest: &str,
    raw: &[Value],
    verified: &[Value],
) -> Result<(Vec<Value>, Vec<Value>, Vec<Value>)> {
    if raw.len() != 62
        || raw[0]["event"] != "registration"
        || raw[0]["plan_sha256"] != digest
        || raw[0]["source_commit"] != p["source_commit"]
        || raw[0]["identity"]["sources"] != p["sources"]
        || raw[61]["event"] != "complete"
        || raw[61]["targets"] != 600
        || raw[61]["sessions"] != 60
        || verified.len() != 642
        || verified[0]["event"] != "verification_registration"
        || verified[0]["plan_sha256"] != digest
        || verified[0]["identity"]["sources"] != p["sources"]
        || verified[641]["event"] != "verification_complete"
        || verified[641]["checks"] != 640
        || verified[641]["accepted"] != true
    {
        return Err("incomplete or unbound shard".into());
    }
    let observed = verified[0]["observations"]
        .as_array()
        .ok_or("receiver observations missing")?;
    observations(
        p,
        observed
            .iter()
            .map(serde_json::to_string)
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join("\n")
            .as_bytes(),
    )?;
    let mut request_index = 0usize;
    let mut last_cache = json!({"hits":0,"misses":0,"preparations":0,"evictions":0});
    let mut all_targets = Vec::new();
    let mut sessions = Vec::new();
    let mut seeds = Vec::new();
    let mut exact_objects = 0;
    for (index, s) in p["sessions"]
        .as_array()
        .ok_or("sessions missing")?
        .iter()
        .enumerate()
    {
        let outer = &raw[index + 1];
        if outer["event"] != "session" || outer["index"] != index || outer["session"] != *s {
            return Err("session order changed".into());
        }
        let rows = outer["rows"].as_array().ok_or("session rows missing")?;
        let arm = s["arm"].as_str().ok_or("arm required")?;
        let trial = s["trial"].as_u64().ok_or("trial required")? as usize;
        let large = s["large"].as_bool().ok_or("size required")?;
        if rows.len() < 23
            || rows[0]["event"] != "session_registration"
            || rows[0]["identity"] != raw[0]["identity"]
            || rows[0]["plan_sha256"] != digest
            || rows[0]["scope"] != s["scope"]
            || rows[0]["arm"] != arm
            || rows[0]["trial"] != trial
            || rows[0]["large"] != large
            || rows[0]["receiver_executable_sha256"] != raw[0]["receiver_executable_sha256"]
            || rows[1]["event"] != "context_setup"
            || !rows[1]["error"].is_null()
            || rows.last().unwrap()["event"] != "session_complete"
            || rows.last().unwrap()["targets"] != 10
        {
            return Err("session identity/status mismatch".into());
        }
        let mut total = json!({"trial":trial,"large":large,"arm":arm,"seed_operations":0,"context_cpu_us":rows[1]["cpu_us"],"context_wall_us":rows[1]["wall_us"]});
        for key in METRICS_WAN {
            total[key] = json!(0.0);
        }
        total["cpu_us"] = rows[1]["cpu_us"].clone();
        total["sender_cpu_us"] = rows[1]["cpu_us"].clone();
        total["wall_us"] = rows[1]["wall_us"].clone();
        let mut position = 0;
        let mut ready = false;
        let mut prep: Option<&Value> = None;
        let mut seed: Option<&Value> = None;
        let mut sent: Option<&Value> = None;
        let mut cached_modes = BTreeSet::new();
        let mut seeded_count = 0;
        for row in &rows[2..rows.len() - 1] {
            match row["event"].as_str() {
                Some("selection_preparation") => {
                    if arm != "selector"
                        || prep.is_some()
                        || row["position"] != position
                        || row["schema_ready_before"] != ready
                    {
                        return Err("unexpected preparation".into());
                    }
                    let m = target(p, trial, large, position)?;
                    for key in ["bytes", "object", "sha256"] {
                        if row[key] != m[key] {
                            return Err("preparation target changed".into());
                        }
                    }
                    validate_choice(&row["receipt"], ready)?;
                    if candidate(&row["receipt"], "Raw")?["target_http_body_bytes"]
                        != raw_body_cost(m["bytes"].as_u64().ok_or("target length missing")?)
                    {
                        return Err("raw candidate cost does not bind original length".into());
                    }
                    prep = Some(row);
                }
                Some("operation") => {
                    accounting(row)?;
                    let is_seed = row["phase"] == "seed";
                    let expected = if is_seed {
                        json!({"bytes":169,"object":hex(&object_id(ARTIFACT)),"sha256":sha(ARTIFACT)})
                    } else {
                        target(p, trial, large, position)?.clone()
                    };
                    for key in ["bytes", "object", "sha256"] {
                        if row[key] != expected[key] {
                            return Err("operation identity differs".into());
                        }
                    }
                    if row["mode"] == "Raw"
                        && row["body_bytes"]
                            != raw_body_cost(
                                row["bytes"].as_u64().ok_or("operation length missing")?,
                            )
                    {
                        return Err("raw transport cost differs from exact length".into());
                    }
                    if is_seed {
                        if ready
                            || seeded_count != 0
                            || arm == "whole"
                            || row["mode"] != "Raw"
                            || row["body_bytes"] != 670
                            || if arm == "schema" {
                                position != 0 || !row["position"].is_null()
                            } else {
                                row["position"] != position
                                    || prep
                                        .is_none_or(|p| p["receipt"]["needs_schema_seed"] != true)
                            }
                        {
                            return Err("unexpected/mispriced seed".into());
                        }
                        seeded_count += 1;
                        ready = true;
                        exact_objects += 1;
                        let mut copy = row.clone();
                        copy["arm"] = json!(arm);
                        copy["trial"] = json!(trial);
                        copy["large"] = json!(large);
                        seeds.push(copy);
                        if arm == "schema" {
                            for key in METRICS_WAN {
                                total[key] = json!(numeric(&total, key)? + numeric(row, key)?);
                            }
                        } else {
                            seed = Some(row);
                        }
                    } else {
                        if row["phase"] != "target" || row["position"] != position || sent.is_some()
                        {
                            return Err("target order differs".into());
                        }
                        if arm == "selector" && prep.is_none() {
                            return Err("missing selection prepare".into());
                        }
                        sent = Some(row);
                        exact_objects += 1;
                    }
                    let end = request_index
                        .checked_add(row["batches"].as_u64().ok_or("batch count missing")? as usize)
                        .ok_or("request overflow")?;
                    let requests = observed
                        .get(request_index..end)
                        .ok_or("missing HTTP observations")?;
                    if requests.is_empty() || requests.iter().any(|o| o["scope"] != s["scope"]) {
                        return Err("HTTP scope/order mismatch".into());
                    }
                    let current = &requests.last().unwrap()["codec_cache"];
                    let mode = row["mode"].as_str().ok_or("mode missing")?;
                    let prepared = matches!(mode, "SchemaZstdV1" | "ZstdWholeV1");
                    let hit = prepared && cached_modes.contains(mode);
                    let miss = prepared && !hit;
                    for (key, n) in [
                        ("hits", u64::from(hit)),
                        ("misses", u64::from(miss)),
                        ("preparations", u64::from(miss)),
                    ] {
                        if numeric(current, key)? - numeric(&last_cache, key)? != n as f64 {
                            return Err("per-mode decoder reuse differs".into());
                        }
                    }
                    if prepared {
                        cached_modes.insert(mode.to_owned());
                    } else if mode != "Raw" {
                        return Err("unregistered mode".into());
                    }
                    last_cache = current.clone();
                    request_index = end;
                }
                Some("charged_target") => {
                    accounting(row)?;
                    let transfer = sent.take().ok_or("missing target send")?;
                    let m = target(p, trial, large, position)?;
                    for key in ["bytes", "object", "sha256", "family"] {
                        if row[key] != m[key] {
                            return Err("charged target changed".into());
                        }
                    }
                    if row["position"] != position
                        || row["mode"] != transfer["mode"]
                        || row["seeded"] != seed.is_some()
                    {
                        return Err("charged target mode/order differs".into());
                    }
                    for key in [
                        "body_bytes",
                        "request_body_bytes",
                        "response_body_bytes",
                        "batches",
                        "receiver_cpu_us",
                        "exchange_cpu_us",
                        "exchange_wall_us",
                    ] {
                        let n = numeric(transfer, key)?
                            + seed.map(|s| numeric(s, key)).transpose()?.unwrap_or(0.0);
                        if numeric(row, key)? != n {
                            return Err("seed/target charged twice or omitted".into());
                        }
                    }
                    let prepared_cpu = prep
                        .map(|p| numeric(p, "cpu_us"))
                        .transpose()?
                        .unwrap_or(0.0);
                    let prepared_wall = prep
                        .map(|p| numeric(p, "wall_us"))
                        .transpose()?
                        .unwrap_or(0.0);
                    if numeric(row, "phase_sender_cpu_sum_us")?
                        != prepared_cpu
                            + numeric(transfer, "sender_cpu_us")?
                            + seed
                                .map(|s| numeric(s, "sender_cpu_us"))
                                .transpose()?
                                .unwrap_or(0.0)
                        || numeric(row, "phase_wall_sum_us")?
                            != prepared_wall
                                + numeric(transfer, "wall_us")?
                                + seed
                                    .map(|s| numeric(s, "wall_us"))
                                    .transpose()?
                                    .unwrap_or(0.0)
                        || numeric(row, "phase_wall_sum_us")? > numeric(row, "wall_us")?
                    {
                        return Err("phase timing mismatch".into());
                    }
                    let mut copy = row.clone();
                    copy["target_phase_exchange_wall_us"] = transfer["exchange_wall_us"].clone();
                    copy["seed_phase_exchange_wall_us"] = json!(seed
                        .map(|s| numeric(s, "exchange_wall_us"))
                        .transpose()?
                        .unwrap_or(0.0));
                    if let Some(p) = prep.take() {
                        let r = &p["receipt"];
                        let chosen = candidate(
                            r,
                            r["selected_mode"].as_str().ok_or("selected mode missing")?,
                        )?;
                        if row["mode"] != r["selected_mode"]
                            || numeric(row, "body_bytes")?
                                != numeric(chosen, "total_http_body_bytes")?
                            || numeric(transfer, "body_bytes")?
                                != numeric(chosen, "target_http_body_bytes")?
                            || row["seeded"] != r["needs_schema_seed"]
                        {
                            return Err("selection receipt does not equal actual transport".into());
                        }
                        copy["selection"] = r.clone();
                        copy["preparation_cpu_us"] = p["cpu_us"].clone();
                        copy["preparation_wall_us"] = p["wall_us"].clone();
                    }
                    let v = find_verification(verified, &s["scope"], Some(position))?;
                    if v["exact"] != true
                        || v["accepted"] != true
                        || v["object"] != row["object"]
                        || v["sha256"] != row["sha256"]
                        || v["bytes"] != row["bytes"]
                        || v["actual_sha256"] != v["sha256"]
                        || v["actual_bytes"] != v["bytes"]
                    {
                        return Err("target reopen failed".into());
                    }
                    copy["arm"] = json!(arm);
                    copy["trial"] = json!(trial);
                    copy["large"] = json!(large);
                    for key in METRICS_WAN {
                        total[key] = json!(numeric(&total, key)? + numeric(row, key)?);
                    }
                    all_targets.push(copy);
                    position += 1;
                    seed = None;
                }
                _ => return Err("unexpected session event".into()),
            }
        }
        if position != 10
            || prep.is_some()
            || seed.is_some()
            || sent.is_some()
            || rows.last().unwrap()["artifact_seeded"] != ready
        {
            return Err("incomplete target sequence".into());
        }
        if arm != "whole" {
            let v = find_verification(verified, &s["scope"], None)?;
            if v["object"] != hex(&object_id(ARTIFACT))
                || v["sha256"] != sha(ARTIFACT)
                || v["bytes"] != 169
                || v["accepted"] != true
                || v["exact"] != ready
                || v["absent"] == ready
                || if ready {
                    v["actual_sha256"] != v["sha256"] || v["actual_bytes"] != 169
                } else {
                    arm != "selector" || v["optional"] != true
                }
            {
                return Err("artifact presence differs from paid seed".into());
            }
        }
        if arm == "schema" && !ready {
            return Err("mandatory seed absent".into());
        }
        total["seed_operations"] = json!(seeded_count);
        sessions.push(total);
    }
    if all_targets.len() != 600
        || request_index != observed.len()
        || verified[641]["exact_objects"] != exact_objects
    {
        return Err("shard totals differ".into());
    }
    Ok((all_targets, sessions, seeds))
}
fn summary(paths: &[String]) -> Result<Value> {
    let mut targets = Vec::new();
    let mut sessions = Vec::new();
    let mut seeds = Vec::new();
    let mut receipts = Vec::new();
    let mut registration = None;
    for shard in 0..3 {
        let (p, digest) = read_plan(Path::new(&paths[3 * shard]), true)?;
        if p["shard"] != shard {
            return Err("wrong shard".into());
        }
        let raw = fs::read(&paths[3 * shard + 1])?;
        let verified = fs::read(&paths[3 * shard + 2])?;
        let rr = rows(&raw)?;
        let vv = rows(&verified)?;
        let signature = json!({"source_commit":p["source_commit"],"run":p["run"],"identity":rr.first().ok_or("empty sender log")?["identity"],"receiver_executable_sha256":rr[0]["receiver_executable_sha256"]});
        if registration.as_ref().is_some_and(|r| r != &signature) {
            return Err("cross-shard identity differs".into());
        }
        registration = Some(signature);
        let (t, s, se) = validate_receipts(&p, &digest, &rr, &vv)?;
        let observed = vv[0]["observations"]
            .as_array()
            .ok_or("observations missing")?;
        receipts.push(json!({"shard":shard,"plan_sha256":digest,"raw_sha256":sha(&raw),"verification_sha256":sha(&verified),"observation_sha256":vv[0]["observation_sha256"],"actual_seeds":se.len(),"actual_target_transfers":t.len(),"exact_objects":vv[641]["exact_objects"],"final_cache":observed.last().map(|r|&r["codec_cache"]),"admitted_requests":observed.len(),"request_ordinal_gaps":observed.last().and_then(|r|r["request_ordinal"].as_u64()).unwrap_or(0).saturating_sub(observed.len() as u64)}));
        targets.extend(t);
        sessions.extend(s);
        seeds.extend(se);
    }
    let mut choices = BTreeMap::<String, usize>::new();
    let mut min_checks = 0;
    for large in [false, true] {
        for trial in 0..30 {
            for position in 0..10 {
                let get = |arm: &str| -> Result<&Value> {
                    let found = targets
                        .iter()
                        .filter(|r| {
                            r["large"] == large
                                && r["trial"] == trial
                                && r["position"] == position
                                && r["arm"] == arm
                        })
                        .collect::<Vec<_>>();
                    if found.len() != 1 {
                        return Err("missing paired target".into());
                    }
                    Ok(found[0])
                };
                let chosen = get("selector")?;
                let whole = get("whole")?;
                let schema = get("schema")?;
                for key in ["object", "sha256", "bytes", "family"] {
                    if chosen[key] != whole[key] || chosen[key] != schema[key] {
                        return Err("paired fixture changed".into());
                    }
                }
                let r = &chosen["selection"];
                let raw = numeric(candidate(r, "Raw")?, "target_http_body_bytes")?;
                if numeric(whole, "body_bytes")?
                    != raw.min(numeric(
                        candidate(r, "ZstdWholeV1")?,
                        "target_http_body_bytes",
                    )?)
                    || numeric(schema, "body_bytes")?
                        != raw.min(numeric(
                            candidate(r, "SchemaZstdV1")?,
                            "target_http_body_bytes",
                        )?)
                    || numeric(chosen, "body_bytes")? > numeric(whole, "body_bytes")?
                {
                    return Err("selected bytes exceed actual equally tuned control or candidate cost differs".into());
                }
                *choices
                    .entry(chosen["mode"].as_str().ok_or("mode missing")?.to_owned())
                    .or_default() += 1;
                min_checks += 1;
            }
        }
    }
    let mut cells = Vec::new();
    let mut paired = Vec::new();
    let mut session_cells = Vec::new();
    let mut actual = Vec::new();
    let mut paired_sessions = Vec::new();
    for large in [false, true] {
        for arm in ARMS_SELECTION {
            for family in [
                "normal",
                "variants",
                "drift",
                "unrelated",
                "normal_first",
                "normal_repeated",
            ] {
                let group = targets
                    .iter()
                    .filter(|r| {
                        r["large"] == large
                            && r["arm"] == arm
                            && match family {
                                "normal_first" => r["position"] == 0,
                                "normal_repeated" => r["family"] == "normal" && r["position"] != 0,
                                _ => r["family"] == family,
                            }
                    })
                    .collect::<Vec<_>>();
                let mut cell = describe(&group)?;
                cell["arm"] = json!(arm);
                cell["large"] = json!(large);
                cell["family"] = json!(family);
                for key in [
                    "target_phase_exchange_wall_us",
                    "seed_phase_exchange_wall_us",
                ] {
                    cell[key] = dist(
                        group
                            .iter()
                            .map(|r| numeric(r, key))
                            .collect::<Result<Vec<_>>>()?,
                    );
                }
                cell["modes"] =
                    json!(group
                        .iter()
                        .fold(BTreeMap::<String, usize>::new(), |mut m, r| {
                            *m.entry(r["mode"].as_str().unwrap_or("invalid").to_owned())
                                .or_default() += 1;
                            m
                        }));
                if arm == "selector" {
                    for key in ["preparation_cpu_us", "preparation_wall_us"] {
                        cell[key] = dist(
                            group
                                .iter()
                                .map(|r| numeric(r, key))
                                .collect::<Result<Vec<_>>>()?,
                        );
                    }
                }
                cells.push(cell);
            }
            let group = sessions
                .iter()
                .filter(|r| r["large"] == large && r["arm"] == arm)
                .collect::<Vec<_>>();
            let mut cell = describe(&group)?;
            cell["arm"] = json!(arm);
            cell["large"] = json!(large);
            for key in ["context_cpu_us", "context_wall_us"] {
                cell[key] = dist(
                    group
                        .iter()
                        .map(|r| numeric(r, key))
                        .collect::<Result<Vec<_>>>()?,
                );
            }
            session_cells.push(cell);
            let mut total = json!({"arm":arm,"large":large,"actual_targets":300,"sessions":30,"actual_seeds":seeds.iter().filter(|r|r["large"]==large&&r["arm"]==arm).count()});
            for key in METRICS_WAN {
                let n = group.iter().map(|r| numeric(r, key)).sum::<Result<f64>>()?;
                total[key] = json!(n);
                total[format!("{key}_per_target_with_actual_setup")] = json!(n / 300.0);
            }
            actual.push(total);
        }
        for family in FAMILIES {
            for (numerator, denominator) in [
                ("selector", "whole"),
                ("schema", "whole"),
                ("selector", "schema"),
            ] {
                let mut item = json!({"large":large,"family":family,"numerator":numerator,"denominator":denominator});
                for key in [
                    "sender_cpu_us",
                    "receiver_cpu_us",
                    "cpu_us",
                    "wall_us",
                    "body_bytes",
                ] {
                    let mut values = Vec::new();
                    for trial in 0..30 {
                        let sum = |arm: &str| -> Result<f64> {
                            targets
                                .iter()
                                .filter(|r| {
                                    r["trial"] == trial
                                        && r["large"] == large
                                        && r["family"] == family
                                        && r["arm"] == arm
                                })
                                .map(|r| numeric(r, key))
                                .sum()
                        };
                        values.push(sum(numerator)? / sum(denominator)?);
                    }
                    item[key] = ratios(values)?;
                }
                paired.push(item);
            }
        }
        for (numerator, denominator) in [
            ("selector", "whole"),
            ("schema", "whole"),
            ("selector", "schema"),
        ] {
            let mut item = json!({"large":large,"numerator":numerator,"denominator":denominator});
            for key in [
                "sender_cpu_us",
                "receiver_cpu_us",
                "cpu_us",
                "wall_us",
                "body_bytes",
            ] {
                let mut values = Vec::new();
                for trial in 0..30 {
                    let get = |arm: &str| -> Result<f64> {
                        let r = sessions
                            .iter()
                            .find(|r| r["trial"] == trial && r["large"] == large && r["arm"] == arm)
                            .ok_or("session total missing")?;
                        numeric(r, key)
                    };
                    let ratio = get(numerator)? / get(denominator)?;
                    if numerator == "selector"
                        && denominator == "whole"
                        && key == "body_bytes"
                        && ratio > 1.0
                    {
                        return Err("selector session costs more body bytes than whole".into());
                    }
                    values.push(ratio);
                }
                item[key] = ratios(values)?;
            }
            paired_sessions.push(item);
        }
    }
    Ok(
        json!({"schema":"spiralcore.schema-selection.summary.v1","verdict":"COMPLETE_EXACT_BYTE_MINIMUM_WAN_CAMPAIGN","registration":registration,"receipts":receipts,"target_operations":1800,"seed_operations":seeds.len(),"sender_context_setups":180,"sessions":180,"exact_reopened_objects":1800+seeds.len(),"independent_presence_checks":1920,"paired_minimum_checks":min_checks,"selected_modes":choices,"discarded":0,
        "metric_scope":"Primary wall is outer encode/selection start through durable ACK. Selector target rows include any lazy seed; forced-schema target rows exclude its separately reported upfront seed. Setup-inclusive session totals charge both symmetrically. CPU is outer sender plus disjoint receiver handlers. Phase sums are supplementary; target-phase exchange excludes optional seed but includes remote work/waiting, not pure wire time. Body bytes exclude transport headers/relay hops.","cells":cells,"paired_family_sums":paired,"setup_inclusive_session_cells":session_cells,"paired_setup_inclusive_sessions":paired_sessions,"actual_totals":actual,"session_totals":sessions,"seed_rows":seeds}),
    )
}
pub(super) fn command(args: &[String]) -> Result<()> {
    match args.get(1).map(String::as_str){
    Some("selection-identity") if args.len()==2=>{println!("{}",serde_json::to_string(&identity()?)?);Ok(())},
    Some("selection-plan") if args.len()==5=>{let source=std::env::var("SPIRALCORE_SELECTION_SOURCE").map_err(|_|"SPIRALCORE_SELECTION_SOURCE required")?;write_row(&mut fresh(Path::new(&args[4]))?,&plan(&args[2],args[3].parse()?,&source)?)},
    Some("selection-session") if args.len()==9=>{let(p,digest)=read_plan(Path::new(&args[2]),false)?;let large=match args[4].as_str(){"0"=>false,"1"=>true,_=>return Err("size must be0or1".into())};let token=std::env::var("SPIRALCORE_WAN_TOKEN").map_err(|_|"SPIRALCORE_WAN_TOKEN required")?;let mut out=fresh(Path::new(&args[8]))?;let result=session(&p,&digest,args[3].parse()?,large,&args[5],&args[6],&token,&args[7],&mut out);if let Err(e)=result{write_row(&mut out,&json!({"event":"failure","error":e.to_string()}))?;return Err(e);}Ok(())},
    Some("selection-paired") if args.len()==7=>paired(Path::new(&args[2]),Path::new(&args[3]),&args[4],&args[5],Path::new(&args[6])),
    Some("selection-verify") if args.len()==5=>verify(Path::new(&args[2]),Path::new(&args[3]),Path::new(&args[4])),
    Some("selection-summary") if args.len()==12=>{let result=summary(&args[2..11])?;write_row(&mut fresh(Path::new(&args[11]))?,&result)},
    _=>Err("usage: selection-plan RUN SHARD NEW_PLAN | selection-session PLAN TRIAL LARGE ARM URL RECEIVER_SHA NEW_JSONL | selection-paired PLAN EXE URL RECEIVER_SHA NEW_JSONL | selection-verify PLAN STOPPED_CACHE NEW_JSONL | selection-summary PLAN0 RAW0 VERIFY0 PLAN1 RAW1 VERIFY1 PLAN2 RAW2 VERIFY2 NEW_SUMMARY".into())
}
}

#[cfg(test)]
mod tests {
    use super::*;
    // Synthetic receipts qualify the analyzer only; never written as measurements.
    fn fake_operation(
        m: &Value,
        phase: &str,
        position: Option<usize>,
        mode: &str,
        body: u64,
    ) -> Value {
        json!({"event":"operation","phase":phase,"position":position,"bytes":m["bytes"],"object":m["object"],"sha256":m["sha256"],"mode":mode,"error":null,
            "sender_cpu_us":5,"receiver_cpu_us":4,"cpu_us":9,"wall_us":20,"exchange_cpu_us":2,"exchange_wall_us":12,"body_bytes":body,"request_body_bytes":body-100,"response_body_bytes":100,"batches":1})
    }
    fn fake_observation(
        observed: &mut Vec<Value>,
        counts: &mut (u64, u64),
        modes: &mut BTreeSet<String>,
        s: &Value,
        op: &Value,
    ) {
        let mode = op["mode"].as_str().unwrap();
        if matches!(mode, "ZstdWholeV1" | "SchemaZstdV1") {
            if modes.insert(mode.into()) {
                counts.1 += 1
            } else {
                counts.0 += 1
            }
        }
        observed.push(json!({"event":"wan_request","request_ordinal":observed.len()+1,"scope":s["scope"],"http_status":200,"codec_cache":{"hits":counts.0,"misses":counts.1,"preparations":counts.1,"evictions":counts.1.saturating_sub(8),"entries":counts.1.min(8),"resident_bytes":counts.1.min(8)*(5<<20)}}));
    }
    fn fake_receipts(p: &Value, digest: &str) -> (Vec<Value>, Vec<Value>) {
        let id = json!({"sources":p["sources"],"executable_sha256":"a".repeat(64),"platform":"unit-test","hardware_sha256":false});
        let receiver = "b".repeat(64);
        let mut raw = vec![
            json!({"event":"registration","identity":id,"source_commit":p["source_commit"],"plan_sha256":digest,"run":p["run"],"shard":p["shard"],"receiver_executable_sha256":receiver}),
        ];
        let mut verified = vec![
            json!({"event":"verification_registration","identity":id,"source_commit":p["source_commit"],"plan_sha256":digest,"observation_sha256":"c".repeat(64)}),
        ];
        let mut observed = Vec::new();
        let mut counts = (0, 0);
        let mut exact_objects = 0;
        let artifact =
            json!({"bytes":169,"object":hex(&object_id(ARTIFACT)),"sha256":sha(ARTIFACT)});
        for (index, s) in p["sessions"].as_array().unwrap().iter().enumerate() {
            let trial = s["trial"].as_u64().unwrap() as usize;
            let large = s["large"].as_bool().unwrap();
            let arm = s["arm"].as_str().unwrap();
            let mut ready = false;
            let mut modes = BTreeSet::new();
            let mut rr = vec![
                json!({"event":"session_registration","identity":id,"source_commit":p["source_commit"],"plan_sha256":digest,"scope":s["scope"],"arm":arm,"trial":trial,"large":large,"receiver_executable_sha256":receiver}),
                json!({"event":"context_setup","cpu_us":1,"wall_us":2,"error":null}),
            ];
            if arm == "schema" {
                let seed = fake_operation(&artifact, "seed", None, "Raw", 670);
                fake_observation(&mut observed, &mut counts, &mut modes, s, &seed);
                rr.push(seed);
                ready = true;
            }
            for position in 0..10 {
                let m = target(p, trial, large, position).unwrap();
                let raw_cost = raw_body_cost(m["bytes"].as_u64().unwrap());
                let whole_cost = if position == 9 {
                    raw_cost + 100
                } else if large {
                    raw_cost / 2
                } else {
                    raw_cost - 100
                };
                let schema_cost = if position == 9 {
                    raw_cost + 200
                } else if large {
                    raw_cost / 4
                } else {
                    raw_cost - 150
                };
                let r = costs(raw_cost, whole_cost, schema_cost, ready);
                let mut seed = None;
                let prepare = arm == "selector";
                if prepare {
                    rr.push(json!({"event":"selection_preparation","position":position,"bytes":m["bytes"],"object":m["object"],"sha256":m["sha256"],"schema_ready_before":ready,"cpu_us":3,"wall_us":4,"receipt":r}));
                    if r["needs_schema_seed"] == true {
                        let op = fake_operation(&artifact, "seed", Some(position), "Raw", 670);
                        fake_observation(&mut observed, &mut counts, &mut modes, s, &op);
                        rr.push(op.clone());
                        seed = Some(op);
                        ready = true;
                    }
                }
                let (mode, body) = match arm {
                    "schema" => {
                        if schema_cost < raw_cost {
                            ("SchemaZstdV1", schema_cost)
                        } else {
                            ("Raw", raw_cost)
                        }
                    }
                    "whole" => {
                        if whole_cost < raw_cost {
                            ("ZstdWholeV1", whole_cost)
                        } else {
                            ("Raw", raw_cost)
                        }
                    }
                    _ => {
                        let mode = r["selected_mode"].as_str().unwrap();
                        (
                            mode,
                            candidate(&r, mode).unwrap()["target_http_body_bytes"]
                                .as_u64()
                                .unwrap(),
                        )
                    }
                };
                let sent = fake_operation(m, "target", Some(position), mode, body);
                fake_observation(&mut observed, &mut counts, &mut modes, s, &sent);
                rr.push(sent.clone());
                let mut charged = sent.clone();
                charged["event"] = json!("charged_target");
                charged["family"] = m["family"].clone();
                charged["seeded"] = json!(seed.is_some());
                charged["raw_fallback"] = json!(mode == "Raw");
                for key in [
                    "body_bytes",
                    "request_body_bytes",
                    "response_body_bytes",
                    "batches",
                    "receiver_cpu_us",
                    "exchange_cpu_us",
                    "exchange_wall_us",
                ] {
                    charged[key] = json!(
                        numeric(&sent, key).unwrap()
                            + seed
                                .as_ref()
                                .map(|s| numeric(s, key).unwrap())
                                .unwrap_or(0.0)
                    );
                }
                let phase_cpu =
                    5 + if seed.is_some() { 5 } else { 0 } + if prepare { 3 } else { 0 };
                let phase_wall =
                    20 + if seed.is_some() { 20 } else { 0 } + if prepare { 4 } else { 0 };
                charged["phase_sender_cpu_sum_us"] = json!(phase_cpu);
                charged["phase_wall_sum_us"] = json!(phase_wall);
                charged["sender_cpu_us"] = json!(phase_cpu + 1);
                charged["wall_us"] = json!(phase_wall + 1);
                charged["cpu_us"] =
                    json!((phase_cpu + 1) as f64 + numeric(&charged, "receiver_cpu_us").unwrap());
                rr.push(charged);
                verified.push(json!({"event":"receiver_verification","scope":s["scope"],"position":position,"phase":"target","object":m["object"],"sha256":m["sha256"],"bytes":m["bytes"],"exact":true,"accepted":true,"absent":false,"actual_bytes":m["bytes"],"actual_sha256":m["sha256"],"error":null}));
                exact_objects += 1;
            }
            if arm != "whole" {
                verified.push(json!({"event":"receiver_verification","scope":s["scope"],"position":null,"phase":"artifact","object":artifact["object"],"sha256":artifact["sha256"],"bytes":169,"exact":ready,"accepted":true,"absent":!ready,"optional":arm=="selector","actual_bytes":if ready{Some(169)}else{None},"actual_sha256":if ready{Some(sha(ARTIFACT))}else{None},"error":if ready{None}else{Some(spiralcore_transfer::Error::Unknown.to_string())}}));
                exact_objects += usize::from(ready);
            }
            rr.push(json!({"event":"session_complete","targets":10,"artifact_seeded":ready}));
            raw.push(json!({"event":"session","index":index,"session":s,"rows":rr}));
        }
        raw.push(json!({"event":"complete","targets":600,"sessions":60}));
        verified[0]["observations"] = json!(observed);
        verified.push(json!({"event":"verification_complete","checks":640,"exact_objects":exact_objects,"accepted":true}));
        (raw, verified)
    }
    #[test]
    fn complete_dynamic_seed_summary_and_corruptions() {
        let directory =
            std::env::temp_dir().join(format!("selection-summary-{}", std::process::id()));
        assert!(!directory.exists());
        fs::create_dir(&directory).unwrap();
        let mut paths = Vec::new();
        for shard in 0..3 {
            let p = plan("synthetic-validator-test", shard, &"0".repeat(40)).unwrap();
            let path = directory.join(format!("plan{shard}.json"));
            write_row(&mut fresh(&path).unwrap(), &p).unwrap();
            let digest = sha(&fs::read(&path).unwrap());
            let (raw, verified) = fake_receipts(&p, &digest);
            let validation = validate_receipts(&p, &digest, &raw, &verified);
            assert!(validation.is_ok(), "{:?}", validation.err());
            let mut missing = verified.clone();
            missing.remove(1);
            assert!(validate_receipts(&p, &digest, &raw, &missing).is_err());
            let mut unseeded = verified.clone();
            let v = unseeded
                .iter_mut()
                .find(|v| v["optional"] == true && v["absent"] == true)
                .unwrap();
            v["exact"] = json!(true);
            v["absent"] = json!(false);
            assert!(validate_receipts(&p, &digest, &raw, &unseeded).is_err());
            let mut changed = raw.clone();
            let rr = changed
                .iter_mut()
                .find(|r| r["session"]["arm"] == "selector")
                .unwrap()["rows"]
                .as_array_mut()
                .unwrap();
            rr.iter_mut()
                .find(|r| r["event"] == "selection_preparation")
                .unwrap()["receipt"]["candidates"][2]["setup_http_body_bytes"] = json!(0);
            assert!(validate_receipts(&p, &digest, &changed, &verified).is_err());
            paths.push(path.to_str().unwrap().to_owned());
            for (kind, rows) in [("raw", raw), ("verify", verified)] {
                let path = directory.join(format!("{kind}{shard}.jsonl"));
                let mut out = fresh(&path).unwrap();
                for row in rows {
                    write_row(&mut out, &row).unwrap();
                }
                paths.push(path.to_str().unwrap().to_owned());
            }
        }
        let result = summary(&paths).unwrap();
        assert_eq!(result["target_operations"], 1800);
        assert_eq!(result["seed_operations"], 90);
        assert_eq!(result["exact_reopened_objects"], 1890);
        assert_eq!(result["paired_minimum_checks"], 600);
        fs::remove_dir_all(directory).unwrap();
    }
    fn costs(raw: u64, whole: u64, schema: u64, ready: bool) -> Value {
        let setup = if ready { 0 } else { 670 };
        let options = [
            ("Raw", raw, 0),
            ("ZstdWholeV1", whole, 0),
            ("SchemaZstdV1", schema, setup),
        ];
        let best = options
            .iter()
            .min_by_key(|(_, body, seed)| body + seed)
            .unwrap();
        json!({"candidates":options.iter().map(|(mode,body,seed)|json!({"mode":mode,"target_http_body_bytes":body,"setup_http_body_bytes":seed,"total_http_body_bytes":body+seed})).collect::<Vec<_>>(),"selected_mode":best.0,"needs_schema_seed":best.0=="SchemaZstdV1"&&!ready})
    }
    #[test]
    fn cold_threshold_ties_and_extra_candidates_are_rejected() {
        let cold = costs(1000, 900, 800, false);
        assert!(validate_choice(&cold, false).is_ok());
        assert_eq!(cold["selected_mode"], "ZstdWholeV1");
        let warm = costs(1000, 900, 800, true);
        assert!(validate_choice(&warm, true).is_ok());
        assert_eq!(warm["selected_mode"], "SchemaZstdV1");
        let tie = costs(1000, 1000, 1000, true);
        assert!(validate_choice(&tie, true).is_ok());
        assert_eq!(tie["selected_mode"], "Raw");
        let mut wrong = tie.clone();
        wrong["selected_mode"] = json!("SchemaZstdV1");
        assert!(validate_choice(&wrong, true).is_err());
        let mut duplicate = cold.clone();
        let first = duplicate["candidates"][0].clone();
        duplicate["candidates"].as_array_mut().unwrap().push(first);
        assert!(validate_choice(&duplicate, false).is_err());
    }
    #[test]
    fn real_persistent_small_sessions_skip_selector_seed_and_reopen_exactly() {
        use std::{
            net::TcpListener,
            sync::{Arc, Mutex},
            time::{Duration, SystemTime, UNIX_EPOCH},
        };
        let directory = std::env::temp_dir().join(format!(
            "selection-loopback-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let cache = directory.join("receiver");
        let p = plan("loopback-qualification", 2, &"0".repeat(40)).unwrap();
        let trial = 29;
        let scopes = ARMS_SELECTION.map(|arm| scope("loopback-qualification", trial, false, arm));
        let mut endpoint = Endpoint::open_with_policy(
            &cache,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        endpoint.set_codec_cache_enabled(true).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let token = "a".repeat(64);
        let server_token = token.clone();
        let observed = Arc::new(Mutex::new(Vec::new()));
        let capture = Arc::clone(&observed);
        let thread = std::thread::spawn(move || {
            wan::serve_observed(
                listener,
                endpoint,
                &server_token,
                &BTreeSet::from(scopes),
                Duration::from_secs(30),
                31,
                move |o| {
                    capture.lock().unwrap().push((o.scope, o.codec_cache));
                    Ok(())
                },
            )
        });
        for arm in ARMS_SELECTION {
            let path = directory.join(format!("{arm}.jsonl"));
            session(
                &p,
                "qualification",
                trial,
                false,
                arm,
                &format!("http://{address}/"),
                &token,
                &"b".repeat(64),
                &mut fresh(&path).unwrap(),
            )
            .unwrap();
            let rr = rows(&fs::read(path).unwrap()).unwrap();
            assert_eq!(
                rr.iter().filter(|r| r["event"] == "charged_target").count(),
                10
            );
            assert_eq!(
                rr.iter()
                    .filter(|r| r["event"] == "operation" && r["phase"] == "seed")
                    .count(),
                usize::from(arm == "schema")
            );
            if arm == "selector" {
                for prep in rr.iter().filter(|r| r["event"] == "selection_preparation") {
                    validate_choice(&prep["receipt"], false).unwrap();
                    assert_eq!(prep["receipt"]["needs_schema_seed"], false);
                }
            }
        }
        thread.join().unwrap().unwrap();
        let mut reopened = Endpoint::open_with_policy(
            &cache,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        for arm in ARMS_SELECTION {
            let scope = scope("loopback-qualification", trial, false, arm);
            for (position, family) in SEQUENCE.iter().enumerate() {
                let bytes = fixture(family, false, trial * 10 + position).unwrap().bytes;
                assert_eq!(
                    reopened
                        .read_object(&wan::context(scope), object_id(&bytes))
                        .unwrap(),
                    bytes
                );
            }
            let artifact = reopened.read_object(&wan::context(scope), object_id(ARTIFACT));
            if arm == "schema" {
                assert_eq!(artifact.unwrap(), ARTIFACT)
            } else {
                assert!(matches!(artifact, Err(spiralcore_transfer::Error::Unknown)));
            }
        }
        drop(reopened);
        assert_eq!(observed.lock().unwrap().len(), 31);
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn cold_seed_charge_cannot_be_omitted() {
        let receipt = json!({"candidates":[{"mode":"Raw","target_http_body_bytes":1000,"setup_http_body_bytes":0,"total_http_body_bytes":1000},
            {"mode":"ZstdWholeV1","target_http_body_bytes":900,"setup_http_body_bytes":0,"total_http_body_bytes":900},
            {"mode":"SchemaZstdV1","target_http_body_bytes":800,"setup_http_body_bytes":0,"total_http_body_bytes":800}],"selected_mode":"SchemaZstdV1","needs_schema_seed":true});
        assert!(validate_choice(&receipt, false).is_err());
    }
}
