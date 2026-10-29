//! Post-review diagnostic cleanup. This never executes the endpoint or a benchmark.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs::{self, File, OpenOptions}, io::Write, path::{Path, PathBuf}};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn sha(bytes: &[u8]) -> String { Sha256::digest(bytes).iter().map(|b|format!("{b:02x}")).collect() }
fn write(path: &Path, value: &Value) -> Result<()> {
    let mut file=OpenOptions::new().write(true).create_new(true).open(path)?;
    serde_json::to_writer_pretty(&mut file,value)?; file.write_all(b"\n")?; file.sync_all()?;
    File::open(path.parent().ok_or("missing output parent")?)?.sync_all()?;
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len()!=2 || !matches!(args[1].as_str(), "--check" | "--apply") {return Err("usage: cleanup-receipts --check|--apply".into());}
    let root=PathBuf::from("/Users/casey.allard/spiralcore/local/cache-profile-20261029");
    let packet=PathBuf::from("/Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore/docs/network/evidence/2026-10-29-cache-profile");
    let cargo="/Users/casey.allard/.cargo/bin/cargo";
    let hardware="--features hardware-sha256 --target-dir target-qualification";
    let snapshot="--manifest-path snapshot/crates/spiralcore-transfer/Cargo.toml";
    let probe_tests=format!("{cargo} test --locked --offline --all-targets {hardware}");
    let snapshot_tests=format!("{cargo} test --locked --offline {snapshot} --lib {hardware}");
    let probe_clippy=format!("{cargo} clippy --locked --offline --all-targets {hardware} -- -D warnings");
    let snapshot_clippy=format!("{cargo} clippy --locked --offline {snapshot} --lib --tests {hardware} -- -D warnings");
    let entries: Vec<(&str,String,i32,&str)> = vec![
        ("qualification/red.log",format!("{cargo} test --locked --offline {snapshot} --lib scan_diagnostic_tests {hardware}"),101,"qualification/red-source.sha256"),
        ("qualification/green-counter.log",format!("{cargo} test --locked --offline {snapshot} --lib diagnostic {hardware}"),0,"intermediate-not-individually-source-frozen"),
        ("qualification/red-probe.log",format!("{cargo} test --locked --offline --lib actual_api_receipts {hardware}"),101,"qualification/red-probe-source.sha256"),
        ("qualification/green-probe.log",format!("{cargo} test --locked --offline --lib {hardware}"),0,"intermediate-not-individually-source-frozen"),
        ("qualification/final-probe.log",probe_tests.clone(),0,"intermediate-not-individually-source-frozen"),
        ("qualification/probe-before-binder.log",probe_tests.clone(),0,"intermediate-not-individually-source-frozen"),
        ("qualification/final-probe-2.log",probe_tests,0,"build-diagnostic/helper-files.sha256"),
        ("qualification/final-snapshot.log",snapshot_tests.clone(),0,"intermediate-not-individually-source-frozen"),
        ("qualification/final-snapshot-2.log",snapshot_tests,0,"snapshot-established.sha256"),
        ("qualification/clippy-attempt-1.log",probe_clippy.clone(),101,"intermediate-not-individually-source-frozen"),
        ("qualification/clippy-before-binder.log",probe_clippy.clone(),0,"intermediate-not-individually-source-frozen"),
        ("qualification/final-clippy.log",probe_clippy,0,"build-diagnostic/helper-files.sha256"),
        ("qualification/snapshot-clippy-attempt-1.log",snapshot_clippy.clone(),101,"intermediate-not-individually-source-frozen"),
        ("qualification/final-snapshot-clippy.log",snapshot_clippy,0,"snapshot-established.sha256"),
        ("qualification/final-fmt.log",format!("{cargo} fmt --all -- --check"),0,"build-diagnostic/helper-files.sha256"),
        ("receipts/build.log","bash build.sh; exact compiler invocation retained in build-diagnostic/command.txt".into(),0,"build-diagnostic/production-source-files.sha256"),
    ];
    let mut records=Vec::new();
    let mut delete=Vec::new();
    for (relative,command,exit,source) in entries {
        let path=root.join(relative);
        let bytes=fs::read(&path)?;
        let content=std::str::from_utf8(&bytes)?;
        let lines: Vec<_>=content.lines().collect();
        let mut tests=Vec::new(); let mut summaries=Vec::new(); let mut diagnostics=Vec::new();
        for (i,line) in lines.iter().enumerate() {
            if line.starts_with("test result:") {
                let words: Vec<_>=line.split_whitespace().collect();
                summaries.push(json!({"status":words[2].trim_end_matches('.'),"passed":words[3].parse::<u64>()?,"failed":words[5].parse::<u64>()?,"original_summary":line}));
            } else if line.starts_with("test ") && (line.ends_with(" ... ok") || line.ends_with(" ... FAILED")) {
                let (name,status)=line.strip_prefix("test ").unwrap().rsplit_once(" ... ").ok_or("invalid test case")?;
                tests.push(json!({"name":name,"status":status}));
            }
            if line.starts_with("thread '") && line.contains("panicked") {
                diagnostics.push(json!({"kind":"test_assertion","lines":&lines[i..(i+6).min(lines.len())]}));
            } else if line.starts_with("error:") && !line.starts_with("error: could not compile") && !line.starts_with("error: test failed") {
                diagnostics.push(json!({"kind":"compiler_or_lint","lines":&lines[i..(i+14).min(lines.len())]}));
            }
        }
        if exit != 0 && diagnostics.is_empty() { return Err(format!("failure has no retained diagnostic: {relative}").into()); }
        if relative.contains("red") && summaries.iter().all(|s| s["failed"].as_u64().unwrap_or(0)==0) { return Err("RED result missing actual failures".into()); }
        let source_hash=if source.starts_with("intermediate") {None} else {Some(sha(&fs::read(root.join(source))?))};
        records.push(json!({"historical_path":relative,"original_sha256":sha(&bytes),"original_bytes":bytes.len(),"command":command,"working_directory":root,"exit_code":exit,"test_cases":tests,"test_summaries":summaries,"failure_details":diagnostics,"source_manifest":source,"source_manifest_sha256":source_hash,"source_limit":"Intermediate setup executions are labeled explicitly; only final source/build/test bindings establish frozen qualification."}));
        let copy=packet.join(relative);
        if copy.exists() { if fs::read(&copy)?!=bytes {return Err("packet log differs before cleanup".into());} delete.push(copy); }
        delete.push(path);
    }
    for block in 0..2 {
        for stream in ["stdout","stderr"] {
            let relative=format!("measured/block{block}-diagnostic.{stream}");
            let path=root.join(&relative); let bytes=fs::read(&path)?;
            if !bytes.is_empty() {return Err("nonempty campaign stream needs explicit preservation".into());}
            records.push(json!({"historical_path":relative,"original_sha256":sha(&bytes),"original_bytes":0,"command":format!("build-diagnostic/receiver-probe run --root {}/experiment --out {}/measured/block{block}-diagnostic.jsonl --block {block} --variant diagnostic",root.display(),root.display()),"working_directory":root,"exit_code":0,"recorded_exit_evidence":"measured/execution.jsonl","diagnostic_stream":stream}));
            delete.push(path);
        }
    }
    let original_binding=sha(&fs::read(root.join("qualification.json"))?);
    if original_binding!="f02cbb5722e69553567c10636d3754fea82a564431c0668d321847585e7a3295" || sha(&fs::read(packet.join("qualification.json"))?)!=original_binding {return Err("original qualification binding differs".into());}
    if records.len()!=20 || delete.len()!=29 {return Err("cleanup selection differs from reviewed20 local/9 packet files".into());}
    let report=json!({"schema":"spiralcore.cache-profile-structured-qualification.v1","source_revision":"f0966ebedcb8994f0985b54e9a2b961eeb8be914","original_qualification_sha256":original_binding,"status":"reviewed_diagnostic_logs_retired","records":records,"frozen_binding_status":"Original premeasurement receipts remain unchanged. Removed diagnostic paths and hashes are historical. Rerunning their former checksum/qualification commands will fail until a new explicitly registered qualification is generated; do not rewrite old bindings.","scientific_evidence":"All raw JSONL, analysis, sources, manifests, test source hashes, fixture/template/binary identities and independent reviews remain."});
    if args[1]=="--check" {println!("Read-only qualification passed:20 structured records,29 owned diagnostic files; no writes/deletions"); return Ok(());}
    write(&root.join("structured-qualification.json"),&report)?;
    write(&packet.join("structured-qualification.json"),&report)?;
    let mut removed=Vec::new();
    for path in delete { let bytes=fs::read(&path)?; let hash=sha(&bytes); fs::remove_file(&path)?; removed.push(json!({"path":path,"bytes":bytes.len(),"sha256":hash})); }
    if sha(&fs::read(root.join("qualification.json"))?)!=original_binding {return Err("historical qualification was changed".into());}
    let cleanup=json!({"schema":"spiralcore.cache-profile-diagnostic-cleanup.v1","removed":removed,"removed_count":29,"retained_structured_sha256":sha(&fs::read(root.join("structured-qualification.json"))?),"original_qualification_sha256_unchanged":original_binding,"scope":"20 local owned diagnostic files plus9 identical packet copies; no scientific JSONL/source/identity removed"});
    write(&root.join("diagnostic-cleanup.json"),&cleanup)?;
    write(&packet.join("diagnostic-cleanup.json"),&cleanup)?;
    println!("Retained20 structured diagnostic records; removed29 owned files; original binding unchanged");
    Ok(())
}
