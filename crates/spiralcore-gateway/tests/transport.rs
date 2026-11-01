use bytes::Bytes;
use rcgen::{
    BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose,
};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use sha2::{Digest, Sha256};
use spiralcore_gateway::transport::{Error, Identity, Listener, Peer, ALPN};
use std::net::SocketAddr;
use tokio::time::{Duration, Instant};

fn identities() -> (Identity, Identity) {
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let ca_key = KeyPair::generate().unwrap();
    let root = params.self_signed(&ca_key).unwrap().der().clone();
    let issuer = Issuer::new(params, ca_key);
    let leaf = || {
        let mut params = CertificateParams::new(vec!["gateway.test".into()]).unwrap();
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params.extended_key_usages = vec![
            ExtendedKeyUsagePurpose::ServerAuth,
            ExtendedKeyUsagePurpose::ClientAuth,
        ];
        let key = KeyPair::generate().unwrap();
        let cert = params.signed_by(&key, &issuer).unwrap().der().clone();
        (cert, key)
    };
    let (server_cert, server_key) = leaf();
    let (client_cert, client_key) = leaf();
    let pin = |cert: &CertificateDer<'_>| -> [u8; 32] { Sha256::digest(cert.as_ref()).into() };
    let server_pin = pin(&server_cert);
    let client_pin = pin(&client_cert);
    (
        Identity {
            certificate_chain: vec![server_cert],
            private_key: PrivatePkcs8KeyDer::from(server_key.serialize_der()).into(),
            trusted_roots: vec![root.clone()],
            peer_leaf_sha256: client_pin,
        },
        Identity {
            certificate_chain: vec![client_cert],
            private_key: PrivatePkcs8KeyDer::from(client_key.serialize_der()).into(),
            trusted_roots: vec![root],
            peer_leaf_sha256: server_pin,
        },
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mutually_authenticated_peers_exchange_exact_datagrams_over_udp() {
    let (server_identity, client_identity) = identities();
    let local: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = Listener::bind(local, server_identity).unwrap();
    let remote = listener.local_addr().unwrap();
    let payload = Bytes::from_static(b"\0exact\xffpacket\0");
    let expected = payload.clone();
    let deadline = Instant::now() + Duration::from_secs(5);
    let server = tokio::spawn(async move {
        let peer = listener.accept(deadline).await.unwrap();
        assert!((700..=1336).contains(&peer.max_datagram_size()));
        let handshake = peer.udp_counters();
        assert!(handshake.sent_bytes > 0 && handshake.received_bytes > 0);
        let received = peer.receive_datagram(deadline).await.unwrap();
        assert_eq!(received, expected);
        peer.send_datagram(received, deadline).await.unwrap();
        assert_eq!(
            peer.receive_datagram(deadline).await.unwrap(),
            Bytes::from_static(b"received")
        );
        peer.udp_counters()
    });
    let client = Peer::connect(local, remote, "gateway.test", client_identity, deadline)
        .await
        .unwrap();
    assert!((700..=1336).contains(&client.max_datagram_size()));
    client
        .send_datagram(payload.clone(), deadline)
        .await
        .unwrap();
    assert_eq!(client.receive_datagram(deadline).await.unwrap(), payload);
    client
        .send_datagram(Bytes::from_static(b"received"), deadline)
        .await
        .unwrap();
    let server_counters = server.await.unwrap();
    let client_counters = client.udp_counters();
    for counters in [server_counters, client_counters] {
        assert!(counters.sent_datagrams > 0 && counters.received_datagrams > 0);
        assert!(counters.sent_bytes > payload.len() as u64);
        assert!(counters.received_bytes > payload.len() as u64);
    }
    client.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn both_endpoints_reject_a_valid_certificate_with_the_wrong_pin() {
    for server_rejects in [false, true] {
        let (mut server_identity, mut client_identity) = identities();
        if server_rejects {
            server_identity.peer_leaf_sha256 = [0; 32];
        } else {
            client_identity.peer_leaf_sha256 = [0; 32];
        }
        let local = "127.0.0.1:0".parse().unwrap();
        let listener = Listener::bind(local, server_identity).unwrap();
        let remote = listener.local_addr().unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        let (server, client) = tokio::join!(
            listener.accept(deadline),
            Peer::connect(local, remote, "gateway.test", client_identity, deadline)
        );
        if server_rejects {
            assert!(matches!(server, Err(Error::PinMismatch)));
        } else {
            assert!(matches!(client, Err(Error::PinMismatch)));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_exact_pin_does_not_bypass_trust_roots_or_server_name_validation() {
    for wrong_name in [false, true] {
        let (server_identity, mut client_identity) = identities();
        if !wrong_name {
            client_identity.trusted_roots = identities().0.trusted_roots;
        }
        let local = "127.0.0.1:0".parse().unwrap();
        let listener = Listener::bind(local, server_identity).unwrap();
        let remote = listener.local_addr().unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        let (server, client) = tokio::join!(
            listener.accept(deadline),
            Peer::connect(
                local,
                remote,
                if wrong_name {
                    "wrong.test"
                } else {
                    "gateway.test"
                },
                client_identity,
                deadline
            )
        );
        assert!(matches!(client, Err(Error::Connection(_))));
        assert!(server.is_err());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_requires_a_client_certificate_at_the_tls_boundary() {
    use std::sync::Arc;
    let (server_identity, client_identity) = identities();
    let mut roots = rustls::RootCertStore::empty();
    for certificate in client_identity.trusted_roots {
        roots.add(certificate).unwrap();
    }
    let mut tls = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    tls.alpn_protocols = vec![ALPN.to_vec()];
    let crypto = quinn::crypto::rustls::QuicClientConfig::try_from(tls).unwrap();
    let local = "127.0.0.1:0".parse().unwrap();
    let listener = Listener::bind(local, server_identity).unwrap();
    let remote = listener.local_addr().unwrap();
    let mut client = quinn::Endpoint::client(local).unwrap();
    client.set_default_client_config(quinn::ClientConfig::new(Arc::new(crypto)));
    let connecting = client.connect(remote, "gateway.test").unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let (server, _) = tokio::join!(
        listener.accept(deadline),
        tokio::time::timeout_at(deadline, connecting)
    );
    assert!(matches!(server, Err(Error::Connection(_))));
    client.close(0u8.into(), b"test complete");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn datagram_bounds_deadlines_and_clean_shutdown_are_explicit() {
    let (server_identity, client_identity) = identities();
    let local = "127.0.0.1:0".parse().unwrap();
    let listener = Listener::bind(local, server_identity).unwrap();
    let remote = listener.local_addr().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let (server, client) = tokio::join!(
        listener.accept(deadline),
        Peer::connect(local, remote, "gateway.test", client_identity, deadline)
    );
    let server = server.unwrap();
    let client = client.unwrap();
    for bytes in [
        Bytes::new(),
        Bytes::from(vec![0; client.max_datagram_size() + 1]),
    ] {
        assert!(matches!(
            client.send_datagram(bytes, deadline).await,
            Err(Error::DatagramLength)
        ));
    }
    assert!(matches!(
        client
            .send_datagram(Bytes::from_static(b"late"), Instant::now())
            .await,
        Err(Error::Deadline)
    ));
    assert!(matches!(
        server
            .receive_datagram(Instant::now() + Duration::from_millis(20))
            .await,
        Err(Error::Deadline)
    ));
    client
        .send_datagram(Bytes::from_static(b"after timeout"), deadline)
        .await
        .unwrap();
    assert_eq!(
        server.receive_datagram(deadline).await.unwrap().as_ref(),
        b"after timeout"
    );
    server.close();
    assert!(
        matches!(client.receive_datagram(deadline).await, Err(Error::Connection(quinn::ConnectionError::ApplicationClosed(close))) if close.error_code == 0u8.into())
    );
}
