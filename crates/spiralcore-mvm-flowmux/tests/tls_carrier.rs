use spiralcore_mvm_flowmux::tls::{PinnedIdentity, TlsCarrier};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};
fn identity() -> PinnedIdentity {
    let key =
        ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    PinnedIdentity::from_pkcs8(key.as_ref().to_vec()).unwrap()
}
#[test]
fn mutual_pinned_tls_uses_one_session_for_cloned_bidirectional_io() {
    let client = identity();
    let server = identity();
    let client_pin = client.spki_sha256();
    let server_pin = server.spki_sha256();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let raw = TcpStream::connect(address).unwrap();
    let (peer, _) = listener.accept().unwrap();
    std::thread::scope(|scope| {
        let other = scope.spawn(move || {
            let mut stream = TlsCarrier::accept(
                peer,
                &server,
                client_pin,
                Instant::now() + Duration::from_secs(2),
            )
            .unwrap();
            let exporter = stream.exporter();
            let mut writer = stream.try_clone().unwrap();
            let reader = scope.spawn(move || {
                let mut bytes = [0; 4096];
                stream.read_exact(&mut bytes).unwrap();
                assert_eq!(bytes, [17; 4096]);
                exporter
            });
            writer.write_all(&[31; 4096]).unwrap();
            writer.flush().unwrap();
            reader.join().unwrap()
        });
        let mut stream = TlsCarrier::connect(
            raw,
            &client,
            server_pin,
            "peer.example",
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        let exporter = stream.exporter();
        let mut writer = stream.try_clone().unwrap();
        let reader = scope.spawn(move || {
            let mut bytes = [0; 4096];
            stream.read_exact(&mut bytes).unwrap();
            assert_eq!(bytes, [31; 4096]);
        });
        writer.write_all(&[17; 4096]).unwrap();
        writer.flush().unwrap();
        reader.join().unwrap();
        assert_eq!(other.join().unwrap(), exporter);
    });
}
#[test]
fn wrong_pin_and_unavailable_peer_refuse_before_application_bytes() {
    for wrong_server in [false, true] {
        let client = identity();
        let server = identity();
        let client_pin = client.spki_sha256();
        let server_pin = server.spki_sha256();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let raw = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (peer, _) = listener.accept().unwrap();
        std::thread::scope(|scope| {
            let other = scope.spawn(move || {
                TlsCarrier::accept(
                    peer,
                    &server,
                    if wrong_server { client_pin } else { [1; 32] },
                    Instant::now() + Duration::from_millis(300),
                )
            });
            assert!(
                TlsCarrier::connect(
                    raw,
                    &client,
                    if wrong_server { [1; 32] } else { server_pin },
                    "peer.example",
                    Instant::now() + Duration::from_millis(300)
                )
                .is_err()
            );
            assert!(other.join().unwrap().is_err());
        });
    }
    let client = identity();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let raw = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (_peer, _) = listener.accept().unwrap();
    assert!(
        TlsCarrier::connect(
            raw,
            &client,
            [1; 32],
            "peer.example",
            Instant::now() + Duration::from_millis(50)
        )
        .is_err()
    );
}
