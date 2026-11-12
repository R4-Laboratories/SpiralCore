mod control;
#[cfg(feature = "sender-ordinal-v1")]
mod sender_ordinal;
mod fixture;
#[cfg(feature="object-reader-v1")]
pub mod object_reader;
#[cfg(feature = "object-reader-v1")]
pub mod object_exchange;
#[cfg(feature="object-reader-v1")]
mod object_destination;
mod retirement;
use anyhow::{Context, Result, ensure};
use cpu_time::ProcessTime;
use mvm_agentd::worker_protocol::{
    WorkerCallRequest, WorkerCallResponse, WorkerOutcome, read_pipe_frame, write_pipe_frame,
};
use mvm_contract::protocol::mailbox::PrincipalId;
use mvm_core::mailbox::frame::OutboxMessage;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use spiralcore_mvm_guest::{ReaderTraffic, application_reader_before, receive_before};
use spiralcore_transfer::protocol::Frame;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    time::{Duration, Instant},
};

use fixture::{BODY_LEN as LEN, FRAME_LEN as FRAME, FRAME_SHA};
pub const BODY_SHA: &str = if cfg!(feature = "large-object-fixture") {
    "4a29fe7d84bd3c8097343ec5723341ac8768f67914bf61e4d3c078bb6b5fc9d9"
} else {
    "54acfbfedc4d8da40f76f275e1a98f10af8ef1fb9fb39e5a67a00aabcbe6597c"
};

fn field(v: &Value, name: &str) -> Result<[u8; 32]> {
    let value = v
        .get(name)
        .and_then(Value::as_str)
        .context("missing original operation identifier")?;
    ensure!(value.len() == 64, "invalid identifier length");
    let mut bytes = [0; 32];
    hex::decode_to_slice(value, &mut bytes)?;
    Ok(bytes)
}
fn stage(v: &Value, deadline: Instant) -> Result<Value> {
    let bytes = hex::decode(v["frame_hex"].as_str().context("original missing")?)?;
    let frame = Frame::decode(&bytes)?;
    ensure!(
        bytes.len() == FRAME
            && bytes == frame.encode()?
            && hex::encode(Sha256::digest(&bytes)) == FRAME_SHA,
        "fixture frame differs"
    );
    let commitment = field(v, "direction")?;
    let recipient = PrincipalId::from_bytes(field(v, "recipient")?);
    let message = OutboxMessage::new(recipient, bytes);
    let mut traffic = ReaderTraffic::default();
    {
        let mut stream = application_reader_before(deadline, &mut traffic)?;
        mvm_agentd::mailbox_outbox::journal_direction::stage_on_original_reader(
            &mut stream,
            commitment,
            &message,
        )?;
    }
    let mut result = json!({"role":"staged","message_id":hex::encode(message.message_id.as_bytes()),"frame_bytes":FRAME,
              "probe_to_agent":traffic.written,"probe_from_agent":traffic.read});
    if cfg!(feature = "receipt-loss-qualification") {
        // The original guest agent is the only process allowed to retire this
        // single staged operation. Do not restage or consume its receipt here.
        let staged_at = Instant::now();
        let slot = std::path::Path::new(mvm_agentd::mailbox_outbox::LOCAL_OUTBOX_PATH)
            .with_file_name(mvm_core::mailbox::outbound::journal_slot_name(&commitment));
        let dir = slot
            .parent()
            .context("missing canonical outbox directory")?;
        let dir_handle = File::open(dir)?;
        let held = dir_handle.metadata()?;
        loop {
            match fs::symlink_metadata(&slot) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => break,
                Ok(meta) => ensure!(meta.is_file(), "unexpected outbox file type"),
                Err(e) => return Err(e.into()),
            }
            ensure!(
                Instant::now() < deadline,
                "original outbox remained pending"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let current = fs::symlink_metadata(dir)?;
        ensure!(
            current.is_dir() && (current.dev(), current.ino()) == (held.dev(), held.ino()),
            "canonical outbox directory changed"
        );
        let message_id = hex::encode(message.message_id.as_bytes());
        retirement::require_no_directed_archive(&slot, &message_id)?;
        result["outbox_retired_without_archive"] = json!(true);
        result["outbox_retirement_wait_ns"] = json!(staged_at.elapsed().as_nanos() as u64);
    }
    Ok(result)
}
fn receive(v: &Value, deadline: Instant) -> Result<Value> {
    let id = field(v, "message_id")?;
    #[cfg(feature = "sender-ordinal-v1")]
    let ordinal=v.get("ordinal_readback").map(|n| n.as_u64().context("invalid readback ordinal")).transpose()?;
    #[cfg(not(feature = "sender-ordinal-v1"))]
    let ordinal:Option<u64>=None;
    ensure!(
        v.as_object().is_some_and(|o| if ordinal.is_some() {o.len()==3 && v["ephemeral_readback"]==true}
            else {o.len()==1 || (o.len()==2 && v["ephemeral_readback"]==true)}),
        "unexpected destination Call parameters"
    );
    let mut traffic = ReaderTraffic::default();
    let delivery = loop {
        if let Some(delivery) = receive_before(deadline, &mut traffic)? {
            break delivery;
        }
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .context("expected delivery did not arrive")?;
        std::thread::sleep(remaining.min(Duration::from_millis(20)));
    };
    let frame = delivery.frame().encode()?;
    let body = &delivery.frame().body;
    ensure!(
        *delivery.message_id().as_bytes() == id
            && frame.len() == FRAME
            && body.len() == LEN
            && match ordinal {
                Some(n)=> fixture::ordinal_frame(n).is_some_and(|expected| frame==expected),
                None=>hex::encode(Sha256::digest(&frame)) == FRAME_SHA && hex::encode(Sha256::digest(body)) == BODY_SHA,
            },
        "destination lease disagrees with original message or fixture digest"
    );
    let path = "/tmp/spiralcore-paired-received";
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(body)?;
    file.sync_all()?;
    drop(file);
    File::open("/tmp")?.sync_all()?;
    let recovered = fs::read(path)?;
    ensure!(
        recovered == *body,
        "destination file readback differs from the original lease"
    );
    let mut ack = ReaderTraffic::default();
    delivery.acknowledge_before(deadline, &mut ack)?;
    if v["ephemeral_readback"] == true {
        // The guest retained and verified this file through ACK. Bound scratch
        // space for the next distinct operation on this same worker.
        fs::remove_file(path)?;
        File::open("/tmp")?.sync_all()?;
    }
    let mut result = json!({"role":"received_and_acknowledged","message_id":hex::encode(id),"frame_bytes":frame.len(),
      "readback_hex":hex::encode(recovered),"receive_to_agent":traffic.written,"receive_from_agent":traffic.read,
      "ack_to_agent":ack.written,"ack_from_agent":ack.read});
    if cfg!(feature = "receipt-loss-qualification") {
        let mut empty = ReaderTraffic::default();
        ensure!(
            receive_before(deadline, &mut empty)?.is_none(),
            "retry made a second delivery available after ACK"
        );
        result["after_ack"] =
            json!({"empty":true,"to_agent":empty.written,"from_agent":empty.read});
    }
    Ok(result)
}
pub enum Role {
    Source,
    Destination,
}
struct Worker {
    role: Role,
    #[cfg(feature = "object-reader-v1")]
    objects: object_exchange::GuestObjectSource,
    #[cfg(feature="object-reader-v1")]
    destination:object_destination::GuestDestination,
}
fn run(call: WorkerCallRequest, worker: &mut Worker) -> Result<Value> {
    let start = Instant::now();
    let cpu = ProcessTime::now();
    ensure!(
        call.stdin.len() <= fixture::CALL_INPUT_LIMIT && call.timeout_secs >= 2,
        "bounded paired workload input required"
    );
    let value: Value = serde_json::from_slice(&call.stdin)?;
    let end = start + Duration::from_secs((call.timeout_secs - 1).min(30));
    let mut result = match &worker.role {
        #[cfg(feature = "object-ack-loss-qualification")]
        Role::Source if value.get("object_ack_control").is_some() => {
            ensure!(value.as_object().is_some_and(|v| v.len() == 1), "ambiguous ACK qualification");
            object_exchange::ack_loss::control(&mut worker.objects, &value["object_ack_control"], end)
        }
        #[cfg(feature = "object-reader-v1")]
        Role::Source if value.get("object_settle").is_some() => {
            ensure!(value.as_object().is_some_and(|v| v.len() == 1), "ambiguous settlement operation");
            object_exchange::retry_settlement(&mut worker.objects, &value["object_settle"], end)
        }
        #[cfg(feature = "object-reader-v1")]
        Role::Source if value.get("object_send").is_some() => {
            ensure!(value.as_object().is_some_and(|v| v.len() == 1), "ambiguous object worker operation");
            object_exchange::run_source(&mut worker.objects, &value["object_send"], end)
        }
        #[cfg(feature = "sender-ordinal-v1")]
        Role::Source if value.get("sender_ordinal").is_some() => sender_ordinal::run(&value, end),
        #[cfg(feature = "sender-ordinal-v1")]
        Role::Destination if value.get("ordinal_empty_control").is_some() => sender_ordinal::empty(&value, end),
        Role::Source if value.get("control_probe").is_some() => control::probe(&value, end),
        Role::Source => stage(&value, end),
        #[cfg(feature="object-reader-v1")]
        Role::Destination if value.get("object_receive").is_some() => {
            ensure!(value.as_object().is_some_and(|fields| fields.len() == 1), "ambiguous object receive");
            worker.destination.run(&value["object_receive"], end).map_err(|error| {
                anyhow::anyhow!("destination attempt wall_ns={} cpu_ns={}; {error:#}", start.elapsed().as_nanos(), cpu.elapsed().as_nanos())
            })
        }
        Role::Destination => receive(&value, end),
    }?;
    result["cpu_ns"] = json!(cpu.elapsed().as_nanos() as u64);
    result["wall_ns"] = json!(start.elapsed().as_nanos() as u64);
    Ok(result)
}
pub fn serve(role: Role) -> io::Result<()> {
    let mut worker = Worker {
        role,
        #[cfg(feature = "object-reader-v1")]
        objects: object_exchange::GuestObjectSource::default(),
        #[cfg(feature="object-reader-v1")]
        destination:Default::default(),
    };
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    loop {
        let call = match read_pipe_frame(&mut input) {
            Ok(v) => v,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e),
        };
        let (stdout, stderr, code) = match run(call, &mut worker) {
            Ok(val) => (val.to_string().into_bytes(), vec![], 0),
            Err(e) => (vec![], e.to_string().into_bytes(), 1),
        };
        write_pipe_frame(
            &mut output,
            &WorkerCallResponse {
                stdout,
                stderr,
                controls: vec![],
                outcome: WorkerOutcome::Exit { code },
            },
        )?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "large-object-fixture")]
    #[test]
    fn four_kib_fixture_is_canonical_and_fits_the_bounded_call() {
        let mut state = 7u32;
        let mut body: Vec<u8> = (0..4096)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect();
        body[0] = 0;
        body[4095] = 0;
        let frame = Frame {
            kind: 2,
            transfer: [7; 32],
            seq: 1,
            body: body.clone(),
        };
        let bytes = frame.encode().unwrap();
        assert_eq!(Frame::decode(&bytes).unwrap(), frame);
        assert_eq!(bytes.len(), FRAME);
        assert_eq!(hex::encode(Sha256::digest(&body)), BODY_SHA);
        assert_eq!(hex::encode(Sha256::digest(&bytes)), FRAME_SHA);
        let request = json!({"frame_hex":hex::encode(bytes),"recipient":"00".repeat(32),"direction":"01".repeat(32)});
        assert!(serde_json::to_vec(&request).unwrap().len() <= fixture::CALL_INPUT_LIMIT);
    }
    #[test]
    fn refuses_unexpected_inputs_before_opening_either_original_reader() {
        let until = Instant::now() + Duration::from_secs(1);
        assert!(stage(&json!({"frame_hex":"0000","recipient":"00".repeat(32),"direction":"01".repeat(32)}),until).is_err());
        assert!(
            receive(
                &json!({"message_id":"ef".repeat(32),"frame_hex":"00"}),
                until
            )
            .unwrap_err()
            .to_string()
            .contains("unexpected destination")
        );
        assert!(
            receive(&json!({"message_id":"123"}), until)
                .unwrap_err()
                .to_string()
                .contains("length")
        );
    }
}
