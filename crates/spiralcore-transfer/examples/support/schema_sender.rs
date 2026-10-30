use super::*;
use spiralcore_transfer::schema_zstd;
use std::process::Command;
use zstd::zstd_safe::{CCtx, CParameter};

const ARTIFACT: &[u8] = include_bytes!(
    "../../../../docs/network/evidence/2026-10-30-schema-reuse/artifacts-corrected/schema.artifact"
);

fn session_order(
    trial: usize,
    large: bool,
) -> std::result::Result<[(&'static str, &'static str); 4], &'static str> {
    if trial >= 10 {
        return Err("invalid trial");
    }
    let (a, b) = if (trial + usize::from(large)).is_multiple_of(2) {
        ("baseline", "candidate")
    } else {
        ("candidate", "baseline")
    };
    Ok([
        (a, "schema_zstd_prepared"),
        (b, "schema_zstd_prepared"),
        (b, "zstd"),
        (a, "zstd"),
    ])
}

fn pins() -> Value {
    let mut p = sources();
    p["reuse_harness"] = json!(sha(include_bytes!("schema_reuse.rs")));
    p["sender_harness"] = json!(sha(include_bytes!("schema_sender.rs")));
    p["schema_zstd"] = json!(sha(include_bytes!("../../src/schema_zstd.rs")));
    p
}
fn sender_plan() -> Result<Value> {
    let reuse = schema_reuse::reuse_plan()?;
    Ok(
        json!({"schema":"spiralcore.schema-sender.plan.v1","sources":pins(),"artifact_bytes":ARTIFACT.len(),"artifact_sha256":sha(ARTIFACT),"artifact_object":hex(&object_id(ARTIFACT)),
        "targets":reuse["targets"],"sequence":reuse["sequence"],"target_operations":800,"seed_operations":40,"preparations":40,"sessions":80,"primary_max_ratio":0.90}),
    )
}
fn valid_source(source: &str) -> Result<()> {
    if source.len() != 40
        || !source
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("invalid source commit".into());
    }
    Ok(())
}
fn encoder() -> Result<CCtx<'static>> {
    let mut e = CCtx::try_create().ok_or("zstd allocation failed")?;
    for p in [
        CParameter::CompressionLevel(3),
        CParameter::WindowLog(25),
        CParameter::EnableLongDistanceMatching(true),
        CParameter::NbWorkers(0),
        CParameter::ContentSizeFlag(true),
        CParameter::ChecksumFlag(true),
        CParameter::DictIdFlag(false),
    ] {
        e.set_parameter(p).map_err(|_| "zstd parameter rejected")?;
    }
    Ok(e)
}
fn direct_encode(e: &mut CCtx<'static>, bytes: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    out.try_reserve_exact(zstd::zstd_safe::compress_bound(bytes.len()))?;
    e.compress2(&mut out, bytes)
        .map_err(|_| "direct zstd encode failed")?;
    Ok(out)
}
fn measured<T, F: FnOnce() -> Result<T>>(f: F) -> Result<(T, u128, u128)> {
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let value = f()?;
    let cpu_ns = cpu.elapsed().as_nanos();
    let wall_ns = wall.elapsed().as_nanos();
    Ok((value, cpu_ns, wall_ns))
}
fn profile(out_path: &Path, source: &str) -> Result<()> {
    valid_source(source)?;
    let mut out = fresh(out_path)?;
    write_row(
        &mut out,
        &json!({"event":"registration","source_commit":source,"sources":pins(),"executable_sha256":sha(&fs::read(std::env::current_exe()?)?),"artifact_sha256":sha(ARTIFACT),"fixtures":160,"stages":640}),
    )?;
    let result = (|| -> Result<()> {
        for large in [false, true] {
            for family in FAMILIES {
                let ((prepared, mut combined, mut direct), cpu_ns, wall_ns) = measured(|| {
                    Ok((
                        schema::PreparedSchema::new(ARTIFACT)?,
                        schema_zstd::PreparedEncoder::new(ARTIFACT)?,
                        encoder()?,
                    ))
                })?;
                write_row(
                    &mut out,
                    &json!({"event":"setup","family":family,"large":large,"cpu_ns":cpu_ns,"wall_ns":wall_ns,"scope":"prepared schema plus combined encoder plus standalone compression context"}),
                )?;
                for index in 0..20 {
                    let target = fixture(family, large, index)?;
                    let intermediate = prepared.encode(&target.bytes)?;
                    let (encoded_schema, sc_cpu, sc_wall) =
                        measured(|| Ok(prepared.encode(&target.bytes)?))?;
                    if encoded_schema != intermediate {
                        return Err("schema stage differs".into());
                    }
                    write_row(
                        &mut out,
                        &json!({"event":"stage","stage":"schema","family":family,"large":large,"index":index,"bytes":target.bytes.len(),"output_bytes":encoded_schema.len(),"cpu_ns":sc_cpu,"wall_ns":sc_wall,"output_sha256":sha(&encoded_schema),"exact":true}),
                    )?;
                    let (compressed, d_cpu, d_wall) =
                        measured(|| direct_encode(&mut direct, &intermediate))?;
                    if schema_zstd::decode(ARTIFACT, &compressed, target.bytes.len())?
                        != target.bytes
                    {
                        return Err("direct compression decode differs".into());
                    }
                    write_row(
                        &mut out,
                        &json!({"event":"stage","stage":"compress_intermediate","family":family,"large":large,"index":index,"bytes":target.bytes.len(),"output_bytes":compressed.len(),"cpu_ns":d_cpu,"wall_ns":d_wall,"output_sha256":sha(&compressed),"exact":true}),
                    )?;
                    let (encoded, c_cpu, c_wall) =
                        measured(|| Ok(combined.encode(&target.bytes)?))?;
                    if encoded != compressed {
                        return Err("direct and combined zstd frames differ".into());
                    }
                    write_row(
                        &mut out,
                        &json!({"event":"stage","stage":"schema_zstd","family":family,"large":large,"index":index,"bytes":target.bytes.len(),"output_bytes":encoded.len(),"cpu_ns":c_cpu,"wall_ns":c_wall,"output_sha256":sha(&encoded),"exact":true,"retained_bytes":combined.retained_bytes()}),
                    )?;
                    let (id, h_cpu, h_wall) = measured(|| Ok(object_id(&target.bytes)))?;
                    write_row(
                        &mut out,
                        &json!({"event":"stage","stage":"object_id","family":family,"large":large,"index":index,"bytes":target.bytes.len(),"output_bytes":32,"cpu_ns":h_cpu,"wall_ns":h_wall,"object":hex(&id),"exact":true}),
                    )?;
                }
            }
        }
        Ok(())
    })();
    if let Err(e) = result {
        write_row(&mut out, &json!({"event":"failure","error":e.to_string()}))?;
        return Err(e);
    }
    write_row(
        &mut out,
        &json!({"event":"complete","fixtures":160,"stages":640,"discarded":0}),
    )
}
fn worker(source: &str, trial: usize, large: bool, arm: &str, dir: &Path) -> Result<()> {
    valid_source(source)?;
    session_order(trial, large)?;
    if !matches!(arm, "schema_zstd_prepared" | "zstd") {
        return Err("invalid arm".into());
    }
    fs::create_dir(dir)?;
    let path = dir.join("observations.jsonl");
    let mut out = fresh(&path)?;
    let artifact = if arm == "schema_zstd_prepared" {
        Some(ARTIFACT)
    } else {
        None
    };
    schema_reuse::launch_recorded_session(&mut out, dir, arm, trial, large, artifact, true)?;
    drop(out);
    let raw = fs::read(&path)?;
    let rows = std::str::from_utf8(&raw)?
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let result = json!({"event":"session","source_commit":source,"sources":pins(),"executable_sha256":sha(&fs::read(std::env::current_exe()?)?),"trial":trial,"large":large,"arm":arm,"rows":rows});
    serde_json::to_writer(std::io::stdout().lock(), &result)?;
    println!();
    fs::remove_dir_all(dir)?;
    Ok(())
}
fn paired(
    plan_path: &Path,
    baseline: &Path,
    base_source: &str,
    candidate: &Path,
    candidate_source: &str,
    out_path: &Path,
    root: &Path,
) -> Result<()> {
    valid_source(base_source)?;
    valid_source(candidate_source)?;
    let plan_bytes = fs::read(plan_path)?;
    let plan: Value = serde_json::from_slice(&plan_bytes)?;
    let expected = sender_plan()?;
    for key in ["targets", "sequence", "artifact_sha256"] {
        if plan[key] != expected[key] {
            return Err("registered fixtures differ".into());
        }
    }
    for key in ["harness", "reuse_harness", "sender_harness"] {
        if plan["sources"][key] != expected["sources"][key] {
            return Err("registered harness differs".into());
        }
    }
    let mut out = fresh(out_path)?;
    fs::create_dir(root)?;
    let base_sha = sha(&fs::read(baseline)?);
    let candidate_sha = sha(&fs::read(candidate)?);
    write_row(
        &mut out,
        &json!({"event":"registration","plan_sha256":sha(&plan_bytes),"baseline_source":base_source,"candidate_source":candidate_source,"baseline_executable_sha256":base_sha,"candidate_executable_sha256":candidate_sha,"artifact_sha256":sha(ARTIFACT),"primary_max_ratio":0.90}),
    )?;
    for trial in 0..10 {
        for large in [false, true] {
            for (version, arm) in session_order(trial, large)? {
                let (exe, source, pin) = if version == "baseline" {
                    (baseline, base_source, &base_sha)
                } else {
                    (candidate, candidate_source, &candidate_sha)
                };
                let dir = root.join(format!("{trial}-{large}-{version}-{arm}"));
                let result = Command::new(exe)
                    .arg("sender-worker")
                    .arg(source)
                    .arg(trial.to_string())
                    .arg(large.to_string())
                    .arg(arm)
                    .arg(&dir)
                    .output();
                let output = match result {
                    Ok(o) => o,
                    Err(e) => {
                        write_row(
                            &mut out,
                            &json!({"event":"failure","trial":trial,"large":large,"version":version,"arm":arm,"error":e.to_string(),"cache":dir}),
                        )?;
                        return Err(e.into());
                    }
                };
                let parsed = serde_json::from_slice::<Value>(&output.stdout);
                if !output.status.success() || parsed.is_err() {
                    write_row(
                        &mut out,
                        &json!({"event":"failure","trial":trial,"large":large,"version":version,"arm":arm,"status":output.status.code(),"stdout":String::from_utf8_lossy(&output.stdout),"stderr":String::from_utf8_lossy(&output.stderr),"cache":dir}),
                    )?;
                    return Err("worker failed; outputs retained".into());
                }
                let mut row = parsed?;
                row["version"] = json!(version);
                write_row(&mut out, &row)?;
                if row["source_commit"] != source || row["executable_sha256"] != *pin {
                    return Err("worker pin differs".into());
                }
            }
        }
    }
    fs::remove_dir(root)?;
    write_row(
        &mut out,
        &json!({"event":"complete","sessions":80,"targets":800,"seeds":40,"preparations":40}),
    )
}

fn session_row<'a>(
    rows: &'a [Value],
    version: &str,
    arm: &str,
    trial: usize,
    large: bool,
) -> Result<&'a Value> {
    let mut found = rows.iter().filter(|r| {
        r["event"] == "session"
            && r["version"] == version
            && r["arm"] == arm
            && r["trial"] == trial
            && r["large"] == large
    });
    let row = found.next().ok_or("missing session")?;
    if found.next().is_some() {
        return Err("duplicate session".into());
    }
    Ok(row)
}
fn summary(rows: &[Value]) -> Result<Value> {
    if rows.len() != 82
        || rows.first().is_none_or(|r| r["event"] != "registration")
        || rows.last().is_none_or(|r| r["event"] != "complete")
    {
        return Err("campaign incomplete".into());
    }
    let mut cells = Vec::new();
    let mut sessions = Vec::new();
    let mut ratios = Vec::new();
    let mut ordinal = 1;
    for trial in 0..10 {
        for large in [false, true] {
            for (version, arm) in session_order(trial, large)? {
                let s = session_row(rows, version, arm, trial, large)?;
                if s != &rows[ordinal] {
                    return Err("worker order differs".into());
                }
                ordinal += 1;
                let observations = s["rows"].as_array().ok_or("missing rows")?;
                let base = session_row(rows, "baseline", arm, trial, large)?["rows"]
                    .as_array()
                    .ok_or("missing baseline rows")?;
                let expected = if arm == "zstd" { 12 } else { 14 };
                if observations.len() != expected || base.len() != expected {
                    return Err("session counts differ".into());
                }
                let mut ids = Vec::new();
                let mut target_ids = std::collections::BTreeSet::new();
                let mut target_count = 0;
                let mut normal_sender = 0.0;
                let mut base_normal_sender = 0.0;
                let mut cpu = 0.0;
                let mut wall = 0.0;
                let mut body = 0.0;
                let mut seed_count = 0;
                let mut prep_count = 0;
                for (row, b) in observations.iter().zip(base) {
                    if row["event"] != b["event"] {
                        return Err("session event parity differs".into());
                    }
                    if row["event"] == "operation" {
                        schema_reuse::validate_operation(row, b)?;
                        for key in [
                            "position",
                            "phase",
                            "mode",
                            "body_bytes",
                            "request_body_bytes",
                            "response_body_bytes",
                            "wire_sha256",
                        ] {
                            if row[key] != b[key] {
                                return Err("protocol frame/mode/cost parity failed".into());
                            }
                        }
                        if row["wire_sha256"].as_str().is_none_or(|s| s.len() != 64) {
                            return Err("missing captured transcript".into());
                        }
                        ids.push(row["object"].clone());
                        cpu += numeric(row, "cpu_us")?;
                        wall += numeric(row, "wall_us")?;
                        body += numeric(row, "body_bytes")?;
                        if row["phase"] == "target" {
                            if row["position"] != target_count
                                || !target_ids.insert(row["object"].as_str().ok_or("missing id")?)
                            {
                                return Err("target sequence differs".into());
                            }
                            target_count += 1;
                            if row["family"] == "normal" {
                                normal_sender += numeric(row, "sender_cpu_us")?;
                                base_normal_sender += numeric(b, "sender_cpu_us")?;
                            }
                        } else {
                            seed_count += 1;
                            if row["object"] != hex(&object_id(ARTIFACT)) {
                                return Err("seed differs".into());
                            }
                        }
                    } else if row["event"] == "preparation" {
                        prep_count += 1;
                        if !row["error"].is_null() {
                            return Err("preparation failed".into());
                        }
                        cpu += numeric(row, "cpu_us")?;
                        wall += numeric(row, "wall_us")?;
                    } else if row["event"] == "session_verified" {
                        if !row["error"].is_null() || row["reopened_objects"] != json!(ids) {
                            return Err("reopen verification failed".into());
                        }
                    } else if row["event"] == "cleanup" {
                        if row["owned_store_removed"] != true {
                            return Err("cleanup failed".into());
                        }
                    } else {
                        return Err("unexpected observation".into());
                    }
                }
                if target_count != 10
                    || seed_count != usize::from(arm != "zstd")
                    || prep_count != usize::from(arm != "zstd")
                {
                    return Err("session operation counts differ".into());
                }
                if version == "candidate" && large && arm == "schema_zstd_prepared" {
                    ratios.push(normal_sender / base_normal_sender);
                }
                sessions.push(json!({"version":version,"arm":arm,"trial":trial,"large":large,"cpu_us":cpu,"wall_us":wall,"body_bytes":body,"normal_sender_cpu_us":normal_sender,"includes_actual_seed_and_preparation":true}));
            }
        }
    }
    for version in ["baseline", "candidate"] {
        for arm in ["schema_zstd_prepared", "zstd"] {
            for large in [false, true] {
                for family in FAMILIES {
                    let group: Vec<_> = rows
                        .iter()
                        .filter(|r| {
                            r["event"] == "session"
                                && r["version"] == version
                                && r["arm"] == arm
                                && r["large"] == large
                        })
                        .flat_map(|r| r["rows"].as_array().unwrap())
                        .filter(|r| {
                            r["event"] == "operation"
                                && r["phase"] == "target"
                                && r["family"] == family
                        })
                        .collect();
                    let mut cell = schema_reuse::describe(&group)?;
                    cell["version"] = json!(version);
                    cell["arm"] = json!(arm);
                    cell["large"] = json!(large);
                    cell["family"] = json!(family);
                    cells.push(cell);
                }
            }
        }
    }
    if ratios.len() != 10 || ratios.iter().any(|v| !v.is_finite() || *v <= 0.0) {
        return Err("invalid primary ratios".into());
    }
    let ratio = (ratios.iter().map(|r| r.ln()).sum::<f64>() / 10.0).exp();
    Ok(
        json!({"schema":"spiralcore.schema-sender.summary.v1","exact_parity":true,"target_operations":800,"seed_operations":40,"preparations":40,"sessions":80,"primary_ratios":ratios,
        "primary_geomean_candidate_over_baseline":ratio,"primary_gate_max":0.90,"primary_pass":ratio<=0.90,"cells":cells,"session_totals":sessions,"discarded_observations":0}),
    )
}
pub(super) fn command(args: &[String]) -> Result<()> {
    match args.get(1).map(String::as_str) {
        Some("sender-plan") if args.len()==3=>write_row(&mut fresh(Path::new(&args[2]))?,&sender_plan()?),
        Some("sender-profile") if args.len()==4=>profile(Path::new(&args[3]),&args[2]),
        Some("sender-worker") if args.len()==7=>worker(&args[2],args[3].parse()?,args[4].parse()?,&args[5],Path::new(&args[6])),
        Some("sender-paired") if args.len()==9=>paired(Path::new(&args[2]),Path::new(&args[3]),&args[4],Path::new(&args[5]),&args[6],Path::new(&args[7]),Path::new(&args[8])),
        Some("sender-summary") if args.len()==4=>{let raw=fs::read(&args[2])?;let rows=std::str::from_utf8(&raw)?.lines().map(serde_json::from_str::<Value>).collect::<std::result::Result<Vec<_>,_>>()?;
            let mut report=summary(&rows)?;report["raw_sha256"]=json!(sha(&raw));write_row(&mut fresh(Path::new(&args[3]))?,&report)},
        _=>Err("usage: sender-plan OUT | sender-profile SOURCE OUT | sender-worker SOURCE TRIAL LARGE ARM NEWDIR | sender-paired PLAN BASE_EXE BASE_SOURCE CAND_EXE CAND_SOURCE OUT NEWROOT | sender-summary INPUT OUT".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pairing_balances_source_order_and_rejects_invalid_cells() {
        let mut all = std::collections::BTreeSet::new();
        for trial in 0..10 {
            for large in [false, true] {
                let order = session_order(trial, large).unwrap();
                assert_eq!(order, session_order(trial, large).unwrap());
                assert_eq!(order[0].0, order[3].0);
                assert_eq!(order[1].0, order[2].0);
                assert_ne!(order[0].0, order[1].0);
                for (source, arm) in order {
                    assert!(all.insert((trial, large, source, arm)));
                }
            }
        }
        assert_eq!(all.len(), 80);
        assert_ne!(
            session_order(0, false).unwrap()[0].0,
            session_order(0, true).unwrap()[0].0
        );
        assert!(session_order(10, false).is_err());
    }

    #[test]
    fn recorded_sessions_keep_exact_transcripts_and_strict_output_creation() {
        assert_eq!(
            sha(ARTIFACT),
            "79cddd88c69e34f09efaefc0184d8ca310010fd50defd8958dc20ca28ddadcd8"
        );
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "spiralcore-sender-test-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let mut transcripts = Vec::new();
        for i in 0..2 {
            let dir = root.join(i.to_string());
            fs::create_dir(&dir).unwrap();
            let output = dir.join("rows.jsonl");
            let mut out = fresh(&output).unwrap();
            assert!(fresh(&output).is_err());
            schema_reuse::launch_recorded_session(
                &mut out,
                &dir,
                "schema_zstd_prepared",
                0,
                false,
                Some(ARTIFACT),
                true,
            )
            .unwrap();
            drop(out);
            let rows: Vec<Value> = fs::read_to_string(&output)
                .unwrap()
                .lines()
                .map(|s| serde_json::from_str(s).unwrap())
                .collect();
            let hashes: Vec<_> = rows
                .iter()
                .filter(|r| r["event"] == "operation")
                .map(|r| r["wire_sha256"].as_str().unwrap().to_owned())
                .collect();
            assert_eq!(hashes.len(), 11);
            assert!(hashes.iter().all(|s| s.len() == 64));
            transcripts.push(hashes);
        }
        assert_eq!(transcripts[0], transcripts[1]);
        assert!(summary(&[]).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn standalone_profile_compressor_matches_the_production_frame() {
        let prepared = schema::PreparedSchema::new(ARTIFACT).unwrap();
        let mut combined = schema_zstd::PreparedEncoder::new(ARTIFACT).unwrap();
        let mut direct = encoder().unwrap();
        for large in [false, true] {
            for family in FAMILIES {
                let target = fixture(family, large, 0).unwrap();
                let intermediate = prepared.encode(&target.bytes).unwrap();
                let compressed = direct_encode(&mut direct, &intermediate).unwrap();
                assert_eq!(compressed, combined.encode(&target.bytes).unwrap());
                assert_eq!(
                    schema_zstd::decode(ARTIFACT, &compressed, target.bytes.len()).unwrap(),
                    target.bytes
                );
            }
        }
    }
}
