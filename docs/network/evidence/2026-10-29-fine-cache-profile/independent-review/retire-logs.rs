//! Explicitly authorized post-review retirement of owned verification text logs.
//! No endpoint execution, no benchmark, no recursive deletion, no manifest rewrite.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const ROOT: &str = "/Users/casey.allard/spiralcore/local/broad-wan-20261029/cache-profile";
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn read(p: &Path) -> Result<Vec<u8>> {
    if !fs::symlink_metadata(p)?.file_type().is_file() {
        return Err("nonregular selected input".into());
    }
    let mut b = Vec::new();
    File::open(p)?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut b)?;
    if b.len() > 64 * 1024 * 1024 {
        return Err("input exceeds bound".into());
    }
    Ok(b)
}
fn durable(p: &Path, v: &Value) -> Result<String> {
    let mut f = OpenOptions::new().create_new(true).write(true).open(p)?;
    serde_json::to_writer_pretty(&mut f, v)?;
    writeln!(f)?;
    f.sync_all()?;
    File::open(p.parent().ok_or("missing parent")?)?.sync_all()?;
    let b = read(p)?;
    if serde_json::from_slice::<Value>(&b)? != *v {
        return Err("durable receipt readback differs".into());
    }
    Ok(hash(&b))
}
fn main() -> Result<()> {
    let mode = std::env::args()
        .nth(1)
        .ok_or("--check or --apply required")?;
    if !["--check", "--apply"].contains(&mode.as_str()) || std::env::args().count() != 2 {
        return Err("invalid args".into());
    }
    let root = PathBuf::from(ROOT);
    if root.canonicalize()? != root {
        return Err("root identity changed".into());
    }
    let final_review: Value =
        serde_json::from_slice(&read(&root.join("independent-review.json"))?)?;
    if final_review["status"] != "passed" || final_review["observations"] != 72 {
        return Err("independent review missing".into());
    }
    let mut selected: Vec<String> = vec![
        "qualification/clippy-before-analysis-hardening.log",
        "qualification/clippy-probe.log",
        "qualification/clippy-snapshot.log",
        "qualification/final-clippy.log",
        "qualification/final-fmt.log",
        "qualification/final-probe.log",
        "qualification/green-probe.log",
        "qualification/green-snapshot.log",
        "qualification/probe-before-analysis-hardening.log",
        "qualification/red-analyzer-safety.log",
        "qualification/red-categories-initial.log",
        "qualification/red-categories.log",
        "qualification/snapshot-all-targets.log",
        "receipts/build-attempt-1.log",
        "receipts/build.log",
        "receipts/run-profile.stdout",
        "receipts/run-profile.stderr",
        "receipts/template-preparation.stdout",
        "receipts/template-preparation.stderr",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    for block in 0..3 {
        for variant in ["counters", "fine", "empty"] {
            for stream in ["stdout", "stderr"] {
                selected.push(format!("measured/block{block}-{variant}.{stream}"));
            }
        }
    }
    selected.sort();
    if selected.len() != 37 {
        return Err("allowlist count differs".into());
    }
    let mut protected = BTreeMap::new();
    for p in [
        "qualification.json",
        "qualification/qualified-inputs.sha256",
        "snapshot-established.sha256",
        "templates-established.sha256",
        "measured/templates-before.sha256",
        "measured/templates-after.sha256",
        "measured/observations.sha256",
        "measured/execution.jsonl",
        "profile-analysis.json",
        "RESULTS.md",
        "schedule.json",
        "independent-review.json",
        "independent-review/review-final.json",
        "result-files.sha256",
    ] {
        protected.insert(p.to_string(), hash(&read(&root.join(p))?));
    }
    let original_qualification = protected["qualification.json"].clone();
    if original_qualification != "7a51e3a886b644c1b71ff829479439873eba06f2c6d52c38034159a5492f2c5e"
        || protected["independent-review.json"]
            != "6cea639201ac050b8086f09007470e2966f61681bbd56b408f4b3270afc54b31"
    {
        return Err("reviewed original bindings changed".into());
    }
    let mut total = 0usize;
    let mut records = Vec::new();
    let mut originals = BTreeMap::new();
    for p in &selected {
        let path = root.join(p);
        if path.canonicalize()? != path {
            return Err("selected path is not canonical/owned".into());
        }
        let b = read(&path)?;
        total = total.checked_add(b.len()).ok_or("length overflow")?;
        let s = std::str::from_utf8(&b)?;
        let mut tests = Vec::new();
        let mut summaries = Vec::new();
        let mut diagnostics = Vec::new();
        let mut build_units = Vec::new();
        let mut lines_other = Vec::new();
        let ls: Vec<_> = s.lines().collect();
        let mut in_failure = false;
        for (i, line) in ls.iter().enumerate() {
            if *line == "failures:" {
                in_failure = true;
            }
            if line.starts_with("test result:") {
                let f: Vec<_> = line.split_whitespace().collect();
                summaries.push(json!({"original_summary":line,"status":f[2].trim_end_matches('.'),"passed":f[3].parse::<u64>()?,"failed":f[5].parse::<u64>()?,"ignored":f[7].parse::<u64>()?,"measured":f[9].parse::<u64>()?,"filtered_out":f[11].parse::<u64>()?}));
            } else if let Some(t) = line.strip_prefix("test ") {
                if let Some((name, status)) = t.rsplit_once(" ... ") {
                    tests.push(json!({"name":name,"status":status,"original_line":line}));
                } else {
                    lines_other.push(json!({"line":i+1,"text":line}));
                }
            } else if line.trim_start().starts_with("Compiling ")
                || line.trim_start().starts_with("Checking ")
            {
                build_units.push(line.trim());
            } else if in_failure || line.starts_with("warning:") || line.starts_with("error:") {
                diagnostics.push(json!({"line":i+1,"text":line}));
            } else if !line.is_empty() {
                lines_other.push(json!({"line":i+1,"text":line}));
            }
        }
        if p.starts_with("measured/") && !b.is_empty() {
            return Err("unexpected nonempty measured stream".into());
        }
        if p.contains("red-")
            && (diagnostics.is_empty()
                || summaries
                    .iter()
                    .all(|s| s["failed"].as_u64().unwrap_or(0) == 0))
        {
            return Err("RED failure preservation incomplete".into());
        }
        let mut command = Value::Null;
        let mut command_evidence="Exact historical Cargo argv not retained; test binary invocation and original summaries preserved without inventing flags.".to_owned();
        let mut source_ref = Value::Null;
        let mut exit = Value::Null;
        let mut exit_evidence = Value::Null;
        match p.as_str() {
            "qualification/final-probe.log" => {
                command = json!([
                    "/Users/casey.allard/.cargo/bin/cargo",
                    "test",
                    "--locked",
                    "--offline",
                    "--all-targets",
                    "--features",
                    "hardware-sha256",
                    "--target-dir",
                    "target-qualification"
                ]);
                command_evidence="Exact argv confirmed by the author during this retirement review; final result/source also bound by original qualification.json.".into();
                source_ref = json!("build-diagnostic/helper-files.sha256");
                exit = json!(0);
                exit_evidence = json!("qualification/final-checks.exit");
            }
            "qualification/final-clippy.log" => {
                command = json!([
                    "/Users/casey.allard/.cargo/bin/cargo",
                    "clippy",
                    "--locked",
                    "--offline",
                    "--all-targets",
                    "--features",
                    "hardware-sha256",
                    "--target-dir",
                    "target-qualification",
                    "--",
                    "-D",
                    "warnings"
                ]);
                command_evidence =
                    "Exact argv confirmed by the author during this retirement review.".into();
                source_ref = json!("build-diagnostic/helper-files.sha256");
                exit = json!(0);
                exit_evidence = json!("qualification/final-checks.exit");
            }
            "qualification/final-fmt.log" => {
                command_evidence="Empty successful formatting transcript; exact historical argv unavailable. Final combined check exit remains retained.".into();
                source_ref = json!("build-diagnostic/helper-files.sha256");
                exit = json!(0);
                exit_evidence = json!("qualification/final-checks.exit");
            }
            "qualification/red-categories-initial.log" => {
                source_ref = json!("qualification/red-category-source.sha256");
                exit_evidence = json!("qualification/red-categories-initial.exit");
            }
            "qualification/red-categories.log" => {
                source_ref = json!("qualification/red-category-source-final.sha256");
                exit_evidence = json!("qualification/red-categories.exit");
            }
            "qualification/red-analyzer-safety.log" => {
                source_ref = json!("qualification/analyzer-before-hardening.rs");
                exit_evidence = json!("qualification/red-analyzer-safety.exit");
                command_evidence="Author confirms analyzer safety_ test filter against preserved pre-fix source; exact complete argv not retained.".into();
            }
            "qualification/green-snapshot.log" => {
                exit_evidence = json!("qualification/green-snapshot.exit");
            }
            "qualification/green-probe.log" => {
                exit_evidence = json!("qualification/green-probe.exit");
            }
            "qualification/snapshot-all-targets.log" | "qualification/clippy-snapshot.log" => {
                source_ref = json!("snapshot-established.sha256");
                exit_evidence = json!("qualification/full-checks.exit");
                command_evidence="Snapshot crate all-target tests / Clippy, hardware feature; exact complete argv not safely retained. Frozen source and binary invocations are preserved.".into();
            }
            "receipts/build-attempt-1.log" => {
                source_ref = json!("build-qualification-attempt-1/production-source-files.sha256");
                command = json!(std::str::from_utf8(&read(
                    &root.join("build-qualification-attempt-1/command.txt")
                )?)?);
                command_evidence="Exact release compiler invocation retained in build-qualification-attempt-1/command.txt; this build was not measured.".into();
                exit_evidence = json!("receipts/build-attempt-1.exit");
            }
            "receipts/build.log" => {
                source_ref = json!("build-diagnostic/production-source-files.sha256");
                command = json!(std::str::from_utf8(&read(
                    &root.join("build-diagnostic/command.txt")
                )?)?);
                command_evidence="Exact release compiler invocation retained in build-diagnostic/command.txt, wrapped by retained build.sh.".into();
                exit_evidence = json!("receipts/build.exit");
            }
            "receipts/run-profile.stdout" | "receipts/run-profile.stderr" => {
                command = json!(["bash", "run-profile.sh"]);
                command_evidence =
                    "Exact retained driver and measured/execution.jsonl bind all nine invocations."
                        .into();
                source_ref = json!("build-diagnostic/helper-files.sha256");
                exit_evidence = json!("receipts/run-profile.exit");
            }
            "receipts/template-preparation.stdout" | "receipts/template-preparation.stderr" => {
                command_evidence="Empty preparation streams; exact historical argv unavailable. Template setup scientific receipts remain in experiment/setup.jsonl.".into();
                exit_evidence = json!("receipts/template-preparation.exit");
            }
            _ => {}
        }
        if p.starts_with("measured/") {
            let stem = p.strip_prefix("measured/block").unwrap();
            let (block, rest) = stem.split_once('-').unwrap();
            let variant = rest.split('.').next().unwrap();
            command = json!([
                "build-diagnostic/receiver-probe",
                "run",
                "--root",
                format!("{ROOT}/experiment"),
                "--out",
                format!("{ROOT}/measured/block{block}-{variant}.jsonl"),
                "--block",
                block,
                "--variant",
                variant
            ]);
            command_evidence="Exact invocation reconstructed from frozen run-profile.sh and recorded schedule/execution identities, not inferred from timings.".into();
            exit = json!(0);
            exit_evidence = json!("measured/execution.jsonl");
            source_ref = json!("build-diagnostic/helper-files.sha256");
        }
        if let Some(e) = exit_evidence.as_str() {
            if e.ends_with(".exit") {
                exit = json!(std::str::from_utf8(&read(&root.join(e))?)?
                    .trim()
                    .parse::<i32>()?);
            }
        }
        let source_hash = source_ref
            .as_str()
            .map(|p| read(&root.join(p)).map(|b| hash(&b)))
            .transpose()?;
        records.push(json!({"historical_path":p,"original_sha256":hash(&b),"original_bytes":b.len(),"working_directory":ROOT,"command":command,"command_evidence":command_evidence,"exit_code":exit,"exit_evidence":exit_evidence,"test_cases":tests,"test_summaries":summaries,"failure_and_warning_details":diagnostics,"build_units":build_units,"other_evidence_lines":lines_other,"source_reference":source_ref,"source_reference_sha256":source_hash,"source_limit":"Intermediate logs without a named source reference are not individually frozen and do not replace final qualified source evidence."}));
        originals.insert(p.clone(), b);
    }
    if total != 38110 {
        return Err(format!("selected bytes differ:{total}").into());
    }
    let structured = json!({"schema":"spiralcore.fine-profile-structured-log-retirement.v1","source_revision":"d03fe1630bba787a28cc3fa3ae201c4a1a75388b","status":"reviewed_verification_text_preserved","original_qualification_sha256":original_qualification,"selected_files":37,"selected_bytes":total,"records":records,"protected_bindings":protected,"historical_binding_notice":"Original premeasurement manifests/qualification remain byte-identical. Retired text paths are historical bindings, not current existence assertions. No missing original is silently treated as present.","scope":"Only37 explicit owned .log/stdout/stderr files. No scientificJSONL,source,schedule,analysis,manifest,executable,template,host-observation,exit-code file or Cargo target directory selected."});
    if mode == "--check" {
        println!(
            "{}",
            json!({"status":"check_passed","writes":0,"deletions":0,"selected_files":selected,"selected_bytes":total,"prospective_structured_payload_sha256":hash(&serde_json::to_vec(&structured)?),"protected_bindings":protected})
        );
        return Ok(());
    }
    let review_path = root.join("independent-review/log-retirement-check.json");
    let check: Value = serde_json::from_slice(&read(&review_path)?)?;
    if check["status"] != "check_passed"
        || check["prospective_structured_payload_sha256"] != hash(&serde_json::to_vec(&structured)?)
    {
        return Err("missing or changed checked selection".into());
    }
    let structured_hash = durable(&root.join("structured-qualification.json"), &structured)?;
    let mut journal = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(root.join("independent-review/log-retirement-journal.jsonl"))?;
    journal.sync_all()?;
    File::open(root.join("independent-review"))?.sync_all()?;
    let mut removed = Vec::new();
    for p in &selected {
        let path = root.join(p);
        let now = read(&path)?;
        if now != originals[p] {
            return Err("selected log changed after durable preservation".into());
        }
        fs::remove_file(&path)?;
        File::open(path.parent().unwrap())?.sync_all()?;
        let r = json!({"historical_path":p,"original_sha256":hash(&now),"original_bytes":now.len(),"removed":true});
        writeln!(journal, "{r}")?;
        journal.sync_all()?;
        removed.push(r);
    }
    for (p, h) in &protected {
        if hash(&read(&root.join(p))?) != *h {
            return Err("protected binding changed".into());
        }
    }
    durable(
        &root.join("diagnostic-cleanup.json"),
        &json!({"schema":"spiralcore.fine-profile-diagnostic-cleanup.v1","status":"completed","removed_count":37,"removed_bytes":total,"removed":removed,"retained_structured_sha256":structured_hash,"original_qualification_sha256_unchanged":original_qualification,"protected_bindings_unchanged":protected,"source_sha256":hash(&read(&root.join("independent-review/retire-logs.rs"))?),"executable_sha256":hash(&read(&std::env::current_exe()?)?)}),
    )?;
    println!("Preserved37 structured records durably; retired37 explicit owned verification text files; all protected bindings unchanged.");
    Ok(())
}
