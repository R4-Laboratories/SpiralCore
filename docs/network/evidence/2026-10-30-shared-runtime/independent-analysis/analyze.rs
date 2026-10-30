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
const SOURCE: &str = "b6627deb7fb8c80a46c0e89d5e074afaf50f37f0";
const EXE: &str = "49c0aa577105718fd4a7019ee719d1815e2b1f16e6d27e909632ecb228386d93";
const ARMS: [&str; 4] = ["raw", "zstd", "adaptive", "runtime"];
const CASES: [&str; 3] = ["generated", "edited", "unrelated"];
const METRICS: [&str; 12] = [
    "body_bytes",
    "operation_cpu_us",
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
        "small" => (48u64, 243u64),
        "large" => (4096, 4096),
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
            && cfg["trials"] == 10
            && cfg["cases"] == json!(CASES)
            && cfg["transport"] == "local-mechanism",
        "registered plan dose/source",
    )?;
    ensure(
        p["methods"] == json!(ARMS)
            && p["expected_operations"] == 240
            && p["seed_mode"] == "raw"
            && p["warm_probe"] == false
            && p["cold_graph_warming"] == false,
        "registered method/policy",
    )?;
    let fixtures = array(p, "fixtures")?;
    ensure(fixtures.len() == 30, "fixture count")?;
    let run_id = s(cfg, "run_id")?;
    ensure(
        run_id == format!("shared-runtime-{}-01", data.label),
        "run identity",
    )?;
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
        header["architecture"] == "aarch64"
            && header["os"] == "macos"
            && header["command"] == "local",
        "local platform",
    )?;
    let mut operations = Vec::new();
    let mut generation = Vec::new();
    let mut pairs = Vec::new();
    let mut scopes = Vec::new();
    let mut selections = 0;
    for (case_index, case) in CASES.iter().enumerate() {
        for trial in 0..10u64 {
            let fixture = &fixtures[case_index * 10 + trial as usize];
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
                            && row["verification"] == "exact-after-reopen",
                        "failed/missing exact reopen",
                    )?;
                    ensure(
                        row["cpu_scope"] == "single-process-combined-sender-receiver"
                            && row["body_scope"] == "serialized-http-body-equivalent",
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
                        u(row, metric)?;
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
                let clean = next(&data.rows, &mut at, "cleanup")?;
                ensure(
                    clean["scope"] == auth
                        && clean["status"] == "owned-arm-cache-removed-after-exact-verification",
                    "owned cleanup",
                )?;
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
        scopes.iter().collect::<BTreeSet<_>>().len() == 120,
        "unique scopes",
    )?;
    let end = next(&data.rows, &mut at, "complete")?;
    ensure(
        end["status"] == "ok"
            && end["operations"] == 240
            && end["remote_verification_pending"] == false,
        "completion semantics",
    )?;
    ensure(
        at == data.rows.len()
            && operations.len() == 240
            && generation.len() == 30
            && selections == 60,
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
fn metrics(rows: &[Value]) -> Value {
    let mut map = serde_json::Map::new();
    for metric in METRICS {
        map.insert(
            metric.into(),
            summary(
                rows.iter()
                    .map(|r| r[metric].as_u64().expect("validated") as f64)
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
            json!(a[k].as_u64().unwrap() + b[k].as_u64().unwrap()),
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
        let values=|key:&str,invert:bool|pairs.iter().map(|(r,b)|{let rv=r[key].as_u64().unwrap() as f64;let bv=b[key].as_u64().unwrap() as f64;if invert {bv/rv}else{rv/bv}}).collect::<Vec<_>>();
        json!({"dataset":dataset,"case":case,"scenario":scenario,"baseline":baseline,"pairs":pairs.len(),
          "runtime":metrics(&pairs.iter().map(|p|p.0.clone()).collect::<Vec<_>>()),"baseline_metrics":metrics(&pairs.iter().map(|p|p.1.clone()).collect::<Vec<_>>()),
          "paired_body_savings_percent":summary(values("body_bytes",false).into_iter().map(|r|100.*(1.-r)).collect()),
          "paired_cpu_inflation_runtime_over_baseline":summary(values("operation_cpu_us",false)),
          "paired_wall_speedup_baseline_over_runtime":summary(values("operation_wall_us",true)),
          "cpu_wins":pairs.iter().filter(|(r,b)|r["operation_cpu_us"].as_u64()<b["operation_cpu_us"].as_u64()).count(),
          "wall_wins":pairs.iter().filter(|(r,b)|r["operation_wall_us"].as_u64()<b["operation_wall_us"].as_u64()).count()})
    }).collect();
    let gens:Vec<_>=generation.into_iter().map(|((dataset,case),rows)|json!({"dataset":dataset,"case":case,"scope":"separate workload creation once per paired input; never assigned as unavoidable raw/zstd file-transfer cost","cpu_us":summary(rows.iter().map(|r|r["generation_cpu_us"].as_u64().unwrap() as f64).collect()),"wall_us":summary(rows.iter().map(|r|r["generation_wall_us"].as_u64().unwrap() as f64).collect())})).collect();
    let total_bytes: u64 = all
        .iter()
        .flat_map(|d| &d.operations)
        .map(|r| r["object_bytes"].as_u64().unwrap())
        .sum();
    json!({"schema":"spiralcore.independent.shared-runtime-local.v1","source_revision":SOURCE,"executable_sha256":EXE,"validation":"PASS","operations":all.iter().map(|d|d.operations.len()).sum::<usize>(),"pairs":60,"target_operations":240,"seed_operations":240,"exact_reopen_receipts":480,"object_bytes_compared_by_measured_harness":total_bytes,"selected_cost_equalities":all.iter().map(|d|d.selection_rows).sum::<usize>(),"cleanup_events":240,"cells":cells,"comparisons":comps,"generation":gens,
      "scope_notes":["Local durable endpoint transfers with serialized HTTP-body-equivalent bytes; no WAN, TLS, packet, or physical-link timing claim.","Operation CPU already includes sender+receiver in one process. Receiver/exchange are nested; never add them to operation totals.","Exchange wall includes serialization, handler, durability and waiting; not pure byte send time.","Verification/reopen CPU+wall separate from durable commit operation; both retained.","Median averages middle two observations; nearest-rank p95 for10 trials is observed maximum.","Deterministic rotating method order, not independent randomized network trials.","Conventional adaptive arm uses the small checkpoint as base; no previous full-trace cache baseline was measured.","Verifier checks complete raw receipts and pinned code. It does not repeat byte comparisons against caches that the measured harness already removed.","Favorable generated/edited traces are deliberately model-derived; unrelated controls and unsuccessful candidate compute retained."]})
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
        "missing_cleanup",
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
            "missing_cleanup" => {
                let i = changed
                    .rows
                    .iter()
                    .position(|r| r["event"] == "cleanup")
                    .unwrap();
                changed.rows.remove(i);
            }
            "wrong_completion" => changed.rows.last_mut().unwrap()["operations"] = json!(239),
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

fn custody(root: &Path, packet: &Path, datasets: &[Dataset]) -> R<Value> {
    let manifest = read(&packet.join("custody/source-files.sha256"))?;
    let text = String::from_utf8(manifest.clone()).map_err(|e| e.to_string())?;
    let mut source_count = 0;
    for line in text.lines() {
        let (expected, path) = line.split_once("  ").ok_or("source manifest syntax")?;
        let output = Command::new("git")
            .current_dir(root)
            .args(["show", &format!("{SOURCE}:{path}")])
            .output()
            .map_err(|e| e.to_string())?;
        ensure(
            output.status.success() && sha(&output.stdout) == expected,
            format!("source commit mismatch:{path}"),
        )?;
        source_count += 1;
    }
    let exec = read(&packet.join("custody/executables.sha256"))?;
    let text = String::from_utf8(exec.clone()).map_err(|e| e.to_string())?;
    let mut executables = Vec::new();
    for line in text.lines() {
        let (expected, path) = line.split_once("  ").ok_or("executable manifest syntax")?;
        let actual = sha(&read(Path::new(path))?);
        ensure(actual == expected, "frozen executable changed")?;
        executables.push(json!({"path":path,"sha256":actual}));
    }
    let mapping = [
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
    for data in datasets {
        for (key, path) in mapping {
            let output = Command::new("git")
                .current_dir(root)
                .args(["show", &format!("{SOURCE}:{path}")])
                .output()
                .map_err(|e| e.to_string())?;
            ensure(
                output.status.success() && data.plan["source_files"][key] == sha(&output.stdout),
                format!("plan source pin:{key}"),
            )?;
        }
    }
    Ok(
        json!({"source_revision":SOURCE,"source_manifest_sha256":sha(&manifest),"source_files_verified_against_commit":source_count,"executables_manifest_sha256":sha(&exec),"actual_frozen_executables":executables,"inputs":datasets.iter().map(|d|json!({"dataset":d.label,"plan_sha256":d.plan_sha,"raw_observations_sha256":d.raw_sha,"rows":d.rows.len()})).collect::<Vec<_>>() }),
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
    let packet = root.join("docs/network/evidence/2026-10-30-shared-runtime");
    let mut datasets = Vec::new();
    for label in ["small", "large"] {
        let plan = read(&packet.join(format!("local/{label}-plan.json")))?;
        let raw = read(&packet.join(format!("local/{label}-observations.jsonl")))?;
        let rows = String::from_utf8(raw.clone())
            .map_err(|e| e.to_string())?
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<Vec<Value>, _>>()
            .map_err(|e| e.to_string())?;
        datasets.push(Dataset {
            label: label.into(),
            plan: serde_json::from_slice(&plan).map_err(|e| e.to_string())?,
            plan_sha: sha(&plan),
            rows,
            raw_sha: sha(&raw),
        });
    }
    let qualified = mutation_checks(&datasets[0])?;
    let checks = custody(&root, &packet, &datasets)?;
    let validated = datasets.iter().map(validate).collect::<R<Vec<_>>>()?;
    let results = summarize(&validated);
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    for (name, value) in [
        ("negative-checks.json", qualified),
        ("custody.json", checks),
        ("results.json", results),
    ] {
        fs::write(
            out.join(name),
            serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    println!("PASS:480 operations,60 paired inputs,240targets+240seeds,120costequalities,240cleanup events;17negative checks rejected");
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("VALIDATION FAILED:{error}");
        std::process::exit(1);
    }
}
