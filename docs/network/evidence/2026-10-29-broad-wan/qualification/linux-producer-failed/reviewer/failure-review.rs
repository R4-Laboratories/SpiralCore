use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const SOURCE: &str = "16fe9f44e858053a202b396fb7fca628dfcfd97c";
const RUN: u64 = 40947946641;
fn read(p: &Path) -> Result<Vec<u8>> {
    let m = fs::symlink_metadata(p)?;
    assert!(m.file_type().is_file() && m.len() <= 16 << 20);
    let mut b = Vec::new();
    File::open(p)?.take((16 << 20) + 1).read_to_end(&mut b)?;
    assert_eq!(b.len() as u64, m.len());
    Ok(b)
}
fn digest(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn j(p: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&read(p)?)?)
}
fn text(p: &Path) -> Result<String> {
    Ok(String::from_utf8(read(p)?)?)
}
fn inventory(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) -> Result<()> {
    for e in fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_dir() {
            assert!(!fs::symlink_metadata(&p)?.file_type().is_symlink());
            inventory(root, &p, out)?;
        } else {
            assert!(out.len() < 256);
            out.insert(
                p.strip_prefix(root)?.display().to_string(),
                digest(&read(&p)?),
            );
        }
    }
    Ok(())
}
fn main() -> Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    assert_eq!(args.len(), 2);
    let root = PathBuf::from(&args[0]);
    let remote = root.join("remote");
    let run = j(&root.join("run-api.json"))?;
    assert_eq!(run["id"], RUN);
    assert_eq!(run["run_attempt"], 1);
    assert_eq!(run["head_sha"], SOURCE);
    assert_eq!(run["status"], "completed");
    assert_eq!(run["conclusion"], "failure");
    assert_eq!(run["path"], ".github/workflows/spiralcore-wan.yml");
    for k in ["repository", "head_repository"] {
        assert_eq!(run[k]["full_name"], "R4-Laboratories/SpiralCore");
    }
    let jobs = j(&root.join("jobs-api.json"))?;
    assert_eq!(jobs["total_count"], 1);
    assert_eq!(jobs["jobs"].as_array().unwrap().len(), 1);
    let steps = jobs["jobs"][0]["steps"].as_array().unwrap();
    let step = |name: &str, state: &str| {
        let v = steps
            .iter()
            .filter(|s| s["name"] == name)
            .collect::<Vec<_>>();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0]["conclusion"], state);
    };
    for name in [
        "Validate and build the exact revision",
        "Qualify the Linux capture lifecycle before any WAN transfer",
    ] {
        step(name, "success");
    }
    step(
        "Record isolated qualification-only source coverage",
        "failure",
    );
    for name in [
        "Seal the qualification-only sender build",
        "Retain the immutable source-qualified sender build",
        "Record producer artifact custody identity",
        "Transfer over the WAN and capture the encrypted runner leg",
    ] {
        step(name, "skipped");
    }
    let artifacts = j(&root.join("artifacts-api.json"))?;
    assert_eq!(artifacts["total_count"], 1);
    assert_eq!(artifacts["artifacts"].as_array().unwrap().len(), 1);
    let artifact = j(&root.join("evidence-artifact-api.json"))?;
    assert_eq!(artifact, artifacts["artifacts"][0]);
    assert_eq!(artifact["id"], 12929774775u64);
    assert_eq!(artifact["name"], "spiralcore-wan-40947946641-1");
    assert_eq!(artifact["workflow_run"]["id"], RUN);
    assert_eq!(artifact["workflow_run"]["head_sha"], SOURCE);
    assert_eq!(artifact["expired"], false);
    let zip = read(&root.join("evidence.zip"))?;
    assert_eq!(artifact["size_in_bytes"], zip.len());
    assert_eq!(artifact["digest"], format!("sha256:{}", digest(&zip)));
    assert_eq!(text(&remote.join("source-revision.txt"))?.trim(), SOURCE);
    let coverage = j(&remote.join("coverage-status.json"))?;
    assert_eq!(coverage["schema"], "spiralcore.source-coverage.v1");
    assert_eq!(coverage["stage"], "all_target_tests_including_ignored");
    assert_eq!(coverage["exit"], 124);
    assert_eq!(coverage["release_unchanged"], true);
    let expected = "target/release/spiralcore-bench: OK\ntarget/release/spiralcore-capture: OK\ntarget/release/spiralcore-capture-control: OK\n";
    assert_eq!(text(&remote.join("release-before-coverage.txt"))?, expected);
    assert_eq!(text(&remote.join("release-after-coverage.txt"))?, expected);
    for path in [
        "coverage.json",
        "instrumented-capture-qualification",
        "producer-build-manifest.json",
        "sender-artifact-custody.json",
        "rows.jsonl",
    ] {
        assert!(!remote.join(path).exists());
    }
    let log = text(&root.join("failure-diagnostic.txt"))?;
    assert!(log.contains("Finished `test` profile [unoptimized + debuginfo] target(s) in 33.45s"));
    assert!(log.contains("broad_local_http_first_pair_for_every_case_reopens_exactly has been running for over 60 seconds"));
    assert!(log.contains("Process completed with exit code 124"));
    assert!(!log.contains("test result: FAILED") && !log.contains("panicked at"));
    let benchmark_passes = log
        .lines()
        .filter(|l| l.contains("test tests::") && l.ends_with(" ... ok"))
        .count();
    assert_eq!(benchmark_passes, 10); // one CLI deadline test plus nine benchmark tests
    let capture = j(&root.join("ordinary-capture-review.json"))?;
    assert_eq!(capture["status"], "passed");
    assert_eq!(capture["source_revision"], SOURCE);
    assert_eq!(capture["run_id"], RUN);
    let mut hashes = BTreeMap::new();
    inventory(&root, &remote, &mut hashes)?;
    for p in [
        "run-api.json",
        "jobs-api.json",
        "artifacts-api.json",
        "evidence-artifact-api.json",
        "evidence.zip",
        "failure-diagnostic.txt",
        "ordinary-capture-review.json",
        "reviewer/ordinary-capture-review.rs",
        "reviewer/ordinary-capture-review",
    ] {
        hashes.insert(p.to_owned(), digest(&read(&root.join(p))?));
    }
    let receipt = json!({"schema":"spiralcore.failed-producer-independent-review.v1","status":"producer_rejected","source":SOURCE,"run_id":RUN,"github_conclusion":"failure","artifact_custody":{"artifact":artifact,"archive_bytes":zip.len(),"archive_sha256":digest(&zip),"authenticated_api_digest_and_length_verified":true,"sender_artifact_created":false},"ordinary_source_and_capture_passed":true,"ordinary_capture_review":capture,"coverage":coverage,"diagnosis":{"observed":"300-second instrumented cargo-test command timed out while the ignored 88-operation broad benchmark qualification was still running","instrumented_compile_seconds":33.45,"completed_tests_visible_before_timeout":14,"completed_benchmark_tests":9,"benchmark_test_count":10,"no_printed_assertion_failure":true,"not_run":["instrumented capture qualification","coverage report","sender manifest sealing/upload","WAN measurement"],"interpretation":"The retained log establishes a timeout, not a correctness failure in the unfinished test and not proof that a longer run would pass."},"release_unchanged":"verified both producer checksum-check receipts; executable bytes absent because sender upload was skipped","input_sha256":hashes,"reviewer_source_sha256":digest(include_bytes!("failure-review.rs")),"reviewer_executable_sha256":digest(&read(&std::env::current_exe()?)?),"limitations":["Ordinary capture validation is local synthetic plaintext TCP, not WAN performance or TLS evidence","No downloaded Linux executable was run locally; this failed producer has no sender ZIP to verify","No campaign gate files were written, no retry or GitHub mutation was performed"]});
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    serde_json::to_writer_pretty(&mut out, &receipt)?;
    out.write_all(b"\n")?;
    out.sync_all()?;
    println!("VERIFIED: failed producer retained; ordinary capture passed; coverage timeout; no reusable sender");
    Ok(())
}
