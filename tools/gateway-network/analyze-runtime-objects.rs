//! Offline arithmetic for the registered reliable-object gateway campaign.
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const ARMS: [&str; 4] = ["raw", "whole-zstd", "warm", "runtime"];
const PROFILES: [&str; 2] = ["lan", "wan80-10"];
const TIERS: [(&str, u64); 2] = [("near1m", 1_047_202), ("max-trace", 14_851_384)];
const LABELS: [&str; 6] = [
    "generated",
    "residual",
    "repeat",
    "unrelated",
    "opaque-edited",
    "opaque-edited-repeat",
];
const CHANNEL: [&str; 6] = [
    "encoded_batch_bytes_sent",
    "encoded_batch_bytes_received",
    "framing_bytes_sent",
    "framing_bytes_received",
    "batches_sent",
    "batches_received",
];
const METRICS: [&str; 7] = [
    "request_bytes",
    "response_bytes",
    "logical_bytes",
    "sender_cpu_us",
    "receiver_cpu_us",
    "endpoint_stage_cpu_us",
    "client_wall_us",
];
fn require(ok: bool, message: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(message.into()) }
}
fn n(v: &Value, key: &str) -> Result<u64> {
    v[key]
        .as_u64()
        .ok_or_else(|| format!("missing/non-u64 {key}").into())
}
fn s<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .ok_or_else(|| format!("missing/non-string {key}").into())
}
fn a<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    v[key]
        .as_array()
        .ok_or_else(|| format!("missing/non-array {key}").into())
}
fn flag(v: &Value, key: &str) -> Result<()> {
    require(v[key] == true, &format!("false/missing {key}"))
}
fn add(x: u64, y: u64) -> Result<u64> {
    x.checked_add(y).ok_or_else(|| "arithmetic overflow".into())
}
fn sub(x: u64, y: u64) -> Result<u64> {
    x.checked_sub(y).ok_or_else(|| "counter rollback".into())
}
fn hex_id(id: &str) -> Result<()> {
    require(
        id.len() == 64
            && id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "invalid 32-byte hex identity",
    )
}
fn read(path: &Path) -> Result<Value> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    require(bytes.len() <= 16 * 1024 * 1024, "receipt exceeds 16 MiB")?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()).into())
}
fn channel_delta(before: &Value, after: &Value) -> Result<(u64, u64)> {
    Ok((
        add(
            sub(n(after, CHANNEL[0])?, n(before, CHANNEL[0])?)?,
            sub(n(after, CHANNEL[2])?, n(before, CHANNEL[2])?)?,
        )?,
        add(
            sub(n(after, CHANNEL[1])?, n(before, CHANNEL[1])?)?,
            sub(n(after, CHANNEL[3])?, n(before, CHANNEL[3])?)?,
        )?,
    ))
}
#[derive(Clone, Copy, Default)]
struct Metrics {
    request: u64,
    response: u64,
    sender_cpu: u64,
    receiver_cpu: u64,
    wall: u64,
}
impl Metrics {
    fn add(self, other: Self) -> Result<Self> {
        Ok(Self {
            request: add(self.request, other.request)?,
            response: add(self.response, other.response)?,
            sender_cpu: add(self.sender_cpu, other.sender_cpu)?,
            receiver_cpu: add(self.receiver_cpu, other.receiver_cpu)?,
            wall: add(self.wall, other.wall)?,
        })
    }
    fn json(self) -> Result<Value> {
        Ok(
            json!({"request_bytes":self.request,"response_bytes":self.response,"logical_bytes":add(self.request,self.response)?,"sender_cpu_us":self.sender_cpu,"receiver_cpu_us":self.receiver_cpu,"endpoint_stage_cpu_us":add(self.sender_cpu,self.receiver_cpu)?,"client_wall_us":self.wall}),
        )
    }
}
fn median(values: &[f64]) -> Result<f64> {
    require(
        !values.is_empty() && values.iter().all(|x| x.is_finite()),
        "invalid median input",
    )?;
    let mut values = values.to_vec();
    values.sort_by(f64::total_cmp);
    let i = values.len() / 2;
    Ok(if values.len().is_multiple_of(2) {
        (values[i - 1] + values[i]) / 2.0
    } else {
        values[i]
    })
}
fn ratio(value: u64, baseline: u64) -> Option<f64> {
    (baseline != 0).then(|| value as f64 / baseline as f64)
}
fn tagged(mut row: Value, profile: &str, trial: u64, arm: &str, group: &str, label: &str) -> Value {
    row["profile"] = json!(profile);
    row["trial"] = json!(trial);
    row["arm"] = json!(arm);
    row["group"] = json!(group);
    row["label"] = json!(label);
    row
}
#[derive(Default)]
struct Batch {
    cpu: u64,
    wall: u64,
    requests: u64,
    responses: u64,
    count: u64,
}
fn batches(server: &Value) -> Result<BTreeMap<String, Batch>> {
    let mut out = BTreeMap::<String, Batch>::new();
    for row in a(server, "server_batches")? {
        flag(row, "accepted")?;
        let id = s(row, "transfer_id")?;
        hex_id(id)?;
        let b = out.entry(id.to_owned()).or_default();
        b.cpu = add(b.cpu, n(row, "cpu_us")?)?;
        b.wall = add(b.wall, n(row, "wall_us")?)?;
        b.requests = add(b.requests, n(row, "request_frames")?)?;
        b.responses = add(b.responses, n(row, "response_frames")?)?;
        b.count = add(b.count, 1)?;
    }
    Ok(out)
}
#[derive(Default)]
struct Rows {
    operations: Vec<Value>,
    cumulative: Vec<Value>,
    prefix: Vec<Value>,
    sessions: Vec<Value>,
    topologies: Vec<Value>,
    fixtures: Vec<Value>,
    deliveries: u64,
    requested: u64,
    actions: BTreeMap<String, u64>,
}
fn expected<'a>(manifest: &'a Value, tier: &'a Value, label: &str) -> Result<&'a Value> {
    Ok(match label {
        "schema" => &manifest["schema"],
        "checkpoint" => &tier["checkpoint"],
        "generated" => &tier["generated"],
        "residual" | "repeat" => &tier["residual"],
        "unrelated" => &tier["opaque"],
        "opaque-edited" | "opaque-edited-repeat" => &tier["opaque_edited"],
        _ => return Err("unexpected operation label".into()),
    })
}
fn artifact(v: &Value) -> Result<()> {
    hex_id(s(v, "object_id")?)?;
    hex_id(s(v, "sha256")?)?;
    n(v, "bytes")?;
    Ok(())
}
fn manifest_check(m: &Value, trial: u64) -> Result<()> {
    require(
        n(m, "trial")? == trial && n(m, "version")? == 1,
        "wrong fixture manifest identity",
    )?;
    require(
        a(m, "tiers")?.len() == 2 && a(m, "verify_files")?.len() == 8,
        "wrong fixture tier/file count",
    )?;
    artifact(&m["schema"])?;
    for (i, (label, len)) in TIERS.iter().enumerate() {
        let tier = &m["tiers"][i];
        require(s(tier, "label")? == *label, "wrong fixture tier order")?;
        for kind in ["generated", "residual", "opaque", "opaque_edited"] {
            artifact(&tier[kind])?;
            require(
                n(&tier[kind], "bytes")? == *len,
                "wrong registered object length",
            )?;
        }
        for kind in ["checkpoint", "recipe"] {
            artifact(&tier[kind])?;
        }
        require(
            n(&tier["checkpoint"], "bytes")? == 3382,
            "wrong checkpoint size",
        )?;
    }
    Ok(())
}
fn calibration(root: &Path, profile: &str, trial: u64) -> Result<Value> {
    let dir = root.join(format!("topology-{profile}-t{trial}"));
    let c = read(&dir.join("calibration.json"))?;
    flag(&c, "accepted")?;
    require(s(&c, "profile")? == profile, "calibration profile mismatch")?;
    let mut pings = Vec::new();
    let mut rates = Vec::new();
    for side in ["a", "b"] {
        let p = read(&dir.join(format!("ping-{side}.json")))?;
        let samples = a(&p, "samples")?;
        let retained = a(&p, "retained")?;
        require(
            n(&p, "count")? == 20 && samples.len() == 20 && retained == &samples[4..],
            "invalid RTT sample retention",
        )?;
        let sum = samples[4..].iter().try_fold(0.0, |sum, v| {
            v.as_f64()
                .filter(|x| x.is_finite() && *x >= 0.0)
                .map(|x| sum + x)
                .ok_or("invalid RTT")
        })?;
        let mean = sum / 16.0;
        let recorded = p["mean_ms"].as_f64().ok_or("missing RTT mean")?;
        require(
            (mean - recorded).abs() < 0.000001
                && if profile == "wan80-10" {
                    (75.0..=90.0).contains(&mean)
                } else {
                    mean < 20.0
                },
            "RTT calibration outside registered bound",
        )?;
        pings.push(p);
    }
    if profile == "wan80-10" {
        for direction in ["forward", "reverse"] {
            let v = read(&dir.join(format!("iperf-{direction}-client.json")))?;
            let bps = v["end"]["sum_received"]["bits_per_second"]
                .as_f64()
                .ok_or("missing rate calibration")?;
            require(
                v["error"].is_null() && (8_500_000.0..=10_200_000.0).contains(&bps),
                "rate calibration outside registered bound",
            )?;
            rates.push(json!({"direction":direction,"bits_per_second":bps}));
        }
    }
    let complete = read(&dir.join("complete.json"))?;
    flag(&complete, "ok")?;
    flag(&complete, "namespace_cleanup")?;
    Ok(
        json!({"profile":profile,"trial":trial,"rtt":pings,"rates":rates,"setup_calibration_wall_us":n(&complete,"setup_calibration_wall_us")?,"topology_all_four_sessions_wall_us":n(&complete,"topology_all_four_sessions_wall_us")?}),
    )
}
fn queue_drops(dir: &Path, profile: &str) -> Result<Value> {
    let mut rows = Vec::new();
    for direction in ["left", "right"] {
        let before = read(&dir.join(format!("tc-{direction}-measurement-before.json")))?;
        let after = read(&dir.join(format!("tc-{direction}-measurement-after.json")))?;
        let find = |v: &Value| -> Result<Vec<Value>> {
            Ok(v.as_array()
                .ok_or("invalid qdisc receipt")?
                .iter()
                .filter(|x| x["kind"] == "netem")
                .cloned()
                .collect())
        };
        let b = find(&before)?;
        let a = find(&after)?;
        if profile == "wan80-10" {
            require(
                b.len() == 1 && a.len() == 1,
                "missing or duplicate shaped qdisc",
            )?;
            require(
                n(&b[0], "drops")? == 0 && n(&a[0], "qlen")? == 0,
                "queue not reset or drained",
            )?;
            for q in [&b[0], &a[0]] {
                require(
                    n(&q["options"], "limit")? == 256
                        && q["options"]["delay"]["delay"] == 0.04
                        && n(&q["options"]["rate"], "rate")? == 1_250_000,
                    "qdisc differs from registration",
                )?;
            }
            rows.push(json!({"direction":direction,"drops":n(&a[0],"drops")?}));
        } else {
            require(b.is_empty() && a.is_empty(), "unexpected LAN shaping")?;
            rows.push(json!({"direction":direction,"drops":null}));
        }
    }
    Ok(json!(rows))
}
fn session(
    root: &Path,
    profile: &str,
    trial: u64,
    arm: &str,
    m: &Value,
    rows: &mut Rows,
) -> Result<()> {
    let name = format!("{profile}-t{trial}-{arm}");
    let dir = root.join(&name);
    let meta = read(&dir.join("session.json"))?;
    let complete = read(&dir.join("complete.json"))?;
    require(
        s(&meta, "profile")? == profile && n(&meta, "trial")? == trial && s(&meta, "arm")? == arm,
        "session metadata mismatch",
    )?;
    require(
        s(&meta, "calibration_directory")? == format!("topology-{profile}-t{trial}"),
        "wrong calibration join",
    )?;
    flag(&complete, "ok")?;
    flag(&complete, "stores_removed")?;
    require(
        n(&complete, "object_deliveries")? == 12,
        "wrong session dose",
    )?;
    let c = read(&dir.join("client.json"))?;
    let r = read(&dir.join("server.json"))?;
    require(
        s(&c, "status")? == "ok"
            && s(&r, "status")? == "ok"
            && s(&c, "role")? == "connect"
            && s(&r, "role")? == "listen",
        "session not successful",
    )?;
    hex_id(s(&c, "namespace")?)?;
    require(
        s(&c, "namespace")? == s(&r, "namespace")? && s(&c, "scope")? == s(&r, "scope")?,
        "peer namespace/scope mismatch",
    )?;
    for i in [0, 2, 4] {
        require(
            n(&c["channel"], CHANNEL[i])? == n(&r["channel"], CHANNEL[i + 1])?
                && n(&c["channel"], CHANNEL[i + 1])? == n(&r["channel"], CHANNEL[i])?,
            "opposite channel totals mismatch",
        )?;
    }
    let ops = a(&c, "operations")?;
    let count = if arm == "warm" || arm == "runtime" {
        14
    } else {
        12
    };
    require(
        ops.len() == count
            && n(&complete, "operations")? == count as u64
            && a(&c, "groups")?.len() == 2,
        "wrong operation/group count",
    )?;
    let mut batch_map = batches(&r)?;
    let mut seen = BTreeSet::new();
    let mut committed = BTreeMap::new();
    let initial = &ops[0]["channel_before"];
    for key in CHANNEL {
        n(initial, key)?;
    }
    require(
        n(initial, "batches_sent")? == 1 && n(initial, "batches_received")? == 1,
        "missing singleton namespace exchange",
    )?;
    let mut prefix = Metrics {
        request: add(n(initial, CHANNEL[0])?, n(initial, CHANNEL[2])?)?,
        response: add(n(initial, CHANNEL[1])?, n(initial, CHANNEL[3])?)?,
        sender_cpu: n(&c, "setup_cpu_us")?,
        receiver_cpu: n(&r, "setup_cpu_us")?,
        wall: n(&c, "setup_wall_us")?,
    };
    let mut connection_setup = prefix.json()?;
    connection_setup["receiver_cpu_scope"] = json!(
        "receiver one-time connection/authentication/namespace setup only; no object handlers"
    );
    let mut previous = initial;
    let mut index = 0usize;
    for (group_index, (group, len)) in TIERS.iter().enumerate() {
        let prep = &c["groups"][group_index];
        require(
            s(prep, "label")? == *group && s(prep, "mode")? == arm,
            "group/profile ordering mismatch",
        )?;
        let mut cumulative = Metrics {
            sender_cpu: n(prep, "cpu_us")?,
            wall: n(prep, "wall_us")?,
            ..Metrics::default()
        };
        prefix = prefix.add(cumulative)?;
        let labels: Vec<&str> = match arm {
            "warm" => vec!["schema"],
            "runtime" => vec!["checkpoint"],
            _ => vec![],
        }
        .into_iter()
        .chain(LABELS)
        .collect();
        if labels.len() == 6 {
            rows.cumulative
                .push(tagged(cumulative.json()?, profile, trial, arm, group, "0"));
            rows.prefix
                .push(tagged(prefix.json()?, profile, trial, arm, group, "0"));
        }
        let mut delivered = 0;
        for label in labels {
            let op = &ops[index];
            index += 1;
            let action = match label {
                "schema" => "schema",
                "checkpoint" => "checkpoint",
                "generated" if arm == "warm" => "seed",
                _ => "send",
            };
            require(
                s(op, "group")? == *group && s(op, "label")? == label && s(op, "action")? == action,
                "operation sequence/action mismatch",
            )?;
            flag(op, "success")?;
            let fixture = expected(m, &m["tiers"][group_index], label)?;
            require(
                s(op, "object_id")? == s(fixture, "object_id")?
                    && n(op, "input_bytes")? == n(fixture, "bytes")?,
                "fixture identity/length mismatch",
            )?;
            let id = s(op, "transfer_id")?;
            hex_id(id)?;
            require(seen.insert(id.to_owned()), "duplicate client transfer ID")?;
            let batch = batch_map
                .remove(id)
                .ok_or("missing receiver batches for operation")?;
            require(
                batch.requests == n(op, "requests")? && batch.count > 0 && batch.responses > 0,
                "receiver request attribution mismatch",
            )?;
            let before = &op["channel_before"];
            let after = &op["channel_after"];
            for key in CHANNEL {
                require(
                    n(before, key)? == n(previous, key)?,
                    "noncontiguous operation counters",
                )?;
                sub(n(after, key)?, n(before, key)?)?;
            }
            require(
                sub(n(after, "batches_sent")?, n(before, "batches_sent")?)? == batch.count
                    && sub(
                        n(after, "batches_received")?,
                        n(before, "batches_received")?,
                    )? == batch.count,
                "batch count attribution mismatch",
            )?;
            let (request, response) = channel_delta(before, after)?;
            previous = after;
            let metric = Metrics {
                request,
                response,
                sender_cpu: n(op, "cpu_us")?,
                receiver_cpu: batch.cpu,
                wall: n(op, "wall_us")?,
            };
            require(
                n(op, "request_frame_bytes")? <= request
                    && n(op, "response_frame_bytes")? <= response
                    && n(op, "data_bytes")? <= request,
                "frame bytes exceed channel bytes",
            )?;
            let path = s(op, "publication_path")?;
            require(
                ["verified-transfer", "verified-cache-hit"].contains(&path),
                "unknown publication path",
            )?;
            let mode = s(op, "selected_mode")?;
            if arm == "runtime" && ["generated", "residual"].contains(&label) {
                require(
                    mode == "RuntimeV1" && path == "verified-transfer",
                    "runtime decoder activity gate failed",
                )?;
            }
            if arm == "warm" && label == "opaque-edited" {
                require(
                    mode == "SparseV1" && path == "verified-transfer",
                    "warm sparse activity gate failed",
                )?;
            }
            let mut row = tagged(metric.json()?, profile, trial, arm, group, label);
            for key in [
                "action",
                "transfer_id",
                "object_id",
                "input_bytes",
                "selected_mode",
                "publication_path",
                "data_bytes",
                "requests",
                "selection",
            ] {
                row[key] = op[key].clone();
            }
            row["receiver_handler_wall_us"] = json!(batch.wall);
            row["receiver_handler_cpu_us"] = json!(batch.cpu);
            row["batches"] = json!(batch.count);
            rows.operations.push(row);
            cumulative = cumulative.add(metric)?;
            prefix = prefix.add(metric)?;
            committed.insert(s(op, "object_id")?.to_owned(), n(op, "input_bytes")?);
            *rows.actions.entry(action.to_owned()).or_default() += 1;
            if action != "schema" && action != "checkpoint" {
                require(n(op, "input_bytes")? == *len, "wrong delivered length")?;
                delivered += 1;
                rows.deliveries = add(rows.deliveries, 1)?;
                rows.requested = add(rows.requested, *len)?;
            }
            rows.cumulative.push(tagged(
                cumulative.json()?,
                profile,
                trial,
                arm,
                group,
                &delivered.to_string(),
            ));
            rows.prefix.push(tagged(
                prefix.json()?,
                profile,
                trial,
                arm,
                group,
                &delivered.to_string(),
            ));
        }
        require(delivered == 6, "wrong group delivery count")?;
    }
    require(
        index == ops.len() && batch_map.is_empty(),
        "orphan operation/receiver batch",
    )?;
    for (i, key) in CHANNEL.iter().enumerate() {
        require(
            sub(n(&c["channel"], key)?, n(previous, key)?)? == if i == 2 || i == 3 { 4 } else { 0 },
            "unexpected finish counters",
        )?;
    }
    flag(&r["readback"], "reopened")?;
    require(
        n(&r["readback"], "expected_files_exact")? == 8,
        "exact local readback absent",
    )?;
    let mut reopened = BTreeMap::new();
    for object in a(&r["readback"], "objects")? {
        flag(object, "digest_verified")?;
        require(
            reopened
                .insert(s(object, "object_id")?.to_owned(), n(object, "bytes")?)
                .is_none(),
            "duplicate reopened object",
        )?;
    }
    require(
        reopened == committed,
        "reopened publication identity/length set mismatch",
    )?;
    let capture = read(&dir.join("capture.json"))?;
    let cap = read(&dir.join("capture-counters.json"))?;
    require(
        n(&capture, "packets")? > 0
            && n(&capture, "other_packets")? == 0
            && n(&capture, "packets")? == n(&cap, "captured")?
            && n(&cap, "captured")? == n(&cap, "received")?
            && n(&cap, "dropped")? == 0,
        "incomplete capture",
    )?;
    let mut row = tagged(
        json!({"session":name,"ip_bytes":n(&capture,"ip_bytes")?,"udp_payload_bytes":n(&capture,"udp_payload_bytes")?,"ethernet_bytes":n(&capture,"ethernet_bytes")?,"packets":n(&capture,"packets")?,"sender_process_cpu_us":n(&c,"process_cpu_us")?,"receiver_process_cpu_us":n(&r,"process_cpu_us")?,"summed_process_cpu_us":add(n(&c,"process_cpu_us")?,n(&r,"process_cpu_us")?)?,"client_wall_us":n(&c,"wall_us")?,"receiver_wall_us":n(&r,"wall_us")?,"harness_wall_us":n(&complete,"harness_wall_us")?,"connection_setup":connection_setup,"groups":c["groups"],"finish_wall_us":n(&c,"finish_wall_us")?,"finish_cpu_us":n(&c,"finish_cpu_us")?,"readback":r["readback"],"client_udp":c["udp"],"server_udp":r["udp"],"queue_drops":queue_drops(&dir,profile)?}),
        profile,
        trial,
        arm,
        "session",
        "complete",
    );
    row["logical_bytes"] = json!(add(
        add(n(&c["channel"], CHANNEL[0])?, n(&c["channel"], CHANNEL[1])?)?,
        add(n(&c["channel"], CHANNEL[2])?, n(&c["channel"], CHANNEL[3])?)?
    )?);
    rows.sessions.push(row);
    Ok(())
}
fn summarize(rows: &[Value], metrics: &[&str]) -> Result<Vec<Value>> {
    let mut groups = BTreeMap::<Vec<String>, Vec<&Value>>::new();
    for row in rows {
        let key = ["profile", "arm", "group", "label"]
            .into_iter()
            .map(|k| s(row, k).map(str::to_owned))
            .collect::<Result<Vec<_>>>()?;
        groups.entry(key).or_default().push(row);
    }
    for values in groups.values_mut() {
        values.sort_by_key(|r| r["trial"].as_u64());
        require(
            values.len() == 3 && values.iter().enumerate().all(|(i, r)| r["trial"] == i),
            "summary needs exactly three distinct matched trials",
        )?;
    }
    let mut out = Vec::new();
    for (key, values) in &groups {
        let mut summary = tagged(json!({}), &key[0], 0, &key[1], &key[2], &key[3]);
        summary
            .as_object_mut()
            .ok_or("invalid summary")?
            .remove("trial");
        let mut measured = serde_json::Map::new();
        let mut comparisons = serde_json::Map::new();
        for metric in metrics {
            let xs = values
                .iter()
                .map(|v| n(v, metric))
                .collect::<Result<Vec<_>>>()?;
            let floats = xs.iter().map(|x| *x as f64).collect::<Vec<_>>();
            measured.insert(
                (*metric).to_owned(),
                json!({"trials":xs,"median":median(&floats)?}),
            );
        }
        for baseline in ["raw", "whole-zstd"] {
            let mut other = key.clone();
            other[1] = baseline.to_owned();
            if let Some(base) = groups.get(&other) {
                let mut comparisons_by_metric = serde_json::Map::new();
                for metric in metrics {
                    let ratios = values
                        .iter()
                        .zip(base)
                        .map(|(v, b)| Ok(ratio(n(v, metric)?, n(b, metric)?)))
                        .collect::<Result<Vec<_>>>()?;
                    let med = ratios
                        .iter()
                        .copied()
                        .collect::<Option<Vec<_>>>()
                        .map(|x| median(&x))
                        .transpose()?;
                    comparisons_by_metric.insert((*metric).to_owned(),json!({"paired_candidate_over_baseline":ratios,"median_ratio":med,"median_fraction_reduction":med.map(|r|1.0-r)}));
                }
                comparisons.insert(baseline.to_owned(), json!(comparisons_by_metric));
            } else {
                comparisons.insert(baseline.to_owned(), Value::Null);
            }
        }
        summary["metrics"] = json!(measured);
        summary["comparisons"] = json!(comparisons);
        out.push(summary);
    }
    Ok(out)
}
fn analyze(root: &Path) -> Result<Value> {
    require(
        std::fs::read_to_string(root.join("run.exit"))?.trim() == "0",
        "campaign terminal exit is not zero",
    )?;
    let complete = read(&root.join("complete.json"))?;
    for key in [
        "exact_reopened",
        "stores_removed",
        "capture_complete",
        "host_routes_unchanged",
    ] {
        flag(&complete, key)?;
    }
    require(
        n(&complete, "sessions")? == 24
            && n(&complete, "operations")? == 312
            && n(&complete, "object_deliveries")? == 288,
        "incomplete registered dose",
    )?;
    let reg = read(&root.join("registration.json"))?;
    require(
        reg["arms"] == json!(ARMS)
            && reg["dose"]["trials"] == json!([0, 1, 2])
            && reg["dose"]["total_requested_object_bytes"] == 2_289_396_384u64,
        "registration differs from analysis contract",
    )?;
    for prefix in ["host-routes", "host-routes-v6"] {
        require(
            read(&root.join(format!("{prefix}-before.json")))?
                == read(&root.join(format!("{prefix}-after.json")))?,
            "host routes changed",
        )?;
    }
    let mut rows = Rows::default();
    let expected_names: BTreeSet<_> = PROFILES
        .into_iter()
        .flat_map(|p| (0..3).flat_map(move |t| ARMS.map(|a| format!("{p}-t{t}-{a}"))))
        .collect();
    let actual_names: BTreeSet<_> = std::fs::read_dir(root)?
        .map(|e| e.map(|x| x.file_name().to_string_lossy().into_owned()))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|name| PROFILES.iter().any(|p| name.starts_with(&format!("{p}-t"))))
        .collect();
    require(
        actual_names == expected_names,
        "missing or extra session directory",
    )?;
    for trial in 0..3 {
        let m = read(&root.join(format!("fixtures-{trial}/manifest.json")))?;
        manifest_check(&m, trial)?;
        rows.fixtures.push(json!({"trial":trial,"generation_cpu_us":n(&m,"generation_cpu_us")?,"generation_wall_us":n(&m,"generation_wall_us")?}));
        for profile in PROFILES {
            rows.topologies.push(calibration(root, profile, trial)?);
            for arm in ARMS {
                session(root, profile, trial, arm, &m, &mut rows)
                    .map_err(|e| format!("{profile}-t{trial}-{arm}: {e}"))?;
            }
        }
    }
    require(
        rows.sessions.len() == 24
            && rows.operations.len() == 312
            && rows.deliveries == 288
            && rows.requested == 2_289_396_384,
        "recomputed dose mismatch",
    )?;
    require(
        rows.actions
            == BTreeMap::from([
                ("send".to_owned(), 276),
                ("seed".to_owned(), 12),
                ("schema".to_owned(), 12),
                ("checkpoint".to_owned(), 12),
            ]),
        "wrong action dose",
    )?;
    let operation_summaries = summarize(&rows.operations, &METRICS)?;
    let cumulative_summaries = summarize(&rows.cumulative, &METRICS)?;
    let mut prefix_summaries = summarize(&rows.prefix, &METRICS)?;
    for row in rows.prefix.iter_mut().chain(prefix_summaries.iter_mut()) {
        row["receiver_cpu_scope"] = json!(
            "receiver one-time connection/authentication/namespace setup plus all preceding object handlers; excludes finish/readback"
        );
    }
    let session_summaries = summarize(
        &rows.sessions,
        &[
            "logical_bytes",
            "ip_bytes",
            "udp_payload_bytes",
            "ethernet_bytes",
            "sender_process_cpu_us",
            "receiver_process_cpu_us",
            "summed_process_cpu_us",
            "client_wall_us",
            "harness_wall_us",
        ],
    )?;
    Ok(
        json!({"schema":"spiralcore.runtime-object.analysis.v1","qualified_receipts":true,"sessions":24,"operations":312,"object_deliveries":288,"requested_bytes":rows.requested,"operation_counts":rows.actions,"method":"analysis-method.md; three descriptive paired trials; ratios are candidate/baseline, lower is better; no confidence or tail claims","limits":["local Linux shaped network is not actual Internet WAN","captured wire bytes are whole-session only","CPU domains are process or labeled nested stages; never add nested stages to process totals","client wall includes waiting and overlapping CPU, isolated send-only time is not identified","runtime/warm are explicit object profiles, not transparent TUN compression","artifact custody and source/binary identity are independently verified"],"fixture_generation":rows.fixtures,"topologies":rows.topologies,"operation_rows":rows.operations,"paid_group_cumulative_rows":rows.cumulative,"paid_connection_prefix_rows":rows.prefix,"session_rows":rows.sessions,"operation_summaries":operation_summaries,"paid_group_cumulative_summaries":cumulative_summaries,"paid_connection_prefix_summaries":prefix_summaries,"session_summaries":session_summaries}),
    )
}
fn main() -> Result<()> {
    let mut args = std::env::args_os();
    args.next();
    let root = args
        .next()
        .ok_or("usage: analyze_runtime_objects CAMPAIGN_DIRECTORY")?;
    require(args.next().is_none(), "unexpected argument")?;
    serde_json::to_writer_pretty(std::io::stdout().lock(), &analyze(Path::new(&root))?)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn channel_deltas_count_both_prefixes_and_reject_counter_rollback() {
        let before = json!({"encoded_batch_bytes_sent":100,"encoded_batch_bytes_received":50,
            "framing_bytes_sent":4,"framing_bytes_received":4});
        let after = json!({"encoded_batch_bytes_sent":400,"encoded_batch_bytes_received":130,
            "framing_bytes_sent":12,"framing_bytes_received":12});
        assert_eq!(channel_delta(&before, &after).unwrap(), (308, 88));
        assert!(channel_delta(&after, &before).is_err());
    }

    #[test]
    fn cumulative_costs_include_seed_once_and_keep_cpu_separate_from_wall() {
        let setup = Metrics {
            request: 30,
            response: 10,
            sender_cpu: 100,
            receiver_cpu: 20,
            wall: 200,
        };
        let seed = Metrics {
            request: 1000,
            response: 60,
            sender_cpu: 300,
            receiver_cpu: 80,
            wall: 1500,
        };
        let target = Metrics {
            request: 100,
            response: 40,
            sender_cpu: 50,
            receiver_cpu: 30,
            wall: 700,
        };
        let one = setup.add(seed).unwrap();
        let two = one.add(target).unwrap();
        assert_eq!(
            (
                one.request + one.response,
                one.sender_cpu + one.receiver_cpu,
                one.wall
            ),
            (1100, 500, 1700)
        );
        assert_eq!(
            (
                two.request + two.response,
                two.sender_cpu + two.receiver_cpu,
                two.wall
            ),
            (1240, 580, 2400)
        );
        assert!(
            Metrics {
                request: u64::MAX,
                ..Metrics::default()
            }
            .add(target)
            .is_err()
        );
    }

    #[test]
    fn descriptive_summary_uses_median_of_matched_ratios_and_retains_zero_denominator() {
        assert_eq!(median(&[3.0, 1.0, 2.0]).unwrap(), 2.0);
        let pairs = [(1, 100), (2, 1), (100, 2)];
        let ratios: Vec<_> = pairs
            .into_iter()
            .map(|(a, b)| ratio(a, b).unwrap())
            .collect();
        assert_eq!(median(&ratios).unwrap(), 2.0);
        assert_eq!(ratio(4, 0), None);
        assert!(median(&[]).is_err());
    }

    #[test]
    fn summaries_reject_duplicate_trials_and_receiver_batches_require_acceptance() {
        let row = |trial| {
            tagged(
                json!({"cost":3}),
                "lan",
                trial,
                "raw",
                "near1m",
                "generated",
            )
        };
        assert!(summarize(&[row(0), row(0), row(2)], &["cost"]).is_err());
        let id = "00".repeat(32);
        let accepted = json!({"transfer_id":id,"accepted":true,"cpu_us":2,"wall_us":3,"request_frames":4,"response_frames":1});
        let summed =
            batches(&json!({"server_batches":[accepted.clone(),accepted.clone()]})).unwrap();
        assert_eq!(
            (summed[&id].cpu, summed[&id].requests, summed[&id].count),
            (4, 8, 2)
        );
        let mut refused = accepted;
        refused["accepted"] = json!(false);
        assert!(batches(&json!({"server_batches":[refused]})).is_err());
    }
}
