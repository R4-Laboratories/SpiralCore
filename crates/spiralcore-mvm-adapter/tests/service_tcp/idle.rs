use super::*;
use spiralcore_mvm_adapter::live::{ReceiveOutcome, SocketIo};
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    net::Shutdown,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct FakeState {
    incoming: VecDeque<u8>,
    written: Vec<u8>,
    read_timeout: Option<Duration>,
    shutdowns: Vec<Shutdown>,
}

struct FakeSocket(Arc<Mutex<FakeState>>);

impl Read for FakeSocket {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let timeout = {
            let mut state = self.0.lock().unwrap();
            if !state.incoming.is_empty() {
                let count = bytes.len().min(state.incoming.len());
                for byte in &mut bytes[..count] {
                    *byte = state.incoming.pop_front().unwrap();
                }
                return Ok(count);
            }
            state
                .read_timeout
                .ok_or_else(|| io::Error::other("missing bounded fake timeout"))?
        };
        // A retryable error alone is not silence: honor the installed positive
        // wait before returning TimedOut. All waits in this fixture are short.
        thread::sleep(timeout);
        Err(io::ErrorKind::TimedOut.into())
    }
}

impl Write for FakeSocket {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().written.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl SocketIo for FakeSocket {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.0.lock().unwrap().read_timeout = timeout;
        Ok(())
    }
    fn set_write_timeout(&self, _timeout: Option<Duration>) -> io::Result<()> {
        Ok(())
    }
    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.0.lock().unwrap().shutdowns.push(how);
        Ok(())
    }
}

fn ready_fake() -> (StreamTransport<FakeSocket>, Arc<Mutex<FakeState>>) {
    let state = Arc::new(Mutex::new(FakeState {
        incoming: b"SCMVS001".iter().copied().collect(),
        ..FakeState::default()
    }));
    let stream = StreamTransport::new(
        FakeSocket(Arc::clone(&state)),
        common::config(MailboxMessageKind::Call)
            .binding
            .claims
            .route_id,
        Duration::from_secs(2),
    )
    .unwrap();
    stream
        .handshake(HandshakeRole::Initiator, &CancellationToken::default())
        .unwrap();
    (stream, state)
}

#[test]
fn zero_prefix_idle_preserves_counters_then_authorized_writer_and_exact_frame_work() {
    let (stream, state) = ready_fake();
    let cancel = CancellationToken::default();
    let before = serde_json::to_value(stream.counters().unwrap()).unwrap();
    assert!(matches!(
        stream
            .receive_or_idle(
                &cancel,
                Instant::now() + Duration::from_secs(2),
                Duration::from_millis(2)
            )
            .unwrap(),
        ReceiveOutcome::Idle,
    ));
    assert_eq!(
        serde_json::to_value(stream.counters().unwrap()).unwrap(),
        before
    );
    assert!(state.lock().unwrap().shutdowns.is_empty());
    let outgoing = fixture::binding(MailboxMessageKind::Call, true, false, true);
    let mut options =
        MessageOptions::fresh(fixture::content_type(), fixture::time(), 2000).unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    let prepared = service_sender::prepare(
        &outgoing,
        b"authorized after idle\0",
        &options,
        fixture::time(),
    )
    .unwrap();
    let wire = encode_envelope(prepared.envelope()).unwrap();
    assert_eq!(
        outgoing
            .send_prepared(&prepared, &stream, &cancel, fixture::time())
            .unwrap(),
        wire.len()
    );
    state
        .lock()
        .unwrap()
        .incoming
        .extend([0, 0, 0, 3, 0, 90, 0]);
    let ReceiveOutcome::Frame(actual) = stream
        .receive_or_idle(
            &cancel,
            Instant::now() + Duration::from_secs(2),
            Duration::from_secs(1),
        )
        .unwrap()
    else {
        panic!("queued frame was classified as Idle")
    };
    assert_eq!(actual, [0, 90, 0]);
    let counters = stream.counters().unwrap();
    assert_eq!((counters.frames_sent, counters.frames_received), (1, 1));
    assert_eq!(counters.envelope_bytes_sent, wire.len() as u64);
    assert_eq!(counters.envelope_bytes_received, 3);
    assert_eq!(
        (counters.framing_bytes_sent, counters.framing_bytes_received),
        (12, 12)
    );
    let written = &state.lock().unwrap().written;
    assert_eq!(&written[..8], b"SCMVS001");
    assert_eq!(&written[8..12], &(wire.len() as u32).to_be_bytes());
    assert_eq!(&written[12..], wire);
}
