use spiralcore_build_manifest::{
    check_proofs, create, read_json, verify, verify_archive, write_json, ArtifactProof, Expected,
    RunProof,
};
use std::path::Path;
fn run() -> Result<(), String> {
    let a: Vec<_> = std::env::args_os().collect();
    if a.len() == 5 && a[1] == "create" {
        let expected: Expected = read_json(Path::new(&a[2]))?;
        let root = Path::new(&a[3]);
        let manifest = create(&expected, root, Path::new(&a[4]))?;
        write_json(&root.join("manifest.json"), &manifest)?;
    } else if a.len() == 6 && a[1] == "proofs" {
        let expected: Expected = read_json(Path::new(&a[2]))?;
        let run: RunProof = read_json(Path::new(&a[3]))?;
        let artifact: ArtifactProof = read_json(Path::new(&a[4]))?;
        check_proofs(&expected, &run, &artifact)?;
        write_json(
            Path::new(&a[5]),
            &serde_json::json!({"schema":"spiralcore.wan-build-proof-validation.v1","status":"passed","producer_run":run,"artifact":artifact}),
        )?;
    } else if a.len() == 7 && a[1] == "archive" {
        let expected: Expected = read_json(Path::new(&a[2]))?;
        let run: RunProof = read_json(Path::new(&a[3]))?;
        let artifact: ArtifactProof = read_json(Path::new(&a[4]))?;
        check_proofs(&expected, &run, &artifact)?;
        let receipt = verify_archive(&artifact, Path::new(&a[5]))?;
        write_json(Path::new(&a[6]), &receipt)?;
    } else if a.len() == 7 && a[1] == "verify" {
        let expected: Expected = read_json(Path::new(&a[2]))?;
        let run: RunProof = read_json(Path::new(&a[3]))?;
        let artifact: ArtifactProof = read_json(Path::new(&a[4]))?;
        let manifest = verify(&expected, &run, &artifact, Path::new(&a[5]))?;
        write_json(
            Path::new(&a[6]),
            &serde_json::json!({"schema":"spiralcore.wan-build-reuse-validation.v1","status":"passed","manifest":manifest,"producer_run":run,"artifact":artifact,"scope":"same-source binary provenance; current runner capture qualification still required"}),
        )?;
    } else {
        return Err("usage: spiralcore-build-manifest create EXPECTED_JSON NEW_ARTIFACT_DIR CAPTURE_QUALIFICATION_JSON | proofs EXPECTED_JSON RUN_PROOF_JSON ARTIFACT_PROOF_JSON NEW_RECEIPT_JSON | archive EXPECTED_JSON RUN_PROOF_JSON ARTIFACT_PROOF_JSON ZIP NEW_RECEIPT_JSON | verify EXPECTED_JSON RUN_PROOF_JSON ARTIFACT_PROOF_JSON ARTIFACT_DIR NEW_RECEIPT_JSON".into());
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("BUILD_MANIFEST_ERROR {error}");
        std::process::exit(1);
    }
}
