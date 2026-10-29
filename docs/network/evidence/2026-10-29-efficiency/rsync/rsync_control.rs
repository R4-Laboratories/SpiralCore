//! Standalone local diagnostic; intentionally outside every production crate.
//! Unsafe code is confined to libc getrusage's initialized-output FFI boundary.
//! Compile against retained release rlibs; see adjacent rsync-control-build.json.
//! No network transport, endpoint-separated CPU, energy, or novelty claim follows.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spiralcore_transfer::{hex, object_id};
use std::{
    collections::BTreeMap,
    env,
    fs::{self, DirBuilder, File, OpenOptions},
    io::{Read, Write},
    mem::MaybeUninit,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::Command,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const MIB: usize = 1 << 20;
const ROOT: &str = "/Users/casey.allard/spiralcore/local/efficiency-20261029";
const RSYNC: &str = "/usr/bin/rsync";
const CASES: [&str; 3] = ["localized_32", "scattered_32x32", "prefix_1"];

#[derive(Debug, PartialEq)]
struct Options {
    plans: Vec<PathBuf>,
    out_name: String,
    trials: u32,
    case: String,
}
fn options(args: &[String]) -> Result<Options> {
    let mut plans = Vec::new();
    let mut flags = BTreeMap::new();
    if args.len() % 2 != 0 {
        return Err("every flag requires exactly one value".into());
    }
    for pair in args.chunks_exact(2) {
        let (key, value) = (&pair[0], &pair[1]);
        match key.as_str() {
            "--plan" => {
                if plans.len() == 2 || plans.contains(&PathBuf::from(value)) {
                    return Err("one or two distinct --plan paths required".into());
                }
                plans.push(PathBuf::from(value));
            }
            "--out-name" | "--trials" | "--case" => {
                if flags.insert(key.clone(), value.clone()).is_some() {
                    return Err(format!("duplicate flag {key}").into());
                }
            }
            _ => return Err(format!("unknown flag {key}").into()),
        }
    }
    let out_name = flags.remove("--out-name").ok_or("--out-name required")?;
    if out_name.is_empty()
        || out_name.len() > 64
        || !out_name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err("--out-name must be 1..64 ASCII letters, digits, - or _".into());
    }
    let trials = flags
        .remove("--trials")
        .unwrap_or_else(|| "30".into())
        .parse()?;
    if ![1, 7, 30].contains(&trials) {
        return Err("--trials must be 1 (pilot), 7, or 30".into());
    }
    let case = flags.remove("--case").unwrap_or_else(|| "all".into());
    if case != "all" && !CASES.contains(&case.as_str()) {
        return Err("unsupported --case".into());
    }
    if plans.is_empty() {
        return Err("--plan required".into());
    }
    Ok(Options {
        plans,
        out_name,
        trials,
        case,
    })
}
#[derive(Clone, Debug)]
struct Task {
    run: String,
    id: String,
    pair: String,
    case: String,
    trial: u32,
    seed: u64,
    original: Value,
}
fn required_str<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing/string field {key}").into())
}
fn required_u64(v: &Value, key: &str) -> Result<u64> {
    v.get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing/integer field {key}").into())
}
fn select_tasks(plans: &[Value], options: &Options) -> Result<Vec<Task>> {
    let mut selected = BTreeMap::new();
    let mut run_pin = None;
    for plan in plans {
        if required_str(plan, "schema")? != "spiralcore.wan.plan.v2"
            || required_str(plan, "fixture_version")? != "xorshift64-v1"
        {
            return Err("only exact v2 / xorshift64-v1 plans are supported".into());
        }
        let run = required_str(plan, "run")?;
        if run.len() != 64 || !run.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid plan run ID".into());
        }
        if run_pin.is_some_and(|p| p != run) {
            return Err("all plans must use the same run ID".into());
        }
        run_pin = Some(run);
        if required_u64(plan, "trials")? < u64::from(options.trials) {
            return Err("plan has too few trials".into());
        }
        let tasks = plan
            .get("tasks")
            .and_then(Value::as_array)
            .ok_or("tasks array required")?;
        if tasks.len() > 1024 {
            return Err("too many plan tasks".into());
        }
        for task in tasks {
            let case = required_str(task, "case")?;
            if required_str(task, "arm")? != "raw_full"
                || !CASES.contains(&case)
                || (options.case != "all" && options.case != case)
            {
                continue;
            }
            let trial = u32::try_from(required_u64(task, "trial")?)?;
            if trial >= options.trials {
                continue;
            }
            if required_u64(task, "size")? != MIB as u64 {
                return Err("selected task must be exactly 1 MiB before mutation".into());
            }
            let row = Task {
                run: run.into(),
                id: required_str(task, "id")?.into(),
                pair: required_str(task, "pair")?.into(),
                case: case.into(),
                trial,
                seed: required_u64(task, "fixture_seed")?,
                original: task.clone(),
            };
            if selected.insert((case.to_owned(), trial), row).is_some() {
                return Err("duplicate selected raw_full case/trial".into());
            }
        }
    }
    let mut ordered = Vec::new();
    for case in CASES {
        if options.case != "all" && options.case != case {
            continue;
        }
        for trial in 0..options.trials {
            ordered.push(
                selected
                    .remove(&(case.into(), trial))
                    .ok_or_else(|| format!("missing raw_full task {case}/{trial}"))?,
            );
        }
    }
    Ok(ordered)
}
fn base_fixture(seed: u64) -> Vec<u8> {
    let mut x = seed.max(1);
    let mut bytes = vec![0; MIB];
    for chunk in bytes.chunks_exact_mut(8) {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        chunk.copy_from_slice(&x.to_le_bytes());
    }
    bytes
}
fn target_fixture(base: &[u8], case: &str) -> Result<Vec<u8>> {
    if base.len() != MIB {
        return Err("base must be 1 MiB".into());
    }
    let mut bytes = base.to_vec();
    match case {
        "localized_32" => {
            for b in &mut bytes[MIB / 2..MIB / 2 + 32] {
                *b ^= 0xa5;
            }
        }
        "scattered_32x32" => {
            for i in 0..32 {
                for b in &mut bytes[i * MIB / 32..i * MIB / 32 + 32] {
                    *b ^= 0xa5;
                }
            }
        }
        "prefix_1" => bytes.insert(0, 0x5a),
        _ => return Err("unsupported fixture".into()),
    }
    Ok(bytes)
}

const STATS: [(&str, bool); 9] = [
    ("Number of files", false),
    ("Number of files transferred", false),
    ("Total file size", true),
    ("Total transferred file size", true),
    ("Unmatched data", true),
    ("Matched data", true),
    ("File list size", true),
    ("Total sent", true),
    ("Total received", true),
];
fn parse_stats(stdout: &str) -> Result<BTreeMap<String, u64>> {
    let mut values = BTreeMap::new();
    for line in stdout.lines() {
        for (label, bytes) in STATS {
            if let Some(value) = line.strip_prefix(&format!("{label}:")) {
                let value = value.strip_prefix(' ').ok_or("missing stats separator")?;
                let value = if bytes {
                    value.strip_suffix(" B").ok_or("invalid byte suffix")?
                } else {
                    value
                };
                if value.is_empty() || !value.bytes().all(|c| c.is_ascii_digit()) {
                    return Err(format!("invalid stat {label}").into());
                }
                if values.insert(label.into(), value.parse()?).is_some() {
                    return Err(format!("duplicate stat {label}").into());
                }
            }
        }
    }
    if values.len() != STATS.len() {
        return Err("missing required statistics".into());
    }
    Ok(values)
}
fn check_stats(stats: &BTreeMap<String, u64>, length: usize, phase: &str, arm: &str) -> Result<()> {
    if stats["Number of files"] != 1
        || stats["Number of files transferred"] != 1
        || stats["Total file size"] != length as u64
        || stats["Total transferred file size"] != length as u64
        || stats["Matched data"].checked_add(stats["Unmatched data"]) != Some(length as u64)
    {
        return Err("single-file transfer count/length/statistics invariant failed".into());
    }
    if (phase == "seed" || arm == "whole_file")
        && (stats["Matched data"] != 0 || stats["Unmatched data"] != length as u64)
    {
        return Err("empty seed or whole-file arm unexpectedly reported matched bytes".into());
    }
    stats["Total sent"]
        .checked_add(stats["Total received"])
        .ok_or("protocol counter overflow")?;
    Ok(())
}
#[derive(Clone, Copy)]
struct Cpu {
    user_ns: u64,
    system_ns: u64,
}
impl Cpu {
    fn delta(self, before: Self) -> Result<Self> {
        Ok(Self {
            user_ns: self
                .user_ns
                .checked_sub(before.user_ns)
                .ok_or("CPU user counter regressed")?,
            system_ns: self
                .system_ns
                .checked_sub(before.system_ns)
                .ok_or("CPU system counter regressed")?,
        })
    }
    fn total(self) -> Result<u64> {
        self.user_ns
            .checked_add(self.system_ns)
            .ok_or_else(|| "CPU overflow".into())
    }
    fn json(self) -> Value {
        json!({"user_ns":self.user_ns,"system_ns":self.system_ns})
    }
}
fn usage(who: libc::c_int) -> Result<Cpu> {
    if who != libc::RUSAGE_SELF && who != libc::RUSAGE_CHILDREN {
        return Err("invalid rusage selector".into());
    }
    let mut value = MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: getrusage writes a valid rusage at this aligned, writable pointer
    // on success. We never read the output if it reports an error.
    let rc = unsafe { libc::getrusage(who, value.as_mut_ptr()) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful getrusage above initialized the complete C structure.
    let value = unsafe { value.assume_init() };
    let nanos = |v: libc::timeval| -> Result<u64> {
        let secs = u64::try_from(v.tv_sec)?;
        let micros = u64::try_from(v.tv_usec)?;
        if micros >= 1_000_000 {
            return Err("invalid timeval".into());
        }
        secs.checked_mul(1_000_000_000)
            .and_then(|x| x.checked_add(micros * 1000))
            .ok_or_else(|| "timeval overflow".into())
    };
    Ok(Cpu {
        user_ns: nanos(value.ru_utime)?,
        system_ns: nanos(value.ru_stime)?,
    })
}
fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn unix_ns() -> Result<String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string())
}
fn ns(start: Instant) -> Result<u64> {
    Ok(u64::try_from(start.elapsed().as_nanos())?)
}
fn read_bounded(path: &Path, cap: usize) -> Result<Vec<u8>> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err("regular non-symlink file required".into());
    }
    let file = File::open(path)?;
    let mut bytes = Vec::new();
    file.take(cap as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > cap {
        return Err("input exceeds cap".into());
    }
    Ok(bytes)
}
fn private_dir(path: &Path) -> Result<()> {
    DirBuilder::new().mode(0o700).create(path)?;
    Ok(())
}
fn sync_dir(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn command_args(arm: &str, source: &Path, destination: &Path) -> Result<Vec<String>> {
    let mode = match arm {
        "delta" => "--no-whole-file",
        "whole_file" => "--whole-file",
        _ => return Err("invalid arm".into()),
    };
    Ok(vec![
        mode.into(),
        "--ignore-times".into(),
        "--cache".into(),
        "--stats".into(),
        "--timeout=30".into(),
        "--".into(),
        source.to_str().ok_or("non-UTF8 source")?.into(),
        format!("{}/", destination.to_str().ok_or("non-UTF8 destination")?),
    ])
}
fn transfer(
    task: &Task,
    arm: &str,
    arm_position: usize,
    phase: &str,
    source: &Path,
    destination: &Path,
    expected: &[u8],
) -> Result<Value> {
    let expected_id = hex(&object_id(expected));
    let args = command_args(arm, source, destination)?;
    let unix_start = unix_ns()?;
    let self_start = usage(libc::RUSAGE_SELF)?;
    let child_start = usage(libc::RUSAGE_CHILDREN)?;
    let start = Instant::now();
    // Absolute executable and paths, no shell; only our owned regular fixture.
    // This waits/reaps rsync before reading cumulative CHILDREN usage. No other
    // children are launched within this measured interval. The 30s option is
    // an rsync I/O timeout, not a claim of hard CPU preemption.
    let output = Command::new(RSYNC)
        .args(&args)
        .env_clear()
        .env("LC_ALL", "C")
        .env("PATH", "/usr/bin:/bin")
        .output()?;
    let rsync_wall_ns = ns(start)?;
    let child_end = usage(libc::RUSAGE_CHILDREN)?;
    let self_command_end = usage(libc::RUSAGE_SELF)?;
    let children = child_end.delta(child_start)?;
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    let mut parsed = None;
    let validation: Result<()> = (|| {
        if !output.status.success() {
            return Err(format!("rsync failed: {}", output.status).into());
        }
        let stats = parse_stats(&stdout)?;
        check_stats(&stats, expected.len(), phase, arm)?;
        parsed = Some(stats);
        let target = destination.join("object.bin");
        let meta = fs::symlink_metadata(&target)?;
        if !meta.is_file() || meta.len() != expected.len() as u64 {
            return Err("unexpected output type/length".into());
        }
        let file = File::open(&target)?;
        let mut bytes = Vec::with_capacity(expected.len());
        (&file)
            .take(expected.len() as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes != expected {
            return Err("exact output comparison failed".into());
        }
        if hex(&object_id(&bytes)) != expected_id {
            return Err("ObjectId mismatch".into());
        }
        file.sync_all()?;
        sync_dir(destination)?;
        // Output buffer and file descriptor teardown occur before final clocks.
        drop(bytes);
        drop(file);
        Ok(())
    })();
    let total_wall_ns = ns(start)?;
    let self_end = usage(libc::RUSAGE_SELF)?;
    let helper = self_end.delta(self_start)?;
    let helper_command = self_command_end.delta(self_start)?;
    let helper_verify_sync = self_end.delta(self_command_end)?;
    let unix_end = unix_ns()?;
    let error = validation.err().map(|e| e.to_string());
    let protocol = parsed
        .as_ref()
        .map(|s| s["Total sent"] + s["Total received"]);
    Ok(json!({
        "schema":"spiralcore.local.rsync.row.v1", "run":task.run,"task_id":task.id,"pair":task.pair,
        "case":task.case,"trial":task.trial,"fixture_seed":task.seed,"wan_task":task.original,
        "arm":arm,"arm_position":arm_position,"phase":phase,"status":if error.is_none(){"verified_synced"}else{"failed"},"error":error,
        "object_id":expected_id,"object_bytes":expected.len(),"unix_start_nanos":unix_start,"unix_end_nanos":unix_end,
        "argv":std::iter::once(RSYNC.to_owned()).chain(args).collect::<Vec<_>>(),"environment":{"LC_ALL":"C","PATH":"/usr/bin:/bin"},
        "stats":parsed,"reported_protocol_bytes":protocol,"stdout":stdout,"stderr":stderr,"exit_code":output.status.code(),
        "metrics":{"rsync_wall_ns":rsync_wall_ns,"post_rsync_validation_sync_wall_ns":total_wall_ns-rsync_wall_ns,"total_wall_ns":total_wall_ns,
            "rsync_children_cpu":children.json(),"helper_cpu":helper.json(),"helper_spawn_wait_cpu":helper_command.json(),"helper_validation_sync_cpu":helper_verify_sync.json(),
            "total_local_process_cpu_ns":children.total()?.checked_add(helper.total()?).ok_or("aggregate CPU overflow")?}
    }))
}
fn append_row(file: &mut File, value: &Value) -> Result<()> {
    serde_json::to_writer(&mut *file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args == ["--help"] {
        println!("rsync-control --plan V2_WARM_JSON [--plan V2_PREFIX_JSON] --out-name NEW_NAME [--trials 1|7|30] [--case all|localized_32|scattered_32x32|prefix_1]");
        return Ok(());
    }
    let options = options(&args)?;
    let mut plans = Vec::new();
    let mut plan_evidence = Vec::new();
    for path in &options.plans {
        let bytes = read_bounded(path, 10 * MIB)?;
        plans.push(serde_json::from_slice(&bytes)?);
        plan_evidence.push(
            json!({"path":fs::canonicalize(path)?,"bytes":bytes.len(),"sha256":sha256(&bytes)}),
        );
    }
    let tasks = select_tasks(&plans, &options)?;
    let version = Command::new(RSYNC)
        .arg("--version")
        .env_clear()
        .env("LC_ALL", "C")
        .output()?;
    if !version.status.success() {
        return Err("rsync --version failed".into());
    }
    let version = String::from_utf8(version.stdout)?;
    if !version.contains("openrsync: protocol version 29")
        || !version.contains("rsync version 2.6.9 compatible")
    {
        return Err("unreviewed rsync version; revalidate statistics contract".into());
    }
    let rsync_hash = sha256(&read_bounded(Path::new(RSYNC), 10 * MIB)?);
    let helper_path = env::current_exe()?;
    let helper_hash = sha256(&read_bounded(&helper_path, 32 * MIB)?);
    let source_path = Path::new(ROOT).join("rsync_control.rs");
    let source_hash = sha256(&read_bounded(&source_path, MIB)?);
    let runs = Path::new(ROOT).join("rsync-runs");
    match private_dir(&runs) {
        Ok(()) => sync_dir(Path::new(ROOT))?,
        Err(e)
            if fs::symlink_metadata(&runs)
                .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink()) =>
        {
            let _ = e;
        }
        Err(e) => return Err(e),
    }
    let run = runs.join(&options.out_name);
    private_dir(&run)?; // refuses an existing directory, file, or symlink
    sync_dir(&runs)?;
    let work = run.join("work");
    private_dir(&work)?;
    let mut rows = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(run.join("rows.jsonl"))?;
    let metadata = json!({
        "schema":"spiralcore.local.rsync.run.v1","started_unix_nanos":unix_ns()?,"arguments":args,
        "trials":options.trials,"pilot":options.trials==1,"case":options.case,"selected_tasks":tasks.len(),
        "plans":plan_evidence,"rsync":{"path":RSYNC,"sha256":rsync_hash,"version":version},
        "helper":{"path":helper_path,"sha256":helper_hash,"source_path":source_path,"source_sha256":source_hash},
        "os":env::consts::OS,"architecture":env::consts::ARCH,
        "method":{
            "transport":"local rsync process transport; reported protocol counters are not captured physical wire bytes",
            "order":"per case, alternate delta/whole_file first with trial parity; odd trial counts differ by one",
            "seed":"each arm independently protocol-transfers the base into an empty destination before its update; no destination copy credited",
            "clock":"fixture generation, source replacement, directory setup, result logging and cleanup excluded; spawn, wait, stats parsing, exact bytes, target ObjectId, file+parent sync and output-buffer teardown included",
            "cpu":"RUSAGE_SELF plus RUSAGE_CHILDREN deltas after synchronous child wait; local aggregate, not separate endpoint CPU",
            "cache":"--cache explicitly enables buffered I/O; source preparation warms source pages; no OS cache flush",
            "sync":"file sync_all plus destination parent sync_all after exact validation; OS sync boundary, not power-loss proof",
            "timeout":"rsync --timeout=30 is an I/O timeout, not hard CPU preemption",
            "limits":"synthetic 1MiB base, prefix target 1MiB+1; at most 90 selected tasks and 360 rows; fixed absolute executable and generated owned paths",
            "claims":"local conventional-delta diagnostic only; no WAN ranking, novelty, energy, FIPS, or production-security claim"
        }
    });
    write_new(
        &run.join("metadata.json"),
        &serde_json::to_vec_pretty(&metadata)?,
    )?;
    sync_dir(&run)?;
    let mut completed = 0usize;
    for (index, task) in tasks.iter().enumerate() {
        let base = base_fixture(task.seed);
        let target = target_fixture(&base, &task.case)?;
        let arms = if task.trial % 2 == 0 {
            ["delta", "whole_file"]
        } else {
            ["whole_file", "delta"]
        };
        for (position, arm) in arms.into_iter().enumerate() {
            let trial_root = work.join(format!("{index:03}-{position}"));
            private_dir(&trial_root)?;
            sync_dir(&work)?;
            let src = trial_root.join("source");
            let dst = trial_root.join("destination");
            private_dir(&src)?;
            private_dir(&dst)?;
            sync_dir(&trial_root)?;
            sync_dir(&src)?;
            sync_dir(&dst)?;
            let source = src.join("object.bin");
            write_new(&source, &base)?;
            sync_dir(&src)?;
            for (phase, expected) in [("seed", base.as_slice()), ("update", target.as_slice())] {
                if phase == "update" {
                    // Source replacement is setup, not destination seeding.
                    let mut source_file = OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .open(&source)?;
                    source_file.write_all(expected)?;
                    source_file.sync_all()?;
                    drop(source_file);
                }
                let row = transfer(task, arm, position, phase, &source, &dst, expected)?;
                append_row(&mut rows, &row)?;
                if row["status"] != "verified_synced" {
                    return Err(format!("failed row preserved in {}", run.display()).into());
                }
                completed += 1;
            }
            // Only generated paths under this freshly-created private run.
            // Cleanup is after measured phases; evidence rows remain retained.
            fs::remove_dir_all(&trial_root)?;
        }
    }
    write_new(
        &run.join("complete.json"),
        &serde_json::to_vec_pretty(
            &json!({"schema":"spiralcore.local.rsync.complete.v1","rows":completed,"finished_unix_nanos":unix_ns()?,"status":"verified_synced"}),
        )?,
    )?;
    sync_dir(&run)?;
    println!("{}", run.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid_stats() -> String {
        "Number of files: 1\nNumber of files transferred: 1\nTotal file size: 1048576 B\nTotal transferred file size: 1048576 B\nUnmatched data: 1024 B\nMatched data: 1047552 B\nFile list size: 27 B\nTotal sent: 2000 B\nTotal received: 3000 B\n\nsent 2000 bytes  received 3000 bytes  50000 bytes/sec\n".into()
    }
    #[test]
    fn stats_valid() {
        let s = parse_stats(&valid_stats()).unwrap();
        check_stats(&s, MIB, "update", "delta").unwrap();
        assert_eq!(s["Total sent"] + s["Total received"], 5000);
    }
    #[test]
    fn stats_missing_and_duplicate() {
        assert!(parse_stats(&valid_stats().replace("Total sent: 2000 B\n", "")).is_err());
        assert!(parse_stats(&(valid_stats() + "Total sent: 2000 B\n")).is_err());
    }
    #[test]
    fn stats_invalid_numbers_units_and_overflow() {
        for value in [
            "-1 B",
            "1,000 B",
            "1 KiB",
            "2000",
            "+2000 B",
            "2000 B ",
            "18446744073709551616 B",
        ] {
            assert!(
                parse_stats(&valid_stats().replace("2000 B", value)).is_err(),
                "{value}"
            );
        }
    }
    #[test]
    fn stats_invariant_rejects_incomplete_and_wholefile_match() {
        let mut s = parse_stats(&valid_stats()).unwrap();
        assert!(check_stats(&s, MIB, "update", "whole_file").is_err());
        s.insert("Matched data".into(), 10);
        assert!(check_stats(&s, MIB, "update", "delta").is_err());
    }
    fn argv(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }
    #[test]
    fn flags_strict() {
        for args in [
            vec![],
            argv(&["--plan"]),
            argv(&["--plan", "p", "--out-name", "../bad"]),
            argv(&["--plan", "p", "--out-name", "ok", "--trials", "8"]),
            argv(&["--plan", "p", "--out-name", "ok", "--out-name", "twice"]),
            argv(&["--plan", "p", "--out-name", "ok", "--unknown", "x"]),
            argv(&["--plan", "p", "--plan", "p", "--out-name", "ok"]),
        ] {
            assert!(options(&args).is_err(), "{args:?}");
        }
        let o = options(&argv(&["--plan", "p", "--out-name", "ok"])).unwrap();
        assert_eq!(o.trials, 30);
    }
    #[test]
    fn fixture_exact_mutation_contract() {
        let b = base_fixture(1);
        assert_eq!(&b[..8], &[0x41, 0x20, 0x82, 0x40, 0, 0, 0, 0]);
        assert_eq!(base_fixture(0), b);
        let local = target_fixture(&b, "localized_32").unwrap();
        let scatter = target_fixture(&b, "scattered_32x32").unwrap();
        for (i, (&x, &y)) in b.iter().zip(&local).enumerate() {
            assert_eq!(
                x ^ y,
                if (MIB / 2..MIB / 2 + 32).contains(&i) {
                    0xa5
                } else {
                    0
                }
            );
        }
        for (i, (&x, &y)) in b.iter().zip(&scatter).enumerate() {
            assert_eq!(x ^ y, if i % (MIB / 32) < 32 { 0xa5 } else { 0 });
        }
        let p = target_fixture(&b, "prefix_1").unwrap();
        assert_eq!(p[0], 0x5a);
        assert_eq!(&p[1..], b);
        assert!(target_fixture(&b, "bad").is_err());
    }
    #[test]
    fn command_flags_force_content_path_without_metadata_or_inplace() {
        let delta = command_args(
            "delta",
            Path::new("/owned/source/object.bin"),
            Path::new("/owned/destination"),
        )
        .unwrap();
        assert_eq!(
            delta,
            argv(&[
                "--no-whole-file",
                "--ignore-times",
                "--cache",
                "--stats",
                "--timeout=30",
                "--",
                "/owned/source/object.bin",
                "/owned/destination/"
            ])
        );
        assert_eq!(
            command_args("whole_file", Path::new("/a"), Path::new("/b")).unwrap()[0],
            "--whole-file"
        );
        assert!(command_args("bad", Path::new("/a"), Path::new("/b")).is_err());
    }
    fn one_plan() -> Value {
        json!({"schema":"spiralcore.wan.plan.v2","fixture_version":"xorshift64-v1","run":"01".repeat(32),"trials":1,"tasks":[{"arm":"raw_full","case":"localized_32","trial":0,"size":MIB,"id":"id","pair":"pair","fixture_seed":1}]})
    }
    #[test]
    fn plan_pin_and_selection() {
        let opts = options(&argv(&[
            "--plan",
            "p",
            "--out-name",
            "x",
            "--trials",
            "1",
            "--case",
            "localized_32",
        ]))
        .unwrap();
        let p = one_plan();
        assert_eq!(select_tasks(&[p.clone()], &opts).unwrap().len(), 1);
        assert!(select_tasks(&[p.clone(), p.clone()], &opts).is_err());
        let mut wrong = p.clone();
        wrong["tasks"][0]["size"] = json!(16 * MIB);
        assert!(select_tasks(&[wrong], &opts).is_err());
        let mut missing = p;
        missing["tasks"] = json!([]);
        assert!(select_tasks(&[missing], &opts).is_err());
    }
    #[test]
    fn cpu_snapshot_nonnegative() {
        for who in [libc::RUSAGE_SELF, libc::RUSAGE_CHILDREN] {
            let first = usage(who).unwrap();
            let next = usage(who).unwrap();
            next.delta(first).unwrap().total().unwrap();
        }
        assert!(usage(99999).is_err());
    }
}
