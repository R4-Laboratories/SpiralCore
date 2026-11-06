use spiralcore_mvm_flowmux::{
    Binding, Duplex, Error, JoinGuard, MappedStream, ObjectChannel, Role,
};
use spiralcore_transfer::{
    endpoint::{Authority, Context, Limits},
    v1::{Capabilities, ObjectEndpoint, ObjectMode, ObjectSender},
};
use std::{
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Allow;
impl Authority for Allow {
    fn authorize(&self, _: &Context) -> spiralcore_transfer::Result<()> {
        Ok(())
    }
}
struct GatedAuthority {
    first: AtomicBool,
    entered: mpsc::SyncSender<()>,
    release: Mutex<mpsc::Receiver<()>>,
    dropped: Arc<AtomicBool>,
}
impl Authority for GatedAuthority {
    fn authorize(&self, _: &Context) -> spiralcore_transfer::Result<()> {
        if self.first.swap(false, Ordering::AcqRel) {
            self.entered.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(3))
                .unwrap();
        }
        Ok(())
    }
}
impl Drop for GatedAuthority {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}
struct Store(PathBuf);
impl Drop for Store {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn context() -> Context {
    Context {
        tenant: "reader-completion".into(),
        domain: "selected".into(),
        sender: "pair".into(),
        recipient: "pair".into(),
        mailbox: "stream".into(),
        capability: "test-only".into(),
        authorization_scope: "flow".into(),
        policy_epoch: 0,
        execution_epoch: 0,
    }
}
fn build<A: Authority + Send + 'static>(
    socket: TcpStream,
    peer: SocketAddr,
    root: &Path,
    role: Role,
    authority: A,
) -> Duplex {
    let context = context();
    let binding = Binding {
        flow: context.namespace().unwrap(),
        policy_sha256: [4; 32],
        scope_commitment: [9; 32],
    };
    let mut caps = Capabilities::full();
    caps.max_object_bytes = 65536;
    let profile = caps.negotiate(&caps).unwrap();
    let channel =
        ObjectChannel::from_connected(socket, binding, peer, Duration::from_secs(2), caps, caps)
            .unwrap();
    let sender =
        ObjectSender::new_with_profile(ObjectMode::Whole, &context, None, profile).unwrap();
    let endpoint = ObjectEndpoint::open(root, Limits::default(), authority, profile).unwrap();
    Duplex::spawn(channel, sender, endpoint, context, role).unwrap()
}
struct OriginalGuard {
    guard: JoinGuard,
    _writer: spiralcore_mvm_flowmux::DuplexWriter,
    _reader: spiralcore_mvm_flowmux::DuplexReader,
}
enum OriginalReader {
    Guard(Box<OriginalGuard>),
    Mapped(MappedStream),
}
impl OriginalReader {
    fn complete(&mut self, until: Instant) -> spiralcore_mvm_flowmux::Result<()> {
        match self {
            Self::Guard(original) => original.guard.try_join_until(until),
            Self::Mapped(stream) => stream.close_and_join_until(until),
        }
    }
}
fn exercise(mapped: bool) {
    let path = std::env::temp_dir().join(format!(
        "spiral-reader-completion-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&path).unwrap();
    let store = Store(path);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (server, peer) = listener.accept().unwrap();
    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let dropped = Arc::new(AtomicBool::new(false));
    let left = build(
        client,
        address,
        &store.0.join("left"),
        Role::Initiator,
        Allow,
    );
    let right = build(
        server,
        peer,
        &store.0.join("right"),
        Role::Acceptor,
        GatedAuthority {
            first: AtomicBool::new(true),
            entered: entered_tx,
            release: Mutex::new(release_rx),
            dropped: dropped.clone(),
        },
    );
    let (mut writer, reader, left_guard) = left.split();
    let mut original = if mapped {
        OriginalReader::Mapped(MappedStream::from_duplex(right))
    } else {
        let (writer, reader, guard) = right.split();
        OriginalReader::Guard(Box::new(OriginalGuard {
            guard,
            _writer: writer,
            _reader: reader,
        }))
    };
    let alias = match &original {
        OriginalReader::Mapped(stream) => Some(stream.clone()),
        OriginalReader::Guard(_) => None,
    };
    std::thread::scope(|scope| {
        let sending = scope.spawn(move || writer.send_block(vec![17; 1024]));
        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let start = Instant::now();
        assert!(matches!(
            original.complete(start + Duration::from_millis(20)),
            Err(Error::Timeout)
        ));
        assert!(
            start.elapsed() < Duration::from_millis(500),
            "timeout joined a still-blocked reader"
        );
        assert!(
            !dropped.load(Ordering::Acquire),
            "timeout destroyed original endpoint"
        );
        release_tx.send(()).unwrap();
        let until = Instant::now() + Duration::from_secs(1);
        if let Some(alias) = &alias {
            alias.close_and_join_until(until).unwrap();
        } else {
            original.complete(until).unwrap();
        }
        assert!(
            dropped.load(Ordering::Acquire),
            "positive join precedes endpoint release"
        );
        // Completion is an original physical fact, not another wait or join.
        original.complete(Instant::now()).unwrap();
        assert!(
            sending.join().unwrap().is_err(),
            "cancelled request was not a successful transfer"
        );
    });
    if let OriginalReader::Guard(original) = original {
        assert!(
            original.guard.join().is_err(),
            "physical completion erased semantic cancellation"
        );
    }
    drop(reader);
    assert!(left_guard.join().is_err());
}

#[test]
fn join_guard_timeout_keeps_original_reader_and_legacy_result() {
    exercise(false);
}

#[test]
fn mapped_alias_timeout_retains_original_reader_until_explicit_completion() {
    exercise(true);
}
