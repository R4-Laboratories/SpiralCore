use spiralcore_mvm_adapter::live::HandshakeRole;
use spiralcore_mvm_live::{
    campaign::{self, CallerEndpoint, TailDriveOutcome},
    config::{Arm, Pair, Plan, Stage},
};
use std::{
    io::Read,
    net::{TcpListener, TcpStream},
    time::Duration,
};

#[path = "common/reconciliation_gate.rs"]
mod gate;

#[test]
fn public_owned_caller_repairs_postcompletion_ack_loss_then_transfers_and_reopens_exact_objects() {
    public_campaign(Fault::CompletedAck);
}
#[test]
fn public_owned_caller_combines_explicit_call_reconciliation_with_retained_tail() {
    public_campaign(Fault::CallReceipt);
}
#[test]
fn strict_owned_call_has_no_tail_and_normal_receiver_remains_compatible() {
    public_campaign(Fault::None);
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    None,
    CallReceipt,
    CompletedAck,
}
fn public_campaign(fault: Fault) {
    use spiralcore_transfer::{
        client,
        protocol::{Frame, Mode},
    };
    use std::{net::Shutdown, sync::mpsc, thread::JoinHandle, time::Instant};
    struct ReceiverTask {
        socket: TcpStream,
        done: mpsc::Receiver<()>,
        thread: Option<JoinHandle<spiralcore_mvm_live::Result<serde_json::Value>>>,
        controller: Option<JoinHandle<()>>,
    }
    impl Drop for ReceiverTask {
        fn drop(&mut self) {
            let _ = self.socket.shutdown(Shutdown::Both);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
            if let Some(thread) = self.controller.take() {
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
    let plan = Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 4096,
        trial: 0,
        bandwidth_profile: Default::default(),
    };
    let pair = Pair::provision(plan.clone()).unwrap();
    let cases = spiralcore_mvm_live::fixtures::objects(plan.size, plan.trial).unwrap();
    let cache = Cache(std::env::temp_dir().join(format!(
        "sc-completed-tail-{}-{:016x}",
        std::process::id(),
        rand::random::<u64>()
    )));
    // Forward frame2 is the first Reply ACK. The complete authenticated frame
    // reaches the gate but is withheld while the caller returns its result.
    let (caller_socket, receiver_socket, gate) = match fault {
        Fault::CompletedAck => gate::Gate::start_withheld(Some(2), None),
        Fault::CallReceipt => gate::Gate::start_missing_first_receipt(),
        Fault::None => gate::Gate::start_withheld(None, None),
    }
    .unwrap();
    let gate_state = gate.shared();
    let close = receiver_socket.try_clone().unwrap();
    let cache_path = cache.0.clone();
    let (signals, signal_rx) = mpsc::channel();
    let (done_tx, done) = mpsc::channel();
    let receiver = std::thread::spawn(move || {
        let result = if fault == Fault::None {
            campaign::receiver_with_broker(
                &pair.receiver,
                &campaign::broker_context()?,
                &cache_path,
                receiver_socket,
                HandshakeRole::Acceptor,
                &mut Vec::new(),
            )
        } else {
            campaign::receiver_with_broker_and_reply_replay(
                &pair.receiver,
                &campaign::broker_context()?,
                &cache_path,
                receiver_socket,
                HandshakeRole::Acceptor,
                &mut Vec::new(),
                &mut |signal| {
                    signals.send(signal)?;
                    Ok(())
                },
            )
        };
        let _ = done_tx.send(());
        result
    });
    let mut receiver = ReceiverTask {
        socket: close,
        done,
        thread: Some(receiver),
        controller: None,
    };
    let broker = campaign::broker_context().unwrap();
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
    let (_, probe) = match fault {
        Fault::None => caller.call(&request),
        Fault::CompletedAck => caller.call_with_completed_tail(&request),
        Fault::CallReceipt => {
            caller.call_with_completed_tail_and_reconciliation(&request, &mut |signal| {
                let state = gate_state.clone();
                receiver.controller = Some(std::thread::spawn(move || {
                    let deadline = Instant::now() + Duration::from_secs(5);
                    while Instant::now() < deadline {
                        if state.lock().unwrap().reverse.withheld.is_some() {
                            signal.request_retry().unwrap();
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    panic!("actual first receipt was not withheld");
                }));
                Ok(())
            })
        }
    }
    .unwrap();
    if let Some(thread) = receiver.controller.take() {
        thread.join().unwrap();
    }
    assert_eq!(probe.kind, 127);
    let signal = if fault == Fault::None {
        None
    } else {
        Some(signal_rx.recv_timeout(Duration::from_secs(5)).unwrap())
    };
    let before = caller.counters().unwrap();
    assert_eq!(
        caller.drive_completed_tail().unwrap(),
        if fault == Fault::None {
            TailDriveOutcome::NoTail
        } else {
            TailDriveOutcome::Idle
        }
    );
    assert_eq!(
        caller.counters().unwrap().frames_sent,
        before.frames_sent,
        "silence sends nothing"
    );
    if fault == Fault::CompletedAck {
        let limit = Instant::now() + Duration::from_secs(5);
        while gate_state.lock().unwrap().forward.withheld.is_none() {
            assert!(
                Instant::now() < limit,
                "completed ACK did not reach the gate"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        signal.as_ref().unwrap().request_replay().unwrap();
        loop {
            match caller.drive_completed_tail().unwrap() {
                TailDriveOutcome::AckWritten => break,
                TailDriveOutcome::Idle if Instant::now() < limit => {}
                other => panic!("explicit replay did not produce a local ACK: {other:?}"),
            }
        }
        assert_eq!(caller.service_report()["recovery"]["tail_acks_written"], 1);
    }
    assert_eq!(caller.service_report()["application_calls"], 1);
    for (position, case) in cases.iter().enumerate() {
        let mut id = [0; 32];
        id[..8].copy_from_slice(&(position as u64 + 1).to_le_bytes());
        id[8..16].copy_from_slice(&plan.trial.to_le_bytes());
        id[16..24].copy_from_slice(&(plan.size as u64).to_le_bytes());
        let mut call_error = None;
        let sent = client::transfer(&case.bytes, Mode::Raw, None, id, |frame| {
            caller
                .call_with_completed_tail(frame)
                .map(|(_, reply)| reply)
                .map_err(|error| {
                    call_error = Some(error.to_string());
                    spiralcore_transfer::Error::Storage
                })
        });
        assert!(
            sent.is_ok(),
            "actual next identity failed: {sent:?}; {call_error:?}"
        );
    }
    let caller_counters = caller.counters().unwrap();
    let caller_calls = caller.service_report()["application_calls"]
        .as_u64()
        .unwrap();
    drop(caller);
    assert!(
        receiver.done.recv_timeout(Duration::from_secs(5)).is_ok(),
        "receiver remained cached/fenced"
    );
    let result = receiver.thread.take().unwrap().join().unwrap().unwrap();
    assert_eq!(result["exact_reopened"], true);
    assert_eq!(result["operations"], cases.len());
    assert_eq!(result["service"]["application_calls"], caller_calls);
    assert_eq!(
        result["service"]["recovery"]["reply_replay_commands_consumed"],
        u64::from(fault == Fault::CompletedAck)
    );
    let snapshot = gate.finish().unwrap();
    assert!(snapshot.relay.is_none(), "this witness is direct TCP");
    assert_eq!(
        snapshot.forward.read.frames_received,
        caller_counters.frames_sent
    );
    assert_eq!(
        snapshot.reverse.written.frames_sent,
        caller_counters.frames_received
    );
    if fault != Fault::None {
        let (direction, ordinal) = if fault == Fault::CompletedAck {
            (&snapshot.forward, 2)
        } else {
            (&snapshot.reverse, 1)
        };
        let held = direction.withheld.as_ref().unwrap();
        let wire = &direction.captured[ordinal as usize - 1];
        assert_eq!(held.ordinal, ordinal);
        assert_eq!(held.length, wire.len());
        assert_eq!(
            held.hash,
            spiralcore_transfer::hash(b"spiralcore.test.withheld.v1", &[wire])
        );
        if fault == Fault::CompletedAck {
            assert_eq!(
                snapshot.reverse.captured[1], snapshot.reverse.captured[2],
                "cached Reply bytes are exact"
            );
        } else {
            assert_eq!(
                result["service"]["recovery"]["cached_reply_retry_invocations"], 0,
                "ACKed old Call receives a historical receipt without another Reply"
            );
            assert_eq!(result["service"]["recovery"]["historical_receipts_sent"], 1);
        }
        assert!(matches!(
            signal.unwrap().request_replay(),
            Err(mpsc::TrySendError::Disconnected(()))
        ));
    }
}

#[test]
fn owned_caller_public_entry_refuses_receiver_role_before_handshake() {
    let pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 4096,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let stream = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut peer, _) = listener.accept().unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    let broker = campaign::broker_context().unwrap();
    assert!(
        CallerEndpoint::new(&pair.receiver, &broker, stream, HandshakeRole::Initiator).is_err()
    );
    assert_eq!(
        peer.read(&mut [0u8; 1]).unwrap(),
        0,
        "role refusal emits no handshake bytes"
    );
}
