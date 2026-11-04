use super::*;
use spiralcore_transfer::endpoint::Limits;
use spiralcore_transfer::v1::Capabilities;
use std::{
    net::{TcpListener, TcpStream},
    path::PathBuf,
};

struct Allow;
impl Authority for Allow {
    fn authorize(&self, _: &Context) -> spiralcore_transfer::Result<()> {
        Ok(())
    }
}
struct Store(PathBuf);
impl Drop for Store {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn completion_requires_live_delivery_fence_at_the_terminal_barrier() {
    let context = Context {
        tenant: "completion".into(),
        domain: "test-only".into(),
        sender: "sender".into(),
        recipient: "receiver".into(),
        mailbox: "stream".into(),
        capability: "selected".into(),
        authorization_scope: "flow".into(),
        policy_epoch: 0,
        execution_epoch: 0,
    };
    let binding = Binding {
        flow: context.namespace().unwrap(),
        policy_sha256: [4; 32],
        scope_commitment: [9; 32],
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let socket = TcpStream::connect(address).unwrap();
    let (_peer, _) = listener.accept().unwrap();
    let mut caps = Capabilities::full();
    caps.max_object_bytes = 65536;
    let channel =
        ObjectChannel::from_connected(socket, binding, address, Duration::from_secs(1), caps, caps)
            .unwrap();
    let path = std::env::temp_dir().join(format!(
        "flow-terminal-cancel-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let endpoint =
        ObjectEndpoint::open(&path, Limits::default(), Allow, channel.profile()).unwrap();
    let _store = Store(path);
    let (responses, _response_rx) = mpsc::sync_channel(1);
    let (delivery, _delivery_rx) = mpsc::sync_channel(4);
    let cancellation = channel.cancellation();
    let mut worker = Worker {
        completed: false,
        rx: channel.rx,
        tx: Arc::new(Mutex::new(channel.tx)),
        pending: Arc::new(Mutex::new(None)),
        responses,
        delivery,
        endpoint: ReceiveEndpoint::General(Box::new(endpoint)),
        context,
        binding,
        role: Role::Initiator,
        index: 0,
        artifact_index: 0,
        active: None,
        remote_closed: true,
        local_closed: Arc::new(AtomicBool::new(true)),
        cancellation,
        timeout: Duration::from_secs(1),
        retention: RetentionObserver::default(),
    };
    // This unit isolates the completion transition, using original real socket
    // ownership. It does not claim a wire-authenticated FIN or native session.
    worker.cancellation.cancel();
    assert!(matches!(worker.complete(), Err(Error::Cancelled)));
    assert!(!worker.completed, "fatal delivery cannot become completed");
    drop(worker);
}
