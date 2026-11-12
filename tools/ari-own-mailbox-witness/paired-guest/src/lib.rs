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
    os::unix::fs::OpenOptionsExt,
    time::{Duration, Instant},
};

const LEN: usize = 257;
const FRAME: usize = 313;
pub const BODY_SHA: &str = "54acfbfedc4d8da40f76f275e1a98f10af8ef1fb9fb39e5a67a00aabcbe6597c";
const FRAME_SHA: &str = "ff26a66df5b1d9de994e60824757686d2af8eb4433b91e74a2f24b279f600044";

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
    Ok(
        json!({"role":"staged","message_id":hex::encode(message.message_id.as_bytes()),"frame_bytes":FRAME,
              "probe_to_agent":traffic.written,"probe_from_agent":traffic.read}),
    )
}
fn receive(v: &Value, deadline: Instant) -> Result<Value> {
    let id = field(v, "message_id")?;
    ensure!(
        v.as_object().is_some_and(|o| o.len() == 1),
        "unexpected destination Call parameters"
    );
    let mut traffic = ReaderTraffic::default();
    let delivery = loop {
        match receive_before(deadline, &mut traffic)? {
            Some(x) => break x,
            None => {}
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
            && hex::encode(Sha256::digest(&frame)) == FRAME_SHA
            && hex::encode(Sha256::digest(body)) == BODY_SHA,
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
    Ok(
        json!({"role":"received_and_acknowledged","message_id":hex::encode(id),"frame_bytes":frame.len(),
      "readback_hex":hex::encode(recovered),"receive_to_agent":traffic.written,"receive_from_agent":traffic.read,
      "ack_to_agent":ack.written,"ack_from_agent":ack.read}),
    )
}
pub enum Role {
    Source,
    Destination,
}
fn run(call: WorkerCallRequest, role: &Role) -> Result<Value> {
    let start = Instant::now();
    let cpu = ProcessTime::now();
    ensure!(
        call.stdin.len() <= 2048 && call.timeout_secs >= 2,
        "bounded paired workload input required"
    );
    let value = serde_json::from_slice(&call.stdin)?;
    let end = start + Duration::from_secs((call.timeout_secs - 1).min(30));
    let mut result = match role {
        Role::Source => stage(&value, end),
        Role::Destination => receive(&value, end),
    }?;
    result["cpu_ns"] = json!(cpu.elapsed().as_nanos() as u64);
    result["wall_ns"] = json!(start.elapsed().as_nanos() as u64);
    Ok(result)
}
pub fn serve(role: Role) -> io::Result<()> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    loop {
        let call = match read_pipe_frame(&mut input) {
            Ok(v) => v,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e),
        };
        let (stdout, stderr, code) = match run(call, &role) {
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
