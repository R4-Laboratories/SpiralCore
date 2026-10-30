//! Independent receipt validation and arithmetic; never runs the measured program.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

type R<T> = Result<T, String>;
const SOURCE: &str = "35114daef384186dacd6123221ec9b5db60b3fcc";
const EXE: &str = "c91be1394ef967f29d1b84fda1345310ed7cd570823c0fad65506c8b8777f0be";
const ARMS: [&str; 4] = ["raw", "zstd", "adaptive", "runtime"];
const CASES: [&str; 3] = ["generated", "edited", "unrelated"];
const METRICS: [&str; 13] = [
    "body_bytes",
    "operation_cpu_us",
    "cpu_subtotal_us",
    "operation_wall_us",
    "receiver_cpu_us",
    "receiver_wall_us",
    "exchange_cpu_us",
    "exchange_wall_us",
    "verification_cpu_us",
    "verification_wall_us",
    "request_body_bytes",
    "response_body_bytes",
    "batches",
];

fn ensure(condition: bool, label: impl Into<String>) -> R<()> {
    if condition {
        Ok(())
    } else {
        Err(label.into())
    }
}
fn u(v: &Value, k: &str) -> R<u64> {
    v[k].as_u64().ok_or_else(|| format!("invalid unsigned {k}"))
}
fn s<'a>(v: &'a Value, k: &str) -> R<&'a str> {
    v[k].as_str().ok_or_else(|| format!("invalid string {k}"))
}
fn array<'a>(v: &'a Value, k: &str) -> R<&'a Vec<Value>> {
    v[k].as_array().ok_or_else(|| format!("invalid array {k}"))
}
fn sha(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
fn hex64(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|x| x.is_ascii_digit() || (b'a'..=b'f').contains(&x))
}
fn read(path: &Path) -> R<Vec<u8>> {
    fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}
fn next<'a>(rows: &'a [Value], at: &mut usize, event: &str) -> R<&'a Value> {
    let row = rows
        .get(*at)
        .ok_or_else(|| format!("missing row {} expected{event}", *at))?;
    ensure(
        row["event"] == event,
        format!("row{} expected{event}, saw{}", *at, row["event"]),
    )?;
    *at += 1;
    Ok(row)
}
fn scope(run_id: &str, width: u64, steps: u64, case: &str, trial: u64, arm: &str) -> String {
    let mut bytes = b"spiralcore.runtime.measurement.scope.v1\0".to_vec();
    for part in [
        run_id.as_bytes(),
        SOURCE.as_bytes(),
        &width.to_le_bytes(),
        &(steps as u32).to_le_bytes(),
        case.as_bytes(),
        &trial.to_le_bytes(),
        arm.as_bytes(),
    ] {
        bytes.extend_from_slice(part);
    }
    sha(&bytes)
}
fn prediction_length(width: u64, steps: u64) -> u64 {
    let mut retained = 0u64;
    let mut bytes = 8;
    for n in 1..=steps {
        retained += 1;
        if n % 3 == 0 && retained >= 3 {
            let (mut quotient, mut power) = (n, 1);
            while quotient % 3 == 0 {
                quotient /= 3;
                power *= 3;
            }
            retained = retained - retained.min(power) + 1;
        }
        retained = retained.min(81);
        bytes += 4 + 310 + 80 * retained + 6 * width.div_ceil(8);
    }
    bytes
}
fn row_identity(
    row: &Value,
    pair: &str,
    arm: &str,
    phase: &str,
    scope: &str,
    order: usize,
) -> R<()> {
    ensure(
        row["pair"] == pair
            && row["arm"] == arm
            && row["phase"] == phase
            && row["scope"] == scope
            && row["order"] == order,
        format!("operation identity/order {pair}/{arm}/{phase}"),
    )
}

#[derive(Clone)]
struct Dataset {
    label: String,
    plan: Value,
    plan_sha: String,
    rows: Vec<Value>,
    raw_sha: String,
}
struct Validated {
    operations: Vec<Value>,
    generation: Vec<Value>,
    pairs: Vec<Value>,
    selection_rows: usize,
}

fn validate(data: &Dataset) -> R<Validated> {
    let p = &data.plan;
    let cfg = &p["config"];
    let (width, steps) = match data.label.as_str() {
        "wan" => (4096, 4096),
        _ => return Err("unknown registered dose".into()),
    };
    ensure(
        p["schema"] == "spiralcore.wan.plan.v4"
            && p["experiment"] == "spiralcore.shared-runtime.v1",
        "plan schema",
    )?;
    ensure(
        cfg["source"] == SOURCE
            && cfg["widths"] == json!([width])
            && cfg["steps"] == json!([steps])
            && cfg["trials"] == 3
            && cfg["cases"] == json!(CASES)
            && cfg["transport"] == "remote-http",
        "registered plan dose/source",
    )?;
    ensure(
        p["methods"] == json!(ARMS)
            && p["expected_operations"] == 72
            && p["seed_mode"] == "raw"
            && p["warm_probe"] == false
            && p["cold_graph_warming"] == false,
        "registered method/policy",
    )?;
    let fixtures = array(p, "fixtures")?;
    ensure(fixtures.len() == 9, "fixture count")?;
    let run_id = s(cfg, "run_id")?;
    ensure(run_id == "shared-runtime-wan-02", "run identity")?;
    let mut at = 0;
    let header = next(&data.rows, &mut at, "run")?;
    ensure(
        header["config"] == *cfg
            && header["source"] == SOURCE
            && header["plan_sha256"] == data.plan_sha
            && header["source_files"] == p["source_files"]
            && header["executable_sha256"] == EXE,
        "run source/plan/executable binding",
    )?;
    ensure(
        header["architecture"] == "x86_64"
            && header["os"] == "linux"
            && header["command"] == "remote",
        "remote sender platform",
    )?;
    let mut operations = Vec::new();
    let mut generation = Vec::new();
    let mut pairs = Vec::new();
    let mut scopes = Vec::new();
    let mut selections = 0;
    for (case_index, case) in CASES.iter().enumerate() {
        for trial in 0..3u64 {
            let fixture = &fixtures[case_index * 3 + trial as usize];
            ensure(
                fixture["width"] == width
                    && fixture["steps"] == steps
                    && fixture["case"] == *case
                    && fixture["trial"] == trial,
                "plan fixture identity/order",
            )?;
            let expected_len = prediction_length(width, steps);
            ensure(
                u(fixture, "target_bytes")? == expected_len
                    && u(fixture, "prediction_bytes")? == expected_len
                    && u(fixture, "checkpoint_bytes")? == 310 + 6 * width.div_ceil(8),
                "independent exact trace/checkpoint lengths",
            )?;
            for name in [
                "target_sha256",
                "target_object",
                "checkpoint_sha256",
                "checkpoint_object",
                "prediction_sha256",
            ] {
                ensure(hex64(s(fixture, name)?), format!("fixture hash {name}"))?;
            }
            ensure(
                (fixture["target_sha256"] == fixture["prediction_sha256"])
                    == (*case == "generated"),
                "generated/control hash relationship",
            )?;
            let mut recipe = b"SCRCP001".to_vec();
            recipe.extend_from_slice(&(steps as u32).to_le_bytes());
            recipe.extend_from_slice(&1f64.to_le_bytes());
            recipe.extend_from_slice(&[1, 0, 0, 0]);
            ensure(
                fixture["recipe_hex"] == hex(&recipe),
                "recipe independently encoded",
            )?;
            let pair = format!("w{width}-s{steps}-{case}-t{trial}");
            ensure(
                next(&data.rows, &mut at, "generation_started")?["pair"] == pair,
                "generation start identity",
            )?;
            let gen = next(&data.rows, &mut at, "generation")?;
            ensure(
                gen["pair"] == pair && gen["fixture"] == *fixture,
                "generation fixture equality",
            )?;
            u(gen, "generation_cpu_us")?;
            u(gen, "generation_wall_us")?;
            let mut gen = gen.clone();
            gen["dataset"] = json!(data.label);
            generation.push(gen);
            let mut grouped = BTreeMap::<String, Value>::new();
            for arm in ARMS {
                scopes.push(scope(run_id, width, steps, case, trial, arm));
            }
            for order in 0..4usize {
                let arm = ARMS[(order + trial as usize + case_index) % 4];
                let auth = scope(run_id, width, steps, case, trial, arm);
                let mut pair_arm = serde_json::Map::new();
                for phase in ["seed", "target"] {
                    row_identity(
                        next(&data.rows, &mut at, "operation_started")?,
                        &pair,
                        arm,
                        phase,
                        &auth,
                        order,
                    )?;
                    let row = next(&data.rows, &mut at, "operation")?;
                    row_identity(row, &pair, arm, phase, &auth, order)?;
                    ensure(
                        row["width"] == width
                            && row["steps"] == steps
                            && row["trial"] == trial
                            && row["case"] == *case,
                        "operation dimensions",
                    )?;
                    ensure(
                        row["status"] == "ok"
                            && row["error"].is_null()
                            && row["verification"] == "commit-ack-only-pending-receiver-reopen",
                        "failed/missing pending remote commit",
                    )?;
                    ensure(
                        row["cpu_scope"] == "sender-process"
                            && row["body_scope"]
                                == "actual-http-bodies-excluding-headers-tls-network",
                        "metric scope",
                    )?;
                    let prefix = if phase == "seed" {
                        "checkpoint"
                    } else {
                        "target"
                    };
                    ensure(
                        row["object"] == fixture[format!("{prefix}_object")]
                            && row["object_bytes"] == fixture[format!("{prefix}_bytes")]
                            && row["target_sha256"] == fixture[format!("{prefix}_sha256")],
                        "observed vs registered object identity",
                    )?;
                    for metric in METRICS {
                        metric_value(row, metric)?;
                    }
                    ensure(
                        u(row, "body_bytes")?
                            == u(row, "request_body_bytes")? + u(row, "response_body_bytes")?,
                        "bidirectional body arithmetic",
                    )?;
                    let protocol =
                        u(row, "protocol_request_bytes")? + u(row, "protocol_response_bytes")?;
                    ensure(
                        protocol < u(row, "body_bytes")? && u(row, "batches")? == 1,
                        "serialization accounting/batches",
                    )?;
                    if phase == "seed" || arm == "raw" {
                        ensure(
                            row["actual_mode"] == 0 && row["selection"].is_null(),
                            "raw/seed mode",
                        )?;
                        let n = u(row, "object_bytes")?;
                        ensure(
                            u(row, "body_bytes")? == 368 + n + 133 * n.div_ceil(61440),
                            "independent raw full-miss cost",
                        )?;
                    } else if arm == "zstd" {
                        ensure(
                            row["actual_mode"] == 1 && row["selection"].is_null(),
                            "forced zstd mode",
                        )?;
                    } else {
                        selections += 1;
                        let selection = &row["selection"];
                        let candidates = array(selection, "candidates")?;
                        ensure(
                            candidates.len() >= 2 && selection["exact_repeat_probe"] == false,
                            "candidate/probe policy",
                        )?;
                        let mut previous = None;
                        let mut winner = (u64::MAX, u64::MAX);
                        for candidate in candidates {
                            let mode = u(candidate, "mode")?;
                            let cost = u(candidate, "http_body_bytes")?;
                            ensure(
                                [0, 1, 3, 4, 5].contains(&mode)
                                    && previous.is_none_or(|m| mode > m),
                                "candidate eligibility/order",
                            )?;
                            ensure(
                                arm == "runtime" || mode != 5,
                                "legacy selector runtime candidate",
                            )?;
                            if cost < winner.1 {
                                winner = (mode, cost);
                            }
                            previous = Some(mode);
                        }
                        ensure(
                            selection["selected_mode"] == winner.0
                                && row["actual_mode"] == winner.0
                                && u(selection, "expected_http_body_bytes")? == winner.1
                                && u(row, "body_bytes")? == winner.1,
                            "strict-min winner/actual projected cost",
                        )?;
                    }
                    pair_arm.insert(phase.into(), row.clone());
                    let mut annotated = row.clone();
                    annotated["dataset"] = json!(data.label);
                    operations.push(annotated);
                }
                grouped.insert(arm.into(), Value::Object(pair_arm));
            }
            let raw_cost = u(&grouped["raw"]["target"], "body_bytes")?;
            let zstd_cost = u(&grouped["zstd"]["target"], "body_bytes")?;
            let conventional = &grouped["adaptive"]["target"]["selection"]["candidates"];
            let runtime = &grouped["runtime"]["target"]["selection"]["candidates"];
            for list in [conventional, runtime] {
                ensure(
                    list[0]["mode"] == 0
                        && list[0]["http_body_bytes"] == raw_cost
                        && list[1]["mode"] == 1
                        && list[1]["http_body_bytes"] == zstd_cost,
                    "forced-arm vs candidate cost",
                )?;
            }
            let conventional = conventional.as_array().ok_or("candidate array")?;
            let runtime = runtime.as_array().ok_or("candidate array")?;
            ensure(
                runtime.starts_with(conventional),
                "retained conventional candidates",
            )?;
            ensure(
                if *case == "unrelated" {
                    runtime.len() == conventional.len()
                        && grouped["runtime"]["target"]["actual_mode"] == 0
                } else {
                    runtime.len() == conventional.len() + 1
                        && runtime.last().ok_or("runtime candidate")?["mode"] == 5
                        && grouped["runtime"]["target"]["actual_mode"] == 5
                },
                "registered favorable/control candidate outcomes",
            )?;
            pairs.push(json!({"dataset":data.label,"case":case,"pair":pair,"trial":trial,"target_bytes":expected_len,"arms":grouped}));
        }
    }
    ensure(
        json!(scopes) == p["allowed_scopes"],
        "independent scope derivation/registration order",
    )?;
    ensure(
        scopes.iter().collect::<BTreeSet<_>>().len() == 36,
        "unique scopes",
    )?;
    let end = next(&data.rows, &mut at, "complete")?;
    ensure(
        end["status"] == "ok"
            && end["operations"] == 72
            && end["remote_verification_pending"] == true,
        "completion semantics",
    )?;
    ensure(
        at == data.rows.len()
            && operations.len() == 72
            && generation.len() == 9
            && selections == 18,
        "no dropped/extra rows or operations",
    )?;
    Ok(Validated {
        operations,
        generation,
        pairs,
        selection_rows: selections,
    })
}

fn summary(mut values: Vec<f64>) -> Value {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    let median = if n % 2 == 0 {
        (values[n / 2 - 1] + values[n / 2]) / 2.
    } else {
        values[n / 2]
    };
    json!({"n":n,"min":values[0],"median":median,"p95_nearest_rank":values[(95*n).div_ceil(100)-1],"max":values[n-1],"sum":values.iter().sum::<f64>()})
}
fn metric_value(row: &Value, metric: &str) -> R<u64> {
    if metric == "cpu_subtotal_us" {
        u(row, "operation_cpu_us")?
            .checked_add(u(row, "receiver_cpu_us")?)
            .ok_or("CPU subtotal overflow".into())
    } else {
        u(row, metric)
    }
}
fn metrics(rows: &[Value]) -> Value {
    let mut map = serde_json::Map::new();
    for metric in METRICS {
        map.insert(
            metric.into(),
            summary(
                rows.iter()
                    .map(|r| metric_value(r, metric).expect("validated") as f64)
                    .collect(),
            ),
        );
    }
    Value::Object(map)
}
fn sum_rows(a: &Value, b: &Value) -> Value {
    let mut m = serde_json::Map::new();
    for k in METRICS {
        m.insert(
            k.into(),
            json!(metric_value(a, k).unwrap() + metric_value(b, k).unwrap()),
        );
    }
    Value::Object(m)
}
fn summarize(all: &[Validated]) -> Value {
    let mut groups = BTreeMap::<(String, String, String, String), Vec<Value>>::new();
    let mut generation = BTreeMap::<(String, String), Vec<Value>>::new();
    let mut comparisons = BTreeMap::<(String, String, String, String), Vec<(Value, Value)>>::new();
    for d in all {
        for row in &d.operations {
            groups
                .entry((
                    row["dataset"].as_str().unwrap().into(),
                    row["case"].as_str().unwrap().into(),
                    row["arm"].as_str().unwrap().into(),
                    row["phase"].as_str().unwrap().into(),
                ))
                .or_default()
                .push(row.clone());
        }
        for row in &d.generation {
            generation
                .entry((
                    row["dataset"].as_str().unwrap().into(),
                    row["fixture"]["case"].as_str().unwrap().into(),
                ))
                .or_default()
                .push(row.clone());
        }
        for pair in &d.pairs {
            let label = pair["dataset"].as_str().unwrap();
            let case = pair["case"].as_str().unwrap();
            let runtime = &pair["arms"]["runtime"];
            for arm in ARMS {
                let row = sum_rows(&pair["arms"][arm]["seed"], &pair["arms"][arm]["target"]);
                groups
                    .entry((
                        label.into(),
                        case.into(),
                        arm.into(),
                        "seed_plus_target".into(),
                    ))
                    .or_default()
                    .push(row);
            }
            for baseline in ["raw", "zstd", "adaptive"] {
                let other = &pair["arms"][baseline];
                for scenario in [
                    "warm_target_only",
                    "equal_setup_seed_plus_target",
                    "practical_cold_start",
                ] {
                    if scenario == "practical_cold_start" && baseline == "adaptive" {
                        continue;
                    }
                    let candidate = if scenario == "warm_target_only" {
                        runtime["target"].clone()
                    } else {
                        sum_rows(&runtime["seed"], &runtime["target"])
                    };
                    let control = if scenario == "equal_setup_seed_plus_target" {
                        sum_rows(&other["seed"], &other["target"])
                    } else {
                        other["target"].clone()
                    };
                    comparisons
                        .entry((label.into(), case.into(), scenario.into(), baseline.into()))
                        .or_default()
                        .push((candidate, control));
                }
            }
        }
    }
    let cells:Vec<_>=groups.into_iter().map(|((dataset,case,arm,phase),rows)|json!({"dataset":dataset,"case":case,"arm":arm,"phase":phase,"metrics":metrics(&rows),"actual_mode_counts":rows.iter().filter_map(|r|r["actual_mode"].as_u64()).fold(BTreeMap::<u64,usize>::new(),|mut m,v|{*m.entry(v).or_default()+=1;m})})).collect();
    let comps:Vec<_>=comparisons.into_iter().map(|((dataset,case,scenario,baseline),pairs)|{
        let values=|key:&str,invert:bool|pairs.iter().map(|(r,b)|{let rv=metric_value(r,key).unwrap() as f64;let bv=metric_value(b,key).unwrap() as f64;if invert {bv/rv}else{rv/bv}}).collect::<Vec<_>>();
        json!({"dataset":dataset,"case":case,"scenario":scenario,"baseline":baseline,"pairs":pairs.len(),
          "runtime":metrics(&pairs.iter().map(|p|p.0.clone()).collect::<Vec<_>>()),"baseline_metrics":metrics(&pairs.iter().map(|p|p.1.clone()).collect::<Vec<_>>()),
          "paired_body_savings_percent":summary(values("body_bytes",false).into_iter().map(|r|100.*(1.-r)).collect()),
          "paired_sender_cpu_inflation_runtime_over_baseline":summary(values("operation_cpu_us",false)),
          "paired_cpu_subtotal_inflation_runtime_over_baseline":summary(values("cpu_subtotal_us",false)),
          "paired_wall_speedup_baseline_over_runtime":summary(values("operation_wall_us",true)),
          "cpu_subtotal_wins":pairs.iter().filter(|(r,b)|metric_value(r,"cpu_subtotal_us").unwrap()<metric_value(b,"cpu_subtotal_us").unwrap()).count(),
          "wall_wins":pairs.iter().filter(|(r,b)|r["operation_wall_us"].as_u64()<b["operation_wall_us"].as_u64()).count()})
    }).collect();
    let gens:Vec<_>=generation.into_iter().map(|((dataset,case),rows)|json!({"dataset":dataset,"case":case,"scope":"separate workload creation once per paired input; never assigned as unavoidable raw/zstd file-transfer cost","cpu_us":summary(rows.iter().map(|r|r["generation_cpu_us"].as_u64().unwrap() as f64).collect()),"wall_us":summary(rows.iter().map(|r|r["generation_wall_us"].as_u64().unwrap() as f64).collect())})).collect();
    let total_bytes: u64 = all
        .iter()
        .flat_map(|d| &d.operations)
        .map(|r| r["object_bytes"].as_u64().unwrap())
        .sum();
    json!({"schema":"spiralcore.independent.shared-runtime-wan.v1","source_revision":SOURCE,"executable_sha256":EXE,"validation":"PASS","operations":all.iter().map(|d|d.operations.len()).sum::<usize>(),"pairs":9,"target_operations":36,"seed_operations":36,"joined_exact_reopen_receipts":72,"object_bytes_compared_by_measured_harness":total_bytes,"selected_cost_equalities":all.iter().map(|d|d.selection_rows).sum::<usize>(),"cleanup_events":0,"cells":cells,"comparisons":comps,"generation":gens,
"scope_notes":["Real remote HTTP transfers through a temporary HTTPS relay; body bytes exclude HTTP/TLS/IP/TCP overhead, retransmissions and relay traffic.","Operation CPU is sender process only; cpu_subtotal_us adds reported receiver-handler CPU per observation, excluding HTTP-server/network/TLS/relay work.","Exchange wall includes serialization, request waiting, handler and durability; not pure byte send time and not additive to operation wall.","Remote verification_us is client post-ack identity work; exact bytes were checked separately after receiver stop and reopen.","Nearest-rank p95 for three observations is the observed maximum, not a qualified population-tail estimate.","Nine paired inputs with deterministic rotating arm order; pilot dose only, not randomized 30-trial WAN acceptance.","Conventional adaptive uses the checkpoint base, not a previously transferred full trace.","Independent analysis joins exact-read receipts but does not itself repeat physical transfers or receiver reads.","Generated/edited data are model-derived; unrelated control and unsuccessful prediction CPU remain included."]})
}

fn mutation_checks(data: &Dataset) -> R<Value> {
    validate(data)?;
    let mut results = Vec::new();
    let op = data
        .rows
        .iter()
        .position(|r| r["event"] == "operation")
        .ok_or("no operation")?;
    let target = data
        .rows
        .iter()
        .position(|r| r["event"] == "operation" && r["phase"] == "target" && r["arm"] == "runtime")
        .ok_or("no target")?;
    for name in [
        "missing_row",
        "duplicate_row",
        "failed_operation",
        "false_reopen",
        "wrong_source",
        "wrong_executable",
        "wrong_plan_hash",
        "wrong_scope",
        "wrong_fixture",
        "wrong_mode",
        "wrong_cost",
        "wrong_order",
        "wrong_metric_scope",
        "wrong_completion",
        "extra_event",
        "missing_control_case",
    ] {
        let mut changed = data.clone();
        match name {
            "missing_row" => {
                changed.rows.remove(op);
            }
            "duplicate_row" => {
                changed.rows.insert(op, changed.rows[op].clone());
            }
            "failed_operation" => changed.rows[op]["status"] = json!("error"),
            "false_reopen" => changed.rows[op]["verification"] = json!("commit-ack-only"),
            "wrong_source" => {
                changed.rows[0]["source"] = json!("0000000000000000000000000000000000000000")
            }
            "wrong_executable" => changed.rows[0]["executable_sha256"] = json!("00".repeat(32)),
            "wrong_plan_hash" => changed.rows[0]["plan_sha256"] = json!("00".repeat(32)),
            "wrong_scope" => changed.rows[op]["scope"] = json!("00".repeat(32)),
            "wrong_fixture" => changed.rows[op]["target_sha256"] = json!("00".repeat(32)),
            "wrong_mode" => changed.rows[target]["actual_mode"] = json!(0),
            "wrong_cost" => {
                changed.rows[target]["selection"]["expected_http_body_bytes"] = json!(0)
            }
            "wrong_order" => changed.rows[op]["order"] = json!(99),
            "wrong_metric_scope" => changed.rows[op]["cpu_scope"] = json!("sender-only"),
            "wrong_completion" => changed.rows.last_mut().unwrap()["operations"] = json!(71),
            "extra_event" => changed.rows.push(json!({"event":"complete"})),
            "missing_control_case" => {
                changed.plan["config"]["cases"] = json!(["generated", "edited"]);
            }
            _ => unreachable!(),
        }
        match validate(&changed) {
            Ok(_) => return Err(format!("mutation escaped:{name}")),
            Err(reason) => results.push(json!({"mutation":name,"rejected":true,"reason":reason})),
        };
    }
    Ok(
        json!({"clean_input_passes":validate(data).is_ok(),"negative_cases":results,"passed":results.len()}),
    )
}

fn join_receiver(data: &Dataset, checked: &Validated, rows: &[Value]) -> R<Value> {
    ensure(rows.len() == 74, "receiver row count")?;
    let h = &rows[0];
    ensure(
        h["event"] == "run"
            && h["command"] == "verify"
            && h["source"] == SOURCE
            && h["config"] == data.plan["config"]
            && h["plan_sha256"] == data.plan_sha
            && h["source_files"] == data.plan["source_files"]
            && h["architecture"] == "aarch64"
            && h["os"] == "macos"
            && h["executable_sha256"]
                == "49c0aa577105718fd4a7019ee719d1815e2b1f16e6d27e909632ecb228386d93",
        "receiver run/source/plan/executable binding",
    )?;
    let mut expected = BTreeMap::new();
    for op in &checked.operations {
        let key = (s(op, "scope")?.to_owned(), s(op, "phase")?.to_owned());
        ensure(
            expected.insert(key, op).is_none(),
            "duplicate expected operation",
        )?;
    }
    let mut joins = Vec::new();
    let mut seen = BTreeSet::new();
    for row in &rows[1..73] {
        ensure(
            row["event"] == "receiver_verification"
                && row["status"] == "ok"
                && row["error"].is_null()
                && row["verification"] == "independent-exact-read-after-server-stop",
            "receiver failed/missing exact read",
        )?;
        let key = (s(row, "scope")?.to_owned(), s(row, "phase")?.to_owned());
        ensure(seen.insert(key.clone()), "duplicate receiver read")?;
        let op = expected
            .get(&key)
            .ok_or("receiver read has no transferred operation")?;
        ensure(
            row["expected_object"] == op["object"]
                && row["expected_bytes"] == op["object_bytes"]
                && row["actual_bytes"] == op["object_bytes"]
                && row["expected_sha256"] == op["target_sha256"]
                && row["actual_sha256"] == op["target_sha256"],
            "receiver exact read/transfer identity mismatch",
        )?;
        joins.push(json!({"pair":op["pair"],"case":op["case"],"arm":op["arm"],"phase":op["phase"],"scope":op["scope"],"object":op["object"],"bytes":op["object_bytes"],"sha256":op["target_sha256"],"exact_reopen":true}));
    }
    ensure(
        seen.len() == expected.len(),
        "receiver verification missing operations",
    )?;
    ensure(
        rows[73]["event"] == "verification_complete"
            && rows[73]["checked"] == 72
            && rows[73]["status"] == "ok",
        "receiver terminal completion",
    )?;
    Ok(
        json!({"status":"PASS","joined":joins.len(),"source_revision":SOURCE,"plan_sha256":data.plan_sha,"comparisons":joins}),
    )
}
fn receiver_mutations(data: &Dataset, checked: &Validated, rows: &[Value]) -> R<Value> {
    join_receiver(data, checked, rows)?;
    let mut results = Vec::new();
    for name in [
        "receiver_missing_read",
        "receiver_duplicate_read",
        "receiver_false_exact",
        "receiver_wrong_scope",
        "receiver_wrong_object",
        "receiver_wrong_length",
        "receiver_wrong_hash",
        "receiver_wrong_source",
        "receiver_wrong_executable",
        "receiver_wrong_plan",
        "receiver_failed_completion",
    ] {
        let mut changed = rows.to_vec();
        match name {
            "receiver_missing_read" => {
                changed.remove(1);
            }
            "receiver_duplicate_read" => changed[2] = changed[1].clone(),
            "receiver_false_exact" => changed[1]["verification"] = json!("ack-only"),
            "receiver_wrong_scope" => changed[1]["scope"] = json!("00".repeat(32)),
            "receiver_wrong_object" => changed[1]["expected_object"] = json!("00".repeat(32)),
            "receiver_wrong_length" => changed[1]["actual_bytes"] = json!(0),
            "receiver_wrong_hash" => changed[1]["actual_sha256"] = json!("00".repeat(32)),
            "receiver_wrong_source" => changed[0]["source"] = json!("00".repeat(20)),
            "receiver_wrong_executable" => changed[0]["executable_sha256"] = json!("00".repeat(32)),
            "receiver_wrong_plan" => changed[0]["plan_sha256"] = json!("00".repeat(32)),
            "receiver_failed_completion" => changed[73]["status"] = json!("error"),
            _ => unreachable!(),
        }
        match join_receiver(data, checked, &changed) {
            Ok(_) => return Err(format!("receiver mutation escaped:{name}")),
            Err(reason) => results.push(json!({"mutation":name,"rejected":true,"reason":reason})),
        }
    }
    Ok(json!({"clean_input_passes":true,"passed":results.len(),"negative_cases":results}))
}
fn git_blob(root: &Path, revision: &str, path: &str) -> R<Vec<u8>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["show", &format!("{revision}:{path}")])
        .output()
        .map_err(|e| e.to_string())?;
    ensure(output.status.success(), format!("git show failed:{path}"))?;
    Ok(output.stdout)
}
fn json_file(path: &Path) -> R<Value> {
    serde_json::from_slice(&read(path)?).map_err(|e| e.to_string())
}
fn lines(bytes: &[u8]) -> R<Vec<Value>> {
    std::str::from_utf8(bytes)
        .map_err(|e| e.to_string())?
        .lines()
        .map(|line| serde_json::from_str(line).map_err(|e| e.to_string()))
        .collect()
}
fn custody(root: &Path, packet: &Path, data: &Dataset, receiver_bytes: &[u8]) -> R<Value> {
    let remote = packet.join("remote");
    let source_manifest = read(&remote.join("source-files.sha256"))?;
    let mut sources = Vec::new();
    for line in std::str::from_utf8(&source_manifest)
        .map_err(|e| e.to_string())?
        .lines()
    {
        let (expected, path) = line.split_once("  ").ok_or("source manifest syntax")?;
        ensure(
            sha(&git_blob(root, SOURCE, path)?) == expected,
            format!("source mismatch:{path}"),
        )?;
        sources.push(json!({"path":path,"sha256":expected}));
    }
    let source_map = [
        (
            "harness",
            "crates/spiralcore-transfer/examples/shared_runtime.rs",
        ),
        (
            "runtime",
            "crates/spiralcore-transfer/src/shared_runtime.rs",
        ),
        (
            "runtime_transfer",
            "crates/spiralcore-transfer/src/shared_transfer.rs",
        ),
        ("client", "crates/spiralcore-transfer/src/client.rs"),
        ("selection", "crates/spiralcore-transfer/src/selection.rs"),
        ("endpoint", "crates/spiralcore-transfer/src/endpoint.rs"),
        ("protocol", "crates/spiralcore-transfer/src/protocol.rs"),
        ("wan", "crates/spiralcore-transfer/src/wan.rs"),
        ("lockfile", "crates/spiralcore-transfer/Cargo.lock"),
    ];
    for (key, path) in source_map {
        ensure(
            data.plan["source_files"][key] == sha(&git_blob(root, SOURCE, path)?),
            format!("plan source:{key}"),
        )?;
        ensure(
            git_blob(root, SOURCE, path)?
                == git_blob(root, "b6627deb7fb8c80a46c0e89d5e074afaf50f37f0", path)?,
            format!("native sender/receiver source parity:{path}"),
        )?;
    }
    let diff = Command::new("git")
        .current_dir(root)
        .args([
            "diff",
            "--exit-code",
            "b6627deb7fb8c80a46c0e89d5e074afaf50f37f0",
            SOURCE,
            "--",
            "crates/spiralcore-transfer",
            "docs/network/evidence/2026-10-30-shared-runtime/reference",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    ensure(
        diff.status.success() && diff.stdout.is_empty(),
        "production/harness/Cargo/reference parity",
    )?;
    ensure(
        read(&packet.join("receiver-plan.json"))? == read(&remote.join("plan.json"))?,
        "Linux plan vs native receiver plan byte parity",
    )?;
    let api = json_file(&packet.join("artifacts-api.json"))?;
    let arts = array(&api, "artifacts")?;
    ensure(arts.len() == 1, "one expected artifact")?;
    let art = &arts[0];
    ensure(
        art["id"] == 12982310131u64
            && art["workflow_run"]["id"] == 41062532491u64
            && art["workflow_run"]["head_sha"] == SOURCE,
        "artifact identity",
    )?;
    let local = Path::new("/Users/casey.allard/spiralcore/local/shared-state-20261030");
    let zip = read(&local.join("wan-artifact-41062532491.zip"))?;
    ensure(
        art["digest"] == format!("sha256:{}", sha(&zip))
            && u(art, "size_in_bytes")? == zip.len() as u64,
        "artifact API/download digest",
    )?;
    let extracted = local.join("wan-artifact-41062532491");
    let manifest = read(&remote.join("artifact-sha256.txt"))?;
    let mut members = Vec::new();
    for line in std::str::from_utf8(&manifest)
        .map_err(|e| e.to_string())?
        .lines()
    {
        let (expected, path) = line.split_once("  ").ok_or("artifact member syntax")?;
        let content = read(&extracted.join(path))?;
        ensure(
            sha(&content) == expected,
            format!("artifact member mismatch:{path}"),
        )?;
        let name = Path::new(path).file_name().ok_or("artifact basename")?;
        if name != "shared_runtime-linux-x86_64" {
            ensure(
                read(&remote.join(name))? == content,
                format!("retained evidence copy mismatch:{path}"),
            )?;
        }
        members.push(json!({"path":path,"sha256":expected}));
    }
    ensure(members.len() == 29, "artifact member count")?;
    let native_harness = local.join("frozen-b6627de/shared_runtime");
    let native_server = local.join("frozen-b6627de/spiralcore-wan");
    let linux = extracted.join("runtime-pilot/shared_runtime-linux-x86_64");
    let native_harness_sha = sha(&read(&native_harness)?);
    let native_server_sha = sha(&read(&native_server)?);
    let linux_sha = sha(&read(&linux)?);
    ensure(
        native_harness_sha == "49c0aa577105718fd4a7019ee719d1815e2b1f16e6d27e909632ecb228386d93"
            && native_server_sha
                == "7b79c143c6e010804ed9a6d990c3333485344607e150d7200965a51f5eba1162"
            && linux_sha == EXE,
        "actual native/Linux executable identity",
    )?;
    ensure(
        data.rows[0]["receiver_executable_sha256_operator_supplied"] == native_server_sha,
        "receiver executable header binding",
    )?;
    let registration = json_file(&remote.join("registration.json"))?;
    ensure(
        registration["source"] == SOURCE
            && registration["public_run_id"] == data.plan["config"]["run_id"]
            && registration["trials"] == 3
            && registration["width"] == 4096
            && registration["steps"] == 4096
            && registration["cases"] == json!(CASES)
            && registration["methods"] == json!(ARMS)
            && registration["scopes"] == 36
            && registration["seed_operations"] == 36
            && registration["target_operations"] == 36
            && registration["github_run_id"] == "41062532491"
            && registration["receiver_executable_sha256_operator_supplied"] == native_server_sha,
        "pilot registration",
    )?;
    let run = json_file(&packet.join("run-api.json"))?;
    ensure(
        run["id"] == 41062532491u64
            && run["head_sha"] == SOURCE
            && run["conclusion"] == "success"
            && run["status"] == "completed",
        "GitHub run completed/source",
    )?;
    let jobs = json_file(&packet.join("jobs.json"))?;
    ensure(array(&jobs, "jobs")?.len() == 1, "one pilot job")?;
    for job in array(&jobs, "jobs")? {
        ensure(
            job["conclusion"] == "success" && job["status"] == "completed",
            "job complete",
        )?;
        for step in array(job, "steps")? {
            ensure(step["conclusion"] == "success", "job step not successful")?;
        }
    }
    ensure(
        std::str::from_utf8(&read(&packet.join("verify-command.exit"))?)
            .map_err(|e| e.to_string())?
            .trim()
            == "0",
        "receiver verify exit",
    )?;
    Ok(
        json!({"source_revision":SOURCE,"source_manifest_sha256":sha(&source_manifest),"source_files":sources,"source_parity_b662_to_35114da":"entire transfer crate and reference packet unchanged","plan_sha256":data.plan_sha,"native_plan_byte_identical":true,"observations_sha256":data.raw_sha,"receiver_verification_sha256":sha(receiver_bytes),"sender_raw_rows":data.rows.len(),"receiver_rows":74,"artifact_id":12982310131u64,"github_run_id":41062532491u64,"artifact_zip_sha256":sha(&zip),"artifact_zip_bytes":zip.len(),"artifact_member_count":members.len(),"artifact_members":members,"actual_executables":[{"path":linux,"sha256":linux_sha},{"path":native_harness,"sha256":native_harness_sha},{"path":native_server,"sha256":native_server_sha}],"receiver_verify_exit":0}),
    )
}
fn run() -> R<()> {
    let args = std::env::args().collect::<Vec<_>>();
    ensure(
        args.len() == 3,
        "usage: analyze REPOSITORY_ROOT OUTPUT_DIRECTORY",
    )?;
    let root = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    let packet = root.join("docs/network/evidence/2026-10-30-shared-runtime/wan/attempt2");
    let plan = read(&packet.join("remote/plan.json"))?;
    let raw = read(&packet.join("remote/observations.jsonl"))?;
    let receiver_bytes = read(&packet.join("receiver-verification.jsonl"))?;
    let receiver = lines(&receiver_bytes)?;
    let data = Dataset {
        label: "wan".into(),
        plan: serde_json::from_slice(&plan).map_err(|e| e.to_string())?,
        plan_sha: sha(&plan),
        rows: lines(&raw)?,
        raw_sha: sha(&raw),
    };
    let checked = validate(&data)?;
    let remote_qualification = mutation_checks(&data)?;
    let receiver_qualification = receiver_mutations(&data, &checked, &receiver)?;
    let joins = join_receiver(&data, &checked, &receiver)?;
    let custody = custody(&root, &packet, &data, &receiver_bytes)?;
    let results = summarize(&[checked]);
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    for (name, value) in [
        (
            "negative-checks.json",
            json!({"sender":remote_qualification,"receiver":receiver_qualification}),
        ),
        ("receiver-joins.json", joins),
        ("custody.json", custody),
        ("results.json", results),
    ] {
        fs::write(
            out.join(name),
            serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    println!("PASS: 72 operations, 9 paired inputs, 36 targets + 36 seeds, 18 selected-cost equalities, 72 stopped-receiver exact reads; 27 negative variants rejected");
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("VALIDATION FAILED: {error}");
        std::process::exit(1);
    }
}
