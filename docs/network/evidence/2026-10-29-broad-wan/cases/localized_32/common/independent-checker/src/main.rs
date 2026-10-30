// Offline evidence review only. No production code, network, subprocesses or measured operations.
// Canonical fixture hashes and statistics are independently reconstructed from the registered spec.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Read,
    path::Path,
};
type Hashes = BTreeMap<String, String>;
type Cells = BTreeMap<(u64, String, String), [u64; 5]>;
const ARMS: [&str; 4] = ["raw_full", "zstd_full", "zstd_prefix", "sparse"];
const KEYS: [&str; 5] = [
    "wall_micros",
    "sender_cpu_micros",
    "receiver_cpu_micros",
    "combined_cpu_micros",
    "body_bytes",
];
const CASES: [&str; 12] = [
    "localized_32",
    "scattered_32x32",
    "prefix_4096",
    "middle_4096",
    "delete_4096",
    "append_4096",
    "truncate_4096",
    "replace_1m",
    "structured_localized_32",
    "compressed_localized_32",
    "cold_random",
    "cold_structured",
];
fn n(v: &Value, k: &str) -> u64 {
    v[k].as_u64().expect(k)
}
fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().expect(k)
}
fn arr(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
fn sum(v: impl IntoIterator<Item = u64>) -> u64 {
    v.into_iter()
        .try_fold(0u64, |a, b| a.checked_add(b))
        .expect("integer overflow")
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|v| format!("{v:02x}")).collect()
}
fn hash(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p);
    }
    h.finalize().into()
}
fn digest(b: &[u8]) -> String {
    hex(&hash(&[b]))
}
fn unhex(s: &str, len: usize) -> Vec<u8> {
    assert_eq!(s.len(), len * 2);
    assert!(s
        .bytes()
        .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v)));
    s.as_bytes()
        .chunks_exact(2)
        .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
        .collect()
}
fn read(p: &Path, h: &mut Hashes) -> Vec<u8> {
    let m = fs::symlink_metadata(p).unwrap();
    assert!(m.file_type().is_file() && m.len() <= 8 << 20);
    let mut b = Vec::new();
    File::open(p)
        .unwrap()
        .take((8 << 20) + 1)
        .read_to_end(&mut b)
        .unwrap();
    assert_eq!(b.len() as u64, m.len());
    assert!(b.len() <= 8 << 20);
    h.insert(p.display().to_string(), digest(&b));
    b
}
fn j(p: &Path, h: &mut Hashes) -> Value {
    serde_json::from_slice(&read(p, h)).unwrap()
}
fn txt(p: &Path, h: &mut Hashes) -> String {
    String::from_utf8(read(p, h)).unwrap()
}
fn file_digest(p: &Path, h: &mut Hashes) -> (u64, String) {
    let m = fs::symlink_metadata(p).unwrap();
    assert!(m.file_type().is_file() && m.len() <= 256 << 20);
    let mut f = File::open(p).unwrap().take((256 << 20) + 1);
    let mut b = [0; 65536];
    let mut sha = Sha256::new();
    let mut len = 0;
    loop {
        let count = f.read(&mut b).unwrap();
        if count == 0 {
            break;
        }
        len = sum([len, count as u64]);
        assert!(len <= 256 << 20);
        sha.update(&b[..count]);
    }
    assert_eq!(len, m.len());
    let d = hex(&sha.finalize());
    h.insert(p.display().to_string(), d.clone());
    (len, d)
}
fn equivalent(a: &Value, b: &Value) {
    match a {
        Value::Object(m) => {
            for (k, v) in m {
                equivalent(v, &b[k]);
            }
        }
        Value::Array(v) => {
            assert_eq!(v.len(), arr(b).len());
            for (x, y) in v.iter().zip(arr(b)) {
                equivalent(x, y)
            }
        }
        Value::Number(x) if x.is_f64() => {
            let x = x.as_f64().unwrap();
            let y = b.as_f64().unwrap();
            assert!(
                x.is_finite() && y.is_finite() && (x - y).abs() <= 1e-12 * x.abs().max(1.),
                "{x} != {y}"
            )
        }
        _ => assert_eq!(a, b),
    }
}
fn distribution(v: &[u64]) -> Value {
    assert!(!v.is_empty() && v.len() <= 30);
    let mut a = v.to_vec();
    a.sort_unstable();
    let len = a.len();
    let med = if len.is_multiple_of(2) {
        a[len / 2 - 1] as f64 / 2. + a[len / 2] as f64 / 2.
    } else {
        a[len / 2] as f64
    };
    json!({"n":len,"median":med,"p95":a[(95*len).div_ceil(100)-1],"min":a[0],"max":a[len-1],"sum":sum(v.iter().copied()),"values":v})
}
fn ratios(base: &[u64], candidate: &[u64]) -> Value {
    assert_eq!(base.len(), candidate.len());
    assert!(!base.is_empty() && !base.contains(&0) && !candidate.contains(&0));
    let ratios: Vec<_> = base
        .iter()
        .zip(candidate)
        .map(|(a, b)| *a as f64 / *b as f64)
        .collect();
    let wins = base.iter().zip(candidate).filter(|(a, b)| a > b).count();
    let ties = base.iter().zip(candidate).filter(|(a, b)| a == b).count();
    json!({"n":base.len(),"geometric_mean":(ratios.iter().map(|v|v.ln()).sum::<f64>()/base.len()as f64).exp(),"candidate_faster":wins,"ties":ties,"candidate_slower":base.len()-wins-ties,"ratios":ratios})
}
fn base(size: usize, seed: u64, structured: bool) -> Vec<u8> {
    let mut state = seed.max(1);
    let mut out = vec![0; size];
    for bytes in out.chunks_mut(8) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        bytes.copy_from_slice(&state.to_le_bytes()[..bytes.len()]);
    }
    if structured {
        let record = out[..4096].to_vec();
        for (i, page) in out.chunks_mut(4096).enumerate() {
            page.copy_from_slice(&record[..page.len()]);
            page[..8].copy_from_slice(&(i as u64 % 16).to_le_bytes());
        }
    }
    out
}
fn fixtures(case: &str, size: usize, seed: u64) -> (Vec<u8>, Vec<u8>) {
    assert!(CASES.contains(&case));
    let b = if case == "compressed_localized_32" {
        zstd::bulk::compress(&base(size - 4096, seed, false), 3).unwrap()
    } else {
        base(
            size,
            seed,
            matches!(case, "structured_localized_32" | "cold_structured"),
        )
    };
    let mut t = b.clone();
    let middle = t.len() / 2;
    match case {
        "localized_32" | "structured_localized_32" | "compressed_localized_32" => {
            for v in &mut t[middle..middle + 32] {
                *v ^= 0xa5
            }
        }
        "scattered_32x32" => {
            for i in 0..32 {
                for v in &mut t[i * size / 32..i * size / 32 + 32] {
                    *v ^= 0xa5
                }
            }
        }
        "prefix_4096" => {
            t.splice(0..0, [0x5a; 4096]);
        }
        "middle_4096" => {
            t.splice(middle..middle, [0x5a; 4096]);
        }
        "delete_4096" => {
            t.drain(middle..middle + 4096);
        }
        "append_4096" => t.extend_from_slice(&[0x5a; 4096]),
        "truncate_4096" => t.truncate(t.len() - 4096),
        "replace_1m" => {
            for v in &mut t[middle..middle + (1 << 20)] {
                *v ^= 0xa5
            }
        }
        "cold_random" | "cold_structured" => {}
        _ => unreachable!(),
    };
    (b, t)
}
fn object(b: &[u8]) -> String {
    hex(&hash(&[
        b"spiralcore.object.bytes.v1\0",
        &(b.len() as u64).to_le_bytes(),
        b,
    ]))
}
fn permutations() -> Vec<[usize; 4]> {
    let mut p = Vec::new();
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                for d in 0..4 {
                    if BTreeSet::from([a, b, c, d]).len() == 4 {
                        p.push([a, b, c, d]);
                    }
                }
            }
        }
    }
    assert_eq!(p.len(), 24);
    p
}
fn tasks(run: &[u8], case: &str, shard: u8) -> Vec<Value> {
    assert!(shard < 5 && CASES.contains(&case));
    let suite = format!("broad16-{case}");
    let size = 16 * 1024 * 1024
        - if matches!(case, "prefix_4096" | "middle_4096" | "append_4096") {
            4096
        } else {
            0
        };
    let warm = !case.starts_with("cold_");
    let p = permutations();
    let offset = hash(&[b"spiralcore.bench.order.v4\0", run, suite.as_bytes()])[0] as usize;
    let mut result = Vec::new();
    for trial in u32::from(shard) * 6..u32::from(shard + 1) * 6 {
        let seed = hash(&[
            b"spiralcore.bench.fixture.v4\0",
            run,
            case.as_bytes(),
            &trial.to_le_bytes(),
        ]);
        let seed = u64::from_le_bytes(seed[..8].try_into().unwrap()).max(1);
        for index in p[(offset + trial as usize) % 24] {
            let arm = ARMS[index];
            let pair = format!("{size}-{case}-{trial:02}");
            let id = format!("{pair}-{arm}");
            let scope = hex(&hash(&[
                b"spiralcore.bench.scope.v4\0",
                run,
                suite.as_bytes(),
                &[shard],
                id.as_bytes(),
            ]));
            result.push(json!({"id":id,"pair":pair,"trial":trial,"case":case,"size":size,"arm":arm,"scope":scope,"fixture_seed":seed,"seed_base":warm,"warm_probe":false,"cache_state":if warm{"base_present_target_absent"}else{"cold"}}));
        }
    }
    result
}
fn counters(v: &Value) -> [u64; 9] {
    std::array::from_fn(|i| {
        n(
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
fn counter_text(t: &str) -> [u64; 3] {
    std::array::from_fn(|i| {
        let suffix = [
            "packets captured",
            "packets received by filter",
            "packets dropped by kernel",
        ][i];
        let l: Vec<_> = t.lines().filter(|l| l.ends_with(suffix)).collect();
        assert_eq!(l.len(), 1);
        let v: Vec<_> = l[0].split_whitespace().collect();
        assert_eq!(v[1..].join(" "), suffix);
        v[0].parse().unwrap()
    })
}
fn quiet(samples: &[Value]) {
    assert!(samples.len() >= 2);
    for s in samples {
        assert_eq!(s["counters"], samples[0]["counters"]);
        assert_eq!(s["file_bytes"], samples[0]["file_bytes"]);
        assert_eq!(n(&s["counters"], "captured"), n(&s["counters"], "received"));
        assert_eq!(n(&s["counters"], "dropped"), 0);
    }
    for p in samples.windows(2) {
        assert!(
            n(&p[1], "elapsed_millis")
                .checked_sub(n(&p[0], "elapsed_millis"))
                .unwrap()
                >= 500
        );
    }
}
fn lifecycle_valid(l: &Value, cap: &Value, pcap_len: u64, pcap_sha: &str) -> bool {
    std::panic::catch_unwind(|| {
        assert_eq!(l["schema"], "spiralcore.capture-lifecycle.v1");
        assert_eq!(l["scope"], "selected_capture_interval_only");
        assert_eq!(l["local_ip"], cap["local_ip"]);
        assert_eq!(l["remote_ip"], cap["expected_remote_ip"]);
        assert_eq!(n(l, "port"), 443);
        assert_eq!(l["pcap_sha256"], pcap_sha);
        assert!(n(l, "capture_pid") > 1);
        assert!(!s(l, "interface").is_empty());
        unhex(s(l, "tcpdump_sha256"), 32);
        unhex(s(l, "stderr_sha256"), 32);
        assert!(!s(l, "tool_versions").is_empty());
        assert!(
            arr(&l["flags"]).contains(&json!("--immediate-mode"))
                && arr(&l["flags"]).contains(&json!("-U"))
        );
        assert_eq!(l["ready"], true);
        assert!(n(l, "ready_elapsed_millis") <= 10000);
        assert_eq!(l["client_exit_code"], 0);
        assert_eq!(l["capture_exit_code"], 0);
        assert_eq!(l["forced_cleanup"], false);
        assert!(arr(&l["errors"]).is_empty());
        let pre = arr(&l["preflight"]);
        let post = arr(&l["quiescence"]);
        assert_eq!(pre.len(), 2);
        assert!((3..=21).contains(&post.len()));
        quiet(pre);
        quiet(&post[post.len() - 3..]);
        let final_s = &l["final_snapshot"];
        let times = [
            n(l, "ready_elapsed_millis"),
            n(&pre[0], "elapsed_millis"),
            n(&pre[1], "elapsed_millis"),
            n(l, "client_started_elapsed_millis"),
            n(l, "client_ended_elapsed_millis"),
            n(&post[0], "elapsed_millis"),
            n(post.last().unwrap(), "elapsed_millis"),
            n(l, "interrupt_elapsed_millis"),
            n(final_s, "elapsed_millis"),
        ];
        assert!(times.windows(2).all(|p| p[0] <= p[1]));
        assert!(times[6] - times[4] <= 10000 && times[8] - times[7] <= 5000);
        let samples: Vec<_> = pre.iter().chain(post).chain([final_s]).collect();
        for (i, sample) in samples.iter().enumerate() {
            let c = &sample["counters"];
            assert_eq!(n(c, "dropped"), 0);
            assert!(n(sample, "stderr_end") <= 1 << 20);
            assert!((24..=256 << 20).contains(&n(sample, "file_bytes")));
            let text = s(sample, "stderr_text");
            assert!(text.len() <= 4096);
            let triple = if i + 1 == samples.len() {
                counter_text(text)
            } else {
                let value = text.trim().strip_prefix("tcpdump: ").unwrap_or(text.trim());
                let fields: Vec<_> = value.split(", ").collect();
                assert_eq!(fields.len(), 3);
                counter_text(&fields.join("\n"))
            };
            assert_eq!(
                triple,
                [n(c, "captured"), n(c, "received"), n(c, "dropped")]
            );
        }
        for p in samples.windows(2) {
            assert!(n(p[0], "elapsed_millis") <= n(p[1], "elapsed_millis"));
            assert_eq!(
                n(p[1], "stderr_end").checked_sub(n(p[0], "stderr_end")),
                Some(s(p[1], "stderr_text").len() as u64)
            );
            assert!(n(p[0], "file_bytes") <= n(p[1], "file_bytes"));
            for k in ["captured", "received", "dropped"] {
                assert!(n(&p[0]["counters"], k) <= n(&p[1]["counters"], k));
            }
        }
        assert_eq!(final_s["counters"], post.last().unwrap()["counters"]);
        assert_eq!(n(final_s, "file_bytes"), pcap_len);
        assert_eq!(final_s["file_bytes"], post.last().unwrap()["file_bytes"]);
        assert_eq!(
            n(&final_s["counters"], "captured"),
            n(cap, "tcpdump_captured_packets")
        );
        assert_eq!(
            n(&final_s["counters"], "received"),
            n(cap, "tcpdump_received_by_filter_packets")
        );
        assert_eq!(
            n(&final_s["counters"], "dropped"),
            n(cap, "kernel_reported_drops")
        );
    })
    .is_ok()
}
fn qualification(remote: &Path, controller: &str, h: &mut Hashes) {
    let q = j(&remote.join("capture-qualification/qualification.json"), h);
    assert_eq!(q["schema"], "spiralcore.capture-qualification.v2");
    assert_eq!(q["passed"], true);
    assert_eq!(q["controller_sha256"], controller);
    assert_eq!(arr(&q["cases"]).len(), 3);
    for (case, expected) in arr(&q["cases"])
        .iter()
        .zip(["normal", "interrupted", "ongoing"])
    {
        assert_eq!(case["case"], expected);
        assert_eq!(case["passed"], true);
        assert_eq!(case["cleanup_success"], true);
        assert!(
            case["cleanup_error"].is_null()
                && case["error"].is_null()
                && case["server_error"].is_null()
        );
        assert_eq!(
            case["observation"]["expected_success"],
            expected == "normal"
        );
        assert_eq!(
            case["observation"]["controller_success"],
            expected == "normal"
        );
        assert_eq!(
            case["observation"]["capture_complete"],
            expected == "normal"
        );
        let l = j(
            &remote.join(format!(
                "capture-qualification/{expected}/capture-lifecycle.json"
            )),
            h,
        );
        assert_eq!(
            case["observation"]["lifecycle_sha256"],
            h[&remote
                .join(format!(
                    "capture-qualification/{expected}/capture-lifecycle.json"
                ))
                .display()
                .to_string()]
        );
        assert_eq!(l["local_ip"], "198.18.0.1");
        assert_eq!(l["remote_ip"], "198.18.0.2");
        assert_eq!(n(&l, "port"), 443);
    }
}
// The full source rows have already passed canonical task, source and object validation.
// This join binds a capture observation to exactly one such source operation.
fn capture_receipt<'a>(capture: &Value, rows: &'a [Value]) -> &'a Value {
    let id = s(&capture["task"], "id");
    let phase = s(capture, "phase");
    let mut matching = rows
        .iter()
        .filter(|r| s(&r["task"], "id") == id && s(r, "phase") == phase);
    let row = matching
        .next()
        .expect("capture operation has no source receipt");
    assert!(
        matching.next().is_none(),
        "ambiguous capture/source operation join"
    );
    assert_eq!(capture["task"], json!({"id": row["task"]["id"]}));
    assert_eq!(capture["status"], row["status"]);
    assert_eq!(capture["unix_start_nanos"], row["unix_start_nanos"]);
    assert_eq!(capture["unix_end_nanos"], row["unix_end_nanos"]);
    let m = &row["metrics"];
    assert_eq!(
        n(&capture["expected_application_body_bytes"], "upload"),
        n(m, "request_body_bytes")
    );
    assert_eq!(
        n(&capture["expected_application_body_bytes"], "download"),
        n(m, "response_body_bytes")
    );
    row
}
fn capture(dir: &Path, rows: &[Value], h: &mut Hashes) -> Value {
    let remote = dir.join("remote");
    let cap = j(&remote.join("capture.json"), h);
    assert_eq!(cap["schema"], "spiralcore.pcap-accounting.v1");
    assert_eq!(cap["scope"], "runner_to_relay_leg_only");
    assert_eq!(cap["partition_sum_verified"], true);
    let mut total = counters(&cap["setup"]);
    add(&mut total, counters(&cap["unattributed"]));
    let captures = arr(&cap["rows"]);
    assert_eq!(captures.len(), rows.len());
    let mut seen = BTreeSet::new();
    let mut covers = true;
    for c in captures {
        let id = s(&c["task"], "id");
        let phase = s(c, "phase");
        assert!(seen.insert((id, phase)));
        let row = capture_receipt(c, rows);
        let m = &row["metrics"];
        let covered = n(c, "upload_ip_bytes") >= n(m, "request_body_bytes")
            && n(c, "download_ip_bytes") >= n(m, "response_body_bytes");
        assert_eq!(c["application_body_coverage_verified"], covered);
        covers &= covered && n(c, "packets_within_measurement") > 0;
        let cc = counters(c);
        assert_eq!(sum(cc[..3].iter().copied()), cc[3]);
        assert_eq!(sum(cc[4..7].iter().copied()), cc[7]);
        add(&mut total, cc);
    }
    assert_eq!(total, counters(&cap["campaign"]));
    let c = counter_text(&txt(&remote.join("tcpdump-stats.txt"), h));
    assert_eq!(
        c,
        [
            n(&cap, "tcpdump_captured_packets"),
            n(&cap, "tcpdump_received_by_filter_packets"),
            n(&cap, "kernel_reported_drops")
        ]
    );
    assert_eq!(
        n(&cap, "capture_counter_difference_packets"),
        c[0].abs_diff(c[1])
    );
    assert_eq!(
        cap["pcap_record_count_matches_tcpdump_captured"],
        c[0] == total[7]
    );
    let (pcap_len, pcap_sha) = file_digest(&remote.join("headers.pcap"), h);
    let lifecycle = j(&remote.join("capture-lifecycle.json"), h);
    assert_eq!(
        cap["capture_lifecycle_sha256"],
        h[&remote.join("capture-lifecycle.json").display().to_string()]
    );
    let lifecycle_good = lifecycle_valid(&lifecycle, &cap, pcap_len, &pcap_sha);
    assert_eq!(cap["capture_lifecycle_verified"], lifecycle_good);
    let offload = txt(&remote.join("offload-state.txt"), h);
    for feature in [
        "tcp-segmentation-offload",
        "generic-segmentation-offload",
        "generic-receive-offload",
        "large-receive-offload",
    ] {
        let key = format!("{feature}: ");
        let lines: Vec<_> = offload
            .lines()
            .filter_map(|l| l.strip_prefix(&key))
            .collect();
        assert_eq!(lines.len(), 1);
        assert!(matches!(lines[0], "off" | "off [fixed]"));
    }
    let pinned = txt(&remote.join("pinned-addresses.txt"), h);
    assert!(!pinned.trim().is_empty());
    for l in pinned.lines() {
        assert_eq!(
            l.split_whitespace().next().unwrap(),
            s(&cap, "expected_remote_ip")
        );
    }
    let possible = lifecycle_good
        && covers
        && c == [total[7], total[7], 0]
        && n(&cap, "ambiguous_connections") == 0
        && n(&cap["unattributed"], "packets") == 0
        && total[6] == 0
        && total[8] == 0;
    let complete = cap["capture_complete"].as_bool().unwrap();
    if complete {
        assert!(possible);
        assert!(arr(&cap["incomplete_reasons"]).is_empty());
        assert_eq!(
            cap["observed_remote_ips"],
            json!([cap["expected_remote_ip"]])
        );
    } else {
        assert!(!arr(&cap["incomplete_reasons"]).is_empty());
    }
    json!({"capture_complete":complete,"incomplete_reasons":cap["incomplete_reasons"],"campaign":cap["campaign"],"pcap_sha256":pcap_sha,"pcap_file_bytes":pcap_len,"captured_packets":c[0],"received_packets":c[1],"kernel_drops":c[2],"independently_checked_lifecycle":lifecycle_good,"scope":"runner_to_relay_leg_only; requires separate independent PCAP packet-total replay; production SYN ownership is not independently duplicated"})
}
fn binary_list(t: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in t.lines() {
        let p: Vec<_> = line.split_whitespace().collect();
        assert_eq!(p.len(), 2);
        unhex(p[0], 32);
        assert!(out.insert(p[1].to_owned(), p[0].to_owned()).is_none());
    }
    assert_eq!(out.len(), 3);
    out
}
fn custody(
    dir: &Path,
    source: &str,
    qualified: &Value,
    manifest: &Value,
    h: &mut Hashes,
) -> String {
    let remote = dir.join("remote");
    assert_eq!(j(&dir.join("linux-qualified.json"), h), *qualified);
    assert_eq!(
        j(&dir.join("frozen-producer-build-manifest.json"), h),
        *manifest
    );
    assert_eq!(
        read(&dir.join("frozen-producer-build-manifest.json"), h),
        read(&remote.join("producer-build-manifest.json"), h)
    );
    let reuse = j(&remote.join("build-reuse-validation.json"), h);
    let proof = j(&remote.join("producer-proof-validation.json"), h);
    let archive = j(&remote.join("sender-archive-validation.json"), h);
    let local = j(&dir.join("local-producer-reuse-validation.json"), h);
    assert_eq!(reuse["schema"], "spiralcore.wan-build-reuse-validation.v1");
    assert_eq!(reuse["status"], "passed");
    assert_eq!(reuse["manifest"], *manifest);
    assert_eq!(proof["schema"], "spiralcore.wan-build-proof-validation.v1");
    assert_eq!(proof["status"], "passed");
    assert_eq!(proof["producer_run"], reuse["producer_run"]);
    assert_eq!(proof["artifact"], reuse["artifact"]);
    let r = &reuse["producer_run"];
    assert_eq!(r["head_sha"], source);
    assert_eq!(r["run_attempt"], 1);
    assert_eq!(r["path"], ".github/workflows/spiralcore-wan.yml");
    for key in ["repository", "head_repository"] {
        assert_eq!(r[key], "R4-Laboratories/SpiralCore");
    }
    assert_eq!(r["status"], "completed");
    assert_eq!(r["conclusion"], "success");
    assert_eq!(r["capture_successes"], 1);
    assert_eq!(r["source_build_successes"], 1);
    let id_string = |v: &Value| {
        if let Some(s) = v.as_str() {
            s.to_owned()
        } else {
            v.as_u64().unwrap().to_string()
        }
    };
    assert_eq!(
        id_string(&r["id"]),
        id_string(&qualified["producer"]["run_id"])
    );
    let a = &reuse["artifact"];
    assert_eq!(a["workflow_run_id"], r["id"]);
    assert_eq!(
        id_string(&a["id"]),
        id_string(&qualified["producer"]["artifact_id"])
    );
    assert_eq!(a["digest"], qualified["producer"]["artifact_digest"]);
    assert_eq!(a["head_sha"], source);
    assert_eq!(a["expired"], false);
    assert_eq!(a["name"], format!("spiralcore-wan-sender-{source}"));
    assert!((1..=100 << 20).contains(&n(a, "size_in_bytes")));
    let sha = s(a, "digest").strip_prefix("sha256:").unwrap();
    unhex(sha, 32);
    assert_eq!(archive["schema"], "spiralcore.sender-archive-validation.v1");
    assert_eq!(archive["status"], "passed");
    assert_eq!(archive["artifact_id"], a["id"]);
    assert_eq!(archive["archive_sha256"], sha);
    assert_eq!(archive["archive_bytes"], a["size_in_bytes"]);
    assert_eq!(
        local["schema"],
        "spiralcore.local-producer-reuse-validation.v1"
    );
    assert_eq!(local["status"], "passed");
    assert_eq!(local["source"], source);
    assert_eq!(s(&local["producer"], "run_id"), id_string(&r["id"]));
    assert_eq!(s(&local["producer"], "artifact_id"), id_string(&a["id"]));
    assert_eq!(local["producer"]["artifact_digest"], a["digest"]);
    assert_eq!(
        local["producer"]["manifest_sha256"],
        h[&dir
            .join("frozen-producer-build-manifest.json")
            .display()
            .to_string()]
    );
    let expected = j(&remote.join("expected-build.json"), h);
    assert_eq!(expected["identity"], manifest["identity"]);
    assert_eq!(expected["artifact_id"], a["id"]);
    assert_eq!(
        txt(&remote.join("rust-version.txt"), h).trim(),
        s(&manifest["identity"], "compiler")
    );
    let features = read(&remote.join("features.txt"), h);
    assert_eq!(manifest["identity"]["features_sha256"], digest(&features));
    assert_eq!(manifest["identity"]["target"], "x86_64-unknown-linux-gnu");
    assert_eq!(manifest["identity"]["source"], source);
    assert_eq!(manifest["identity"]["run_id"], r["id"]);
    for key in ["source_tests_passed", "clippy_passed", "capture_passed"] {
        assert_eq!(manifest["qualification"][key], true);
    }
    assert_eq!(
        manifest["qualification"]["capture_schema"],
        "spiralcore.capture-qualification.v2"
    );
    assert_eq!(manifest["schema"], "spiralcore.wan-sender-build.v1");
    let binaries = binary_list(&txt(&remote.join("binary-sha256.txt"), h));
    assert_eq!(arr(&manifest["binaries"]).len(), 3);
    let mut names = BTreeSet::new();
    for b in arr(&manifest["binaries"]) {
        let name = s(b, "name");
        assert!(names.insert(name));
        assert_eq!(binaries[&format!("target/release/{name}")], s(b, "sha256"));
        assert!((1..=32 << 20).contains(&n(b, "bytes")));
    }
    qualification(
        &remote,
        &binaries["target/release/spiralcore-capture-control"],
        h,
    );
    binaries["target/release/spiralcore-bench"].clone()
}
fn row_metrics(
    row: &Value,
    task: &Value,
    phase: &str,
    bytes: &[u8],
    source: &str,
    binary: &str,
) -> [u64; 5] {
    assert_eq!(row["task"], *task);
    assert_eq!(row["phase"], phase);
    assert_eq!(row["schema"], "spiralcore.wan.row.v4");
    assert_eq!(row["cold_graph_warming"], false);
    assert_eq!(row["status"], "published");
    assert!(row["error"].is_null());
    assert_eq!(row["object"], object(bytes));
    assert_eq!(n(row, "object_bytes"), bytes.len() as u64);
    assert_eq!(row["source_revision"], source);
    assert_eq!(row["executable_sha256"], binary);
    assert_eq!(row["os"], "linux");
    assert_eq!(row["architecture"], "x86_64");
    let start = s(row, "unix_start_nanos").parse::<u128>().unwrap();
    let end = s(row, "unix_end_nanos").parse::<u128>().unwrap();
    assert!(start < end);
    let m = &row["metrics"];
    let prep = &m["preparation"];
    let mode = n(prep, "selected_mode");
    let arm = s(task, "arm");
    let warm = task["seed_base"] == true;
    let expected = if phase == "seed" || arm == "raw_full" || (!warm && arm == "sparse") {
        vec![0]
    } else if arm == "zstd_full" || (!warm && arm == "zstd_prefix") {
        vec![1]
    } else if arm == "sparse" {
        vec![0, 3]
    } else {
        vec![0, 4]
    };
    assert!(expected.contains(&mode));
    assert_eq!(prep["promotion_performed"], false);
    assert_eq!(
        prep["retained_base_reused"],
        phase == "transfer" && warm && matches!(arm, "sparse" | "zstd_prefix")
    );
    assert_eq!(n(m, "records_sent"), 0);
    assert_eq!(n(m, "http_requests"), 1);
    let app_request = n(m, "application_request_bytes");
    let app_response = n(m, "application_response_bytes");
    let extra = n(m, "request_body_bytes")
        .checked_sub(sum([app_request, 12]))
        .unwrap();
    assert_eq!(extra % 4, 0);
    let frames = extra / 4;
    assert!((3..=512).contains(&frames));
    assert_eq!(
        n(m, "response_body_bytes"),
        sum([app_response, 28, 4 * frames])
    );
    let chunks = frames - 2;
    assert_eq!(app_response, 154 + 64 * chunks);
    assert_eq!(
        app_request,
        sum([
            158,
            if mode >= 3 { 32 } else { 0 },
            61 * chunks,
            n(m, "data_bytes")
        ])
    );
    if mode <= 1 {
        assert_eq!(chunks, (bytes.len() as u64).div_ceil(61440));
        let payload = if mode == 0 {
            bytes.len() as u64
        } else {
            bytes
                .chunks(61440)
                .map(|c| zstd::bulk::compress(c, 1).unwrap().len().min(c.len()) as u64)
                .sum()
        };
        assert_eq!(n(m, "data_bytes"), payload);
    }
    let wall = n(m, "wall_micros");
    let sender = n(m, "sender_cpu_micros");
    let receiver = n(m, "receiver_cpu_micros");
    assert!(wall > 0 && sender > 0 && receiver > 0);
    assert!(n(m, "receiver_wall_micros") <= wall);
    [
        wall,
        sender,
        receiver,
        sum([sender, receiver]),
        sum([n(m, "request_body_bytes"), n(m, "response_body_bytes")]),
    ]
}
fn aggregate(cells: &Cells, warm: bool) -> Value {
    let phases = if warm {
        vec!["seed", "transfer", "seed_plus_transfer"]
    } else {
        vec!["transfer"]
    };
    let vector = |arm: &str, phase: &str, k: usize| -> Vec<u64> {
        (0..30)
            .map(|t| {
                let key = |p: &str| (t, arm.to_owned(), p.to_owned());
                match phase {
                    "seed" => cells[&key("seed")][k],
                    "transfer" => cells[&key("transfer")][k],
                    _ => sum([cells[&key("seed")][k], cells[&key("transfer")][k]]),
                }
            })
            .collect()
    };
    let mut result = json!({"arms":{},"comparisons":{}});
    for arm in ARMS {
        for phase in &phases {
            for (k, key) in KEYS.iter().enumerate() {
                result["arms"][arm][*phase][*key] = distribution(&vector(arm, phase, k));
                if arm != "sparse" {
                    result["comparisons"][arm][*phase][*key] =
                        ratios(&vector(arm, phase, k), &vector("sparse", phase, k));
                }
            }
        }
    }
    if warm {
        let raw = result["arms"]["raw_full"]["transfer"]["body_bytes"]["median"]
            .as_f64()
            .unwrap();
        let sparse = result["arms"]["sparse"]["transfer"]["body_bytes"]["median"]
            .as_f64()
            .unwrap();
        let ratio = result["comparisons"]["raw_full"]["transfer"]["wall_micros"]["geometric_mean"]
            .as_f64()
            .unwrap();
        let median_ratio = result["arms"]["raw_full"]["transfer"]["wall_micros"]["median"]
            .as_f64()
            .unwrap()
            / result["arms"]["sparse"]["transfer"]["wall_micros"]["median"]
                .as_f64()
                .unwrap();
        result["warm_gate"] = json!({"median_body_reduction":1.-sparse/raw,"paired_wall_ratio":ratio,"median_wall_ratio":median_ratio,"performance_thresholds_pass":sparse<=0.1*raw&&median_ratio>=2.,"requires_separate_exact_reopen_verification":true});
    } else {
        result["warm_gate"] = Value::Null;
        for arm in ARMS {
            let a = &result["arms"][arm]["transfer"];
            let b = &result["arms"]["raw_full"]["transfer"];
            result["cold_gates"][arm] = json!({"body_within_5pct":a["body_bytes"]["median"].as_f64().unwrap()<=b["body_bytes"]["median"].as_f64().unwrap()*1.05,"p95_within_10pct":a["wall_micros"]["p95"].as_f64().unwrap()<=b["wall_micros"]["p95"].as_f64().unwrap()*1.10});
        }
    }
    result
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--check-plan") {
        assert_eq!(args.len(), 3);
        let mut hashes = Hashes::new();
        let plan = j(Path::new(&args[2]), &mut hashes);
        let run = unhex(s(&plan, "run"), 32);
        let case = s(&plan, "suite").strip_prefix("broad16-").unwrap();
        let shard = u8::try_from(n(&plan, "shard")).unwrap();
        let want = tasks(&run, case, shard);
        assert_eq!(plan["schema"], "spiralcore.wan.plan.v4");
        assert_eq!(plan["cold_graph_warming"], false);
        assert_eq!(plan["tasks"], json!(want));
        assert_eq!(
            plan["allowed_scopes"],
            json!(want.iter().map(|t| &t["scope"]).collect::<Vec<_>>())
        );
        println!(
            "{}",
            json!({"status":"canonical_plan_matches","tasks":want.len(),"input_sha256":hashes})
        );
        return;
    }
    assert_eq!(
        args.len(),
        5,
        "campaign root, registered case, author aggregate JSON and independent PCAP replay JSON required"
    );
    let root = Path::new(&args[1]);
    let case = &args[2];
    assert!(CASES.contains(&case.as_str()));
    let suite = format!("broad16-{case}");
    let warm = !case.starts_with("cold_");
    let mut hashes = Hashes::new();
    let run = txt(&root.join("run-id.txt"), &mut hashes).trim().to_owned();
    let run_bytes = unhex(&run, 32);
    let source = txt(&root.join("source-revision.txt"), &mut hashes)
        .trim()
        .to_owned();
    unhex(&source, 20);
    let qualified = j(
        &root.join("qualification/linux-qualified.json"),
        &mut hashes,
    );
    let manifest = j(
        &root.join("qualification/producer-build-manifest.json"),
        &mut hashes,
    );
    assert_eq!(qualified["status"], "passed");
    assert_eq!(qualified["source"], source);
    let author = j(Path::new(&args[3]), &mut hashes);
    let pcap_review = j(Path::new(&args[4]), &mut hashes);
    assert_eq!(
        pcap_review["schema"],
        "spiralcore.independent-broad-wan-pcap-review.v1"
    );
    assert_eq!(
        pcap_review["status"],
        "campaign_totals_match_raw_header_replay"
    );
    assert_eq!(arr(&pcap_review["shards"]).len(), 5);
    assert_eq!(author["schema"], "spiralcore.broad-wan.analysis.v1");
    assert_eq!(author["run"], run);
    assert_eq!(author["source"], source);
    assert_eq!(author["suite"], suite);
    assert_eq!(author["cold_graph_warming"], false);
    assert_eq!(n(&author, "rows"), if warm { 240 } else { 120 });
    let mut cells = Cells::new();
    let mut captures = Vec::new();
    let mut ip_cells = BTreeMap::new();
    let mut identities = BTreeMap::new();
    let mut expected_binary = None;
    let mut byte_total = 0u64;
    let mut previous_end = 0u128;
    let mut scopes = BTreeSet::new();
    for shard in 0u8..5 {
        let dir = root.join(format!("campaign/{case}/shard-{shard}"));
        let remote = dir.join("remote");
        let binary = custody(&dir, &source, &qualified, &manifest, &mut hashes);
        if let Some(old) = expected_binary.replace(binary.clone()) {
            assert_eq!(old, binary);
        }
        assert_eq!(
            txt(&remote.join("source-revision.txt"), &mut hashes).trim(),
            source
        );
        let local_plan = read(&dir.join("plan.json"), &mut hashes);
        assert_eq!(local_plan, read(&remote.join("plan.json"), &mut hashes));
        let plan: Value = serde_json::from_slice(&local_plan).unwrap();
        let task_list = tasks(&run_bytes, case, shard);
        assert_eq!(plan["schema"], "spiralcore.wan.plan.v4");
        assert_eq!(plan["run"], run);
        assert_eq!(plan["suite"], suite);
        assert_eq!(plan["shard"], shard);
        assert_eq!(plan["cold_graph_warming"], false);
        assert_eq!(plan["trials"], 30);
        assert_eq!(plan["fixture_version"], "xorshift64-v1-broad-v1");
        assert_eq!(plan["tasks"], json!(task_list));
        assert_eq!(
            plan["allowed_scopes"],
            json!(task_list.iter().map(|t| &t["scope"]).collect::<Vec<_>>())
        );
        assert_eq!(
            n(&plan, "retained_payload_bytes"),
            24 * (1 + u64::from(warm)) * 16777216
        );
        let raw = txt(&remote.join("rows.jsonl"), &mut hashes);
        let rows: Vec<Value> = raw
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(rows.len(), 24 * (1 + usize::from(warm)));
        let mut i = 0;
        let mut shard_bytes = 0;
        for task in &task_list {
            assert!(scopes.insert(s(task, "scope").to_owned()));
            let trial = n(task, "trial");
            let (b, t) = fixtures(case, n(task, "size") as usize, n(task, "fixture_seed"));
            identities.insert(trial,json!({"base":object(&b),"target":object(&t),"base_bytes":b.len(),"target_bytes":t.len(),"seed":n(task,"fixture_seed")}));
            let phases = if warm {
                vec![("seed", b.as_slice()), ("transfer", t.as_slice())]
            } else {
                vec![("transfer", t.as_slice())]
            };
            for (phase, bytes) in phases {
                let row = &rows[i];
                i += 1;
                assert_eq!(row["run"], run);
                assert_eq!(row["suite"], suite);
                assert_eq!(row["shard"], shard);
                assert_eq!(row["plan_trials"], 30);
                let start = s(row, "unix_start_nanos").parse::<u128>().unwrap();
                assert!(start >= previous_end);
                previous_end = s(row, "unix_end_nanos").parse().unwrap();
                let values = row_metrics(row, task, phase, bytes, &source, &binary);
                assert!(cells
                    .insert((trial, s(task, "arm").to_owned(), phase.to_owned()), values)
                    .is_none());
                shard_bytes = sum([shard_bytes, bytes.len() as u64]);
            }
        }
        let verification = j(&dir.join("verification.json"), &mut hashes);
        equivalent(
            &json!({"schema":"spiralcore.wan.verification.v4","run":run,"suite":suite,"shard":shard,"cold_graph_warming":false,"status":"exact","receipts_verified":rows.len(),"bytes_compared":shard_bytes,"qualification_outside_transfer_timing":true}),
            &verification,
        );
        let publications = txt(&dir.join("publication-files.sha256"), &mut hashes);
        let verification_text = txt(&dir.join("publication-hash-verification.txt"), &mut hashes);
        assert!(!publications.trim().is_empty());
        assert_eq!(
            publications.lines().count(),
            verification_text.lines().count()
        );
        for (entry, verified) in publications.lines().zip(verification_text.lines()) {
            let (sha, path) = entry.split_once("  ").expect("publication SHA format");
            unhex(sha, 32);
            assert!(Path::new(path).starts_with(dir.join("cache")));
            assert_eq!(verified, format!("{path}: OK"));
        }
        assert_eq!(
            j(&dir.join("cache-cleanup.json"), &mut hashes)["status"],
            "removed_after_exact_reopen_and_hashing"
        );
        byte_total = sum([byte_total, shard_bytes]);
        let cap = capture(&dir, &rows, &mut hashes);
        let packet_rows = j(&remote.join("capture.json"), &mut hashes);
        for r in arr(&packet_rows["rows"]) {
            let task = &capture_receipt(r, &rows)["task"];
            assert!(ip_cells
                .insert(
                    (
                        n(task, "trial"),
                        s(task, "arm").to_owned(),
                        s(r, "phase").to_owned()
                    ),
                    [
                        n(r, "upload_ip_bytes"),
                        n(r, "download_ip_bytes"),
                        n(r, "total_ip_bytes")
                    ]
                )
                .is_none());
        }
        let replay = &pcap_review["shards"][usize::from(shard)];
        assert_eq!(replay["shard"], shard);
        assert_eq!(
            replay["capture_report_sha256"],
            hashes[&remote.join("capture.json").display().to_string()]
        );
        assert_eq!(replay["pcap_sha256"], cap["pcap_sha256"]);
        assert_eq!(replay["pcap_file_bytes"], cap["pcap_file_bytes"]);
        assert_eq!(replay["records"], cap["campaign"]["packets"]);
        assert_eq!(
            replay["capture_complete_as_reported"],
            cap["capture_complete"]
        );
        assert_eq!(replay["incomplete_reasons"], cap["incomplete_reasons"]);
        for key in [
            "upload_ip_bytes",
            "download_ip_bytes",
            "total_ip_bytes",
            "upload_packets",
            "download_packets",
        ] {
            assert_eq!(replay[key], cap["campaign"][key]);
        }
        captures.push(cap);
    }
    assert_eq!(cells.len(), if warm { 240 } else { 120 });
    assert_eq!(scopes.len(), 120);
    let computed = aggregate(&cells, warm);
    equivalent(&computed, &author);
    assert_eq!(
        author["binaries"],
        json!((0..5)
            .map(|shard| json!([shard, expected_binary]))
            .collect::<Vec<_>>())
    );
    let all_complete = captures.iter().all(|c| c["capture_complete"] == true);
    let mut ip = Value::Null;
    if all_complete {
        let phases = if warm {
            vec!["seed", "transfer", "seed_plus_transfer"]
        } else {
            vec!["transfer"]
        };
        let vector = |arm: &str, phase: &str, k: usize| -> Vec<u64> {
            (0..30)
                .map(|trial| {
                    let key = |p: &str| (trial, arm.to_owned(), p.to_owned());
                    if phase == "seed_plus_transfer" {
                        sum([ip_cells[&key("seed")][k], ip_cells[&key("transfer")][k]])
                    } else {
                        ip_cells[&key(phase)][k]
                    }
                })
                .collect()
        };
        for arm in ARMS {
            for phase in &phases {
                for (k, metric) in ["upload_ip_bytes", "download_ip_bytes", "total_ip_bytes"]
                    .iter()
                    .enumerate()
                {
                    ip["arms"][arm][*phase][*metric] = distribution(&vector(arm, phase, k));
                    if arm != "sparse" {
                        ip["comparisons"][arm][*phase][*metric] =
                            ratios(&vector(arm, phase, k), &vector("sparse", phase, k));
                    }
                }
            }
        }
    }
    let result = json!({"schema":"spiralcore.independent-broad-wan-review.v1","status":"raw_receipts_and_author_statistics_agree","run":run,"source":source,"case":case,"rows":cells.len(),"exact_receipt_bytes":byte_total,"identities":identities,"sender_sha256":expected_binary,"captures":captures,"all_five_captures_complete":all_complete,"statistics":computed,"complete_case_ip_statistics":ip,"input_sha256":hashes,"limitations":["Offline independent fixture construction, flat object SHA256, canonical task/plan checks, protocol-body arithmetic and numerical analysis; no production benchmark or analyzer code linked","Reopened bytes were verified by the frozen receiver executable before owned cache removal; this review binds and cross-checks its receipt, not a second reopening after deletion","Capture JSON partition, body coverage, counter texts and bounded lifecycle independently checked; independent PCAP packet-total replay bound by report and raw-PCAP hashes; TCP SYN ownership remains production-parser evidence","HTTP bodies are not wire bytes. Any incomplete shard suppresses a complete 30-pair IP claim. No post hoc subset is silently substituted","Combined CPU adds sender and receiver handler CPU before summaries; excludes receiver transport, relay and tunnel overhead; not energy","Fixture generation excluded from measured wall. Raw/zstd encodings are independently recomputed only during offline verification."]});
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_capture_projection_binds_full_source_receipt_and_rejects_mutations() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        let capture = j(
            &root.join("localized-real-capture-row.json"),
            &mut Hashes::new(),
        );
        let receipt = j(
            &root.join("localized-real-receipt.json"),
            &mut Hashes::new(),
        );
        let rows = [receipt.clone()];
        assert_eq!(capture["task"], json!({"id": receipt["task"]["id"]}));
        let joined = capture_receipt(&capture, &rows);
        assert_eq!(joined["task"], receipt["task"]);
        assert_eq!(n(&joined["task"], "trial"), 0);
        assert_eq!(s(&joined["task"], "arm"), "sparse");
        for mutation in 0..7 {
            let mut bad = capture.clone();
            match mutation {
                0 => bad["task"]["id"] = "wrong-id".into(),
                1 => bad["phase"] = "transfer".into(),
                2 => bad["unix_start_nanos"] = "0".into(),
                3 => bad["unix_end_nanos"] = "0".into(),
                4 => bad["expected_application_body_bytes"]["upload"] = 0.into(),
                5 => bad["task"]["trial"] = 0.into(),
                _ => bad["status"] = "failed".into(),
            }
            assert!(std::panic::catch_unwind(|| capture_receipt(&bad, &rows)).is_err());
        }
        let duplicate = [receipt.clone(), receipt];
        assert!(std::panic::catch_unwind(|| capture_receipt(&capture, &duplicate)).is_err());
    }
    #[test]
    fn minority_wall_outliers_cannot_substitute_geometric_mean_for_median_gate() {
        let mut cells = Cells::new();
        for trial in 0..30 {
            for arm in ARMS {
                let raw = arm == "raw_full";
                let wall = if raw {
                    if trial < 6 {
                        60_000
                    } else {
                        600
                    }
                } else {
                    400
                };
                cells.insert(
                    (trial, arm.into(), "seed".into()),
                    [100, 20, 20, 40, 10_000],
                );
                cells.insert(
                    (trial, arm.into(), "transfer".into()),
                    [wall, 20, 20, 40, if raw { 10_000 } else { 100 }],
                );
            }
        }
        let result = aggregate(&cells, true);
        let gate = &result["warm_gate"];
        assert!(gate["paired_wall_ratio"].as_f64().unwrap() > 2.0);
        assert!(gate["median_body_reduction"].as_f64().unwrap() >= 0.9);
        assert_eq!(gate["performance_thresholds_pass"], false);
        assert_eq!(gate["median_wall_ratio"], 1.5);
        // The unchanged threshold is inclusive at a ratio of exactly two medians.
        for trial in 0..30 {
            cells
                .get_mut(&(trial, "sparse".into(), "transfer".into()))
                .unwrap()[0] = 300;
        }
        let at_threshold = aggregate(&cells, true);
        assert_eq!(at_threshold["warm_gate"]["median_wall_ratio"], 2.0);
        assert_eq!(
            at_threshold["warm_gate"]["performance_thresholds_pass"],
            true
        );
    }
    #[test]
    fn retained_real_linux_lifecycle_and_mutations() {
        for (name, want) in [("normal", true), ("interrupted", false), ("ongoing", false)] {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
            let l = j(
                &root.join(format!("{name}-lifecycle.json")),
                &mut Hashes::new(),
            );
            let cap = j(
                &root.join(format!("{name}-capture.json")),
                &mut Hashes::new(),
            );
            let length = n(&l["final_snapshot"], "file_bytes");
            let sha = s(&l, "pcap_sha256");
            assert_eq!(lifecycle_valid(&l, &cap, length, sha), want);
            if want {
                for mutation in 0..5 {
                    let mut bad = l.clone();
                    match mutation {
                        0 => bad["final_snapshot"]["counters"]["received"] = json!(u64::MAX),
                        1 => bad["quiescence"][0]["elapsed_millis"] = 0.into(),
                        2 => bad["forced_cleanup"] = true.into(),
                        3 => {
                            bad["preflight"][0]["stderr_text"] =
                                "tcpdump: 0 frogs, 0 frogs, 0 frogs\n".into()
                        }
                        _ => bad["remote_ip"] = "198.18.0.3".into(),
                    }
                    assert!(!lifecycle_valid(&bad, &cap, length, sha));
                }
            }
        }
    }
    #[test]
    fn byte_receipt_arithmetic_and_authorship_mutations() {
        let task = tasks(&[7; 32], "cold_random", 0)
            .into_iter()
            .find(|t| t["arm"] == "raw_full")
            .unwrap();
        let bytes = b"public";
        let source = "a".repeat(40);
        let binary = "b".repeat(64);
        let row = json!({"task":task,"phase":"transfer","schema":"spiralcore.wan.row.v4","cold_graph_warming":false,"status":"published","error":null,"object":object(bytes),"object_bytes":bytes.len(),"source_revision":source,"executable_sha256":binary,"os":"linux","architecture":"x86_64","unix_start_nanos":"1000000","unix_end_nanos":"1100000","metrics":{"preparation":{"selected_mode":0,"promotion_performed":false,"retained_base_reused":false},"records_sent":0,"http_requests":1,"application_request_bytes":225,"application_response_bytes":218,"request_body_bytes":249,"response_body_bytes":258,"data_bytes":6,"wall_micros":100,"sender_cpu_micros":20,"receiver_cpu_micros":30,"receiver_wall_micros":40}});
        assert_eq!(
            row_metrics(&row, &task, "transfer", bytes, &source, &binary),
            [100, 20, 30, 50, 507]
        );
        for mutation in 0..6 {
            let mut bad = row.clone();
            match mutation {
                0 => bad["metrics"]["response_body_bytes"] = 257.into(),
                1 => bad["object"] = "0".repeat(64).into(),
                2 => bad["metrics"]["preparation"]["selected_mode"] = 4.into(),
                3 => bad["source_revision"] = "c".repeat(40).into(),
                4 => bad["metrics"]["sender_cpu_micros"] = u64::MAX.into(),
                _ => bad["cold_graph_warming"] = true.into(),
            }
            assert!(std::panic::catch_unwind(|| row_metrics(
                &bad, &task, "transfer", bytes, &source, &binary
            ))
            .is_err());
        }
    }
    #[test]
    fn midpoint_stats_ratio_and_overflow() {
        assert_eq!(distribution(&[4, 10, 2, 8])["median"], 6.);
        assert_eq!(distribution(&[4, 10, 2, 8])["p95"], 10);
        assert_eq!(ratios(&[8, 4, 2], &[2, 4, 8])["geometric_mean"], 1.);
        assert!(std::panic::catch_unwind(|| sum([u64::MAX, 1])).is_err());
        assert!(std::panic::catch_unwind(|| ratios(&[1], &[0])).is_err());
    }
    #[test]
    fn canonical_tasks_are_unique_and_permutations_exhaustive() {
        let run = [7; 32];
        for case in CASES {
            let tasks: Vec<_> = (0..5).flat_map(|s| tasks(&run, case, s)).collect();
            assert_eq!(tasks.len(), 120);
            let scopes: BTreeSet<_> = tasks.iter().map(|t| s(t, "scope")).collect();
            assert_eq!(scopes.len(), 120);
            let first24: BTreeSet<_> = tasks
                .chunks(4)
                .take(24)
                .map(|t| t.iter().map(|t| s(t, "arm")).collect::<Vec<_>>())
                .collect();
            assert_eq!(first24.len(), 24);
        }
    }
    #[test]
    fn mutation_and_exact_hash_bind_length() {
        let (b, t) = fixtures("localized_32", 8192, 9);
        assert_eq!(b.len(), t.len());
        assert_eq!(b.iter().zip(&t).filter(|(x, y)| x != y).count(), 32);
        assert_ne!(object(&b), object(&t));
        assert_ne!(object(&b), object(&b[..8191]));
        let (b, t) = fixtures("prefix_4096", 8192, 9);
        assert_eq!(&t[4096..], b);
    }
    #[test]
    fn previously_measured_localized_fixture_golden_ids() {
        // Historical measured shard-0 trial-0, source f4e5e9afe6f3628ffe82e4553a88111f5bd6d126.
        // This tests byte-fixture compatibility only, never imports its measurements into v4.
        let (b, t) = fixtures("localized_32", 16777216, 1141005889793895525);
        assert_eq!(
            object(&b),
            "bc9e067fedfbaf78f9a55c4af3676243fad04de20f378bc6a50f13b4d45e3e6a"
        );
        assert_eq!(
            object(&t),
            "af2a48c0aca83032bfca91d16879a4ac6167798516a21293db5785e3fe3770b1"
        );
    }
    #[test]
    fn full_lifecycle_costs_and_negative_author_values() {
        let mut cells = Cells::new();
        for t in 0..30 {
            for arm in ARMS {
                cells.insert((t, arm.into(), "seed".into()), [10, 2, 3, 5, 100]);
                cells.insert((t, arm.into(), "transfer".into()), [20, 3, 4, 7, 200]);
            }
        }
        let result = aggregate(&cells, true);
        assert_eq!(
            result["arms"]["sparse"]["seed_plus_transfer"]["combined_cpu_micros"]["median"],
            12.
        );
        let mut bad = result.clone();
        bad["arms"]["sparse"]["transfer"]["wall_micros"]["values"][0] = 19.into();
        assert!(std::panic::catch_unwind(|| equivalent(&result, &bad)).is_err());
        cells.remove(&(0, "raw_full".into(), "seed".into()));
        assert!(std::panic::catch_unwind(|| aggregate(&cells, true)).is_err());
    }
    #[test]
    fn missing_counter_and_invalid_hash_rejected() {
        assert_eq!(
            counter_text(
                "1 packets captured\n1 packets received by filter\n0 packets dropped by kernel\n"
            ),
            [1, 1, 0]
        );
        assert!(
            std::panic::catch_unwind(|| counter_text("0 packets dropped by kernel\n")).is_err()
        );
        assert!(std::panic::catch_unwind(|| unhex(&"g".repeat(64), 32)).is_err());
    }
}
