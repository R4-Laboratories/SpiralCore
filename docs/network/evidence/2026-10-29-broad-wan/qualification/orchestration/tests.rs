use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
const SCRIPT: &str = "/Users/casey.allard/spiralcore/local/broad-wan-20261029/run-shard.sh";
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    q: Value,
    manifest: Value,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "spiralcore-shard-pin-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("qualification")).unwrap();
        fs::create_dir(root.join("bin")).unwrap();
        fs::create_dir(root.join("shard")).unwrap();
        fs::create_dir(root.join("shard/remote")).unwrap();
        let q = json!({"status":"passed","source":"a".repeat(40),"producer":{"run_id":"123","artifact_id":"456","artifact_digest":format!("sha256:{}","b".repeat(64))}});
        let manifest = json!({"schema":"spiralcore.wan-sender-build.v1","identity":{"source":"a".repeat(40),"run_id":123,"run_attempt":1},"qualification":{"source_tests_passed":true,"clippy_passed":true,"capture_passed":true},"binaries":[{"name":"spiralcore-bench","sha256":"c".repeat(64)}]});
        let f = Self { root, q, manifest };
        f.save();
        fs::write(f.root.join("bin/gh"),"#!/bin/bash\nset -eu\n[[ \"$1\" == variable && \"$2\" == get ]] || exit 90\ncase \"$3\" in\nSPIRALCORE_WAN_BUILD_RUN) printf '%s\\n' \"$MOCK_RUN\";;\nSPIRALCORE_WAN_BUILD_ARTIFACT) printf '%s\\n' \"$MOCK_ARTIFACT\";;\n*) exit 91;;\nesac\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(f.root.join("bin/gh"), fs::Permissions::from_mode(0o700)).unwrap();
        }
        f.download();
        f
    }
    fn save(&self) {
        fs::write(
            self.root.join("qualification/linux-qualified.json"),
            serde_json::to_vec(&self.q).unwrap(),
        )
        .unwrap();
        fs::write(
            self.root.join("qualification/producer-build-manifest.json"),
            serde_json::to_vec(&self.manifest).unwrap(),
        )
        .unwrap();
    }
    fn download(&self) {
        let run = json!({"id":123,"head_sha":"a".repeat(40),"run_attempt":1,"status":"completed","conclusion":"success"});
        let artifact = json!({"id":456,"workflow_run_id":123,"head_sha":"a".repeat(40),"digest":format!("sha256:{}","b".repeat(64)),"size_in_bytes":42});
        let reuse = json!({"schema":"spiralcore.wan-build-reuse-validation.v1","status":"passed","manifest":self.manifest,"producer_run":run,"artifact":artifact});
        let proof = json!({"schema":"spiralcore.wan-build-proof-validation.v1","status":"passed","producer_run":run,"artifact":artifact});
        let archive = json!({"schema":"spiralcore.sender-archive-validation.v1","status":"passed","artifact_id":456,"archive_sha256":"b".repeat(64),"archive_bytes":42});
        for (name, v) in [
            ("build-reuse-validation.json", reuse),
            ("producer-proof-validation.json", proof),
            ("sender-archive-validation.json", archive),
            ("producer-build-manifest.json", self.manifest.clone()),
        ] {
            fs::write(
                self.root.join("shard/remote").join(name),
                serde_json::to_vec(&v).unwrap(),
            )
            .unwrap();
        }
    }
    fn mutate_remote(&self, name: &str, f: impl FnOnce(&mut Value)) {
        let p = self.root.join("shard/remote").join(name);
        let mut v: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
        f(&mut v);
        fs::write(p, serde_json::to_vec(&v).unwrap()).unwrap();
    }
    fn run(&self, stage: &str, run: &str, artifact: &str) -> bool {
        let text = fs::read_to_string(SCRIPT).unwrap();
        let block = |name: &str| {
            let start = format!("# BEGIN {name}\n");
            let end = format!("# END {name}");
            text.split_once(&start)
                .expect("registered block missing")
                .1
                .split_once(&end)
                .expect("block end missing")
                .0
                .to_owned()
        };
        let mut shell=String::from("set -euo pipefail\nrepo=fixture/repo\nsource_rev=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n");
        shell.push_str(&block("producer preflight"));
        if stage != "preflight" {
            shell.push_str(&block("producer checkpoint"));
        }
        if stage == "download" {
            shell.push_str(&block("producer download validation"));
        }
        shell.push_str("printf 'ACCEPTED\\n'\n");
        let output = Command::new("/bin/bash")
            .arg("-e")
            .arg("-c")
            .arg(shell)
            .env_clear()
            .env(
                "PATH",
                format!(
                    "{}:/opt/homebrew/bin:/usr/bin:/bin",
                    self.root.join("bin").display()
                ),
            )
            .env("root", &self.root)
            .env("dir", self.root.join("shard"))
            .env("MOCK_RUN", run)
            .env("MOCK_ARTIFACT", artifact)
            .output()
            .unwrap();
        output.status.success() && output.stdout.ends_with(b"ACCEPTED\n")
    }
}
#[test]
fn preflight_accepts_only_paired_canonical_source_bound_producer_pins() {
    let f = Fixture::new();
    assert!(f.run("preflight", "123", "456"));
    for (run, artifact) in [("124", "456"), ("123", "457"), ("", "456"), ("123", "")] {
        assert!(!f.run("preflight", run, artifact));
    }
    for mutation in 0..10 {
        let mut f = Fixture::new();
        match mutation {
            0 => f.q["producer"]["run_id"] = json!("0"),
            1 => f.q["producer"]["run_id"] = json!("01"),
            2 => f.q["producer"]["artifact_id"] = json!("-1"),
            3 => f.q["producer"]["artifact_id"] = json!("18446744073709551616"),
            4 => f.q["producer"]["artifact_digest"] = json!("b".repeat(64)),
            5 => f.q["producer"]["artifact_digest"] = json!(format!("sha256:{}", "B".repeat(64))),
            6 => f.q["source"] = json!("c".repeat(40)),
            7 => f.q["status"] = json!("failed"),
            8 => f.manifest["identity"]["source"] = json!("c".repeat(40)),
            _ => f.manifest["identity"]["run_id"] = json!(124),
        }
        f.save();
        assert!(!f.run("preflight", "123", "456"), "mutation{mutation}");
    }
}
#[test]
fn checkpoint_preserves_exact_qualification_and_manifest() {
    let f = Fixture::new();
    assert!(f.run("checkpoint", "123", "456"));
    for (name, target) in [
        ("linux-qualified.json", "linux-qualified.json"),
        (
            "producer-build-manifest.json",
            "frozen-producer-build-manifest.json",
        ),
    ] {
        assert_eq!(
            fs::read(f.root.join("qualification").join(name)).unwrap(),
            fs::read(f.root.join("shard").join(target)).unwrap()
        );
    }
}
#[test]
fn downloaded_reuse_proof_archive_and_manifest_must_match_pins() {
    let f = Fixture::new();
    assert!(f.run("download", "123", "456"));
    for mutation in 0..12 {
        let f = Fixture::new();
        match mutation {
            0 => f.mutate_remote("build-reuse-validation.json", |v| {
                v["status"] = json!("failed")
            }),
            1 => f.mutate_remote("build-reuse-validation.json", |v| {
                v["manifest"]["identity"]["source"] = json!("c".repeat(40))
            }),
            2 => f.mutate_remote("build-reuse-validation.json", |v| {
                v["producer_run"]["id"] = json!(124)
            }),
            3 => f.mutate_remote("build-reuse-validation.json", |v| {
                v["artifact"]["id"] = json!(457)
            }),
            4 => f.mutate_remote("build-reuse-validation.json", |v| {
                v["artifact"]["digest"] = json!(format!("sha256:{}", "c".repeat(64)))
            }),
            5 => f.mutate_remote("producer-build-manifest.json", |v| {
                v["binaries"][0]["sha256"] = json!("d".repeat(64))
            }),
            6 => f.mutate_remote("producer-proof-validation.json", |v| {
                v["producer_run"]["id"] = json!(124)
            }),
            7 => f.mutate_remote("sender-archive-validation.json", |v| {
                v["archive_sha256"] = json!("c".repeat(64))
            }),
            8 => f.mutate_remote("sender-archive-validation.json", |v| {
                v["artifact_id"] = json!(457)
            }),
            9 => f.mutate_remote("sender-archive-validation.json", |v| {
                v["archive_bytes"] = json!(43)
            }),
            10 => f.mutate_remote("producer-proof-validation.json", |v| {
                v["status"] = json!("failed")
            }),
            _ => f.mutate_remote("sender-archive-validation.json", |v| {
                v["status"] = json!("failed")
            }),
        }
        assert!(!f.run("download", "123", "456"), "mutation{mutation}");
    }
}
