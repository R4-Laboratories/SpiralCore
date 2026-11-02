use rcgen::{
    BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose,
};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use sha2::{Digest, Sha256};
use spiralcore_gateway::object_channel::{Error, ObjectChannel, MAX_BATCH_BYTES};
use spiralcore_gateway::transport::{Identity, Listener, Peer};
use spiralcore_transfer::{protocol::Frame, wan::encode_batch};
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

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

async fn object_peers() -> (Peer, Peer) {
    let (server_identity, client_identity) = identities();
    let local = "127.0.0.1:0".parse().unwrap();
    let listener = Listener::bind_objects(local, server_identity).unwrap();
    let remote = listener.local_addr().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let (server, client) = tokio::join!(
        listener.accept(deadline),
        Peer::connect_objects(local, remote, "gateway.test", client_identity, deadline)
    );
    (server.unwrap(), client.unwrap())
}

// A real authenticated peer deliberately bypassing only our application framing.
struct RawPeer {
    _endpoint: quinn::Endpoint,
    _connection: quinn::Connection,
    send: quinn::SendStream,
    receive: quinn::RecvStream,
}
async fn incoming_peer(wire: &[u8], finish: bool) -> (Peer, RawPeer) {
    let (server_identity, client_identity) = identities();
    let local = "127.0.0.1:0".parse().unwrap();
    let listener = Listener::bind_objects(local, server_identity).unwrap();
    let remote = listener.local_addr().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let server = tokio::spawn(async move { listener.accept(deadline).await.unwrap() });
    let mut roots = rustls::RootCertStore::empty();
    for cert in client_identity.trusted_roots {
        roots.add(cert).unwrap();
    }
    let mut tls = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .unwrap()
    .with_root_certificates(roots)
    .with_client_auth_cert(
        client_identity.certificate_chain,
        client_identity.private_key,
    )
    .unwrap();
    tls.alpn_protocols = vec![spiralcore_gateway::transport::OBJECT_ALPN.to_vec()];
    tls.enable_early_data = false;
    tls.resumption = rustls::client::Resumption::disabled();
    let crypto = quinn::crypto::rustls::QuicClientConfig::try_from(tls).unwrap();
    let mut endpoint = quinn::Endpoint::client(local).unwrap();
    endpoint.set_default_client_config(quinn::ClientConfig::new(std::sync::Arc::new(crypto)));
    let connection = endpoint
        .connect(remote, "gateway.test")
        .unwrap()
        .await
        .unwrap();
    let (mut send, receive) = connection.open_bi().await.unwrap();
    send.write_all(wire).await.unwrap();
    if finish {
        send.finish().unwrap();
    }
    (
        server.await.unwrap(),
        RawPeer {
            _endpoint: endpoint,
            _connection: connection,
            send,
            receive,
        },
    )
}

async fn incoming_wire(wire: &[u8], finish: bool) -> (ObjectChannel, RawPeer) {
    let (peer, raw) = incoming_peer(wire, finish).await;
    let channel = peer
        .accept_object_channel(
            Instant::now() + Duration::from_secs(5),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    (channel, raw)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn object_profile_cannot_send_datagrams_even_if_the_authenticated_peer_advertises_support() {
    let (peer, _raw) = incoming_peer(&[0; 4], false).await;
    assert!(
        peer.max_datagram_size() >= 700,
        "raw peer must advertise DATAGRAM support"
    );
    assert!(matches!(
        peer.send_datagram(
            bytes::Bytes::from_static(b"wrong profile"),
            Instant::now() + Duration::from_millis(50)
        )
        .await,
        Err(spiralcore_gateway::transport::Error::Protocol)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn object_profile_rejects_packet_alpn_and_wrong_leaf_pins() {
    for server_is_object in [false, true] {
        let (server_identity, client_identity) = identities();
        let local = "127.0.0.1:0".parse().unwrap();
        let listener = if server_is_object {
            Listener::bind_objects(local, server_identity)
        } else {
            Listener::bind(local, server_identity)
        }
        .unwrap();
        let remote = listener.local_addr().unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        let client = async {
            if server_is_object {
                Peer::connect(local, remote, "gateway.test", client_identity, deadline).await
            } else {
                Peer::connect_objects(local, remote, "gateway.test", client_identity, deadline)
                    .await
            }
        };
        let (server, client) = tokio::join!(listener.accept(deadline), client);
        assert!(server.is_err() && client.is_err());
    }
    for server_rejects in [false, true] {
        let (mut server_identity, mut client_identity) = identities();
        if server_rejects {
            server_identity.peer_leaf_sha256 = [0; 32];
        } else {
            client_identity.peer_leaf_sha256 = [0; 32];
        }
        let local = "127.0.0.1:0".parse().unwrap();
        let listener = Listener::bind_objects(local, server_identity).unwrap();
        let remote = listener.local_addr().unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        let (server, client) = tokio::join!(
            listener.accept(deadline),
            Peer::connect_objects(local, remote, "gateway.test", client_identity, deadline)
        );
        if server_rejects {
            assert!(matches!(
                server,
                Err(spiralcore_gateway::transport::Error::PinMismatch)
            ));
        } else {
            assert!(matches!(
                client,
                Err(spiralcore_gateway::transport::Error::PinMismatch)
            ));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_length_truncation_and_noncanonical_batches_poison_the_channel() {
    let mut empty_batch = 12u32.to_le_bytes().to_vec();
    empty_batch.extend_from_slice(b"SCWBAT01");
    empty_batch.extend_from_slice(&0u32.to_le_bytes());
    let mut gap = frames();
    gap.truncate(2);
    gap[1].seq = 2;
    let gap = encode_batch(&gap).unwrap();
    let mut gap_wire = (gap.len() as u32).to_le_bytes().to_vec();
    gap_wire.extend_from_slice(&gap);
    let cases = [
        ((MAX_BATCH_BYTES as u32 + 1).to_le_bytes().to_vec(), 0),
        (11u32.to_le_bytes().to_vec(), 0),
        (vec![10, 0], 1),
        (100u32.to_le_bytes().into_iter().chain([0; 5]).collect(), 1),
        (empty_batch, 2),
        (gap_wire, 2),
        (Vec::new(), 1),
    ];
    for (wire, expected) in cases {
        let (mut channel, _raw) = incoming_wire(&wire, true).await;
        let deadline = Instant::now() + Duration::from_secs(2);
        let cancel = CancellationToken::new();
        let error = channel.recv_batch(deadline, &cancel).await.unwrap_err();
        match expected {
            0 => assert!(matches!(error, Error::Length)),
            1 => assert!(matches!(error, Error::Read(_))),
            _ => assert!(matches!(error, Error::Batch(_))),
        }
        assert!(matches!(
            channel.recv_batch(deadline, &cancel).await,
            Err(Error::Poisoned)
        ));
        assert_eq!(channel.counters().batches_received, 0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fragmented_stream_writes_preserve_exact_framing_and_cancellation_abandons_partial_prefix()
{
    let request = Frame {
        kind: 5,
        transfer: [9; 32],
        seq: 0,
        body: vec![0, 255, 0, 12],
    };
    let batch = encode_batch(std::slice::from_ref(&request)).unwrap();
    let mut wire = (batch.len() as u32).to_le_bytes().to_vec();
    wire.extend_from_slice(&batch);
    let (mut channel, mut raw) = incoming_wire(&wire[..2], false).await;
    let remaining = wire[2..].to_vec();
    let writer = tokio::spawn(async move {
        for chunk in remaining.chunks(3) {
            raw.send.write_all(chunk).await.unwrap();
        }
        let mut length = [0; 4];
        raw.receive.read_exact(&mut length).await.unwrap();
        let mut reply = vec![0; u32::from_le_bytes(length) as usize];
        raw.receive.read_exact(&mut reply).await.unwrap();
        raw.send.write_all(&0u32.to_le_bytes()).await.unwrap();
        raw.send.finish().unwrap();
        raw.receive.read_exact(&mut length).await.unwrap();
        assert_eq!(length, [0; 4]);
        assert_eq!(raw.receive.read(&mut [0; 1]).await.unwrap(), None);
        (raw, reply)
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    let cancel = CancellationToken::new();
    assert_eq!(
        channel.recv_batch(deadline, &cancel).await.unwrap(),
        Some(vec![request.clone()])
    );
    channel
        .send_batch(std::slice::from_ref(&request), deadline, &cancel)
        .await
        .unwrap();
    assert!(channel
        .recv_batch(deadline, &cancel)
        .await
        .unwrap()
        .is_none());
    channel.acknowledge_finish(deadline, &cancel).await.unwrap();
    let (_raw, reply) = writer.await.unwrap();
    assert_eq!(reply, batch);

    let (mut channel, _raw) = incoming_wire(&[12, 0], false).await;
    assert!(matches!(
        channel
            .recv_batch(Instant::now() + Duration::from_millis(10), &cancel)
            .await,
        Err(Error::Deadline)
    ));
    assert!(matches!(
        channel.recv_batch(deadline, &cancel).await,
        Err(Error::Poisoned)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deadlines_explicit_cancellation_and_dropped_futures_cannot_resume_reads() {
    for interruption in 0..3 {
        let (_server, peer) = object_peers().await;
        let deadline = Instant::now() + Duration::from_secs(2);
        let cancel = CancellationToken::new();
        let mut channel = peer.open_object_channel(deadline, &cancel).await.unwrap();
        if interruption == 0 {
            assert!(matches!(
                channel
                    .recv_batch(Instant::now() + Duration::from_millis(10), &cancel)
                    .await,
                Err(Error::Deadline)
            ));
        } else if interruption == 1 {
            let signal = cancel.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                signal.cancel();
            });
            assert!(matches!(
                channel.recv_batch(deadline, &cancel).await,
                Err(Error::Cancelled)
            ));
        } else {
            assert!(tokio::time::timeout(
                Duration::from_millis(10),
                channel.recv_batch(deadline, &cancel)
            )
            .await
            .is_err());
        }
        assert!(matches!(
            channel
                .recv_batch(deadline, &CancellationToken::new())
                .await,
            Err(Error::Poisoned)
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn outbound_invalid_batch_is_rejected_without_writing_a_prefix() {
    for frames in [Vec::new(), vec![frames()[0].clone(); 513]] {
        let (_server, peer) = object_peers().await;
        let deadline = Instant::now() + Duration::from_secs(2);
        let cancel = CancellationToken::new();
        let mut channel = peer.open_object_channel(deadline, &cancel).await.unwrap();
        assert!(matches!(
            channel.send_batch(&frames, deadline, &cancel).await,
            Err(Error::Batch(spiralcore_transfer::Error::Limit))
        ));
        assert_eq!(channel.counters().framing_bytes_sent, 0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn zero_end_rejects_trailing_data_and_finish_requires_acknowledgement() {
    let (mut channel, _raw) = incoming_wire(&[0, 0, 0, 0, 7], true).await;
    let deadline = Instant::now() + Duration::from_secs(2);
    let cancel = CancellationToken::new();
    assert!(channel
        .recv_batch(deadline, &cancel)
        .await
        .unwrap()
        .is_none());
    assert!(matches!(
        channel.acknowledge_finish(deadline, &cancel).await,
        Err(Error::UnexpectedEnd)
    ));

    let (_server, peer) = object_peers().await;
    let mut channel = peer.open_object_channel(deadline, &cancel).await.unwrap();
    assert!(matches!(
        channel
            .finish(Instant::now() + Duration::from_millis(10), &cancel)
            .await,
        Err(Error::Deadline)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_finished_client_can_be_dropped_immediately() {
    let (server, client) = object_peers().await;
    let deadline = Instant::now() + Duration::from_secs(3);
    let cancel = CancellationToken::new();
    let server = tokio::spawn(async move {
        let cancel = CancellationToken::new();
        let mut channel = server
            .accept_object_channel(deadline, &cancel)
            .await
            .unwrap();
        assert!(channel
            .recv_batch(deadline, &cancel)
            .await
            .unwrap()
            .is_none());
        channel.acknowledge_finish(deadline, &cancel).await
    });
    let mut channel = client.open_object_channel(deadline, &cancel).await.unwrap();
    channel.finish(deadline, &cancel).await.unwrap();
    drop(channel);
    server.await.unwrap().unwrap();
}

fn frames() -> Vec<Frame> {
    (0..20)
        .map(|seq| Frame {
            kind: 3,
            transfer: [42; 32],
            seq,
            body: vec![seq as u8; 60_000],
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exact_batches_share_one_authenticated_stream_and_finish_explicitly() {
    let (server_identity, client_identity) = identities();
    let local = "127.0.0.1:0".parse().unwrap();
    let listener = Listener::bind_objects(local, server_identity).unwrap();
    let remote = listener.local_addr().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let cancel = CancellationToken::new();
    let server_cancel = cancel.clone();
    let requests = frames();
    let expected_requests = requests.clone();
    let batch_bytes = encode_batch(&requests).unwrap().len() as u64;
    let server = tokio::spawn(async move {
        let peer = listener.accept(deadline).await.unwrap();
        let mut channel = peer
            .accept_object_channel(deadline, &server_cancel)
            .await
            .unwrap();
        for _ in 0..2 {
            let received = channel
                .recv_batch(deadline, &server_cancel)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(received, expected_requests);
            let replies: Vec<_> = received
                .iter()
                .map(|frame| frame.reply(102, frame.body.clone()))
                .collect();
            channel
                .send_batch(&replies, deadline, &server_cancel)
                .await
                .unwrap();
        }
        assert!(channel
            .recv_batch(deadline, &server_cancel)
            .await
            .unwrap()
            .is_none());
        channel
            .acknowledge_finish(deadline, &server_cancel)
            .await
            .unwrap();
        channel
    });
    let peer = Peer::connect_objects(local, remote, "gateway.test", client_identity, deadline)
        .await
        .unwrap();
    let mut client = peer.open_object_channel(deadline, &cancel).await.unwrap();
    let replies: Vec<_> = requests
        .iter()
        .map(|frame| frame.reply(102, frame.body.clone()))
        .collect();
    for _ in 0..2 {
        assert_eq!(
            client.exchange(&requests, deadline, &cancel).await.unwrap(),
            replies
        );
    }
    client.finish(deadline, &cancel).await.unwrap();
    let server = server.await.unwrap();
    for channel in [&client, &server] {
        let counters = channel.counters();
        assert_eq!(counters.batches_sent, 2);
        assert_eq!(counters.batches_received, 2);
        assert_eq!(counters.encoded_batch_bytes_sent, batch_bytes * 2);
        assert_eq!(counters.encoded_batch_bytes_received, batch_bytes * 2);
        assert_eq!(counters.framing_bytes_sent, 12);
        assert_eq!(counters.framing_bytes_received, 12);
        let udp = channel.udp_counters();
        assert!(udp.sent_bytes > counters.encoded_batch_bytes_sent);
        assert!(udp.received_bytes > counters.encoded_batch_bytes_received);
    }
}
