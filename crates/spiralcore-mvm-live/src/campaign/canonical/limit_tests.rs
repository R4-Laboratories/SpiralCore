use super::*;
use crate::config::{Arm, Pair, Plan, Stage};
use std::{net::Shutdown, os::unix::net::UnixStream, sync::mpsc};

const WATCHDOG: Duration = Duration::from_secs(5);

struct PeerReport {
    call_wire: Vec<u8>,
    receipt_wire_len: usize,
    counters: StreamCounters,
}

struct PeerGuard {
    caller_socket: UnixStream,
    peer_socket: UnixStream,
    done: mpsc::Receiver<()>,
    worker: Option<std::thread::JoinHandle<Result<PeerReport>>>,
}

impl PeerGuard {
    fn stop(&self) {
        let _ = self.caller_socket.shutdown(Shutdown::Both);
        let _ = self.peer_socket.shutdown(Shutdown::Both);
    }

    fn finish(mut self) -> (Result<PeerReport>, bool) {
        let completed = self.done.recv_timeout(WATCHDOG).is_ok();
        self.stop();
        let result = self.worker.take().unwrap().join().expect("peer panicked");
        (result, completed)
    }
}

impl Drop for PeerGuard {
    fn drop(&mut self) {
        self.stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[test]
fn authenticated_receipt_flood_stops_at_sixteen_events_and_retains_original_call() {
    let pair = Pair::provision(Plan {
        stage: Stage::Two,
        arm: Arm::Raw,
        size: 128,
        trial: 0,
        bandwidth_profile: Default::default(),
    })
    .unwrap();
    let broker = broker_context().unwrap();
    let mut caller = Session::new(&pair.sender, &broker).unwrap();
    let peer_broker = broker_context().unwrap();
    let mut peer = Session::new(&pair.receiver, &peer_broker).unwrap();
    let (socket, peer_socket) = UnixStream::pair().unwrap();
    let caller_shutdown = socket.try_clone().unwrap();
    let peer_shutdown = peer_socket.try_clone().unwrap();
    let stream = StreamTransport::new(
        socket,
        pair.sender
            .outgoing
            .as_ref()
            .unwrap()
            .binding
            .claims
            .route_id,
        WATCHDOG,
    )
    .unwrap();
    let peer_stream = StreamTransport::new(
        peer_socket,
        pair.receiver
            .outgoing
            .as_ref()
            .unwrap()
            .binding
            .claims
            .route_id,
        WATCHDOG,
    )
    .unwrap();
    let (done_tx, done) = mpsc::channel();
    let worker = std::thread::spawn(move || -> Result<PeerReport> {
        let result = (|| -> Result<PeerReport> {
            let cancel = CancellationToken::default();
            peer_stream.handshake(HandshakeRole::Acceptor, &cancel)?;
            let call_wire = peer_stream.receive(&cancel, Instant::now() + WATCHDOG)?;
            let receipt = peer_broker.with_time(|_, now| -> Result<_> {
                let (accepted, receipt) =
                    peer.returns
                        .admit(&mut peer.service, &peer.incoming, &call_wire, now)?;
                if accepted.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
                    return Err("fixture Call was not admitted exactly once".into());
                }
                Ok(receipt)
            })?;
            let receipt_wire_len = receipt.encoded_bytes().len();
            for _ in 0..17 {
                // Retransmit the exact canonical encrypted receipt. Each send
                // reserves its existing signed quota; no new identity/grant.
                let (permit, now) = peer_broker.with_time(|_, now| -> Result<_> {
                    Ok((receipt.reserve_send(&cancel, now)?, now))
                })?;
                permit.send(&peer_stream, &cancel, now)?;
            }
            Ok(PeerReport {
                call_wire,
                receipt_wire_len,
                counters: peer_stream.counters()?,
            })
        })();
        let _ = done_tx.send(());
        result
    });
    let guard = PeerGuard {
        caller_socket: caller_shutdown,
        peer_socket: peer_shutdown,
        done,
        worker: Some(worker),
    };
    let cancel = CancellationToken::default();
    stream.handshake(HandshakeRole::Initiator, &cancel).unwrap();
    let request = Frame {
        kind: 6,
        transfer: [43; 32],
        seq: 0,
        body: Vec::new(),
    };
    let error = caller.call(&request, &stream, &cancel).unwrap_err();
    let counters = stream.counters().unwrap();
    let (peer_result, completed) = guard.finish();
    assert!(
        completed,
        "peer failed to queue all seventeen bounded receipt frames"
    );
    let report = peer_result.expect("canonical signed/control quota refused receipt flood");
    assert_eq!(
        error.downcast_ref::<SessionError>(),
        Some(&SessionError::EventLimit)
    );
    assert_eq!(caller.calls, 0);
    let pending = caller
        .pending
        .as_ref()
        .expect("event limit discarded pending Call");
    assert!(pending.send_started());
    assert_eq!(pending.encoded().unwrap(), report.call_wire);
    assert_eq!(pending.prepared().unwrap().header().sequence.get(), 1);
    assert!(pending.call_handle().is_some());
    assert!(pending.send_handle().is_some());
    assert_eq!(pending.events_received(), 16);
    assert_eq!(
        pending.receipt_outcome(),
        Some(DeliveryOutcome::AcceptedEphemeral)
    );
    assert!(!pending.reply_observed());
    assert_eq!(pending.reply_wire(), None);
    assert_eq!(
        pending.reply_ack_state(),
        crate::campaign::session::ReplyAckState::None
    );
    // The seventeenth frame was actually transmitted but intentionally remains
    // unread. Exactly one original request and no Reply ACK were transmitted.
    assert_eq!((counters.frames_sent, counters.frames_received), (1, 16));
    assert_eq!(
        (report.counters.frames_sent, report.counters.frames_received),
        (17, 1)
    );
    assert_eq!(counters.envelope_bytes_sent, report.call_wire.len() as u64);
    assert_eq!(
        report.counters.envelope_bytes_received,
        report.call_wire.len() as u64
    );
    assert_eq!(
        counters.envelope_bytes_received,
        16 * report.receipt_wire_len as u64
    );
    assert_eq!(
        report.counters.envelope_bytes_sent,
        17 * report.receipt_wire_len as u64
    );
    assert_eq!(counters.framing_bytes_sent, 8 + 4);
    assert_eq!(counters.framing_bytes_received, 8 + 16 * 4);
    assert_eq!(report.counters.framing_bytes_sent, 8 + 17 * 4);
    assert_eq!(report.counters.framing_bytes_received, 8 + 4);
}
