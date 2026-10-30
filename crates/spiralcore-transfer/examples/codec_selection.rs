//! Local, serial codec/publication measurements. This is not a WAN benchmark.
use cpu_time::ProcessTime;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spiralcore_transfer::{
    client,
    endpoint::{ColdGraphPolicy, Endpoint},
    hash, hex, object_id,
    protocol::{Frame, Mode},
    wan::{self, BenchAuthority},
    Error, Id, Result,
};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

const CASES: [&str; 4] = [
    "cold_random",
    "cold_structured",
    "localized_32",
    "unrelated_structured",
];
const ARMS: [&str; 5] = ["raw", "zstd", "sparse", "prefix", "adaptive"];
type RunResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Config {
    trials: usize,
    sizes: Vec<usize>,
    out: PathBuf,
    cache: PathBuf,
    source: String,
}
impl Config {
    fn parse(args: impl IntoIterator<Item = String>) -> RunResult<Self> {
        let mut args = args.into_iter();
        let mut values = BTreeMap::new();
        while let Some(key) = args.next() {
            let value = args.next().ok_or("missing option value")?;
            if values.insert(key, value).is_some() {
                return Err("duplicate option".into());
            }
        }
        let mut take = |key: &str| values.remove(key).ok_or("missing required option");
        let trials = take("--trials")?.parse::<usize>()?;
        let sizes = take("--sizes")?
            .split(',')
            .map(str::parse::<usize>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let out = PathBuf::from(take("--out")?);
        let cache = PathBuf::from(take("--cache")?);
        let source = take("--source")?;
        if !values.is_empty()
            || !(1..=30).contains(&trials)
            || sizes.is_empty()
            || sizes.len() > 8
            || sizes
                .iter()
                .any(|n| !(64..=client::MAX_BATCHED_OBJECT).contains(n))
            || sizes
                .iter()
                .enumerate()
                .any(|(i, n)| sizes[..i].contains(n))
            || source.len() != 40
            || !source
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("invalid or out-of-bounds configuration".into());
        }
        Ok(Self {
            trials,
            sizes,
            out,
            cache,
            source,
        })
    }
}

fn expected_operations(trials: usize, sizes: usize) -> usize {
    trials * sizes * ARMS.len() * 6
}

fn arm_order(case_index: usize, trial: usize) -> [usize; 5] {
    std::array::from_fn(|i| (i + trial + case_index) % ARMS.len())
}

fn write_row(file: &mut File, value: &Value) -> RunResult<()> {
    serde_json::to_writer(&mut *file, value)?;
    file.write_all(b"\n")?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}

fn sha256_file(path: &Path) -> RunResult<String> {
    let mut reader = File::open(path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut reader, &mut hasher)?;
    Ok(hex(&hasher.finalize().into()))
}

#[derive(Default)]
struct ExchangeMetrics {
    request_bytes: u64,
    response_bytes: u64,
    calls: u64,
    frames: usize,
    receiver_cpu_micros: u128,
    receiver_wall_micros: u128,
    exchange_cpu_micros: u128,
    exchange_wall_micros: u128,
}
impl ExchangeMetrics {
    fn exchange(
        &mut self,
        endpoint: &mut Endpoint<BenchAuthority>,
        scope: Id,
        frames: &[Frame],
    ) -> Result<Vec<Frame>> {
        let start = Instant::now();
        let cpu = ProcessTime::now();
        let request = wan::encode_batch(frames)?;
        self.request_bytes += request.len() as u64;
        self.calls += 1;
        self.frames += frames.len();
        let receiver_start = Instant::now();
        let receiver_cpu = ProcessTime::now();
        let decoded = wan::decode_batch(&request)?;
        let mut replies = Vec::new();
        for frame in &decoded {
            let reply = endpoint
                .handle(&wan::context(scope), frame, 0)
                .unwrap_or_else(|error| frame.reply(127, (error as u16).to_le_bytes().to_vec()));
            let terminal = matches!(reply.kind, 103 | 127);
            replies.push(reply);
            if terminal {
                break;
            }
        }
        let reply_body = wan::encode_batch(&replies)?;
        // SCWREP01 carries two extra u64 timing fields versus SCWBAT01.
        // We count their exact length, without claiming an HTTP exchange occurred.
        self.response_bytes += reply_body.len() as u64 + 16;
        self.receiver_cpu_micros += receiver_cpu.elapsed().as_micros();
        self.receiver_wall_micros += receiver_start.elapsed().as_micros();
        let replies = wan::decode_batch(&reply_body)?;
        self.exchange_cpu_micros += cpu.elapsed().as_micros();
        self.exchange_wall_micros += start.elapsed().as_micros();
        Ok(replies)
    }
}

fn operation(
    cache: &Path,
    scope: Id,
    target: &[u8],
    base: Option<&[u8]>,
    arm: &str,
    phase: &str,
) -> RunResult<Value> {
    let mut endpoint = Endpoint::open_with_policy(
        cache,
        wan::limits(),
        BenchAuthority,
        ColdGraphPolicy::Disabled,
    )?;
    let transfer_id = hash(
        b"spiralcore.selector.local.operation.v1\0",
        &[&scope, phase.as_bytes()],
    );
    let mut metrics = ExchangeMetrics::default();
    let mut selected = None;
    let start = Instant::now();
    let cpu = ProcessTime::now();
    let exchange = |frames: &[Frame]| metrics.exchange(&mut endpoint, scope, frames);
    let stats = if phase == "seed" {
        client::transfer_batched(target, Mode::Raw, None, transfer_id, false, exchange)?
    } else {
        match (arm, base) {
            ("adaptive", base) => {
                let adaptive =
                    client::transfer_adaptive_batched(target, base, transfer_id, false, exchange)?;
                selected = Some(adaptive.selection);
                adaptive.transfer
            }
            ("sparse", Some(base)) => {
                client::transfer_sparse_batched(base, target, transfer_id, false, exchange)?
            }
            ("prefix", Some(base)) => {
                client::transfer_zstd_prefix_batched(base, target, transfer_id, false, exchange)?
            }
            ("raw" | "sparse", _) => {
                client::transfer_batched(target, Mode::Raw, None, transfer_id, false, exchange)?
            }
            ("zstd" | "prefix", _) => {
                client::transfer_batched(target, Mode::Zstd, None, transfer_id, false, exchange)?
            }
            _ => return Err("unknown arm".into()),
        }
    };
    let total_cpu_micros = cpu.elapsed().as_micros();
    let total_wall_micros = start.elapsed().as_micros();
    let selection = selected.map_or(Value::Null, |selected| json!({
        "selected_mode": selected.selected as u8,
        "expected_http_body_bytes": selected.expected_http_body_bytes,
        "exact_repeat_probe": selected.exact_repeat_probe,
        "candidates": selected.candidates.iter().map(|c| json!({"mode":c.mode as u8,"http_body_bytes":c.http_body_bytes})).collect::<Vec<_>>()
    }));
    // Close/reopen is a separate exact-byte qualification, outside operation timing.
    drop(endpoint);
    let mut reopened = Endpoint::open_with_policy(
        cache,
        wan::limits(),
        BenchAuthority,
        ColdGraphPolicy::Disabled,
    )?;
    let decoded = reopened.read_object(&wan::context(scope), stats.object)?;
    if decoded != target || stats.object != object_id(target) {
        return Err("reopened object differs from the original input".into());
    }
    let body = metrics.request_bytes + metrics.response_bytes;
    if selection
        .get("expected_http_body_bytes")
        .and_then(Value::as_u64)
        .is_some_and(|n| n != body)
    {
        return Err("adaptive projected body bytes disagree with actual serialized bodies".into());
    }
    Ok(json!({
        "event":"operation", "phase":phase, "arm":arm, "actual_mode":stats.mode as u8,
        "object":hex(&stats.object), "base":base.map(|b|hex(&object_id(b))), "object_bytes":target.len(),
        "verification":"exact-after-reopen", "verified_bytes":decoded.len(),
        "http_body_equivalent_bytes":body, "request_body_equivalent_bytes":metrics.request_bytes,
        "response_body_equivalent_bytes":metrics.response_bytes,
        "serialized_batches":metrics.calls, "request_frames":metrics.frames,
        "protocol_request_bytes":stats.request_bytes, "protocol_response_bytes":stats.response_bytes,
        "operation_wall_micros":total_wall_micros, "operation_cpu_micros":total_cpu_micros,
        "receiver_cpu_micros":metrics.receiver_cpu_micros, "receiver_wall_micros":metrics.receiver_wall_micros,
        "exchange_cpu_micros":metrics.exchange_cpu_micros,"exchange_wall_micros":metrics.exchange_wall_micros,
        "selection":selection
    }))
}

fn distribution(values: &[f64]) -> RunResult<Value> {
    if values.is_empty() || values.iter().any(|n| !n.is_finite()) {
        return Err("empty or nonfinite distribution".into());
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    Ok(json!({
        "count":n,"min":sorted[0],"median":(sorted[(n-1)/2]+sorted[n/2])/2.0,
        "p95":sorted[(95*n).div_ceil(100)-1],"max":sorted[n-1]
    }))
}

fn row_metric(row: &Value, field: &str) -> RunResult<f64> {
    Ok(row[field].as_u64().ok_or("missing numeric observation")? as f64)
}

fn summarize(rows: &[Value], config: &Config) -> RunResult<Vec<Value>> {
    if rows.len() != expected_operations(config.trials, config.sizes.len()) {
        return Err("incomplete observation set".into());
    }
    let mut pairs: BTreeMap<&str, BTreeMap<(&str, &str), &Value>> = BTreeMap::new();
    for row in rows {
        let pair = row["pair"].as_str().ok_or("missing pair")?;
        let arm = row["arm"].as_str().ok_or("missing arm")?;
        let phase = row["phase"].as_str().ok_or("missing phase")?;
        if !ARMS.contains(&arm)
            || !["seed", "transfer"].contains(&phase)
            || pairs
                .entry(pair)
                .or_default()
                .insert((arm, phase), row)
                .is_some()
        {
            return Err("unknown or duplicate arm/phase".into());
        }
    }
    if pairs.len() != config.trials * config.sizes.len() * CASES.len() {
        return Err("pair count mismatch".into());
    }
    for arms in pairs.values() {
        let raw = arms.get(&("raw", "transfer")).ok_or("missing raw arm")?;
        let warm = !raw["base"].is_null();
        if arms.len() != ARMS.len() * if warm { 2 } else { 1 } {
            return Err("missing paired observation".into());
        }
        let mut best_fixed = u64::MAX;
        for arm in ARMS {
            let row = arms.get(&(arm, "transfer")).ok_or("missing transfer arm")?;
            if row["object"] != raw["object"]
                || row["object_bytes"] != raw["object_bytes"]
                || row["base"] != raw["base"]
            {
                return Err("unpaired fixtures".into());
            }
            if warm {
                let seed = arms.get(&(arm, "seed")).ok_or("missing seed")?;
                if seed["object"] != row["base"] || seed["actual_mode"] != 0 {
                    return Err("incorrect seed identity or mode".into());
                }
            }
            if arm != "adaptive" {
                best_fixed = best_fixed.min(
                    row["http_body_equivalent_bytes"]
                        .as_u64()
                        .ok_or("missing body")?,
                );
            }
        }
        let adaptive = arms
            .get(&("adaptive", "transfer"))
            .ok_or("missing adaptive arm")?;
        if adaptive["http_body_equivalent_bytes"].as_u64() != Some(best_fixed) {
            return Err("adaptive body is not the minimum of the measured fixed arms".into());
        }
    }
    let fields = [
        "http_body_equivalent_bytes",
        "operation_wall_micros",
        "operation_cpu_micros",
        "receiver_cpu_micros",
        "receiver_wall_micros",
        "exchange_cpu_micros",
        "exchange_wall_micros",
    ];
    let mut output = vec![
        json!({"event":"paired_acceptance","pairs":pairs.len(),"adaptive_equals_best_fixed_body_pairs":pairs.len(),"all_fixture_and_seed_identities_match":true}),
    ];
    for &size in &config.sizes {
        for case in CASES {
            for arm in ARMS {
                let selected: Vec<_> = rows
                    .iter()
                    .filter(|row| {
                        row["size"] == size
                            && row["case"] == case
                            && row["arm"] == arm
                            && row["phase"] == "transfer"
                    })
                    .collect();
                if selected.len() != config.trials {
                    return Err("cell count mismatch".into());
                }
                let mut summary = json!({"event":"summary","case":case,"size":size,"arm":arm,"count":selected.len()});
                let mut modes = BTreeMap::new();
                for row in &selected {
                    let mode = row["actual_mode"].as_u64().ok_or("missing actual mode")?;
                    *modes.entry(mode.to_string()).or_insert(0usize) += 1;
                }
                summary["selected_modes"] = json!(modes);
                for field in fields {
                    let values = selected
                        .iter()
                        .map(|row| row_metric(row, field))
                        .collect::<RunResult<Vec<_>>>()?;
                    summary[field] = distribution(&values)?;
                    let mut lifecycle = Vec::new();
                    let mut seeds = Vec::new();
                    let mut raw_ratios = Vec::new();
                    for (row, value) in selected.iter().zip(values) {
                        let pair = row["pair"].as_str().ok_or("missing pair")?;
                        let arms = pairs.get(pair).ok_or("missing pair")?;
                        let raw = arms.get(&("raw", "transfer")).ok_or("missing raw")?;
                        if value > 0.0 {
                            raw_ratios.push(row_metric(raw, field)? / value);
                        }
                        if let Some(seed) = arms.get(&(arm, "seed")) {
                            let cost = row_metric(seed, field)?;
                            seeds.push(cost);
                            lifecycle.push(cost + value);
                        }
                    }
                    if raw_ratios.len() == selected.len() {
                        summary[format!("paired_raw_over_candidate_{field}")] =
                            distribution(&raw_ratios)?;
                    }
                    if !seeds.is_empty() {
                        summary[format!("seed_{field}")] = distribution(&seeds)?;
                        summary[format!("seed_plus_transfer_{field}")] = distribution(&lifecycle)?;
                    }
                }
                output.push(summary);
            }
        }
    }
    Ok(output)
}

fn run(config: &Config) -> RunResult<()> {
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&config.out)?;
    fs::create_dir(&config.cache)?;
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string();
    fs::write(config.cache.join(".selector-owner"), started.as_bytes())?;
    write_row(
        &mut output,
        &json!({
            "event":"registration", "schema":"spiralcore.selector.local.v1", "started_unix_nanos":started,
            "claimed_source_revision":config.source, "source_pin_verification":"external custody record required",
            "executable_sha256":sha256_file(&std::env::current_exe()?)?,
            "harness_source_sha256":hex(&Sha256::digest(include_bytes!("codec_selection.rs")).into()),
            "os":std::env::consts::OS,"arch":std::env::consts::ARCH,
            "hardware_sha256_feature":cfg!(feature="hardware-sha256"),
            "trials":config.trials,"sizes":config.sizes,"cases":CASES,"arms":ARMS,
            "expected_operations":expected_operations(config.trials,config.sizes.len()),
            "order":"cyclic arm rotation by trial and case index, five positions balanced for 30 trials",
            "transport":"same-process serial encode/decode, no sockets, HTTP, TLS or WAN",
            "body_accounting":"SCWBAT01 serialized requests; serialized replies plus 16 SCWREP01 metric bytes",
            "timing":"input-ready to verified durable publication return, including encoding and serialization; endpoint open, fixtures, reopen comparison, row writes and cache cleanup excluded",
            "cpu":"one process serial combined sender and receiver; receiver and exchange are nested subsets, never add to operation total",
            "cache_policy":"fresh per arm and trial, optional DAG warming disabled equally; raw seed separately measured for warm arms; endpoint reopened between seed and update",
            "claims_excluded":["WAN speed","network-only send time","energy","peak memory","FIPS","MVM"]
        }),
    )?;
    let mut completed = 0;
    let mut observations = Vec::new();
    for &size in &config.sizes {
        for (case_index, case) in CASES.iter().enumerate() {
            for trial in 0..config.trials {
                let (base, target) = fixture(case, size, trial)?;
                for arm_index in arm_order(case_index, trial) {
                    let arm = ARMS[arm_index];
                    let label = format!("{size}-{case}-{trial:02}-{arm}");
                    let cache = config.cache.join(&label);
                    let scope = hash(b"spiralcore.selector.local.scope.v1\0", &[label.as_bytes()]);
                    let mut phases = Vec::new();
                    if let Some(base) = &base {
                        phases.push(("seed", base.as_slice(), None));
                    }
                    phases.push(("transfer", target.as_slice(), base.as_deref()));
                    for (phase, bytes, phase_base) in phases {
                        match operation(&cache, scope, bytes, phase_base, arm, phase) {
                            Ok(mut row) => {
                                row["case"] = json!(case);
                                row["size"] = json!(size);
                                row["trial"] = json!(trial);
                                row["pair"] = json!(format!("{size}-{case}-{trial:02}"));
                                row["scope"] = json!(hex(&scope));
                                write_row(&mut output, &row)?;
                                observations.push(row);
                                completed += 1;
                            }
                            Err(error) => {
                                write_row(
                                    &mut output,
                                    &json!({"event":"failure","label":label,"phase":phase,"error":error.to_string(),"operations_exact_before_failure":completed,"cache_preserved":cache}),
                                )?;
                                return Err(error);
                            }
                        }
                    }
                    fs::remove_dir_all(&cache)?;
                    write_row(
                        &mut output,
                        &json!({"event":"cleanup","cache":cache,"status":"removed-after-all-phase-reopen-checks","label":label}),
                    )?;
                }
            }
        }
    }
    if completed != expected_operations(config.trials, config.sizes.len()) {
        return Err("operation count mismatch".into());
    }
    match summarize(&observations, config) {
        Ok(summaries) => {
            for summary in summaries {
                write_row(&mut output, &summary)?;
            }
        }
        Err(error) => {
            write_row(
                &mut output,
                &json!({"event":"failure","stage":"paired-analysis","error":error.to_string(),"operations_exact_before_failure":completed}),
            )?;
            return Err(error);
        }
    }
    write_row(
        &mut output,
        &json!({"event":"completion","status":"exact","operations_exact":completed,"cache_root_retained_with_owner_marker":config.cache}),
    )?;
    Ok(())
}

fn fixture(case: &str, size: usize, trial: usize) -> Result<(Option<Vec<u8>>, Vec<u8>)> {
    if !(64..=client::MAX_BATCHED_OBJECT).contains(&size) {
        return Err(Error::Limit);
    }
    let mut state = 0x1634_10a7_2bc9_05dfu64 ^ size as u64 ^ ((trial as u64 + 1) << 32);
    for byte in case.bytes() {
        state = state.rotate_left(7) ^ u64::from(byte);
    }
    let mut random = |len| {
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect::<Vec<_>>()
    };
    match case {
        "cold_random" => Ok((None, random(size))),
        "cold_structured" | "unrelated_structured" => {
            let base = (case == "unrelated_structured").then(|| random(size));
            let template = random(4096);
            let mut target = Vec::with_capacity(size);
            for record in 0..size.div_ceil(4096) {
                let n = 4096.min(size - target.len());
                let start = target.len();
                target.extend_from_slice(&template[..n]);
                let tag = ((record % 16) as u64).to_le_bytes();
                target[start..start + n.min(8)].copy_from_slice(&tag[..n.min(8)]);
            }
            Ok((base, target))
        }
        "localized_32" => {
            let base = random(size);
            let mut target = base.clone();
            for byte in &mut target[size / 2..size / 2 + 32] {
                *byte ^= 0xa5;
            }
            Ok((Some(base), target))
        }
        _ => Err(Error::Unsupported),
    }
}

fn main() {
    let result = Config::parse(std::env::args().skip(1)).and_then(|config| run(&config));
    if let Err(error) = result {
        eprintln!("codec-selection measurement: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn distribution_uses_midpoint_median_and_nearest_rank_p95() {
        let summary = super::distribution(&[9.0, 1.0, 5.0, 3.0]).unwrap();
        assert_eq!(summary["median"], 4.0);
        assert_eq!(summary["p95"], 9.0);
        let summary = super::distribution(&(1..=30).map(f64::from).collect::<Vec<_>>()).unwrap();
        assert_eq!(summary["median"], 15.5);
        assert_eq!(summary["p95"], 29.0);
        assert!(super::distribution(&[]).is_err());
    }
    #[test]
    fn workload_cardinality_and_order_are_exact_and_balanced() {
        assert_eq!(super::expected_operations(30, 2), 1800);
        let mut counts = [0; 5];
        for trial in 0..30 {
            let order = super::arm_order(2, trial);
            let mut sorted = order;
            sorted.sort_unstable();
            assert_eq!(sorted, [0, 1, 2, 3, 4]);
            counts[order[0]] += 1;
        }
        assert_eq!(counts, [6; 5]);
    }

    #[test]
    fn configuration_rejects_ambiguous_or_out_of_bounds_inputs() {
        let args = [
            "--trials",
            "30",
            "--sizes",
            "1048576,16777216",
            "--out",
            "/tmp/rows",
            "--cache",
            "/tmp/cache",
            "--source",
            "0123456789012345678901234567890123456789",
        ];
        let parsed = super::Config::parse(args.iter().map(|s| s.to_string())).unwrap();
        assert_eq!(parsed.trials, 30);
        assert_eq!(parsed.sizes, [1048576, 16777216]);
        for value in ["0", "31", "-1", ""] {
            let mut bad = args;
            bad[1] = value;
            assert!(super::Config::parse(bad.iter().map(|s| s.to_string())).is_err());
        }
        for value in ["63", "16777217", "1048576,1048576", "", "1048576,"] {
            let mut bad = args;
            bad[3] = value;
            assert!(super::Config::parse(bad.iter().map(|s| s.to_string())).is_err());
        }
        assert!(super::Config::parse(["--unknown".to_string(), "x".to_string()]).is_err());
    }

    #[test]
    fn one_trial_serial_transfer_reopens_exact_bytes_and_preserves_existing_output() {
        let root =
            std::env::temp_dir().join(format!("selector-example-test-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let config = super::Config {
            trials: 1,
            sizes: vec![65536],
            out: root.join("rows.jsonl"),
            cache: root.join("cache"),
            source: "0".repeat(40),
        };
        super::run(&config).unwrap();
        let bytes = std::fs::read(&config.out).unwrap();
        let rows: Vec<serde_json::Value> = bytes
            .split(|b| *b == b'\n')
            .filter(|b| !b.is_empty())
            .map(|b| serde_json::from_slice(b).unwrap())
            .collect();
        assert_eq!(
            rows.iter().filter(|r| r["event"] == "operation").count(),
            30
        );
        assert_eq!(rows.iter().filter(|r| r["event"] == "cleanup").count(), 20);
        assert_eq!(rows.last().unwrap()["operations_exact"], 30);
        for row in rows.iter().filter(|r| r["event"] == "operation") {
            assert_eq!(row["verification"], "exact-after-reopen");
            if let Some(cost) = row["selection"]["expected_http_body_bytes"].as_u64() {
                assert_eq!(cost, row["http_body_equivalent_bytes"].as_u64().unwrap());
            }
        }
        assert!(super::run(&config).is_err());
        assert_eq!(std::fs::read(&config.out).unwrap(), bytes);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fixtures_are_paired_and_localized_changes_have_exact_extent() {
        let (base, target) = super::fixture("localized_32", 65536, 3).unwrap();
        let base = base.unwrap();
        assert_eq!(base.len(), target.len());
        assert_eq!(base.iter().zip(&target).filter(|(a, b)| a != b).count(), 32);
        assert_eq!(super::fixture("localized_32", 65536, 3).unwrap().1, target);
        assert_ne!(super::fixture("localized_32", 65536, 4).unwrap().1, target);
    }
}
