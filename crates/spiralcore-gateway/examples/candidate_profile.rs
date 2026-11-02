//! Independent candidate CPU stages on the campaign's exact public inputs.
//! These are local diagnostics, not network times or additive transfer costs.
#[allow(dead_code)]
#[path = "object_fixtures.rs"]
mod fixtures;
use serde_json::{json, Value};
use spiralcore_gateway::measurement::{self as m, Result};
use spiralcore_transfer::{
    delta, hex, object_id, schema, schema_zstd, shared_runtime::Recipe,
    shared_transfer::PreparedPrediction, zstd_whole,
};
use std::{fs, path::Path, time::Instant};

fn timed<T>(f: impl FnOnce() -> Result<T>) -> Result<(T, Value)> {
    let cpu = m::cpu_us()?;
    let wall = Instant::now();
    let value = f()?;
    let elapsed = wall.elapsed().as_micros() as u64;
    let cpu = m::cpu_us()? - cpu;
    Ok((value, json!({"cpu_us":cpu,"wall_us":elapsed})))
}
fn read(entry: &Value) -> Result<Vec<u8>> {
    let b = fs::read(
        entry["path"]
            .as_str()
            .ok_or_else(|| m::invalid("fixture path"))?,
    )?;
    if entry["bytes"] != b.len()
        || entry["sha256"] != m::digest(&b)
        || entry["object_id"] != hex(&object_id(&b))
    {
        return Err(m::invalid("fixture identity").into());
    }
    Ok(b)
}
fn verify_delta(base: &[u8], target: &[u8], frames: &[Vec<u8>]) -> Result<()> {
    let mut output = Vec::new();
    let mut ops = 0;
    for frame in frames {
        delta::apply(base, &mut output, target.len(), &mut ops, frame)?;
    }
    if output != target {
        return Err(m::invalid("delta fidelity").into());
    }
    Ok(())
}
fn run(out: &Path) -> Result<Value> {
    fs::create_dir(out)?;
    let mut rows = Vec::new();
    for trial in 0..3 {
        let generated_dir = out.join(format!("fixtures-{trial}"));
        let manifest = fixtures::generate_profile(&generated_dir, trial, "combined")?;
        let artifact = read(&manifest["schema"])?;
        for tier in manifest["tiers"]
            .as_array()
            .ok_or_else(|| m::invalid("tiers"))?
        {
            let checkpoint = read(&tier["checkpoint"])?;
            let recipe = Recipe::decode(&read(&tier["recipe"])?)?;
            let (prediction, prediction_setup) =
                timed(|| Ok(PreparedPrediction::new(&checkpoint, &recipe)?))?;
            let (mut whole, whole_setup) = timed(|| Ok(zstd_whole::PreparedEncoder::new()?))?;
            let (mut compressed_schema, schema_setup) =
                timed(|| Ok(schema_zstd::PreparedEncoder::new(&artifact)?))?;
            let fitted_schema = schema::PreparedSchema::new(&artifact)?;
            let generated = read(&tier["generated"])?;
            if generated != prediction.prediction() {
                return Err(m::invalid("prediction identity").into());
            }
            for kind in ["generated", "residual", "opaque"] {
                let target = read(&tier[kind])?;
                let (runtime, runtime_time) = timed(|| Ok(prediction.prepare(&target)?))?;
                let (sparse, sparse_time) = timed(|| Ok(delta::encode(&generated, &target)?))?;
                let (whole_body, whole_time) = timed(|| Ok(whole.encode(&target)?))?;
                let (schema_body, schema_time) = timed(|| Ok(compressed_schema.encode(&target)?))?;
                let (intermediate, intermediate_time) =
                    timed(|| Ok(fitted_schema.encode(&target)?))?;
                let (id, hash_time) = timed(|| Ok(object_id(&target)))?;
                // Every output survives the timer and is verified separately.
                if id != object_id(&target)
                    || zstd_whole::decode(&whole_body, target.len())? != target
                    || schema_zstd::decode(&artifact, &schema_body, target.len())? != target
                    || fitted_schema.decode(&intermediate, target.len())? != target
                {
                    return Err(m::invalid("codec fidelity").into());
                }
                if let Some(frames) = &runtime {
                    verify_delta(prediction.prediction(), &target, &frames[1..])?;
                }
                if let Some(recipe) = &sparse {
                    verify_delta(&generated, &target, &recipe.frames)?;
                }
                rows.push(json!({"trial":trial,"tier":tier["label"],"kind":kind,
                    "bytes":target.len(),"object_id":hex(&id),"sha256":m::digest(&target),"exact":true,
                    "setup":{"prediction":prediction_setup,"whole":whole_setup,"schema":schema_setup},
                    "stages":{"runtime":runtime_time,"sparse":sparse_time,"whole":whole_time,
                        "schema_zstd":schema_time,"schema_intermediate":intermediate_time,"object_id":hash_time},
                    "output_bytes":{"runtime":runtime.as_ref().map(|f|f.iter().map(Vec::len).sum::<usize>()),
                        "sparse":sparse.as_ref().map(|f|f.frames.iter().map(Vec::len).sum::<usize>()),
                        "whole":whole_body.len(),"schema_zstd":schema_body.len(),"schema_intermediate":intermediate.len()}}));
            }
        }
        // Only this process's freshly created, generated public fixture directory.
        fs::remove_dir_all(generated_dir)?;
    }
    Ok(
        json!({"version":1,"scope":"independent local candidate stages; prepared contexts reused within tier; setup recorded once per tier but repeated by reference in rows; stage timings must not be summed; fixture generation, loading, identity checks and reconstruction verification outside stage timers; no network", "rows":rows}),
    )
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err(m::invalid("usage: candidate_profile NEW_ABSOLUTE_DIRECTORY").into());
    }
    let out = Path::new(&args[1]);
    if !out.is_absolute() {
        return Err(m::invalid("absolute output required").into());
    }
    let result = run(out)?;
    m::write_json(&out.join("profile.json"), &result)?;
    println!(
        "{}",
        serde_json::to_string(
            &json!({"rows":result["rows"].as_array().map(Vec::len),"exact":true})
        )?
    );
    Ok(())
}
