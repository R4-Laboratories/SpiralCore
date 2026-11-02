//! Public deterministic fixtures for the reliable object-channel campaign.
//! Generation is outside transfer timing; service preparation/loading is charged.
use serde_json::{json, Value};
use spiralcore_gateway::measurement::{self as m, Result};
use spiralcore_transfer::{
    hex, object_id, schema,
    shared_runtime::{self, OutputKind, Recipe, State},
};
use std::{fs, io::Write, path::Path, time::Instant};

const WIDTH: usize = 4096;
const TIERS: [(&str, u32, usize); 2] =
    [("near1m", 289, 1_047_202), ("max-trace", 4096, 14_851_384)];
const ARMS: [&str; 4] = ["raw", "whole-zstd", "warm", "runtime"];

struct Tier {
    checkpoint: Vec<u8>,
    recipe: Vec<u8>,
    generated: Vec<u8>,
    residual: Vec<u8>,
    opaque: Vec<u8>,
    opaque_edited: Vec<u8>,
    edit_offsets: Vec<usize>,
}

fn build_tier(label: &str, steps: u32, trial: u64) -> Result<Tier> {
    let (_, _, expected) = TIERS
        .iter()
        .find(|(name, count, _)| *name == label && *count == steps)
        .ok_or_else(|| m::invalid("unregistered fixture shape"))?;
    if trial > 2 {
        return Err(m::invalid("trial outside registered dose").into());
    }
    let checkpoint = State::seeded(WIDTH)?.encode()?;
    let recipe = Recipe {
        steps,
        payload: 1.0 + trial as f64 / 8.0,
        output: OutputKind::Trace,
    };
    let generated = shared_runtime::replay(&checkpoint, &recipe, shared_runtime::MAX_OUTPUT)?;
    if generated.len() != *expected || checkpoint.len() != 3382 {
        return Err(m::invalid("runtime fixture shape changed").into());
    }
    let edit_offsets: Vec<usize> = (1..=16).map(|i| i * generated.len() / 17).collect();
    let mut residual = generated.clone();
    for offset in &edit_offsets {
        for byte in residual
            .get_mut(*offset..*offset + 32)
            .ok_or_else(|| m::invalid("residual edit range"))?
        {
            *byte ^= 0xa5;
        }
    }
    let opaque = m::fixture(
        generated.len(),
        "opaque",
        0x5350_4952_414c_u64 ^ trial ^ (u64::from(steps) << 32),
    )?;
    let mut opaque_edited = opaque.clone();
    for offset in &edit_offsets {
        for byte in &mut opaque_edited[*offset..*offset + 32] {
            *byte ^= 0xa5;
        }
    }
    Ok(Tier {
        checkpoint,
        recipe: recipe.encode()?,
        generated,
        residual,
        opaque,
        opaque_edited,
        edit_offsets,
    })
}
fn artifact(out: &Path, name: &str, bytes: &[u8]) -> Result<Value> {
    let path = out.join(name);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(
        json!({"path":path,"bytes":bytes.len(),"sha256":m::digest(bytes),"object_id":hex(&object_id(bytes))}),
    )
}
fn generate(out: &Path, trial: u64) -> Result<Value> {
    if trial > 2 || !out.is_absolute() {
        return Err(m::invalid("absolute fresh output and trial 0..2 required").into());
    }
    // create_dir refuses existing files/directories; no existing work is replaced.
    fs::create_dir(out)?;
    let wall = Instant::now();
    let cpu = m::cpu_us()?;
    let fitted = schema::fit(&[b"{\"value\":0,\"message\":\"public training\"}\n"])?;
    let schema = artifact(out, "schema.bin", &fitted)?;
    let mut tiers = Vec::new();
    let mut verify = Vec::new();
    for (label, steps, _) in TIERS {
        let tier = build_tier(label, steps, trial)?;
        let checkpoint = artifact(out, &format!("{label}-checkpoint.bin"), &tier.checkpoint)?;
        let recipe = artifact(out, &format!("{label}-recipe.bin"), &tier.recipe)?;
        let generated = artifact(out, &format!("{label}-generated.bin"), &tier.generated)?;
        let residual = artifact(out, &format!("{label}-residual.bin"), &tier.residual)?;
        let opaque = artifact(out, &format!("{label}-opaque.bin"), &tier.opaque)?;
        let opaque_edited = artifact(
            out,
            &format!("{label}-opaque-edited.bin"),
            &tier.opaque_edited,
        )?;
        for entry in [&generated, &residual, &opaque, &opaque_edited] {
            verify.push(entry["path"].clone());
        }
        tiers.push(json!({"label":label,"width":WIDTH,"steps":steps,
            "payload":1.0+trial as f64/8.0,"checkpoint":checkpoint,"recipe":recipe,
            "generated":generated,"residual":residual,"opaque":opaque,"opaque_edited":opaque_edited,
            "repeat_of":"residual","residual_edit_offsets":tier.edit_offsets,
            "residual_edit_length":32,"residual_xor_byte":165,"residual_changed_bytes":512}));
    }
    for arm in ARMS {
        let groups: Vec<Value> = tiers
            .iter()
            .map(|tier| {
                let mut operations = Vec::new();
                if arm == "warm" {
                    operations.push(json!({"label":"schema","action":"schema","warm_probe":false}));
                } else if arm == "runtime" {
                    operations.push(json!({"label":"checkpoint","action":"checkpoint","warm_probe":false}));
                }
                for (label, kind) in [
                    ("generated", "generated"),
                    ("residual", "residual"),
                    ("repeat", "residual"),
                    ("unrelated", "opaque"),
                    ("opaque-edited", "opaque_edited"),
                    ("opaque-edited-repeat", "opaque_edited"),
                ] {
                    let action = if arm == "warm" && label == "generated" { "seed" } else { "send" };
                    operations.push(json!({"label":label,"path":tier[kind]["path"],"action":action,"warm_probe":action=="send"}));
                }
                let mut group = json!({"label":tier["label"],"mode":arm,"operations":operations});
                if arm == "warm" {
                    group["schema_path"] = schema["path"].clone();
                } else if arm == "runtime" {
                    group["checkpoint_path"] = tier["checkpoint"]["path"].clone();
                    group["recipe_path"] = tier["recipe"]["path"].clone();
                }
                group
            })
            .collect();
        m::write_json(&out.join(format!("groups-{arm}.json")), &groups)?;
    }
    m::write_json(&out.join("verify-files.json"), &verify)?;
    let manifest = json!({"version":1,"trial":trial,"tiers":tiers,"schema":schema,
        "verify_files":verify,"arms":ARMS,
        "generation_cpu_us":m::cpu_us()? - cpu,"generation_wall_us":wall.elapsed().as_micros() as u64,
        "generation_scope":"fixture generation and fixture writes outside transfer timing; final manifest serialization excluded; service input loading, codec preparation, checkpoint replay, seeding and durable publication charged separately",
        "runtime_reuse":"existing shared_runtime::replay, State::seeded and canonical Recipe encoding; no copied CA kernel or received code execution"});
    m::write_json(&out.join("manifest.json"), &manifest)?;
    Ok(manifest)
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        return Err(m::invalid("usage: object_fixtures NEW_ABSOLUTE_OUT TRIAL_0_TO_2").into());
    }
    let manifest = generate(Path::new(&args[1]), args[2].parse()?)?;
    println!("{}", serde_json::to_string(&manifest)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn near_tier_is_deterministic_trial_bound_and_exactly_512_residual_bytes() {
        let a = build_tier("near1m", 289, 0).unwrap();
        let b = build_tier("near1m", 289, 0).unwrap();
        let changed_trial = build_tier("near1m", 289, 1).unwrap();
        assert_eq!(a.checkpoint.len(), 3382);
        assert_eq!(a.generated.len(), TIERS[0].2);
        assert_eq!(a.generated, b.generated);
        assert_eq!(a.recipe, b.recipe);
        assert_eq!(a.opaque, b.opaque);
        assert_ne!(a.recipe, changed_trial.recipe);
        assert_ne!(a.generated, changed_trial.generated);
        assert_ne!(a.opaque, changed_trial.opaque);
        assert_eq!(a.residual.len(), a.generated.len());
        assert_eq!(a.opaque.len(), a.generated.len());
        assert_eq!(a.opaque_edited.len(), a.opaque.len());
        assert_ne!(a.opaque, a.generated);
        assert_eq!(a.edit_offsets.len(), 16);
        assert_eq!(
            a.generated
                .iter()
                .zip(&a.residual)
                .filter(|(x, y)| x != y)
                .count(),
            512
        );
        assert_eq!(
            a.opaque
                .iter()
                .zip(&a.opaque_edited)
                .filter(|(x, y)| x != y)
                .count(),
            512
        );
        for offset in a.edit_offsets {
            for i in offset..offset + 32 {
                assert_eq!(a.residual[i], a.generated[i] ^ 0xa5);
                assert_eq!(a.opaque_edited[i], a.opaque[i] ^ 0xa5);
            }
        }
    }

    #[test]
    fn full_fixture_set_has_correct_sizes_and_explicit_group_setup() {
        let temp = tempfile::tempdir().unwrap();
        let out = temp.path().join("fixtures");
        let manifest = generate(&out, 0).unwrap();
        assert_eq!(manifest["tiers"].as_array().unwrap().len(), 2);
        let mut expected = Vec::new();
        for (i, (label, steps, length)) in TIERS.into_iter().enumerate() {
            let tier = &manifest["tiers"][i];
            assert_eq!(tier["label"], label);
            assert_eq!(tier["steps"], steps);
            assert_eq!(tier["width"], WIDTH);
            assert_eq!(tier["generated"]["bytes"], length);
            for kind in ["generated", "residual", "opaque", "opaque_edited"] {
                let path = tier[kind]["path"].as_str().unwrap();
                let bytes = fs::read(path).unwrap();
                assert_eq!(bytes.len(), length);
                assert_eq!(tier[kind]["sha256"], m::digest(&bytes));
                expected.push(path.to_string());
            }
        }
        let verify: Vec<String> =
            serde_json::from_slice(&fs::read(out.join("verify-files.json")).unwrap()).unwrap();
        assert_eq!(verify, expected);
        for arm in ARMS {
            let groups: Value =
                serde_json::from_slice(&fs::read(out.join(format!("groups-{arm}.json"))).unwrap())
                    .unwrap();
            assert_eq!(groups.as_array().unwrap().len(), 2);
            for group in groups.as_array().unwrap() {
                assert_eq!(group["mode"], arm);
                let operations = group["operations"].as_array().unwrap();
                let offset = usize::from(arm == "warm" || arm == "runtime");
                assert_eq!(operations.len(), 6 + offset);
                if arm == "warm" {
                    assert_eq!(operations[0]["action"], "schema");
                    assert!(group["schema_path"].is_string());
                } else if arm == "runtime" {
                    assert_eq!(operations[0]["action"], "checkpoint");
                    assert!(group["checkpoint_path"].is_string());
                    assert!(group["recipe_path"].is_string());
                }
                let deliveries = &operations[offset..];
                assert_eq!(deliveries[0]["label"], "generated");
                assert_eq!(
                    deliveries[0]["action"],
                    if arm == "warm" { "seed" } else { "send" }
                );
                assert_eq!(deliveries[1]["label"], "residual");
                assert_eq!(deliveries[2]["label"], "repeat");
                assert_eq!(deliveries[1]["path"], deliveries[2]["path"]);
                assert_eq!(deliveries[3]["label"], "unrelated");
                assert_eq!(deliveries[4]["label"], "opaque-edited");
                assert_eq!(deliveries[5]["label"], "opaque-edited-repeat");
                assert_eq!(deliveries[4]["path"], deliveries[5]["path"]);
                assert!(deliveries
                    .iter()
                    .filter(|op| op["action"] == "send")
                    .all(|op| op["warm_probe"] == true));
            }
        }
    }

    #[test]
    fn invalid_trial_or_existing_output_is_rejected_without_overwrite() {
        let temp = tempfile::tempdir().unwrap();
        let existing = temp.path().join("existing");
        fs::create_dir(&existing).unwrap();
        fs::write(existing.join("marker"), b"preserve").unwrap();
        assert!(generate(&existing, 0).is_err());
        assert_eq!(fs::read(existing.join("marker")).unwrap(), b"preserve");
        let invalid = temp.path().join("invalid");
        assert!(generate(&invalid, 3).is_err());
        assert!(!invalid.exists());
        assert!(generate(Path::new("relative-output"), 0).is_err());
        assert!(build_tier("near1m", 4096, 0).is_err());
    }
}
