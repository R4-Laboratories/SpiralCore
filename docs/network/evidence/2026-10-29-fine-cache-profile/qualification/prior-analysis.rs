use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spiralcore_receiver_probe::{ordered_arms, POPULATIONS, SOURCE_REVISION, SOURCE_SHA256};
use spiralcore_transfer::{hex, scan_profile::ScanProfile};
use std::{collections::BTreeMap, fs, io::Write, path::Path};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn require(ok: bool, why: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(why.into())
    }
}
fn num(v: &Value, key: &str) -> Result<u64> {
    v[key]
        .as_u64()
        .ok_or_else(|| format!("missing u64 {key}").into())
}
fn digest(path: &Path) -> Result<String> {
    Ok(hex(&Sha256::digest(fs::read(path)?).into()))
}
fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    if s.len().is_multiple_of(2) {
        (s[s.len() / 2 - 1] + s[s.len() / 2]) / 2.0
    } else {
        s[s.len() / 2]
    }
}
fn summarize(values: &[f64]) -> Value {
    json!({"values":values,"midpoint_median":median(values),"minimum":values.iter().copied().reduce(f64::min),"maximum":values.iter().copied().reduce(f64::max)})
}
fn test_totals(text: &str) -> Result<(u64, u64)> {
    let (mut passed, mut failed, mut summaries) = (0, 0, 0);
    for line in text.lines().filter(|s| s.starts_with("test result:")) {
        let words: Vec<_> = line.split_whitespace().collect();
        require(
            words.len() >= 7 && words[4] == "passed;" && words[6] == "failed;",
            "invalid test summary",
        )?;
        passed += words[3].parse::<u64>()?;
        failed += words[5].parse::<u64>()?;
        summaries += 1;
    }
    require(summaries > 0, "no test summary")?;
    Ok((passed, failed))
}
fn qualify(root: &Path, out: &Path) -> Result<()> {
    require(
        root.is_absolute() && out.is_absolute() && !out.exists(),
        "new absolute qualification output required",
    )?;
    let mut evidence = BTreeMap::new();
    for (name, expected) in [
        ("red.log", (0, 3)),
        ("red-probe.log", (0, 1)),
        ("final-probe-2.log", (10, 0)),
        ("final-snapshot-2.log", (8, 0)),
    ] {
        let file = root.join("qualification").join(name);
        let counts = test_totals(&fs::read_to_string(&file)?)?;
        require(counts == expected, "test outcome/count mismatch")?;
        evidence.insert(
            name.to_string(),
            json!({"passed":counts.0,"failed":counts.1,"sha256":digest(&file)?}),
        );
    }
    require(
        fs::read_to_string(root.join("qualification/final.exit"))?.trim() == "0",
        "final qualification did not exit0",
    )?;
    let mut hashes = BTreeMap::new();
    for file in [
        "build-diagnostic/receiver-probe",
        "build-diagnostic/profile-analysis",
        "build-diagnostic/helper-files.sha256",
        "build-diagnostic/production-source-files.sha256",
        "build-diagnostic/metadata.json",
        "build-diagnostic/build.json",
        "snapshot-established.sha256",
        "schedule.json",
        "qualification/final-clippy.log",
        "qualification/final-snapshot-clippy.log",
        "qualification/final.exit",
        "receipts/snapshot-origin.tsv",
        "receipts/endpoint.patch",
        "receipts/module-export.patch",
        "receipts/probe.patch",
    ] {
        hashes.insert(file.to_string(), digest(&root.join(file))?);
    }
    let report = json!({"schema":"spiralcore.cache-profile-qualification.v1","status":"passed","source_revision":SOURCE_REVISION,"diagnostic_source_fingerprint_sha256":SOURCE_SHA256,"test_evidence":evidence,"hashes":hashes,"performance_measurement":"NOT_RUN","scope":"snapshot counter semantics, exact local API receipts and Rust analysis; no production changes or performance inference"});
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)?;
    serde_json::to_writer_pretty(&mut f, &report)?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    println!("Qualification bound: 10 helper/analysis tests and 8 snapshot unit tests; both runtime RED cases retained");
    Ok(())
}
fn run(dir: &Path, out: &Path) -> Result<()> {
    require(
        dir.is_absolute() && out.is_absolute(),
        "absolute paths required",
    )?;
    require(!out.exists(), "output already exists")?;
    let root = dir.parent().ok_or("measurement parent missing")?;
    let expected_exe = digest(&root.join("build-diagnostic/receiver-probe"))?;
    let mut groups: BTreeMap<String, BTreeMap<String, Vec<f64>>> = BTreeMap::new();
    let mut input_hashes = BTreeMap::new();
    let (mut observations, mut frame_count, mut scan_count, mut metadata_count, mut app_bytes) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    for block in 0..2 {
        let file = dir.join(format!("block{block}-diagnostic.jsonl"));
        input_hashes.insert(
            file.file_name().unwrap().to_string_lossy().to_string(),
            digest(&file)?,
        );
        let rows: Vec<Value> = fs::read_to_string(&file)?
            .lines()
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()?;
        require(
            rows.len() == 17,
            "block must contain metadata plus16 observations",
        )?;
        let m = &rows[0];
        require(
            m["kind"] == "metadata"
                && m["schema"] == "spiralcore.cache-scan-probe.v1"
                && m["library_revision"] == SOURCE_REVISION
                && m["library_source_sha256"] == SOURCE_SHA256
                && m["executable_sha256"] == expected_exe
                && m["block"] == block
                && m["variant"] == "diagnostic",
            "metadata/provenance mismatch",
        )?;
        let mut index = 1;
        for i in 0..4 {
            let population = POPULATIONS[(i + block) % 4];
            for arm in ordered_arms(block, population) {
                let r = &rows[index];
                index += 1;
                require(
                    r["kind"] == "observation"
                        && r["status"] == "exact"
                        && r["error"].is_null()
                        && r["error_stage"] == ""
                        && r["block"] == block
                        && r["population"] == population
                        && r["arm_label"] == arm.label()
                        && r["variant"] == "diagnostic",
                    "failed/unexpected/reordered observation",
                )?;
                require(
                    r["base_object"]
                        == "c7fceaf53550e0ab1393604c2bc88eabd267676766ae90ad52459ece381822c0"
                        && r["target_object"]
                            == "227bbf10e522740c837d03da96838331a00fe3dac2ca77c095ba1044467b0ebf"
                        && r["object_bytes"] == 1048576,
                    "fixture identity mismatch",
                )?;
                let before = &r["before"];
                let after = &r["after"];
                let n = population as u64;
                require(
                    num(before, "publications")? == n + 1
                        && num(before, "namespaces")? == n / 2 + 1
                        && num(before, "active_publications")? == 1
                        && num(after, "publications")? == n + 2
                        && num(after, "namespaces")? == n / 2 + 1
                        && num(after, "active_publications")? == 2,
                    "inventory mismatch",
                )?;
                require(
                    r["target_manifest_present"] == (!arm.sparse && !arm.tiny)
                        && r["graph_cache_after"]["entries"] == 0
                        && r["graph_cache_after"]["resident_bytes"] == 0,
                    "readiness/retention mismatch",
                )?;
                let frames = r["frame_samples"].as_array().ok_or("frames missing")?;
                require(
                    frames.len() == if arm.sparse { 3 } else { 20 },
                    "frame count mismatch",
                )?;
                let (mut scpu, mut swall, mut hcpu, mut hwall, mut bytes) =
                    (0u64, 0u64, 0u64, 0u64, 0u64);
                let mut stage_sums = [[0u64; 3]; 3];
                let mut stage_scan = [[0u64; 2]; 3];
                for (sequence, f) in frames.iter().enumerate() {
                    let kind = if sequence == 0 {
                        1
                    } else if sequence == frames.len() - 1 {
                        3
                    } else {
                        2
                    };
                    require(
                        num(f, "kind")? == kind
                            && num(f, "sequence")? == sequence as u64
                            && num(f, "reply_kind")? == kind + 100,
                        "frame shape mismatch",
                    )?;
                    let p = &f["disk_usage"];
                    let cpu = num(&f["cost"], "cpu_ns")?;
                    let wall = num(&f["cost"], "wall_ns")?;
                    let scan_cpu = num(p, "cpu_ns")?;
                    let scan_wall = num(p, "wall_ns")?;
                    require(scan_cpu <= cpu && scan_wall <= wall, "scan exceeds handler")?;
                    if kind == 2 {
                        require(
                            *p == serde_json::to_value(ScanProfile::default())?,
                            "Data has scan",
                        )?;
                    } else {
                        require(
                            p["overflow"] == false
                                && p["last_error"].is_null()
                                && p["last_error_stage"].is_null(),
                            "scan error/overflow",
                        )?;
                        for (key, expected) in [
                            ("calls", 1),
                            ("successes", 1),
                            ("errors", 0),
                            ("root_read_dir_calls", 1),
                            ("root_entries", n / 2 + 3),
                            ("root_file_type_calls", n / 2 + 3),
                            ("namespace_read_dir_calls", n / 2 + 1),
                            ("namespace_entries", n + 1),
                            ("object_file_type_calls", n + 1),
                            ("metadata_calls", n + 1),
                            ("metadata_successes", n + 1),
                            ("namespace_comparisons", n + 1),
                            ("objects_counted", n + 1),
                            ("returned_objects", n + 1),
                            ("returned_scoped_bytes", num(before, "active_bytes")?),
                            ("returned_all_bytes", num(before, "publication_bytes")?),
                        ] {
                            require(
                                num(p, key)? == expected,
                                &format!("scan counter mismatch: {key}"),
                            )?;
                        }
                        scan_count += 1;
                        metadata_count += n + 1;
                    }
                    scpu += scan_cpu;
                    swall += scan_wall;
                    hcpu += cpu;
                    hwall += wall;
                    bytes += num(f, "request_bytes")? + num(f, "reply_bytes")?;
                    stage_sums[(kind - 1) as usize][0] += cpu;
                    stage_sums[(kind - 1) as usize][1] += wall;
                    stage_sums[(kind - 1) as usize][2] += 1;
                    stage_scan[(kind - 1) as usize][0] += scan_cpu;
                    stage_scan[(kind - 1) as usize][1] += scan_wall;
                }
                require(
                    bytes == if arm.sparse { 536 } else { 1051138 },
                    "modeled application bytes changed",
                )?;
                for (s, label) in ["begin", "data", "finish"].iter().enumerate() {
                    let stage = &r["handler_stages"][label];
                    require(
                        num(stage, "cpu_ns")? == stage_sums[s][0]
                            && num(stage, "wall_ns")? == stage_sums[s][1]
                            && num(stage, "frames")? == stage_sums[s][2],
                        "stage sums mismatch",
                    )?;
                }
                require(
                    num(&r["handler_stages"]["total"], "cpu_ns")? == hcpu
                        && num(&r["handler_stages"]["total"], "wall_ns")? == hwall,
                    "handler sum mismatch",
                )?;
                let ocpu = num(&r["local_operation_total"], "cpu_ns")?;
                let owall = num(&r["local_operation_total"], "wall_ns")?;
                require(
                    ocpu >= hcpu
                        && owall >= hwall
                        && ocpu > 0
                        && hcpu > 0
                        && owall > 0
                        && hwall > 0,
                    "invalid containing clock",
                )?;
                let group = groups
                    .entry(format!("N{population}/{}", arm.label()))
                    .or_default();
                for (name, value) in [
                    ("scan_cpu_ns", scpu),
                    ("scan_wall_ns", swall),
                    ("handler_cpu_ns", hcpu),
                    ("handler_wall_ns", hwall),
                    ("operation_cpu_ns", ocpu),
                    ("operation_wall_ns", owall),
                    ("scan_begin_cpu_ns", stage_scan[0][0]),
                    ("scan_finish_cpu_ns", stage_scan[2][0]),
                    ("scan_begin_wall_ns", stage_scan[0][1]),
                    ("scan_finish_wall_ns", stage_scan[2][1]),
                    (
                        "preparation_cpu_ns",
                        num(&r["frame_preparation"], "cpu_ns")?,
                    ),
                    ("modeled_application_bytes", bytes),
                ] {
                    group.entry(name.into()).or_default().push(value as f64);
                }
                for (name, value) in [
                    ("scan_fraction_of_handler_cpu", scpu as f64 / hcpu as f64),
                    ("scan_fraction_of_operation_cpu", scpu as f64 / ocpu as f64),
                    ("scan_fraction_of_handler_wall", swall as f64 / hwall as f64),
                    (
                        "scan_fraction_of_operation_wall",
                        swall as f64 / owall as f64,
                    ),
                ] {
                    group.entry(name.into()).or_default().push(value);
                }
                observations += 1;
                frame_count += frames.len() as u64;
                app_bytes += bytes;
            }
        }
    }
    require(
        observations == 32
            && groups.len() == 16
            && scan_count == 64
            && frame_count == 368
            && metadata_count == 17472
            && app_bytes == 16826784,
        "campaign accounting mismatch",
    )?;
    let cells: BTreeMap<_, _> = groups
        .into_iter()
        .map(|(k, metrics)| {
            let values: BTreeMap<_, _> = metrics
                .into_iter()
                .map(|(name, v)| (name, summarize(&v)))
                .collect();
            (k, values)
        })
        .collect();
    let report = json!({"schema":"spiralcore.cache-profile-analysis.v1","source_revision":SOURCE_REVISION,"diagnostic_source_fingerprint_sha256":SOURCE_SHA256,"receiver_executable_sha256":expected_exe,"input_hashes":input_hashes,"observations":observations,"exact":observations,"failures":0,"frames":frame_count,"scans":scan_count,"metadata_api_calls":metadata_count,"modeled_application_bytes":app_bytes,"cells":cells,"limitations":["two observations per cell, descriptive attribution only","midpoint medians, raw values in block order retained","nested process CPU and wall clocks; scan overhead retained, not subtracted","not a speed comparison with prior uninstrumented data","API calls are not operating-system syscall counts","OS caches and host scheduling uncontrolled","no network or energy measurement"]});
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)?;
    serde_json::to_writer_pretty(&mut file, &report)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    println!("Validated {observations} exact observations, {scan_count} scans, {metadata_count} metadata API calls");
    Ok(())
}
fn main() {
    let a: Vec<_> = std::env::args_os().collect();
    let result = if a.len() == 4 && a[1] == "--qualify" {
        qualify(Path::new(&a[2]), Path::new(&a[3]))
    } else if a.len() == 3 {
        run(Path::new(&a[1]), Path::new(&a[2]))
    } else {
        Err("usage: profile-analysis [--qualify] ABS_INPUT_DIR ABS_NEW_OUTPUT".into())
    };
    if let Err(e) = result {
        eprintln!("profile-analysis: {e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn two_observations_use_midpoint_and_preserve_order() {
        assert_eq!(median(&[11.0, 3.0]), 7.0);
        assert_eq!(summarize(&[11.0, 3.0])["values"], json!([11.0, 3.0]));
        assert_eq!(median(&[2.0, 0.0, 5.0]), 2.0);
    }
    #[test]
    fn malformed_counters_do_not_become_zero_or_success() {
        assert!(num(&json!({}), "calls").is_err());
        assert!(num(&json!({"calls":-1}), "calls").is_err());
        assert!(require(false, "failed observation").is_err());
        assert!(test_totals("nothing ran").is_err());
        assert_eq!(test_totals("test result: ok. 8 passed; 0 failed; 0 ignored;\ntest result: ok. 2 passed; 0 failed; 0 ignored;").unwrap(), (10,0));
    }
}
