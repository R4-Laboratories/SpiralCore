use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::Path,
};
fn bounded_text(reader: impl Read) -> Result<String> {
    let mut text = String::new();
    reader.take((8 << 20) + 1).read_to_string(&mut text)?;
    if text.len() > 8 << 20 {
        return Err("input too large".into());
    }
    Ok(text)
}
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn stats(values: &[u64]) -> Result<Value> {
    if values.is_empty() {
        return Err("empty distribution".into());
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let n = sorted.len();
    let sum = values
        .iter()
        .try_fold(0u64, |a, b| a.checked_add(*b))
        .ok_or("sum overflow")?;
    let median = if n.is_multiple_of(2) {
        sorted[n / 2 - 1] as f64 / 2.0 + sorted[n / 2] as f64 / 2.0
    } else {
        sorted[n / 2] as f64
    };
    Ok(
        json!({"n":n,"median":median,"p95":sorted[(95*n).div_ceil(100)-1],"min":sorted[0],"max":sorted[n-1],"sum":sum,"values":values}),
    )
}
fn paired(base: &[u64], candidate: &[u64]) -> Result<Value> {
    if base.is_empty()
        || base.len() != candidate.len()
        || base.contains(&0)
        || candidate.contains(&0)
    {
        return Err("invalid ratio input".into());
    }
    let ratios: Vec<f64> = base
        .iter()
        .zip(candidate)
        .map(|(a, b)| *a as f64 / *b as f64)
        .collect();
    Ok(
        json!({"n":base.len(),"geometric_mean":(ratios.iter().map(|v|v.ln()).sum::<f64>()/ratios.len() as f64).exp(),"candidate_faster":base.iter().zip(candidate).filter(|(a,b)|a>b).count(),"ties":base.iter().zip(candidate).filter(|(a,b)|a==b).count(),"candidate_slower":base.iter().zip(candidate).filter(|(a,b)|a<b).count(),"ratios":ratios}),
    )
}
fn number(v: &Value, p: &str) -> Result<u64> {
    v.pointer(p)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing integer {p}").into())
}
fn string<'a>(v: &'a Value, p: &str) -> Result<&'a str> {
    v.pointer(p)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string {p}").into())
}
const ARMS: [&str; 4] = ["raw_full", "zstd_full", "sparse", "zstd_prefix"];
const METRICS: [&str; 5] = [
    "wall_micros",
    "sender_cpu_micros",
    "receiver_cpu_micros",
    "combined_cpu_micros",
    "body_bytes",
];
fn analyze(rows: &[Value]) -> Result<Value> {
    let first = rows.first().ok_or("empty receipts")?;
    let broad = string(first, "/schema")? == "spiralcore.wan.row.v4";
    let suite = string(first, "/suite")?;
    let case = string(first, "/task/case")?;
    let warm = !broad
        || first
            .pointer("/task/seed_base")
            .and_then(Value::as_bool)
            .ok_or("missing seed policy")?;
    let arms_list = if broad { &ARMS[..] } else { &ARMS[..3] };
    let expected = 30 * arms_list.len() * (1 + usize::from(warm));
    if rows.len() != expected
        || (broad && (suite != format!("broad16-{case}") || warm == case.starts_with("cold_")))
    {
        return Err("receipt count or workload policy mismatch".into());
    }
    let run = string(&rows[0], "/run")?;
    let source = string(&rows[0], "/source_revision")?;
    let mut cells = BTreeMap::new();
    let mut objects = BTreeMap::new();
    let mut seeds = BTreeMap::new();
    let mut binaries = BTreeSet::new();
    for r in rows {
        let trial = number(r, "/task/trial")?;
        let arm = string(r, "/task/arm")?;
        let phase = string(r, "/phase")?;
        if string(r, "/schema")?
            != if broad {
                "spiralcore.wan.row.v4"
            } else {
                "spiralcore.wan.row.v3"
            }
            || string(r, "/suite")? != suite
            || (!broad && suite != "large16-localized")
            || (broad
                && (r.get("cold_graph_warming") != Some(&Value::Bool(false))
                    || r.pointer("/task/seed_base") != Some(&Value::Bool(warm))))
            || string(r, "/run")? != run
            || string(r, "/source_revision")? != source
            || number(r, "/plan_trials")? != 30
            || trial >= 30
            || number(r, "/shard")? != trial / if broad { 6 } else { 10 }
            || !arms_list.contains(&arm)
            || !(phase == "transfer" || (warm && phase == "seed"))
            || string(r, "/status")? != "published"
            || r.get("error") != Some(&Value::Null)
            || number(r, "/object_bytes")? > 16777216
            || number(r, "/task/size")? > 16777216
            || (!broad
                && (number(r, "/object_bytes")? != 16777216
                    || number(r, "/task/size")? != 16777216))
            || string(r, "/task/case")? != case
            || (!broad && case != "localized_32")
            || number(r, "/metrics/http_requests")? != 1
        {
            return Err("receipt contract mismatch".into());
        }
        let object = string(r, "/object")?;
        if objects
            .insert((trial, phase), object)
            .is_some_and(|old| old != object)
        {
            return Err("paired object mismatch".into());
        }
        let seed = number(r, "/task/fixture_seed")?;
        if seeds.insert(trial, seed).is_some_and(|old| old != seed) {
            return Err("paired seed mismatch".into());
        }
        binaries.insert((number(r, "/shard")?, string(r, "/executable_sha256")?));
        let sender = number(r, "/metrics/sender_cpu_micros")?;
        let receiver = number(r, "/metrics/receiver_cpu_micros")?;
        let values = [
            number(r, "/metrics/wall_micros")?,
            sender,
            receiver,
            sender.checked_add(receiver).ok_or("CPU overflow")?,
            number(r, "/metrics/request_body_bytes")?
                .checked_add(number(r, "/metrics/response_body_bytes")?)
                .ok_or("bytes overflow")?,
        ];
        if cells.insert((trial, arm, phase), values).is_some() {
            return Err("duplicate operation".into());
        }
    }
    if binaries.len() != if broad { 5 } else { 3 } {
        return Err("one executable per shard required".into());
    }
    let vector = |arm: &str, phase: &str, metric: usize| -> Result<Vec<u64>> {
        (0..30)
            .map(|trial| {
                let transfer = cells
                    .get(&(trial, arm, "transfer"))
                    .ok_or("missing transfer")?[metric];
                let seed = if warm {
                    cells.get(&(trial, arm, "seed")).ok_or("missing seed")?[metric]
                } else {
                    0
                };
                Ok(match phase {
                    "seed" => seed,
                    "transfer" => transfer,
                    _ => seed.checked_add(transfer).ok_or("lifecycle overflow")?,
                })
            })
            .collect()
    };
    let mut arms = json!({});
    let mut comparisons = json!({});
    let phases: &[&str] = if warm {
        &["seed", "transfer", "seed_plus_transfer"]
    } else {
        &["transfer"]
    };
    for &arm in arms_list {
        for &phase in phases {
            for (i, metric) in METRICS.iter().enumerate() {
                arms[arm][phase][metric] = stats(&vector(arm, phase, i)?)?;
                if arm != "sparse" {
                    comparisons[arm][phase][metric] =
                        paired(&vector(arm, phase, i)?, &vector("sparse", phase, i)?)?;
                }
            }
        }
    }
    let raw_body = arms["raw_full"]["transfer"]["body_bytes"]["median"]
        .as_f64()
        .ok_or("median")?;
    let sparse_body = arms["sparse"]["transfer"]["body_bytes"]["median"]
        .as_f64()
        .ok_or("median")?;
    let ratio = comparisons["raw_full"]["transfer"]["wall_micros"]["geometric_mean"]
        .as_f64()
        .ok_or("ratio")?;
    let median_wall_ratio = arms["raw_full"]["transfer"]["wall_micros"]["median"]
        .as_f64()
        .ok_or("raw wall median")?
        / arms["sparse"]["transfer"]["wall_micros"]["median"]
            .as_f64()
            .ok_or("sparse wall median")?;
    let mut result = json!({"schema":"spiralcore.broad-wan.analysis.v1","rows":rows.len(),"run":run,"source":source,"suite":suite,"binaries":binaries,"arms":arms,"comparisons":comparisons,"statistics":"midpoint median; nearest-rank p95; ordered vectors in global trial order; all 30 pairs retained","boundaries":"CPU is sender plus receiver handler, not energy. Wall already includes compute. HTTP bodies are not total path traffic. Warm policy must match source/plan. All costs start with in-memory inputs; fixture generation excluded."});
    result["warm_gate"] = if warm {
        json!({"median_body_reduction":1.0-sparse_body/raw_body,"median_wall_ratio":median_wall_ratio,"paired_wall_ratio":ratio,"performance_thresholds_pass":sparse_body<=0.1*raw_body && median_wall_ratio>=2.0,"requires_separate_exact_reopen_verification":true})
    } else {
        Value::Null
    };
    if broad {
        result["cold_graph_warming"] = false.into();
    }
    if !warm {
        for &arm in arms_list {
            let a = &result["arms"][arm]["transfer"];
            let raw = &result["arms"]["raw_full"]["transfer"];
            result["cold_gates"][arm] = json!({"body_within_5pct":a["body_bytes"]["median"].as_f64().ok_or("median")? <= raw["body_bytes"]["median"].as_f64().ok_or("median")? * 1.05,
                "p95_within_10pct":a["wall_micros"]["p95"].as_f64().ok_or("p95")? <= raw["wall_micros"]["p95"].as_f64().ok_or("p95")? * 1.10});
        }
    }
    Ok(result)
}
fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let root = args.next().ok_or("campaign directory required")?;
    if args.next().is_some() {
        return Err("one directory argument required".into());
    }
    let mut rows = Vec::new();
    for shard in 0..5 {
        let path = Path::new(&root).join(format!("shard-{shard}/remote/rows.jsonl"));
        let bytes = bounded_text(fs::File::open(path)?)?;
        for line in bytes.lines() {
            rows.push(serde_json::from_str(line)?);
        }
    }
    println!("{}", serde_json::to_string_pretty(&analyze(&rows)?)?);
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("analysis: {e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_read_is_bounded_before_parsing() {
        assert_eq!(bounded_text(&b"abc"[..]).unwrap(), "abc");
        assert!(bounded_text(std::io::repeat(b'x').take((8 << 20) + 1)).is_err());
    }
    #[test]
    fn midpoint_median_nearest_rank_p95_and_ratios_are_explicit() {
        let s = stats(&[10, 2, 8, 4]).unwrap();
        assert_eq!(s["median"], 6.0);
        assert_eq!(s["p95"], 10);
        assert_eq!(s["sum"], 24);
        let p = paired(&[8, 4, 2], &[2, 4, 8]).unwrap();
        assert_eq!(p["geometric_mean"], 1.0);
        assert_eq!(p["candidate_faster"], 1);
        assert_eq!(p["ties"], 1);
        assert_eq!(p["candidate_slower"], 1);
        assert!(stats(&[]).is_err());
        assert!(paired(&[1], &[0]).is_err());
        assert!(paired(&[1], &[1, 2]).is_err());
    }
    fn rows() -> Vec<Value> {
        let mut rows = Vec::new();
        for trial in 0..30 {
            for arm in ["raw_full", "zstd_full", "sparse"] {
                for phase in ["seed", "transfer"] {
                    let cost = if arm == "sparse" && phase == "transfer" {
                        1
                    } else {
                        4
                    };
                    rows.push(json!({"schema":"spiralcore.wan.row.v3","suite":"large16-localized","run":"aa".repeat(32),"source_revision":"bb".repeat(20),"executable_sha256":"cc".repeat(32),"plan_trials":30,"shard":trial/10,"phase":phase,"status":"published","error":null,"object":format!("{trial}-{phase}"),"object_bytes":16777216,"task":{"trial":trial,"arm":arm,"case":"localized_32","size":16777216,"fixture_seed":trial+1},"metrics":{"wall_micros":cost*100,"sender_cpu_micros":cost*10,"receiver_cpu_micros":cost*20,"request_body_bytes":cost*1000,"response_body_bytes":100,"http_requests":1}}));
                }
            }
        }
        rows
    }
    #[test]
    fn broad_cold_and_warm_include_four_controls_and_all_five_shards() {
        for warm in [false, true] {
            let mut input = rows();
            let extra: Vec<_> = input
                .iter()
                .filter(|r| r["task"]["arm"] == "sparse")
                .cloned()
                .map(|mut r| {
                    r["task"]["arm"] = "zstd_prefix".into();
                    r
                })
                .collect();
            input.extend(extra);
            input.retain(|r| warm || r["phase"] == "transfer");
            for r in &mut input {
                r["schema"] = "spiralcore.wan.row.v4".into();
                r["suite"] = if warm {
                    "broad16-localized_32"
                } else {
                    "broad16-cold_random"
                }
                .into();
                r["cold_graph_warming"] = false.into();
                r["shard"] = (number(r, "/task/trial").unwrap() / 6).into();
                r["task"]["seed_base"] = warm.into();
                r["task"]["case"] = if warm { "localized_32" } else { "cold_random" }.into();
            }
            let a = analyze(&input).unwrap();
            assert_eq!(a["rows"], if warm { 240 } else { 120 });
            assert_eq!(a["cold_graph_warming"], false);
            assert_eq!(a["arms"]["zstd_prefix"]["transfer"]["wall_micros"]["n"], 30);
            assert_eq!(a["warm_gate"].is_object(), warm);
            assert!(analyze(&input[..input.len() - 1]).is_err());
            input[0]["cold_graph_warming"] = true.into();
            assert!(analyze(&input).is_err());
        }
    }
    #[test]
    fn full_campaign_is_paired_and_every_corrupt_or_missing_row_fails_closed() {
        let r = rows();
        let a = analyze(&r).unwrap();
        assert_eq!(a["rows"], 180);
        assert_eq!(
            a["arms"]["sparse"]["transfer"]["combined_cpu_micros"]["median"],
            30.0
        );
        assert_eq!(
            a["comparisons"]["raw_full"]["transfer"]["wall_micros"]["candidate_faster"],
            30
        );
        assert!(
            (a["comparisons"]["raw_full"]["transfer"]["wall_micros"]["geometric_mean"]
                .as_f64()
                .unwrap()
                - 4.0)
                .abs()
                < 1e-12
        );
        assert!(analyze(&r[..179]).is_err());
        for (pointer, bad) in [
            ("/shard", json!(2)),
            ("/status", json!("failed")),
            ("/source_revision", json!("other")),
            ("/object", json!("mismatch")),
            ("/task/fixture_seed", json!(999)),
            ("/metrics/http_requests", json!(2)),
        ] {
            let mut bad_rows = r.clone();
            *bad_rows[0].pointer_mut(pointer).unwrap() = bad;
            assert!(analyze(&bad_rows).is_err(), "{pointer}");
        }
        let mut duplicate = r.clone();
        duplicate[1] = duplicate[0].clone();
        assert!(analyze(&duplicate).is_err());
    }

    #[test]
    fn warm_time_gate_uses_medians_not_a_geometric_mean_inflated_by_outliers() {
        let mut input = rows();
        for r in &mut input {
            if r["phase"] != "transfer" {
                continue;
            }
            if r["task"]["arm"] == "raw_full" {
                let trial = number(r, "/task/trial").unwrap();
                r["metrics"]["wall_micros"] = if trial < 16 { 110u64 } else { 1_000_000 }.into();
                r["metrics"]["request_body_bytes"] = 20_000.into();
            }
        }
        let result = analyze(&input).unwrap();
        assert!(result["warm_gate"]["paired_wall_ratio"].as_f64().unwrap() > 2.0);
        assert_eq!(result["warm_gate"]["performance_thresholds_pass"], false);
        assert_eq!(result["warm_gate"]["median_wall_ratio"], 1.1);
    }
}
