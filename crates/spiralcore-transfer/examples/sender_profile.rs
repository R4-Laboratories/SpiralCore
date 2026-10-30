//! Bounded baseline phase profile; no production instrumentation or inference
//! that these independent timings can be summed into a transfer duration.
#[allow(dead_code)]
#[path = "replay_cpu.rs"]
mod harness;
use cpu_time::ProcessTime;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spiralcore_transfer::{
    delta, hash, hex, object_id,
    shared_runtime::{self, OutputKind, Recipe, State},
    shared_transfer::PreparedPrediction,
    zstd_prefix,
};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    time::Instant,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn sha(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes).into())
}
fn measured<T>(f: impl FnOnce() -> spiralcore_transfer::Result<T>) -> Result<(T, u128, u128)> {
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let value = f()?;
    Ok((value, cpu.elapsed().as_micros(), wall.elapsed().as_micros()))
}
fn row(out: &mut File, value: &Value) -> Result<()> {
    serde_json::to_writer(&mut *out, value)?;
    writeln!(out)?;
    out.flush()?;
    Ok(())
}
fn residual_exact(base: &[u8], target: &[u8], frames: Option<&[Vec<u8>]>) -> Result<()> {
    if let Some(frames) = frames {
        let mut rebuilt = Vec::new();
        let mut operations = 0;
        for frame in frames {
            delta::apply(base, &mut rebuilt, target.len(), &mut operations, frame)?;
        }
        if rebuilt != target {
            return Err("residual reconstruction differs".into());
        }
    }
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: sender_profile NEW_JSONL".into());
    }
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    row(
        &mut out,
        &json!({"event":"registration","trials":3,"discarded_warmups":0,
        "executable_sha256":sha(&fs::read(std::env::current_exe()?)?),
        "source":{"profile":sha(include_bytes!("sender_profile.rs")),"fixture_harness":sha(include_bytes!("replay_cpu.rs")),
            "runtime":sha(include_bytes!("../src/shared_runtime.rs")),"transfer":sha(include_bytes!("../src/shared_transfer.rs")),
            "delta":sha(include_bytes!("../src/delta.rs")),"prefix":sha(include_bytes!("../src/zstd_prefix.rs"))},
        "scope":"independent phases; construction and verification outside phase timing; returned buffers dropped after timing"}),
    )?;
    for trial in 0..3u64 {
        for width in [48usize, 4096] {
            let steps = if width == 48 { 243u32 } else { 4096 };
            let checkpoint = State::seeded(width)?.encode()?;
            let recipe = Recipe {
                steps,
                payload: 1.0,
                output: OutputKind::Trace,
            };
            let prediction =
                shared_runtime::replay(&checkpoint, &recipe, shared_runtime::MAX_OUTPUT)?;
            let (prepared, cpu, wall) = measured(|| PreparedPrediction::new(&checkpoint, &recipe))?;
            row(
                &mut out,
                &json!({"event":"preparation","width":width,"trial":trial,"cpu_us":cpu,"wall_us":wall,"retained_bytes":prepared.retained_bytes()}),
            )?;
            let seed = hash(
                b"spiralcore.runtime.measurement.fixture.v1\0",
                &[
                    &(width as u64).to_le_bytes(),
                    &steps.to_le_bytes(),
                    &trial.to_le_bytes(),
                ],
            );
            let seed = u64::from_le_bytes(seed[..8].try_into()?);
            for case in ["generated", "edited", "unrelated"] {
                let target = harness::target_for(case, &prediction, seed)?;
                let identity = hex(&object_id(&target));
                for phase in [
                    "object_id",
                    "prepared_residual",
                    "prediction_delta",
                    "checkpoint_delta",
                    "checkpoint_prefix",
                ] {
                    let (cpu, wall, encoded_bytes, fallback) = match phase {
                        "object_id" => {
                            let (id, c, w) = measured(|| Ok(object_id(&target)))?;
                            if hex(&id) != identity {
                                return Err("identity differs".into());
                            }
                            (c, w, 32, false)
                        }
                        "prepared_residual" => {
                            let (frames, c, w) = measured(|| prepared.prepare(&target))?;
                            if let Some(frames) = &frames {
                                residual_exact(&prediction, &target, Some(&frames[1..]))?;
                            }
                            (
                                c,
                                w,
                                frames.as_ref().map_or(0, |v| v.iter().map(Vec::len).sum()),
                                frames.is_none(),
                            )
                        }
                        "prediction_delta" | "checkpoint_delta" => {
                            let base = if phase == "prediction_delta" {
                                &prediction
                            } else {
                                &checkpoint
                            };
                            let (encoded, c, w) = measured(|| delta::encode(base, &target))?;
                            residual_exact(
                                base,
                                &target,
                                encoded.as_ref().map(|e| e.frames.as_slice()),
                            )?;
                            (
                                c,
                                w,
                                encoded
                                    .as_ref()
                                    .map_or(0, |v| v.frames.iter().map(Vec::len).sum()),
                                encoded.is_none(),
                            )
                        }
                        _ => {
                            let (encoded, c, w) =
                                measured(|| zstd_prefix::encode(&checkpoint, &target))?;
                            if zstd_prefix::decode(&checkpoint, &encoded, target.len())? != target {
                                return Err("prefix differs".into());
                            }
                            (c, w, encoded.len(), false)
                        }
                    };
                    row(
                        &mut out,
                        &json!({"event":"phase","width":width,"trial":trial,"case":case,"phase":phase,
                    "target_bytes":target.len(),"target_object":identity,"cpu_us":cpu,"wall_us":wall,
                    "returned_bytes":encoded_bytes,"raw_fallback":fallback,"exact_or_explicit_fallback":true}),
                    )?;
                }
            }
        }
    }
    row(
        &mut out,
        &json!({"event":"complete","phases":90,"preparations":6}),
    )
}
