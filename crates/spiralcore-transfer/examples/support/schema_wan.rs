//! Registered, persistent schema transfers across the existing authenticated WAN adapter.
use super::*;
use spiralcore_transfer::{schema_zstd, wan::HttpClient};
use std::{collections::BTreeSet, process::Command};

const ARMS_WAN: [&str; 3] = ["baseline", "candidate", "zstd"];
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
const OLD_SCHEMA: &str = "6c228201197e1ce26a6ac8137da490e0040ed26ab06d4576b330e8b240a2577a";
const NEW_SCHEMA: &str = "8c81c80b94faf4579a9875a825e47575b61652b9e4c03130ab6bc783e2905109";
const ARTIFACT: &[u8] = include_bytes!(
    "../../../../docs/network/evidence/2026-10-30-schema-reuse/artifacts-corrected/schema.artifact"
);

pub(super) fn order(trial: usize, large: bool) -> Result<[&'static str; 3]> {
    if trial >= 30 {
        return Err("trial outside registered dose".into());
    }
    let mut permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let mut random = 0x2510_3013_abba_0303u64 ^ u64::from(large);
    for i in (1..6).rev() {
        permutations.swap(i, next(&mut random) as usize % (i + 1));
    }
    Ok(permutations[trial % 6].map(|i| ARMS_WAN[i]))
}
fn scope(run: &str, trial: usize, large: bool, arm: &str) -> Id {
    hash(
        b"spiralcore.schema-wan.scope.v1\0",
        &[
            run.as_bytes(),
            &(trial as u64).to_le_bytes(),
            &[u8::from(large)],
            arm.as_bytes(),
        ],
    )
}
fn pins() -> Value {
    let mut value = sources();
    value.as_object_mut().unwrap().remove("hardware_sha256");
    value["wan_harness"] = json!(sha(include_bytes!("schema_wan.rs")));
    value["schema_zstd"] = json!(sha(include_bytes!("../../src/schema_zstd.rs")));
    value["wan"] = json!(sha(include_bytes!("../../src/wan.rs")));
    value["wan_server"] = json!(sha(include_bytes!("../../src/bin/spiralcore-wan.rs")));
    value
}
fn identity() -> Result<Value> {
    Ok(
        json!({"sources":pins(),"executable_sha256":sha(&fs::read(std::env::current_exe()?)?),"platform":format!("{}-{}",std::env::consts::ARCH,std::env::consts::OS),"hardware_sha256":cfg!(feature="hardware-sha256")}),
    )
}
fn valid_commit(s: &str) -> bool {
    s.len() == 40
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn plan(run: &str, shard: usize, source: &str) -> Result<Value> {
    if run.is_empty()
        || run.len() > 64
        || !run
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        || shard >= 3
        || !valid_commit(source)
    {
        return Err("invalid bounded WAN registration".into());
    }
    let mut targets = Vec::new();
    let mut sessions = Vec::new();
    let mut allowed = Vec::new();
    let mut retained = 0u64;
    for trial in shard * 10..shard * 10 + 10 {
        for large in [false, true] {
            let mut distinct = BTreeSet::new();
            for (position, family) in SEQUENCE.iter().enumerate() {
                let target = fixture(family, large, trial * 10 + position)?;
                let id = hex(&object_id(&target.bytes));
                if !distinct.insert(id.clone()) {
                    return Err("duplicate session target".into());
                }
                retained = retained
                    .checked_add(3 * target.bytes.len() as u64)
                    .ok_or("storage overflow")?;
                targets.push(json!({"trial":trial,"large":large,"position":position,"family":family,"fixture_index":trial*10+position,"bytes":target.bytes.len(),"object":id,"sha256":sha(&target.bytes)}));
            }
            for arm in order(trial, large)? {
                let id = hex(&scope(run, trial, large, arm));
                allowed.push(id.clone());
                sessions.push(json!({"trial":trial,"large":large,"arm":arm,"scope":id}));
            }
        }
    }
    retained += 40 * ARTIFACT.len() as u64;
    // Exact payload plus conservative 4KiB/publication metadata, 8 codec charges,
    // and one active 16MiB object: all remain inside the unchanged 1GiB quota.
    let storage_bound = retained + 640 * 4096 + (40 << 20) + (16 << 20);
    if storage_bound > 768 << 20 || allowed.len() != 60 || targets.len() != 200 {
        return Err("shard storage/dose bound".into());
    }
    let mut source_pins = pins();
    source_pins["schema"] = json!(NEW_SCHEMA);
    Ok(
        json!({"schema":"spiralcore.wan.plan.v4","experiment":"schema-wan.v1","run":run,"shard":shard,"source_commit":source,
        "sources":source_pins,"baseline_schema_sha256":OLD_SCHEMA,"source_override":"baseline substitutes only src/schema.rs with the immutable SC012 oracle",
        "cold_graph_warming":false,"codec_cache_enabled":true,"observe_requests":true,"allowed_scopes":allowed,"sessions":sessions,"targets":targets,
        "target_operations":600,"seed_operations":40,"sender_preparations":40,"total_operations":640,"request_bound":4096,
        "retained_payload_bytes":retained,"conservative_storage_bound":storage_bound,"sequence":SEQUENCE,
        "artifact_bytes":ARTIFACT.len(),"artifact_sha256":sha(ARTIFACT),"artifact_object":hex(&object_id(ARTIFACT))}),
    )
}
fn read_plan(path: &Path, full: bool) -> Result<(Value, String)> {
    let bytes = fs::read(path)?;
    let p: Value = serde_json::from_slice(&bytes)?;
    let mut expected = pins();
    expected["schema"] = json!(NEW_SCHEMA);
    if p["schema"] != "spiralcore.wan.plan.v4"
        || p["experiment"] != "schema-wan.v1"
        || p["sources"] != expected
        || p["baseline_schema_sha256"] != OLD_SCHEMA
        || p["artifact_sha256"] != sha(ARTIFACT)
        || p["artifact_bytes"] != 169
        || p["total_operations"] != 640
    {
        return Err("plan source/artifact/dose mismatch".into());
    }
    if full
        && p != plan(
            p["run"].as_str().ok_or("run required")?,
            p["shard"].as_u64().ok_or("shard required")? as usize,
            p["source_commit"].as_str().ok_or("source required")?,
        )?
    {
        return Err("registered plan changed".into());
    }
    Ok((p, sha(&bytes)))
}
fn target_manifest(p: &Value, trial: usize, large: bool, position: usize) -> Result<&Value> {
    let matches = p["targets"]
        .as_array()
        .ok_or("targets required")?
        .iter()
        .filter(|r| r["trial"] == trial && r["large"] == large && r["position"] == position)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err("missing/duplicate fixture".into());
    }
    Ok(matches[0])
}
fn validate_target(
    p: &Value,
    trial: usize,
    large: bool,
    position: usize,
    bytes: &[u8],
) -> Result<()> {
    let m = target_manifest(p, trial, large, position)?;
    if m["bytes"] != bytes.len()
        || m["sha256"] != sha(bytes)
        || m["object"] != hex(&object_id(bytes))
        || m["family"] != SEQUENCE[position]
    {
        return Err("fixture identity mismatch".into());
    }
    Ok(())
}
fn operation(
    client: &mut HttpClient,
    prepared: &mut Option<schema_zstd::PreparedEncoder>,
    bytes: &[u8],
    scope: Id,
    position: Option<usize>,
) -> Value {
    let before = client.metrics();
    let seed = position.is_none();
    let transfer = hash(
        b"spiralcore.schema-wan.transfer.v1\0",
        &[
            &scope,
            &(position.map(|p| p as u64).unwrap_or(u64::MAX)).to_le_bytes(),
        ],
    );
    let mut exchange_cpu = 0u128;
    let mut exchange_wall = 0u128;
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let result = {
        let exchange = |frames: &[Frame]| {
            let w = Instant::now();
            let c = ProcessTime::now();
            let result = client.exchange(frames);
            exchange_cpu += c.elapsed().as_micros();
            exchange_wall += w.elapsed().as_micros();
            result
        };
        if seed {
            client::transfer_batched(bytes, Mode::Raw, None, transfer, false, exchange)
        } else if let Some(p) = prepared {
            client::transfer_prepared_schema_zstd_batched(p, bytes, transfer, false, exchange)
        } else {
            client::transfer_batched(bytes, Mode::Zstd, None, transfer, false, exchange)
        }
    };
    let sender_cpu = cpu.elapsed().as_micros();
    let wall_us = wall.elapsed().as_micros();
    let after = client.metrics();
    let request = after.request_body_bytes - before.request_body_bytes;
    let response = after.response_body_bytes - before.response_body_bytes;
    let receiver_cpu = after.receiver_cpu_micros - before.receiver_cpu_micros;
    json!({"event":"operation","phase":if seed{"seed"}else{"target"},"position":position,"bytes":bytes.len(),"object":hex(&object_id(bytes)),"sha256":sha(bytes),
        "sender_cpu_us":sender_cpu,"receiver_cpu_us":receiver_cpu,"cpu_us":sender_cpu+receiver_cpu as u128,"wall_us":wall_us,
        "exchange_cpu_us":exchange_cpu,"exchange_wall_us":exchange_wall,"receiver_wall_us":after.receiver_wall_micros-before.receiver_wall_micros,
        "request_body_bytes":request,"response_body_bytes":response,"body_bytes":request+response,"batches":after.http_requests-before.http_requests,
        "mode":result.as_ref().ok().map(|s|format!("{:?}",s.mode)),"raw_fallback":!seed&&prepared.is_some()&&result.as_ref().is_ok_and(|s|s.mode==Mode::Raw),
        "sender_retained_bytes":prepared.as_ref().map_or(0,|p|p.retained_bytes()),"sender_retained_charge":prepared.as_ref().map_or(0,|p|p.retained_charge()),
        "verification":"commit-acknowledged-independent-reopen-pending","error":result.err().map(|e|e.to_string())})
}
#[allow(clippy::too_many_arguments)]
fn session(
    p: &Value,
    plan_sha: &str,
    label: &str,
    trial: usize,
    large: bool,
    arm: &str,
    url: &str,
    receiver_sha: &str,
    out: &mut File,
) -> Result<()> {
    let name = match (label, arm) {
        ("baseline", "schema") => "baseline",
        ("candidate", "schema") => "candidate",
        ("candidate", "zstd") => "zstd",
        _ => return Err("invalid source/arm".into()),
    };
    let expected = if label == "baseline" {
        OLD_SCHEMA
    } else {
        NEW_SCHEMA
    };
    if pins()["schema"] != expected {
        return Err("wrong encoder executable".into());
    }
    wan::parse_id(receiver_sha)?;
    let run = p["run"].as_str().ok_or("run missing")?;
    let scope = scope(run, trial, large, name);
    let scope_hex = hex(&scope);
    if !p["sessions"]
        .as_array()
        .ok_or("sessions missing")?
        .iter()
        .any(|s| {
            s["trial"] == trial
                && s["large"] == large
                && s["arm"] == name
                && s["scope"] == scope_hex
        })
    {
        return Err("session outside plan".into());
    }
    let targets = SEQUENCE
        .iter()
        .enumerate()
        .map(|(position, family)| {
            let target = fixture(family, large, trial * 10 + position)?;
            validate_target(p, trial, large, position, &target.bytes)?;
            Ok(target)
        })
        .collect::<Result<Vec<_>>>()?;
    let token =
        std::env::var("SPIRALCORE_WAN_TOKEN").map_err(|_| "SPIRALCORE_WAN_TOKEN required")?;
    let mut client = HttpClient::new(url, &token, scope)?;
    write_row(
        out,
        &json!({"event":"session_registration","identity":identity()?,"source_commit":p["source_commit"],"plan_sha256":plan_sha,"scope":scope_hex,
        "trial":trial,"large":large,"arm":name,"receiver_executable_sha256":receiver_sha,"source_label":label,"url":url,
        "cpu_scope":"sender process including HTTP/TLS plus disjoint receiver handler CPU; excludes receiver socket/HTTP and relay CPU"}),
    )?;
    let mut prepared = None;
    if arm == "schema" {
        let wall = Instant::now();
        let cpu = ProcessTime::now();
        let result = schema_zstd::PreparedEncoder::new(ARTIFACT);
        let cpu_us = cpu.elapsed().as_micros();
        let wall_us = wall.elapsed().as_micros();
        write_row(
            out,
            &json!({"event":"preparation","cpu_us":cpu_us,"wall_us":wall_us,"artifact_bytes":ARTIFACT.len(),"artifact_sha256":sha(ARTIFACT),"error":result.as_ref().err().map(ToString::to_string)}),
        )?;
        prepared = Some(result?);
        let row = operation(&mut client, &mut prepared, ARTIFACT, scope, None);
        let failed = !row["error"].is_null();
        write_row(out, &row)?;
        if failed {
            return Err("artifact seed failed; no retry".into());
        }
    }
    for (position, target) in targets.iter().enumerate() {
        let mut row = operation(
            &mut client,
            &mut prepared,
            &target.bytes,
            scope,
            Some(position),
        );
        row["family"] = json!(SEQUENCE[position]);
        let failed = !row["error"].is_null();
        write_row(out, &row)?;
        if failed {
            return Err("target failed; no retry".into());
        }
    }
    write_row(
        out,
        &json!({"event":"session_complete","target_operations":10,"seed_operations":usize::from(arm=="schema"),"status":"commit-acknowledged-reopen-pending"}),
    )
}
pub(super) fn rows(bytes: &[u8]) -> Result<Vec<Value>> {
    std::str::from_utf8(bytes)?
        .lines()
        .map(|s| Ok(serde_json::from_str(s)?))
        .collect()
}
fn binary_identity(path: &Path) -> Result<Value> {
    let output = Command::new(path).arg("wan-identity").output()?;
    if !output.status.success() {
        return Err("binary identity command failed".into());
    }
    let identity: Value = serde_json::from_slice(&output.stdout)?;
    if identity["executable_sha256"] != sha(&fs::read(path)?) {
        return Err("executable identity differs".into());
    }
    Ok(identity)
}
fn validate_binary(p: &Value, id: &Value, baseline: bool) -> Result<()> {
    let mut expected = p["sources"].clone();
    expected["schema"] = json!(if baseline { OLD_SCHEMA } else { NEW_SCHEMA });
    if id["sources"] != expected {
        return Err("binary source substitution mismatch".into());
    }
    wan::parse_id(
        id["executable_sha256"]
            .as_str()
            .ok_or("executable hash required")?,
    )?;
    Ok(())
}
fn paired(
    plan_path: &Path,
    baseline: &Path,
    candidate: &Path,
    url: &str,
    receiver: &str,
    out_path: &Path,
) -> Result<()> {
    let (p, plan_sha) = read_plan(plan_path, true)?;
    let mut out = fresh(out_path)?;
    let result = (|| -> Result<()> {
        let old = binary_identity(baseline)?;
        let new = binary_identity(candidate)?;
        validate_binary(&p, &old, true)?;
        validate_binary(&p, &new, false)?;
        if old["platform"] != new["platform"] || old["hardware_sha256"] != new["hardware_sha256"] {
            return Err("sender platform/features differ".into());
        }
        wan::parse_id(receiver)?;
        write_row(
            &mut out,
            &json!({"event":"registration","plan_sha256":plan_sha,"source_commit":p["source_commit"],"run":p["run"],"shard":p["shard"],"baseline":old,"candidate":new,"receiver_executable_sha256":receiver,"url":url}),
        )?;
        let temp = out_path.with_extension("sessions");
        fs::create_dir(&temp)?;
        for (index, s) in p["sessions"]
            .as_array()
            .ok_or("sessions required")?
            .iter()
            .enumerate()
        {
            let arm = s["arm"].as_str().ok_or("arm missing")?;
            let is_old = arm == "baseline";
            let label = if is_old { "baseline" } else { "candidate" };
            let path = temp.join(format!("{index:03}.jsonl"));
            let child = Command::new(if is_old { baseline } else { candidate })
                .arg("wan-session")
                .arg(plan_path)
                .arg(label)
                .arg(s["trial"].as_u64().ok_or("trial missing")?.to_string())
                .arg(if s["large"] == true { "1" } else { "0" })
                .arg(if arm == "zstd" { "zstd" } else { "schema" })
                .arg(url)
                .arg(receiver)
                .arg(&path)
                .output();
            let output = match child {
                Ok(o) => o,
                Err(e) => {
                    write_row(
                        &mut out,
                        &json!({"event":"spawn_failure","index":index,"session":s,"error":e.to_string()}),
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
                return Err("session failed; all output retained, no retry".into());
            }
            write_row(
                &mut out,
                &json!({"event":"session","index":index,"session":s,"rows":parsed?,"raw_sha256":sha(&raw)}),
            )?;
            fs::remove_file(path)?;
        }
        fs::remove_dir(temp)?;
        write_row(
            &mut out,
            &json!({"event":"complete","target_operations":600,"seed_operations":40,"sessions":60,"verification":"pending-independent-reopen"}),
        )
    })();
    if let Err(e) = result {
        write_row(&mut out, &json!({"event":"failure","error":e.to_string()}))?;
        return Err(e);
    }
    Ok(())
}
pub(super) fn observations(p: &Value, bytes: &[u8]) -> Result<Vec<Value>> {
    let allowed = p["allowed_scopes"].as_array().ok_or("scope list missing")?;
    let mut result = Vec::new();
    let mut previous = json!({"hits":0,"misses":0,"preparations":0,"evictions":0});
    for line in std::str::from_utf8(bytes)?.lines() {
        if line.starts_with("POLICY ") || line.is_empty() {
            continue;
        }
        let row: Value = serde_json::from_str(line)?;
        let ordinal = row["request_ordinal"]
            .as_u64()
            .ok_or("request ordinal missing")?;
        let previous_ordinal = result
            .last()
            .and_then(|r: &Value| r["request_ordinal"].as_u64())
            .unwrap_or(0);
        if row["event"] != "wan_request"
            || ordinal <= previous_ordinal
            || row["http_status"] != 200
            || !allowed.contains(&row["scope"])
        {
            return Err("unexpected receiver request observation".into());
        }
        let cache = &row["codec_cache"];
        for key in ["hits", "misses", "preparations", "evictions"] {
            if numeric(cache, key)? < numeric(&previous, key)? {
                return Err("receiver counters moved backwards".into());
            }
        }
        let entries = cache["entries"].as_u64().ok_or("cache entries missing")?;
        if entries > 8 || cache["resident_bytes"] != entries * (5 << 20) {
            return Err("receiver cache charge invalid".into());
        }
        previous = cache.clone();
        result.push(row);
    }
    if result.is_empty() || result.len() > 4096 {
        return Err("receiver request count invalid".into());
    }
    Ok(result)
}
fn verify(plan_path: &Path, cache: &Path, out_path: &Path) -> Result<()> {
    let (p, plan_sha) = read_plan(plan_path, true)?;
    let mut out = fresh(out_path)?;
    let result = (|| -> Result<()> {
        if !cache.is_dir() {
            return Err("stopped existing receiver store required".into());
        }
        let log = std::env::var("SPIRALCORE_SCHEMA_OBSERVATIONS")
            .map_err(|_| "SPIRALCORE_SCHEMA_OBSERVATIONS required")?;
        let raw = fs::read(log)?;
        let observations = observations(&p, &raw)?;
        write_row(
            &mut out,
            &json!({"event":"verification_registration","plan_sha256":plan_sha,"identity":identity()?,"source_commit":p["source_commit"],"observation_sha256":sha(&raw),"observations":observations}),
        )?;
        let mut endpoint = Endpoint::open_with_policy(
            cache,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )?;
        let mut failed = false;
        let mut checked = 0;
        for s in p["sessions"].as_array().ok_or("sessions required")? {
            let scope = wan::parse_id(s["scope"].as_str().ok_or("scope missing")?)?;
            let trial = s["trial"].as_u64().ok_or("trial missing")? as usize;
            let large = s["large"].as_bool().ok_or("size missing")?;
            for position in 0..11 {
                if position == 0 && s["arm"] == "zstd" {
                    continue;
                }
                let bytes = if position == 0 {
                    ARTIFACT.to_vec()
                } else {
                    let t = fixture(SEQUENCE[position - 1], large, trial * 10 + position - 1)?;
                    validate_target(&p, trial, large, position - 1, &t.bytes)?;
                    t.bytes
                };
                let wall = Instant::now();
                let cpu = ProcessTime::now();
                let actual = endpoint.read_object(&wan::context(scope), object_id(&bytes));
                let exact = actual.as_ref().is_ok_and(|v| v == &bytes);
                let cpu_us = cpu.elapsed().as_micros();
                let wall_us = wall.elapsed().as_micros();
                failed |= !exact;
                checked += 1;
                write_row(
                    &mut out,
                    &json!({"event":"receiver_verification","scope":hex(&scope),"position":if position==0{None}else{Some(position-1)},"phase":if position==0{"seed"}else{"target"},
                    "object":hex(&object_id(&bytes)),"sha256":sha(&bytes),"bytes":bytes.len(),"exact":exact,"actual_bytes":actual.as_ref().ok().map(Vec::len),"actual_sha256":actual.as_ref().ok().map(|v|sha(v)),
                    "cpu_us":cpu_us,"wall_us":wall_us,"error":actual.err().map(|e|e.to_string()),"verification":"independent-exact-read-after-server-stop"}),
                )?;
            }
        }
        write_row(
            &mut out,
            &json!({"event":"verification_complete","checked":checked,"exact":!failed}),
        )?;
        if failed || checked != 640 {
            return Err("independent exact verification failed".into());
        }
        Ok(())
    })();
    if let Err(e) = result {
        write_row(&mut out, &json!({"event":"failure","error":e.to_string()}))?;
        return Err(e);
    }
    Ok(())
}
pub(super) const METRICS_WAN: [&str; 10] = [
    "sender_cpu_us",
    "receiver_cpu_us",
    "cpu_us",
    "wall_us",
    "exchange_wall_us",
    "exchange_cpu_us",
    "body_bytes",
    "request_body_bytes",
    "response_body_bytes",
    "batches",
];
pub(super) fn describe(group: &[&Value]) -> Result<Value> {
    let mut value = json!({"n":group.len(),"raw_fallbacks":group.iter().filter(|r|r["raw_fallback"]==true).count()});
    for metric in METRICS_WAN {
        value[metric] = dist(
            group
                .iter()
                .map(|r| numeric(r, metric))
                .collect::<Result<Vec<_>>>()?,
        );
    }
    Ok(value)
}
fn validate_operation(row: &Value, expected: &Value) -> Result<()> {
    if row["event"] != "operation"
        || !row["error"].is_null()
        || row["bytes"] != expected["bytes"]
        || row["object"] != expected["object"]
        || row["sha256"] != expected["sha256"]
        || row["family"] != expected["family"]
        || row["verification"] != "commit-acknowledged-independent-reopen-pending"
    {
        return Err("operation fidelity/status mismatch".into());
    }
    if numeric(row, "cpu_us")? != numeric(row, "sender_cpu_us")? + numeric(row, "receiver_cpu_us")?
        || numeric(row, "body_bytes")?
            != numeric(row, "request_body_bytes")? + numeric(row, "response_body_bytes")?
        || numeric(row, "exchange_wall_us")? > numeric(row, "wall_us")?
        || numeric(row, "batches")? < 1.0
        || numeric(row, "batches")? > 6.0
    {
        return Err("operation accounting mismatch".into());
    }
    if !matches!(row["mode"].as_str(), Some("Raw" | "Zstd" | "SchemaZstdV1")) {
        return Err("unexpected codec mode".into());
    }
    Ok(())
}
fn parity(a: &Value, b: &Value) -> Result<()> {
    for key in [
        "phase",
        "position",
        "bytes",
        "object",
        "sha256",
        "mode",
        "body_bytes",
        "request_body_bytes",
        "response_body_bytes",
        "batches",
    ] {
        if a[key] != b[key] {
            return Err(format!("before/after parity differs: {key}").into());
        }
    }
    Ok(())
}
fn validate_shard(
    p: &Value,
    plan_sha: &str,
    raw: &[Value],
    verified: &[Value],
) -> Result<(Vec<Value>, Vec<Value>)> {
    if raw.len() != 62
        || raw[0]["event"] != "registration"
        || raw[0]["plan_sha256"] != plan_sha
        || raw[0]["run"] != p["run"]
        || raw[0]["shard"] != p["shard"]
        || raw[61]["event"] != "complete"
        || raw[61]["target_operations"] != 600
        || raw[61]["seed_operations"] != 40
        || raw[61]["sessions"] != 60
        || verified.len() != 642
        || verified[0]["event"] != "verification_registration"
        || verified[0]["plan_sha256"] != plan_sha
        || verified[641]["event"] != "verification_complete"
        || verified[641]["checked"] != 640
        || verified[641]["exact"] != true
    {
        return Err("incomplete shard receipts".into());
    }
    validate_binary(p, &raw[0]["baseline"], true)?;
    validate_binary(p, &raw[0]["candidate"], false)?;
    validate_binary(p, &verified[0]["identity"], false)?;
    if raw[0]["source_commit"] != p["source_commit"]
        || verified[0]["source_commit"] != p["source_commit"]
    {
        return Err("source commit mismatch".into());
    }
    let observations = verified[0]["observations"]
        .as_array()
        .ok_or("receiver observations required")?;
    let log = observations
        .iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");
    self::observations(p, log.as_bytes())?;
    wan::parse_id(
        verified[0]["observation_sha256"]
            .as_str()
            .ok_or("observation digest missing")?,
    )?;
    let mut operations = Vec::new();
    let mut totals = Vec::new();
    let mut checked = BTreeSet::new();
    let mut request_index: usize = 0;
    let mut cache_previous = json!({"hits":0,"misses":0,"preparations":0,"evictions":0});
    for (index, s) in p["sessions"]
        .as_array()
        .ok_or("sessions missing")?
        .iter()
        .enumerate()
    {
        let outer = &raw[index + 1];
        let arm = s["arm"].as_str().ok_or("arm missing")?;
        if outer["event"] != "session" || outer["index"] != index || outer["session"] != *s {
            return Err("registered session order differs".into());
        }
        let rows = outer["rows"].as_array().ok_or("session rows missing")?;
        let is_schema = arm != "zstd";
        if rows.len() != if is_schema { 14 } else { 12 }
            || rows[0]["event"] != "session_registration"
            || rows.last().unwrap()["event"] != "session_complete"
            || rows.last().unwrap()["target_operations"] != 10
            || rows.last().unwrap()["seed_operations"] != usize::from(is_schema)
            || rows[0]["scope"] != s["scope"]
            || rows[0]["arm"] != arm
            || rows[0]["trial"] != s["trial"]
            || rows[0]["large"] != s["large"]
            || rows[0]["plan_sha256"] != plan_sha
            || rows[0]["identity"]
                != raw[0][if arm == "baseline" {
                    "baseline"
                } else {
                    "candidate"
                }]
            || rows[0]["receiver_executable_sha256"] != raw[0]["receiver_executable_sha256"]
        {
            return Err("session identity/count mismatch".into());
        }
        let trial = s["trial"].as_u64().ok_or("trial missing")? as usize;
        let large = s["large"].as_bool().ok_or("size missing")?;
        let mut total = json!({"trial":trial,"large":large,"arm":arm,"target_operations":10,"seed_operations":usize::from(is_schema)});
        for key in METRICS_WAN {
            total[key] = json!(0.0);
        }
        if is_schema {
            let prep = &rows[1];
            if prep["event"] != "preparation"
                || !prep["error"].is_null()
                || prep["artifact_bytes"] != 169
                || prep["artifact_sha256"] != sha(ARTIFACT)
            {
                return Err("preparation failed/changed".into());
            }
            total["preparation_cpu_us"] = prep["cpu_us"].clone();
            total["preparation_wall_us"] = prep["wall_us"].clone();
            total["cpu_us"] = prep["cpu_us"].clone();
            total["sender_cpu_us"] = prep["cpu_us"].clone();
            total["wall_us"] = prep["wall_us"].clone();
        } else {
            total["preparation_cpu_us"] = json!(0);
            total["preparation_wall_us"] = json!(0);
        }
        let mut seen_codec = false;
        for (op_index, row) in rows
            .iter()
            .filter(|r| r["event"] == "operation")
            .enumerate()
        {
            let seed = is_schema && op_index == 0;
            let position = if seed {
                None
            } else {
                Some(op_index - usize::from(is_schema))
            };
            let expected = if let Some(pos) = position {
                target_manifest(p, trial, large, pos)?.clone()
            } else {
                json!({"bytes":ARTIFACT.len(),"sha256":sha(ARTIFACT),"object":hex(&object_id(ARTIFACT))})
            };
            validate_operation(row, &expected)?;
            if row["phase"] != if seed { "seed" } else { "target" }
                || row["position"] != json!(position)
                || (seed && row["mode"] != "Raw")
                || (arm == "zstd" && row["mode"] != "Zstd")
            {
                return Err("operation phase/mode mismatch".into());
            }
            let matches = verified[1..641]
                .iter()
                .filter(|v| {
                    v["scope"] == s["scope"]
                        && v["phase"] == row["phase"]
                        && v["position"] == row["position"]
                })
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                return Err("missing/duplicate independent readback".into());
            }
            let v = matches[0];
            if !checked.insert((s["scope"].as_str().unwrap().to_owned(), position))
                || v["exact"] != true
                || v["object"] != row["object"]
                || v["sha256"] != row["sha256"]
                || v["bytes"] != row["bytes"]
                || v["actual_sha256"] != v["sha256"]
                || v["actual_bytes"] != v["bytes"]
                || !v["error"].is_null()
            {
                return Err("independent readback mismatch".into());
            }
            let batches = row["batches"].as_u64().ok_or("batch count missing")? as usize;
            let end = request_index
                .checked_add(batches)
                .ok_or("request overflow")?;
            let requests = observations
                .get(request_index..end)
                .ok_or("missing HTTP observations")?;
            if requests.iter().any(|r| r["scope"] != s["scope"]) {
                return Err("request scopes/order differ".into());
            }
            let cache = &requests.last().ok_or("empty requests")?["codec_cache"];
            let fitted = row["mode"] == "SchemaZstdV1";
            let hit = u64::from(fitted && seen_codec);
            let miss = u64::from(fitted && !seen_codec);
            for (key, increment) in [("hits", hit), ("misses", miss), ("preparations", miss)] {
                if numeric(cache, key)? - numeric(&cache_previous, key)? != increment as f64 {
                    return Err("persistent decoder cache progression differs".into());
                }
            }
            seen_codec |= fitted;
            cache_previous = cache.clone();
            request_index = end;
            let mut op = row.clone();
            op["trial"] = json!(trial);
            op["large"] = json!(large);
            op["arm"] = json!(arm);
            op["scope"] = s["scope"].clone();
            op["cache_hits"] = json!(hit);
            op["cache_preparations"] = json!(miss);
            for key in METRICS_WAN {
                total[key] = json!(numeric(&total, key)? + numeric(row, key)?);
            }
            operations.push(op);
        }
        if checked.len() != operations.len() {
            return Err("duplicate operation".into());
        }
        totals.push(total);
    }
    if checked.len() != 640 || request_index != observations.len() {
        return Err("shard operations/request observations differ".into());
    }
    Ok((operations, totals))
}
pub(super) fn ratios(values: Vec<f64>) -> Result<Value> {
    if values.len() != 30 || values.iter().any(|v| !v.is_finite() || *v <= 0.0) {
        return Err("thirty positive paired ratios required".into());
    }
    Ok(
        json!({"paired_trials":30,"geometric_mean":(values.iter().map(|v|v.ln()).sum::<f64>()/30.0).exp(),"wins":values.iter().filter(|v|**v<1.0).count(),"ties":values.iter().filter(|v|**v==1.0).count(),"distribution":dist(values.clone()),"ratios":values}),
    )
}
fn summary(paths: &[String]) -> Result<Value> {
    let mut operations = Vec::new();
    let mut totals = Vec::new();
    let mut receipts = Vec::new();
    let mut registration = None;
    for shard in 0..3 {
        let (p, plan_sha) = read_plan(Path::new(&paths[3 * shard]), true)?;
        if p["shard"] != shard {
            return Err("shard order differs".into());
        }
        let raw = fs::read(&paths[3 * shard + 1])?;
        let verified = fs::read(&paths[3 * shard + 2])?;
        let rr = rows(&raw)?;
        let vv = rows(&verified)?;
        let signature = json!({"run":p["run"],"source_commit":p["source_commit"],"sources":p["sources"],"baseline":rr.first().ok_or("empty sender log")?["baseline"],"candidate":rr[0]["candidate"],"receiver":rr[0]["receiver_executable_sha256"]});
        if registration.as_ref().is_some_and(|r| r != &signature) {
            return Err("shard source/executable/run identities differ".into());
        }
        registration = Some(signature);
        let (ops, session_totals) = validate_shard(&p, &plan_sha, &rr, &vv)?;
        let observed = vv[0]["observations"]
            .as_array()
            .ok_or("observations missing")?;
        receipts.push(json!({"shard":shard,"plan_sha256":plan_sha,"sender_sha256":sha(&raw),"verification_sha256":sha(&verified),"observations_sha256":vv[0]["observation_sha256"],"final_codec_cache":observed.last().map(|r|&r["codec_cache"]),
            "admitted_requests":observed.len(),"request_ordinal_gaps":observed.last().and_then(|r|r["request_ordinal"].as_u64()).unwrap_or(0).saturating_sub(observed.len() as u64),"ordinal_gap_scope":"unobserved rejected/incomplete request attempts; retained, not assigned to measured operations"}));
        operations.extend(ops);
        totals.extend(session_totals);
    }
    let mut paired = Vec::new();
    let mut cells = Vec::new();
    let mut parity_count = 0;
    for large in [false, true] {
        for trial in 0..30 {
            for position in [None].into_iter().chain((0..10).map(Some)) {
                let get = |arm: &str| -> Result<&Value> {
                    let matches = operations
                        .iter()
                        .filter(|r| {
                            r["large"] == large
                                && r["trial"] == trial
                                && r["arm"] == arm
                                && r["position"] == json!(position)
                        })
                        .collect::<Vec<_>>();
                    if matches.len() != 1 {
                        return Err("paired observation missing".into());
                    }
                    Ok(matches[0])
                };
                parity(get("baseline")?, get("candidate")?)?;
                parity_count += 1;
            }
        }
        for arm in ARMS_WAN {
            for family in [
                "normal",
                "variants",
                "drift",
                "unrelated",
                "normal_first",
                "normal_repeated",
            ] {
                let group = operations
                    .iter()
                    .filter(|r| {
                        r["large"] == large
                            && r["arm"] == arm
                            && r["phase"] == "target"
                            && match family {
                                "normal_first" => r["position"] == 0,
                                "normal_repeated" => r["family"] == "normal" && r["position"] != 0,
                                _ => r["family"] == family,
                            }
                    })
                    .collect::<Vec<_>>();
                let mut cell = describe(&group)?;
                cell["large"] = json!(large);
                cell["arm"] = json!(arm);
                cell["family"] = json!(family);
                cells.push(cell);
            }
        }
        for arm in ["baseline", "candidate"] {
            let group = operations
                .iter()
                .filter(|r| r["large"] == large && r["arm"] == arm && r["phase"] == "seed")
                .collect::<Vec<_>>();
            let mut cell = describe(&group)?;
            cell["large"] = json!(large);
            cell["arm"] = json!(arm);
            cell["family"] = json!("seed");
            cells.push(cell);
        }
        for family in FAMILIES {
            for (numerator, denominator) in [
                ("candidate", "baseline"),
                ("candidate", "zstd"),
                ("baseline", "zstd"),
            ] {
                let mut comparison = json!({"large":large,"family":family,"numerator":numerator,"denominator":denominator,"unit":"per-trial sum across all targets in this family; 30 independent paired trial blocks"});
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
                            operations
                                .iter()
                                .filter(|r| {
                                    r["large"] == large
                                        && r["trial"] == trial
                                        && r["arm"] == arm
                                        && r["family"] == family
                                })
                                .map(|r| numeric(r, key))
                                .sum()
                        };
                        values.push(sum(numerator)? / sum(denominator)?);
                    }
                    comparison[key] = ratios(values)?;
                }
                paired.push(comparison);
            }
        }
    }
    let mut session_cells = Vec::new();
    let mut actual_totals = Vec::new();
    for large in [false, true] {
        for arm in ARMS_WAN {
            let group = totals
                .iter()
                .filter(|r| r["large"] == large && r["arm"] == arm)
                .collect::<Vec<_>>();
            let mut cell = describe(&group)?;
            cell["large"] = json!(large);
            cell["arm"] = json!(arm);
            cell["preparation_cpu_us"] = dist(
                group
                    .iter()
                    .map(|r| numeric(r, "preparation_cpu_us"))
                    .collect::<Result<Vec<_>>>()?,
            );
            cell["preparation_wall_us"] = dist(
                group
                    .iter()
                    .map(|r| numeric(r, "preparation_wall_us"))
                    .collect::<Result<Vec<_>>>()?,
            );
            session_cells.push(cell);
            let mut total =
                json!({"large":large,"arm":arm,"actual_sessions":30,"actual_target_uses":300});
            for key in METRICS_WAN {
                let sum = group.iter().map(|r| numeric(r, key)).sum::<Result<f64>>()?;
                total[key] = json!(sum);
                total[format!("{key}_per_target_including_actual_setup")] = json!(sum / 300.0);
            }
            actual_totals.push(total);
        }
    }
    let mut paired_sessions = Vec::new();
    for large in [false, true] {
        for (numerator, denominator) in [
            ("candidate", "baseline"),
            ("candidate", "zstd"),
            ("baseline", "zstd"),
        ] {
            let mut comparison =
                json!({"large":large,"numerator":numerator,"denominator":denominator});
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
                        let r = totals
                            .iter()
                            .find(|r| r["large"] == large && r["trial"] == trial && r["arm"] == arm)
                            .ok_or("session total missing")?;
                        numeric(r, key)
                    };
                    values.push(get(numerator)? / get(denominator)?);
                }
                comparison[key] = ratios(values)?;
            }
            paired_sessions.push(comparison);
        }
    }
    Ok(
        json!({"schema":"spiralcore.schema-wan.summary.v1","verdict":"COMPLETE_EXACT_PAIRED_WAN_CAMPAIGN","registration":registration,"receipts":receipts,"target_operations":1800,"seed_operations":120,"sender_preparations":120,"sessions":180,"exact_reopened_objects":1920,"before_after_body_mode_pairs":parity_count,"discarded":0,
        "metric_scope":"HTTP protocol-body bytes, sender process CPU plus disjoint receiver handler CPU, operation entry through durable commit acknowledgment; excludes receiver HTTP/socket and relay CPU; not packet bytes or pure network time",
        "cells":cells,"paired_family_sums":paired,"setup_inclusive_session_cells":session_cells,"paired_setup_inclusive_sessions":paired_sessions,"actual_totals":actual_totals,"session_totals":totals}),
    )
}
pub(super) fn command(args: &[String]) -> Result<()> {
    match args.get(1).map(String::as_str) {
        Some("wan-identity") if args.len()==2=>{println!("{}",serde_json::to_string(&identity()?)?);Ok(())},
        Some("wan-plan") if args.len()==5=>{let source=std::env::var("SPIRALCORE_SCHEMA_SOURCE").map_err(|_|"SPIRALCORE_SCHEMA_SOURCE required")?;write_row(&mut fresh(Path::new(&args[4]))?,&plan(&args[2],args[3].parse()?,&source)?)},
        Some("wan-session") if args.len()==10=>{let(p,digest)=read_plan(Path::new(&args[2]),false)?;let large=match args[5].as_str(){"0"=>false,"1"=>true,_=>return Err("size must be0or1".into())};let mut out=fresh(Path::new(&args[9]))?;let result=session(&p,&digest,&args[3],args[4].parse()?,large,&args[6],&args[7],&args[8],&mut out);if let Err(e)=result{write_row(&mut out,&json!({"event":"failure","error":e.to_string()}))?;return Err(e);}Ok(())},
        Some("wan-paired") if args.len()==8=>paired(Path::new(&args[2]),Path::new(&args[3]),Path::new(&args[4]),&args[5],&args[6],Path::new(&args[7])),
        Some("wan-verify") if args.len()==5=>verify(Path::new(&args[2]),Path::new(&args[3]),Path::new(&args[4])),
        Some("wan-summary") if args.len()==12=>{let value=summary(&args[2..11])?;write_row(&mut fresh(Path::new(&args[11]))?,&value)},
        _=>Err("usage: wan-plan RUN SHARD NEW_PLAN | wan-session PLAN baseline|candidate TRIAL0..29 LARGE0|1 schema|zstd URL RECEIVER_SHA NEW_JSONL | wan-paired PLAN BASE_EXE CAND_EXE URL RECEIVER_SHA NEW_JSONL | wan-verify PLAN STOPPED_CACHE NEW_JSONL | wan-summary PLAN0 RAW0 VERIFY0 PLAN1 RAW1 VERIFY1 PLAN2 RAW2 VERIFY2 NEW_SUMMARY".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn family_label_must_match_the_frozen_target() {
        let p = plan("family-label-test", 0, &"0".repeat(40)).unwrap();
        let (mut raw, verified) = fabricated_receipts(&p, "unit-test-digest");
        let row = raw[1]["rows"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["event"] == "operation" && r["phase"] == "target")
            .unwrap();
        row["family"] = json!("unrelated");
        assert!(validate_shard(&p, "unit-test-digest", &raw, &verified).is_err());
    }
    // Fabricated accounting receipts are unit-test data only, never measurements.
    fn fabricated_receipts(p: &Value, digest: &str) -> (Vec<Value>, Vec<Value>) {
        let new = json!({"sources":p["sources"],"executable_sha256":"b".repeat(64),"platform":"unit-test","hardware_sha256":false});
        let mut old = new.clone();
        old["sources"]["schema"] = json!(OLD_SCHEMA);
        old["executable_sha256"] = json!("a".repeat(64));
        let receiver = "c".repeat(64);
        let mut raw = vec![
            json!({"event":"registration","run":p["run"],"shard":p["shard"],"plan_sha256":digest,"source_commit":p["source_commit"],"baseline":old,"candidate":new,"receiver_executable_sha256":receiver}),
        ];
        let mut verified = vec![
            json!({"event":"verification_registration","plan_sha256":digest,"identity":new,"source_commit":p["source_commit"],"observation_sha256":"d".repeat(64)}),
        ];
        let mut observations = Vec::new();
        let mut hits = 0;
        let mut preparations = 0u64;
        for (index, s) in p["sessions"].as_array().unwrap().iter().enumerate() {
            let arm = s["arm"].as_str().unwrap();
            let fitted = arm != "zstd";
            let trial = s["trial"].as_u64().unwrap() as usize;
            let large = s["large"].as_bool().unwrap();
            let mut session = vec![
                json!({"event":"session_registration","scope":s["scope"],"trial":trial,"large":large,"arm":arm,"plan_sha256":digest,"identity":if arm=="baseline"{&old}else{&new},"receiver_executable_sha256":receiver}),
            ];
            if fitted {
                session.push(json!({"event":"preparation","cpu_us":1,"wall_us":2,"artifact_bytes":169,"artifact_sha256":sha(ARTIFACT),"error":null}));
            }
            for n in usize::from(!fitted)..11 {
                let pos = if n == 0 { None } else { Some(n - 1) };
                let m = if let Some(pos) = pos {
                    target_manifest(p, trial, large, pos).unwrap().clone()
                } else {
                    json!({"bytes":169,"sha256":sha(ARTIFACT),"object":hex(&object_id(ARTIFACT))})
                };
                if fitted && n > 0 {
                    if n == 1 {
                        preparations += 1
                    } else {
                        hits += 1
                    }
                }
                let codec = if n == 0 {
                    "Raw"
                } else if fitted {
                    "SchemaZstdV1"
                } else {
                    "Zstd"
                };
                let op = json!({"event":"operation","phase":if n==0{"seed"}else{"target"},"position":pos,"family":pos.map(|p|SEQUENCE[p]),"bytes":m["bytes"],"sha256":m["sha256"],"object":m["object"],"error":null,"verification":"commit-acknowledged-independent-reopen-pending",
                    "cpu_us":9,"sender_cpu_us":5,"receiver_cpu_us":4,"wall_us":20,"exchange_cpu_us":2,"exchange_wall_us":12,"receiver_wall_us":7,"body_bytes":20,"request_body_bytes":12,"response_body_bytes":8,"batches":1,"mode":codec,"raw_fallback":false});
                verified.push(json!({"event":"receiver_verification","scope":s["scope"],"phase":op["phase"],"position":pos,"exact":true,"object":m["object"],"sha256":m["sha256"],"bytes":m["bytes"],"actual_sha256":m["sha256"],"actual_bytes":m["bytes"],"error":null}));
                observations.push(json!({"event":"wan_request","request_ordinal":observations.len()+1,"scope":s["scope"],"http_status":200,"codec_cache":{"hits":hits,"misses":preparations,"preparations":preparations,"evictions":preparations.saturating_sub(8),"entries":preparations.min(8),"resident_bytes":preparations.min(8)*(5<<20)}}));
                session.push(op);
            }
            session.push(json!({"event":"session_complete","target_operations":10,"seed_operations":usize::from(fitted)}));
            raw.push(json!({"event":"session","index":index,"session":s,"rows":session}));
        }
        raw.push(
            json!({"event":"complete","target_operations":600,"seed_operations":40,"sessions":60}),
        );
        verified[0]["observations"] = json!(observations);
        verified.push(json!({"event":"verification_complete","checked":640,"exact":true}));
        (raw, verified)
    }
    #[test]
    fn complete_summary_accepts_all_shards_and_rejects_missing_or_corrupt_rows() {
        let directory =
            std::env::temp_dir().join(format!("schema-wan-summary-test-{}", std::process::id()));
        assert!(!directory.exists());
        fs::create_dir(&directory).unwrap();
        let mut paths = Vec::new();
        for shard in 0..3 {
            let p = plan("unit-test-summary", shard, &"0".repeat(40)).unwrap();
            let file = directory.join(format!("plan{shard}.json"));
            write_row(&mut fresh(&file).unwrap(), &p).unwrap();
            let digest = sha(&fs::read(&file).unwrap());
            let (raw, verified) = fabricated_receipts(&p, &digest);
            assert!(validate_shard(&p, &digest, &raw, &verified).is_ok());
            let mut missing = verified.clone();
            missing.remove(1);
            assert!(validate_shard(&p, &digest, &raw, &missing).is_err());
            let mut corrupt = verified.clone();
            corrupt[1]["actual_sha256"] = json!("wrong");
            assert!(validate_shard(&p, &digest, &raw, &corrupt).is_err());
            let mut wrong = raw.clone();
            wrong[1]["rows"][0]["identity"]["sources"]["client"] = json!("changed");
            assert!(validate_shard(&p, &digest, &wrong, &verified).is_err());
            paths.push(file.to_str().unwrap().to_owned());
            for (name, rows) in [("raw", raw), ("verified", verified)] {
                let file = directory.join(format!("{name}{shard}.jsonl"));
                let mut out = fresh(&file).unwrap();
                for row in rows {
                    write_row(&mut out, &row).unwrap();
                }
                paths.push(file.to_str().unwrap().to_owned());
            }
        }
        let result = summary(&paths).unwrap();
        assert_eq!(result["exact_reopened_objects"], 1920);
        assert_eq!(result["before_after_body_mode_pairs"], 660);
        assert_eq!(
            result["paired_family_sums"][0]["sender_cpu_us"]["geometric_mean"],
            1.0
        );
        let mut raw = rows(&fs::read(&paths[1]).unwrap()).unwrap();
        let target = raw
            .iter_mut()
            .find(|r| r["session"]["arm"] == "candidate")
            .unwrap();
        target["rows"][3]["body_bytes"] = json!(21);
        target["rows"][3]["request_body_bytes"] = json!(13);
        let replacement = directory.join("parity-corruption.jsonl");
        let mut file = fresh(&replacement).unwrap();
        for row in raw {
            write_row(&mut file, &row).unwrap();
        }
        paths[1] = replacement.to_str().unwrap().to_owned();
        assert!(summary(&paths).is_err());
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn thirty_trials_cover_six_orders_and_isolate_every_scope() {
        let mut scopes = std::collections::BTreeSet::new();
        for large in [false, true] {
            let mut counts = std::collections::BTreeMap::new();
            for trial in 0..30 {
                *counts.entry(order(trial, large).unwrap()).or_insert(0) += 1;
                for arm in ARMS_WAN {
                    assert!(scopes.insert(scope("qualified-run", trial, large, arm)));
                }
            }
            assert_eq!(counts.len(), 6);
            assert!(counts.values().all(|n| *n == 5));
        }
        assert_eq!(scopes.len(), 180);
        assert!(order(30, false).is_err());
        assert_ne!(
            scope("run-a", 0, false, "baseline"),
            scope("run-b", 0, false, "baseline")
        );
    }
    #[test]
    fn expanded_fixture_range_keeps_historical_identity() {
        let old: Value = serde_json::from_slice(include_bytes!(
            "../../../../docs/network/evidence/2026-10-30-schema-sender/plan.json"
        ))
        .unwrap();
        for row in old["targets"].as_array().unwrap() {
            let data = fixture(
                row["family"].as_str().unwrap(),
                row["large"].as_bool().unwrap(),
                row["fixture_index"].as_u64().unwrap() as usize,
            )
            .unwrap();
            assert_eq!(sha(&data.bytes), row["sha256"]);
        }
        assert!(fixture("normal", false, 299).is_ok());
        assert!(fixture("normal", false, 300).is_err());
    }
    #[test]
    fn shard_budget_and_summary_guards_are_fail_closed() {
        let p = plan("test-registration", 2, &"0".repeat(40)).unwrap();
        assert_eq!(p["sessions"].as_array().unwrap().len(), 60);
        assert_eq!(p["targets"].as_array().unwrap().len(), 200);
        assert!(p["conservative_storage_bound"].as_u64().unwrap() < 768 << 20);
        assert!(p["retained_payload_bytes"].as_u64().unwrap() > 300 << 20);
        assert!(plan("bad/run", 0, &"0".repeat(40)).is_err());
        assert!(plan("test", 3, &"0".repeat(40)).is_err());
        assert!(validate_shard(&p, "digest", &[], &[]).is_err());
        let mut id = json!({"sources":p["sources"],"executable_sha256":"a".repeat(64)});
        assert!(validate_binary(&p, &id, false).is_ok());
        id["sources"]["client"] = json!("changed");
        assert!(validate_binary(&p, &id, false).is_err());
        let expected = json!({"bytes":1,"object":"object","sha256":"digest"});
        let mut op = json!({"event":"operation","error":null,"bytes":1,"object":"object","sha256":"digest","verification":"commit-acknowledged-independent-reopen-pending",
            "cpu_us":9,"sender_cpu_us":5,"receiver_cpu_us":4,"body_bytes":20,"request_body_bytes":12,"response_body_bytes":8,"wall_us":10,"exchange_wall_us":8,"batches":1,"mode":"Raw"});
        assert!(validate_operation(&op, &expected).is_ok());
        let baseline = op.clone();
        op["body_bytes"] = json!(21);
        assert!(parity(&baseline, &op).is_err());
        assert!(validate_operation(&op, &expected).is_err());
        assert!(ratios(vec![1.0; 29]).is_err());
        assert_eq!(ratios(vec![1.1; 30]).unwrap()["wins"], 0);
    }
    #[test]
    fn loopback_persistent_session_reopens_all_ten_exact_targets() {
        use std::{
            net::TcpListener,
            sync::{Arc, Mutex},
            time::{Duration, SystemTime, UNIX_EPOCH},
        };
        let path = std::env::temp_dir().join(format!(
            "spiralcore-schema-wan-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(!path.exists());
        let mut endpoint = Endpoint::open_with_policy(
            &path,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        endpoint.set_codec_cache_enabled(true).unwrap();
        let scope = scope("loopback-test", 29, false, "candidate");
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
                &BTreeSet::from([scope]),
                Duration::from_secs(20),
                11,
                move |o| {
                    capture.lock().unwrap().push(json!({"request_ordinal":o.request_ordinal,"codec_cache":{"hits":o.codec_cache.hits,"preparations":o.codec_cache.preparations}}));
                    Ok(())
                },
            )
        });
        let mut client = HttpClient::new(&format!("http://{address}/"), &token, scope).unwrap();
        let mut prepared = Some(schema_zstd::PreparedEncoder::new(ARTIFACT).unwrap());
        let seed = operation(&mut client, &mut prepared, ARTIFACT, scope, None);
        assert!(seed["error"].is_null());
        assert_eq!(seed["body_bytes"], 670);
        let targets = SEQUENCE
            .iter()
            .enumerate()
            .map(|(position, family)| fixture(family, false, 290 + position).unwrap().bytes)
            .collect::<Vec<_>>();
        for (position, bytes) in targets.iter().enumerate() {
            let row = operation(&mut client, &mut prepared, bytes, scope, Some(position));
            assert!(row["error"].is_null(), "{row}");
            assert_eq!(row["batches"], 1);
        }
        thread.join().unwrap().unwrap();
        drop(client);
        let mut reopened = Endpoint::open_with_policy(
            &path,
            wan::limits(),
            BenchAuthority,
            ColdGraphPolicy::Disabled,
        )
        .unwrap();
        for bytes in [ARTIFACT]
            .into_iter()
            .chain(targets.iter().map(Vec::as_slice))
        {
            assert_eq!(
                reopened
                    .read_object(&wan::context(scope), object_id(bytes))
                    .unwrap(),
                bytes
            );
        }
        drop(reopened);
        fs::remove_dir_all(path).unwrap();
        let observations = observed.lock().unwrap();
        assert_eq!(observations.len(), 11);
        assert!(
            observations.last().unwrap()["codec_cache"]["hits"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert_eq!(
            observations.last().unwrap()["codec_cache"]["preparations"],
            1
        );
    }
}
