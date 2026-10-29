use serde_json::Value;
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
    let out = t.0.join(filename);
    let result = command()
        .args([
            "plan",
            "--trials",
            trials,
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
    serde_json::from_slice(&fs::read(out).unwrap()).unwrap()
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
