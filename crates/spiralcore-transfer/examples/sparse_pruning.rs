//! Local sparse-encoder experiment; no transfer or WAN timing is measured.
// This example-only switch builds a separate diagnostic executable. No allocator
// hook is compiled into the ordinary timed executable.
#![allow(unexpected_cfgs)]
use cpu_time::ProcessTime;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
pub use spiralcore_transfer::{
    dag, hash, object_id, zstd_prefix, Error, Id, Result, MAX_CHUNK, MAX_FRAME, MAX_OBJECT,
};
use spiralcore_transfer::{delta, hex};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{self, Write},
    path::Path,
    process::Command,
    time::Instant,
};
#[cfg(sparse_allocation_diagnostic)]
#[path = "support/sparse_pruning/allocation.rs"]
mod allocation;
#[allow(dead_code)]
#[path = "../tests/support/delta_legacy_cc0199f.rs"]
mod legacy;
#[allow(dead_code)]
#[path = "../tests/support/protocol_legacy_cc0199f.rs"]
mod protocol;

const CASES: [&str; 7] = [
    "prefix_insert",
    "middle_insert",
    "unrelated",
    "repeat",
    "localized",
    "scattered",
    "periodic",
];
const SIZE: usize = 16 << 20;
type RunResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn random(size: usize, mut state: u64) -> Vec<u8> {
    (0..size)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}
fn fixture(case: &str, pair: usize) -> RunResult<(Vec<u8>, Vec<u8>)> {
    let case_index = CASES
        .iter()
        .position(|c| *c == case)
        .ok_or("unknown case")?;
    let seed =
        0x704e_1039_a268_bd15u64 ^ ((pair as u64 + 1) << 32) ^ ((case_index as u64 + 1) << 16);
    let insert = matches!(case, "prefix_insert" | "middle_insert");
    let mut base = random(if insert { SIZE - 4096 } else { SIZE }, seed);
    let target = match case {
        "prefix_insert" => [&[0x5a; 4096][..], &base].concat(),
        "middle_insert" => [
            &base[..base.len() / 2],
            &[0x5a; 4096][..],
            &base[base.len() / 2..],
        ]
        .concat(),
        "unrelated" => random(SIZE, seed ^ 0xacd7_309f_5b26_8213),
        "repeat" => base.clone(),
        "localized" => {
            let mut t = base.clone();
            for b in &mut t[SIZE / 2..SIZE / 2 + 32] {
                *b ^= 0xa5;
            }
            t
        }
        "scattered" => {
            let mut t = base.clone();
            for start in (4096..SIZE - 32).step_by(32768) {
                for b in &mut t[start..start + 32] {
                    *b ^= 0xa5;
                }
            }
            t
        }
        "periodic" => {
            let template = random(4096, seed);
            for chunk in base.chunks_mut(4096) {
                chunk.copy_from_slice(&template);
            }
            let mut t = base.clone();
            t[..4096].fill(0x5a);
            t
        }
        _ => return Err("unknown case".into()),
    };
    Ok((base, target))
}
struct Job {
    case: &'static str,
    pair: usize,
    arm: &'static str,
}
fn schedule(baseline: bool) -> Vec<Job> {
    let mut jobs = Vec::new();
    for case in CASES {
        for pair in 0..if baseline { 3 } else { 10 } {
            let arms: &[&str] = if baseline {
                &["old"]
            } else if pair % 2 == 0 {
                &["old", "new"]
            } else {
                &["new", "old"]
            };
            for &arm in arms {
                jobs.push(Job { case, pair, arm });
            }
        }
    }
    jobs
}
fn sha_file(path: &Path) -> RunResult<String> {
    let mut file = File::open(path)?;
    let mut h = Sha256::new();
    io::copy(&mut file, &mut h)?;
    Ok(hex(&h.finalize().into()))
}
enum Encoded {
    Old(legacy::Recipe),
    New(delta::Recipe),
}
impl Encoded {
    fn parts(&self) -> (&[Vec<u8>], usize) {
        match self {
            Self::Old(r) => (&r.frames, r.operations),
            Self::New(r) => (&r.frames, r.operations),
        }
    }
}
fn encode(arm: &str, base: &[u8], target: &[u8]) -> RunResult<Option<Encoded>> {
    match arm {
        "old" => Ok(legacy::encode(base, target)?.map(Encoded::Old)),
        "new" => Ok(delta::encode(base, target)?.map(Encoded::New)),
        _ => Err("unknown arm".into()),
    }
}
fn verify(recipe: &Option<Encoded>, base: &[u8], target: &[u8]) -> RunResult<Value> {
    let Some(recipe) = recipe else {
        return Ok(
            json!({"decision":"raw","frames":0,"operations":0,"serialized_bytes":0,"frame_sha256":null,"retained_frame_capacity":0,"verification":"raw-fallback-original-bytes"}),
        );
    };
    let (frames, operations) = recipe.parts();
    let mut h = Sha256::new();
    h.update(b"spiralcore.sparse.recipe.receipt.v1\0");
    h.update((operations as u64).to_le_bytes());
    h.update((frames.len() as u64).to_le_bytes());
    let mut output = Vec::new();
    let mut applied = 0;
    for frame in frames {
        h.update((frame.len() as u64).to_le_bytes());
        h.update(frame);
        legacy::apply(base, &mut output, target.len(), &mut applied, frame)?;
    }
    if output != target || applied != operations {
        return Err("recipe reconstruction mismatch".into());
    }
    Ok(
        json!({"decision":"recipe","frames":frames.len(),"operations":operations,"serialized_bytes":frames.iter().map(Vec::len).sum::<usize>(),"frame_sha256":hex(&h.finalize().into()),"retained_frame_capacity":frames.iter().map(Vec::capacity).sum::<usize>(),"verification":"legacy-decoder-exact-original-bytes"}),
    )
}
fn sample(arm: &str, case: &str, pair: usize, source: &str) -> RunResult<Value> {
    let (base, target) = fixture(case, pair)?;
    let base_id = hex(&object_id(&base));
    let target_id = hex(&object_id(&target));
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let recipe = std::hint::black_box(encode(
        arm,
        std::hint::black_box(&base),
        std::hint::black_box(&target),
    )?);
    let encode_cpu = cpu.elapsed().as_micros();
    let encode_wall = wall.elapsed().as_micros();
    let verification_wall = Instant::now();
    let verification_cpu = ProcessTime::now();
    let receipt = verify(&recipe, &base, &target)?;
    let verification_cpu = verification_cpu.elapsed().as_micros();
    let verification_wall = verification_wall.elapsed().as_micros();
    let drop_wall = Instant::now();
    let drop_cpu = ProcessTime::now();
    drop(recipe);
    let drop_cpu = drop_cpu.elapsed().as_micros();
    let drop_wall = drop_wall.elapsed().as_micros();
    let release_cpu = cpu.elapsed().as_micros();
    let release_wall = wall.elapsed().as_micros();
    Ok(
        json!({"event":"sample","arm":arm,"case":case,"pair":pair,"source_revision":source,
        "pid":std::process::id(),"executable_sha256":sha_file(&std::env::current_exe()?)?,
        "base_object":base_id,"target_object":target_id,"base_bytes":base.len(),"target_bytes":target.len(),
        "encode_cpu_micros":encode_cpu,"encode_wall_micros":encode_wall,
        "drop_cpu_micros":drop_cpu,"drop_wall_micros":drop_wall,
        "encode_plus_drop_cpu_micros":encode_cpu+drop_cpu,"encode_plus_drop_wall_micros":encode_wall+drop_wall,
        "verification_cpu_micros":verification_cpu,"verification_wall_micros":verification_wall,
        "entry_to_release_including_verification_cpu_micros":release_cpu,"entry_to_release_including_verification_wall_micros":release_wall,
        "cpu_resolution_qualified":encode_cpu>=100,"recipe":receipt}),
    )
}
fn write_row(file: &mut File, row: &Value) -> RunResult<()> {
    serde_json::to_writer(&mut *file, row)?;
    file.write_all(b"\n")?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}
#[cfg(sparse_allocation_diagnostic)]
fn diagnose(arm: &str, case: &str, pair: usize, source: &str) -> RunResult<Value> {
    let (base, target) = fixture(case, pair)?;
    let old_source = include_str!("../tests/support/delta_legacy_cc0199f.rs");
    let new_source = include_str!("../src/delta.rs");
    let marker = "vec![3, 0, 0, 0, 0, 1, 0, 0, 0]";
    if old_source.matches(marker).count() != 1 || new_source.matches(marker).count() != 1 {
        return Err("source-bound header allocation marker changed; requalify attribution".into());
    }
    allocation::start()?;
    let result = encode(arm, &base, &target);
    let at_return = allocation::stop_discovery()?;
    let recipe = result?;
    let receipt = verify(&recipe, &base, &target)?;
    if receipt["retained_frame_capacity"].as_u64() != Some(at_return.live_bytes as u64) {
        return Err("tracked capacity differs from actual returned frame Vec capacity".into());
    }
    drop(recipe);
    let after_drop = allocation::snapshot()?;
    if after_drop.live_bytes != 0
        || after_drop.active_buffers != 0
        || after_drop.header_allocations + after_drop.reallocations != after_drop.frees
    {
        return Err("tracked allocation did not have a matching free".into());
    }
    Ok(
        json!({"event":"allocation_sample","arm":arm,"case":case,"pair":pair,"source_revision":source,
        "pid":std::process::id(),"executable_sha256":sha_file(&std::env::current_exe()?)?,
        "legacy_source_sha256":hex(&Sha256::digest(old_source.as_bytes()).into()),
        "candidate_source_sha256":hex(&Sha256::digest(new_source.as_bytes()).into()),
        "base_object":hex(&object_id(&base)),"target_object":hex(&object_id(&target)),
        "base_bytes":base.len(),"target_bytes":target.len(),"recipe":receipt,
        "at_return":at_return,"after_drop":after_drop,
        "policy":"diagnostic tracked-frame alloc-copy-free; actual requested capacities; not production allocator or RSS",
        "source_attribution":"exclusive encode interval; reviewed nine-byte alignment-one header allocation; marker shape checked; new discovery disabled after return",
        "timing_use":"none"}),
    )
}
fn campaign(baseline: bool, diagnostic: bool, source: &str, out: &Path) -> RunResult<()> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(out)?;
    let exe = std::env::current_exe()?;
    let jobs = schedule(baseline);
    write_row(
        &mut file,
        &json!({"event":"registration","schema":if diagnostic {"spiralcore.sparse-pruning.allocations.v1"} else {"spiralcore.sparse-pruning.local.v1"},"baseline":baseline,"source_revision":source,"source_verification":"external custody required","executable_sha256":sha_file(&exe)?,"observations":jobs.len(),"cases":CASES,"size":SIZE,"calls_per_process":1,"minimum_encode_cpu_micros":if diagnostic {Value::Null} else {json!(100)},"schedule":"case-major; ten pairs; even AB odd BA; baseline three old-only processes per case","timing":if diagnostic {"untimed allocation diagnostic"} else {"encode-return; recipe drop separate; verification outside both; continuous release timer explicitly includes verification"},"allocation_instrumentation":diagnostic,"transport":"none"}),
    )?;
    let mut rows = Vec::new();
    for job in &jobs {
        let launched = Command::new(&exe)
            .args([
                if diagnostic { "diagnose" } else { "sample" },
                "--arm",
                job.arm,
                "--case",
                job.case,
                "--pair",
                &job.pair.to_string(),
                "--source",
                source,
            ])
            .output();
        let result = match launched {
            Ok(result) => result,
            Err(error) => {
                write_row(
                    &mut file,
                    &json!({"event":"failure","case":job.case,"pair":job.pair,"arm":job.arm,"stage":"spawn","error":error.to_string()}),
                )?;
                return Err(error.into());
            }
        };
        if !result.status.success() {
            write_row(
                &mut file,
                &json!({"event":"failure","case":job.case,"pair":job.pair,"arm":job.arm,"exit":result.status.code(),"stdout":String::from_utf8_lossy(&result.stdout),"stderr":String::from_utf8_lossy(&result.stderr)}),
            )?;
            return Err("child sample failed; observations retained".into());
        }
        let row: Value = match serde_json::from_slice(&result.stdout) {
            Ok(row) => row,
            Err(error) => {
                write_row(
                    &mut file,
                    &json!({"event":"failure","case":job.case,"pair":job.pair,"arm":job.arm,"stage":"parse-child-output","error":error.to_string(),"stdout":String::from_utf8_lossy(&result.stdout),"stderr":String::from_utf8_lossy(&result.stderr)}),
                )?;
                return Err(error.into());
            }
        };
        write_row(&mut file, &row)?;
        rows.push(row);
    }
    if !baseline {
        for pair in rows.chunks_exact(2) {
            if pair[0]["base_object"] != pair[1]["base_object"]
                || pair[0]["target_object"] != pair[1]["target_object"]
                || pair[0]["recipe"] != pair[1]["recipe"]
            {
                write_row(
                    &mut file,
                    &json!({"event":"failure","reason":"paired output mismatch","pair":pair}),
                )?;
                return Err("paired output mismatch".into());
            }
        }
    }
    let qualified = diagnostic || rows.iter().all(|r| r["cpu_resolution_qualified"] == true);
    write_row(
        &mut file,
        &json!({"event":"completion","samples":rows.len(),"exact_pair_decisions_and_frames":!baseline,"cpu_resolution_qualified":if diagnostic {Value::Null} else {json!(qualified)},"allocation_diagnostic":diagnostic}),
    )?;
    if !qualified {
        return Err("CPU interval below preregistered resolution floor".into());
    }
    Ok(())
}
fn run() -> RunResult<()> {
    let mut args = std::env::args().skip(1);
    let command = args
        .next()
        .ok_or("command required: sample, baseline or campaign")?;
    let mut values = BTreeMap::new();
    while let Some(k) = args.next() {
        let v = args.next().ok_or("missing value")?;
        if values.insert(k, v).is_some() {
            return Err("duplicate option".into());
        }
    }
    let mut take = |key: &str| values.remove(key).ok_or("missing option");
    let source = take("--source")?;
    if source.len() != 40
        || !source
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("source must be40 lowercase hex".into());
    }
    match command.as_str() {
        "sample" | "diagnose" => {
            let arm = take("--arm")?;
            let case = take("--case")?;
            let pair = take("--pair")?.parse::<usize>()?;
            if pair >= 10 || !values.is_empty() {
                return Err("invalid sample options".into());
            }
            if command == "diagnose" {
                #[cfg(sparse_allocation_diagnostic)]
                {
                    println!("{}", diagnose(&arm, &case, pair, &source)?);
                }
                #[cfg(not(sparse_allocation_diagnostic))]
                {
                    return Err("diagnose requires the separate diagnostic build".into());
                }
            } else {
                if cfg!(sparse_allocation_diagnostic) {
                    return Err("timing is forbidden in the diagnostic binary".into());
                }
                println!("{}", sample(&arm, &case, pair, &source)?);
            }
        }
        "baseline" | "campaign" | "allocations" => {
            let out = take("--out")?;
            if !values.is_empty() {
                return Err("unknown option".into());
            }
            let diagnostic = command == "allocations";
            if diagnostic != cfg!(sparse_allocation_diagnostic) {
                return Err("use the separately qualified normal or diagnostic executable".into());
            }
            campaign(command == "baseline", diagnostic, &source, Path::new(&out))?;
        }
        _ => return Err("unknown command".into()),
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("sparse-pruning: {e}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    #[cfg(sparse_allocation_diagnostic)]
    #[test]
    fn diagnostic_follows_live_capacities_and_keeps_exact_output() {
        let row = super::diagnose("old", "prefix_insert", 0, &"0".repeat(40)).unwrap();
        assert_eq!(
            row["at_return"]["live_bytes"],
            row["recipe"]["retained_frame_capacity"]
        );
        assert_eq!(row["after_drop"]["live_bytes"], 0);
        assert_eq!(row["after_drop"]["active_buffers"], 0);
        assert_eq!(row["recipe"]["serialized_bytes"], 4123);
    }
    #[test]
    fn fixtures_have_registered_lengths_and_changes() {
        let (base, target) = super::fixture("prefix_insert", 0).unwrap();
        assert_eq!(target.len(), 16 << 20);
        assert_eq!(base.len(), (16 << 20) - 4096);
        assert_eq!(&target[4096..], base);
        let (base, target) = super::fixture("localized", 0).unwrap();
        assert_eq!(base.iter().zip(&target).filter(|(a, b)| a != b).count(), 32);
    }

    #[test]
    fn schedule_is_ten_interleaved_ab_ba_pairs_per_cell() {
        let jobs = super::schedule(false);
        assert_eq!(jobs.len(), 140);
        assert_eq!(jobs[0].arm, "old");
        assert_eq!(jobs[1].arm, "new");
        assert_eq!(jobs[2].arm, "new");
        assert_eq!(jobs[3].arm, "old");
        assert_eq!(super::schedule(true).len(), 21);
    }

    #[test]
    fn every_registered_fixture_has_exact_legacy_and_current_receipts() {
        for case in super::CASES {
            let (base, target) = super::fixture(case, 0).unwrap();
            let old = super::encode("old", &base, &target).unwrap();
            let new = super::encode("new", &base, &target).unwrap();
            assert_eq!(
                super::verify(&old, &base, &target).unwrap(),
                super::verify(&new, &base, &target).unwrap()
            );
            assert_eq!(target.len(), super::SIZE);
            assert_eq!(old.is_none(), case == "unrelated");
        }
    }
}
