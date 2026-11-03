use serde_json::Value;
use spiralcore_mvm_adapter::live::HandshakeRole;
use spiralcore_mvm_live::{
    Result, campaign,
    campaign::{ReconciliationSignal, sender_with_broker_and_reconciliation},
    config::{Arm, Pair, Plan, Published, Stage},
};
use std::{
    net::{Shutdown, TcpListener, TcpStream},
    sync::mpsc::{self, TrySendError},
    thread::JoinHandle,
    time::{Duration, Instant},
};

#[path = "common/reconciliation_gate.rs"]
mod gate;

// A deadlock bound, not a transfer-time acceptance threshold. Owned raw shutdown
// handles wake either endpoint without acquiring StreamTransport's mutex.
const WATCHDOG: Duration = Duration::from_secs(5);

#[derive(Debug)]
struct Report {
    result: Value,
    records: Vec<u8>,
    signals: usize,
}

struct Endpoints {
    sockets: [TcpStream; 2],
    done: mpsc::Receiver<()>,
    sender: Option<JoinHandle<Result<Report>>>,
    receiver: Option<JoinHandle<Result<Report>>>,
    controller: Option<JoinHandle<()>>,
}
impl Endpoints {
    fn stop(&self) {
        for socket in &self.sockets {
            if socket.shutdown(Shutdown::Both).is_err() {
                let _ = socket.shutdown(Shutdown::Write);
                let _ = socket.shutdown(Shutdown::Read);
            }
        }
    }
    fn wait(&self) -> bool {
        let deadline = Instant::now() + WATCHDOG;
        (0..2).all(|_| {
            self.done
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .is_ok()
        })
    }
    fn finish(mut self) -> (Result<Report>, Result<Report>) {
        self.stop();
        let sender = self.sender.take().unwrap().join().expect("sender panicked");
        let receiver = self
            .receiver
            .take()
            .unwrap()
            .join()
            .expect("receiver panicked");
        if let Some(controller) = self.controller.take() {
            controller.join().expect("controller panicked");
        }
        (sender, receiver)
    }
}
impl Drop for Endpoints {
    fn drop(&mut self) {
        self.stop();
        for worker in [&mut self.sender, &mut self.receiver] {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
        if let Some(controller) = self.controller.take() {
            let _ = controller.join();
        }
    }
}

fn tcp_pair() -> [TcpStream; 2] {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let sender = TcpStream::connect_timeout(&listener.local_addr().unwrap(), WATCHDOG).unwrap();
    let (receiver, _) = listener.accept().unwrap();
    for socket in [&sender, &receiver] {
        socket.set_nodelay(true).unwrap();
        socket.set_read_timeout(Some(WATCHDOG)).unwrap();
        socket.set_write_timeout(Some(WATCHDOG)).unwrap();
    }
    [sender, receiver]
}

#[test]
fn public_reconciliation_entry_uses_a_fresh_signal_for_each_real_tcp_call_and_probe() {
    // First gate is public API reachability and real Client wiring. It injects
    // no loss and requests no retry; it is not a semantic recovery claim.
    let plan = Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 4096,
        trial: 0,
        bandwidth_profile: Default::default(),
    };
    let pair = Pair::provision(plan.clone()).unwrap();
    let store = tempfile::tempdir().unwrap();
    let cache = store.path().join("cache");
    let receiver_cache = cache.clone();
    let [sender_socket, receiver_socket] = tcp_pair();
    let sockets = [
        sender_socket.try_clone().unwrap(),
        receiver_socket.try_clone().unwrap(),
    ];
    let (done_tx, done) = mpsc::channel();
    let sender_done = done_tx.clone();
    let sender = std::thread::spawn(move || {
        let result = (|| -> Result<Report> {
            let broker = campaign::broker_context()?;
            let mut records = Vec::new();
            let mut signals: Vec<ReconciliationSignal> = Vec::new();
            let result = sender_with_broker_and_reconciliation(
                &pair.sender,
                &broker,
                sender_socket,
                HandshakeRole::Initiator,
                &mut records,
                &mut |signal| {
                    if let Some(previous) = signals.last() {
                        assert!(matches!(
                            previous.request_retry(),
                            Err(TrySendError::Disconnected(()))
                        ));
                    }
                    signals.push(signal);
                    Ok(())
                },
            )?;
            for signal in &signals {
                assert!(matches!(
                    signal.request_retry(),
                    Err(TrySendError::Disconnected(()))
                ));
            }
            Ok(Report {
                result,
                records,
                signals: signals.len(),
            })
        })();
        let _ = sender_done.send(());
        result
    });
    let receiver = std::thread::spawn(move || {
        let result = (|| -> Result<Report> {
            let broker = campaign::broker_context()?;
            let mut records = Vec::new();
            let result = campaign::receiver_with_broker(
                &pair.receiver,
                &broker,
                &receiver_cache,
                receiver_socket,
                HandshakeRole::Acceptor,
                &mut records,
            )?;
            Ok(Report {
                result,
                records,
                signals: 0,
            })
        })();
        let _ = done_tx.send(());
        result
    });
    let endpoints = Endpoints {
        sockets,
        done,
        sender: Some(sender),
        receiver: Some(receiver),
        controller: None,
    };
    let completed = endpoints.wait();
    let (sent, received) = endpoints.finish();
    assert!(
        completed,
        "bounded TCP embedding did not finish: {sent:?}; {received:?}"
    );
    let sent = sent.unwrap();
    let received = received.unwrap();
    assert_eq!(sent.result["event"], "sender_done");
    assert_eq!(received.result["event"], "receiver_done");
    assert!(!sent.records.is_empty() && !received.records.is_empty());
    let calls = sent.result["service"]["application_calls"]
        .as_u64()
        .unwrap();
    assert!(
        calls > sent.result["operations"].as_u64().unwrap(),
        "Status probe must also use the embedding entry"
    );
    assert_eq!(sent.signals as u64, calls);
    assert_eq!(received.result["service"]["application_calls"], calls);
    for (send, receive) in [
        ("envelope_bytes_sent", "envelope_bytes_received"),
        ("envelope_bytes_received", "envelope_bytes_sent"),
        ("framing_bytes_sent", "framing_bytes_received"),
        ("framing_bytes_received", "framing_bytes_sent"),
        ("frames_sent", "frames_received"),
        ("frames_received", "frames_sent"),
    ] {
        assert_eq!(
            sent.result["counters"][send].as_u64().unwrap(),
            received.result["counters"][receive].as_u64().unwrap()
        );
    }
    let published: Vec<Published> =
        serde_json::from_value(received.result["published"].clone()).unwrap();
    assert_eq!(
        published.len() as u64,
        sent.result["operations"].as_u64().unwrap()
    );
    assert!(received.result["exact_reopened"].as_bool().unwrap());
    campaign::verify(&plan, &cache, &published, &mut Vec::new()).unwrap();
}

#[test]
fn public_reconciliation_entry_recovers_missing_reply_and_reopens_all_exact_objects() {
    public_fault_campaign(true);
}

#[test]
fn public_reconciliation_entry_recovers_missing_call_receipt_and_reopens_all_exact_objects() {
    public_fault_campaign(false);
}

fn public_fault_campaign(missing_reply: bool) {
    let plan = Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 4096,
        trial: 0,
        bandwidth_profile: Default::default(),
    };
    let pair = Pair::provision(plan.clone()).unwrap();
    let store = tempfile::tempdir().unwrap();
    let cache = store.path().join("cache");
    let receiver_cache = cache.clone();
    // Withhold the first Reply or Call receipt by post-handshake ordinal.
    // The whole public sender/receiver campaign still has to finish and reopen.
    let (sender_socket, receiver_socket, fault_gate) = if missing_reply {
        gate::Gate::start_withheld(None, Some(2)).unwrap()
    } else {
        gate::Gate::start_missing_first_receipt().unwrap()
    };
    let sockets = [
        sender_socket.try_clone().unwrap(),
        receiver_socket.try_clone().unwrap(),
    ];
    let state = fault_gate.shared();
    let (signal_tx, signal_rx) = mpsc::sync_channel::<ReconciliationSignal>(1);
    let controller = std::thread::spawn(move || {
        let signal = signal_rx.recv_timeout(WATCHDOG).unwrap();
        let deadline = Instant::now() + WATCHDOG;
        loop {
            if state.lock().unwrap().reverse.withheld.is_some() {
                break;
            }
            assert!(Instant::now() < deadline, "first Reply fault did not occur");
            std::thread::yield_now();
        }
        assert_eq!(signal.request_retry(), Ok(()));
    });
    let (done_tx, done) = mpsc::channel();
    let sender_done = done_tx.clone();
    let sender = std::thread::spawn(move || {
        let result = (|| -> Result<Report> {
            let broker = campaign::broker_context()?;
            let mut records = Vec::new();
            let mut signals = 0;
            let result = sender_with_broker_and_reconciliation(
                &pair.sender,
                &broker,
                sender_socket,
                HandshakeRole::Initiator,
                &mut records,
                &mut |signal| {
                    signals += 1;
                    if signals == 1 {
                        signal_tx
                            .send(signal)
                            .map_err(|_| "first-Call controller disconnected")?;
                    }
                    Ok(())
                },
            )?;
            Ok(Report {
                result,
                records,
                signals,
            })
        })();
        let _ = sender_done.send(());
        result
    });
    let receiver = std::thread::spawn(move || {
        let result = (|| -> Result<Report> {
            let broker = campaign::broker_context()?;
            let mut records = Vec::new();
            let result = campaign::receiver_with_broker(
                &pair.receiver,
                &broker,
                &receiver_cache,
                receiver_socket,
                HandshakeRole::Acceptor,
                &mut records,
            )?;
            Ok(Report {
                result,
                records,
                signals: 0,
            })
        })();
        let _ = done_tx.send(());
        result
    });
    let endpoints = Endpoints {
        sockets,
        done,
        sender: Some(sender),
        receiver: Some(receiver),
        controller: Some(controller),
    };
    let complete = endpoints.wait();
    let (sent, received) = endpoints.finish();
    let traffic = fault_gate.finish().unwrap();
    assert!(
        complete,
        "bounded public recovery failed: {sent:?}; {received:?}"
    );
    let sent = sent.unwrap();
    let received = received.unwrap();
    assert_eq!(sent.result["event"], "sender_done");
    assert_eq!(received.result["event"], "receiver_done");
    assert_eq!(
        sent.result["service"]["recovery"]["call_retry_invocations"],
        1
    );
    assert_eq!(
        received.result["service"]["recovery"]["cached_reply_retry_invocations"],
        u64::from(missing_reply)
    );
    assert_eq!(
        sent.signals as u64,
        sent.result["service"]["application_calls"]
            .as_u64()
            .unwrap()
    );
    assert_eq!(
        sent.result["service"]["application_calls"],
        received.result["service"]["application_calls"]
    );
    let published: Vec<Published> =
        serde_json::from_value(received.result["published"].clone()).unwrap();
    assert_eq!(
        published.len() as u64,
        sent.result["operations"].as_u64().unwrap()
    );
    assert!(received.result["exact_reopened"].as_bool().unwrap());
    campaign::verify(&plan, &cache, &published, &mut Vec::new()).unwrap();
    let withheld = traffic.reverse.withheld.as_ref().unwrap();
    assert_eq!(withheld.ordinal, if missing_reply { 2 } else { 1 });
    assert_eq!(
        withheld.length,
        traffic.reverse.captured[(withheld.ordinal - 1) as usize].len()
    );
    assert_eq!(
        withheld.hash,
        spiralcore_transfer::hash(
            b"spiralcore.test.withheld.v1",
            &[&traffic.reverse.captured[(withheld.ordinal - 1) as usize]]
        )
    );
    assert_eq!(
        traffic.forward.captured[0],
        traffic.forward.captured[if missing_reply { 1 } else { 2 }]
    );
    if missing_reply {
        assert_eq!(traffic.reverse.captured[1], traffic.reverse.captured[3]);
    }
    for (endpoint, incoming, outgoing, handshake) in [
        (
            &sent.result,
            &traffic.reverse,
            &traffic.forward,
            &traffic.caller_handshake,
        ),
        (
            &received.result,
            &traffic.forward,
            &traffic.reverse,
            &traffic.receiver_handshake,
        ),
    ] {
        for (field, expected) in [
            ("frames_sent", outgoing.read.frames_received),
            ("frames_received", incoming.written.frames_sent),
            ("envelope_bytes_sent", outgoing.read.envelope_bytes_received),
            (
                "envelope_bytes_received",
                incoming.written.envelope_bytes_sent,
            ),
            (
                "framing_bytes_sent",
                outgoing.read.framing_bytes_received + handshake.framing_bytes_received,
            ),
            (
                "framing_bytes_received",
                incoming.written.framing_bytes_sent + handshake.framing_bytes_sent,
            ),
        ] {
            assert_eq!(
                endpoint["counters"][field].as_u64().unwrap(),
                expected,
                "{field}"
            );
        }
    }
    assert!(traffic.forward.terminal.is_some() && traffic.reverse.terminal.is_some());
    if let Some(directory) = std::env::var_os("SPIRALCORE_RECONCILIATION_EVIDENCE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        let metadata = std::fs::symlink_metadata(&directory).unwrap();
        assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
        let case = if missing_reply {
            "public-missing-reply"
        } else {
            "public-missing-call-receipt"
        };
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(format!("{case}.json")))
            .unwrap();
        serde_json::to_writer_pretty(file, &serde_json::json!({
            "evidence":"canonical-reconciliation-test/v1", "case":case,
            "scope":"Unix public embedding TCP full object publication/readback; not WAN/native MVM",
            "sender":sent.result, "receiver":received.result,
            "verified_publications":published.len(),
            "withheld":{"ordinal":withheld.ordinal,"envelope_bytes":withheld.length,
                "hash_domain":"spiralcore.test.withheld.v1", "hash":spiralcore_transfer::hex(&withheld.hash)},
            "gate":{"forward_incoming":traffic.forward.read,"forward_outgoing":traffic.forward.written,
                "reverse_incoming":traffic.reverse.read,"reverse_outgoing":traffic.reverse.written,
                "caller_handshake":traffic.caller_handshake,"receiver_handshake":traffic.receiver_handshake},
            "accounting":"Endpoint/processing/whole lifecycle observations overlap; no energy or performance claim"
        })).unwrap();
    }
}
