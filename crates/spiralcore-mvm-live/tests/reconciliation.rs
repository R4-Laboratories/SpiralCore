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
            sent.result["counters"][send],
            received.result["counters"][receive]
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
