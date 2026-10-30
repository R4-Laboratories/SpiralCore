use super::*;
use spiralcore_transfer::schema_zstd;

const SEQUENCE: [&str; 10] = [
    "normal",
    "normal",
    "normal",
    "normal",
    "variants",
    "variants",
    "drift",
    "drift",
    "unrelated",
    "unrelated",
];

const REUSE_ARMS: [&str; 8] = [
    "raw",
    "zstd",
    "adaptive",
    "schema",
    "schema_zstd",
    "schema_zstd_prepared",
    "dictionary",
    "dictionary_prepared",
];

fn session_index(trial: usize, position: usize) -> std::result::Result<usize, &'static str> {
    if trial >= 10 || position >= 10 {
        return Err("invalid session index");
    }
    Ok(trial * 10 + position)
}

fn ordered_arms(trial: usize, large: bool) -> [&'static str; 8] {
    let mut arms = REUSE_ARMS;
    let mut random = 0x930d_546b_168f_e2c7u64 ^ (trial as u64).rotate_left(13) ^ u64::from(large);
    for i in (1..arms.len()).rev() {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        arms.swap(i, random as usize % (i + 1));
    }
    arms
}

fn reuse_sources() -> Value {
    let mut result = sources();
    result["reuse_harness"] = json!(sha(include_bytes!("schema_reuse.rs")));
    result["schema_zstd"] = json!(sha(include_bytes!("../../src/schema_zstd.rs")));
    result
}

fn reuse_plan() -> Result<Value> {
    let training = training_records();
    let mut targets = Vec::new();
    for trial in 0..10 {
        for large in [false, true] {
            let mut ids = std::collections::BTreeSet::new();
            for (position, family) in SEQUENCE.iter().enumerate() {
                let index = session_index(trial, position)?;
                let target = fixture(family, large, index)?;
                let id = hex(&object_id(&target.bytes));
                if !ids.insert(id.clone()) {
                    return Err("duplicate target in session".into());
                }
                targets.push(json!({"trial":trial,"large":large,"position":position,"family":family,"fixture_index":index,
                    "bytes":target.bytes.len(),"records":target.records,"object":id,"sha256":sha(&target.bytes)}));
            }
        }
    }
    Ok(
        json!({"schema":"spiralcore.schema-reuse.plan.v1","sources":reuse_sources(),"arms":REUSE_ARMS,
        "sequence":SEQUENCE,"trials":10,"target_operations":1600,"seed_operations":120,"sender_preparations":40,
        "training_bytes":training.concat().len(),"training_sha256":sha(&training.concat()),"training_sample_sha256":training.iter().map(|r|sha(r)).collect::<Vec<_>>(),
        "dictionary_budget":8192,"targets":targets}),
    )
}

fn prepared_arm(arm: &str) -> bool {
    arm.ends_with("_prepared")
}
fn fitted_kind(arm: &str) -> Option<&'static str> {
    if arm.starts_with("schema") {
        Some("schema")
    } else if arm.starts_with("dictionary") {
        Some("dictionary")
    } else {
        None
    }
}
enum Sender {
    Unprepared,
    Schema(schema_zstd::PreparedEncoder),
    Dictionary(zstd_dictionary::PreparedEncoder),
}
fn cache_stats(channel: &Channel) -> Value {
    let stats = channel.endpoint.codec_cache_stats();
    json!({"hits":stats.hits,"misses":stats.misses,"preparations":stats.preparations,"evictions":stats.evictions,"resident_bytes":stats.resident_bytes,"entries":stats.entries})
}
fn retained_sender(sender: &Sender) -> (usize, u64) {
    match sender {
        Sender::Schema(p) => (p.retained_bytes(), p.retained_charge()),
        Sender::Dictionary(p) => (p.retained_bytes(), p.retained_charge()),
        Sender::Unprepared => (0, 0),
    }
}

fn persistent_operation(
    channel: &mut Channel,
    sender: &mut Sender,
    artifact: Option<&[u8]>,
    bytes: &[u8],
    arm: &str,
    position: Option<usize>,
) -> Value {
    channel.metrics = Metrics::default();
    let seed = position.is_none();
    let cache_before = cache_stats(channel);
    let transfer = hash(
        b"spiralcore.schema-reuse.transfer.v1\0",
        &[
            &channel.scope,
            &(position.map(|n| n as u64).unwrap_or(u64::MAX)).to_le_bytes(),
        ],
    );
    let mut expected_cost = None;
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let result = (|| -> spiralcore_transfer::Result<client::Stats> {
        let exchange = |frames: &[Frame]| channel.exchange(frames);
        if seed || arm == "raw" {
            return client::transfer_batched(bytes, Mode::Raw, None, transfer, false, exchange);
        }
        match sender {
            Sender::Schema(prepared) => {
                return client::transfer_prepared_schema_zstd_batched(
                    prepared, bytes, transfer, false, exchange,
                )
            }
            Sender::Dictionary(prepared) => {
                return client::transfer_prepared_zstd_dictionary_batched(
                    prepared, bytes, transfer, false, exchange,
                )
            }
            Sender::Unprepared => (),
        }
        match arm {
            "zstd" => client::transfer_batched(bytes, Mode::Zstd, None, transfer, false, exchange),
            "adaptive" => {
                let selected =
                    client::transfer_adaptive_batched(bytes, artifact, transfer, false, exchange)?;
                expected_cost = Some(selected.selection.expected_http_body_bytes);
                Ok(selected.transfer)
            }
            "schema" => client::transfer_schema_batched(
                artifact.ok_or(spiralcore_transfer::Error::Malformed)?,
                bytes,
                transfer,
                false,
                exchange,
            ),
            "schema_zstd" => client::transfer_schema_zstd_batched(
                artifact.ok_or(spiralcore_transfer::Error::Malformed)?,
                bytes,
                transfer,
                false,
                exchange,
            ),
            "dictionary" => client::transfer_zstd_dictionary_batched(
                artifact.ok_or(spiralcore_transfer::Error::Malformed)?,
                bytes,
                transfer,
                false,
                exchange,
            ),
            _ => Err(spiralcore_transfer::Error::Unsupported),
        }
    })();
    let cpu_us = cpu.elapsed().as_micros();
    let wall_us = wall.elapsed().as_micros();
    let cache_after = cache_stats(channel);
    let (sender_retained_bytes, sender_retained_charge) = retained_sender(sender);
    let mut error = result.as_ref().err().map(ToString::to_string);
    let check_wall = Instant::now();
    let check_cpu = ProcessTime::now();
    let mut exact = false;
    if result.is_ok() {
        match channel
            .endpoint
            .read_object(&wan::context(channel.scope), object_id(bytes))
        {
            Ok(restored) => {
                exact = restored == bytes;
                if !exact {
                    error = Some("immediate bytes differ".into());
                }
            }
            Err(e) => error = Some(e.to_string()),
        }
    }
    let verification_cpu = check_cpu.elapsed().as_micros();
    let verification_wall = check_wall.elapsed().as_micros();
    let m = &channel.metrics;
    if expected_cost.is_some_and(|n| n != m.request + m.response) {
        error = Some("adaptive complete-body cost differs".into());
    }
    json!({"event":"operation","phase":if seed {"seed"}else{"target"},"position":position,
        "bytes":bytes.len(),"object":hex(&object_id(bytes)),"cpu_us":cpu_us,"wall_us":wall_us,"sender_cpu_us":cpu_us.saturating_sub(m.exchange_cpu),
        "exchange_cpu_us":m.exchange_cpu,"exchange_wall_us":m.exchange_wall,"receiver_cpu_us":m.receiver_cpu,"receiver_wall_us":m.receiver_wall,
        "request_body_bytes":m.request,"response_body_bytes":m.response,"body_bytes":m.request+m.response,"batches":m.batches,
        "mode":result.as_ref().ok().map(|s|format!("{:?}",s.mode)),"raw_fallback":!seed&&fitted_kind(arm).is_some()&&result.as_ref().is_ok_and(|s|s.mode==Mode::Raw),
        "exact_immediate":exact,"verification_cpu_us":verification_cpu,"verification_wall_us":verification_wall,"error":error,
        "receiver_cache_enabled":prepared_arm(arm),"cache_before":cache_before,"cache_after":cache_after,
        "sender_retained_bytes":sender_retained_bytes,"sender_retained_charge":sender_retained_charge})
}

fn write_session_row(
    out: &mut File,
    mut row: Value,
    arm: &str,
    trial: usize,
    large: bool,
) -> Result<()> {
    row["arm"] = json!(arm);
    row["trial"] = json!(trial);
    row["large"] = json!(large);
    write_row(out, &row)
}

fn session(
    out: &mut File,
    dir: &Path,
    scope: Id,
    arm: &str,
    trial: usize,
    large: bool,
    artifact: Option<&[u8]>,
) -> Result<()> {
    let targets = SEQUENCE
        .iter()
        .enumerate()
        .map(|(p, f)| fixture(f, large, session_index(trial, p)?))
        .collect::<Result<Vec<_>>>()?;
    let mut channel = Channel::open(dir, scope)?;
    channel
        .endpoint
        .set_codec_cache_enabled(prepared_arm(arm))?;
    let mut sender = Sender::Unprepared;
    if let Some(bytes) = artifact {
        let row = persistent_operation(&mut channel, &mut sender, Some(bytes), bytes, arm, None);
        let success = row["exact_immediate"] == true && row["error"].is_null();
        write_session_row(out, row, arm, trial, large)?;
        if !success {
            return Err("seed failed; store retained".into());
        }
    }
    if prepared_arm(arm) {
        let bytes = artifact.ok_or("prepared arm missing artifact")?;
        let wall = Instant::now();
        let cpu = ProcessTime::now();
        let prepared = if arm == "schema_zstd_prepared" {
            schema_zstd::PreparedEncoder::new(bytes).map(Sender::Schema)
        } else {
            zstd_dictionary::PreparedEncoder::new(bytes).map(Sender::Dictionary)
        };
        let cpu_us = cpu.elapsed().as_micros();
        let wall_us = wall.elapsed().as_micros();
        let (retained, charge) = match &prepared {
            Ok(Sender::Schema(p)) => (p.retained_bytes(), p.retained_charge()),
            Ok(Sender::Dictionary(p)) => (p.retained_bytes(), p.retained_charge()),
            _ => (0, 0),
        };
        write_session_row(
            out,
            json!({"event":"preparation","cpu_us":cpu_us,"wall_us":wall_us,"retained_bytes":retained,"retained_charge":charge,
            "artifact_object":hex(&object_id(bytes)),"actual_target_uses":10,"error":prepared.as_ref().err().map(ToString::to_string)}),
            arm,
            trial,
            large,
        )?;
        sender = prepared?;
    }
    for (position, target) in targets.iter().enumerate() {
        let mut row = persistent_operation(
            &mut channel,
            &mut sender,
            artifact,
            &target.bytes,
            arm,
            Some(position),
        );
        row["family"] = json!(SEQUENCE[position]);
        row["records"] = json!(target.records);
        let success = row["exact_immediate"] == true && row["error"].is_null();
        write_session_row(out, row, arm, trial, large)?;
        if !success {
            return Err("target failed; store retained".into());
        }
    }
    let final_cache = cache_stats(&channel);
    drop(channel.endpoint);
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let mut checked = Vec::new();
    let verify = (|| -> Result<()> {
        let mut reopened = Channel::open(dir, scope)?;
        for bytes in artifact
            .into_iter()
            .chain(targets.iter().map(|t| t.bytes.as_slice()))
        {
            let id = object_id(bytes);
            if reopened.endpoint.read_object(&wan::context(scope), id)? != bytes {
                return Err("reopened bytes differ".into());
            }
            checked.push(hex(&id));
        }
        Ok(())
    })();
    let cpu_us = cpu.elapsed().as_micros();
    let wall_us = wall.elapsed().as_micros();
    write_session_row(
        out,
        json!({"event":"session_verified","reopened_objects":checked,"cpu_us":cpu_us,"wall_us":wall_us,
        "final_cache":final_cache,"error":verify.as_ref().err().map(ToString::to_string)}),
        arm,
        trial,
        large,
    )?;
    verify?;
    fs::remove_dir_all(dir)?;
    write_session_row(
        out,
        json!({"event":"cleanup","owned_store_removed":true}),
        arm,
        trial,
        large,
    )
}

fn launch_session(
    out: &mut File,
    cache: &Path,
    arm: &str,
    trial: usize,
    large: bool,
    artifact: Option<&[u8]>,
) -> Result<()> {
    let dir = cache.join(format!("{trial}-{large}-{arm}"));
    let scope = hash(
        b"spiralcore.schema-reuse.scope.v1\0",
        &[
            arm.as_bytes(),
            &(trial as u64).to_le_bytes(),
            &[u8::from(large)],
        ],
    );
    session(out, &dir, scope, arm, trial, large, artifact)
}

fn run_reuse(
    plan_path: &Path,
    source: &str,
    out_path: &Path,
    cache: &Path,
    artifacts: &Path,
) -> Result<()> {
    if source.len() != 40
        || !source
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("invalid source commit".into());
    }
    let plan_bytes = fs::read(plan_path)?;
    let registered: Value = serde_json::from_slice(&plan_bytes)?;
    if registered != reuse_plan()? {
        return Err("fixture/source plan changed".into());
    }
    let mut out = fresh(out_path)?;
    fs::create_dir(cache)?;
    fs::create_dir(artifacts)?;
    write_row(
        &mut out,
        &json!({"event":"registration","source_commit":source,"sources":reuse_sources(),"plan_sha256":sha(&plan_bytes),
        "executable_sha256":sha(&fs::read(std::env::current_exe()?)?),"platform":format!("{}-{}",std::env::consts::ARCH,std::env::consts::OS),
        "target_operations":1600,"seed_operations":120,"sender_preparations":40,"training_bytes":registered["training_bytes"],"training_sha256":registered["training_sha256"],
        "scope":"local serialized durable persistent receiver; combined sender and receiver preparation; body equivalent"}),
    )?;
    let training = training_records();
    let corpus = training.concat();
    let samples: Vec<_> = training.iter().map(Vec::as_slice).collect();
    save(&artifacts.join("training.jsonl"), &corpus)?;
    let mut fitted = Vec::new();
    for name in ["schema", "dictionary"] {
        let wall = Instant::now();
        let cpu = ProcessTime::now();
        let result = if name == "schema" {
            schema::fit(&samples)
        } else {
            zstd_dictionary::train(&samples, 8192)
        };
        let cpu_us = cpu.elapsed().as_micros();
        let wall_us = wall.elapsed().as_micros();
        match result {
            Ok(bytes) => {
                write_row(
                    &mut out,
                    &json!({"event":"fit","arm":name,"cpu_us":cpu_us,"wall_us":wall_us,"bytes":bytes.len(),"sha256":sha(&bytes),"object":hex(&object_id(&bytes)),
                "actual_target_uses":if name=="schema" {600}else{400},"scope":"fit or training plus artifact serialization"}),
                )?;
                save(&artifacts.join(format!("{name}.artifact")), &bytes)?;
                fitted.push(bytes);
            }
            Err(e) => {
                write_row(
                    &mut out,
                    &json!({"event":"fit_failure","arm":name,"cpu_us":cpu_us,"wall_us":wall_us,"error":e.to_string()}),
                )?;
                return Err(e.into());
            }
        }
    }
    for trial in 0..10 {
        for large in [false, true] {
            for arm in ordered_arms(trial, large) {
                let artifact = match fitted_kind(arm) {
                    Some("schema") => Some(fitted[0].as_slice()),
                    Some("dictionary") => Some(fitted[1].as_slice()),
                    _ if arm == "adaptive" => Some(corpus.as_slice()),
                    _ => None,
                };
                if let Err(e) = launch_session(&mut out, cache, arm, trial, large, artifact) {
                    write_session_row(
                        &mut out,
                        json!({"event":"session_failure","error":e.to_string(),"cache_preserved":true}),
                        arm,
                        trial,
                        large,
                    )?;
                    return Err(e);
                }
            }
        }
    }
    fs::remove_dir(cache)?;
    write_row(
        &mut out,
        &json!({"event":"complete","target_operations":1600,"seed_operations":120,"preparations":40,"owned_stores_removed":160}),
    )
}

fn one_row<'a>(
    rows: &'a [Value],
    event: &str,
    arm: &str,
    trial: usize,
    large: bool,
    position: Option<usize>,
) -> Result<&'a Value> {
    let mut found = rows.iter().filter(|r| {
        r["event"] == event
            && r["arm"] == arm
            && r["trial"] == trial
            && r["large"] == large
            && (event != "operation" || r["position"] == json!(position))
    });
    let row = found.next().ok_or("missing session observation")?;
    if found.next().is_some() {
        return Err("duplicate session observation".into());
    }
    Ok(row)
}
fn validate_operation(row: &Value, reference: &Value) -> Result<()> {
    if row["object"] != reference["object"]
        || row["bytes"] != reference["bytes"]
        || row["exact_immediate"] != true
        || !row["error"].is_null()
    {
        return Err("paired/immediate fidelity failure".into());
    }
    if numeric(row, "body_bytes")?
        != numeric(row, "request_body_bytes")? + numeric(row, "response_body_bytes")?
        || numeric(row, "cpu_us")? < numeric(row, "exchange_cpu_us")?
        || numeric(row, "exchange_cpu_us")? < numeric(row, "receiver_cpu_us")?
        || numeric(row, "sender_cpu_us")?
            != numeric(row, "cpu_us")? - numeric(row, "exchange_cpu_us")?
    {
        return Err("byte/timer accounting failure".into());
    }
    for key in ["hits", "misses", "preparations", "evictions"] {
        if numeric(&row["cache_after"], key)? < numeric(&row["cache_before"], key)? {
            return Err("cache counters decreased".into());
        }
    }
    if row["receiver_cache_enabled"] == false
        && (row["cache_after"]["entries"] != 0 || row["cache_after"]["resident_bytes"] != 0)
    {
        return Err("disabled cache retained state".into());
    }
    Ok(())
}
const METRICS: [&str; 13] = [
    "bytes",
    "body_bytes",
    "request_body_bytes",
    "response_body_bytes",
    "cpu_us",
    "sender_cpu_us",
    "wall_us",
    "exchange_cpu_us",
    "exchange_wall_us",
    "receiver_cpu_us",
    "receiver_wall_us",
    "sender_retained_bytes",
    "sender_retained_charge",
];
fn describe(group: &[&Value]) -> Result<Value> {
    let mut result = json!({"n":group.len(),"raw_fallbacks":group.iter().filter(|r|r["raw_fallback"]==true).count()});
    for key in METRICS {
        result[key] = dist(
            group
                .iter()
                .map(|r| numeric(r, key))
                .collect::<Result<Vec<_>>>()?,
        );
    }
    for key in ["hits", "misses", "preparations", "evictions"] {
        result[format!("cache_{key}")] = json!(group
            .iter()
            .map(|r| Ok(numeric(&r["cache_after"], key)? - numeric(&r["cache_before"], key)?))
            .collect::<Result<Vec<f64>>>()?
            .iter()
            .sum::<f64>());
    }
    result["cache_resident_bytes"] = dist(
        group
            .iter()
            .map(|r| numeric(&r["cache_after"], "resident_bytes"))
            .collect::<Result<Vec<_>>>()?,
    );
    Ok(result)
}
fn reuse_summary(rows: &[Value]) -> Result<Value> {
    for (event, expected) in [
        ("registration", 1),
        ("fit", 2),
        ("preparation", 40),
        ("operation", 1720),
        ("session_verified", 160),
        ("cleanup", 160),
        ("complete", 1),
    ] {
        if rows.iter().filter(|r| r["event"] == event).count() != expected {
            return Err("registered campaign counts differ".into());
        }
    }
    if rows.len() != 2084
        || rows.first().is_none_or(|r| r["event"] != "registration")
        || rows.last().is_none_or(|r| r["event"] != "complete")
    {
        return Err("unexpected campaign rows".into());
    }
    let mut cells = Vec::new();
    let mut positions = Vec::new();
    let mut session_cells = Vec::new();
    let mut totals = Vec::new();
    for arm in REUSE_ARMS {
        let model = fitted_kind(arm).and_then(|kind| {
            rows.iter()
                .find(|r| r["event"] == "fit" && r["arm"] == kind)
        });
        let fit_cpu = model
            .map(|r| numeric(r, "cpu_us"))
            .transpose()?
            .unwrap_or(0.0);
        let fit_wall = model
            .map(|r| numeric(r, "wall_us"))
            .transpose()?
            .unwrap_or(0.0);
        let mut total_cpu = fit_cpu;
        let mut total_wall = fit_wall;
        let mut total_body = 0.0;
        for large in [false, true] {
            let mut groups: Vec<Vec<&Value>> = (0..10).map(|_| Vec::new()).collect();
            let mut session_metrics = Vec::new();
            for trial in 0..10 {
                let mut expected_ids = Vec::new();
                let mut unique = std::collections::BTreeSet::new();
                let mut cpu = 0.0;
                let mut wall = 0.0;
                let mut body = 0.0;
                let mut first_cpu = 0.0;
                let mut first_wall = 0.0;
                let mut first_body = 0.0;
                if !matches!(arm, "raw" | "zstd") {
                    let seed = one_row(rows, "operation", arm, trial, large, None)?;
                    validate_operation(seed, seed)?;
                    if seed["phase"] != "seed"
                        || model.is_some_and(|m| {
                            seed["object"] != m["object"] || seed["bytes"] != m["bytes"]
                        })
                    {
                        return Err("artifact seed differs".into());
                    }
                    expected_ids.push(seed["object"].clone());
                    cpu += numeric(seed, "cpu_us")?;
                    wall += numeric(seed, "wall_us")?;
                    body += numeric(seed, "body_bytes")?;
                }
                if prepared_arm(arm) {
                    let prepared = one_row(rows, "preparation", arm, trial, large, None)?;
                    if !prepared["error"].is_null()
                        || prepared["artifact_object"] != model.ok_or("missing model")?["object"]
                        || prepared["actual_target_uses"] != 10
                    {
                        return Err("preparation failed".into());
                    }
                    cpu += numeric(prepared, "cpu_us")?;
                    wall += numeric(prepared, "wall_us")?;
                }
                for (position, group) in groups.iter_mut().enumerate() {
                    let row = one_row(rows, "operation", arm, trial, large, Some(position))?;
                    let raw = one_row(rows, "operation", "raw", trial, large, Some(position))?;
                    validate_operation(row, raw)?;
                    if row["phase"] != "target"
                        || row["family"] != SEQUENCE[position]
                        || row["receiver_cache_enabled"] != prepared_arm(arm)
                        || !unique.insert(row["object"].as_str().ok_or("missing object")?)
                    {
                        return Err("invalid target sequence".into());
                    }
                    if prepared_arm(arm) {
                        let plain = one_row(
                            rows,
                            "operation",
                            arm.strip_suffix("_prepared").unwrap(),
                            trial,
                            large,
                            Some(position),
                        )?;
                        if row["body_bytes"] != plain["body_bytes"] || row["mode"] != plain["mode"]
                        {
                            return Err("prepared path changed selected wire cost".into());
                        }
                    }
                    cpu += numeric(row, "cpu_us")?;
                    wall += numeric(row, "wall_us")?;
                    body += numeric(row, "body_bytes")?;
                    if position == 0 {
                        first_cpu = cpu;
                        first_wall = wall;
                        first_body = body;
                    }
                    expected_ids.push(row["object"].clone());
                    group.push(row);
                }
                let verification = one_row(rows, "session_verified", arm, trial, large, None)?;
                if !verification["error"].is_null()
                    || verification["reopened_objects"] != json!(expected_ids)
                    || one_row(rows, "cleanup", arm, trial, large, None)?["owned_store_removed"]
                        != true
                {
                    return Err("reopen/cleanup failure".into());
                }
                total_cpu += cpu;
                total_wall += wall;
                total_body += body;
                session_metrics.push(json!({"cpu_us":cpu,"wall_us":wall,"body_bytes":body,"first_use_cpu_us":first_cpu,"first_use_wall_us":first_wall,"first_use_body_bytes":first_body,
                    "later_nine_cpu_us":cpu-first_cpu,"later_nine_wall_us":wall-first_wall,"later_nine_body_bytes":body-first_body}));
            }
            for (position, group) in groups.iter().enumerate() {
                let mut result = describe(group)?;
                result["arm"] = json!(arm);
                result["large"] = json!(large);
                result["position"] = json!(position);
                result["family"] = json!(SEQUENCE[position]);
                positions.push(result);
            }
            for family in FAMILIES {
                let group: Vec<_> = groups
                    .iter()
                    .flatten()
                    .copied()
                    .filter(|r| r["family"] == family)
                    .collect();
                let mut result = describe(&group)?;
                result["arm"] = json!(arm);
                result["large"] = json!(large);
                result["family"] = json!(family);
                cells.push(result);
            }
            let mut result = json!({"arm":arm,"large":large,"actual_targets_per_session":10,"includes_seed_and_preparation":true,"fit_excluded_here":true});
            for key in [
                "cpu_us",
                "wall_us",
                "body_bytes",
                "first_use_cpu_us",
                "first_use_wall_us",
                "first_use_body_bytes",
                "later_nine_cpu_us",
                "later_nine_wall_us",
                "later_nine_body_bytes",
            ] {
                result[key] = dist(
                    session_metrics
                        .iter()
                        .map(|r| numeric(r, key))
                        .collect::<Result<Vec<_>>>()?,
                );
            }
            session_cells.push(result);
        }
        totals.push(json!({"arm":arm,"actual_target_uses":200,"actual_sessions":20,"fit_cpu_us":fit_cpu,"fit_wall_us":fit_wall,
            "body_bytes":total_body,"cpu_us":total_cpu,"wall_us":total_wall,"body_bytes_per_target":total_body/200.0,"cpu_us_per_target":total_cpu/200.0,"wall_us_per_target":total_wall/200.0,
            "fit_charged_once_per_independently_compared_arm":true}));
    }
    Ok(
        json!({"schema":"spiralcore.schema-reuse.summary.v1","verdict":"EXACT_PERSISTENT_PAIRED_CAMPAIGN_PASS","target_operations":1600,"seed_operations":120,"preparations":40,
        "immediate_exact":1720,"reopened_exact":1720,"sessions":160,"discarded_observations":0,"fits":rows.iter().filter(|r|r["event"]=="fit").collect::<Vec<_>>(),
        "preparation_observations":rows.iter().filter(|r|r["event"]=="preparation").collect::<Vec<_>>(),"cells":cells,"positions":positions,"sessions_including_actual_seed_preparation":session_cells,"arm_totals":totals}),
    )
}

pub(super) fn command(args: &[String]) -> Result<()> {
    match args.get(1).map(String::as_str) {
        Some("reuse-plan") if args.len()==3=>write_row(&mut fresh(Path::new(&args[2]))?,&reuse_plan()?),
        Some("reuse-run") if args.len()==7=>run_reuse(Path::new(&args[2]),&args[3],Path::new(&args[4]),Path::new(&args[5]),Path::new(&args[6])),
        Some("reuse-summarize") if args.len()==4=>{let raw=fs::read(&args[2])?;let rows=std::str::from_utf8(&raw)?.lines().map(serde_json::from_str::<Value>).collect::<std::result::Result<Vec<_>,_>>()?;
            let mut summary=reuse_summary(&rows)?;summary["raw_sha256"]=json!(sha(&raw));write_row(&mut fresh(Path::new(&args[3]))?,&summary)},
        _=>Err("usage: schema_fitting reuse-plan NEW_PLAN | reuse-run PLAN SOURCE_COMMIT NEW_JSONL NEW_CACHE NEW_ARTIFACT_DIR | reuse-summarize JSONL NEW_SUMMARY".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_assign_distinct_held_out_indexes_and_reject_bad_bounds() {
        let mut indexes = std::collections::BTreeSet::new();
        for trial in 0..10 {
            for position in 0..10 {
                assert!(indexes.insert(session_index(trial, position).unwrap()));
            }
        }
        assert_eq!(indexes, (0..100).collect());
        assert!(session_index(10, 0).is_err());
        assert!(session_index(0, 10).is_err());
    }

    #[test]
    fn randomized_orders_preserve_all_arms_and_are_repeatable() {
        let mut orders = std::collections::BTreeSet::new();
        for trial in 0..10 {
            for large in [false, true] {
                let order = ordered_arms(trial, large);
                assert_eq!(order, ordered_arms(trial, large));
                assert_eq!(order.len(), 8);
                let arms: std::collections::BTreeSet<_> = order.iter().copied().collect();
                assert_eq!(arms.len(), 8);
                assert!(
                    arms.contains("raw")
                        && arms.contains("schema_zstd_prepared")
                        && arms.contains("dictionary_prepared")
                );
                orders.insert(order);
            }
        }
        assert!(orders.len() > 1);
    }

    #[test]
    fn historical_fixture_identities_are_unchanged() {
        let prior: Value = serde_json::from_slice(include_bytes!(
            "../../../../docs/network/evidence/2026-10-30-schema-fitting/plan.json"
        ))
        .unwrap();
        assert_eq!(sha(&training_records().concat()), prior["training_sha256"]);
        for row in prior["targets"].as_array().unwrap() {
            let target = fixture(
                row["family"].as_str().unwrap(),
                row["large"].as_bool().unwrap(),
                row["trial"].as_u64().unwrap() as usize,
            )
            .unwrap();
            assert_eq!(sha(&target.bytes), row["sha256"]);
            assert_eq!(hex(&object_id(&target.bytes)), row["object"]);
            assert_eq!(target.bytes.len(), row["bytes"]);
        }
        let plan = reuse_plan().unwrap();
        assert_eq!(plan["targets"].as_array().unwrap().len(), 200);
        assert_eq!(plan["target_operations"], 1600);
        assert_eq!(plan["seed_operations"], 120);
    }

    #[test]
    fn observation_validation_rejects_false_fidelity_and_double_counting() {
        let row = json!({"object":"target","bytes":100,"exact_immediate":true,"error":null,
            "body_bytes":70,"request_body_bytes":50,"response_body_bytes":20,
            "cpu_us":10,"exchange_cpu_us":7,"receiver_cpu_us":5,"sender_cpu_us":3,
            "receiver_cache_enabled":false,"cache_before":{"hits":0,"misses":0,"preparations":0,"evictions":0},
            "cache_after":{"hits":0,"misses":0,"preparations":0,"evictions":0,"resident_bytes":0,"entries":0}});
        validate_operation(&row, &row).unwrap();
        for (key, bad) in [
            ("object", json!("wrong")),
            ("exact_immediate", json!(false)),
            ("body_bytes", json!(69)),
            ("sender_cpu_us", json!(10)),
            ("receiver_cpu_us", json!(8)),
        ] {
            let mut poisoned = row.clone();
            poisoned[key] = bad;
            assert!(validate_operation(&poisoned, &row).is_err());
        }
        let mut retained = row.clone();
        retained["cache_after"]["resident_bytes"] = json!(1);
        assert!(validate_operation(&retained, &row).is_err());
        assert!(reuse_summary(&[]).is_err());
        let duplicated = json!({"event":"cleanup","arm":"raw","trial":0,"large":false});
        assert!(one_row(
            &[duplicated.clone(), duplicated],
            "cleanup",
            "raw",
            0,
            false,
            None
        )
        .is_err());
    }

    #[test]
    fn real_session_owns_private_store_and_reopens_every_distinct_target() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "spiralcore-reuse-test-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let training = training_records();
        let artifact =
            schema::fit(&training.iter().map(Vec::as_slice).collect::<Vec<_>>()).unwrap();
        let output = root.join("rows.jsonl");
        let mut out = fresh(&output).unwrap();
        launch_session(&mut out, &root, "schema", 0, false, Some(&artifact)).unwrap();
        drop(out);
        let rows = fs::read_to_string(&output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            rows.iter()
                .filter(|r| r["event"] == "operation" && r["exact_immediate"] == true)
                .count(),
            11
        );
        let verified = rows
            .iter()
            .find(|r| r["event"] == "session_verified")
            .unwrap();
        assert!(verified["error"].is_null());
        assert_eq!(verified["reopened_objects"].as_array().unwrap().len(), 11);
        assert_eq!(rows.last().unwrap()["owned_store_removed"], true);
        assert!(!root.join("0-false-schema").exists());
        fs::remove_dir_all(&root).unwrap();
    }
}
