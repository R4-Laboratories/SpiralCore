use super::*;
use crate::config::{Arm, Pair, Plan, Stage};
use std::{
    net::Shutdown,
    os::unix::net::UnixStream,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
};

const WATCHDOG: Duration = Duration::from_secs(5);

// The peer sends Reply first. Count the eight readiness bytes and complete
// first frame; fail only subsequent writes, so handshake/Call always delegate.
struct FailReplyAck {
    socket: UnixStream,
    read_bytes: usize,
    prefix: [u8; 4],
    first_frame_len: Option<usize>,
    armed: bool,
    receipt_queued: mpsc::Receiver<()>,
    failed_writes: Arc<AtomicUsize>,
}

impl Read for FailReplyAck {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let count = self.socket.read(bytes)?;
        for (offset, byte) in bytes[..count].iter().enumerate() {
            let position = self.read_bytes + offset;
            if (8..12).contains(&position) {
                self.prefix[position - 8] = *byte;
            }
        }
        self.read_bytes += count;
        if self.read_bytes >= 12 && self.first_frame_len.is_none() {
            self.first_frame_len = Some(u32::from_be_bytes(self.prefix) as usize);
        }
        if !self.armed
            && self
                .first_frame_len
                .is_some_and(|len| self.read_bytes == 12 + len)
        {
            // Do not race ACK failure against the peer's following canonical
            // Call receipt. Both frames are queued before this read completes.
            self.receipt_queued
                .recv_timeout(WATCHDOG)
                .map_err(|error| {
                    io::Error::new(io::ErrorKind::TimedOut, format!("fixture receipt: {error}"))
                })?;
            self.armed = true;
        }
        Ok(count)
    }
}

impl Write for FailReplyAck {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.armed {
            self.failed_writes.fetch_add(1, Ordering::Relaxed);
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        self.socket.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.socket.flush()
    }
}

impl SocketIo for FailReplyAck {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        SocketIo::set_read_timeout(&self.socket, timeout)
    }

    fn set_write_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        SocketIo::set_write_timeout(&self.socket, timeout)
    }

    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.socket.shutdown(how)
    }
}

struct PeerWires {
    call: Vec<u8>,
    reply: Vec<u8>,
}

struct PeerGuard {
    caller_socket: UnixStream,
    peer_socket: UnixStream,
    worker: Option<std::thread::JoinHandle<Result<PeerWires>>>,
}

impl PeerGuard {
    fn stop(&self) {
        let _ = self.caller_socket.shutdown(Shutdown::Both);
        let _ = self.peer_socket.shutdown(Shutdown::Both);
    }

    fn finish(mut self) -> Result<PeerWires> {
        self.stop();
        self.worker.take().unwrap().join().expect("peer panicked")
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
fn failed_reply_ack_retains_exact_service_payload_and_original_pending_identity() {
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
    let request = Frame {
        kind: 6,
        transfer: [37; 32],
        seq: 0,
        body: vec![0, 9, 0],
    };
    let expected = request.reply(101, vec![8, 0, 7, 0, 6]);
    let reply_payload = expected.encode().unwrap();
    let peer_payload = reply_payload.clone();
    let expected_request = request.encode().unwrap();
    let (socket, peer_socket) = UnixStream::pair().unwrap();
    let caller_shutdown = socket.try_clone().unwrap();
    let peer_shutdown = peer_socket.try_clone().unwrap();
    let failed_writes = Arc::new(AtomicUsize::new(0));
    let (receipt_tx, receipt_queued) = mpsc::channel();
    let stream = StreamTransport::new(
        FailReplyAck {
            socket,
            read_bytes: 0,
            prefix: [0; 4],
            first_frame_len: None,
            armed: false,
            receipt_queued,
            failed_writes: Arc::clone(&failed_writes),
        },
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
    let worker = std::thread::spawn(move || -> Result<PeerWires> {
        let cancel = CancellationToken::default();
        peer_stream.handshake(HandshakeRole::Acceptor, &cancel)?;
        let original = peer_stream.receive(&cancel, Instant::now() + WATCHDOG)?;
        let (receipt, proof) = peer_broker.with_time(|_, now| -> Result<_> {
            let (accepted, receipt) =
                peer.returns
                    .admit(&mut peer.service, &peer.incoming, &original, now)?;
            if accepted.receipt.outcome != DeliveryOutcome::AcceptedEphemeral {
                return Err("fixture Call was not accepted".into());
            }
            let proof = peer
                .service
                .receive(&peer.incoming, now)?
                .ok_or("missing fixture proof")?;
            Ok((receipt, proof))
        })?;
        if proof.payload() != expected_request {
            return Err("fixture received a changed Call".into());
        }
        let (prepared, _send, now, reply_wire) = peer_broker.with_time(|_, now| -> Result<_> {
            let options = peer
                .ordering
                .next_reply(&peer.outgoing, &proof, now, ISSUANCE_POLICY)?;
            let prepared = peer.service.reply_prepared(
                &peer.outgoing,
                proof.handle(),
                service_sender::request(&peer.outgoing, &peer_payload, &options, now)?,
                now,
            )?;
            let reply_wire = encode_envelope(prepared.envelope())?;
            let send = peer.tracker.register_send(
                &peer.outgoing,
                &peer.control,
                &peer.peer_key,
                &reply_wire,
                now,
            )?;
            Ok((prepared, send, now, reply_wire))
        })?;
        peer.outgoing
            .send_prepared(&prepared, &peer_stream, &cancel, now)?;
        let (permit, now) = peer_broker
            .with_time(|_, now| -> Result<_> { Ok((receipt.reserve_send(&cancel, now)?, now)) })?;
        permit.send(&peer_stream, &cancel, now)?;
        receipt_tx.send(())?;
        // Remain open until the caller's injected write failure closes it.
        // No half-close or scheduling race is used to induce the failure.
        if peer_stream
            .receive(&cancel, Instant::now() + WATCHDOG)
            .is_ok()
        {
            return Err("injected ACK failure nevertheless transmitted a frame".into());
        }
        let counters = peer_stream.counters()?;
        if counters.frames_received != 1 || counters.frames_sent != 2 {
            return Err("fixture did not exchange exactly one Call and two peer frames".into());
        }
        Ok(PeerWires {
            call: original,
            reply: reply_wire,
        })
    });
    let guard = PeerGuard {
        caller_socket: caller_shutdown,
        peer_socket: peer_shutdown,
        worker: Some(worker),
    };
    let cancel = CancellationToken::default();
    stream.handshake(HandshakeRole::Initiator, &cancel).unwrap();
    let error = caller.call(&request, &stream, &cancel).unwrap_err();
    let PeerWires {
        call: original,
        reply: authenticated_reply,
    } = guard.finish().unwrap();
    assert_eq!(failed_writes.load(Ordering::Relaxed), 1, "{error}");
    assert_eq!(caller.calls, 0);
    let pending = caller
        .pending
        .as_ref()
        .expect("ACK failure discarded pending Call");
    assert!(pending.send_started());
    assert_eq!(pending.encoded().unwrap(), original);
    assert_eq!(pending.prepared().unwrap().header().sequence.get(), 1);
    assert_eq!(pending.reply_wire(), Some(authenticated_reply.as_slice()));
    assert_eq!(pending.events_received(), 1);
    assert!(pending.reply_observed());
    assert_eq!(
        pending.reply_ack_state(),
        crate::campaign::session::ReplyAckState::InvokedUncertain,
    );
    // Reply was first. Its following Call receipt is queued but unread because
    // the injected ACK failure returns before the next receive operation.
    assert_eq!(pending.receipt_outcome(), None);
    let original_call = pending.call_handle().unwrap();
    let status = broker
        .with_time(|_, now| -> Result<_> {
            Ok(caller
                .service
                .poll_call(&caller.outgoing, &original_call, now)?)
        })
        .unwrap();
    assert_eq!(
        status,
        CallStatus::Replied(Some(reply_payload)),
        "ACK failure consumed the only exact Reply payload"
    );
    assert_eq!(
        caller.pending.as_ref().unwrap().call_handle(),
        Some(original_call)
    );
    assert_eq!(
        caller.pending.as_ref().unwrap().encoded().unwrap(),
        original
    );
}
