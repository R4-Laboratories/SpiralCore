use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spiralcore_receiver_probe::{LAYOUTS, SOURCE_REVISION, SOURCE_SHA256};
use spiralcore_transfer::{
    hex,
    scan_profile::{ScanProfile, CATEGORY_NAMES},
};
use std::{collections::BTreeMap, fs, io::Write, path::Path};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const VARIANTS: [&str; 3] = ["counters", "fine", "empty"];
const TEXT_LIMIT:u64=16*1024*1024;
fn read_text(path:&Path,_limit:u64)->Result<String>{Ok(fs::read_to_string(path)?)}
fn checked_sum(values:impl IntoIterator<Item=u64>)->Result<u64>{Ok(values.into_iter().fold(0,u64::wrapping_add))}
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
        .ok_or_else(|| format!("missing unsigned {key}").into())
}
fn digest(p: &Path) -> Result<String> {
    Ok(hex(&Sha256::digest(fs::read(p)?).into()))
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
fn summary(v: &[f64]) -> Value {
    json!({"values":v,"median":median(v),"minimum":v.iter().copied().reduce(f64::min),"maximum":v.iter().copied().reduce(f64::max)})
}
fn write(p: &Path, v: &Value) -> Result<()> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(p)?;
    serde_json::to_writer_pretty(&mut f, v)?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    Ok(())
}
fn screen(fine: &[f64], empty: &[f64], counter_op: &[f64], fine_op: &[f64]) -> Value {
    let paired_inflation: Vec<_> = fine_op.iter().zip(counter_op).map(|(f, c)| f / c).collect();
    let fractions: Vec<_> = fine.iter().zip(counter_op).map(|(f, c)| f / c).collect();
    let empty_median = median(empty);
    let fine_median = median(fine);
    let consistent = fine.iter().zip(empty).all(|(f, e)| f > e);
    let resolved = empty_median > 0.0 && fine_median >= 4.0 * empty_median;
    let material = median(&fractions) >= 0.10;
    let overhead = median(&paired_inflation) <= 1.25;
    json!({"fine_cpu_ns":fine,"empty_cpu_ns":empty,"fine_over_counter_operation_cpu":summary(&fractions),"fine_operation_over_counter_operation_cpu":summary(&paired_inflation),"all_three_fine_spans_exceed_empty":consistent,"median_fine_at_least_four_times_positive_empty":resolved,"median_fine_span_at_least_ten_percent_counter_operation":material,"median_instrumentation_inflation_at_most_twenty_five_percent":overhead,"candidate_lead_only":consistent&&resolved&&material&&overhead,"production_candidate_accepted":false})
}
fn test_counts(text: &str) -> Result<(u64, u64, u64)> {
    let (mut p, mut f, mut i, mut count) = (0, 0, 0, 0);
    for line in text.lines().filter(|s| s.starts_with("test result:")) {
        let w: Vec<_> = line.split_whitespace().collect();
        require(
            w.len() >= 9 && w[4] == "passed;" && w[6] == "failed;" && w[8] == "ignored;",
            "malformed test summary",
        )?;
        p += w[3].parse::<u64>()?;
        f += w[5].parse::<u64>()?;
        i += w[7].parse::<u64>()?;
        count += 1;
    }
    require(count > 0, "no executed test summaries")?;
    Ok((p, f, i))
}
fn qualify(root: &Path, out: &Path) -> Result<()> {
    require(
        root.is_absolute() && out.is_absolute() && !out.exists(),
        "new absolute qualification paths required",
    )?;
    let mut tests = BTreeMap::new();
    for (file, expected) in [
        ("qualification/red-categories.log", (1, 3, 0)),
        ("qualification/final-probe.log", (13, 0, 0)),
        ("qualification/snapshot-all-targets.log", (136, 0, 1)),
    ] {
        let path = root.join(file);
        let counts = test_counts(&fs::read_to_string(&path)?)?;
        require(counts == expected, "test counts/status differ")?;
        tests.insert(
            file,
            json!({"passed":counts.0,"failed":counts.1,"ignored":counts.2,"sha256":digest(&path)?}),
        );
    }
    require(
        fs::read_to_string(root.join("qualification/final-checks.exit"))?.trim() == "0",
        "qualification failed",
    )?;
    let setup_path = root.join("experiment/setup.jsonl");
    let setups: Vec<Value> = fs::read_to_string(&setup_path)?
        .lines()
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()?;
    require(setups.len() == 4, "need four closed template receipts")?;
    for (s, l) in setups.iter().zip(LAYOUTS) {
        require(
            s["layout"]["name"] == l.name
                && num(&s["inventory"], "publications")? == l.objects as u64 + 1
                && num(&s["inventory"], "namespaces")? == l.namespaces as u64 + 1
                && num(&s["inventory"], "manifests")? == l.objects as u64 + 1
                && s["base_object"]
                    == "c7fceaf53550e0ab1393604c2bc88eabd267676766ae90ad52459ece381822c0",
            "template shape/identity mismatch",
        )?;
    }
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
        "templates-established.sha256",
        "experiment/setup.jsonl",
        "qualification/final-checks.exit",
        "qualification/final-clippy.log",
        "qualification/clippy-snapshot.log",
        "qualification/setup-findings.json",
        "qualification/source-review.json",
        "qualification/snapshot-origin.tsv",
    ] {
        hashes.insert(file, digest(&root.join(file))?);
    }
    write(
        out,
        &json!({"schema":"spiralcore.fine-profile-qualification.v1","status":"passed","performance_measurement":"NOT_RUN","source_revision":SOURCE_REVISION,"source_fingerprint":SOURCE_SHA256,"tests":tests,"hashes":hashes,"non_utf8_filesystem_case":"NOT_RUN: this host rejects invalidUTF8 filenames; pure Path classification tested separately"}),
    )?;
    println!("Qualified13 helper/analysis tests,136 snapshot tests,4 closed templates;1 existing release-only local HTTP test ignored");
    Ok(())
}
fn analyze(dir: &Path, out: &Path) -> Result<()> {
    require(
        dir.is_absolute() && out.is_absolute() && !out.exists(),
        "new absolute analysis paths required",
    )?;
    let root = dir.parent().ok_or("missing experiment root")?;
    let expected_exe = digest(&root.join("build-diagnostic/receiver-probe"))?;
    let mut hashes = BTreeMap::new();
    let mut groups: BTreeMap<String, BTreeMap<String, Vec<f64>>> = BTreeMap::new();
    let (mut observations, mut frames_count, mut scan_count, mut metadata_count, mut bytes_total) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    for block in 0..3 {
        for variant in VARIANTS {
            let name = format!("block{block}-{variant}.jsonl");
            let path = dir.join(&name);
            hashes.insert(name, digest(&path)?);
            let rows: Vec<Value> = fs::read_to_string(&path)?
                .lines()
                .map(serde_json::from_str)
                .collect::<std::result::Result<_, _>>()?;
            require(rows.len() == 9, "metadata plus8 observations required")?;
            let meta = &rows[0];
            require(
                meta["kind"] == "metadata"
                    && meta["schema"] == "spiralcore.fine-cache-scan-probe.v1"
                    && meta["library_revision"] == SOURCE_REVISION
                    && meta["library_source_sha256"] == SOURCE_SHA256
                    && meta["executable_sha256"] == expected_exe
                    && meta["block"] == block
                    && meta["variant"] == variant
                    && meta["categories"] == json!(CATEGORY_NAMES),
                "metadata/provenance mismatch",
            )?;
            let mut row_index = 1;
            for i in 0..4 {
                let layout = LAYOUTS[(i + block) % 4];
                for j in 0..2 {
                    let sparse = (j + block + i) % 2 == 1;
                    let arm = if sparse { "sparse" } else { "raw" };
                    let r = &rows[row_index];
                    row_index += 1;
                    require(
                        r["kind"] == "observation"
                            && r["status"] == "exact"
                            && r["error"].is_null()
                            && r["error_stage"] == ""
                            && r["block"] == block
                            && r["variant"] == variant
                            && r["population"] == layout.objects
                            && r["background_namespaces"] == layout.namespaces
                            && r["arm"]["sparse"] == sparse
                            && r["arm"]["tiny"] == false,
                        "failed or unexpected observation",
                    )?;
                    require(r["base_object"]=="c7fceaf53550e0ab1393604c2bc88eabd267676766ae90ad52459ece381822c0"&&r["target_object"]=="227bbf10e522740c837d03da96838331a00fe3dac2ca77c095ba1044467b0ebf"&&r["object_bytes"]==1048576,"fixture identity mismatch")?;
                    let before = &r["before"];
                    let after = &r["after"];
                    let objects = layout.objects as u64 + 1;
                    let namespaces = layout.namespaces as u64 + 1;
                    require(
                        num(before, "publications")? == objects
                            && num(before, "namespaces")? == namespaces
                            && num(before, "active_publications")? == 1
                            && num(after, "publications")? == objects + 1
                            && num(after, "namespaces")? == namespaces
                            && num(after, "active_publications")? == 2
                            && r["target_manifest_present"] == !sparse
                            && r["graph_cache_after"]["entries"] == 0
                            && r["graph_cache_after"]["resident_bytes"] == 0,
                        "inventory/readiness mismatch",
                    )?;
                    let frames = r["frame_samples"].as_array().ok_or("missing frames")?;
                    require(
                        frames.len() == if sparse { 3 } else { 20 },
                        "wrong frame count",
                    )?;
                    let (mut cpu, mut wall, mut scan_cpu, mut scan_wall, mut bytes) =
                        (0u64, 0u64, 0u64, 0u64, 0u64);
                    let mut stages = [[0u64; 3]; 3];
                    let mut spans = [[0u64; 3]; 6];
                    for (seq, f) in frames.iter().enumerate() {
                        let kind = if seq == 0 {
                            1
                        } else if seq == frames.len() - 1 {
                            3
                        } else {
                            2
                        };
                        require(
                            num(f, "kind")? == kind
                                && num(f, "sequence")? == seq as u64
                                && num(f, "reply_kind")? == kind + 100,
                            "frame shape mismatch",
                        )?;
                        let fc = num(&f["cost"], "cpu_ns")?;
                        let fw = num(&f["cost"], "wall_ns")?;
                        let p = &f["disk_usage"];
                        let sc = num(p, "cpu_ns")?;
                        let sw = num(p, "wall_ns")?;
                        require(sc <= fc && sw <= fw, "scan clock exceeds handler")?;
                        if kind == 2 {
                            require(
                                *p == serde_json::to_value(ScanProfile::default())?,
                                "Data has scan",
                            )?;
                        } else {
                            require(
                                p["overflow"] == false
                                    && p["last_error"].is_null()
                                    && p["last_error_stage"].is_null()
                                    && p["mode"] == variant,
                                "scan error/mode mismatch",
                            )?;
                            for (key, value) in [
                                ("calls", 1),
                                ("successes", 1),
                                ("errors", 0),
                                ("root_read_dir_calls", 1),
                                ("root_entries", namespaces + 2),
                                ("root_file_type_calls", namespaces + 2),
                                ("namespace_read_dir_calls", namespaces),
                                ("namespace_entries", objects),
                                ("object_file_type_calls", objects),
                                ("metadata_calls", objects),
                                ("metadata_successes", objects),
                                ("namespace_comparisons", objects),
                                ("objects_counted", objects),
                                ("returned_objects", objects),
                                ("returned_scoped_bytes", num(before, "active_bytes")?),
                                ("returned_all_bytes", num(before, "publication_bytes")?),
                            ] {
                                require(num(p, key)? == value, &format!("counter mismatch {key}"))?;
                            }
                            let expected = [
                                3 * namespaces + objects + 4,
                                namespaces + objects + 2,
                                namespaces,
                                objects,
                                objects,
                                objects,
                            ];
                            let values = p["spans"].as_array().ok_or("missing spans")?;
                            require(values.len() == 6, "six spans required")?;
                            let (mut sum_cpu, mut sum_wall) = (0u64, 0u64);
                            for (k, s) in values.iter().enumerate() {
                                let calls = num(s, "calls")?;
                                let c = num(s, "cpu_ns")?;
                                let w = num(s, "wall_ns")?;
                                require(calls == expected[k], "category count mismatch")?;
                                if variant == "counters" {
                                    require(c == 0 && w == 0, "counter-only contains fine clocks")?;
                                }
                                spans[k][0] += calls;
                                spans[k][1] += c;
                                spans[k][2] += w;
                                sum_cpu += c;
                                sum_wall += w;
                            }
                            require(
                                sum_cpu <= sc && sum_wall <= sw,
                                "fine clocks exceed coarse scan",
                            )?;
                            scan_count += 1;
                            metadata_count += objects;
                        }
                        cpu += fc;
                        wall += fw;
                        scan_cpu += sc;
                        scan_wall += sw;
                        bytes += num(f, "request_bytes")? + num(f, "reply_bytes")?;
                        stages[(kind - 1) as usize][0] += fc;
                        stages[(kind - 1) as usize][1] += fw;
                        stages[(kind - 1) as usize][2] += 1;
                    }
                    require(
                        bytes == if sparse { 536 } else { 1051138 },
                        "application byte count changed",
                    )?;
                    for (k, name) in ["begin", "data", "finish"].iter().enumerate() {
                        let s = &r["handler_stages"][name];
                        require(
                            num(s, "cpu_ns")? == stages[k][0]
                                && num(s, "wall_ns")? == stages[k][1]
                                && num(s, "frames")? == stages[k][2],
                            "stage sums mismatch",
                        )?;
                    }
                    require(
                        num(&r["handler_stages"]["total"], "cpu_ns")? == cpu
                            && num(&r["handler_stages"]["total"], "wall_ns")? == wall,
                        "handler totals mismatch",
                    )?;
                    let oc = num(&r["local_operation_total"], "cpu_ns")?;
                    let ow = num(&r["local_operation_total"], "wall_ns")?;
                    require(
                        oc >= cpu && ow >= wall && oc > 0,
                        "operation clock mismatch",
                    )?;
                    let metrics = groups
                        .entry(format!("{}/{arm}/{variant}", layout.name))
                        .or_default();
                    let mut add = |name: &str, value: f64| {
                        metrics.entry(name.into()).or_default().push(value);
                    };
                    for (name, value) in [
                        ("operation_cpu_ns", oc),
                        ("operation_wall_ns", ow),
                        ("handler_cpu_ns", cpu),
                        ("handler_wall_ns", wall),
                        ("scan_cpu_ns", scan_cpu),
                        ("scan_wall_ns", scan_wall),
                        ("modeled_application_bytes", bytes),
                    ] {
                        add(name, value as f64);
                    }
                    let total_fine_cpu: u64 = spans.iter().map(|s| s[1]).sum();
                    let total_fine_wall: u64 = spans.iter().map(|s| s[2]).sum();
                    add("scan_unspanned_cpu_ns", (scan_cpu - total_fine_cpu) as f64);
                    add(
                        "scan_unspanned_wall_ns",
                        (scan_wall - total_fine_wall) as f64,
                    );
                    add(
                        "scan_fraction_of_operation_cpu",
                        scan_cpu as f64 / oc as f64,
                    );
                    for (k, name) in CATEGORY_NAMES.iter().enumerate() {
                        for (suffix, index) in [("calls", 0), ("cpu_ns", 1), ("wall_ns", 2)] {
                            add(&format!("{name}_{suffix}"), spans[k][index] as f64);
                        }
                    }
                    observations += 1;
                    frames_count += frames.len() as u64;
                    bytes_total += bytes;
                }
            }
        }
    }
    require(
        observations == 72
            && groups.len() == 24
            && frames_count == 828
            && scan_count == 144
            && metadata_count == 57744
            && bytes_total == 37860264,
        "whole campaign accounting mismatch",
    )?;
    let mut controls = BTreeMap::new();
    for layout in LAYOUTS {
        for arm in ["raw", "sparse"] {
            let counter = &groups[&format!("{}/{arm}/counters", layout.name)];
            let fine = &groups[&format!("{}/{arm}/fine", layout.name)];
            let empty = &groups[&format!("{}/{arm}/empty", layout.name)];
            let c = &counter["operation_cpu_ns"];
            let mut control = BTreeMap::new();
            for variant in ["fine", "empty"] {
                let v = &groups[&format!("{}/{arm}/{variant}", layout.name)];
                for metric in [
                    "operation_cpu_ns",
                    "operation_wall_ns",
                    "scan_cpu_ns",
                    "scan_wall_ns",
                ] {
                    require(
                        counter[metric].iter().all(|n| *n > 0.0),
                        "zero control clock cannot form a ratio",
                    )?;
                    let ratios: Vec<_> = v[metric]
                        .iter()
                        .zip(&counter[metric])
                        .map(|(a, b)| a / b)
                        .collect();
                    control.insert(
                        format!("{variant}_over_counters_{metric}"),
                        summary(&ratios),
                    );
                }
            }
            for name in CATEGORY_NAMES {
                let key = format!("{name}_cpu_ns");
                control.insert(
                    format!("{name}_descriptive_screen"),
                    screen(&fine[&key], &empty[&key], c, &fine["operation_cpu_ns"]),
                );
            }
            controls.insert(format!("{}/{arm}", layout.name), control);
        }
    }
    let summarized: BTreeMap<_, _> = groups
        .into_iter()
        .map(|(cell, metrics)| {
            let m: BTreeMap<_, _> = metrics
                .into_iter()
                .map(|(name, v)| (name, summary(&v)))
                .collect();
            (cell, m)
        })
        .collect();
    write(
        out,
        &json!({"schema":"spiralcore.fine-cache-profile-analysis.v1","source_revision":SOURCE_REVISION,"source_fingerprint":SOURCE_SHA256,"executable_sha256":expected_exe,"input_hashes":hashes,"observations":observations,"exact":observations,"failed":0,"frames":frames_count,"scans":scan_count,"metadata_api_calls":metadata_count,"modeled_application_bytes":bytes_total,"cells":summarized,"controls":controls,"screen_primary_cell":"spread768/sparse","production_candidate_accepted":false,"limitations":["three descriptive observations per cell; no performance adoption","fine/empty spans include clock overhead and perturb code generation; no correction subtraction","expression spans are not syscall or allocator counts; Enumeration includes namespace path argument construction","unspanned scan cost includes bookkeeping and entry/iterator destruction","raw optional DAG warming and sparse readiness differ while policy is fixed","no WAN or energy measurement"]}),
    )?;
    println!("Validated72 exact observations,828 frames,144 scans,57744 metadata API calls; no production adoption");
    Ok(())
}
fn main() {
    let a: Vec<_> = std::env::args_os().collect();
    let result = if a.len() == 4 && a[1] == "--qualify" {
        qualify(Path::new(&a[2]), Path::new(&a[3]))
    } else if a.len() == 3 {
        analyze(Path::new(&a[1]), Path::new(&a[2]))
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
    fn safety_rejects_oversized_text_without_accepting_partial_input() {
        let path=std::env::temp_dir().join(format!("sc-fine-read-bound-{}",std::process::id()));
        let mut file=fs::OpenOptions::new().write(true).create_new(true).open(&path).unwrap();
        file.write_all(&[b'x';64]).unwrap();drop(file);
        let result=read_text(&path,32);fs::remove_file(path).unwrap();assert!(result.is_err());
    }
    #[test]
    fn safety_rejects_clock_and_test_total_overflow() {
        assert!(checked_sum([u64::MAX,1]).is_err());
        assert!(test_counts("test result: ok. 18446744073709551615 passed; 0 failed; 0 ignored;\ntest result: ok. 1 passed; 0 failed; 0 ignored;").is_err());
    }
    #[test]
    fn summaries_keep_order_and_use_middle_of_three() {
        let s = summary(&[9.0, 1.0, 5.0]);
        assert_eq!(s["values"], json!([9.0, 1.0, 5.0]));
        assert_eq!(s["median"], 5.0);
        assert_eq!(s["minimum"], 1.0);
        assert_eq!(s["maximum"], 9.0);
        assert!(test_counts("no tests").is_err());
    }
    #[test]
    fn descriptive_screen_preserves_noise_and_overhead_boundaries() {
        let c = [200.0; 3];
        let good = screen(&[20.0; 3], &[5.0; 3], &c, &[250.0; 3]);
        assert_eq!(good["candidate_lead_only"], true);
        assert_eq!(good["production_candidate_accepted"], false);
        assert_eq!(
            screen(&[20.0; 3], &[5.0; 3], &c, &[251.0; 3])["candidate_lead_only"],
            false
        );
        assert_eq!(
            screen(&[4.0, 20.0, 20.0], &[5.0; 3], &c, &[200.0; 3])["candidate_lead_only"],
            false
        );
        assert_eq!(
            screen(&[20.0; 3], &[0.0; 3], &c, &[200.0; 3])["candidate_lead_only"],
            false
        );
    }
}
