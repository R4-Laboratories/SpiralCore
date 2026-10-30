//! Independent post-run evidence calculation. No endpoint, probe, or author analyzer code is linked.
//! Reads a fixed local packet, verifies hashes/receipts, and writes one new JSON report.
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};
const REV: &str = "d03fe1630bba787a28cc3fa3ae201c4a1a75388b";
const REG: &str = "9c9650dc156cd2511ba74932537f691a68fdada4";
const EXE: &str = "26942f5f371df05e6d63dac16c639273eaac5313e7e5ca2f8412444200006c37";
const SOURCE: &str = "640c5de7312ec20d32bad8fe7a9e7e4980c7580e5bdbcf10598a6a7d7fe4798d";
const LAYOUTS: [(&str, u64, u64); 4] = [
    ("empty", 0, 0),
    ("compact64", 64, 1),
    ("compact768", 768, 1),
    ("spread768", 768, 384),
];
const CATS: [&str; 6] = [
    "enumeration",
    "entry_type",
    "canonical_name",
    "extension_path",
    "metadata",
    "namespace_compare",
];
fn read(path: &Path) -> Vec<u8> {
    assert!(
        fs::symlink_metadata(path).unwrap().file_type().is_file(),
        "nonregular {}",
        path.display()
    );
    let mut out = Vec::new();
    File::open(path)
        .unwrap()
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut out)
        .unwrap();
    assert!(out.len() <= 64 * 1024 * 1024, "oversized input");
    out
}
fn text(path: &Path) -> String {
    String::from_utf8(read(path)).unwrap()
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn json_file(path: &Path) -> Value {
    serde_json::from_slice(&read(path)).unwrap()
}
fn lines(path: &Path) -> Vec<Value> {
    text(path)
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn n(v: &Value) -> u64 {
    v.as_u64().expect("unsigned integer")
}
fn add(a: u64, b: u64) -> u64 {
    a.checked_add(b).expect("integer overflow")
}
fn sum(it: impl Iterator<Item = u64>) -> u64 {
    it.fold(0, add)
}
fn stats(xs: &[f64]) -> Value {
    assert_eq!(xs.len(), 3);
    assert!(xs.iter().all(|v| v.is_finite()));
    let mut s = xs.to_vec();
    s.sort_by(f64::total_cmp);
    json!({"values":xs,"minimum":s[0],"median":s[1],"maximum":s[2]})
}
fn med(xs: &[f64]) -> f64 {
    stats(xs)["median"].as_f64().unwrap()
}
fn same(a: &Value, b: &Value, path: &str) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            assert!(
                (x - y).abs() <= 1e-12 * x.abs().max(y.abs()).max(1.0),
                "numeric mismatch {path}: {x} != {y}"
            );
        }
        (Value::Array(x), Value::Array(y)) => {
            assert_eq!(x.len(), y.len(), "{path}");
            for (i, (a, b)) in x.iter().zip(y).enumerate() {
                same(a, b, &format!("{path}/{i}"));
            }
        }
        (Value::Object(x), Value::Object(y)) => {
            assert_eq!(
                x.keys().collect::<Vec<_>>(),
                y.keys().collect::<Vec<_>>(),
                "keys {path}"
            );
            for (k, a) in x {
                same(a, &y[k], &format!("{path}/{k}"));
            }
        }
        _ => assert_eq!(a, b, "{path}"),
    }
}
fn manifest(root: &Path, file: &Path) -> usize {
    let mut count = 0;
    let mut seen = BTreeSet::new();
    for line in text(file).lines() {
        let (digest, p) = line.split_once("  ").unwrap();
        assert_eq!(digest.len(), 64);
        assert!(!p.split('/').any(|s| s == ".."));
        let path = root.join(p);
        assert!(seen.insert(path.clone()));
        assert_eq!(hash(&read(&path)), digest, "{}", path.display());
        count += 1;
    }
    count
}
fn object_id(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b"spiralcore.object.bytes.v1\0");
    h.update((b.len() as u64).to_le_bytes());
    h.update(b);
    format!("{:x}", h.finalize())
}
fn all_template_files(dir: &Path, out: &mut BTreeSet<PathBuf>, depth: usize) {
    assert!(depth <= 4 && out.len() <= 1612, "template census bound");
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            all_template_files(&entry.path(), out, depth + 1);
        } else {
            assert!(kind.is_file(), "nonregular template entry");
            assert!(out.insert(entry.path()));
        }
    }
}
fn fixture() -> (Vec<u8>, Vec<u8>) {
    let mut x = 42u64;
    let mut b = vec![0; 1 << 20];
    for v in &mut b {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *v = x as u8;
    }
    let mut t = b.clone();
    for v in &mut t[524288..524320] {
        *v ^= 165;
    }
    (b, t)
}
fn decode_hex(s: &str) -> Vec<u8> {
    assert_eq!(s.len() % 2, 0);
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn check_row(
    r: &Value,
    variant: &str,
    block: u64,
    li: usize,
    sparse: bool,
    target: &str,
) -> (BTreeMap<String, f64>, u64, u64, u64) {
    let (layout, pop, ns) = LAYOUTS[li];
    let frames = r["frame_samples"].as_array().unwrap();
    let count = if sparse { 3 } else { 20 };
    assert_eq!(r["kind"], "observation");
    assert_eq!(r["variant"], variant);
    assert_eq!(n(&r["block"]), block);
    assert_eq!(n(&r["population"]), pop);
    assert_eq!(n(&r["background_namespaces"]), ns);
    assert_eq!(r["arm"], json!({"sparse":sparse,"tiny":false}));
    assert_eq!(
        r["arm_label"],
        format!(
            "{}-node-budget-262144",
            if sparse { "sparse" } else { "raw" }
        )
    );
    assert_eq!(r["status"], "exact");
    assert!(r["error"].is_null());
    assert_eq!(r["error_stage"], "");
    assert_eq!(n(&r["object_bytes"]), 1 << 20);
    assert_eq!(r["target_object"], target);
    assert_eq!(frames.len(), count);
    assert_eq!(
        r["active_namespace"],
        "c017eb0b2e6c1cae26a0f1818b7089a486ebe712ace4cd79259a2521045c039c"
    );
    assert!(r["work_cache"]
        .as_str()
        .unwrap()
        .contains(&format!("/experiment/{layout}/working/{variant}-")));
    assert_eq!(r["target_manifest_present"], !sparse);
    assert_eq!(n(&r["graph_cache_after"]["entries"]), 0);
    assert_eq!(n(&r["graph_cache_after"]["resident_bytes"]), 0);
    let before = 1_048_657 + pop * 113;
    let target_size = 1_048_576 + if sparse { 49 } else { 81 };
    for (k, v) in [
        ("namespaces", ns + 1),
        ("publications", pop + 1),
        ("publication_bytes", before),
        ("active_publications", 1),
        ("active_bytes", 1_048_657),
        ("manifests", pop + 1),
    ] {
        assert_eq!(n(&r["before"][k]), v, "before {k}");
    }
    for (k, v) in [
        ("namespaces", ns + 1),
        ("publications", pop + 2),
        ("publication_bytes", before + target_size),
        ("active_publications", 2),
        ("active_bytes", 1_048_657 + target_size),
        ("manifests", pop + 1 + u64::from(!sparse)),
    ] {
        assert_eq!(n(&r["after"][k]), v, "after {k}");
    }
    let mut bytes = 0;
    let mut scans = 0;
    let mut metadata = 0;
    let mut m = BTreeMap::new();
    let mut sums: BTreeMap<&str, u64> = BTreeMap::new();
    let mut spans = [[0u64; 3]; 6];
    let mut accepted = 0u64;
    for (i, f) in frames.iter().enumerate() {
        let kind = if i == 0 {
            1
        } else if i + 1 == count {
            3
        } else {
            2
        };
        assert_eq!(n(&f["kind"]), kind);
        assert_eq!(n(&f["sequence"]), i as u64);
        assert_eq!(n(&f["reply_kind"]), kind + 100);
        assert_eq!(f["reply_body_truncated"], false);
        let response: Vec<u8> = f["reply_body_prefix"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| u8::try_from(n(x)).unwrap())
            .collect();
        let request = if kind == 1 {
            if sparse {
                134
            } else {
                102
            }
        } else if kind == 3 {
            56
        } else if sparse {
            128
        } else {
            61 + ((1 << 20) - accepted).min(61440)
        };
        assert_eq!(n(&f["request_bytes"]), request);
        assert_eq!(n(&f["reply_bytes"]), 56 + response.len() as u64);
        let expected = match kind {
            1 => {
                let mut v = vec![if sparse { 3 } else { 0 }, u8::from(sparse)];
                v.extend_from_slice(&1u64.to_le_bytes());
                v
            }
            2 => {
                accepted = if sparse {
                    1 << 20
                } else {
                    add(accepted, ((1 << 20) - accepted).min(61440))
                };
                accepted.to_le_bytes().to_vec()
            }
            _ => {
                assert_eq!(accepted, 1 << 20);
                decode_hex(target)
            }
        };
        assert_eq!(response, expected);
        bytes = add(bytes, add(n(&f["request_bytes"]), n(&f["reply_bytes"])));
        let p = &f["disk_usage"];
        assert_eq!(p["overflow"], false);
        assert!(p["last_error"].is_null() && p["last_error_stage"].is_null());
        assert_eq!(p["spans"].as_array().unwrap().len(), 6);
        if kind != 2 {
            assert_eq!(p["mode"], variant);
            scans = add(scans, n(&p["calls"]));
            metadata = add(metadata, n(&p["metadata_calls"]));
            for (k, v) in [
                ("calls", 1),
                ("successes", 1),
                ("errors", 0),
                ("root_read_dir_calls", 1),
                ("root_entries", ns + 3),
                ("root_file_type_calls", ns + 3),
                ("namespace_read_dir_calls", ns + 1),
                ("namespace_entries", pop + 1),
                ("object_file_type_calls", pop + 1),
                ("metadata_calls", pop + 1),
                ("metadata_successes", pop + 1),
                ("namespace_comparisons", pop + 1),
                ("objects_counted", pop + 1),
                ("returned_scoped_bytes", 1_048_657),
                ("returned_all_bytes", before),
                ("returned_objects", pop + 1),
            ] {
                assert_eq!(n(&p[k]), v, "scan {k}");
            }
            let calls = [
                (ns + 3) + (pop + 1) + 2 + 2 * (ns + 1),
                (ns + 3) + (pop + 1),
                ns + 1,
                pop + 1,
                pop + 1,
                pop + 1,
            ];
            for j in 0..6 {
                assert_eq!(n(&p["spans"][j]["calls"]), calls[j]);
            }
        } else {
            for (k, v) in p.as_object().unwrap() {
                if v.is_number() {
                    assert_eq!(n(v), 0, "Data {k}");
                }
            }
        }
        for (j, sp) in p["spans"].as_array().unwrap().iter().enumerate() {
            for (k, field) in ["calls", "cpu_ns", "wall_ns"].iter().enumerate() {
                spans[j][k] = add(spans[j][k], n(&sp[field]));
                if kind == 2 || variant == "counters" && k > 0 {
                    assert_eq!(n(&sp[field]), 0);
                }
            }
        }
        for clock in ["cpu_ns", "wall_ns"] {
            assert!(
                sum(p["spans"].as_array().unwrap().iter().map(|s| n(&s[clock]))) <= n(&p[clock])
            );
            assert!(n(&p[clock]) <= n(&f["cost"][clock]));
            let key = if clock == "cpu_ns" {
                "scan_cpu_ns"
            } else {
                "scan_wall_ns"
            };
            *sums.entry(key).or_default() = add(*sums.get(key).unwrap_or(&0), n(&p[clock]));
            let key = if clock == "cpu_ns" {
                "handler_cpu_ns"
            } else {
                "handler_wall_ns"
            };
            *sums.entry(key).or_default() = add(*sums.get(key).unwrap_or(&0), n(&f["cost"][clock]));
        }
    }
    assert_eq!(scans, 2);
    assert_eq!(bytes, if sparse { 536 } else { 1_051_138 });
    for clock in ["cpu_ns", "wall_ns"] {
        let op = n(&r["local_operation_total"][clock]);
        let dispatch = n(&r["dispatch_including_instrumentation"][clock]);
        let handler = n(&r["handler_stages"]["total"][clock]);
        assert!(op >= dispatch && dispatch >= handler);
        assert_eq!(
            handler,
            sum(["begin", "data", "finish"]
                .into_iter()
                .map(|k| n(&r["handler_stages"][k][clock])))
        );
        let key = if clock == "cpu_ns" {
            "handler_cpu_ns"
        } else {
            "handler_wall_ns"
        };
        assert_eq!(handler, sums[key]);
        m.insert(format!("operation_{clock}"), op as f64);
    }
    assert_eq!(n(&r["handler_stages"]["total"]["frames"]), count as u64);
    assert_eq!(n(&r["local_operation_total"]["frames"]), count as u64);
    for (k, v) in sums {
        m.insert(k.into(), v as f64);
    }
    for j in 0..6 {
        for (k, field) in ["calls", "cpu_ns", "wall_ns"].iter().enumerate() {
            m.insert(format!("{}_{}", CATS[j], field), spans[j][k] as f64);
        }
    }
    for (i, clock) in ["cpu_ns", "wall_ns"].iter().enumerate() {
        let s = n(&json!(m[&format!("scan_{clock}")] as u64));
        let p = sum(spans.iter().map(|v| v[i + 1]));
        m.insert(
            format!("scan_unspanned_{clock}"),
            s.checked_sub(p).unwrap() as f64,
        );
    }
    m.insert(
        "scan_fraction_of_operation_cpu".into(),
        m["scan_cpu_ns"] / m["operation_cpu_ns"],
    );
    m.insert("modeled_application_bytes".into(), bytes as f64);
    (m, count as u64, scans, metadata)
}
fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("packet root"));
    let out = PathBuf::from(std::env::args().nth(2).expect("new report"));
    let sched = json_file(&root.join("schedule.json"));
    assert_eq!(sched["source_revision"], REV);
    assert_eq!(sched["registration_revision"], REG);
    assert_eq!(n(&sched["observations"]), 72);
    assert_eq!(sched["feature"], "hardware-sha256");
    assert_eq!(n(&sched["node_budget"]), 262144);
    assert_eq!(n(&sched["graph_cache_entries"]), 0);
    assert_eq!(n(&sched["graph_cache_bytes"]), 0);
    let (base, target) = fixture();
    let bid = object_id(&base);
    let tid = object_id(&target);
    assert_eq!(sched["base_object"], bid);
    assert_eq!(sched["target_object"], tid);
    let author = json_file(&root.join("profile-analysis.json"));
    let qualification = json_file(&root.join("qualification.json"));
    assert_eq!(
        hash(&read(&root.join("build-diagnostic/receiver-probe"))),
        EXE
    );
    assert_eq!(
        hash(&read(
            &root.join("build-diagnostic/production-source-files.sha256")
        )),
        SOURCE
    );
    let mut checks = Map::new();
    for name in [
        "snapshot-established.sha256",
        "build-diagnostic/helper-files.sha256",
        "build-diagnostic/executables.sha256",
        "qualification/qualified-inputs.sha256",
        "measured/observations.sha256",
    ] {
        checks.insert(name.into(), json!(manifest(&root, &root.join(name))));
    }
    checks.insert(
        "production-source-files.sha256".into(),
        json!(manifest(
            &root.join("snapshot"),
            &root.join("build-diagnostic/production-source-files.sha256")
        )),
    );
    assert_eq!(
        read(&root.join("build-diagnostic/production-source-files.sha256")),
        read(&root.join("build-diagnostic/production-source-files.after.sha256"))
    );
    for (p, d) in qualification["hashes"].as_object().unwrap() {
        assert_eq!(
            hash(&read(&root.join(p))),
            d.as_str().unwrap(),
            "qualification {p}"
        );
    }
    let established = read(&root.join("templates-established.sha256"));
    assert_eq!(
        established,
        read(&root.join("measured/templates-before.sha256"))
    );
    assert_eq!(
        established,
        read(&root.join("measured/templates-after.sha256"))
    );
    let template_count = manifest(
        &root.join("experiment"),
        &root.join("templates-established.sha256"),
    );
    assert_eq!(template_count, 1612);
    let expected_files: BTreeSet<_> = std::str::from_utf8(&established)
        .unwrap()
        .lines()
        .map(|line| {
            root.join("experiment")
                .join(line.split_once("  ").unwrap().1)
        })
        .collect();
    let mut actual_files = BTreeSet::new();
    for (layout, _, _) in LAYOUTS {
        all_template_files(
            &root.join("experiment").join(layout).join("templates"),
            &mut actual_files,
            0,
        );
    }
    assert_eq!(
        actual_files, expected_files,
        "template manifest must cover the complete live file census"
    );
    let mut publications = 0;
    for line in std::str::from_utf8(&established).unwrap().lines() {
        let (_, name) = line.split_once("  ").unwrap();
        if name.ends_with(".object") {
            let b = read(&root.join("experiment").join(name));
            assert_eq!(&b[..8], b"SCPUB001");
            let len = u64::from_le_bytes(b[8..16].try_into().unwrap());
            assert_eq!(b[48], 1);
            assert_eq!(b.len() as u64, len + 81);
            let id = object_id(&b[81..]);
            assert_eq!(decode_hex(&id), b[16..48]);
            assert!(name.ends_with(&format!("{id}.object")));
            if id == bid {
                assert_eq!(&b[81..], base);
            }
            assert_ne!(id, tid);
            publications += 1;
        }
    }
    assert_eq!(publications, 1604);
    let repo = "/Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore";
    let mut origins = 0;
    for line in text(&root.join("qualification/snapshot-origin.tsv")).lines() {
        let f: Vec<_> = line.split('\t').collect();
        let path = f.last().unwrap();
        let bytes = Command::new("git")
            .args(["-C", repo, "show", &format!("{REV}:{path}")])
            .output()
            .unwrap();
        assert!(bytes.status.success());
        if f[0] == "exact" {
            assert_eq!(bytes.stdout, read(&root.join("snapshot").join(path)));
        } else {
            assert!([
                "crates/spiralcore-transfer/src/endpoint.rs",
                "crates/spiralcore-transfer/src/lib.rs"
            ]
            .contains(path));
            if path.ends_with("endpoint.rs") {
                assert_eq!(
                    bytes.stdout,
                    read(&root.join("qualification/original-endpoint.rs"))
                );
            }
        }
        origins += 1;
    }
    let metadata = json_file(&root.join("build-diagnostic/metadata.json"));
    let nodes = metadata["resolve"]["nodes"].as_array().unwrap();
    let sha = nodes
        .iter()
        .find(|v| v["id"].as_str().unwrap().contains("#sha2@0.10.9"))
        .unwrap();
    assert_eq!(
        sha["features"],
        json!(["asm", "default", "sha2-asm", "std"])
    );
    let mut test_counts = Map::new();
    for (p, v) in qualification["tests"].as_object().unwrap() {
        let raw = text(&root.join(p));
        assert_eq!(hash(raw.as_bytes()), v["sha256"].as_str().unwrap());
        let mut c = [0u64; 3];
        for l in raw.lines().filter(|l| l.starts_with("test result:")) {
            let fields: Vec<_> = l.split_whitespace().collect();
            for i in 0..fields.len() - 1 {
                for (j, label) in ["passed;", "failed;", "ignored;"].iter().enumerate() {
                    if fields[i + 1] == *label {
                        c[j] = add(c[j], fields[i].parse().unwrap());
                    }
                }
            }
        }
        for (i, k) in ["passed", "failed", "ignored"].iter().enumerate() {
            assert_eq!(c[i], n(&v[k]));
        }
        test_counts.insert(p.clone(), json!(c));
    }
    let execution = lines(&root.join("measured/execution.jsonl"));
    assert_eq!(execution.len(), 18);
    let fixed = [
        (0, "counters"),
        (0, "fine"),
        (0, "empty"),
        (1, "fine"),
        (1, "empty"),
        (1, "counters"),
        (2, "empty"),
        (2, "counters"),
        (2, "fine"),
    ];
    let mut cells: BTreeMap<String, BTreeMap<String, Vec<f64>>> = BTreeMap::new();
    let mut raw: BTreeMap<(String, String), Vec<BTreeMap<String, f64>>> = BTreeMap::new();
    let (mut frame_total, mut scan_total, mut meta_total, mut byte_total) = (0, 0, 0, 0);
    let mut seen_work = BTreeSet::new();
    let mut first_row = None;
    for (pos, (block, variant)) in fixed.into_iter().enumerate() {
        assert_eq!(
            sched["invocations"][pos],
            json!({"block":block,"variant":variant})
        );
        for (offset, event) in ["start", "end"].into_iter().enumerate() {
            let e = &execution[2 * pos + offset];
            assert_eq!(e["event"], event);
            assert_eq!(n(&e["block"]), block);
            assert_eq!(e["variant"], variant);
            if offset == 1 {
                assert_eq!(n(&e["exit_code"]), 0);
                assert!(e["utc"].as_str().unwrap() >= execution[2 * pos]["utc"].as_str().unwrap());
            } else if pos > 0 {
                assert!(
                    e["utc"].as_str().unwrap() >= execution[2 * pos - 1]["utc"].as_str().unwrap()
                );
            }
        }
        let file = format!("block{block}-{variant}.jsonl");
        assert_eq!(
            hash(&read(&root.join("measured").join(&file))),
            author["input_hashes"][&file].as_str().unwrap()
        );
        let rows = lines(&root.join("measured").join(&file));
        assert_eq!(rows.len(), 9);
        let h = &rows[0];
        assert_eq!(h["kind"], "metadata");
        assert_eq!(h["library_revision"], REV);
        assert_eq!(h["library_source_sha256"], SOURCE);
        assert_eq!(h["executable_sha256"], EXE);
        assert_eq!(h["variant"], variant);
        assert_eq!(n(&h["block"]), block);
        assert_eq!(
            h["policies"],
            json!({"nodes":262144,"graph_cache_entries":0,"graph_cache_bytes":0})
        );
        assert_eq!(h["categories"], json!(CATS));
        for i in 0..8 {
            let li = (i / 2 + block as usize) % 4;
            let sparse = (i % 2 + block as usize + i / 2) % 2 == 1;
            let r = &rows[i + 1];
            assert_eq!(r["base_object"], bid);
            let work = r["work_cache"].as_str().unwrap();
            assert!(seen_work.insert(work.to_string()));
            assert!(
                !Path::new(work).exists(),
                "successful work cache retained unexpectedly"
            );
            let (m, fr, sc, me) = check_row(r, variant, block, li, sparse, &tid);
            if first_row.is_none() {
                first_row = Some(r.clone());
            }
            frame_total = add(frame_total, fr);
            scan_total = add(scan_total, sc);
            meta_total = add(meta_total, me);
            byte_total = add(byte_total, m["modeled_application_bytes"] as u64);
            let arm = if sparse { "sparse" } else { "raw" };
            let cell = format!("{}/{arm}/{variant}", LAYOUTS[li].0);
            for (k, v) in &m {
                cells
                    .entry(cell.clone())
                    .or_default()
                    .entry(k.clone())
                    .or_default()
                    .push(*v);
            }
            raw.entry((format!("{}/{arm}", LAYOUTS[li].0), variant.into()))
                .or_default()
                .push(m);
        }
    }
    let cells_json: Map<_, _> = cells
        .iter()
        .map(|(k, m)| {
            (
                k.clone(),
                Value::Object(m.iter().map(|(k, v)| (k.clone(), stats(v))).collect()),
            )
        })
        .collect();
    same(
        &Value::Object(cells_json.clone()),
        &author["cells"],
        "cells",
    );
    let mut controls = Map::new();
    let mut passed = Vec::new();
    for (layout, _, _) in LAYOUTS {
        for arm in ["raw", "sparse"] {
            let cell = format!("{layout}/{arm}");
            let c = &raw[&(cell.clone(), "counters".into())];
            let f = &raw[&(cell.clone(), "fine".into())];
            let e = &raw[&(cell.clone(), "empty".into())];
            let mut control = Map::new();
            for (variant, rows) in [("fine", f), ("empty", e)] {
                for metric in [
                    "operation_cpu_ns",
                    "operation_wall_ns",
                    "scan_cpu_ns",
                    "scan_wall_ns",
                ] {
                    let ratios: Vec<_> = (0..3).map(|i| rows[i][metric] / c[i][metric]).collect();
                    control.insert(format!("{variant}_over_counters_{metric}"), stats(&ratios));
                }
            }
            let inflation: Vec<_> = (0..3)
                .map(|i| f[i]["operation_cpu_ns"] / c[i]["operation_cpu_ns"])
                .collect();
            for cat in CATS {
                let name = format!("{cat}_cpu_ns");
                let fs: Vec<_> = f.iter().map(|r| r[&name]).collect();
                let es: Vec<_> = e.iter().map(|r| r[&name]).collect();
                let share: Vec<_> = (0..3).map(|i| fs[i] / c[i]["operation_cpu_ns"]).collect();
                let all = (0..3).all(|i| fs[i] > es[i]);
                let four = med(&es) > 0.0 && med(&fs) >= 4.0 * med(&es);
                let ten = med(&share) >= 0.10;
                let overhead = med(&inflation) <= 1.25;
                let pass = all && four && ten && overhead;
                if pass {
                    passed.push(format!("{cell}/{cat}"));
                }
                control.insert(format!("{cat}_descriptive_screen"),json!({"all_three_fine_spans_exceed_empty":all,"candidate_lead_only":pass,"empty_cpu_ns":es,"fine_cpu_ns":fs,"fine_operation_over_counter_operation_cpu":stats(&inflation),"fine_over_counter_operation_cpu":stats(&share),"median_fine_at_least_four_times_positive_empty":four,"median_fine_span_at_least_ten_percent_counter_operation":ten,"median_instrumentation_inflation_at_most_twenty_five_percent":overhead,"production_candidate_accepted":false}));
            }
            controls.insert(cell, Value::Object(control));
        }
    }
    same(
        &Value::Object(controls.clone()),
        &author["controls"],
        "controls",
    );
    for (k, v) in [
        ("observations", 72),
        ("exact", 72),
        ("failed", 0),
        ("frames", frame_total),
        ("scans", scan_total),
        ("metadata_api_calls", meta_total),
        ("modeled_application_bytes", byte_total),
    ] {
        assert_eq!(n(&author[k]), v, "total {k}");
    }
    assert_eq!(author["production_candidate_accepted"], false);
    assert_eq!(
        passed,
        vec![
            "compact768/sparse/metadata",
            "compact768/sparse/namespace_compare"
        ]
    );
    let mut negatives = Vec::new();
    let original = first_row.unwrap();
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    for (kind, mut r) in [
        ("wrong_status", original.clone()),
        ("wrong_accepted_bytes", original.clone()),
        ("wrong_inventory", original.clone()),
        ("wrong_scan_count", original.clone()),
    ] {
        match kind {
            "wrong_status" => r["status"] = json!("failed"),
            "wrong_accepted_bytes" => r["frame_samples"][1]["reply_body_prefix"][0] = json!(1),
            "wrong_inventory" => r["before"]["publications"] = json!(2),
            _ => r["frame_samples"][0]["disk_usage"]["calls"] = json!(2),
        }
        assert!(std::panic::catch_unwind(|| check_row(&r, "counters", 0, 0, false, &tid)).is_err());
        negatives.push(kind);
    }
    assert!(std::panic::catch_unwind(|| add(u64::MAX, 1)).is_err());
    negatives.push("checked_integer_overflow");
    std::panic::set_hook(hook);
    let result = json!({"schema":"spiralcore.independent-fine-cache-review.v1","status":"passed","method":"Independent Rust implementation using only serde_json and SHA256; no author analyzer/probe/endpoint code linked or invoked; no performance rerun","observations":72,"frames":frame_total,"scans":scan_total,"metadata_api_calls":meta_total,"modeled_application_bytes":byte_total,"source_revision":REV,"registration_revision":REG,"source_fingerprint":SOURCE,"executable_sha256":EXE,"source_origin_entries_checked":origins,"source_and_evidence_manifests":checks,"template_files_checked":template_count,"template_publications_rehashed":publications,"fixture_base_object":bid,"fixture_target_object":tid,"processes":9,"all_process_exits_zero":true,"execution_start":execution[0]["utc"],"execution_end":execution[17]["utc"],"qualification_test_counts_pass_fail_ignore":test_counts,"negative_checks":negatives,"all_24_cell_statistics_match":true,"all_8_paired_controls_and_48_screens_match":true,"primary_passed_categories":0,"secondary_passed_categories":passed,"production_candidate_accepted":false,"input_analysis_sha256":hash(&read(&root.join("profile-analysis.json"))),"input_report_sha256":hash(&read(&root.join("RESULTS.md"))),"cells":cells_json,"controls":controls,"limitations":["Historical measured work caches were intentionally removed after producer exact verification; this checker validates retained receipts and source, not a second independent reopen of those deleted publications.","Template byte hashes and ObjectIds are independently rechecked; optional DAG manifest contents are not independently rebuilt.","Observed fixed 72-row schedule and nine exits are verified; no cryptographic attestation that no unrecorded run occurred.","Primary fine-clock overhead invalidates candidate selection from its expression ranking; no empty-span subtraction, attainable-savings, energy, syscall/allocation-count or WAN inference.","Actual invalid-UTF8 filesystem entry remains NOT_RUN on this host."]});
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(out)
        .unwrap();
    serde_json::to_writer_pretty(&mut f, &result).unwrap();
    writeln!(f).unwrap();
    f.sync_all().unwrap();
    println!("Independent verification passed:72 rows,24 cells,48 screens,0 primary leads; retained output only, no performance rerun.");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paired_median_is_not_unpaired_ratio() {
        let a = [1.0, 50.0, 100.0];
        let b = [10.0, 25.0, 20.0];
        assert_eq!(med(&(0..3).map(|i| a[i] / b[i]).collect::<Vec<_>>()), 2.0);
        assert_eq!(med(&a) / med(&b), 2.5);
    }
    #[test]
    fn overflowing_sum_is_rejected() {
        assert!(std::panic::catch_unwind(|| sum([u64::MAX, 1].into_iter())).is_err());
    }
    #[test]
    fn altered_numeric_stat_is_rejected() {
        assert!(std::panic::catch_unwind(|| same(
            &stats(&[1.0, 2.0, 3.0]),
            &stats(&[1.0, 2.1, 3.0]),
            "negative"
        ))
        .is_err());
    }
}
