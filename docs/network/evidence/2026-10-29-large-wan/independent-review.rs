// Independent post-run analysis; compile and execute only after WAN timing ends.
// Inputs: campaign root, author's aggregate JSON. This never performs networking.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::Path,
};
type Values = BTreeMap<String, BTreeMap<String, BTreeMap<String, Vec<u64>>>>;
fn num(v: &Value, key: &str) -> u64 {
    v[key].as_u64().expect(key)
}
fn checked_sum(values: &[u64]) -> u64 {
    values
        .iter()
        .try_fold(0u64, |a, b| a.checked_add(*b))
        .expect("integer overflow")
}
fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().expect(key)
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn hash(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p);
    }
    h.finalize().into()
}
fn read(path: &Path, hashes: &mut BTreeMap<String, String>) -> Vec<u8> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .unwrap()
        .take((8 << 20) + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 8 << 20);
    hashes.insert(path.display().to_string(), digest(&bytes));
    bytes
}
fn decode(s: &str) -> Vec<u8> {
    assert_eq!(s.len() % 2, 0);
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn fixture_ids(seed: u64) -> [String; 2] {
    let mut x = seed;
    let mut b = vec![0u8; 16 << 20];
    for c in b.chunks_exact_mut(8) {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        c.copy_from_slice(&x.to_le_bytes());
    }
    let len = (b.len() as u64).to_le_bytes();
    let base = hex(&hash(&[b"spiralcore.object.bytes.v1\0", &len, &b]));
    for v in &mut b[(8 << 20)..(8 << 20) + 32] {
        *v ^= 0xa5;
    }
    let target = hex(&hash(&[b"spiralcore.object.bytes.v1\0", &len, &b]));
    [base, target]
}
fn metrics(row: &Value) -> BTreeMap<String, u64> {
    let m = &row["metrics"];
    let mut out = BTreeMap::new();
    for key in [
        "wall_micros",
        "sender_cpu_micros",
        "receiver_cpu_micros",
        "receiver_wall_micros",
        "request_body_bytes",
        "response_body_bytes",
    ] {
        out.insert(key.into(), num(m, key));
    }
    out.insert(
        "combined_cpu_micros".into(),
        num(m, "sender_cpu_micros")
            .checked_add(num(m, "receiver_cpu_micros"))
            .unwrap(),
    );
    out.insert(
        "body_bytes".into(),
        num(m, "request_body_bytes")
            .checked_add(num(m, "response_body_bytes"))
            .unwrap(),
    );
    out
}
fn summary(v: &[u64]) -> Value {
    let n = v.len();
    assert!(n > 0 && n <= 30);
    let mut s = v.to_vec();
    s.sort_unstable();
    let median = if n.is_multiple_of(2) {
        (s[n / 2 - 1] as f64 + s[n / 2] as f64) / 2.
    } else {
        s[n / 2] as f64
    };
    json!({"n":n,"values":v,"median":median,"p95":s[(95*n).div_ceil(100)-1],"min":s[0],"max":s[n-1],"sum":v.iter().try_fold(0u64,|a,b|a.checked_add(*b)).unwrap()})
}
fn comparison(b: &[u64], s: &[u64]) -> Value {
    let n = b.len();
    assert!(n > 0 && n <= 30);
    assert_eq!(s.len(), n);
    assert!(!b.contains(&0) && !s.contains(&0));
    let ratios: Vec<_> = b
        .iter()
        .zip(s)
        .map(|(b, s)| *b as f64 / *s as f64)
        .collect();
    let wins = b.iter().zip(s).filter(|(b, s)| b > s).count();
    let ties = b.iter().zip(s).filter(|(b, s)| b == s).count();
    let gm = (ratios.iter().map(|r| r.ln()).sum::<f64>() / n as f64).exp();
    json!({"n":n,"ratios":ratios,"geometric_mean":gm,"candidate_faster":wins,"ties":ties,"candidate_slower":n-wins-ties})
}
fn byte_reductions(base: &[u64], candidate: &[u64]) -> Value {
    let n = base.len();
    assert!(n > 0 && n <= 30);
    assert_eq!(candidate.len(), n);
    assert!(!base.contains(&0));
    let values: Vec<_> = base
        .iter()
        .zip(candidate)
        .map(|(b, c)| 1. - *c as f64 / *b as f64)
        .collect();
    let mut sorted = values.clone();
    sorted.sort_by(f64::total_cmp);
    let base_sum = base
        .iter()
        .try_fold(0u64, |a, b| a.checked_add(*b))
        .unwrap();
    let candidate_sum = candidate
        .iter()
        .try_fold(0u64, |a, b| a.checked_add(*b))
        .unwrap();
    let median = if n.is_multiple_of(2) {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.
    } else {
        sorted[n / 2]
    };
    json!({"n":n,"paired_fraction_reductions":values,"midpoint_median_fraction_reduction":median,"minimum_fraction_reduction":sorted[0],"maximum_fraction_reduction":sorted[n-1],"total_base_ip_bytes":base_sum,"total_candidate_ip_bytes":candidate_sum,"aggregate_fraction_reduction":1.-candidate_sum as f64/base_sum as f64})
}
fn close(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 1e-12 * a.abs().max(1.),
        "numeric mismatch: {a} / {b}"
    );
}
fn same(a: &Value, b: &Value) {
    match a {
        Value::Object(m) => {
            for (k, v) in m {
                same(v, &b[k]);
            }
        }
        Value::Array(v) => {
            assert_eq!(v.len(), b.as_array().unwrap().len());
            for (x, y) in v.iter().zip(b.as_array().unwrap()) {
                same(x, y);
            }
        }
        Value::Number(n) if n.is_f64() => close(n.as_f64().unwrap(), b.as_f64().unwrap()),
        _ => assert_eq!(a, b),
    }
}
fn author_identity(author: &Value, run: &str) {
    assert_eq!(author["schema"], "spiralcore.large-wan.analysis.v1");
    assert_eq!(author["run"], run);
    assert_eq!(author["source"], "f4e5e9afe6f3628ffe82e4553a88111f5bd6d126");
    assert_eq!(author["rows"], 180);
}
fn counters(v: &Value) -> [u64; 9] {
    std::array::from_fn(|i| {
        num(
            v,
            [
                "upload_ip_bytes",
                "download_ip_bytes",
                "unclassified_ip_bytes",
                "total_ip_bytes",
                "upload_packets",
                "download_packets",
                "unclassified_packets",
                "packets",
                "unknown_ip_length_packets",
            ][i],
        )
    })
}
fn add(dst: &mut [u64; 9], src: [u64; 9]) {
    for (a, b) in dst.iter_mut().zip(src) {
        *a = a.checked_add(b).unwrap();
    }
}
fn capture_review(
    root: &Path,
    rows: &[Value],
    hashes: &mut BTreeMap<String, String>,
    ip_values: &mut Values,
) -> Value {
    let path = root.join("remote/capture.json");
    let cap: Value = serde_json::from_slice(&read(&path, hashes)).unwrap();
    assert_eq!(cap["scope"], "runner_to_relay_leg_only");
    assert_eq!(cap["partition_sum_verified"], true);
    let mut total = counters(&cap["setup"]);
    add(&mut total, counters(&cap["unattributed"]));
    let captured = cap["rows"].as_array().unwrap();
    assert_eq!(captured.len(), rows.len());
    let mut seen = BTreeSet::new();
    let mut by_operation = BTreeMap::new();
    for c in captured {
        let id = text(&c["task"], "id");
        let phase = text(c, "phase");
        assert!(seen.insert((id, phase)));
        let row = rows
            .iter()
            .find(|r| text(&r["task"], "id") == id && text(r, "phase") == phase)
            .unwrap();
        assert_eq!(c["unix_start_nanos"], row["unix_start_nanos"]);
        assert_eq!(c["unix_end_nanos"], row["unix_end_nanos"]);
        let m = &row["metrics"];
        assert_eq!(
            num(&c["expected_application_body_bytes"], "upload"),
            num(m, "request_body_bytes")
        );
        assert_eq!(
            num(&c["expected_application_body_bytes"], "download"),
            num(m, "response_body_bytes")
        );
        let coverage = num(c, "upload_ip_bytes") >= num(m, "request_body_bytes")
            && num(c, "download_ip_bytes") >= num(m, "response_body_bytes");
        assert_eq!(c["application_body_coverage_verified"], coverage);
        let cc = counters(c);
        assert_eq!(checked_sum(&cc[..3]), cc[3]);
        assert_eq!(checked_sum(&cc[4..7]), cc[7]);
        add(&mut total, cc);
        assert!(by_operation
            .insert((id, phase), [cc[0], cc[1], cc[3]])
            .is_none());
    }
    assert_eq!(total, counters(&cap["campaign"]));
    let diagnostic = read(&root.join("remote/tcpdump-stats.txt"), hashes);
    let diagnostic = std::str::from_utf8(&diagnostic).unwrap();
    for (suffix, key) in [
        ("packets captured", "tcpdump_captured_packets"),
        (
            "packets received by filter",
            "tcpdump_received_by_filter_packets",
        ),
        ("packets dropped by kernel", "kernel_reported_drops"),
    ] {
        let lines: Vec<_> = diagnostic.lines().filter(|l| l.ends_with(suffix)).collect();
        assert_eq!(lines.len(), 1);
        let value: u64 = lines[0].split_whitespace().next().unwrap().parse().unwrap();
        assert_eq!(num(&cap, key), value);
    }
    assert_eq!(
        num(&cap, "capture_counter_difference_packets"),
        num(&cap, "tcpdump_captured_packets")
            .abs_diff(num(&cap, "tcpdump_received_by_filter_packets"))
    );
    let offload = read(&root.join("remote/offload-state.txt"), hashes);
    let offload = std::str::from_utf8(&offload).unwrap();
    for feature in [
        "tcp-segmentation-offload",
        "generic-segmentation-offload",
        "generic-receive-offload",
        "large-receive-offload",
    ] {
        let prefix = format!("{feature}: ");
        let value = offload
            .lines()
            .find_map(|l| l.strip_prefix(&prefix))
            .unwrap();
        assert!(value == "off" || value == "off [fixed]");
    }
    let pinned = read(&root.join("remote/pinned-addresses.txt"), hashes);
    let pinned = std::str::from_utf8(&pinned).unwrap();
    assert!(!pinned.trim().is_empty());
    for line in pinned.lines() {
        assert_eq!(
            line.split_whitespace().next().unwrap(),
            text(&cap, "expected_remote_ip")
        );
    }
    let complete = cap["capture_complete"].as_bool().unwrap();
    if complete {
        assert!(cap["incomplete_reasons"].as_array().unwrap().is_empty());
        assert_eq!(cap["kernel_reported_drops"], 0);
        assert_eq!(cap["ambiguous_connections"], 0);
        assert_eq!(num(&cap["unattributed"], "packets"), 0);
        assert_eq!(total[6], 0);
        assert_eq!(total[8], 0);
        assert_eq!(num(&cap, "tcpdump_captured_packets"), total[7]);
        assert_eq!(num(&cap, "tcpdump_received_by_filter_packets"), total[7]);
        assert_eq!(cap["capture_counter_difference_packets"], 0);
        assert_eq!(cap["pcap_record_count_matches_tcpdump_captured"], true);
        for c in captured {
            assert_eq!(c["application_body_coverage_verified"], true);
            assert!(num(c, "packets_within_measurement") > 0);
        }
        let ips = cap["observed_remote_ips"].as_array().unwrap();
        assert_eq!(ips.len(), 1);
        assert_eq!(ips[0], cap["expected_remote_ip"]);
        // Canonical receipt order controls vector order; capture array ordering does not.
        // Full connection lifetime is already assigned by the observed-SYN parser.
        for row in rows {
            let task = &row["task"];
            let id = text(task, "id");
            let arm = text(task, "arm");
            let phase = text(row, "phase");
            let values = by_operation[&(id, phase)];
            for (index, metric) in ["upload_ip_bytes", "download_ip_bytes", "total_ip_bytes"]
                .iter()
                .enumerate()
            {
                ip_values
                    .entry(arm.into())
                    .or_default()
                    .entry(phase.into())
                    .or_default()
                    .entry((*metric).into())
                    .or_default()
                    .push(values[index]);
                if phase == "transfer" {
                    let seed = by_operation[&(id, "seed")][index];
                    ip_values
                        .entry(arm.into())
                        .or_default()
                        .entry("seed_plus_transfer".into())
                        .or_default()
                        .entry((*metric).into())
                        .or_default()
                        .push(seed.checked_add(values[index]).unwrap());
                }
            }
        }
    } else {
        assert!(!cap["incomplete_reasons"].as_array().unwrap().is_empty());
    }
    json!({"capture_complete":complete,"incomplete_reasons":cap["incomplete_reasons"],"campaign":cap["campaign"],"unattributed":cap["unattributed"],"kernel_reported_drops":cap["kernel_reported_drops"],"captured_packets":cap["tcpdump_captured_packets"],"received_packets":cap["tcpdump_received_by_filter_packets"],"scope":"runner_to_relay_leg_only","review":"JSON partition and every row/body coverage checked; separate PCAP replay and offload provenance required"})
}
fn main() {
    let a: Vec<_> = std::env::args().collect();
    assert_eq!(
        a.len(),
        3,
        "campaign root and author aggregate JSON required"
    );
    let root = Path::new(&a[1]);
    let mut hashes = BTreeMap::new();
    let run_raw = read(&root.join("run-id.txt"), &mut hashes);
    let run = std::str::from_utf8(&run_raw).unwrap().trim().to_owned();
    let run_bytes = decode(&run);
    assert_eq!(run_bytes.len(), 32);
    let author: Value = serde_json::from_slice(&read(Path::new(&a[2]), &mut hashes)).unwrap();
    author_identity(&author, &run);
    let arms = ["raw_full", "zstd_full", "sparse"];
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let offset = hash(&[
        b"spiralcore.bench.order.v3\0",
        &run_bytes,
        b"large16-localized",
    ])[0] as usize;
    let mut values: Values = BTreeMap::new();
    let mut ip_values: Values = BTreeMap::new();
    let mut captures = Vec::new();
    let mut binaries = BTreeMap::new();
    let mut ids = BTreeMap::new();
    let mut global_scopes = BTreeSet::new();
    let mut frames = 0u64;
    let mut orders = [0u64; 6];
    for shard in 0u8..3 {
        let dir = root.join(format!("shard-{shard}"));
        let raw = read(&dir.join("remote/rows.jsonl"), &mut hashes);
        let rows: Vec<Value> = std::str::from_utf8(&raw)
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(rows.len(), 60);
        let plan: Value =
            serde_json::from_slice(&read(&dir.join("remote/plan.json"), &mut hashes)).unwrap();
        let revision = read(&dir.join("remote/source-revision.txt"), &mut hashes);
        assert_eq!(
            std::str::from_utf8(&revision).unwrap().trim(),
            "f4e5e9afe6f3628ffe82e4553a88111f5bd6d126"
        );
        let binary_manifest = read(&dir.join("remote/binary-sha256.txt"), &mut hashes);
        let binary_manifest = std::str::from_utf8(&binary_manifest).unwrap();
        let lines: Vec<_> = binary_manifest
            .lines()
            .filter(|l| l.split_whitespace().nth(1) == Some("target/release/spiralcore-bench"))
            .collect();
        assert_eq!(lines.len(), 1);
        let expected_binary = lines[0].split_whitespace().next().unwrap();
        assert_eq!(decode(expected_binary).len(), 32);
        let verification: Value =
            serde_json::from_slice(&read(&dir.join("verification.json"), &mut hashes)).unwrap();
        assert_eq!(verification["schema"], "spiralcore.wan.verification.v3");
        assert_eq!(verification["suite"], "large16-localized");
        assert_eq!(verification["run"], run);
        assert_eq!(verification["shard"], shard);
        assert_eq!(verification["status"], "exact");
        assert_eq!(verification["receipts_verified"], 60);
        assert_eq!(verification["bytes_compared"], 1006632960u64);
        assert_eq!(verification["qualification_outside_transfer_timing"], true);
        let evidence = read(&dir.join("evidence.sha256"), &mut hashes);
        let evidence = std::str::from_utf8(&evidence).unwrap();
        for relative in [
            "remote/plan.json",
            "remote/rows.jsonl",
            "remote/capture.json",
            "verification.json",
        ] {
            let path = dir.join(relative);
            let data = read(&path, &mut hashes);
            let rendered = path.display().to_string();
            let matching: Vec<_> = evidence
                .lines()
                .filter(|l| l.split_whitespace().nth(1) == Some(rendered.as_str()))
                .collect();
            assert_eq!(matching.len(), 1);
            assert_eq!(
                matching[0].split_whitespace().next().unwrap(),
                digest(&data)
            );
        }
        assert_eq!(plan["schema"], "spiralcore.wan.plan.v3");
        assert_eq!(plan["shard"], shard);
        assert_eq!(plan["run"], run);
        assert_eq!(num(&plan, "trials"), 30);
        assert_eq!(num(&plan, "retained_payload_bytes"), 960 << 20);
        assert_eq!(plan["tasks"].as_array().unwrap().len(), 30);
        let mut index = 0;
        for trial in u32::from(shard) * 10..u32::from(shard + 1) * 10 {
            let seed_hash = hash(&[
                b"spiralcore.bench.fixture.v1\0",
                &run_bytes,
                &(16u64 << 20).to_le_bytes(),
                &trial.to_le_bytes(),
            ]);
            let seed = u64::from_le_bytes(seed_hash[..8].try_into().unwrap()).max(1);
            let objects = fixture_ids(seed);
            ids.insert(trial, objects.clone());
            let perm = (offset + trial as usize) % 6;
            orders[perm] += 1;
            for arm_index in permutations[perm] {
                let arm = arms[arm_index];
                let pair = format!("16777216-localized_32-{trial:02}");
                let id = format!("{pair}-{arm}");
                let scope = hex(&hash(&[
                    b"spiralcore.bench.scope.v3\0",
                    &run_bytes,
                    b"large16-localized",
                    &[shard],
                    id.as_bytes(),
                ]));
                assert!(global_scopes.insert(scope.clone()));
                let task = json!({"id":id,"pair":pair,"trial":trial,"case":"localized_32","size":16777216,"arm":arm,"scope":scope,"fixture_seed":seed,"seed_base":true,"warm_probe":false,"cache_state":"base_present_target_absent"});
                assert_eq!(plan["tasks"][index / 2], task);
                assert_eq!(plan["allowed_scopes"][index / 2], scope);
                let mut row_values = Vec::new();
                for (phase_index, phase) in ["seed", "transfer"].iter().enumerate() {
                    let row = &rows[index];
                    index += 1;
                    assert_eq!(row["task"], task);
                    assert_eq!(row["phase"], *phase);
                    assert_eq!(row["run"], run);
                    assert_eq!(row["shard"], shard);
                    assert_eq!(row["schema"], "spiralcore.wan.row.v3");
                    assert_eq!(row["suite"], "large16-localized");
                    assert_eq!(row["plan_trials"], 30);
                    assert_eq!(row["status"], "published");
                    assert!(row["error"].is_null());
                    assert_eq!(row["object_bytes"], 16777216);
                    assert_eq!(row["object"], objects[phase_index]);
                    assert_eq!(
                        row["source_revision"],
                        "f4e5e9afe6f3628ffe82e4553a88111f5bd6d126"
                    );
                    assert_eq!(row["os"], "linux");
                    assert_eq!(row["architecture"], "x86_64");
                    let binary = text(row, "executable_sha256");
                    assert_eq!(binary, expected_binary);
                    if let Some(old) = binaries.insert(shard, binary.to_owned()) {
                        assert_eq!(old, binary);
                    }
                    let m = &row["metrics"];
                    let prep = &m["preparation"];
                    let sparse = phase_index == 1 && arm == "sparse";
                    assert_eq!(num(m, "http_requests"), 1);
                    assert_eq!(prep["promotion_performed"], false);
                    assert_eq!(prep["retained_base_reused"], sparse);
                    assert_eq!(
                        num(prep, "selected_mode"),
                        if sparse {
                            3
                        } else if phase_index == 1 && arm == "zstd_full" {
                            1
                        } else {
                            0
                        }
                    );
                    assert_eq!(num(m, "records_sent"), 0);
                    let count: u64 = if sparse { 3 } else { 276 };
                    frames += count;
                    let lengths = count.checked_mul(4).unwrap();
                    assert_eq!(
                        num(m, "request_body_bytes"),
                        checked_sum(&[num(m, "application_request_bytes"), 12, lengths])
                    );
                    assert_eq!(
                        num(m, "response_body_bytes"),
                        checked_sum(&[num(m, "application_response_bytes"), 28, lengths])
                    );
                    assert_eq!(
                        checked_sum(&[num(m, "request_body_bytes"), num(m, "response_body_bytes")]),
                        if sparse { 600 } else { 16814026 }
                    );
                    if !sparse {
                        assert_eq!(num(m, "data_bytes"), 16777216);
                    }
                    let start = text(row, "unix_start_nanos").parse::<u128>().unwrap();
                    let end = text(row, "unix_end_nanos").parse::<u128>().unwrap();
                    assert!(end >= start);
                    assert!(
                        num(m, "wall_micros") > 0
                            && num(m, "sender_cpu_micros") > 0
                            && num(m, "receiver_cpu_micros") > 0
                    );
                    assert!(num(m, "receiver_wall_micros") <= num(m, "wall_micros"));
                    let rv = metrics(row);
                    for (key, value) in &rv {
                        values
                            .entry(arm.into())
                            .or_default()
                            .entry((*phase).into())
                            .or_default()
                            .entry(key.clone())
                            .or_default()
                            .push(*value);
                    }
                    row_values.push(rv);
                }
                for (key, value) in &row_values[0] {
                    values
                        .entry(arm.into())
                        .or_default()
                        .entry("seed_plus_transfer".into())
                        .or_default()
                        .entry(key.clone())
                        .or_default()
                        .push(value.checked_add(row_values[1][key]).unwrap());
                }
            }
        }
        captures.push(capture_review(&dir, &rows, &mut hashes, &mut ip_values));
    }
    assert_eq!(orders, [5; 6]);
    assert_eq!(global_scopes.len(), 90);
    assert_eq!(frames, 41490);
    let mut report = json!({});
    let complete_ip_campaign = captures.iter().all(|c| c["capture_complete"] == true);
    if complete_ip_campaign {
        for (arm, phases) in &ip_values {
            for (phase, metrics) in phases {
                for (metric, vector) in metrics {
                    assert_eq!(vector.len(), 30);
                    assert!(values
                        .get_mut(arm)
                        .unwrap()
                        .get_mut(phase)
                        .unwrap()
                        .insert(metric.clone(), vector.clone())
                        .is_none());
                }
            }
        }
    }
    if !complete_ip_campaign && !ip_values.is_empty() {
        let shards: Vec<_> = captures
            .iter()
            .enumerate()
            .filter_map(|(i, c)| (c["capture_complete"] == true).then_some(i))
            .collect();
        let trials: Vec<_> = shards.iter().flat_map(|s| s * 10..(s + 1) * 10).collect();
        let excluded: Vec<_> = captures
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                (c["capture_complete"] != true)
                    .then_some(json!({"shard":i,"reason":c["incomplete_reasons"]}))
            })
            .collect();
        report["descriptive_qualified_ip_subset"] = json!({"registered_30_pair_claim":false,"qualified_shards":shards,"global_trials":trials,"n":trials.len(),"excluded_shards":excluded,"selection_rule":"All and only complete capture shards, selected by recorded completeness rather than measured byte reduction; incomplete shard retained in full30-trial application/CPU/wall results","scope":"runner_to_cloudflare_relay_leg_only","limitation":"Post-run descriptive subset; does not satisfy complete30-pair IP qualification and is not a substitute for the registered full campaign"});
        for arm in arms {
            for phase in ["seed", "transfer", "seed_plus_transfer"] {
                for metric in ["upload_ip_bytes", "download_ip_bytes", "total_ip_bytes"] {
                    let v = &ip_values[arm][phase][metric];
                    assert_eq!(v.len(), trials.len());
                    report["descriptive_qualified_ip_subset"]["arms"][arm][phase][metric] =
                        summary(v);
                    if arm != "sparse" {
                        report["descriptive_qualified_ip_subset"]["comparisons"][arm][phase]
                            [metric] = comparison(v, &ip_values["sparse"][phase][metric]);
                        report["descriptive_qualified_ip_subset"]["reductions"][arm][phase]
                            [metric] = byte_reductions(v, &ip_values["sparse"][phase][metric]);
                    }
                }
            }
        }
    }
    let author_binaries: Vec<_> = binaries
        .iter()
        .map(|(shard, id)| (*shard, id.clone()))
        .collect();
    assert_eq!(author["binaries"], json!(author_binaries));
    let mut compared = 0;
    for arm in arms {
        for phase in ["seed", "transfer", "seed_plus_transfer"] {
            for (metric, v) in &values[arm][phase] {
                let s = summary(v);
                report["arms"][arm][phase][metric] = s.clone();
                if author["arms"][arm][phase].get(metric).is_some() {
                    same(&s, &author["arms"][arm][phase][metric]);
                    compared += 1;
                }
                if arm != "sparse" {
                    let c = comparison(v, &values["sparse"][phase][metric]);
                    report["comparisons"][arm][phase][metric] = c.clone();
                    if author["comparisons"][arm][phase].get(metric).is_some() {
                        same(&c, &author["comparisons"][arm][phase][metric]);
                        compared += 1;
                    }
                }
            }
        }
    }
    assert_eq!(compared, 75);
    let sparse = report["arms"]["sparse"]["transfer"]["body_bytes"]["median"]
        .as_f64()
        .unwrap();
    let raw = report["arms"]["raw_full"]["transfer"]["body_bytes"]["median"]
        .as_f64()
        .unwrap();
    let ratio = report["comparisons"]["raw_full"]["transfer"]["wall_micros"]["geometric_mean"]
        .as_f64()
        .unwrap();
    close(
        1. - sparse / raw,
        author["warm_gate"]["median_body_reduction"]
            .as_f64()
            .unwrap(),
    );
    close(
        ratio,
        author["warm_gate"]["paired_wall_ratio"].as_f64().unwrap(),
    );
    assert_eq!(
        author["warm_gate"]["performance_thresholds_pass"],
        sparse <= raw / 10. && ratio >= 2.
    );
    report["ip_measurement"] = json!({"qualified_all_three_shards":complete_ip_campaign,"scope":"runner_to_cloudflare_relay_leg_only","byte_layer":"IP header total lengths including TLS/HTTP/TCP overhead, ACKs and retransmissions; not physical Ethernet wire bytes","attribution":"Each row owns the complete observed TCP connection lifetime including delayed ACKs and teardown; setup and unattributed traffic remain separate campaign counters","paired_statistics":"Global trial order; midpoint median and nearest-rank p95; paired raw/sparse and zstd/sparse geometric means; seed plus transfer added per trial before summarizing","full_path_measured":false});
    if complete_ip_campaign {
        for arm in ["raw_full", "zstd_full"] {
            for phase in ["seed", "transfer", "seed_plus_transfer"] {
                for metric in ["upload_ip_bytes", "download_ip_bytes", "total_ip_bytes"] {
                    report["ip_reductions"][arm][phase][metric] = byte_reductions(
                        &values[arm][phase][metric],
                        &values["sparse"][phase][metric],
                    );
                }
            }
        }
    }
    report["schema"] = json!("spiralcore.independent-large-wan-review.v1");
    report["status"] = json!("passed_receipt_arithmetic_and_identity_checks");
    report["run"] = json!(run);
    report["source"] = json!("f4e5e9afe6f3628ffe82e4553a88111f5bd6d126");
    report["input_sha256"] = json!(hashes);
    report["binaries"] = json!(binaries);
    report["canonical_fixture_objects"] = json!(ids);
    report["frames_each_direction"] = json!(frames);
    report["rows"] = json!(180);
    report["shared_author_metric_records_checked"] = json!(compared);
    report["capture_reviews"] = json!(captures);
    report["warm_gate"] = author["warm_gate"].clone();
    report["limitations"]=json!(["Canonical fixture IDs regenerated independently and reopened verification receipts/checksums bound; independent analysis does not rerun persisted-byte comparison","Capture JSON partitions/body coverage and preserved offload/pinned-address evidence checked; independent raw PCAP replay is separate and does not reproduce individual connection ownership","Combined30-pair IP metrics require all three complete captures; any separately labeled complete-shard subset is descriptive and does not satisfy that full-campaign qualification","Measured runner-to-Cloudflare relay leg excludes relay-to-receiver traffic and is not full-path traffic","Receiver CPU covers protocol decode/handle/encode; sender plus receiver CPU is not energy","Application body savings do not measure complete network path bytes","Raw/zstd attempt bounded optional DAG warming; sparse skips it; this is not an equal-instruction CPU experiment"]);
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copied_aggregate_identity_and_row_count_fail_closed() {
        let run = "ab".repeat(32);
        let good = json!({"schema":"spiralcore.large-wan.analysis.v1","run":run,"source":"f4e5e9afe6f3628ffe82e4553a88111f5bd6d126","rows":180});
        author_identity(&good, &run);
        for (key, bad) in [
            ("schema", json!("wrong")),
            ("run", json!("cd".repeat(32))),
            ("source", json!("00".repeat(20))),
            ("rows", json!(179)),
        ] {
            let mut value = good.clone();
            value[key] = bad;
            assert!(
                std::panic::catch_unwind(|| author_identity(&value, &run)).is_err(),
                "{key}"
            );
        }
    }
    #[test]
    fn paired_lifecycle_statistics_preserve_all_trials() {
        let base: Vec<u64> = (1..=30).collect();
        let candidate: Vec<u64> = base.iter().map(|v| v * 2).collect();
        assert_eq!(summary(&base)["median"], 15.5);
        assert_eq!(summary(&base)["p95"], 29);
        let c = comparison(&base, &candidate);
        close(c["geometric_mean"].as_f64().unwrap(), 0.5);
        assert_eq!(c["candidate_slower"], 30);
        assert_eq!(
            byte_reductions(&base, &candidate)["aggregate_fraction_reduction"],
            -1.0
        );
    }
    #[test]
    fn malformed_external_counter_overflow_is_rejected() {
        assert!(std::panic::catch_unwind(|| checked_sum(&[u64::MAX, 1])).is_err());
        assert!(std::panic::catch_unwind(|| checked_sum(&[u64::MAX, 12, 1104])).is_err());
    }
    #[test]
    fn incomplete_campaign_subset_has_its_own_sample_count() {
        let v: Vec<u64> = (1..=20).collect();
        let s = summary(&v);
        assert_eq!(s["n"], 20);
        assert_eq!(s["median"], 10.5);
        assert_eq!(s["p95"], 19);
        assert_eq!(comparison(&v, &v)["ties"], 20);
        assert_eq!(byte_reductions(&v, &v)["aggregate_fraction_reduction"], 0.0);
    }
}
