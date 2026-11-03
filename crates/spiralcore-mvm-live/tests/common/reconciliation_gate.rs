//! Test-only complete-ciphertext gate. Receiving is real traffic on the incoming
//! leg; deliberately withheld ciphertext contributes zero outgoing-leg bytes.
use mvm_core::mailbox::transport::CancellationToken;
use spiralcore_mvm_adapter::live::{self, HandshakeRole, StreamCounters};
use std::{
    net::{Shutdown, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};

const WATCHDOG: Duration = Duration::from_secs(5);
const CAPTURED_FRAMES: usize = 4;
const FRAME_LIMIT: u64 = 512;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Debug)]
pub struct Withheld {
    pub ordinal: u64,
    pub length: usize,
    pub hash: [u8; 32],
}
#[derive(Clone, Debug, Default)]
pub struct Direction {
    pub read: StreamCounters,
    pub written: StreamCounters,
    pub captured: Vec<Vec<u8>>,
    pub withheld: Option<Withheld>,
    pub terminal: Option<String>,
}
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub forward: Direction,
    pub reverse: Direction,
    pub caller_handshake: StreamCounters,
    pub receiver_handshake: StreamCounters,
}
pub type Shared = Arc<Mutex<Snapshot>>;

pub fn tcp_pair() -> Result<[TcpStream; 2]> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let a = TcpStream::connect_timeout(&listener.local_addr()?, WATCHDOG)?;
    let (b, _) = listener.accept()?;
    for socket in [&a, &b] {
        socket.set_nodelay(true)?;
        socket.set_read_timeout(Some(Duration::from_millis(50)))?;
        socket.set_write_timeout(Some(Duration::from_millis(50)))?;
    }
    Ok([a, b])
}

pub struct Gate {
    shared: Shared,
    cancel: CancellationToken,
    shutdown: [TcpStream; 2],
    worker: Option<JoinHandle<Result<()>>>,
}
impl Gate {
    pub fn start_missing_first_receipt() -> Result<(TcpStream, TcpStream, Self)> {
        Self::start_withheld(None, Some(1))
    }
    pub fn start_withheld(
        forward_withheld: Option<u64>,
        reverse_withheld: Option<u64>,
    ) -> Result<(TcpStream, TcpStream, Self)> {
        let [caller, mut a] = tcp_pair()?;
        let [mut b, receiver] = tcp_pair()?;
        let shutdown = [a.try_clone()?, b.try_clone()?];
        let shared = Shared::default();
        let state = shared.clone();
        let cancel = CancellationToken::default();
        let token = cancel.clone();
        let worker = std::thread::spawn(move || {
            let deadline = Instant::now() + WATCHDOG;
            let mut ac = StreamCounters::default();
            let mut bc = StreamCounters::default();
            live::handshake_stream(&mut a, HandshakeRole::Acceptor, &token, deadline, &mut ac)?;
            live::handshake_stream(&mut b, HandshakeRole::Initiator, &token, deadline, &mut bc)?;
            {
                let mut snapshot = state.lock().unwrap();
                snapshot.caller_handshake = ac;
                snapshot.receiver_handshake = bc;
            }
            let aw = a.try_clone()?;
            let bw = b.try_clone()?;
            std::thread::scope(|scope| {
                let forward =
                    scope.spawn(|| pump(a, bw, false, forward_withheld, &state, &token, deadline));
                let reverse =
                    scope.spawn(|| pump(b, aw, true, reverse_withheld, &state, &token, deadline));
                forward.join().expect("forward fault gate panicked");
                reverse.join().expect("reverse fault gate panicked");
            });
            Ok(())
        });
        Ok((
            caller,
            receiver,
            Self {
                shared,
                cancel,
                shutdown,
                worker: Some(worker),
            },
        ))
    }
    pub fn shared(&self) -> Shared {
        self.shared.clone()
    }
    pub fn snapshot(&self) -> Snapshot {
        self.shared.lock().unwrap().clone()
    }
    pub fn stop(&self) {
        self.cancel.cancel();
        for socket in &self.shutdown {
            if socket.shutdown(Shutdown::Both).is_err() {
                let _ = socket.shutdown(Shutdown::Write);
                let _ = socket.shutdown(Shutdown::Read);
            }
        }
    }
    pub fn finish(mut self) -> Result<Snapshot> {
        self.stop();
        self.worker
            .take()
            .unwrap()
            .join()
            .expect("fault gate panicked")?;
        Ok(self.snapshot())
    }
}
impl Drop for Gate {
    fn drop(&mut self) {
        self.stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn pump(
    mut input: TcpStream,
    mut output: TcpStream,
    reverse: bool,
    withhold: Option<u64>,
    shared: &Shared,
    cancel: &CancellationToken,
    deadline: Instant,
) {
    let mut read = StreamCounters::default();
    let mut written = StreamCounters::default();
    let result = (|| -> Result<()> {
        loop {
            if read.frames_received >= FRAME_LIMIT {
                return Err("test fault-gate frame bound".into());
            }
            let Some(wire) =
                live::read_ciphertext_frame_or_eof(&mut input, cancel, deadline, &mut read)?
            else {
                let _ = output.shutdown(Shutdown::Write);
                return Ok(());
            };
            let ordinal = read.frames_received;
            {
                let mut snapshot = shared.lock().unwrap();
                let direction = if reverse {
                    &mut snapshot.reverse
                } else {
                    &mut snapshot.forward
                };
                direction.read = read;
                if direction.captured.len() < CAPTURED_FRAMES {
                    direction.captured.push(wire.clone());
                }
                if Some(ordinal) == withhold {
                    direction.withheld = Some(Withheld {
                        ordinal,
                        length: wire.len(),
                        hash: spiralcore_transfer::hash(b"spiralcore.test.withheld.v1", &[&wire]),
                    });
                }
            }
            if Some(ordinal) != withhold {
                live::write_ciphertext_frame(&mut output, &wire, cancel, deadline, &mut written)?;
            }
            let mut snapshot = shared.lock().unwrap();
            let direction = if reverse {
                &mut snapshot.reverse
            } else {
                &mut snapshot.forward
            };
            direction.written = written;
        }
    })();
    let mut snapshot = shared.lock().unwrap();
    let direction = if reverse {
        &mut snapshot.reverse
    } else {
        &mut snapshot.forward
    };
    direction.read = read;
    direction.written = written;
    direction.terminal = Some(match result {
        Ok(()) => "clean EOF".into(),
        Err(error) => error.to_string(),
    });
}
