use spiralcore_mvm_flowmux::{Binding, Duplex, MappedStream, ObjectChannel, Role};
use spiralcore_transfer::{
    endpoint::{Authority, Context, Limits},
    v1::{Capabilities, ObjectEndpoint, ObjectMode, ObjectSender},
};
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    path::PathBuf,
    time::Duration,
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
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn standard_relay_io_preserves_short_reads_clones_and_reverse_after_half_close() {
    let c = Context {
        tenant: "mapped".into(),
        domain: "test".into(),
        sender: "roles".into(),
        recipient: "roles".into(),
        mailbox: "stream".into(),
        capability: "test-selected".into(),
        authorization_scope: "flow".into(),
        policy_epoch: 0,
        execution_epoch: 0,
    };
    let binding = Binding {
        flow: c.namespace().unwrap(),
        policy_sha256: [4; 32],
        scope_commitment: [9; 32],
    };
    let mut caps = Capabilities::full();
    caps.max_object_bytes = 65536;
    let profile = caps.negotiate(&caps).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = TcpStream::connect(address).unwrap();
    let (server, peer) = listener.accept().unwrap();
    let stores = [
        Store(std::env::temp_dir().join(format!("spiral-mapped-left-{}", std::process::id()))),
        Store(std::env::temp_dir().join(format!("spiral-mapped-right-{}", std::process::id()))),
    ];
    let make = |socket, peer, role, store: &Store| {
        let channel = ObjectChannel::from_connected(
            socket,
            binding,
            peer,
            Duration::from_secs(2),
            caps,
            caps,
        )
        .unwrap();
        let sender = ObjectSender::new_with_profile(ObjectMode::Whole, &c, None, profile).unwrap();
        let endpoint = ObjectEndpoint::open(&store.0, Limits::default(), Allow, profile).unwrap();
        MappedStream::from_duplex(
            Duplex::spawn(channel, sender, endpoint, c.clone(), role).unwrap(),
        )
    };
    let original = make(client, address, Role::Initiator, &stores[0]);
    let mut left = original.clone();
    drop(original);
    let mut right = make(server, peer, Role::Acceptor, &stores[1]);
    assert_eq!(left.read(&mut []).unwrap(), 0);
    assert_eq!(left.write(&[]).unwrap(), 0);
    std::thread::scope(|scope| {
        let peer = scope.spawn(move || {
            let mut target = vec![0; 20000];
            for chunk in target.chunks_mut(137) {
                right.read_exact(chunk).unwrap();
            }
            assert_eq!(target, vec![33; 20000]);
            assert_eq!(right.read(&mut [0; 1]).unwrap(), 0);
            assert_eq!(right.read(&mut [0; 1]).unwrap(), 0);
            right.write_all(&vec![77; 4096]).unwrap();
            right.flush().unwrap();
            right.shutdown(Shutdown::Write).unwrap();
        });
        left.write_all(&vec![33; 20000]).unwrap();
        left.shutdown(Shutdown::Write).unwrap();
        assert!(left.write_all(b"refused while reverse still open").is_err());
        let mut response = Vec::new();
        left.read_to_end(&mut response).unwrap();
        assert_eq!(response, vec![77; 4096]);
        assert!(left.write_all(b"after fin").is_err());
        peer.join().unwrap();
    });
}
