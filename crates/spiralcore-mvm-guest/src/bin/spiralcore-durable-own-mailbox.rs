//! One synthetic own-mailbox transfer per fresh guest. The original signed
//! plan selects the fixed MVM mailbox. Worker requests supply frame bytes only.
#![forbid(unsafe_code)]
use cpu_time::ProcessTime;
use mvm_agentd::worker_protocol::{
    WorkerCallRequest, WorkerCallResponse, WorkerOutcome, read_pipe_frame, write_pipe_frame,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use spiralcore_mvm_guest::{PendingFrame, ReaderTraffic, receive_before};
use spiralcore_transfer::{MAX_FRAME, hex, protocol::Frame};
use std::{
    error::Error,
    fs::{DirBuilder, File},
    io::{self, Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::Path,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn commit_body(dir: &Path, delivered: &[u8], original: &[u8]) -> Result<[u8; 32]> {
    let path = dir.join("verified-body");
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    file.write_all(delivered)?;
    file.sync_all()?;
    File::open(dir)?.sync_all()?;
    let mut readback = Vec::new();
    File::open(path)?
        .take((MAX_FRAME + 1) as u64)
        .read_to_end(&mut readback)?;
    if readback != original {
        return Err("workload file readback differs from original body".into());
    }
    Ok(Sha256::digest(readback).into())
}

fn transfer(call: WorkerCallRequest) -> Result<Value> {
    let start = Instant::now();
    let cpu = ProcessTime::now();
    if !call.env.is_empty() || call.timeout_secs <= 1 {
        return Err("expected encoded frame, no overrides, and a bounded call budget".into());
    }
    let deadline = start + Duration::from_secs((call.timeout_secs - 1).min(40));
    // The signed reader reply has a 64 KiB JSON limit and base64-encodes
    // its payload alongside the signed lease. This fixture admits 16 KiB.
    if call.stdin.len() > 16 * 1024 {
        return Err("own-mailbox qualification frame exceeds 16 KiB".into());
    }
    let original = Frame::decode(&call.stdin)?;
    let pending = PendingFrame::new(&original)?;
    if pending.encoded() != call.stdin {
        return Err("noncanonical input".into());
    }

    // The isolated image exposes /tmp as a per-boot tmpfs. Reserve one
    // operation before staging. On error or uncertain ACK, retain its evidence.
    let dir = Path::new("/tmp/spiralcore-durable-own-mailbox");
    DirBuilder::new().mode(0o700).create(dir)?;
    let stage_start = Instant::now();
    pending.stage()?;
    let stage_ns = stage_start.elapsed().as_nanos() as u64;
    let mut receive_io = ReaderTraffic::default();
    let mut empty_polls = 0;
    let read_start = Instant::now();
    let delivery = loop {
        if let Some(delivery) = receive_before(deadline, &mut receive_io)? {
            break delivery;
        }
        empty_polls += 1;
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "waiting for own delivery"))?;
        std::thread::sleep(remaining.min(Duration::from_millis(100)));
    };
    let read_ns = read_start.elapsed().as_nanos() as u64;
    let verify_start = Instant::now();
    if delivery.message_id() != pending.message_id() || delivery.frame().encode()? != call.stdin {
        return Err("received lease or frame differs from the staged original".into());
    }
    let digest = commit_body(dir, &delivery.frame().body, &original.body)?;
    let commit_ns = verify_start.elapsed().as_nanos() as u64;
    let ack_start = Instant::now();
    let mut ack_io = ReaderTraffic::default();
    delivery.acknowledge_before(deadline, &mut ack_io)?;
    Ok(json!({
        "profile": "mvm_own_mailbox_guest_v1",
        "message_id": hex(pending.message_id().as_bytes()),
        "body_sha256": hex(&digest), "body_bytes": original.body.len(), "frame_bytes": call.stdin.len(),
        "empty_polls": empty_polls, "receive_request_bytes": receive_io.written,
        "receive_response_bytes": receive_io.read, "ack_request_bytes": ack_io.written,
        "ack_response_bytes": ack_io.read, "stage_ns": stage_ns, "receive_ns": read_ns,
        "compare_and_file_commit_ns": commit_ns, "ack_ns": ack_start.elapsed().as_nanos() as u64,
        "workload_process_cpu_ns": cpu.elapsed().as_nanos() as u64,
        "workload_wall_ns": start.elapsed().as_nanos() as u64,
    }))
}

fn response(call: WorkerCallRequest) -> WorkerCallResponse {
    match transfer(call) {
        Ok(value) => WorkerCallResponse {
            stdout: value.to_string().into_bytes(),
            stderr: vec![],
            controls: vec![],
            outcome: WorkerOutcome::Exit { code: 0 },
        },
        Err(error) => WorkerCallResponse {
            stdout: vec![],
            stderr: error.to_string().into_bytes(),
            controls: vec![],
            outcome: WorkerOutcome::Exit { code: 1 },
        },
    }
}

fn main() -> io::Result<()> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    loop {
        let request = match read_pipe_frame(&mut input) {
            Ok(request) => request,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e),
        };
        write_pipe_frame(&mut output, &response(request))?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_commit_is_exact_and_single_use() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = b"\0one\0\0";
        assert_eq!(
            commit_body(dir.path(), bytes, bytes).unwrap().as_slice(),
            &Sha256::digest(bytes)[..]
        );
        assert!(commit_body(dir.path(), b"replacement", b"replacement").is_err());
        assert_eq!(
            std::fs::read(dir.path().join("verified-body")).unwrap(),
            bytes
        );
        let different = tempfile::tempdir().unwrap();
        assert!(commit_body(different.path(), b"one", b"one\0").is_err());
    }

    #[test]
    fn unsupported_invocations_fail_before_staging() {
        let call = |stdin, timeout_secs, env| WorkerCallRequest {
            stdin,
            timeout_secs,
            env,
        };
        let too_large = Frame {
            kind: 1,
            transfer: [0; 32],
            seq: 1,
            body: vec![1; 16 * 1024],
        }
        .encode()
        .unwrap();
        assert!(
            transfer(call(too_large, 45, vec![]))
                .unwrap_err()
                .to_string()
                .contains("exceeds 16 KiB")
        );
        assert!(transfer(call(vec![0], 45, vec![])).is_err());
        assert!(transfer(call(vec![], 0, vec![])).is_err());
        assert!(
            transfer(call(
                vec![],
                45,
                vec![("recipient".into(), "another".into())]
            ))
            .is_err()
        );
    }
}
