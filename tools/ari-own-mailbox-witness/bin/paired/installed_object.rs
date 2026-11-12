//! One authorized object pilot. Both starts and both workers stay with the
//! orchestrator until the canonical two-sided data gate and owned retirement.
use super::*;
use meter::selected::Meters;
use object_enrollment::ObjectEnrollment;

fn event(file: &mut fs::File, row: &Value) -> Result<()> {
    serde_json::to_writer(&mut *file, row)?;
    file.write_all(b"\n")?;
    file.sync_data()?;
    Ok(())
}
fn report(home: &Path, name: &str) -> Result<fs::File> {
    Ok(fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(home.join(name))?)
}
fn runtime() -> Result<Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
}
fn digest(s: &str) -> Result<[u8; 32]> {
    let bytes = hex::decode(s)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("expected a SHA-256 digest"))?;
    ensure!(hex::encode(bytes) == s, "expected lowercase SHA-256");
    Ok(bytes)
}
fn closed(meters: &Value) -> Result<()> {
    for name in ["guest_5257", "sealed_in", "sealed_out"] {
        ensure!(
            meters[name]["capture"] == "closed_session",
            "missing finalized {name} stream"
        );
    }
    Ok(())
}
fn verify_links(a: &Value, b: &Value) -> Result<()> {
    closed(a)?;
    closed(b)?;
    positive_crossed_counts(a, b)
}

fn positive_crossed_counts(a: &Value, b: &Value) -> Result<()> {
    for role in [a, b] {
        for field in ["read", "write"] {
            ensure!(
                role["guest_5257"][field].as_u64().is_some_and(|n| n > 0),
                "required guest I/O absent"
            );
        }
    }
    for (source, destination) in [(a, b), (b, a)] {
        for (sent, accepted) in [("write", "read"), ("read", "write")] {
            let sent = source["sealed_out"][sent]
                .as_u64()
                .context("missing original sealed output count")?;
            ensure!(
                Some(sent) == destination["sealed_in"][accepted].as_u64() && sent > 0,
                "sealed channel byte counts differ or no exchange was observed"
            );
        }
    }
    Ok(())
}
const CHANNELS: [&str; 3] = ["guest_5257", "sealed_in", "sealed_out"];
const FIELDS: [&str; 5] = ["read", "write", "turns", "read_errors", "write_errors"];

fn boundary(meters: &[Option<Meters>; 2]) -> Result<[Value; 2]> {
    let read = |slot: &Option<Meters>| {
        slot.as_ref()
            .context("missing original meter")?
            .snapshot(None)
    };
    let until = Instant::now() + Duration::from_millis(350);
    loop {
        let snap = [read(&meters[0])?, read(&meters[1])?];
        if snap
            .iter()
            .all(|vm| CHANNELS.iter().all(|&ch| vm[ch]["read"].is_u64()))
            || Instant::now() >= until
        {
            return Ok(snap);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn phase_counts(before: &[Value; 2], after: &[Value; 2]) -> Result<[Value; 2]> {
    let mut delta = [json!({}), json!({})];
    for i in 0..2 {
        for ch in CHANNELS {
            for field in FIELDS {
                let current = after[i][ch][field]
                    .as_u64()
                    .context("incomplete final phase boundary")?;
                let previous = before[i][ch][field]
                    .as_u64()
                    .context("incomplete initial phase boundary")?;
                delta[i][ch][field] = json!(
                    current
                        .checked_sub(previous)
                        .context("original counter regressed")?
                );
            }
            ensure!(
                delta[i][ch]["read_errors"] == 0 && delta[i][ch]["write_errors"] == 0,
                "I/O errors during the measured phase"
            );
        }
    }
    positive_crossed_counts(&delta[0], &delta[1])?;
    Ok(delta)
}
fn start(
    name: &str,
    native: &'static str,
    hash: [u8; 32],
    nonce: &mvm_contract::plan::Nonce,
    endpoint: mvm_hostd::mailbox_process::original_journal::object_workers::OriginalObjectLaunch<
        '_,
    >,
    slot: &mut Option<
        mvm_hostd::mailbox_process::original_journal::object_workers::OriginalObjectWorker,
    >,
    attempted: &mut bool,
) -> Result<Value> {
    let backend = LocalBackend::with_hypervisor(native);
    let rt = runtime()?;
    *attempted = true;

    let granted = endpoint.plan().grant();
    let start = rt.block_on(backend.start_persistent_with_journal(
        name,
        granted,
        &mut InstalledJournal::Object {
            launch: &endpoint,
            owner: slot,
        },
    ))?;
    let plan = start.admitted.plan();
    ensure!(
        plan.image.sha256 == hex::encode(hash)
            && plan.mailbox == Some(granted)
            && &plan.nonce == nonce
            && plan.image.name == name,
        "admitted original differs from selected image/nonce/grant"
    );
    let posture = start
        .admitted
        .mailbox_posture()
        .context("missing original mailbox posture")?;
    posture.verify(plan, name)?;
    ensure!(
        matches!(posture.variant(), mvm_core::plan::Variant::Prod),
        "expected sealed production lifecycle"
    );
    let observed = mvm_client::AnyBackend::started_vm_kind(name);
    native_backend::check_observation(native, observed.as_ref().map(|k| k.as_str()))
        .map_err(anyhow::Error::msg)?;
    let boot = MailboxReaderBootBinding::load(&mvm_core::config::vm_state_dir(name))?;
    boot.check_plan(plan)?;
    Ok(
        json!({"name":name,"backend":native,"plan":plan.plan_id,"boot":hex::encode(boot.boot.as_bytes()),"image":plan.image.sha256}),
    )
}

pub fn run(args: &[String], codec: Option<object_repeat::Codec>) -> Result<()> {
    let [ap, ad, bp, bd] = args else {
        anyhow::bail!("expected original caller ext4 and SHA-256, then callee ext4 and SHA-256")
    };
    ensure!(
        !cfg!(feature = "receipt-loss-qualification"),
        "select the clean installed object profile"
    );
    let home = private_home()?;
    let images = [image(Path::new(ap), ad)?, image(Path::new(bp), bd)?];
    let hashes = [digest(ad)?, digest(bd)?];
    ensure!(
        images[0] != images[1] && hashes[0] != hashes[1],
        "original role images must differ"
    );
    let native = native_backend::select(env::consts::OS, env::consts::ARCH, Some("firecracker"))
        .map_err(anyhow::Error::msg)?;
    let backend = LocalBackend::with_hypervisor(native);
    let rt = runtime()?;
    let mut history = report(&home, "object-pair-progress.jsonl")?;
    let mut final_report = report(&home, "object-pair-result.json")?;
    let mut state = State::default();
    let mut meters: [Option<Meters>; 2] = [None, None];
    let begin = Instant::now();
    let cpu = cpu_time::ProcessTime::now();
    let measured = (|| -> Result<Value> {
        let selection = ObjectEnrollment::issue()?;
        let (ag, bg) = selection.prepared.selected_grants();
        let pair = ag
            .durable_object_pair_sha256_v1
            .context("missing selected pair")?;
        ensure!(bg.durable_object_pair_sha256_v1 == Some(pair));
        for (i, (name, path)) in [A, B].into_iter().zip(images).enumerate() {
            let req =
                LaunchRequest::builder(LifecycleMode::Persistent, RootfsSource::LocalPath(path))
                    .name(name)
                    .profile("standard")
                    .backend(native)
                    .cpus(1)
                    .memory_mib(256)
                    .ttl_seconds(180)
                    .build()?;
            backend.create_from_request(&req)?;
            state.defined[i] = true;
        }
        let (a, b) = selection.prepared.reserve_original_workers(
            A,
            hashes[0],
            B,
            hashes[1],
            begin + Duration::from_secs(90),
        )?;
        let [slot_a, slot_b] = &mut state.objects;
        let [attempt_a, attempt_b] = &mut state.attempted;
        let starts = std::thread::scope(|s| {
            let a_nonce = &selection.caller_nonce;
            let b_nonce = &selection.callee_nonce;
            let a = s.spawn(move || start(A, native, hashes[0], a_nonce, a, slot_a, attempt_a));
            let b = s.spawn(move || start(B, native, hashes[1], b_nonce, b, slot_b, attempt_b));
            [a.join(), b.join()].map(|result| {
                result
                    .map_err(|_| anyhow::anyhow!("original SDK start panicked"))
                    .and_then(|r| r)
            })
        });
        let [a, b] = starts;
        event(
            &mut history,
            &json!({"stage":"starts","source":a.as_ref().ok(),"destination":b.as_ref().ok(),
            "source_error":a.as_ref().err().map(|e|format!("{e:#}")),"destination_error":b.as_ref().err().map(|e|format!("{e:#}")),"elapsed_ns":begin.elapsed().as_nanos() as u64}),
        )?;
        a?;
        b?;
        ensure!(
            state.objects.iter().all(Option::is_some),
            "missing original worker after READY"
        );
        for (i, name) in [A, B].into_iter().enumerate() {
            meters[i] = Some(Meters::open(name)?);
        }
        let mut previous = boundary(&meters)?;
        let mut phases = serde_json::Map::new();
        event(
            &mut history,
            &json!({"stage":"before_cold","meters_a_b":previous}),
        )?;
        let mut result = object_repeat::measure_observed(
            &hex::encode(pair),
            codec,
            |phase, row| {
                let observed = boundary(&meters)?;
                let delta = phase_counts(&previous, &observed);
                event(
                    &mut history,
                    &json!({"stage":phase,"data":row,"meters_a_b":observed,"transport_delta":delta.as_ref().ok(),
                "transport_delta_error":delta.as_ref().err().map(|e|format!("{e:#}")),"elapsed_ns":begin.elapsed().as_nanos() as u64}),
                )?;
                // Preserve the actual cold/warm application gate even if a capture is incomplete.
                phases.insert(phase.into(),json!({"meters_a_b":delta.as_ref().ok(),"error":delta.err().map(|e|format!("{e:#}"))}));
                previous = observed;
                Ok(())
            },
        )?;
        result["transport_phases"] = Value::Object(phases);
        Ok(result)
    })();
    let pre_stop = event(
        &mut history,
        &json!({"stage":"before_stop","data":measured.as_ref().ok(),"error":measured.as_ref().err().map(|e|format!("{e:#}"))}),
    );
    let stopped = state.retire(&backend, &rt, &home);
    let captured = meters
        .iter()
        .zip([A, B])
        .map(|(m, name)| {
            m.as_ref()
                .context("unobserved original meter")?
                .snapshot(Some((&home, name)))
        })
        .collect::<Result<Vec<_>>>();
    let links = captured.as_ref().ok().map(|v| verify_links(&v[0], &v[1]));
    let row = json!({"schema":"spiralcore.installed-object-pair.v1","native_backend":native,"data":measured.as_ref().ok(),"data_error":measured.as_ref().err().map(|e|format!("{e:#}")),
        "retirement":stopped.as_ref().ok(),"retirement_error":stopped.as_ref().err().map(|e|format!("{e:#}")),"original_meters_a_b":captured.as_ref().ok(),
        "meter_error":captured.as_ref().err().map(|e|format!("{e:#}")),"link_error":links.as_ref().and_then(|r|r.as_ref().err()).map(|e|format!("{e:#}")),
        "whole_wall_ns":begin.elapsed().as_nanos() as u64,"driver_process_cpu_ns":cpu.elapsed().as_nanos() as u64,"wan":null});
    event(&mut final_report, &row)?;
    let data = measured?;
    for phase in ["cold", "warm"] {
        ensure!(
            data["transport_phases"][phase]["meters_a_b"].is_array(),
            "incomplete measured transport phase {phase}"
        );
    }
    pre_stop?;
    stopped?;
    captured?;
    links.context("unobserved original sealed channels")??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phase_attribution_requires_complete_monotonic_guest_and_sealed_io() {
        let fixture = |n| {
            let turn = json!({"turns":n,"read":n,"write":n,"read_errors":0,"write_errors":0});
            let role = json!({"guest_5257":turn,"sealed_in":turn,"sealed_out":turn});
            [role.clone(), role]
        };
        let before = fixture(1);
        let mut after = fixture(3);
        assert_eq!(
            phase_counts(&before, &after).unwrap()[1]["guest_5257"]["read"],
            2
        );
        after[1]["guest_5257"]["write"] = json!(1);
        assert!(phase_counts(&before, &after).is_err());
        let mut after = fixture(3);
        after[0]["sealed_out"]["write_errors"] = json!(1);
        assert!(phase_counts(&before, &after).is_err());
        after[0]["sealed_out"]["write_errors"] = Value::Null;
        assert!(phase_counts(&before, &after).is_err());
        assert!(phase_counts(&fixture(4), &fixture(3)).is_err());
    }
    #[test]
    fn counts_only_agree_for_two_closed_crossed_sealed_links() {
        let frame = |r, w| json!({"capture":"closed_session","read":r,"write":w});
        let a =
            json!({"guest_5257":frame(20,12),"sealed_in":frame(50,10),"sealed_out":frame(13,40)});
        let b =
            json!({"guest_5257":frame(17,19),"sealed_in":frame(40,13),"sealed_out":frame(10,50)});
        assert!(verify_links(&a, &b).is_ok());
        let mut b = b;
        b["sealed_in"]["capture"] = json!("complete_prefix");
        assert!(verify_links(&a, &b).is_err());
        b["sealed_in"]["capture"] = json!("closed_session");
        b["sealed_in"]["read"] = json!(39);
        assert!(verify_links(&a, &b).is_err());
    }
}
