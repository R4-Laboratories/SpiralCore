//! One synthetic B-addressed operation through two original persistent Firecracker guests.
mod enrollment;
mod meter;
mod receipt_retry;
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
const FRAME_HASH: &str = "ff26a66df5b1d9de994e60824757686d2af8eb4433b91e74a2f24b279f600044";
const BODY: &[u8] = include_bytes!("../../fixtures/workload-body.bin");
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
struct State {
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
                let outcome = if i == 0 {
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
    let [ar, ad, br, bd, frame] = args else {
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
        fs::metadata(frame)?.len() == 313
            && mvm_core::crypto::image_verify::sha256_file(Path::new(frame))? == FRAME_HASH,
        "incorrect fixture frame"
    );
    let raw = fs::read(frame)?;
    let report = home.join("paired-result.json");
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(report)?;
    let backend = LocalBackend::with_hypervisor("firecracker");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let begin = Instant::now();
    let cpu = cpu_time::ProcessTime::now();
    let mut meters: [Option<meter::OriginalMeter>; 2] = [None, None];
    let mut state = State {
        a: None,
        b: None,
        b_pending: None,
        defined: [false; 2],
        attempted: [false; 2],
    };
    let measured = (|| -> Result<Value> {
        let setup = Enrollment::new()?;
        for (i, name, path) in [(1, B, bi), (0, A, ai)] {
            let r =
                LaunchRequest::builder(LifecycleMode::Persistent, RootfsSource::LocalPath(path))
                    .name(name)
                    .profile("standard")
                    .backend("firecracker")
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
        let transaction = Instant::now();
        let until = transaction + Duration::from_secs(50);
        let source_request = json!({"frame_hex":hex::encode(&raw),"recipient":hex::encode(direction.address.recipient.as_bytes()),"direction":hex::encode(commitment)});
        let mut source_call = workload::Call::start(A, &source_request)?;
        let mut staged = None;
        let mut relay = None;
        let mut first_receipt: Option<receipt_retry::FirstReceipt<MailboxEnvelope>> = None;
        let helper = state.a.as_mut().context("source helper missing")?;
        loop {
            if staged.is_none() {
                staged = source_call.completed()?
            }
            if relay.is_none() {
                let mut observed = None;
                let counts = helper.forward_once_observed(until, |env, bytes| {
                    observed = Some((
                        env.clone(),
                        bytes.to_vec(),
                        verify_return(&direction, env, bytes),
                    ))
                })?;
                if let Some(counts) = counts {
                    let (envelope, raw_receipt, checked) =
                        observed.context("missing original relay observation")?;
                    let receipt = checked?;
                    let at_ns = transaction.elapsed().as_nanos() as u64;
                    if cfg!(feature = "receipt-loss-qualification") {
                        if let Some(first) = &first_receipt {
                            first
                                .verify_retry(
                                    &envelope,
                                    &raw_receipt,
                                    counts.sealed_to_destination,
                                    counts.reply_to_source,
                                )
                                .map_err(anyhow::Error::msg)?;
                        } else {
                            eprintln!(
                                "first verified B receipt: {} bytes, forward {} bytes, returned {}",
                                raw_receipt.len(),
                                counts.sealed_to_destination,
                                counts.reply_to_source
                            );
                            first_receipt = Some(
                                receipt_retry::FirstReceipt::lost(
                                    envelope,
                                    raw_receipt,
                                    counts.sealed_to_destination,
                                    counts.reply_to_source,
                                    at_ns,
                                )
                                .map_err(anyhow::Error::msg)?,
                            );
                            continue;
                        }
                    }
                    relay = Some((counts, receipt, at_ns));
                }
            }
            if staged.is_some() && relay.is_some() {
                break;
            }
            ensure!(
                Instant::now() < until,
                "timed out awaiting original source Call or authenticated B verdict"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let source = staged.unwrap();
        let (counts, v2, verified_ns) = relay.unwrap();
        let id = hex::encode(v2.receipt.message_id.as_bytes());
        ensure!(
            source["role"] == "staged"
                && source["message_id"] == id
                && source["frame_bytes"] == 313,
            "B receipt differs from A original staged ID"
        );
        let mut bcall = workload::Call::start(B, &json!({"message_id":id}))?;
        let mut destination = loop {
            if let Some(v) = bcall.completed()? {
                break v;
            }
            ensure!(
                Instant::now() < until,
                "destination application Call incomplete"
            );
            std::thread::sleep(Duration::from_millis(5));
        };
        ensure!(
            destination["role"] == "received_and_acknowledged"
                && destination["message_id"] == id
                && destination["frame_bytes"] == 313
                && [
                    "receive_to_agent",
                    "receive_from_agent",
                    "ack_to_agent",
                    "ack_from_agent"
                ]
                .into_iter()
                .all(|k| destination[k].as_u64().is_some_and(|n| n > 0)),
            "missing destination application acceptance"
        );
        let recovery = if let Some(first) = first_receipt {
            ensure!(
                transaction.elapsed() < Duration::from_secs(30),
                "empty probe outside journal retention"
            );
            ensure!(
                source["outbox_retired_without_archive"] == true
                    && source["outbox_retirement_wait_ns"].is_u64()
                    && destination["after_ack"]["empty"] == true
                    && ["to_agent", "from_agent"]
                        .iter()
                        .all(|k| destination["after_ack"][*k].as_u64().is_some_and(|v| v > 0)),
                "guest did not confirm original outbox retirement and empty B queue"
            );
            json!({"first_b_receipt_ns":first.at_ns,"verified_retry_ns":verified_ns,
                "first_sealed_a_to_b":first.forward_bytes,"first_receipt_from_b":first.bytes.len(),
                "first_receipt_to_a":0,"retry_sealed_a_to_b":counts.sealed_to_destination,
                "retry_receipt_from_b":first.bytes.len(),"retry_receipt_to_a":counts.reply_to_source,
                "exact_envelope_equal":true,"exact_signed_receipt_bytes_equal":true})
        } else {
            Value::Null
        };
        let reply_hex = destination
            .as_object_mut()
            .context("expected guest record")?
            .remove("readback_hex")
            .context("missing independent readback")?;
        let bytes = hex::decode(reply_hex.as_str().context("bad guest readback")?)?;
        ensure!(
            bytes.as_slice() == BODY,
            "byte comparison failed after B workload ACK"
        );
        Ok(
            json!({"source_plan":astart.admitted.plan().plan_id,"destination_plan":bstart.admitted.plan().plan_id,"setup_ns":setup_ns,
            "guest_a":source,"guest_b":destination,"sealed_a_to_b_bytes":counts.sealed_to_destination,"signed_b_to_a_bytes":counts.reply_to_source,
            "sdk_call_application_io":{"source_request":source_call.input_bytes,"source_reply":source_call.output_bytes,"destination_request":bcall.input_bytes,"destination_reply_with_readback":bcall.output_bytes},
            "through_observed_v2_ns":verified_ns,"through_verified_destination_call_ns":transaction.elapsed().as_nanos() as u64,
            "receipt_recovery":recovery, "verified_body_bytes":bytes.len(),"receipt_outcome":"AcceptedLocalDurable","guest_a_observed_receipt":null,"wan_bytes":null}),
        )
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
    let row = json!({"original_5257_helper_streams_a_b":streams,"result":measured.as_ref().ok(),"transfer_error":measured.as_ref().err().map(|e|format!("{e:#}")),
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
    run(&args)
}
