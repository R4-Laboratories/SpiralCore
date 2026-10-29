use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "spiral-bench-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_spiralcore-bench"))
}
fn plan(t: &Temp, trials: &str, filename: &str) -> Value {
    suite_plan(t, trials, filename, None)
}
fn suite_plan(t: &Temp, trials: &str, filename: &str, suite: Option<&str>) -> Value {
    let out = t.0.join(filename);
    let mut cmd = command();
    cmd.args([
        "plan",
        "--trials",
        trials,
        "--run",
        &"ab".repeat(32),
        "--out",
    ])
    .arg(&out);
    if let Some(suite) = suite {
        cmd.args(["--suite", suite]);
    }
    let result = cmd.output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&fs::read(out).unwrap()).unwrap()
}
fn large_plan(t: &Temp, shard: u32) -> Value {
    let out = t.0.join(format!("large-{shard}.json"));
    let result = command()
        .args([
            "plan",
            "--suite",
            "large16-localized",
            "--trials",
            "30",
            "--shard",
        ])
        .arg(shard.to_string())
        .args(["--run", &"ab".repeat(32), "--out"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&fs::read(out).unwrap()).unwrap()
}
#[test]
fn broad16_plans_isolate_all_controls_and_bound_each_fresh_cache() {
    let t = Temp::new();
    let cases = [
        "localized_32",
        "scattered_32x32",
        "prefix_4096",
        "middle_4096",
        "delete_4096",
        "append_4096",
        "truncate_4096",
        "replace_1m",
        "structured_localized_32",
        "compressed_localized_32",
        "cold_random",
        "cold_structured",
    ];
    let mut scopes = std::collections::BTreeSet::new();
    for case in cases {
        let mut trials = std::collections::BTreeSet::new();
        for shard in 0..5 {
            let out = t.0.join(format!("broad-{case}-{shard}.json"));
            let result = command()
                .args([
                    "plan",
                    "--suite",
                    &format!("broad16-{case}"),
                    "--trials",
                    "30",
                    "--shard",
                    &shard.to_string(),
                    "--run",
                    &"ab".repeat(32),
                    "--out",
                ])
                .arg(&out)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            let plan: Value = serde_json::from_slice(&fs::read(&out).unwrap()).unwrap();
            assert_eq!(plan["schema"], "spiralcore.wan.plan.v4");
            assert_eq!(plan["cold_graph_warming"], false);
            assert_eq!(plan["shard"], shard);
            let tasks = plan["tasks"].as_array().unwrap();
            assert_eq!(tasks.len(), 24);
            assert!(plan["retained_payload_bytes"].as_u64().unwrap() + 48 * 4096 < 1 << 30);
            for pair in tasks.chunks_exact(4) {
                let trial = pair[0]["trial"].as_u64().unwrap();
                assert!((shard * 6..(shard + 1) * 6).contains(&trial));
                assert!(trials.insert(trial));
                let arms: std::collections::BTreeSet<_> =
                    pair.iter().map(|v| v["arm"].as_str().unwrap()).collect();
                assert_eq!(
                    arms,
                    ["raw_full", "zstd_full", "zstd_prefix", "sparse"].into()
                );
                for task in pair {
                    assert_eq!(task["case"], case);
                    assert_eq!(task["pair"], pair[0]["pair"]);
                    assert_eq!(task["fixture_seed"], pair[0]["fixture_seed"]);
                    assert_eq!(task["seed_base"], !case.starts_with("cold_"));
                    assert_eq!(task["warm_probe"], false);
                    assert!(scopes.insert(task["scope"].as_str().unwrap().to_owned()));
                }
            }
        }
        assert_eq!(trials, (0..30).collect());
    }
    assert_eq!(scopes.len(), cases.len() * 30 * 4);
}
#[test]
fn large16_shards_cover_thirty_pairs_with_balanced_orders_and_bounded_caches() {
    let t = Temp::new();
    let mut scopes = std::collections::BTreeSet::new();
    let mut orders = std::collections::BTreeMap::<Vec<String>, usize>::new();
    let mut trials = std::collections::BTreeSet::new();
    for shard in 0..3 {
        let p = large_plan(&t, shard);
        assert_eq!(p["schema"], "spiralcore.wan.plan.v3");
        assert_eq!(p["suite"], "large16-localized");
        assert_eq!(p["shard"], shard);
        assert_eq!(p["trials"], 30);
        assert_eq!(p["retained_payload_bytes"], 960 * 1024 * 1024u64);
        let tasks = p["tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), 30);
        assert!(p["retained_payload_bytes"].as_u64().unwrap() + 60 * 4096 < 1 << 30);
        for pair in tasks.chunks_exact(3) {
            let trial = pair[0]["trial"].as_u64().unwrap();
            assert!((u64::from(shard) * 10..u64::from(shard + 1) * 10).contains(&trial));
            assert!(trials.insert(trial));
            let mut order = Vec::new();
            for task in pair {
                assert_eq!(task["trial"], trial);
                assert_eq!(task["pair"], pair[0]["pair"]);
                assert_eq!(task["fixture_seed"], pair[0]["fixture_seed"]);
                assert_eq!(task["size"], 16 * 1024 * 1024);
                assert_eq!(task["case"], "localized_32");
                assert_eq!(task["seed_base"], true);
                assert_eq!(task["warm_probe"], false);
                assert_eq!(task["cache_state"], "base_present_target_absent");
                assert!(scopes.insert(task["scope"].as_str().unwrap().to_owned()));
                order.push(task["arm"].as_str().unwrap().to_owned());
            }
            assert_eq!(
                order
                    .iter()
                    .map(String::as_str)
                    .collect::<std::collections::BTreeSet<_>>(),
                std::collections::BTreeSet::from(["raw_full", "zstd_full", "sparse"])
            );
            *orders.entry(order).or_default() += 1;
        }
    }
    assert_eq!(trials, (0..30).collect());
    assert_eq!(scopes.len(), 90);
    assert_eq!(orders.len(), 6);
    assert!(orders.values().all(|count| *count == 5));
}
#[test]
fn large16_shard_arguments_are_strict_and_canonical() {
    let t = Temp::new();
    for args in [
        vec!["--suite", "large16-localized", "--trials", "30"],
        vec![
            "--suite",
            "large16-localized",
            "--trials",
            "30",
            "--shard",
            "3",
        ],
        vec![
            "--suite",
            "large16-localized",
            "--trials",
            "1",
            "--shard",
            "0",
        ],
        vec![
            "--suite",
            "efficient-warm",
            "--trials",
            "30",
            "--shard",
            "0",
        ],
        vec!["--trials", "30", "--shard", "0"],
    ] {
        assert!(!command()
            .arg("plan")
            .args(args)
            .args(["--run", &"ab".repeat(32), "--out"])
            .arg(t.0.join("invalid.json"))
            .output()
            .unwrap()
            .status
            .success());
        assert!(!t.0.join("invalid.json").exists());
    }
    let mut p = large_plan(&t, 0);
    p["tasks"][0]["fixture_seed"] = 7.into();
    fs::write(t.0.join("changed.json"), serde_json::to_vec(&p).unwrap()).unwrap();
    let result = command()
        .args(["run", "--plan"])
        .arg(t.0.join("changed.json"))
        .args(["--url", "http://127.0.0.1:9", "--out"])
        .arg(t.0.join("rows.jsonl"))
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("canonical generated plan"));
    assert!(!t.0.join("rows.jsonl").exists());
}
#[test]
fn efficiency_suites_preserve_v1_and_bound_each_thirty_trial_cache() {
    let t = Temp::new();
    let old = plan(&t, "30", "old.json");
    assert_eq!(old["schema"], "spiralcore.wan.plan.v1");
    assert!(old.get("suite").is_none());
    assert_eq!(old["tasks"].as_array().unwrap().len(), 575);
    // Captured from the preserved 354b616 baseline executable before this change.
    assert_eq!(
        format!(
            "{:x}",
            Sha256::digest(fs::read(t.0.join("old.json")).unwrap())
        ),
        "77e500ff33a7401b44a06a6f98911e6d244bdf6ce9f43ea7b9094a71100963ed"
    );
    let mut all_scopes = std::collections::BTreeSet::new();
    for (suite, expected_tasks, expected_mib, expected_rows) in [
        ("efficient-warm", 480, 780, 900),
        ("efficient-prefix", 120, 240, 240),
    ] {
        let p = suite_plan(&t, "30", &format!("{suite}.json"), Some(suite));
        assert_eq!(p["schema"], "spiralcore.wan.plan.v2");
        assert_eq!(p["suite"], suite);
        assert!(p.get("shard").is_none());
        // Captured from f0966eb before implementing the v3 profile.
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(fs::read(t.0.join(format!("{suite}.json"))).unwrap())
            ),
            if suite == "efficient-warm" {
                "1f12e93820b09b2d1e8dd840d74fe3900799e48ca575962b8728a1b7c055352d"
            } else {
                "4f5c1298f4f9d816c4319fb0389d8d6e30c1ad302c422d5e492ff788fb48d80b"
            }
        );
        assert_eq!(
            p,
            suite_plan(&t, "30", &format!("{suite}-again.json"), Some(suite))
        );
        let tasks = p["tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), expected_tasks);
        assert_eq!(
            tasks
                .iter()
                .map(|x| 1 + usize::from(x["seed_base"] == true))
                .sum::<usize>(),
            expected_rows
        );
        let payload = p["retained_payload_bytes"].as_u64().unwrap();
        assert!(
            payload >= expected_mib * 1024 * 1024 && payload < expected_mib * 1024 * 1024 + 1024
        );
        assert!(payload + expected_rows as u64 * 4096 < 1 << 30);
        for task in tasks {
            assert!(all_scopes.insert(task["scope"].as_str().unwrap().to_owned()));
            assert_eq!(
                task["warm_probe"],
                task["case"] == "repeat"
                    && !matches!(task["arm"].as_str().unwrap(), "raw_full" | "zstd_full")
            );
            assert!(!(task["case"] == "prefix_1" && task["arm"] == "prepared"));
        }
        let cases: std::collections::BTreeSet<_> =
            tasks.iter().map(|t| t["case"].as_str().unwrap()).collect();
        for case in cases {
            let mut positions: std::collections::BTreeMap<&str, Vec<usize>> =
                std::collections::BTreeMap::new();
            for trial in 0..30 {
                let pair: Vec<_> = tasks
                    .iter()
                    .filter(|x| x["case"] == case && x["trial"] == trial)
                    .collect();
                for (position, task) in pair.iter().enumerate() {
                    positions
                        .entry(task["arm"].as_str().unwrap())
                        .or_insert_with(|| vec![0; pair.len()])[position] += 1;
                }
            }
            for counts in positions.values() {
                assert!(counts.iter().max().unwrap() - counts.iter().min().unwrap() <= 1);
            }
        }
    }
}
#[test]
fn deterministic_plan_is_bounded_and_uses_isolated_scopes() {
    let t = Temp::new();
    let a = plan(&t, "30", "a.json");
    assert_eq!(a, plan(&t, "30", "b.json"));
    assert!(a["retained_payload_bytes"].as_u64().unwrap() < 1024 * 1024 * 1024);
    let scopes = a["allowed_scopes"].as_array().unwrap();
    let unique: std::collections::BTreeSet<_> =
        scopes.iter().map(|x| x.as_str().unwrap()).collect();
    assert_eq!(unique.len(), scopes.len());
    let tasks = a["tasks"].as_array().unwrap();
    assert!(tasks.iter().all(|x| x["warm_probe"]
        == (x["case"] == "repeat" && (x["arm"] == "raw_cas" || x["arm"] == "dag"))));
    assert!(tasks
        .iter()
        .all(|x| x["size"] != 16 * 1024 * 1024 || x["arm"] != "dag"));
    assert!(tasks
        .iter()
        .filter(|x| x["case"] == "repeat" && (x["arm"] == "raw_full" || x["arm"] == "zstd_full"))
        .all(|x| x["seed_base"] == false));
    assert!(tasks
        .iter()
        .any(|x| x["case"] == "repeat" && x["arm"] == "raw_cas" && x["seed_base"] == true));
}
#[test]
fn plan_rejects_unbounded_trials_and_does_not_replace_output() {
    let t = Temp::new();
    for trials in ["0", "31", "999999999"] {
        assert!(!command()
            .args([
                "plan",
                "--trials",
                trials,
                "--run",
                &"ab".repeat(32),
                "--out"
            ])
            .arg(t.0.join("invalid"))
            .status()
            .unwrap()
            .success());
    }
    plan(&t, "1", "owned.json");
    let before = fs::read(t.0.join("owned.json")).unwrap();
    assert!(!command()
        .args(["plan", "--trials", "2", "--run", &"ab".repeat(32), "--out"])
        .arg(t.0.join("owned.json"))
        .status()
        .unwrap()
        .success());
    assert_eq!(before, fs::read(t.0.join("owned.json")).unwrap());
}

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[test]
fn efficiency_campaigns_retain_prepared_state_and_reopen_exact_bytes() {
    for suite in ["efficient-warm", "efficient-prefix"] {
        let t = Temp::new();
        suite_plan(&t, "1", "plan.json", Some(suite));
        let token = "ef".repeat(32);
        let cache = t.0.join("cache");
        let mut server = Server(
            Command::new(env!("CARGO_BIN_EXE_spiralcore-wan"))
                .args(["serve", "--plan"])
                .arg(t.0.join("plan.json"))
                .arg("--cache")
                .arg(&cache)
                .args(["--seconds", "180"])
                .env("SPIRALCORE_WAN_TOKEN", &token)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let mut ready = String::new();
        BufReader::new(server.0.stdout.take().unwrap())
            .read_line(&mut ready)
            .unwrap();
        let url = format!(
            "http://{}",
            ready.trim().strip_prefix("READY addr=").unwrap()
        );
        let result = command()
            .args(["run", "--plan"])
            .arg(t.0.join("plan.json"))
            .args(["--url", &url, "--out"])
            .arg(t.0.join("rows.jsonl"))
            .env("SPIRALCORE_WAN_TOKEN", &token)
            .env("SPIRALCORE_SOURCE_REV", "integration-test")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        drop(server);
        let rows: Vec<Value> = fs::read_to_string(t.0.join("rows.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(rows.len(), if suite == "efficient-warm" { 30 } else { 8 });
        for row in &rows {
            assert_eq!(row["schema"], "spiralcore.wan.row.v2");
            assert_eq!(row["suite"], suite);
            assert_eq!(row["status"], "published");
            assert_eq!(row["metrics"]["preparation"]["promotion_performed"], false);
            if row["task"]["arm"] == "prepared" {
                let m = &row["metrics"];
                if row["phase"] == "seed" {
                    assert!(m["base_prepare_cpu_micros"].as_u64().unwrap() > 0);
                    assert!(
                        m["preparation"]["base_resident_charge_bytes"]
                            .as_u64()
                            .unwrap()
                            > 1048576
                    );
                    assert_eq!(m["preparation"]["retained_base_reused"], false);
                } else {
                    assert_eq!(m["base_prepare_cpu_micros"], 0);
                    assert_eq!(m["base_prepare_wall_micros"], 0);
                    assert_eq!(m["preparation"]["retained_base_reused"], true);
                    assert!(
                        m["preparation"]["target_prepare_cpu_micros"]
                            .as_u64()
                            .unwrap()
                            > 0
                    );
                    let dag = rows
                        .iter()
                        .find(|d| {
                            d["task"]["pair"] == row["task"]["pair"]
                                && d["task"]["arm"] == "dag"
                                && d["phase"] == "transfer"
                        })
                        .unwrap();
                    if row["task"]["case"] == "repeat" {
                        assert_eq!(m["preparation"]["selected_mode"], 0);
                        assert!(m["request_body_bytes"].as_u64().unwrap() < 1024);
                    } else {
                        assert_eq!(
                            m["request_body_bytes"],
                            dag["metrics"]["request_body_bytes"]
                        );
                        assert_eq!(
                            m["response_body_bytes"],
                            dag["metrics"]["response_body_bytes"]
                        );
                        assert_eq!(m["records_sent"], dag["metrics"]["records_sent"]);
                    }
                }
            }
            if row["task"]["arm"] == "sparse" && row["phase"] == "transfer" {
                assert_eq!(row["metrics"]["preparation"]["retained_base_reused"], true);
                if row["task"]["case"] == "prefix_1" {
                    assert_eq!(row["metrics"]["preparation"]["selected_mode"], 3);
                    assert!(row["metrics"]["request_body_bytes"].as_u64().unwrap() < 1024);
                }
            }
        }
        let verify = command()
            .args(["verify", "--plan"])
            .arg(t.0.join("plan.json"))
            .arg("--rows")
            .arg(t.0.join("rows.jsonl"))
            .arg("--cache")
            .arg(&cache)
            .output()
            .unwrap();
        assert!(
            verify.status.success(),
            "{}",
            String::from_utf8_lossy(&verify.stderr)
        );
        let verified: Value = serde_json::from_slice(&verify.stdout).unwrap();
        assert_eq!(verified["receipts_verified"], rows.len());
        assert!(command()
            .args(["summarize", "--rows"])
            .arg(t.0.join("rows.jsonl"))
            .arg("--out")
            .arg(t.0.join("summary.json"))
            .status()
            .unwrap()
            .success());
        let summary: Value =
            serde_json::from_slice(&fs::read(t.0.join("summary.json")).unwrap()).unwrap();
        assert_eq!(summary["schema"], "spiralcore.wan.summary.v2");
        assert_eq!(summary["missing_rows"], 0);
        assert_eq!(summary["failed_rows"], 0);
        assert_eq!(summary["suite"], suite);
        assert!(summary["groups"]
            .as_array()
            .unwrap()
            .iter()
            .all(|g| g["paired_zstd_over_candidate_wall"]["n"] == 1));
        for row in rows
            .iter()
            .filter(|r| r["task"]["arm"] == "prepared" && r["phase"] == "transfer")
        {
            let seed = rows
                .iter()
                .find(|s| s["task"]["id"] == row["task"]["id"] && s["phase"] == "seed")
                .unwrap();
            let group = summary["groups"]
                .as_array()
                .unwrap()
                .iter()
                .find(|g| g["arm"] == "prepared" && g["case"] == row["task"]["case"])
                .unwrap();
            assert_eq!(
                group["seed_plus_transfer_sender_cpu_micros"]["median"]
                    .as_f64()
                    .unwrap(),
                (seed["metrics"]["sender_cpu_micros"].as_u64().unwrap()
                    + row["metrics"]["sender_cpu_micros"].as_u64().unwrap()) as f64
            );
            assert_eq!(
                group["seed_base_prepare_cpu_micros"]["median"]
                    .as_f64()
                    .unwrap(),
                seed["metrics"]["base_prepare_cpu_micros"].as_u64().unwrap() as f64
            );
        }
        let mut tampered = rows;
        tampered[0]["schema"] = "spiralcore.wan.row.v1".into();
        fs::write(
            t.0.join("tampered.jsonl"),
            tampered
                .iter()
                .map(|r| serde_json::to_string(r).unwrap() + "\n")
                .collect::<String>(),
        )
        .unwrap();
        assert!(!command()
            .args(["summarize", "--rows"])
            .arg(t.0.join("tampered.jsonl"))
            .arg("--out")
            .arg(t.0.join("invalid.json"))
            .status()
            .unwrap()
            .success());
    }
}
#[test]
fn full_one_trial_http_campaign_qualifies_exact_bytes_after_restart() {
    let t = Temp::new();
    let p = plan(&t, "1", "plan.json");
    let token = "cd".repeat(32);
    let cache = t.0.join("cache");
    let mut server = Server(
        Command::new(env!("CARGO_BIN_EXE_spiralcore-wan"))
            .args(["serve", "--plan"])
            .arg(t.0.join("plan.json"))
            .arg("--cache")
            .arg(&cache)
            .args(["--seconds", "180"])
            .env("SPIRALCORE_WAN_TOKEN", &token)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut ready = String::new();
    BufReader::new(server.0.stdout.take().unwrap())
        .read_line(&mut ready)
        .unwrap();
    assert!(ready.starts_with("READY addr="), "{ready}");
    let url = format!(
        "http://{}",
        ready.trim().strip_prefix("READY addr=").unwrap()
    );
    let output = command()
        .args(["run", "--plan"])
        .arg(t.0.join("plan.json"))
        .args(["--url", &url, "--out"])
        .arg(t.0.join("rows.jsonl"))
        .env("SPIRALCORE_WAN_TOKEN", token)
        .env("SPIRALCORE_SOURCE_REV", "integration-test")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    drop(server);
    let rows = fs::read_to_string(t.0.join("rows.jsonl")).unwrap();
    let values: Vec<Value> = rows
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(values.len(), 36);
    assert!(
        values
            .iter()
            .all(|r| r["status"] == "published"
                && r["metrics"]["http_requests"].as_u64().unwrap() > 0)
    );
    let cold_raw = values
        .iter()
        .find(|r| {
            r["task"]["case"] == "cold_random"
                && r["task"]["arm"] == "raw_full"
                && r["task"]["size"] == 1048576
        })
        .unwrap();
    assert_eq!(cold_raw["metrics"]["http_requests"], 1);
    let repeat = values
        .iter()
        .find(|r| {
            r["task"]["case"] == "repeat"
                && r["task"]["arm"] == "raw_cas"
                && r["phase"] == "transfer"
                && r["task"]["size"] == 1048576
        })
        .unwrap();
    assert!(repeat["metrics"]["request_body_bytes"].as_u64().unwrap() < 1024);
    let verified = command()
        .args(["verify", "--plan"])
        .arg(t.0.join("plan.json"))
        .arg("--rows")
        .arg(t.0.join("rows.jsonl"))
        .arg("--cache")
        .arg(&cache)
        .output()
        .unwrap();
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );
    let verification: Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(verification["status"], "exact");
    assert_eq!(verification["receipts_verified"], 36);
    assert!(command()
        .args(["summarize", "--rows"])
        .arg(t.0.join("rows.jsonl"))
        .arg("--out")
        .arg(t.0.join("summary.json"))
        .status()
        .unwrap()
        .success());
    let summary: Value =
        serde_json::from_slice(&fs::read(t.0.join("summary.json")).unwrap()).unwrap();
    assert_eq!(summary["missing_rows"], 0);
    assert_eq!(summary["failed_rows"], 0);
    // Metadata substitution must fail even if the substituted object exists in another scope.
    let mut tampered = values;
    tampered[0]["task"]["scope"] = p["allowed_scopes"][1].clone();
    let altered = tampered
        .into_iter()
        .map(|v| serde_json::to_string(&v).unwrap() + "\n")
        .collect::<String>();
    fs::write(t.0.join("tampered.jsonl"), altered).unwrap();
    assert!(!command()
        .args(["verify", "--plan"])
        .arg(t.0.join("plan.json"))
        .arg("--rows")
        .arg(t.0.join("tampered.jsonl"))
        .arg("--cache")
        .arg(&cache)
        .status()
        .unwrap()
        .success());
}
