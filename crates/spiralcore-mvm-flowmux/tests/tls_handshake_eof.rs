use spiralcore_mvm_flowmux::{
    Role,
    tls::{PinnedIdentity, TlsCarrier},
};
use std::{
    io::Read,
    net::{Shutdown, TcpListener, TcpStream},
    sync::mpsc,
    time::{Duration, Instant},
};

fn identity() -> PinnedIdentity {
    let key =
        ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    PinnedIdentity::from_pkcs8(key.as_ref().to_vec()).unwrap()
}
fn exercise(role: Role) {
    let identity = identity();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (server, _) = listener.accept().unwrap();
    let (owned, mut peer) = match role {
        Role::Initiator => (client, server),
        Role::Acceptor => (server, client),
    };
    peer.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
    let (release, released) = mpsc::sync_channel(1);
    std::thread::scope(|scope| {
        let peer = scope.spawn(move || {
            if role == Role::Initiator {
                // Consume this concrete library's actual first TLS record so
                // the close is an orderly FIN, not an unread-input TCP reset.
                let mut header = [0u8; 5];
                peer.read_exact(&mut header).unwrap();
                assert_eq!(header[0], 22);
                let length = usize::from(u16::from_be_bytes([header[3], header[4]]));
                assert!(length > 0 && length <= 16384);
                let mut body = vec![0u8; length];
                peer.read_exact(&mut body).unwrap();
            }
            peer.shutdown(Shutdown::Write).unwrap();
            // Keep the original read side alive until the constructor returns.
            released.recv_timeout(Duration::from_secs(3)).unwrap();
        });
        let started = Instant::now();
        let until = started + Duration::from_secs(2);
        let result = match role {
            Role::Initiator => {
                TlsCarrier::connect(owned, &identity, [7; 32], "peer.example", until)
            }
            Role::Acceptor => TlsCarrier::accept(owned, &identity, [7; 32], until),
        };
        let elapsed = started.elapsed();
        release.send(()).unwrap();
        peer.join().unwrap();
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::UnexpectedEof
        );
        assert!(
            elapsed < Duration::from_millis(500),
            "known EOF waited for the original deadline"
        );
    });
}

#[test]
fn initiator_refuses_physical_eof_during_handshake_without_waiting_for_timeout() {
    exercise(Role::Initiator);
}

#[test]
fn acceptor_refuses_physical_eof_before_client_hello_without_waiting_for_timeout() {
    exercise(Role::Acceptor);
}
