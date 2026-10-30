use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const SOURCE: &str = "9dce4c57d1f41cef7365b297cd690849afce34e7";
const RUN: u64 = 40950252067;
fn read(p: &Path) -> Result<Vec<u8>> {
    let m = fs::symlink_metadata(p)?;
    assert!(m.file_type().is_file() && m.len() <= 128 << 20);
    let mut b = Vec::new();
    File::open(p)?.take((128 << 20) + 1).read_to_end(&mut b)?;
    assert_eq!(b.len() as u64, m.len());
    Ok(b)
}
fn digest(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn j(p: &Path) -> Result<Value> {
    let b = read(p)?;
    assert!(b.len() <= 16 << 20);
    Ok(serde_json::from_slice(&b)?)
}
fn text(p: &Path) -> Result<String> {
    let b = read(p)?;
    assert!(b.len() <= 16 << 20);
    Ok(String::from_utf8(b)?)
}
fn n(v: &Value) -> u64 {
    v.as_u64().unwrap()
}
fn s(v: &Value) -> &str {
    v.as_str().unwrap()
}
fn write(p: &Path, v: &Value) -> Result<()> {
    let mut f = OpenOptions::new().write(true).create_new(true).open(p)?;
    serde_json::to_writer_pretty(&mut f, v)?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    Ok(())
}
fn inventory(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) -> Result<()> {
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        let m = fs::symlink_metadata(&p)?;
        if m.is_dir() {
            inventory(root, &p, out)?;
        } else {
            assert!(out.len() < 512);
            out.insert(
                p.strip_prefix(root)?.display().to_string(),
                digest(&read(&p)?),
            );
        }
    }
    Ok(())
}
fn common(root: &Path, worktree: &Path) -> Result<(Value, Value, Value)> {
    let remote = root.join("remote");
    let run = j(&root.join("run-api.json"))?;
    assert_eq!(run["id"], RUN);
    assert_eq!(run["run_attempt"], 1);
    assert_eq!(run["head_sha"], SOURCE);
    assert_eq!(run["status"], "completed");
    assert_eq!(run["conclusion"], "success");
    assert_eq!(run["path"], ".github/workflows/spiralcore-wan.yml");
    for k in ["repository", "head_repository"] {
        assert_eq!(run[k]["full_name"], "R4-Laboratories/SpiralCore");
    }
    let jobs = j(&root.join("jobs-api.json"))?;
    assert_eq!(jobs["total_count"], 1);
    assert_eq!(jobs["jobs"].as_array().unwrap().len(), 1);
    let steps = jobs["jobs"][0]["steps"].as_array().unwrap();
    let step = |name: &str, state: &str| {
        let matches = steps
            .iter()
            .filter(|v| v["name"] == name)
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["conclusion"], state);
    };
    for name in [
        "Build the checked-out provenance validator and record current runner",
        "Validate and build the exact revision",
        "Qualify the Linux capture lifecycle before any WAN transfer",
        "Record isolated qualification-only source coverage",
        "Seal the qualification-only sender build",
        "Retain the immutable source-qualified sender build",
        "Record producer artifact custody identity",
    ] {
        step(name, "success");
    }
    step(
        "Transfer over the WAN and capture the encrypted runner leg",
        "skipped",
    );
    let artifact = j(&root.join("sender-artifact-api.json"))?;
    let artifacts = j(&root.join("artifacts-api.json"))?;
    assert_eq!(artifacts["total_count"], 2);
    assert_eq!(artifacts["artifacts"].as_array().unwrap().len(), 2);
    assert!(artifacts["artifacts"]
        .as_array()
        .unwrap()
        .contains(&artifact));
    assert_eq!(artifact["name"], format!("spiralcore-wan-sender-{SOURCE}"));
    assert_eq!(artifact["expired"], false);
    assert_eq!(artifact["workflow_run"]["id"], RUN);
    assert_eq!(artifact["workflow_run"]["head_sha"], SOURCE);
    assert!((1..=999999999999999999).contains(&n(&artifact["id"])));
    assert!((1..=100 << 20).contains(&n(&artifact["size_in_bytes"])));
    let custody = j(&remote.join("sender-artifact-custody.json"))?;
    assert_eq!(custody["schema"], "spiralcore.sender-artifact-custody.v1");
    assert_eq!(custody["artifact_id"], artifact["id"]);
    assert_eq!(custody["source"], SOURCE);
    assert_eq!(custody["producer_run"], RUN);
    assert_eq!(custody["retention_days"], 1);
    let upload_digest = s(&custody["artifact_digest"])
        .strip_prefix("sha256:")
        .unwrap_or(s(&custody["artifact_digest"]));
    assert_eq!(artifact["digest"], format!("sha256:{upload_digest}"));
    assert_eq!(upload_digest.len(), 64);
    assert!(upload_digest
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    let mut expected = j(&remote.join("expected-build.json"))?;
    assert!(expected["artifact_id"].is_null());
    let id = &expected["identity"];
    assert_eq!(id["source"], SOURCE);
    assert_eq!(id["repository"], "R4-Laboratories/SpiralCore");
    assert_eq!(id["workflow"], ".github/workflows/spiralcore-wan.yml");
    assert_eq!(id["run_id"], RUN);
    assert_eq!(id["run_attempt"], 1);
    assert_eq!(id["target"], "x86_64-unknown-linux-gnu");
    assert_eq!(text(&remote.join("source-revision.txt"))?.trim(), SOURCE);
    assert_eq!(
        id["compiler"],
        text(&remote.join("rust-version.txt"))?.trim()
    );
    assert!(s(&id["compiler"]).starts_with("rustc 1.97.1 ("));
    assert_eq!(
        id["features_sha256"],
        digest(&read(&remote.join("features.txt"))?)
    );
    assert_eq!(
        id["lock_sha256"],
        digest(&read(
            &worktree.join("crates/spiralcore-transfer/Cargo.lock")
        )?)
    );
    let coverage = j(&remote.join("coverage-status.json"))?;
    assert_eq!(coverage["schema"], "spiralcore.source-coverage.v1");
    assert_eq!(coverage["stage"], "complete");
    assert_eq!(coverage["exit"], 0);
    assert_eq!(coverage["release_unchanged"], true);
    assert_eq!(coverage["coverage_tool"], "cargo-llvm-cov 0.9.1");
    assert_eq!(coverage["coverage_gate"], "report_only");
    let checked="target/release/spiralcore-bench: OK\ntarget/release/spiralcore-capture: OK\ntarget/release/spiralcore-capture-control: OK\n";
    for file in ["release-before-coverage.txt", "release-after-coverage.txt"] {
        assert_eq!(text(&remote.join(file))?, checked);
    }
    let log = text(&root.join("producer-diagnostic.txt"))?;
    let cov = log
        .lines()
        .filter(|l| l.contains("\tRecord isolated qualification-only source coverage\t"))
        .collect::<Vec<_>>();
    assert!(cov.iter().any(|l| l.contains("Finished `release` profile")));
    assert!(!cov.iter().any(|l| l.contains("Finished `test` profile")));
    for command in [
        "test --release --locked",
        "build --release --locked",
        "llvm-cov report --release",
    ] {
        assert!(cov.iter().any(|l| l.contains(command)));
    }
    assert!(cov
        .iter()
        .any(|l| l.contains("broad_local_http_first_pair_for_every_case_reopens_exactly ... ok")));
    assert!(!cov.iter().any(|l| l.contains("test result: FAILED")
        || l.contains("panicked at")
        || l.contains("##[error]")));
    expected["artifact_id"] = artifact["id"].clone();
    let run_proof = json!({"id":run["id"],"head_sha":run["head_sha"],"run_attempt":run["run_attempt"],"path":run["path"],"status":run["status"],"conclusion":run["conclusion"],"repository":run["repository"]["full_name"],"head_repository":run["head_repository"]["full_name"],"capture_successes":1,"source_build_successes":1});
    let artifact_proof = json!({"id":artifact["id"],"name":artifact["name"],"size_in_bytes":artifact["size_in_bytes"],"expired":artifact["expired"],"digest":artifact["digest"],"workflow_run_id":artifact["workflow_run"]["id"],"head_sha":artifact["workflow_run"]["head_sha"]});
    Ok((expected, run_proof, artifact_proof))
}
fn main() -> Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    assert_eq!(args.len(), 3);
    let mode = args[0].to_str().unwrap();
    let root = PathBuf::from(&args[1]);
    let worktree = PathBuf::from(&args[2]);
    let remote = root.join("remote");
    let (expected, run, artifact) = common(&root, &worktree)?;
    if mode == "prepare" {
        write(&root.join("expected-reuse.json"), &expected)?;
        write(&root.join("producer-run.json"), &run)?;
        write(&root.join("producer-artifact.json"), &artifact)?;
        println!("PREPARED: authenticated API projections and successful runtime verified");
        return Ok(());
    }
    assert_eq!(mode, "final");
    assert_eq!(j(&root.join("expected-reuse.json"))?, expected);
    assert_eq!(j(&root.join("producer-run.json"))?, run);
    assert_eq!(j(&root.join("producer-artifact.json"))?, artifact);
    let proof = j(&root.join("producer-proof-validation.json"))?;
    let archive = j(&root.join("sender-archive-validation.json"))?;
    let reuse = j(&root.join("build-reuse-validation.json"))?;
    assert_eq!(proof["schema"], "spiralcore.wan-build-proof-validation.v1");
    assert_eq!(proof["status"], "passed");
    assert_eq!(proof["producer_run"], run);
    assert_eq!(proof["artifact"], artifact);
    assert_eq!(archive["schema"], "spiralcore.sender-archive-validation.v1");
    assert_eq!(archive["status"], "passed");
    assert_eq!(archive["artifact_id"], artifact["id"]);
    assert_eq!(archive["archive_bytes"], artifact["size_in_bytes"]);
    let zip = read(&root.join("sender.zip"))?;
    assert_eq!(archive["archive_bytes"], zip.len());
    assert_eq!(archive["archive_sha256"], digest(&zip));
    assert_eq!(artifact["digest"], format!("sha256:{}", digest(&zip)));
    assert_eq!(reuse["schema"], "spiralcore.wan-build-reuse-validation.v1");
    assert_eq!(reuse["status"], "passed");
    assert_eq!(reuse["producer_run"], run);
    assert_eq!(reuse["artifact"], artifact);
    let manifest = j(&remote.join("producer-build-manifest.json"))?;
    assert_eq!(reuse["manifest"], manifest);
    assert_eq!(manifest["identity"], expected["identity"]);
    assert_eq!(
        read(&remote.join("producer-build-manifest.json"))?,
        read(&root.join("sender/manifest.json"))?
    );
    let capture = j(&root.join("dual-capture-review.json"))?;
    assert_eq!(capture["status"], "passed");
    assert_eq!(capture["source"], SOURCE);
    assert_eq!(capture["run_id"], RUN);
    for kind in ["ordinary", "instrumented"] {
        assert_eq!(capture[kind]["status"], "passed");
    }
    assert_ne!(
        capture["ordinary"]["controller_receipt_sha256"],
        capture["instrumented"]["controller_receipt_sha256"]
    );
    assert_eq!(
        manifest["qualification"]["capture_sha256"],
        digest(&read(
            &remote.join("capture-qualification/qualification.json")
        )?)
    );
    assert_eq!(
        manifest["qualification"]["capture_controller_sha256"],
        capture["ordinary"]["controller_receipt_sha256"]
    );
    let binary_receipt = text(&remote.join("binary-sha256.txt"))?;
    let mut names = BTreeSet::new();
    for binary in manifest["binaries"].as_array().unwrap() {
        let name = s(&binary["name"]);
        assert!([
            "spiralcore-bench",
            "spiralcore-capture",
            "spiralcore-capture-control"
        ]
        .contains(&name));
        assert!(names.insert(name));
        let p = root.join("sender").join(name);
        let b = read(&p)?;
        assert_eq!(binary["bytes"], b.len());
        assert_eq!(binary["sha256"], digest(&b));
        assert_eq!(&b[..6], b"\x7fELF\x02\x01");
        assert_eq!(u16::from_le_bytes([b[18], b[19]]), 62);
        assert_eq!(fs::metadata(&p)?.permissions().mode() & 0o111, 0);
        assert!(binary_receipt
            .lines()
            .any(|l| l == format!("{}  target/release/{name}", digest(&b))));
    }
    assert_eq!(names.len(), 3);
    assert_eq!(binary_receipt.lines().count(), 3);
    let coverage = j(&remote.join("coverage.json"))?;
    assert_eq!(coverage["type"], "llvm.coverage.json.export");
    assert_eq!(coverage["data"].as_array().unwrap().len(), 1);
    let data = &coverage["data"][0];
    assert!(n(&data["totals"]["lines"]["count"]) > 0);
    assert!(n(&data["totals"]["lines"]["covered"]) > 0);
    let mut files = Vec::new();
    for file in data["files"].as_array().unwrap() {
        let filename = s(&file["filename"]);
        if filename.ends_with("/capture_fixture.rs")
            || filename.ends_with("/capture_lifecycle.rs")
            || filename.ends_with("/spiralcore-capture-control.rs")
            || filename.ends_with("/spiralcore-capture.rs")
        {
            files.push(json!({"filename":filename,"summary":file["summary"]}));
        }
    }
    assert_eq!(files.len(), 4);
    let log = text(&root.join("producer-diagnostic.txt"))?;
    let mut test_summaries = Vec::new();
    let mut passed = 0u64;
    for line in log
        .lines()
        .filter(|l| l.contains("\tRecord isolated qualification-only source coverage\t"))
    {
        if let Some((_, result)) = line.split_once("test result: ok. ") {
            let parts = result.split(';').collect::<Vec<_>>();
            assert_eq!(parts[1].trim(), "0 failed");
            assert_eq!(parts[2].trim(), "0 ignored");
            let count = parts[0].split_whitespace().next().unwrap().parse::<u64>()?;
            passed = passed.checked_add(count).unwrap();
            test_summaries.push(result.to_owned());
        }
    }
    // The frozen transfer source has 20 test binaries and 167 tests. Do not
    // confuse the separate provenance-validator package with transfer tests.
    assert_eq!(passed, 167);
    assert_eq!(test_summaries.len(), 20);
    let native_path = worktree
        .join("docs/network/evidence/2026-10-29-broad-wan/qualification/native-complete.json");
    let native = j(&native_path)?;
    assert_eq!(native["all_targets"]["passed"], 167);
    assert_eq!(native["all_targets"]["test_binaries"], 20);
    assert_eq!(native["source_sha256"].as_object().unwrap().len(), 33);
    for (path, expected_hash) in native["source_sha256"].as_object().unwrap() {
        assert_eq!(*expected_hash, digest(&read(&worktree.join(path))?));
    }
    let mut ordinary = (0u64, 0u64, 0usize);
    for line in log
        .lines()
        .filter(|l| l.contains("\tValidate and build the exact revision\t"))
    {
        if let Some((_, result)) = line.split_once("test result: ok. ") {
            let parts = result.split(';').collect::<Vec<_>>();
            assert_eq!(parts[1].trim(), "0 failed");
            ordinary.0 = ordinary
                .0
                .checked_add(parts[0].split_whitespace().next().unwrap().parse::<u64>()?)
                .unwrap();
            ordinary.1 = ordinary
                .1
                .checked_add(parts[2].split_whitespace().next().unwrap().parse::<u64>()?)
                .unwrap();
            ordinary.2 += 1;
        }
    }
    assert_eq!(ordinary, (165, 2, 20));
    let mut hashes = BTreeMap::new();
    hashes.insert(
        "frozen-native-source-qualification.json".into(),
        digest(&read(&native_path)?),
    );
    inventory(&root, &remote, &mut hashes)?;
    inventory(&root, &root.join("sender"), &mut hashes)?;
    for name in [
        "run-api.json",
        "jobs-api.json",
        "artifacts-api.json",
        "sender-artifact-api.json",
        "producer-diagnostic.txt",
        "expected-reuse.json",
        "producer-run.json",
        "producer-artifact.json",
        "producer-proof-validation.json",
        "sender-archive-validation.json",
        "build-reuse-validation.json",
        "sender.zip",
        "dual-capture-review.json",
        "reviewer/capture-review.rs",
        "reviewer/capture-review",
    ] {
        hashes.insert(name.to_owned(), digest(&read(&root.join(name))?));
    }
    let receipt = json!({"schema":"spiralcore.producer-independent-runtime-custody-review.v1","status":"passed","source":SOURCE,"producer":{"run_id":RUN,"artifact_id":artifact["id"],"artifact_digest":artifact["digest"],"archive_bytes":zip.len()},"local_manifest":root.join("sender/manifest.json"),"source_and_api_identity_verified":true,"ordinary_and_instrumented_capture":capture,"coverage":{"status":j(&remote.join("coverage-status.json"))?,"optimized_profile_verified_from_actual_log":true,"all_target_tests_including_ignored_passed":passed,"test_summaries":test_summaries,"totals":data["totals"],"capture_files":files,"gate":"report_only"},"release_unchanged":true,"manifest":manifest,"mac_validator_sha256":digest(&read(Path::new("/Users/casey.allard/spiralcore/target/debug/spiralcore-build-manifest"))?),"mac_validator_source_sha256":{"lib":digest(&read(&worktree.join("tools/wan-build-manifest/src/lib.rs"))?),"main":digest(&read(&worktree.join("tools/wan-build-manifest/src/main.rs"))?)},"input_sha256":hashes,"reviewer_source_sha256":digest(include_bytes!("custody-review.rs")),"reviewer_executable_sha256":digest(&read(&std::env::current_exe()?)?),"limitations":["No WAN transfer ran; synthetic veth plaintext TCP qualification only","Linux binaries were authenticated and retained as nonexecutable files, not executed on macOS","Instrumented executable bytes are not uploaded; instrumented identity is receipt-bound and distinct from ordinary release","Coverage is an optimized instrumented source execution report, not an energy/performance measurement or a minimum coverage acceptance gate","Each consumer still requires its own same-run capture qualification; artifact retention is one day","No campaign gate files, GitHub variables or secrets were modified"]});
    write(
        &root.join("independent-runtime-custody-review.json"),
        &receipt,
    )?;
    println!("PASS: source, both captures, optimized coverage, release identity and authenticated sender custody verified");
    Ok(())
}
