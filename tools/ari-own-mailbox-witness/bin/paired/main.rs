//! One synthetic B-addressed operation through two original persistent native guests.
mod enrollment;
#[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
mod installed_collector;
#[cfg(all(target_os = "linux", feature = "installed-object-pair"))]
mod installed_object;
mod meter;
mod native_backend;
mod object_enrollment;
mod object_repeat;
mod receipt_retry;
#[cfg(feature = "sender-ordinal-v1")]
mod sender_ordinal;
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
    let (collector_mode, args) = match args {
        [flag, rest @ ..] if flag == "--installed-collector" => (true, rest),
        _ => (false, args),
    };
    ensure!(
        !collector_mode
            || cfg!(all(
                target_os = "linux",
                feature = "installed-collector-pair"
            )),
        "installed collector mode requires its Linux feature"
    );
    let (no_helper_callback_retry, args) = match args {
        [flag, rest @ ..] if flag == "--no-helper-callback-retry" => (true, rest),
        _ => (false, args),
    };
    ensure!(
        !no_helper_callback_retry || collector_mode,
        "no-helper callback retry requires installed collector qualification"
    );
    let (requested, inputs) = match args {
        [flag, backend, rest @ ..] if flag == "--backend" => (Some(backend.as_str()), rest),
        _ => (None, args),
    };
    let (restart_helpers, inputs) = match inputs {
        [flag, rest @ ..] if flag == "--restart-helpers" => (true, rest),
        _ => (false, inputs),
    };
    #[cfg(feature = "sender-ordinal-v1")]
    let (ordinal_count, inputs) = match inputs {
        [flag, total, rest @ ..] if flag == "--sender-ordinals" => {
            let count: usize = total.parse()?;
            ensure!(
                (1..=2049).contains(&count) && !cfg!(feature = "receipt-loss-qualification"),
                "invalid ordinal campaign"
            );
            (Some(count), rest)
        }
        _ => (None, inputs),
    };
    #[cfg(not(feature = "sender-ordinal-v1"))]
    let ordinal_count: Option<usize> = None;
    ensure!(
        !restart_helpers || (cfg!(target_os = "linux") && ordinal_count == Some(1)),
        "controlled helper recovery requires one Linux sender ordinal"
    );
    let (operations, inputs) = match inputs {
        [flag, total, rest @ ..] if flag == "--objects" => (Some(series::count(total)?), rest),
        _ => (None, inputs),
    };
    ensure!(
        operations.is_none()
            || (ordinal_count.is_none() && !cfg!(feature = "receipt-loss-qualification")),
        "series and recovery are separate measurements"
    );
    ensure!(
        !collector_mode
            || (!restart_helpers
                && ordinal_count.is_none()
                && operations.is_none()
                && !cfg!(feature = "receipt-loss-qualification")
                && !cfg!(feature = "large-object-fixture")
                && !cfg!(feature = "sender-ordinal-v1")
                && !cfg!(feature = "installed-object-pair")),
        "installed collector requires the clean 257-byte profile"
    );
    let native = native_backend::select(env::consts::OS, env::consts::ARCH, requested)
        .map_err(anyhow::Error::msg)?;
    ensure!(
        !collector_mode || native == "firecracker",
        "collector requires Firecracker"
    );
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
    let begin = Instant::now();
    let cpu = cpu_time::ProcessTime::now();
    #[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
    let mut collector = if collector_mode {
        Some(installed_collector::operator(&home)?)
    } else {
        None
    };
    #[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
    if no_helper_callback_retry {
        collector
            .as_mut()
            .context("no-helper callback retry requires original collector")?
            .0
            .enable_no_helper_callback_retry()?;
    }
    #[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
    let backend = collector
        .as_ref()
        .map(|(pair, _)| Arc::clone(pair.backend()))
        .unwrap_or_else(|| Arc::new(LocalBackend::with_hypervisor(native)));
    #[cfg(not(all(target_os = "linux", feature = "installed-collector-pair")))]
    let backend = Arc::new(LocalBackend::with_hypervisor(native));
    #[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
    let collector_before = collector
        .as_ref()
        .map(|(pair, _)| pair.resources())
        .transpose()?;
    #[cfg(not(all(target_os = "linux", feature = "installed-collector-pair")))]
    let collector_before: Option<Value> = None;
    let mut observed = [None; 2];
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut meters: [Option<meter::OriginalMeter>; 2] = [None, None];
    let mut state = State::default();
    let measured = (|| -> Result<Value> {
        let setup = Enrollment::new()?;
        #[cfg(feature = "sender-ordinal-v1")]
        let setup = {
            let mut s = setup;
            s.sender_ordinals = ordinal_count.is_some();
            s
        };
        for (i, name, path) in [(1, B, bi.clone()), (0, A, ai.clone())] {
            let r =
                LaunchRequest::builder(LifecycleMode::Persistent, RootfsSource::LocalPath(path))
                    .name(name)
                    .profile("standard")
                    .backend(native)
                    .cpus(1)
                    .memory_mib(256)
                    .ttl_seconds(
                        ordinal_count.map_or(180, |n| (n as u64 * 2 + 60).clamp(180, 3600)),
                    )
                    .build()?;
            backend.create_from_request(&r)?;
            state.defined[i] = true;
        }
        state.attempted[1] = true;
        let mut destination = InstalledJournal::Destination {
            admission: &setup.incoming,
            owner: &mut state.b_pending,
        };
        #[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
        let bstart = if let Some((pair, operator)) = &mut collector {
            rt.block_on(pair.start(
                B,
                operator.quote(B)?,
                &bi,
                setup.destination,
                &mut destination,
            ))?;
            None
        } else {
            Some(rt.block_on(backend.start_persistent_with_journal(
                B,
                setup.destination,
                &mut destination,
            ))?)
        };
        #[cfg(not(all(target_os = "linux", feature = "installed-collector-pair")))]
        let bstart = Some(rt.block_on(backend.start_persistent_with_journal(
            B,
            setup.destination,
            &mut destination,
        ))?);
        observed[1] = mvm_client::AnyBackend::started_vm_kind(B).map(|kind| kind.as_str());
        native_backend::check_observation(native, observed[1]).map_err(anyhow::Error::msg)?;
        #[cfg(feature = "sender-ordinal-v1")]
        let observed_meter = if restart_helpers {
            meter::OriginalMeter::open_predecessor(B)?
        } else if ordinal_count.is_some() {
            meter::OriginalMeter::open_ordinal(B)?
        } else {
            meter::OriginalMeter::open(B)?
        };
        #[cfg(not(feature = "sender-ordinal-v1"))]
        let observed_meter = meter::OriginalMeter::open(B)?;
        meters[1] = Some(observed_meter);
        let b = Arc::new(Mutex::new(
            state
                .b_pending
                .take()
                .context("destination not owned after start")?,
        ));
        state.b = Some(b.clone());
        #[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
        let destination_admitted = if let Some((pair, _)) = &collector {
            &pair.machine(B)?.admitted
        } else {
            &bstart
                .as_ref()
                .context("ordinary destination start missing")?
                .admitted
        };
        #[cfg(not(all(target_os = "linux", feature = "installed-collector-pair")))]
        let destination_admitted = &bstart
            .as_ref()
            .context("destination start missing")?
            .admitted;
        let (grant, auth) = setup
            .bind_source(
                destination_admitted,
                b,
                Instant::now() + Duration::from_secs(90),
            )
            .context(
                "bind source enrollment after original destination startup and meter acquisition",
            )?;
        let direction = grant
            .outbound_journal_v1
            .context("missing outbound direction")?;
        state.attempted[0] = true;
        let mut source = InstalledJournal::Source {
            admission: &auth,
            owner: &mut state.a,
        };
        #[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
        let astart = if let Some((pair, operator)) = &mut collector {
            rt.block_on(pair.start(A, operator.quote(A)?, &ai, grant, &mut source))?;
            None
        } else {
            Some(rt.block_on(backend.start_persistent_with_journal(A, grant, &mut source))?)
        };
        #[cfg(not(all(target_os = "linux", feature = "installed-collector-pair")))]
        let astart =
            Some(rt.block_on(backend.start_persistent_with_journal(A, grant, &mut source))?);
        observed[0] = mvm_client::AnyBackend::started_vm_kind(A).map(|kind| kind.as_str());
        native_backend::check_observation(native, observed[0]).map_err(anyhow::Error::msg)?;
        #[cfg(feature = "sender-ordinal-v1")]
        let observed_meter = if restart_helpers {
            meter::OriginalMeter::open_predecessor(A)?
        } else if ordinal_count.is_some() {
            meter::OriginalMeter::open_ordinal(A)?
        } else {
            meter::OriginalMeter::open(A)?
        };
        #[cfg(not(feature = "sender-ordinal-v1"))]
        let observed_meter = meter::OriginalMeter::open(A)?;
        meters[0] = Some(observed_meter);
        #[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
        let admissions = if let Some((pair, _)) = &collector {
            [&pair.machine(A)?.admitted, &pair.machine(B)?.admitted]
        } else {
            [
                &astart.as_ref().context("source start missing")?.admitted,
                &bstart
                    .as_ref()
                    .context("destination start missing")?
                    .admitted,
            ]
        };
        #[cfg(not(all(target_os = "linux", feature = "installed-collector-pair")))]
        let admissions = [
            &astart.as_ref().context("source start missing")?.admitted,
            &bstart
                .as_ref()
                .context("destination start missing")?
                .admitted,
        ];
        for (name, original, admitted) in [
            (A, grant, admissions[0]),
            (B, setup.destination, admissions[1]),
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
        boot.check_plan(admissions[0].plan())?;
        let source_plan = admissions[0].plan().plan_id.clone();
        let destination_plan = admissions[1].plan().plan_id.clone();
        let commitment = journal_direction_commitment(&boot)?;
        let setup_ns = begin.elapsed().as_nanos() as u64;
        let helper = state.a.as_mut().context("source helper missing")?;
        #[cfg(feature = "sender-ordinal-v1")]
        let ordinal_result = ordinal_count
            .map(|count| {
                sender_ordinal::campaign(
                    count,
                    &home,
                    &direction,
                    commitment,
                    helper,
                    restart_helpers,
                )
            })
            .transpose()?;
        #[cfg(not(feature = "sender-ordinal-v1"))]
        let ordinal_result: Option<Value> = None;
        let mut result = if let Some(row) = ordinal_result {
            row
        } else if let Some(count) = operations {
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
            #[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
            let row = if let Some((pair, _)) = &mut collector {
                transfer::one_with_calls(&raw, &direction, commitment, helper, None, pair)?
            } else {
                transfer::one(&raw, &direction, commitment, helper, None)?
            };
            #[cfg(not(all(target_os = "linux", feature = "installed-collector-pair")))]
            let row = transfer::one(&raw, &direction, commitment, helper, None)?;
            row
        };
        result["setup_ns"] = json!(setup_ns);
        result["source_plan"] = json!(source_plan);
        result["destination_plan"] = json!(destination_plan);
        result["call_transport"] = json!(if collector_mode {
            "original_owned_journal_control"
        } else {
            "ordinary_sdk_process"
        });
        result["driver_cpu_domain"] = json!(if collector_mode {
            "controller process includes SDK call/handshake work; excludes guest, mailbox children and other processes"
        } else {
            "controller process excludes SDK call subprocess CPU"
        });
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
    // Acquire originals before typed SDK stop removes each VM state directory.
    // If a capture fails, still complete retirement using the retained owners.
    let successors = restart_helpers.then(|| [A, B].map(meter::OriginalMeter::open_replacement));
    #[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
    let collector_before_stop = collector.as_ref().map(|(pair, _)| pair.resources());
    #[cfg(not(all(target_os = "linux", feature = "installed-collector-pair")))]
    let collector_before_stop: Option<Result<Value>> = None;
    #[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
    let exit = if let Some((pair, _)) = &mut collector {
        drop(backend);
        installed_collector::retire(pair, &mut state, &home)
    } else {
        state.retire(&backend, &rt, &home)
    };
    #[cfg(not(all(target_os = "linux", feature = "installed-collector-pair")))]
    let exit = state.retire(&backend, &rt, &home);
    #[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
    let collector_after = collector.as_ref().map(|(pair, _)| pair.resources());
    #[cfg(not(all(target_os = "linux", feature = "installed-collector-pair")))]
    let collector_after: Option<Result<Value>> = None;
    if collector_mode && exit.is_err() {
        // The original journal handles accompany the retained SDK/parent owners.
        // Failed retirement never falls through to ordinary cleanup on Drop.
        std::mem::forget(std::mem::take(&mut state));
    }
    let mut streams = [Value::Null, Value::Null];
    let mut meter_errors = Vec::new();
    for (i, meter) in meters.into_iter().enumerate() {
        if let Some(meter) = meter {
            match meter.finish(&home.join(format!("original-{}.counts", [A, B][i]))) {
                Ok(row) => streams[i] = row,
                Err(error) => meter_errors.push(format!("{}: {error:#}", [A, B][i])),
            }
        }
    }
    let mut successor_streams = [Value::Null, Value::Null];
    if let Some(held) = successors {
        for (i, meter) in held.into_iter().enumerate() {
            if let Ok(meter) = meter {
                successor_streams[i] =
                    meter.finish(&home.join(format!("replacement-{}.counts", [A, B][i])))?;
            }
        }
    }
    let meters_complete = if restart_helpers {
        recovery_meters(&streams, &successor_streams)
    } else {
        (ordinal_count.is_none() && !collector_mode)
            || streams.iter().all(|s| {
                s["capture"] == "closed_session"
                    && s["completed_exchanges"].as_u64().is_some_and(|n| n > 0)
                    && s["guest_to_host_read_bytes"]
                        .as_u64()
                        .is_some_and(|n| n > 0)
                    && s["host_to_guest_write_accepted_bytes"]
                        .as_u64()
                        .is_some_and(|n| n > 0)
            })
    };
    #[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
    let collector_limits = Some((
        installed_collector::START_MEMORY_BYTES,
        installed_collector::START_DISK_BYTES,
        installed_collector::HOST_RESIDENT_BYTES,
        installed_collector::TENANT_PROCESS_AS,
    ));
    #[cfg(not(all(target_os = "linux", feature = "installed-collector-pair")))]
    let collector_limits: Option<(u64, u64, u64, u64)> = None;
    let row = json!({"native_host":{"os":env::consts::OS,"architecture":env::consts::ARCH,
    "requested_backend":native,"observed_source_backend":observed[0],"observed_destination_backend":observed[1]},
    "original_5257_helper_streams_a_b":streams,"replacement_5257_helper_streams_a_b":restart_helpers.then_some(successor_streams),"result":measured.as_ref().ok(),"transfer_error":measured.as_ref().err().map(|e|format!("{e:#}")),
        "original_exits":exit.as_ref().ok(),"teardown_error":exit.as_ref().err().map(|e|format!("{e:#}")),"meter_errors":meter_errors,"driver_process_cpu_ns":cpu.elapsed().as_nanos() as u64,
    "whole_wall_ns":begin.elapsed().as_nanos() as u64,
    "installed_collector":collector_mode.then_some(collector_limits).flatten().map(|(memory,disk,resident,tenant)|json!({
        "per_start_memory_ceiling_bytes":memory,"per_start_disk_ceiling_bytes":disk,
        "host_resident_ceiling_bytes":resident,"tenant_process_address_space_bytes":tenant,
        "requested_guest_memory_bytes":512u64<<20,
        "initial_resources":collector_before,
        "before_stop_resources":collector_before_stop.as_ref().and_then(|r|r.as_ref().ok()),
        "before_stop_resource_error":collector_before_stop.as_ref().and_then(|r|r.as_ref().err()).map(|e|format!("{e:#}")),
        "after_stop_resources":collector_after.as_ref().and_then(|r|r.as_ref().ok()),
        "after_stop_resource_error":collector_after.as_ref().and_then(|r|r.as_ref().err()).map(|e|format!("{e:#}")),
        "runtime_state_removed":false,"wan":null
    }))});
    serde_json::to_writer(&mut output, &row)?;
    output.write_all(b"\n")?;
    output.sync_all()?;
    exit?;
    recorded?;
    measured?;
    ensure!(
        meter_errors.is_empty(),
        "original byte meter finalization failed"
    );
    ensure!(
        meters_complete,
        "incomplete original sender-ordinal guest stream measurements"
    );
    Ok(())
}
fn recovery_meters(originals: &[Value; 2], replacements: &[Value; 2]) -> bool {
    [(originals, "prefix_only"), (replacements, "closed_session")]
        .into_iter()
        .all(|(meters, expected)| {
            meters.iter().all(|s| {
                s["capture"] == expected
                    && [
                        "completed_exchanges",
                        "guest_to_host_read_bytes",
                        "host_to_guest_write_accepted_bytes",
                    ]
                    .iter()
                    .all(|k| s[k].as_u64().is_some_and(|n| n > 0))
            })
        })
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
        return installed_object::run(images, None, false);
    }
    #[cfg(all(target_os = "linux", feature = "installed-object-pair"))]
    if let [cmd, mode, images @ ..] = args.as_slice()
        && cmd == "installed-object-codec"
    {
        return installed_object::run(images, Some(object_repeat::Codec::parse(mode)?), false);
    }
    #[cfg(all(target_os = "linux", feature = "object-ack-loss-qualification"))]
    if let [cmd, images @ ..] = args.as_slice()
        && cmd == "installed-object-ack-loss"
    {
        return installed_object::run(images, Some(object_repeat::Codec::Raw), true);
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

#[cfg(test)]
mod tests {
    use super::run;

    #[cfg(all(target_os = "linux", feature = "installed-collector-pair"))]
    #[test]
    fn collector_rejects_series_before_opening_operator_inputs() {
        let args = ["--installed-collector", "--objects", "2"].map(str::to_owned);
        let error = run(&args).expect_err("collector series must refuse before setup");
        assert!(error.to_string().contains("clean 257-byte profile"));
    }

    #[cfg(not(all(target_os = "linux", feature = "installed-collector-pair")))]
    #[test]
    fn collector_requires_the_explicit_linux_feature_before_setup() {
        let args = ["--installed-collector"].map(str::to_owned);
        let error = run(&args).expect_err("unavailable collector must refuse before setup");
        assert!(error.to_string().contains("requires its Linux feature"));
    }

    #[test]
    fn helper_restart_requires_an_explicit_single_ordinal() {
        let args = vec!["--restart-helpers".to_owned()];
        let error = run(&args).expect_err("missing ordinal must refuse before setup");
        assert!(
            error
                .to_string()
                .contains("requires one Linux sender ordinal")
        );
    }

    #[cfg(feature = "sender-ordinal-v1")]
    #[test]
    fn helper_restart_refuses_a_campaign_before_preparing_guests() {
        let args = ["--restart-helpers", "--sender-ordinals", "2"].map(str::to_owned);
        let error = run(&args).expect_err("multi-ordinal restart must refuse before setup");
        assert!(
            error
                .to_string()
                .contains("requires one Linux sender ordinal")
        );
    }
}
