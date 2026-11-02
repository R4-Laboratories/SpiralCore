//! Local paired allocation/context diagnostic; no production change.
use cpu_time::ProcessTime;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spiralcore_transfer::{hex, object_id, schema, schema_zstd, zstd_whole};
use std::{fs, io::Write, path::Path, time::Instant};
use zstd::zstd_safe::{self, CCtx, CParameter};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn sha(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes).into())
}
fn timed<T>(f: impl FnOnce() -> Result<T>) -> Result<(T, Value)> {
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let value = f()?;
    Ok((
        value,
        json!({"cpu_ns":cpu.elapsed().as_nanos(),"wall_ns":wall.elapsed().as_nanos()}),
    ))
}
fn read(entry: &Value) -> Result<Vec<u8>> {
    let path = Path::new(entry["path"].as_str().ok_or("path missing")?);
    if fs::metadata(path)?.len() > 16 << 20 {
        return Err("input bound".into());
    }
    let bytes = fs::read(path)?;
    if entry["bytes"] != bytes.len()
        || entry["sha256"] != sha(&bytes)
        || entry["object_id"] != hex(&object_id(&bytes))
    {
        return Err("input identity".into());
    }
    Ok(bytes)
}
fn emit(out: &mut fs::File, row: &Value) -> Result<()> {
    serde_json::to_writer(&mut *out, row)?;
    writeln!(out)?;
    out.flush()?;
    Ok(())
}
struct Arm {
    whole: CCtx<'static>,
    second: Option<CCtx<'static>>,
    whole_output: Vec<u8>,
    schema_output: Vec<u8>,
}
// Exact unchanged new_encoder parameters; no alternative codec settings.
fn encoder() -> Result<CCtx<'static>> {
    let mut context = CCtx::try_create().ok_or("context allocation")?;
    for parameter in [
        CParameter::CompressionLevel(3),
        CParameter::WindowLog(25),
        CParameter::EnableLongDistanceMatching(true),
        CParameter::NbWorkers(0),
        CParameter::ContentSizeFlag(true),
        CParameter::ChecksumFlag(true),
        CParameter::DictIdFlag(false),
    ] {
        context
            .set_parameter(parameter)
            .map_err(|_| "context parameters")?;
    }
    Ok(context)
}
fn compress(
    context: &mut CCtx<'static>,
    output: &mut Vec<u8>,
    input: &[u8],
    reuse: bool,
) -> Result<()> {
    if reuse {
        output.clear();
    } else {
        *output = Vec::new();
    }
    output.try_reserve_exact(zstd_safe::compress_bound(input.len()))?;
    context
        .compress2(output, input)
        .map_err(|_| "compression failed")?;
    if context.sizeof() as u64 > zstd_whole::ENCODER_ALLOWANCE {
        return Err("context limit".into());
    }
    Ok(())
}
impl Arm {
    fn new(shared: bool) -> Result<Self> {
        Ok(Self {
            whole: encoder()?,
            second: if shared { None } else { Some(encoder()?) },
            whole_output: Vec::new(),
            schema_output: Vec::new(),
        })
    }
    fn retained(&self) -> usize {
        self.whole.sizeof() + self.second.as_ref().map_or(0, CCtx::sizeof)
    }
    fn encode(&mut self, target: &[u8], intermediate: &[u8]) -> Result<Value> {
        let wall = Instant::now();
        let cpu = ProcessTime::now();
        let reuse = self.second.is_none();
        let (_, whole_time) =
            timed(|| compress(&mut self.whole, &mut self.whole_output, target, reuse))?;
        let (_, schema_time) = timed(|| {
            compress(
                self.second.as_mut().unwrap_or(&mut self.whole),
                &mut self.schema_output,
                intermediate,
                reuse,
            )
        })?;
        let pair_time =
            json!({"cpu_ns":cpu.elapsed().as_nanos(),"wall_ns":wall.elapsed().as_nanos()});
        Ok(
            json!({"whole":whole_time,"schema":schema_time,"pair":pair_time,"retained_context_bytes":self.retained(),"retained_output_capacity":self.whole_output.capacity()+self.schema_output.capacity()}),
        )
    }
}
fn median(mut x: Vec<f64>) -> f64 {
    x.sort_by(f64::total_cmp);
    let n = x.len();
    if n % 2 == 1 {
        x[n / 2]
    } else {
        (x[n / 2 - 1] + x[n / 2]) / 2.0
    }
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 || args[3].len() != 40 || !args[3].bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("usage: schema_context_profile FIXTURES_ROOT NEW_OUTPUT_JSONL SOURCE40".into());
    }
    let mut out = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args[2])?;
    emit(
        &mut out,
        &json!({"event":"registration","variant":"shared-context-reused-output-vectors","source":args[3],"executable_sha256":sha(&fs::read(std::env::current_exe()?)?),"helper_sha256":sha(include_bytes!("schema_context_profile.rs")),"schema_source_sha256":sha(include_bytes!("../src/schema.rs")),"schema_zstd_source_sha256":sha(include_bytes!("../src/schema_zstd.rs")),"whole_source_sha256":sha(include_bytes!("../src/zstd_whole.rs")),"profile_source_sha256":sha(include_bytes!("../src/zstd_dictionary.rs")),"lock_sha256":sha(include_bytes!("../Cargo.lock")),"fixture_trials":3,"tiers":2,"kinds":4,"paired_repetitions":6,"observations":288,"discarded":0,"scope":"Local allocation diagnostic only. Separate arm uses two same-parameter raw CCtx instances and fresh output Vecs; shared arm uses one CCtx and two reusable output Vecs. Both provide full compress_bound capacity. Releasing previous outputs versus clearing reusable outputs is included in each encode timer. Canonical SCVAL construction, input loading, hashes, comparison to independent original Whole and SchemaZstd frames, and decoding are outside compression timers. The intermediate is prepared equally for both arms; no intermediate-allocation saving is claimed. Context constructors separately timed. Pair interval contains nested stages; never add those intervals. Arm CCtx workspace and output capacities are not process RSS. All first-use and repeated cases retained; no network or energy claim."}),
    )?;
    let mut pairs = Vec::new();
    for trial in 0..3 {
        let manifest: Value = serde_json::from_slice(&fs::read(
            Path::new(&args[1]).join(format!("fixtures-{trial}/manifest.json")),
        )?)?;
        let artifact = read(&manifest["schema"])?;
        for tier in manifest["tiers"].as_array().ok_or("tiers missing")? {
            for (kind_index, kind) in ["generated", "residual", "opaque", "fitting"]
                .into_iter()
                .enumerate()
            {
                let target = if kind == "fitting" {
                    let n = tier["generated"]["bytes"].as_u64().ok_or("size missing")? as usize;
                    if n > 16 << 20 {
                        return Err("fitting input bound".into());
                    }
                    let mut bytes = Vec::with_capacity(n);
                    let mut line = 0;
                    while bytes.len() < n {
                        let record = format!(
                            "{{\"value\":{},\"message\":\"{}\"}}\n",
                            line + trial * 17,
                            "public fitting ".repeat(20)
                        );
                        let take = (n - bytes.len()).min(record.len());
                        bytes.extend_from_slice(&record.as_bytes()[..take]);
                        line += 1;
                    }
                    bytes
                } else {
                    read(&tier[kind])?
                };
                let (prepared, prepare_time) =
                    timed(|| Ok(schema::PreparedSchema::new(&artifact)?))?;
                let (intermediate, intermediate_time) = timed(|| Ok(prepared.encode(&target)?))?;
                if intermediate.len() > 16 << 20 {
                    return Err("outside existing public Whole/proof input domain".into());
                }
                // Independent original schema encoder, not two Whole wrappers.
                let expected_schema = schema_zstd::encode(&artifact, &target)?;
                let expected_whole = zstd_whole::encode(&target)?;
                emit(
                    &mut out,
                    &json!({"event":"input","trial":trial,"tier":tier["label"],"kind":kind,"bytes":target.len(),"sha256":sha(&target),"object_id":hex(&object_id(&target)),"artifact_sha256":sha(&artifact),"intermediate_bytes":intermediate.len(),"intermediate_sha256":sha(&intermediate),"reference_schema_sha256":sha(&expected_schema),"schema_prepare":prepare_time,"intermediate_prepare":intermediate_time}),
                )?;
                let (mut separate, separate_setup);
                let (mut shared, shared_setup);
                if (trial + kind_index) % 2 == 0 {
                    (separate, separate_setup) = timed(|| Arm::new(false))?;
                    (shared, shared_setup) = timed(|| Arm::new(true))?;
                } else {
                    (shared, shared_setup) = timed(|| Arm::new(true))?;
                    (separate, separate_setup) = timed(|| Arm::new(false))?;
                }
                emit(
                    &mut out,
                    &json!({"event":"context_setup","trial":trial,"tier":tier["label"],"kind":kind,"separate":separate_setup,"shared":shared_setup,"separate_retained_bytes":separate.retained(),"shared_retained_bytes":shared.retained()}),
                )?;
                for repetition in 0..6 {
                    let (separate_result, shared_result);
                    let shared_first = (trial + kind_index + repetition) % 2 == 1;
                    if shared_first {
                        shared_result = shared.encode(&target, &intermediate)?;
                        separate_result = separate.encode(&target, &intermediate)?;
                    } else {
                        separate_result = separate.encode(&target, &intermediate)?;
                        shared_result = shared.encode(&target, &intermediate)?;
                    }
                    if separate.whole_output != shared.whole_output
                        || separate.schema_output != shared.schema_output
                        || shared.schema_output != expected_schema
                        || shared.whole_output != expected_whole
                    {
                        return Err("canonical compressed bytes differ".into());
                    }
                    for arm in [&separate, &shared] {
                        if zstd_whole::decode(&arm.whole_output, target.len())? != target
                            || schema_zstd::decode(&artifact, &arm.schema_output, target.len())?
                                != target
                        {
                            return Err("exact decode differs".into());
                        }
                    }
                    let pair = json!({"event":"pair","trial":trial,"tier":tier["label"],"kind":kind,"repetition":repetition,"first_use":repetition==0,"shared_first":shared_first,"exact_compressed_bytes":true,"exact_decode":true,"whole_bytes":shared.whole_output.len(),"schema_bytes":shared.schema_output.len(),"separate":separate_result,"shared":shared_result});
                    emit(&mut out, &pair)?;
                    pairs.push(pair);
                }
            }
        }
    }
    if pairs.len() != 144 {
        return Err("incomplete dose".into());
    }
    let mut summaries = Vec::new();
    for tier in ["near1m", "max-trace"] {
        for kind in ["generated", "residual", "opaque", "fitting"] {
            for phase in ["first", "repeated", "all"] {
                let rows: Vec<_> = pairs
                    .iter()
                    .filter(|r| {
                        r["tier"] == tier
                            && r["kind"] == kind
                            && (phase == "all" || (r["first_use"] == true) == (phase == "first"))
                    })
                    .collect();
                for stage in ["whole", "schema", "pair"] {
                    for metric in ["cpu_ns", "wall_ns"] {
                        let numbers = |arm: &str| -> Vec<f64> {
                            rows.iter()
                                .map(|r| r[arm][stage][metric].as_u64().unwrap() as f64)
                                .collect()
                        };
                        let old = numbers("separate");
                        let new = numbers("shared");
                        summaries.push(json!({"tier":tier,"kind":kind,"phase":phase,"stage":stage,"metric":metric,"paired_rows":rows.len(),"separate_median":median(old.clone()),"shared_median":median(new.clone()),"paired_shared_over_separate_median":median(new.iter().zip(old).map(|(n,d)|n/d).collect())}));
                    }
                }
            }
        }
    }
    let summary = json!({"event":"complete","pairs":pairs.len(),"observations":pairs.len()*2,"all_exact":true,"summaries":summaries});
    emit(&mut out, &summary)?;
    let summary_path = Path::new(&args[2]).with_extension("summary.json");
    let summary_file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(summary_path)?;
    serde_json::to_writer_pretty(summary_file, &summary)?;
    println!("288 observations,144 pairs,all exact");
    Ok(())
}
