//! Bounded matched object campaigns through canonical TCP-loopback mailboxes.
//! This is a host-process witness, not native MVM or WAN performance.
use cpu_time::{ProcessTime, ThreadTime};
use serde_json::{Value, json};
use spiralcore_mvm_adapter::live::HandshakeRole;
use spiralcore_mvm_live::{
    Result, campaign,
    config::{Arm, Pair, Plan, Published, Stage},
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    net::{TcpListener, TcpStream},
    path::Path,
    time::Instant,
};

const DISK_CEILING: u64 = 512 << 20;
const RSS_CEILING: u64 = 1 << 30;

pub fn validate(size: usize, trial: u64) -> Result<()> {
    if ![1 << 20, 16 << 20].contains(&size) || trial > 2 {
        return Err("registered sizes are 1/16 MiB and trials are 0..2".into());
    }
    Ok(())
}
fn emit(out: &mut impl Write, row: &Value) -> Result<()> {
    serde_json::to_writer(&mut *out, row)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}
fn rss_bytes() -> Option<u64> {
    #[cfg(unix)]
    {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
        // SAFETY: the writable structure is initialized only on success.
        let usage = unsafe {
            if libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) != 0 {
                return None;
            }
            usage.assume_init()
        };
        let rss = u64::try_from(usage.ru_maxrss).ok()?;
        if cfg!(target_os = "macos") {
            Some(rss)
        } else {
            rss.checked_mul(1024)
        }
    }
    #[cfg(not(unix))]
    {
        None
    }
}
fn store_bytes(root: &Path) -> Result<u64> {
    let mut total = 0u64;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() {
            return Err("unexpected symlink in owned payload store".into());
        }
        let bytes = if metadata.is_dir() {
            store_bytes(&entry.path())?
        } else {
            metadata.len()
        };
        total = total.checked_add(bytes).ok_or("store byte overflow")?;
        if total > DISK_CEILING {
            return Err("owned payload store exceeded 512 MiB".into());
        }
    }
    Ok(total)
}
fn public_records(bytes: &[u8]) -> Result<Vec<Value>> {
    if bytes.len() > 8 << 20 {
        return Err("measurement output exceeds 8 MiB".into());
    }
    Ok(serde_json::Deserializer::from_slice(bytes)
        .into_iter::<Value>()
        .collect::<std::result::Result<Vec<_>, _>>()?)
}
fn local_role_grants(pair: &mut Pair) -> Result<()> {
    // The same local host budget charges both Call roles. Sign the explicit
    // two-role synthetic grant used by the existing shared-broker test. Native
    // remote provisioning remains one outstanding Call and is not modified.
    let authority = ed25519_dalek::SigningKey::from_bytes(&[91; 32]);
    for bootstrap in [&mut pair.sender, &mut pair.receiver] {
        let service = bootstrap
            .service
            .as_mut()
            .ok_or("missing canonical service")?;
        for material in [
            bootstrap
                .incoming
                .as_mut()
                .ok_or("missing incoming material")?,
            bootstrap
                .outgoing
                .as_mut()
                .ok_or("missing outgoing material")?,
            &mut service.incoming_control,
            &mut service.outgoing_control,
        ] {
            material.capability.grant.max_outstanding_calls = 2;
            material.capability =
                mvm_core::mailbox::sign_capability(material.capability.grant.clone(), &authority)?;
            material.binding = mvm_core::mailbox::transport::sign_transport_binding(
                material.binding.claims.clone(),
                &authority,
            )?;
            material.authority = authority.verifying_key().to_bytes();
        }
    }
    Ok(())
}

/// NEWSCRATCH must not exist. Completed evidence is flushed before deleting
/// this cell's owned payload directory; errors preserve it for diagnosis.
pub fn run_cell(
    scratch: &Path,
    arm: Arm,
    size: usize,
    trial: u64,
    out: &mut impl Write,
) -> Result<Value> {
    validate(size, trial)?;
    if !matches!(arm, Arm::RawMixed | Arm::WholeMixed | Arm::CombinedCpu) {
        return Err("arm must be raw-mixed, whole-mixed or combined-cpu".into());
    }
    fs::create_dir(scratch)?;
    let cache = scratch.join("cache");
    let wall = Instant::now();
    let cpu = ProcessTime::now();
    let setup_wall = Instant::now();
    let setup_cpu = ProcessTime::now();
    let plan = Plan {
        stage: Stage::Two,
        arm,
        size,
        trial,
        bandwidth_profile: Default::default(),
    };
    let mut pair = Pair::provision(plan.clone())?;
    local_role_grants(&mut pair)?;
    let broker = campaign::broker_context()?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let sender_socket = TcpStream::connect(listener.local_addr()?)?;
    let (receiver_socket, _) = listener.accept()?;
    drop(listener);
    sender_socket.set_nodelay(true)?;
    receiver_socket.set_nodelay(true)?;
    let setup_wall_us = setup_wall.elapsed().as_micros() as u64;
    let setup_cpu_us = setup_cpu.elapsed().as_micros() as u64;
    let (
        sent,
        received,
        sender_records,
        receiver_records,
        sender_thread_cpu_us,
        receiver_thread_cpu_us,
    ) = std::thread::scope(|scope| -> Result<_> {
        let receiver = scope.spawn(|| -> Result<_> {
            let thread_cpu = ThreadTime::now();
            let mut records = Vec::new();
            let result = campaign::receiver_with_broker(
                &pair.receiver,
                &broker,
                &cache,
                receiver_socket,
                HandshakeRole::Acceptor,
                &mut records,
            );
            Ok((result?, records, thread_cpu.elapsed().as_micros() as u64))
        });
        let thread_cpu = ThreadTime::now();
        let mut records = Vec::new();
        let sent = campaign::sender_with_broker(
            &pair.sender,
            &broker,
            sender_socket,
            HandshakeRole::Initiator,
            &mut records,
        );
        let sender_cpu = thread_cpu.elapsed().as_micros() as u64;
        // Always join the receiver, even if the sender reports an error.
        let received = receiver.join().map_err(|_| "receiver thread panicked")?;
        let sent = sent?;
        let (received, receiver_records, receiver_cpu) = received?;
        Ok((
            sent,
            received,
            records,
            receiver_records,
            sender_cpu,
            receiver_cpu,
        ))
    })?;
    let transfer_verified_wall_us = wall.elapsed().as_micros() as u64;
    let transfer_process_cpu_us = cpu.elapsed().as_micros() as u64;
    let sender_records = public_records(&sender_records)?;
    let receiver_records = public_records(&receiver_records)?;
    let publications: Vec<Published> = serde_json::from_value(received["published"].clone())?;
    let readback_wall = Instant::now();
    let readback_cpu = ThreadTime::now();
    let mut verification = Vec::new();
    campaign::verify(&plan, &cache, &publications, &mut verification)?;
    let offline_readback_cpu_us = readback_cpu.elapsed().as_micros() as u64;
    let offline_readback_wall_us = readback_wall.elapsed().as_micros() as u64;
    let counters_match = [
        ("envelope_bytes_sent", "envelope_bytes_received"),
        ("envelope_bytes_received", "envelope_bytes_sent"),
        ("framing_bytes_sent", "framing_bytes_received"),
        ("framing_bytes_received", "framing_bytes_sent"),
        ("frames_sent", "frames_received"),
        ("frames_received", "frames_sent"),
    ]
    .iter()
    .all(|(a, b)| sent["counters"][a] == received["counters"][b]);
    if !counters_match
        || received["exact_reopened"] != true
        || publications.len() != if arm.is_combined() { 10 } else { 8 }
    {
        return Err("mixed object campaign fidelity or stream accounting gate failed".into());
    }
    let disk_bytes = store_bytes(scratch)?;
    let peak_rss = rss_bytes();
    if peak_rss.is_some_and(|rss| rss > RSS_CEILING) {
        return Err("process lifetime RSS exceeded 1 GiB".into());
    }
    let actual = |name: &str| -> Result<u64> {
        sent["counters"][name]
            .as_u64()
            .ok_or_else(|| "missing actual stream counter".into())
    };
    let forward = actual("envelope_bytes_sent")?
        .checked_add(actual("framing_bytes_sent")?)
        .ok_or("counter overflow")?;
    let reverse = actual("envelope_bytes_received")?
        .checked_add(actual("framing_bytes_received")?)
        .ok_or("counter overflow")?;
    let operations = sender_records
        .iter()
        .filter(|row| row["event"] == "sender_operation")
        .cloned()
        .collect::<Vec<_>>();
    let targets = operations
        .iter()
        .filter(|row| row["kind"] != "runtime-checkpoint" && row["kind"] != "schema")
        .cloned()
        .collect::<Vec<_>>();
    if targets.len() != 8 {
        return Err("expected eight registered application targets".into());
    }
    let row = json!({"event":"broker_objects_verified","plan":plan,"scope":"TCP loopback; two roles in one process and shared broker; no native MVM or WAN claim",
        "completed_objects":publications.len(),"verified_exact_objects":publications.len(),"targets":targets,
        "actual_caller_to_receiver_bytes":forward,"actual_receiver_to_caller_bytes":reverse,"actual_both_direction_bytes":forward+reverse,
        "sender_receiver_counters_match":counters_match,"sender_counters":sent["counters"],"receiver_counters":received["counters"],
        "setup_cpu_us":setup_cpu_us,"setup_wall_us":setup_wall_us,"verified_transfer_wall_us":transfer_verified_wall_us,"aggregate_process_cpu_us":transfer_process_cpu_us,
        "sender_thread_cpu_us":sender_thread_cpu_us,"receiver_thread_cpu_us":receiver_thread_cpu_us,
        "offline_readback_cpu_us":offline_readback_cpu_us,"offline_readback_wall_us":offline_readback_wall_us,
        "peak_process_rss_bytes":peak_rss,"owned_store_bytes_before_cleanup":disk_bytes,"sender_service":sent["service"],"receiver_service":received["service"],
        "sender_records":sender_records,"receiver_records":receiver_records,"offline_verification":public_records(&verification)?,
        "measurement_contract":{"baseline":"All arms use the same eight target byte strings, durable object verification, canonical encryption and exact-object warm probe. RawMixed is a raw-byte codec with the shared exact-object cache, not a cacheless resend baseline.",
            "setup":"Aggregate starts before provisioning and includes both actors fixture generation, codec preparation, initial status, all checkpoint/schema seeding and object exchanges; publication receipt precedes success. Setup fields separately report only harness provisioning/socket construction; per-role setup is retained in records.",
            "cpu":"Aggregate process CPU measured once. Per-role ThreadTime includes that role and broker work on its thread; receiver includes its independent readback. Original campaign cpu_us fields are overlapping process-clock intervals; do not sum them as endpoint CPU.",
            "time":"Verified transfer includes compute, durable publications and receiver readback; offline re-verification and cleanup measured separately. No send-only or energy inference.",
            "bytes":"Actual canonical encrypted envelopes plus framing, both directions including initialization and all setup publications; excludes TCP/IP headers and retransmission.",
            "rss":"Process lifetime high-water value; independent cells require fresh processes.","ceilings":{"owned_payload_bytes":DISK_CEILING,"sampled_peak_process_rss_bytes":RSS_CEILING}}});
    emit(out, &row)?;
    let cleanup_wall = Instant::now();
    let cleanup_cpu = ThreadTime::now();
    fs::remove_dir_all(scratch)?;
    let cleanup = json!({"event":"broker_objects_cleanup","plan":plan,"cache_removed_after_verified_record":true,
        "cleanup_cpu_us":cleanup_cpu.elapsed().as_micros() as u64,"cleanup_wall_us":cleanup_wall.elapsed().as_micros() as u64,
        "lifecycle_cpu_us":cpu.elapsed().as_micros() as u64,"lifecycle_wall_us":wall.elapsed().as_micros() as u64});
    emit(out, &cleanup)?;
    let mut result = row;
    result["cache_removed_after_verified_record"] = Value::Bool(true);
    result["cleanup"] = cleanup;
    Ok(result)
}

#[allow(dead_code)]
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 5 {
        return Err("usage: broker_objects NEW_SCRATCH NEW_RESULT raw-mixed|whole-mixed|combined-cpu SIZE TRIAL".into());
    }
    let arm = spiralcore_mvm_live::config::parse_arm(&args[2])?;
    let size = args[3].parse()?;
    let trial = args[4].parse()?;
    validate(size, trial)?;
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    run_cell(Path::new(&args[0]), arm, size, trial, &mut out)?;
    Ok(())
}
