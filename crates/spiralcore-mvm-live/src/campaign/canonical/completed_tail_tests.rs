use super::*;
use crate::config::{Arm, Pair, Plan, Stage};
use spiralcore_transfer::{client, protocol::Mode};
use std::{net::Shutdown, os::unix::net::UnixStream, sync::mpsc, thread::JoinHandle};

struct Receiver {
    socket: UnixStream,
    thread: Option<JoinHandle<Result<Value>>>,
}
impl Drop for Receiver {
    fn drop(&mut self) {
        let _ = self.socket.shutdown(Shutdown::Both);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
struct Cache(std::path::PathBuf);
impl Drop for Cache {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn completed_ack_loss_through_actual_duplex_relay_replays_exactly_then_reopens_all_objects() {
    let plan = Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 4096,
        trial: 0,
        bandwidth_profile: Default::default(),
    };
    let pair = Pair::provision(plan.clone()).unwrap();
    let cases = crate::fixtures::objects(plan.size, plan.trial).unwrap();
    let cache = Cache(std::env::temp_dir().join(format!(
        "sc-completed-relay-{}-{:016x}",
        std::process::id(),
        rand::random::<u64>()
    )));
    let (caller_socket, receiver_socket, gate, relay) =
        super::reconciliation_tests::relayed_transport(&pair, Some(2), None);
    let gate_state = gate.shared();
    let close = receiver_socket.try_clone().unwrap();
    let cache_path = cache.0.clone();
    let (signals, signal_rx) = mpsc::channel();
    let (done_tx, done) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let result = receiver_with_broker_and_reply_replay(
            &pair.receiver,
            &broker_context()?,
            &cache_path,
            receiver_socket,
            HandshakeRole::Initiator,
            &mut Vec::new(),
            &mut |signal| {
                signals.send(signal)?;
                Ok(())
            },
        );
        let _ = done_tx.send(());
        result
    });
    let mut receiver = Receiver {
        socket: close,
        thread: Some(thread),
    };
    let broker = broker_context().unwrap();
    let mut caller = CallerEndpoint::new(
        &pair.sender,
        &broker,
        caller_socket,
        HandshakeRole::Initiator,
    )
    .unwrap();
    let request = Frame {
        kind: 5,
        transfer: [0; 32],
        seq: 0,
        body: vec![],
    };
    let (_, reply) = caller.call_with_completed_tail(&request).unwrap();
    assert_eq!(reply.kind, 127);
    assert_eq!(caller.service_report()["application_calls"], 1);
    let signal = signal_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while gate_state.lock().unwrap().forward.withheld.is_none() {
        assert!(
            Instant::now() < deadline,
            "completed ACK never reached fault gate"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let before = caller.counters().unwrap();
    assert_eq!(
        caller.drive_completed_tail().unwrap(),
        TailDriveOutcome::Idle
    );
    assert_eq!(caller.counters().unwrap().frames_sent, before.frames_sent);
    signal.request_replay().unwrap();
    loop {
        match caller.drive_completed_tail().unwrap() {
            TailDriveOutcome::AckWritten => break,
            TailDriveOutcome::Idle if Instant::now() < deadline => {}
            other => panic!("explicit relay replay failed: {other:?}"),
        }
    }
    for (position, case) in cases.iter().enumerate() {
        let mut id = [0; 32];
        id[..8].copy_from_slice(&(position as u64 + 1).to_le_bytes());
        id[8..16].copy_from_slice(&plan.trial.to_le_bytes());
        id[16..24].copy_from_slice(&(plan.size as u64).to_le_bytes());
        client::transfer(&case.bytes, Mode::Raw, None, id, |request| {
            caller
                .call_with_completed_tail(request)
                .map(|(_, reply)| reply)
                .map_err(|_| spiralcore_transfer::Error::Storage)
        })
        .unwrap();
    }
    let caller_counters = caller.counters().unwrap();
    let calls = caller.service_report()["application_calls"]
        .as_u64()
        .unwrap();
    assert_eq!(caller.service_report()["recovery"]["tail_acks_written"], 1);
    drop(caller);
    done.recv_timeout(Duration::from_secs(5))
        .expect("receiver cache was not released");
    let result = receiver.thread.take().unwrap().join().unwrap().unwrap();
    assert_eq!(result["exact_reopened"], true);
    assert_eq!(result["operations"], cases.len());
    assert_eq!(result["service"]["application_calls"], calls);
    assert_eq!(
        result["service"]["recovery"]["reply_replay_commands_consumed"],
        1
    );
    assert_eq!(
        result["service"]["recovery"]["cached_reply_retry_invocations"],
        1
    );
    let snapshot = gate.finish().unwrap();
    assert_eq!(
        snapshot.forward.read.frames_received,
        caller_counters.frames_sent
    );
    assert_eq!(
        snapshot.reverse.written.frames_sent,
        caller_counters.frames_received
    );
    let held = snapshot.forward.withheld.as_ref().unwrap();
    assert_eq!(held.ordinal, 2);
    assert_eq!(held.length, snapshot.forward.captured[1].len());
    assert_eq!(snapshot.reverse.captured[1], snapshot.reverse.captured[2]);
    let relay = relay.finish();
    assert_eq!(
        relay["report"]["a"]["frames_received"],
        snapshot.forward.written.frames_sent
    );
    assert_eq!(
        relay["report"]["b"]["frames_sent"],
        snapshot.forward.written.frames_sent
    );
    assert_eq!(
        relay["report"]["b"]["frames_received"],
        snapshot.reverse.read.frames_received
    );
    assert_eq!(
        relay["report"]["a"]["frames_sent"],
        snapshot.reverse.read.frames_received
    );
    assert!(matches!(
        signal.request_replay(),
        Err(mpsc::TrySendError::Disconnected(()))
    ));
}
