use serde_json::json;
use sha2::{Digest, Sha256};
use spiralcore_build_manifest::{
    write_json, ArtifactProof, Expected, Identity, RunProof, BINARIES,
};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    expected: Expected,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "spiralcore-build-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("artifact")).unwrap();
        for name in BINARIES {
            fs::write(root.join("artifact").join(name), name.as_bytes()).unwrap();
        }
        let expected = Expected {
            identity: Identity {
                source: "a".repeat(40),
                repository: "R4-Laboratories/SpiralCore".into(),
                workflow: ".github/workflows/spiralcore-wan.yml".into(),
                run_id: 123,
                run_attempt: 1,
                compiler: "rustc 1.97.1 (fixture 2026-01-01)".into(),
                target: "x86_64-unknown-linux-gnu".into(),
                features_sha256: "b".repeat(64),
                lock_sha256: "c".repeat(64),
            },
            artifact_id: None,
        };
        write_json(&root.join("expected.json"), &expected).unwrap();
        write_json(
            &root.join("capture.json"),
            &json!({"schema":"spiralcore.capture-qualification.v2","passed":true,"cases":[],"controller_sha256":format!("{:x}",Sha256::digest(BINARIES[2].as_bytes()))}),
        )
        .unwrap();
        Self { root, expected }
    }
    fn call(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_spiralcore-build-manifest"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .unwrap()
    }
    fn create(&self) -> std::process::Output {
        self.call(&["create", "expected.json", "artifact", "capture.json"])
    }
    fn proof(&mut self) {
        self.expected.artifact_id = Some(456);
        fs::write(
            self.root.join("expected.json"),
            serde_json::to_vec(&self.expected).unwrap(),
        )
        .unwrap();
        write_json(
            &self.root.join("run.json"),
            &RunProof {
                id: 123,
                head_sha: "a".repeat(40),
                run_attempt: 1,
                path: self.expected.identity.workflow.clone(),
                status: "completed".into(),
                conclusion: "success".into(),
                repository: self.expected.identity.repository.clone(),
                head_repository: self.expected.identity.repository.clone(),
                capture_successes: 1,
                source_build_successes: 1,
            },
        )
        .unwrap();
        write_json(
            &self.root.join("artifact.json"),
            &ArtifactProof {
                id: 456,
                name: format!("spiralcore-wan-sender-{}", "a".repeat(40)),
                size_in_bytes: 2048,
                expired: false,
                workflow_run_id: 123,
                head_sha: "a".repeat(40),
                digest: format!("sha256:{}", "d".repeat(64)),
            },
        )
        .unwrap();
    }
}
#[test]
fn creates_checks_and_preserves_exact_artifact_without_execution() {
    let mut f = Fixture::new();
    assert!(f.create().status.success());
    assert!(!f.create().status.success());
    f.proof();
    assert!(f
        .call(&[
            "proofs",
            "expected.json",
            "run.json",
            "artifact.json",
            "proof.json"
        ])
        .status
        .success());
    assert!(f
        .call(&[
            "verify",
            "expected.json",
            "run.json",
            "artifact.json",
            "artifact",
            "verified.json"
        ])
        .status
        .success());
    let v: serde_json::Value =
        serde_json::from_slice(&fs::read(f.root.join("verified.json")).unwrap()).unwrap();
    assert_eq!(v["status"], "passed");
    assert_eq!(v["manifest"]["binaries"].as_array().unwrap().len(), 3);
}
#[test]
fn failed_capture_does_not_create_a_manifest() {
    for mutation in 0..2 {
        let f = Fixture::new();
        fs::write(f.root.join("capture.json"),serde_json::to_vec(&json!({"schema":if mutation==0{"spiralcore.capture-qualification.v1"}else{"spiralcore.capture-qualification.v2"},"passed":mutation==0,"controller_sha256":format!("{:x}",Sha256::digest(BINARIES[2].as_bytes()))})).unwrap()).unwrap();
        assert!(!f.create().status.success());
        assert!(!f.root.join("artifact/manifest.json").exists());
    }
}
#[test]
fn corruption_produces_failure_without_a_success_receipt() {
    let mut f = Fixture::new();
    assert!(f.create().status.success());
    f.proof();
    fs::write(f.root.join("artifact").join(BINARIES[0]), b"changed").unwrap();
    assert!(!f
        .call(&[
            "verify",
            "expected.json",
            "run.json",
            "artifact.json",
            "artifact",
            "verified.json"
        ])
        .status
        .success());
    assert!(!f.root.join("verified.json").exists());
}
#[test]
fn capture_qualification_must_bind_the_packaged_controller() {
    let f = Fixture::new();
    fs::write(f.root.join("capture.json"),serde_json::to_vec(&json!({"schema":"spiralcore.capture-qualification.v2","passed":true,"controller_sha256":"f".repeat(64)})).unwrap()).unwrap();
    assert!(!f.create().status.success());
    assert!(!f.root.join("artifact/manifest.json").exists());
}
#[test]
fn rejects_unknown_cli_and_preserves_existing_output() {
    let f = Fixture::new();
    assert!(!f.call(&["other"]).status.success());
    assert!(f.create().status.success());
    let before = fs::read(f.root.join("artifact/manifest.json")).unwrap();
    assert!(!f.create().status.success());
    assert_eq!(
        fs::read(f.root.join("artifact/manifest.json")).unwrap(),
        before
    );
}
#[test]
fn archive_cli_binds_api_digest_before_producing_extraction_permission() {
    use std::io::Write;
    let mut f = Fixture::new();
    assert!(f.create().status.success());
    f.proof();
    let zip_path = f.root.join("sender.zip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
    for name in BINARIES.into_iter().chain(["manifest.json"]) {
        zip.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(&fs::read(f.root.join("artifact").join(name)).unwrap())
            .unwrap();
    }
    zip.finish().unwrap();
    let bytes = fs::read(&zip_path).unwrap();
    let proof_path = f.root.join("artifact.json");
    let mut proof: ArtifactProof = serde_json::from_slice(&fs::read(&proof_path).unwrap()).unwrap();
    proof.size_in_bytes = bytes.len() as u64;
    proof.digest = format!("sha256:{:x}", Sha256::digest(&bytes));
    fs::write(&proof_path, serde_json::to_vec(&proof).unwrap()).unwrap();
    assert!(f
        .call(&[
            "archive",
            "expected.json",
            "run.json",
            "artifact.json",
            "sender.zip",
            "archive-receipt.json"
        ])
        .status
        .success());
    fs::write(zip_path, b"corrupt archive").unwrap();
    assert!(!f
        .call(&[
            "archive",
            "expected.json",
            "run.json",
            "artifact.json",
            "sender.zip",
            "bad-archive-receipt.json"
        ])
        .status
        .success());
    assert!(!f.root.join("bad-archive-receipt.json").exists());
}
