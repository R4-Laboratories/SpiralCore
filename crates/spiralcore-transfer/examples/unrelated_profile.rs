//! Independent stage observations; never sum these as one transfer's cost.
use cpu_time::ProcessTime;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spiralcore_transfer::{
    client, delta,
    endpoint::{ColdGraphPolicy, Endpoint},
    hash, hex, object_id,
    protocol::{Frame, Mode},
    schema, schema_selection, schema_zstd, wan, zstd_whole, Id,
};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
    time::Instant,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const ARTIFACT: &[u8] = include_bytes!(
    "../../../docs/network/evidence/2026-10-30-schema-reuse/artifacts-corrected/schema.artifact"
);
const STAGES: [&str; 6] = [
    "delta",
    "schema",
    "whole",
    "schema-zstd",
    "identity",
    "selector",
];
fn sha(b: &[u8]) -> String {
    hex(&Sha256::digest(b).into())
}
fn emit(out: &mut File, row: &Value) -> Result<()> {
    serde_json::to_writer(&mut *out, row)?;
    writeln!(out)?;
    out.flush()?;
    Ok(())
}
fn stage_schedule() -> Vec<(&'static str, usize, bool, &'static str)> {
    let mut rows = Vec::new();
    for large in [false, true] {
        for family in ["localized", "unrelated"] {
            for stage in STAGES {
                for trial in 0..3 {
                    rows.push((stage, trial, large, family));
                }
            }
        }
    }
    rows
}
fn random_bytes(mut state: u64, len: usize) -> Vec<u8> {
    let mut bytes = vec![0; len];
    for chunk in bytes.chunks_mut(8) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        chunk.copy_from_slice(&state.to_le_bytes()[..chunk.len()]);
    }
    bytes
}
fn fixture_seed(trial: usize, large: bool, escape: bool) -> u64 {
    let domain: &[u8] = if escape {
        b"spiralcore.persistent-session.escape.v1\0"
    } else {
        b"spiralcore.warm-object.fixture.v1\0"
    };
    let id = hash(domain, &[&(trial as u64).to_le_bytes(), &[u8::from(large)]]);
    u64::from_le_bytes(id[..8].try_into().unwrap()).max(1)
}
fn profile_fixture(trial: usize, large: bool, family: &str) -> Result<(Vec<u8>, Vec<u8>)> {
    if trial >= 3 || !matches!(family, "localized" | "unrelated") {
        return Err("fixture outside registered profile".into());
    }
    let n = if large { 16 << 20 } else { 1 << 20 };
    let mut base = random_bytes(fixture_seed(trial, large, false), n);
    if family == "unrelated" {
        for index in 0..4 {
            for byte in &mut base[n / 2 + index * 64..n / 2 + index * 64 + 32] {
                *byte ^= 0xa5;
            }
        }
        return Ok((base, random_bytes(fixture_seed(trial, large, true), n)));
    }
    let mut target = base.clone();
    for byte in &mut target[n / 2..n / 2 + 32] {
        *byte ^= 0xa5;
    }
    Ok((base, target))
}
fn record_counts(bytes: &[u8]) -> (usize, usize, bool) {
    let newlines = bytes.iter().filter(|b| **b == b'\n').count();
    let records = bytes.split_inclusive(|b| *b == b'\n').count();
    (newlines, records, records > 65_536)
}
enum Context {
    Plain,
    Schema(schema::PreparedSchema),
    Whole(zstd_whole::PreparedEncoder),
    SchemaZstd(schema_zstd::PreparedEncoder),
    Selector(schema_selection::PreparedSelector),
}
impl Context {
    fn new(stage: &str) -> Result<Self> {
        Ok(match stage {
            "delta" | "identity" => Self::Plain,
            "schema" => Self::Schema(schema::PreparedSchema::new(ARTIFACT)?),
            "whole" => Self::Whole(zstd_whole::PreparedEncoder::new()?),
            "schema-zstd" => Self::SchemaZstd(schema_zstd::PreparedEncoder::new(ARTIFACT)?),
            "selector" => Self::Selector(schema_selection::PreparedSelector::new(ARTIFACT)?),
            _ => return Err("unknown stage".into()),
        })
    }
    fn retention(&self) -> (usize, u64) {
        match self {
            Self::Plain => (0, 0),
            Self::Schema(c) => (c.retained_bytes(), c.retained_charge()),
            Self::Whole(c) => (c.retained_bytes(), c.retained_charge()),
            Self::SchemaZstd(c) => (c.retained_bytes(), c.retained_charge()),
            Self::Selector(c) => (c.retained_bytes(), c.retained_charge()),
        }
    }
    fn encode(
        &mut self,
        stage: &str,
        base: &[u8],
        target: &[u8],
    ) -> spiralcore_transfer::Result<Output> {
        Ok(match self {
            Self::Plain if stage == "delta" => Output::Delta(delta::encode(base, target)?),
            Self::Plain => Output::Identity(object_id(target)),
            Self::Schema(c) => Output::Bytes(c.encode(target)?),
            Self::Whole(c) => Output::Bytes(c.encode(target)?),
            Self::SchemaZstd(c) => Output::Bytes(c.encode(target)?),
            Self::Selector(c) => {
                Output::Selector(Some(c.prepare_warm(target, Some(base), false, false)?))
            }
        })
    }
}
enum Output {
    Delta(Option<delta::Recipe>),
    Bytes(Vec<u8>),
    Identity(Id),
    Selector(Option<schema_selection::PreparedWarmTransfer>),
}
impl Output {
    fn metadata(&self) -> Value {
        match self {
            Self::Delta(None) => json!({"available":false,"encoded_bytes":0}),
            Self::Delta(Some(recipe)) => {
                let mut digest = Sha256::new();
                for body in &recipe.frames {
                    digest.update((body.len() as u64).to_le_bytes());
                    digest.update(body);
                }
                json!({"available":true,"encoded_bytes":recipe.frames.iter().map(Vec::len).sum::<usize>(),"frame_sha256":hex(&digest.finalize().into()),"operations":recipe.operations,"frames":recipe.frames.len()})
            }
            Self::Bytes(bytes) => json!({"encoded_bytes":bytes.len(),"encoded_sha256":sha(bytes)}),
            Self::Identity(id) => json!({"object":hex(id)}),
            Self::Selector(Some(plan)) => {
                let c = plan.selection();
                json!({"selected_mode":format!("{:?}",c.selected_mode),"needs_schema_seed":c.needs_schema_seed,"candidates":c.candidates.iter().map(|v|json!({"mode":format!("{:?}",v.mode),"target_http_body_bytes":v.target_http_body_bytes,"setup_http_body_bytes":v.setup_http_body_bytes,"total_http_body_bytes":v.total_http_body_bytes})).collect::<Vec<_>>(),"pruned":c.pruned.iter().map(|v|json!({"mode":format!("{:?}",v.mode),"target_http_body_lower_bound":v.target_http_body_lower_bound,"setup_http_body_bytes":v.setup_http_body_bytes,"total_http_body_lower_bound":v.total_http_body_lower_bound})).collect::<Vec<_>>()})
            }
            Self::Selector(None) => Value::Null,
        }
    }
}
fn verify(
    stage: &str,
    output: &mut Output,
    base: &[u8],
    target: &[u8],
    cache: &Path,
) -> Result<Value> {
    match output {
        Output::Delta(Some(recipe)) => {
            let mut decoded = Vec::new();
            let mut count = 0;
            for body in &recipe.frames {
                delta::apply(base, &mut decoded, target.len(), &mut count, body)?;
            }
            if decoded != target {
                return Err("sparse decode differs".into());
            }
        }
        Output::Delta(None) => {
            return Ok(
                json!({"exact":true,"scope":"no sparse recipe; Raw fallback retains original bytes"}),
            )
        }
        Output::Identity(id) => {
            if *id != object_id(target) {
                return Err("identity differs".into());
            }
        }
        Output::Bytes(encoded) => {
            let decoded = match stage {
                "schema" => schema::decode(ARTIFACT, encoded, target.len())?,
                "whole" => zstd_whole::decode(encoded, target.len())?,
                "schema-zstd" => schema_zstd::decode(ARTIFACT, encoded, target.len())?,
                _ => return Err("invalid byte stage".into()),
            };
            if decoded != target {
                return Err("decoded output differs".into());
            }
        }
        Output::Selector(plan) => {
            let plan = plan.take().ok_or("selector already consumed")?;
            if plan.selection().needs_schema_seed {
                return Err("unexpected unseeded schema winner".into());
            }
            let scope = [73; 32];
            let c = wan::context(scope);
            let mut endpoint = Endpoint::open_with_policy(
                cache,
                wan::limits(),
                wan::BenchAuthority,
                ColdGraphPolicy::Disabled,
            )?;
            let mut exchange = |frames: &[Frame]| {
                frames
                    .iter()
                    .map(|f| endpoint.handle(&c, f, 0))
                    .collect::<spiralcore_transfer::Result<Vec<_>>>()
            };
            client::transfer_batched(base, Mode::Raw, None, [1; 32], false, &mut exchange)?;
            let committed = client::transfer_warm_selection_batched(plan, [2; 32], &mut exchange)?;
            if committed.object != object_id(target) {
                return Err("selector ACK differs".into());
            }
            drop(endpoint);
            let mut reopened = Endpoint::open_with_policy(
                cache,
                wan::limits(),
                wan::BenchAuthority,
                ColdGraphPolicy::Disabled,
            )?;
            if reopened.read_object(&c, object_id(base))? != base
                || reopened.read_object(&c, object_id(target))? != target
            {
                return Err("selector stopped-store bytes differ".into());
            }
            drop(reopened);
            fs::remove_dir_all(cache)?;
            return Ok(
                json!({"exact":true,"scope":"untimed real Endpoint seed/selected transfer, closed and reopened exact readback","owned_store_removed":true,"selector_plan_consumed_during_verification":true}),
            );
        }
    }
    Ok(json!({"exact":true,"scope":"independent decode or identity comparison after encode timer"}))
}
fn run(out_path: &Path, root: &Path, source: &str) -> Result<()> {
    if source.len() != 40
        || !source
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("source40 required".into());
    }
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out_path)?;
    fs::create_dir(root)?;
    emit(
        &mut out,
        &json!({"event":"registration","schema":"spiralcore.unrelated-stage-profile.v1","source_commit":source,"executable_sha256":sha(&fs::read(std::env::current_exe()?)?),"sources":{"harness":sha(include_bytes!("unrelated_profile.rs")),"delta":sha(include_bytes!("../src/delta.rs")),"schema":sha(include_bytes!("../src/schema.rs")),"schema_zstd":sha(include_bytes!("../src/schema_zstd.rs")),"whole":sha(include_bytes!("../src/zstd_whole.rs")),"selector":sha(include_bytes!("../src/schema_selection.rs")),"client":sha(include_bytes!("../src/client.rs")),"endpoint":sha(include_bytes!("../src/endpoint.rs")),"lockfile":sha(include_bytes!("../Cargo.lock"))},"artifact_sha256":sha(ARTIFACT),"artifact_bytes":ARTIFACT.len(),"artifact_policy":"previously fitted fixed artifact; no refit in diagnostic","observations":72,"trials_per_cell":3,"discarded":0,"hardware_sha256":cfg!(feature="hardware-sha256"),"scope":"independent encode-return calls, not additive transfer components; generation/setup/validation/drop separate; no network or energy claim; selector uses public prepare_warm not cached-base session"}),
    )?;
    let result = (|| -> Result<()> {
        let mut active = None;
        let mut context = Context::Plain;
        let mut observed = 0;
        for (stage, trial, large, family) in stage_schedule() {
            let key = (stage, large, family);
            if active != Some(key) {
                let w = Instant::now();
                let c = ProcessTime::now();
                drop(context);
                let dropped = (c.elapsed().as_nanos(), w.elapsed().as_nanos());
                if let Some(old) = active {
                    emit(
                        &mut out,
                        &json!({"event":"context_drop","cell":old,"cpu_ns":dropped.0,"wall_ns":dropped.1}),
                    )?;
                }
                let w = Instant::now();
                let c = ProcessTime::now();
                let made = Context::new(stage);
                let duration = (c.elapsed().as_nanos(), w.elapsed().as_nanos());
                emit(
                    &mut out,
                    &json!({"event":"setup","stage":stage,"large":large,"family":family,"cpu_ns":duration.0,"wall_ns":duration.1,"error":made.as_ref().err().map(ToString::to_string)}),
                )?;
                context = made?;
                active = Some(key);
            }
            let w = Instant::now();
            let c = ProcessTime::now();
            let (base, target) = profile_fixture(trial, large, family)?;
            let generation = (c.elapsed().as_nanos(), w.elapsed().as_nanos());
            let counts = record_counts(&target);
            let target_id = object_id(&target);
            let base_id = object_id(&base);
            let input_sha = sha(&target);
            let retained = context.retention();
            let charged = retained
                .1
                .checked_add(base.capacity() as u64)
                .and_then(|v| v.checked_add(target.capacity() as u64))
                .and_then(|v| v.checked_add(192 << 20))
                .ok_or("shared charge overflow")?;
            if charged > 512 << 20 {
                return Err("shared sender budget exceeded".into());
            }
            let w = Instant::now();
            let c = ProcessTime::now();
            let output = context.encode(stage, &base, &target);
            let timing = (c.elapsed().as_nanos(), w.elapsed().as_nanos());
            let mut row = json!({"event":"observation","stage":stage,"trial":trial,"large":large,"family":family,"first_use":trial==0,"bytes":target.len(),"base_object":hex(&base_id),"object":hex(&target_id),"sha256":input_sha,"newlines":counts.0,"records":counts.1,"schema_whole_literal_by_record_count":counts.2,"generation_cpu_ns":generation.0,"generation_wall_ns":generation.1,"encode_cpu_ns":timing.0,"encode_wall_ns":timing.1,"retained_bytes_before":retained.0,"retained_charge":retained.1,"shared_charge":charged,"error":output.as_ref().err().map(ToString::to_string)});
            let mut output = match output {
                Ok(v) => v,
                Err(e) => {
                    emit(&mut out, &row)?;
                    return Err(e.into());
                }
            };
            row["output"] = output.metadata();
            let w = Instant::now();
            let c = ProcessTime::now();
            let validation = verify(
                stage,
                &mut output,
                &base,
                &target,
                &root.join(format!("{stage}-{large}-{family}-{trial}")),
            );
            let check_time = (c.elapsed().as_nanos(), w.elapsed().as_nanos());
            row["validation_cpu_ns"] = json!(check_time.0);
            row["validation_wall_ns"] = json!(check_time.1);
            row["validation_error"] = json!(validation.as_ref().err().map(ToString::to_string));
            row["verification"] = validation.as_ref().ok().cloned().unwrap_or(Value::Null);
            let w = Instant::now();
            let c = ProcessTime::now();
            drop(output);
            row["returned_value_drop_cpu_ns"] = json!(c.elapsed().as_nanos());
            row["returned_value_drop_wall_ns"] = json!(w.elapsed().as_nanos());
            row["drop_scope"] = json!(if stage == "selector" {
                "remaining wrapper only; plan consumed in separately timed verification"
            } else {
                "returned output after separately timed verification"
            });
            row["retained_bytes_after"] = json!(context.retention().0);
            emit(&mut out, &row)?;
            validation?;
            observed += 1;
        }
        let w = Instant::now();
        let c = ProcessTime::now();
        drop(context);
        emit(
            &mut out,
            &json!({"event":"context_drop","cell":active,"cpu_ns":c.elapsed().as_nanos(),"wall_ns":w.elapsed().as_nanos()}),
        )?;
        if observed != 72 {
            return Err("registered stage dose incomplete".into());
        }
        fs::remove_dir(root)?;
        emit(
            &mut out,
            &json!({"event":"complete","observations":observed,"all_exact":true,"discarded":0,"owned_store_parent_removed":true}),
        )
    })();
    if let Err(e) = result {
        emit(
            &mut out,
            &json!({"event":"failure","error":e.to_string(),"no_retry":true}),
        )?;
        return Err(e);
    }
    out.sync_all()?;
    Ok(())
}
fn main() -> Result<()> {
    let a = std::env::args().collect::<Vec<_>>();
    if a.len() != 4 {
        return Err("usage: unrelated_profile NEW_JSONL NEW_STORE_PARENT SOURCE40".into());
    }
    run(Path::new(&a[1]), Path::new(&a[2]), &a[3])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn registered_profile_has_exactly_seventy_two_unique_observations() {
        let schedule = stage_schedule();
        assert_eq!(schedule.len(), 72);
        assert_eq!(schedule.iter().copied().collect::<BTreeSet<_>>().len(), 72);
        let stages = [
            "delta",
            "schema",
            "whole",
            "schema-zstd",
            "identity",
            "selector",
        ];
        for stage in stages {
            for large in [false, true] {
                for family in ["localized", "unrelated"] {
                    let trials = schedule
                        .iter()
                        .filter(|s| s.0 == stage && s.2 == large && s.3 == family)
                        .map(|s| s.1)
                        .collect::<Vec<_>>();
                    assert_eq!(trials, vec![0, 1, 2]);
                }
            }
        }
    }

    #[test]
    fn profile_fixtures_preserve_warm_edit_and_independent_escape() {
        let (base, warm) = profile_fixture(0, false, "localized").unwrap();
        assert_eq!(base.len(), 1 << 20);
        assert_eq!(base.iter().zip(&warm).filter(|(a, b)| a != b).count(), 32);
        assert_eq!(&base[..base.len() / 2], &warm[..warm.len() / 2]);
        let (prior, unrelated) = profile_fixture(0, false, "unrelated").unwrap();
        assert!(prior.iter().zip(&unrelated).filter(|(a, b)| a != b).count() > prior.len() / 2);
        assert_ne!(
            spiralcore_transfer::object_id(&base),
            spiralcore_transfer::object_id(&prior)
        );
        assert_eq!(
            profile_fixture(0, false, "unrelated").unwrap(),
            (prior, unrelated)
        );
        assert!(profile_fixture(3, false, "unrelated").is_err());
        assert!(profile_fixture(0, false, "unknown").is_err());
    }

    #[test]
    fn profile_record_counts_preserve_trailing_newline_and_schema_limit() {
        assert_eq!(record_counts(b""), (0, 0, false));
        assert_eq!(record_counts(b"a\n"), (1, 1, false));
        assert_eq!(record_counts(b"a\nb"), (1, 2, false));
        assert_eq!(record_counts(&vec![b'\n'; 65_536]), (65_536, 65_536, false));
        assert_eq!(record_counts(&vec![b'\n'; 65_537]), (65_537, 65_537, true));
    }
}
