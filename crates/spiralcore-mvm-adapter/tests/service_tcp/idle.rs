use super::*;
use spiralcore_mvm_adapter::live::{LiveError, LiveResult, ReceiveOutcome, SocketIo};
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    net::Shutdown,
    sync::{Arc, Mutex, mpsc},
};

enum ReadStep {
    Bytes(Vec<u8>),
    Error(io::ErrorKind),
    Eof,
    At(Instant, Vec<u8>),
    Cancel(CancellationToken, Vec<u8>),
    Hold(mpsc::Sender<()>, mpsc::Receiver<Vec<u8>>),
}

#[derive(Default)]
struct FakeState {
    incoming: VecDeque<u8>,
    written: Vec<u8>,
    read_timeout: Option<Duration>,
    shutdowns: Vec<Shutdown>,
    script: VecDeque<ReadStep>,
    read_callers: Vec<thread::ThreadId>,
    timeouts: Vec<Duration>,
    fail_read_timeout: bool,
}

struct FakeSocket(Arc<Mutex<FakeState>>);

impl Read for FakeSocket {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let (step, timeout) = {
            let mut state = self.0.lock().unwrap();
            state.read_callers.push(thread::current().id());
            if !state.incoming.is_empty() {
                let count = bytes.len().min(state.incoming.len());
                for byte in &mut bytes[..count] {
                    *byte = state.incoming.pop_front().unwrap();
                }
                return Ok(count);
            }
            (
                state.script.pop_front(),
                state
                    .read_timeout
                    .ok_or_else(|| io::Error::other("missing bounded fake timeout"))?,
            )
        };
        let input = match step {
            Some(ReadStep::Bytes(data)) => data,
            Some(ReadStep::Error(kind)) => return Err(kind.into()),
            Some(ReadStep::Eof) => return Ok(0),
            Some(ReadStep::At(at, data)) => {
                assert!(at.saturating_duration_since(Instant::now()) < Duration::from_secs(5));
                while let Some(wait) = at.checked_duration_since(Instant::now()) {
                    thread::park_timeout(wait);
                }
                data
            }
            Some(ReadStep::Cancel(cancel, data)) => {
                cancel.cancel();
                data
            }
            Some(ReadStep::Hold(entered, release)) => {
                entered
                    .send(())
                    .map_err(|_| io::Error::other("lost hold observer"))?;
                release
                    .recv_timeout(Duration::from_secs(5))
                    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "fixture hold watchdog"))?
            }
            None => {
                // A retryable error alone is not silence: honor the installed positive
                // wait before returning TimedOut. All waits in this fixture are short.
                thread::sleep(timeout);
                return Err(io::ErrorKind::TimedOut.into());
            }
        };
        let count = bytes.len().min(input.len());
        bytes[..count].copy_from_slice(&input[..count]);
        self.0
            .lock()
            .unwrap()
            .incoming
            .extend(input[count..].iter().copied());
        Ok(count)
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
        let mut state = self.0.lock().unwrap();
        if state.fail_read_timeout {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        if let Some(timeout) = timeout {
            state.timeouts.push(timeout);
        }
        state.read_timeout = timeout;
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

fn unready_fake() -> (StreamTransport<FakeSocket>, Arc<Mutex<FakeState>>) {
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
    (stream, state)
}

fn ready_fake() -> (StreamTransport<FakeSocket>, Arc<Mutex<FakeState>>) {
    let (stream, state) = unready_fake();
    stream
        .handshake(HandshakeRole::Initiator, &CancellationToken::default())
        .unwrap();
    (stream, state)
}

fn frame() -> Vec<u8> {
    vec![0, 0, 0, 4, 0, 90, 0, 91]
}

fn authorized_write(
    stream: &impl mvm_core::mailbox::transport::PeerTransport,
) -> Result<usize, spiralcore_mvm_adapter::service::ServiceError> {
    let outgoing = fixture::binding(MailboxMessageKind::Call, true, false, true);
    let mut options =
        MessageOptions::fresh(fixture::content_type(), fixture::time(), 2000).unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    let prepared =
        service_sender::prepare(&outgoing, b"bounded writer", &options, fixture::time()).unwrap();
    outgoing.send_prepared(
        &prepared,
        stream,
        &CancellationToken::default(),
        fixture::time(),
    )
}

fn assert_fenced(stream: &StreamTransport<FakeSocket>, state: &Arc<Mutex<FakeState>>) {
    let before = serde_json::to_value(stream.counters().unwrap()).unwrap();
    let reads = state.lock().unwrap().read_callers.len();
    assert!(matches!(
        stream.receive_or_idle(
            &CancellationToken::default(),
            Instant::now() + Duration::from_secs(1),
            Duration::from_millis(1)
        ),
        Err(LiveError::Closed)
    ));
    assert!(matches!(
        stream.receive(
            &CancellationToken::default(),
            Instant::now() + Duration::from_secs(1)
        ),
        Err(LiveError::Closed)
    ));
    assert_eq!(
        authorized_write(stream),
        Err(spiralcore_mvm_adapter::service::ServiceError::ShuttingDown)
    );
    assert_eq!(
        serde_json::to_value(stream.counters().unwrap()).unwrap(),
        before
    );
    assert_eq!(state.lock().unwrap().read_callers.len(), reads);
    assert!(!state.lock().unwrap().shutdowns.is_empty());
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

#[test]
fn early_retryable_errors_are_not_a_completed_positive_idle_window() {
    let (stream, state) = ready_fake();
    state.lock().unwrap().script.extend([
        ReadStep::Error(io::ErrorKind::WouldBlock),
        ReadStep::Error(io::ErrorKind::TimedOut),
        ReadStep::Error(io::ErrorKind::Interrupted),
        ReadStep::Bytes(frame()),
    ]);
    let ReceiveOutcome::Frame(bytes) = stream
        .receive_or_idle(
            &CancellationToken::default(),
            Instant::now() + Duration::from_secs(2),
            Duration::from_secs(1),
        )
        .unwrap()
    else {
        panic!("early retryable error became Idle")
    };
    assert_eq!(bytes, [0, 90, 0, 91]);
    assert_eq!(stream.counters().unwrap().frames_received, 1);
    assert!(state.lock().unwrap().shutdowns.is_empty());
}

#[test]
fn partial_prefix_and_payload_ignore_idle_but_never_refresh_absolute_deadline() {
    for payload_phase in [false, true] {
        for expires in [false, true] {
            let (stream, state) = ready_fake();
            let initial_timeouts = state.lock().unwrap().timeouts.len();
            let bytes = frame();
            let first = if payload_phase { 5 } else { 1 };
            let anchor = Instant::now();
            let deadline = anchor
                + if expires {
                    Duration::from_millis(8)
                } else {
                    Duration::from_secs(2)
                };
            let at = if expires {
                deadline + Duration::from_millis(4)
            } else {
                anchor + Duration::from_millis(4)
            };
            let second = if expires { first + 1 } else { bytes.len() };
            state.lock().unwrap().script.extend([
                ReadStep::Bytes(bytes[..first].to_vec()),
                ReadStep::At(at, bytes[first..second].to_vec()),
                ReadStep::Bytes(bytes[second..].to_vec()),
            ]);
            let result = stream.receive_or_idle(
                &CancellationToken::default(),
                deadline,
                Duration::from_millis(1),
            );
            let counts = stream.counters().unwrap();
            if expires {
                assert!(matches!(result, Err(LiveError::Deadline)));
                assert_eq!(
                    counts.framing_bytes_received,
                    if payload_phase { 12 } else { 10 }
                );
                assert_eq!(
                    counts.envelope_bytes_received,
                    if payload_phase { 2 } else { 0 }
                );
                assert_eq!(counts.frames_received, 0);
                assert_fenced(&stream, &state);
            } else {
                let ReceiveOutcome::Frame(actual) = result.unwrap() else {
                    panic!("partial frame became Idle")
                };
                assert_eq!(actual, bytes[4..]);
                assert_eq!(
                    (
                        counts.framing_bytes_received,
                        counts.envelope_bytes_received,
                        counts.frames_received
                    ),
                    (12, 4, 1)
                );
                // A byte disables idle. The next timeout uses the remaining
                // ordinary frame lifetime, rather than the 1ms idle window.
                assert!(
                    state
                        .lock()
                        .unwrap()
                        .timeouts
                        .iter()
                        .skip(initial_timeouts + 1)
                        .any(|timeout| *timeout > Duration::from_millis(1))
                );
                assert!(state.lock().unwrap().shutdowns.is_empty());
            }
        }
    }
}

#[test]
fn owned_fatal_paths_fence_with_exact_partial_counters_and_no_extra_frame() {
    for mode in [
        "expired",
        "shorter",
        "equal",
        "cancelled",
        "cancel-over-expiry",
        "partial-cancel",
        "eof",
        "prefix1",
        "prefix2",
        "prefix3",
        "payload-eof",
        "zero-length",
        "oversize",
        "read-error",
        "timeout-error",
    ] {
        let (stream, state) = ready_fake();
        let cancel = CancellationToken::default();
        let mut deadline = Instant::now() + Duration::from_secs(1);
        let mut idle = Duration::from_secs(1);
        let mut prefix = 0;
        let mut payload = 0;
        match mode {
            "expired" => deadline = Instant::now() - Duration::from_millis(1),
            "shorter" | "equal" => {
                deadline = Instant::now() + Duration::from_millis(3);
                idle = if mode == "equal" {
                    Duration::from_millis(3)
                } else {
                    Duration::from_millis(10)
                };
            }
            "cancelled" => cancel.cancel(),
            "cancel-over-expiry" => {
                cancel.cancel();
                deadline = Instant::now() - Duration::from_millis(1);
            }
            "partial-cancel" => {
                prefix = 1;
                state
                    .lock()
                    .unwrap()
                    .script
                    .push_back(ReadStep::Cancel(cancel.clone(), vec![0]));
            }
            "eof" => state.lock().unwrap().script.push_back(ReadStep::Eof),
            "prefix1" | "prefix2" | "prefix3" => {
                prefix = mode.as_bytes()[6] as u64 - b'0' as u64;
                state
                    .lock()
                    .unwrap()
                    .script
                    .extend([ReadStep::Bytes(vec![0; prefix as usize]), ReadStep::Eof]);
            }
            "payload-eof" => {
                prefix = 4;
                payload = 1;
                state
                    .lock()
                    .unwrap()
                    .script
                    .extend([ReadStep::Bytes(vec![0, 0, 0, 4, 7]), ReadStep::Eof]);
            }
            "zero-length" => {
                prefix = 4;
                state
                    .lock()
                    .unwrap()
                    .script
                    .push_back(ReadStep::Bytes(vec![0; 4]));
            }
            "oversize" => {
                prefix = 4;
                state
                    .lock()
                    .unwrap()
                    .script
                    .push_back(ReadStep::Bytes(262145u32.to_be_bytes().to_vec()));
            }
            "read-error" => state
                .lock()
                .unwrap()
                .script
                .push_back(ReadStep::Error(io::ErrorKind::PermissionDenied)),
            "timeout-error" => state.lock().unwrap().fail_read_timeout = true,
            _ => unreachable!(),
        }
        let result = stream.receive_or_idle(&cancel, deadline, idle);
        match (mode, result) {
            ("expired" | "shorter" | "equal", Err(LiveError::Deadline)) => {}
            ("cancelled" | "cancel-over-expiry" | "partial-cancel", Err(LiveError::Cancelled)) => {}
            ("zero-length" | "oversize", Err(LiveError::Framing)) => {}
            (
                "eof" | "prefix1" | "prefix2" | "prefix3" | "payload-eof",
                Err(LiveError::Io(error)),
            ) if error.kind() == io::ErrorKind::UnexpectedEof => {}
            ("read-error" | "timeout-error", Err(LiveError::Io(error)))
                if error.kind() == io::ErrorKind::PermissionDenied => {}
            (_, result) => panic!(
                "unexpected fatal outcome for {mode}: {}",
                match result {
                    Ok(_) => "nonfatal result".into(),
                    Err(error) => error.to_string(),
                }
            ),
        }
        let counts = stream.counters().unwrap();
        assert_eq!(
            (
                counts.framing_bytes_received,
                counts.envelope_bytes_received,
                counts.frames_received
            ),
            (8 + prefix, payload, 0),
            "{mode}"
        );
        if mode == "zero-length" || mode == "oversize" {
            assert_eq!(
                state.lock().unwrap().read_callers.len(),
                2,
                "invalid length performed a payload read"
            );
        }
        assert_fenced(&stream, &state);
    }
}

#[test]
fn local_window_validation_precedes_state_without_fencing_and_boundaries_are_valid() {
    let (stream, state) = unready_fake();
    let cancel = CancellationToken::default();
    cancel.cancel();
    let before = serde_json::to_value(stream.counters().unwrap()).unwrap();
    for window in [
        Duration::ZERO,
        Duration::from_micros(999),
        Duration::from_secs(1) + Duration::from_nanos(1),
    ] {
        assert!(
            matches!(stream.receive_or_idle(&cancel, Instant::now()-Duration::from_millis(1),window), Err(LiveError::Io(error)) if error.kind()==io::ErrorKind::InvalidInput)
        );
    }
    assert_eq!(
        serde_json::to_value(stream.counters().unwrap()).unwrap(),
        before
    );
    assert!(state.lock().unwrap().read_callers.is_empty());
    assert!(state.lock().unwrap().shutdowns.is_empty());
    let cancel = CancellationToken::default();
    assert!(matches!(
        stream.receive_or_idle(
            &cancel,
            Instant::now() + Duration::from_secs(1),
            Duration::from_millis(1)
        ),
        Err(LiveError::Closed)
    ));
    assert!(state.lock().unwrap().read_callers.is_empty());
    stream.handshake(HandshakeRole::Initiator, &cancel).unwrap();
    for window in [Duration::from_millis(1), Duration::from_secs(1)] {
        state.lock().unwrap().incoming.extend(frame());
        assert!(matches!(
            stream
                .receive_or_idle(&cancel, Instant::now() + Duration::from_secs(2), window)
                .unwrap(),
            ReceiveOutcome::Frame(_)
        ));
    }
    authorized_write(&stream).unwrap();
    stream.close().unwrap();
    assert_fenced(&stream, &state);
}

struct HeldReader {
    release: Option<mpsc::Sender<Vec<u8>>>,
    worker: Option<thread::JoinHandle<LiveResult<Vec<u8>>>>,
}
impl HeldReader {
    fn finish(mut self) -> LiveResult<Vec<u8>> {
        if let Some(release) = self.release.take() {
            let _ = release.send(frame());
        }
        self.worker
            .take()
            .unwrap()
            .join()
            .expect("held reader panicked")
    }
}
impl Drop for HeldReader {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(frame());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn held_reader() -> (
    Arc<StreamTransport<FakeSocket>>,
    Arc<Mutex<FakeState>>,
    HeldReader,
) {
    let (stream, state) = ready_fake();
    let stream = Arc::new(stream);
    let (entered_tx, entered) = mpsc::channel();
    let (release, held) = mpsc::channel();
    state
        .lock()
        .unwrap()
        .script
        .push_back(ReadStep::Hold(entered_tx, held));
    let receiver = Arc::clone(&stream);
    let worker = thread::spawn(move || {
        receiver.receive(
            &CancellationToken::default(),
            Instant::now() + Duration::from_secs(2),
        )
    });
    let guard = HeldReader {
        release: Some(release),
        worker: Some(worker),
    };
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    (stream, state, guard)
}

#[test]
fn lock_wait_never_becomes_idle_and_idle_clock_starts_after_acquisition() {
    for cancelled in [false, true] {
        let (stream, state, held) = held_reader();
        let cancel = CancellationToken::default();
        if cancelled {
            cancel.cancel();
        }
        let reads = state.lock().unwrap().read_callers.len();
        let result = stream.receive_or_idle(
            &cancel,
            Instant::now() + Duration::from_millis(10),
            Duration::from_millis(1),
        );
        assert!(matches!(
            (cancelled, result),
            (true, Err(LiveError::Cancelled)) | (false, Err(LiveError::Deadline))
        ));
        assert_eq!(state.lock().unwrap().read_callers.len(), reads);
        // Existing lock semantics: the unsuccessful contender never owned the
        // state and does not globally close the independent current reader.
        assert!(state.lock().unwrap().shutdowns.is_empty());
        assert_eq!(held.finish().unwrap(), frame()[4..]);
        authorized_write(stream.as_ref()).unwrap();
    }
    let (stream, state, mut held) = held_reader();
    let release = held.release.take().unwrap();
    let caller = thread::current().id();
    let before = state
        .lock()
        .unwrap()
        .read_callers
        .iter()
        .filter(|id| **id == caller)
        .count();
    let outcome = thread::scope(|scope| {
        let releaser = scope.spawn(move || {
            thread::sleep(Duration::from_millis(5));
            let _ = release.send(frame());
        });
        let outcome = stream.receive_or_idle(
            &CancellationToken::default(),
            Instant::now() + Duration::from_secs(2),
            Duration::from_millis(1),
        );
        releaser.join().unwrap();
        outcome
    });
    assert_eq!(held.finish().unwrap(), frame()[4..]);
    assert!(matches!(outcome.unwrap(), ReceiveOutcome::Idle));
    // A full positive socket poll occurs after lock acquisition; elapsed lock
    // wait cannot cause an immediate, unobserved Idle decision.
    let after = state
        .lock()
        .unwrap()
        .read_callers
        .iter()
        .filter(|id| **id == caller)
        .count();
    assert!(after > before);
    assert!(state.lock().unwrap().shutdowns.is_empty());
}

struct TcpPeer {
    local: TcpStream,
    remote: TcpStream,
    release: mpsc::Sender<()>,
    worker: Option<thread::JoinHandle<LiveResult<spiralcore_mvm_adapter::live::StreamCounters>>>,
}
impl TcpPeer {
    fn finish(mut self) -> LiveResult<spiralcore_mvm_adapter::live::StreamCounters> {
        self.worker
            .take()
            .unwrap()
            .join()
            .expect("TCP peer panicked")
    }
}
impl Drop for TcpPeer {
    fn drop(&mut self) {
        let _ = self.release.send(());
        let _ = self.local.shutdown(Shutdown::Both);
        let _ = self.remote.shutdown(Shutdown::Both);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[test]
fn real_tcp_idle_then_later_exact_frame_and_authorized_reverse_write() {
    use spiralcore_mvm_adapter::live::{self, StreamCounters};
    let outgoing = fixture::binding(MailboxMessageKind::Call, true, false, true);
    let mut options =
        MessageOptions::fresh(fixture::content_type(), fixture::time(), 2000).unwrap();
    options.reply_to = common::config(MailboxMessageKind::Call).reply_to;
    let prepared =
        service_sender::prepare(&outgoing, b"TCP after idle\0", &options, fixture::time()).unwrap();
    let expected_wire = encode_envelope(prepared.envelope()).unwrap();
    let wire_len = expected_wire.len();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let socket =
        TcpStream::connect_timeout(&listener.local_addr().unwrap(), Duration::from_secs(2))
            .unwrap();
    let (mut peer, _) = listener.accept().unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    peer.set_write_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let local = socket.try_clone().unwrap();
    let remote = peer.try_clone().unwrap();
    let (release, go) = mpsc::channel();
    let worker = thread::spawn(move || -> LiveResult<StreamCounters> {
        let cancel = CancellationToken::default();
        let mut counters = StreamCounters::default();
        live::handshake_stream(
            &mut peer,
            HandshakeRole::Acceptor,
            &cancel,
            Instant::now() + Duration::from_secs(2),
            &mut counters,
        )?;
        go.recv_timeout(Duration::from_secs(5))
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "TCP fixture watchdog"))?;
        live::write_ciphertext_frame(
            &mut peer,
            b"\0later\0",
            &cancel,
            Instant::now() + Duration::from_secs(2),
            &mut counters,
        )?;
        let actual = live::read_ciphertext_frame(
            &mut peer,
            &cancel,
            Instant::now() + Duration::from_secs(2),
            &mut counters,
        )?;
        if actual != expected_wire {
            return Err(LiveError::Framing);
        }
        Ok(counters)
    });
    let guard = TcpPeer {
        local,
        remote,
        release,
        worker: Some(worker),
    };
    let stream = StreamTransport::new(
        socket,
        common::config(MailboxMessageKind::Call)
            .binding
            .claims
            .route_id,
        Duration::from_secs(2),
    )
    .unwrap();
    let cancel = CancellationToken::default();
    stream.handshake(HandshakeRole::Initiator, &cancel).unwrap();
    let before = serde_json::to_value(stream.counters().unwrap()).unwrap();
    assert!(matches!(
        stream
            .receive_or_idle(
                &cancel,
                Instant::now() + Duration::from_secs(2),
                Duration::from_millis(2)
            )
            .unwrap(),
        ReceiveOutcome::Idle
    ));
    assert_eq!(
        serde_json::to_value(stream.counters().unwrap()).unwrap(),
        before
    );
    guard.release.send(()).unwrap();
    outgoing
        .send_prepared(&prepared, &stream, &cancel, fixture::time())
        .unwrap();
    let ReceiveOutcome::Frame(bytes) = stream
        .receive_or_idle(
            &cancel,
            Instant::now() + Duration::from_secs(2),
            Duration::from_secs(1),
        )
        .unwrap()
    else {
        panic!("released TCP frame became Idle")
    };
    assert_eq!(bytes, b"\0later\0");
    let own = stream.counters().unwrap();
    let peer = guard.finish().unwrap();
    assert_eq!((own.frames_sent, own.frames_received), (1, 1));
    assert_eq!((peer.frames_sent, peer.frames_received), (1, 1));
    assert_eq!(own.envelope_bytes_sent, wire_len as u64);
    assert_eq!(own.envelope_bytes_received, b"\0later\0".len() as u64);
    assert_eq!(own.envelope_bytes_sent, peer.envelope_bytes_received);
    assert_eq!(own.envelope_bytes_received, peer.envelope_bytes_sent);
    assert_eq!(
        (own.framing_bytes_sent, own.framing_bytes_received),
        (12, 12)
    );
    assert_eq!(
        (peer.framing_bytes_sent, peer.framing_bytes_received),
        (12, 12)
    );
}

#[test]
fn recovery_distinguishes_clean_fin_from_truncated_tcp_frames() {
    let cancel = CancellationToken::default();
    for bytes in [&[][..], &[0, 0][..], &[0, 0, 0, 4, 7][..]] {
        let (stream, mut peer) = recovery_tcp_pair();
        let before = stream.counters().unwrap();
        peer.write_all(bytes).unwrap();
        peer.shutdown(Shutdown::Write).unwrap();

        let deadline = Instant::now() + Duration::from_secs(2);
        let outcome =
            stream.receive_or_idle_observing_eof(&cancel, deadline, Duration::from_millis(250));
        if bytes.is_empty() {
            assert!(matches!(outcome, Ok(None)), "{outcome:?}");
        } else {
            assert!(
                matches!(outcome, Err(LiveError::Io(e)) if e.kind() == io::ErrorKind::UnexpectedEof)
            );
        }
        let after = stream.counters().unwrap();
        assert_eq!(after.frames_received, before.frames_received);
        assert_eq!(
            after.framing_bytes_received - before.framing_bytes_received,
            bytes.len().min(4) as u64
        );
        assert_eq!(
            after.envelope_bytes_received - before.envelope_bytes_received,
            bytes.len().saturating_sub(4) as u64
        );
        assert!(matches!(
            stream.receive_or_idle_observing_eof(&cancel, deadline, Duration::from_millis(250)),
            Err(LiveError::Closed)
        ));
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        assert_eq!(peer.read(&mut [0]).unwrap(), 0);
    }
}

fn recovery_tcp_pair() -> (StreamTransport<TcpStream>, TcpStream) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let mut peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (socket, _) = listener.accept().unwrap();
    peer.write_all(b"SCMVS001").unwrap();
    let route = common::config(MailboxMessageKind::Call)
        .binding
        .claims
        .route_id;
    let stream = StreamTransport::new(socket, route, Duration::from_secs(2)).unwrap();
    stream
        .handshake(HandshakeRole::Acceptor, &CancellationToken::default())
        .unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut discriminator = [0; 8];
    peer.read_exact(&mut discriminator).unwrap();
    assert_eq!(&discriminator, b"SCMVS001");
    (stream, peer)
}

#[test]
fn recovery_fin_fences_an_existing_writer_and_strict_receive_stays_an_error() {
    let cancel = CancellationToken::default();
    let (stream, peer) = recovery_tcp_pair();
    let deadline = Instant::now() + Duration::from_secs(2);
    let writer = stream.before(deadline);
    peer.shutdown(Shutdown::Write).unwrap();
    assert!(matches!(
        stream.receive_or_idle_observing_eof(&cancel, deadline, Duration::from_millis(250)),
        Ok(None)
    ));
    assert_eq!(
        authorized_write(&writer),
        Err(spiralcore_mvm_adapter::service::ServiceError::ShuttingDown)
    );

    let (strict, peer) = recovery_tcp_pair();
    peer.shutdown(Shutdown::Write).unwrap();
    assert!(matches!(
        strict.receive_or_idle(&cancel, deadline, Duration::from_millis(250)),
        Err(LiveError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof
    ));
}

#[test]
fn cancellation_and_expiry_precede_recovery_fin() {
    for cancelled in [true, false] {
        let (stream, peer) = recovery_tcp_pair();
        peer.shutdown(Shutdown::Write).unwrap();
        let cancel = CancellationToken::default();
        let mut deadline = Instant::now();
        if cancelled {
            cancel.cancel();
            deadline += Duration::from_secs(2);
        }
        let result =
            stream.receive_or_idle_observing_eof(&cancel, deadline, Duration::from_millis(250));
        assert!(matches!(
            (cancelled, result),
            (true, Err(LiveError::Cancelled)) | (false, Err(LiveError::Deadline))
        ));
    }
}
