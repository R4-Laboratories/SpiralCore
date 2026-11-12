//! One synthetic B-addressed operation through two original persistent native guests.
mod enrollment;
#[cfg(all(target_os = "linux", feature = "installed-object-pair"))]
mod installed_object;
mod meter;
mod native_backend;
mod object_enrollment;
mod object_repeat;
mod receipt_retry;
mod series;
mod transfer;
mod workload;
use anyhow::{Context, Result, ensure};
use enrollment::Enrollment;
use mvm_client::launch::persistent::journal::InstalledJournal;
use mvm_client::{LaunchRequest, LifecycleMode, LocalBackend, MvmClient, RootfsSource};
use mvm_contract::protocol::mailbox::{
    DeliveryOutcome, MailboxEnvelope, MailboxJournalDirectionV1, SignedDestinationReceipt,
};
use mvm_core::mailbox::{
    frame::Refusal, outbound::journal_direction_commitment, reader::MailboxReaderBootBinding,
};
use mvm_hostd::mailbox_process::original_journal::{
    OriginalJournalDestination, relay::OriginalJournalSourceHost,
};
use serde_json::{Value, json};
use std::{
    env, fs,
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub const A: &str = "sc-journal-a";
pub const B: &str = "sc-journal-b";
#[path = "../../paired-guest/src/fixture.rs"]
mod fixture;
#[cfg(not(feature = "large-object-fixture"))]
const BODY: &[u8; fixture::BODY_LEN] = include_bytes!("../../fixtures/workload-body.bin");
#[cfg(feature = "large-object-fixture")]
const BODY: &[u8; fixture::BODY_LEN] = include_bytes!("../../fixtures/workload-4k-body.bin");
type BHandle = Arc<Mutex<OriginalJournalDestination>>;
type Runtime = tokio::runtime::Runtime;

fn private_home() -> Result<PathBuf> {
    let p = PathBuf::from(env::var_os("MVM_HOME").context("fresh isolated MVM_HOME required")?);
    let m = fs::symlink_metadata(&p)?;
    ensure!(
        p.is_absolute()
            && p.canonicalize()? == p
            && m.is_dir()
            && m.mode() & 0o077 == 0
            && !p.join("vms").exists()
            && !mvm_core::config::machine_spec_path(A).exists()
            && !mvm_core::config::machine_spec_path(B).exists(),
        "expected empty private VM namespace"
    );
    ensure!(
        mvm_core::user_config::load_strict(None)?.max_mailbox_retention_ms == Some(30_000),
        "expected operator retention ceiling"
    );
    Ok(p)
}
fn verify_return(
    direction: &MailboxJournalDirectionV1,
    envelope: &MailboxEnvelope,
    raw: &[u8],
) -> Result<SignedDestinationReceipt> {
    let val =
        serde_json::from_slice::<std::result::Result<SignedDestinationReceipt, Refusal>>(raw)??;
    let r = &val.receipt;
    mvm_core::mailbox::verify_receipt(
        &val,
        &ed25519_dalek::VerifyingKey::from_bytes(&direction.receipt_key)?,
    )?;
    ensure!(
        r.outcome == DeliveryOutcome::AcceptedLocalDurable
            && r.address == direction.address
            && r.execution_epoch == direction.execution_epoch
            && r.delivery_id == envelope.routing.delivery_id
            && r.ciphertext_commitment == envelope.ciphertext_commitment
            && (direction.not_before_unix_ms..direction.expires_at_unix_ms)
                .contains(&r.observed_at_unix_ms),
        "returned receipt disagrees with original destination and sealed operation"
    );
    Ok(val)
}
fn image(path: &Path, digest: &str) -> Result<PathBuf> {
    let path = path.canonicalize()?;
    ensure!(
        mvm_core::crypto::image_verify::sha256_file(&path)? == digest,
        "role-specific image digest differs"
    );
    Ok(path)
}
#[derive(Default)]
struct State {
    #[cfg(all(target_os = "linux", feature = "installed-object-pair"))]
    objects: [Option<
        mvm_hostd::mailbox_process::original_journal::object_workers::OriginalObjectWorker,
    >; 2],
    a: Option<OriginalJournalSourceHost>,
    b: Option<BHandle>,
    b_pending: Option<OriginalJournalDestination>,
    defined: [bool; 2],
    attempted: [bool; 2],
}
impl State {
    fn retire(&mut self, backend: &LocalBackend, rt: &Runtime, home: &Path) -> Result<Value> {
        use mvm_hostd::mailbox_process::original_journal::JournalOwner;
        let mut journal = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(home.join("paired-retirement.jsonl"));
        let mut retired = Vec::new();
        let mut failure = Vec::new();
        let mut clean_definitions = [false; 2];
        let mut pending = self.defined;
        let mut first = [true; 2];
        while pending.iter().any(|v| *v) {
            for (i, name) in [A, B].into_iter().enumerate() {
                if !pending[i] {
                    continue;
                }
                let id = mvm_client::MachineId(name.to_owned());
                if !self.attempted[i] {
                    if let Err(e) = rt.block_on(backend.remove_machine(&id)) {
                        failure.push(format!("{name}: unused definition: {e}"));
                    }
                    pending[i] = false;
                    continue;
                }
                let until = Instant::now() + Duration::from_secs(20);
                #[cfg(all(target_os = "linux", feature = "installed-object-pair"))]
                let object = self.objects[i].as_mut().map(|worker| {
                    rt.block_on(backend.stop_persistent_journal(&id, &mut worker.owner(), until))
                });
                #[cfg(not(all(target_os = "linux", feature = "installed-object-pair")))]
                let object = None;
                let outcome = if object.is_some() {
                    object
                } else if i == 0 {
                    self.a.as_mut().map(|a| {
                        rt.block_on(backend.stop_persistent_journal(
                            &id,
                            &mut JournalOwner::Source(a),
                            until,
                        ))
                    })
                } else if let Some(b) = &self.b {
                    let mut held = b.lock().unwrap_or_else(|e| e.into_inner());
                    Some(rt.block_on(backend.stop_persistent_journal(
                        &id,
                        &mut JournalOwner::Destination(&mut held),
                        until,
                    )))
                } else {
                    self.b_pending.as_mut().map(|b| {
                        rt.block_on(backend.stop_persistent_journal(
                            &id,
                            &mut JournalOwner::Destination(b),
                            until,
                        ))
                    })
                };
                let row = match outcome {
                    None => {
                        pending[i] = false;
                        failure.push(format!("{name}: no returned original helper; preserve definition and inspect original startup rollback"));
                        json!({"name":name,"original_helper_available":false})
                    }
                    Some(Ok(result)) => {
                        pending[i] = false;
                        if !first[i] || result.assisted || !result.helper_status.success() {
                            failure.push(format!(
                                "{name}: assisted, delayed, or nonzero original exit"
                            ));
                        }
                        let row = json!({"name":name,"assisted":result.assisted,"success":result.helper_status.success(),"exit_code":result.helper_status.code(),"first_stop_attempt":first[i]});
                        retired.push(row.clone());
                        clean_definitions[i] = true;
                        row
                    }
                    Some(Err(e)) => {
                        // Keep both original handles. Let the other helper
                        // retire independently instead of blocking behind this one.
                        if !first[i] {
                            continue;
                        }
                        first[i] = false;
                        json!({"name":name,"original_stop_unconfirmed":e.to_string()})
                    }
                };
                if let Ok(file) = journal.as_mut() {
                    let _ = writeln!(file, "{}", row);
                    let _ = file.sync_all();
                }
                eprintln!("{}", row);
            }
            if pending.iter().any(|v| *v) {
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        // A also owns an Arc to B. Drop only already-collected originals
        // before deleting their definitions via the normal SDK.
        #[cfg(all(target_os = "linux", feature = "installed-object-pair"))]
        for worker in &mut self.objects {
            drop(worker.take());
        }
        drop(self.a.take());
        drop(self.b_pending.take());
        drop(self.b.take());
        for (i, name) in [A, B].into_iter().enumerate() {
            if clean_definitions[i] {
                let id = mvm_client::MachineId(name.to_owned());
                if let Err(e) = rt.block_on(backend.remove_machine(&id)) {
                    failure.push(format!("{name}: definition cleanup: {e}"));
                }
            }
        }
        ensure!(
            failure.is_empty(),
            "paired retirement needs review: {}",
            failure.join(", ")
        );
        Ok(json!(retired))
    }
}
fn run(args: &[String]) -> Result<()> {
    let (requested, inputs) = match args {
        [flag, backend, rest @ ..] if flag == "--backend" => (Some(backend.as_str()), rest),
        _ => (None, args),
    };
    let (operations, inputs) = match inputs {
        [flag, total, rest @ ..] if flag == "--objects" => (Some(series::count(total)?), rest),
        _ => (None, inputs),
    };
    ensure!(
        operations.is_none() || !cfg!(feature = "receipt-loss-qualification"),
        "series and recovery are separate measurements"
    );
    let native = native_backend::select(env::consts::OS, env::consts::ARCH, requested)
        .map_err(anyhow::Error::msg)?;
    let [ar, ad, br, bd, frame] = inputs else {
        anyhow::bail!("expected A ext4 + digest, B ext4 + digest, and fixture frame")
    };
    let home = private_home()?;
    let ai = image(Path::new(ar), ad)?;
    let bi = image(Path::new(br), bd)?;
    ensure!(
        ad != bd && ai != bi,
        "paired guest images must identify distinct workloads"
    );
    ensure!(
        fs::metadata(frame)?.len() == fixture::FRAME_LEN as u64
            && mvm_core::crypto::image_verify::sha256_file(Path::new(frame))? == fixture::FRAME_SHA,
        "incorrect fixture frame"
    );
    let raw = fs::read(frame)?;
    let report = home.join("paired-result.json");
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(report)?;
    let backend = LocalBackend::with_hypervisor(native);
    let mut observed = [None; 2];
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let begin = Instant::now();
    let cpu = cpu_time::ProcessTime::now();
    let mut meters: [Option<meter::OriginalMeter>; 2] = [None, None];
    let mut state = State::default();
    let measured = (|| -> Result<Value> {
        let setup = Enrollment::new()?;
        for (i, name, path) in [(1, B, bi), (0, A, ai)] {
            let r =
                LaunchRequest::builder(LifecycleMode::Persistent, RootfsSource::LocalPath(path))
                    .name(name)
                    .profile("standard")
                    .backend(native)
                    .cpus(1)
                    .memory_mib(256)
                    .ttl_seconds(180)
                    .build()?;
            backend.create_from_request(&r)?;
            state.defined[i] = true;
        }
        state.attempted[1] = true;
        let bstart = rt.block_on(backend.start_persistent_with_journal(
            B,
            setup.destination,
            &mut InstalledJournal::Destination {
                admission: &setup.incoming,
                owner: &mut state.b_pending,
            },
        ))?;
        observed[1] = mvm_client::AnyBackend::started_vm_kind(B).map(|kind| kind.as_str());
        native_backend::check_observation(native, observed[1]).map_err(anyhow::Error::msg)?;
        meters[1] = Some(meter::OriginalMeter::open(B)?);
        let b = Arc::new(Mutex::new(
            state
                .b_pending
                .take()
                .context("destination not owned after start")?,
        ));
        state.b = Some(b.clone());
        let (grant, auth) = setup.bind_source(
            &bstart.admitted,
            b,
            Instant::now() + Duration::from_secs(90),
        )?;
        let direction = grant
            .outbound_journal_v1
            .context("missing outbound direction")?;
        state.attempted[0] = true;
        let astart = rt.block_on(backend.start_persistent_with_journal(
            A,
            grant,
            &mut InstalledJournal::Source {
                admission: &auth,
                owner: &mut state.a,
            },
        ))?;
        observed[0] = mvm_client::AnyBackend::started_vm_kind(A).map(|kind| kind.as_str());
        native_backend::check_observation(native, observed[0]).map_err(anyhow::Error::msg)?;
        meters[0] = Some(meter::OriginalMeter::open(A)?);
        for (name, original, admitted) in [
            (A, grant, &astart.admitted),
            (B, setup.destination, &bstart.admitted),
        ] {
            let posture = admitted
                .mailbox_posture()
                .context("missing production posture")?;
            ensure!(
                format!("{:?}", posture.variant()) == "Prod"
                    && admitted.plan().mailbox == Some(original),
                "initial grant differs"
            );
            posture.verify(admitted.plan(), name)?;
        }
        let boot = MailboxReaderBootBinding::load(&mvm_core::config::vm_state_dir(A))?;
        boot.check_plan(astart.admitted.plan())?;
        let commitment = journal_direction_commitment(&boot)?;
        let setup_ns = begin.elapsed().as_nanos() as u64;
        let helper = state.a.as_mut().context("source helper missing")?;
        let mut result = if let Some(count) = operations {
            let start = Instant::now();
            let mut rows = Vec::with_capacity(count);
            let mut progress = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(home.join("paired-series.jsonl"))?;
            let mut ids = std::collections::BTreeSet::new();
            for ordinal in 1..=count {
                let row = transfer::one(&raw, &direction, commitment, helper, Some(ordinal))
                    .with_context(|| format!("data operation {ordinal}/{count}"))?;
                series::record(&mut ids, &row)?;
                serde_json::to_writer(&mut progress, &row)?;
                progress.write_all(b"\n")?;
                progress.sync_data()?;
                rows.push(row);
            }
            json!({"completed_objects":rows.len(),"unique_message_ids":ids.len(),
                "series_elapsed_ns":start.elapsed().as_nanos() as u64,"transfers":rows})
        } else {
            transfer::one(&raw, &direction, commitment, helper, None)?
        };
        result["setup_ns"] = json!(setup_ns);
        result["source_plan"] = json!(astart.admitted.plan().plan_id);
        result["destination_plan"] = json!(bstart.admitted.plan().plan_id);
        Ok(result)
    })();
    let recorded = (|| -> Result<()> {
        let mut first = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(home.join("paired-transfer-before-stop.json"))?;
        serde_json::to_writer(
            &mut first,
            &json!({"result":measured.as_ref().ok(), "error":measured.as_ref().err().map(|e|format!("{e:#}"))}),
        )?;
        first.sync_all()?;
        Ok(())
    })();
    let exit = state.retire(&backend, &rt, &home);
    let mut streams = [Value::Null, Value::Null];
    for (i, meter) in meters.into_iter().enumerate() {
        if let Some(meter) = meter {
            streams[i] = meter.finish(&home.join(format!("original-{}.counts", [A, B][i])))?;
        }
    }
    let row = json!({"native_host":{"os":env::consts::OS,"architecture":env::consts::ARCH,
        "requested_backend":native,"observed_source_backend":observed[0],"observed_destination_backend":observed[1]},
        "original_5257_helper_streams_a_b":streams,"result":measured.as_ref().ok(),"transfer_error":measured.as_ref().err().map(|e|format!("{e:#}")),
        "original_exits":exit.as_ref().ok(),"teardown_error":exit.as_ref().err().map(|e|format!("{e:#}")),"driver_process_cpu_ns":cpu.elapsed().as_nanos() as u64,
        "whole_wall_ns":begin.elapsed().as_nanos() as u64});
    serde_json::to_writer(&mut output, &row)?;
    output.write_all(b"\n")?;
    output.sync_all()?;
    exit?;
    recorded?;
    measured?;
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    if let [cmd, name] = args.as_slice()
        && cmd == "call"
    {
        return workload::dispatch(name);
    }
    #[cfg(all(target_os = "linux", feature = "installed-object-pair"))]
    if let [cmd, images @ ..] = args.as_slice()
        && cmd == "installed-object-repeat"
    {
        return installed_object::run(images);
    }
    if args.as_slice() == ["object-enrollment-check"] {
        println!(
            "{}",
            object_enrollment::ObjectEnrollment::issue()?.summary()?
        );
        return Ok(());
    }
    if let [cmd, pair] = args.as_slice()
        && cmd == "object-repeat"
    {
        return object_repeat::run(pair);
    }
    run(&args)
}
