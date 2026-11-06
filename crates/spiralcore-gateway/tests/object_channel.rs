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

#[derive(Default, Clone, Copy, serde::Serialize)]
struct GateCounts {
    client_received: u64,
    client_forwarded: u64,
    client_discarded: u64,
    server_received: u64,
    server_forwarded: u64,
    server_discarded: u64,
    held_datagrams: u64,
    held_bytes: u64,
}

// Holds real UDP datagrams, never acknowledges a fictitious successful write.
// The gate is armed only after the authenticated initiator's end prefix.
struct AckGate {
    address: std::net::SocketAddr,
    armed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    blocked: std::sync::Arc<tokio::sync::Notify>,
    release: std::sync::Arc<tokio::sync::Notify>,
    discard_held: std::sync::Arc<std::sync::atomic::AtomicBool>,
    discard_next_server: std::sync::Arc<std::sync::atomic::AtomicBool>,
    discard_server_flight: std::sync::Arc<std::sync::atomic::AtomicBool>,
    server_discarded: std::sync::Arc<tokio::sync::Notify>,
    stop: CancellationToken,
    task: Option<tokio::task::JoinHandle<std::io::Result<GateCounts>>>,
}
impl AckGate {
    async fn start(server: std::net::SocketAddr) -> Self {
        use std::sync::{atomic::Ordering, Arc};
        let front = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let back = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        back.connect(server).await.unwrap();
        let address = front.local_addr().unwrap();
        let armed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let blocked = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let discard_held = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let discard_next_server = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let discard_server_flight = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let server_discarded = Arc::new(tokio::sync::Notify::new());
        let stop = CancellationToken::new();
        let (gate, observed, flush, cancelled) = (
            armed.clone(),
            blocked.clone(),
            release.clone(),
            stop.clone(),
        );
        let (discard, discard_server, observed_server) = (
            discard_held.clone(),
            discard_next_server.clone(),
            server_discarded.clone(),
        );
        let discard_flight = discard_server_flight.clone();
        let task = tokio::spawn(async move {
            let mut client = None;
            let mut held = std::collections::VecDeque::<Vec<u8>>::new();
            let mut incoming = vec![0; 65536];
            let mut outgoing = vec![0; 65536];
            let mut counts = GateCounts::default();
            loop {
                tokio::select! {
                    biased;
                    _ = cancelled.cancelled() => {
                        if !held.is_empty() {
                            return Err(std::io::Error::other("ACK gate stopped with retained datagrams"));
                        }
                        return Ok(counts);
                    }
                    _ = flush.notified() => {
                        while let Some(packet) = held.pop_front() {
                            if discard.load(Ordering::Acquire) {
                                counts.client_discarded += 1;
                            } else {
                                back.send(&packet).await?;
                                counts.client_forwarded += 1;
                            }
                        }
                    }
                    packet = front.recv_from(&mut incoming) => {
                        let (len, from) = packet?;
                        if client.is_some_and(|original| original != from) {
                            return Err(std::io::Error::other("unexpected second proxy client"));
                        }
                        client = Some(from);
                        counts.client_received += 1;
                        if gate.load(Ordering::Acquire) {
                            if held.len() == 64 {
                                return Err(std::io::Error::other("bounded ACK gate capacity"));
                            }
                            held.push_back(incoming[..len].to_vec());
                            counts.held_datagrams += 1;
                            counts.held_bytes += len as u64;
                            observed.notify_one();
                        } else {
                            back.send(&incoming[..len]).await?;
                            counts.client_forwarded += 1;
                        }
                    }
                    packet = back.recv(&mut outgoing) => {
                        let len = packet?;
                        counts.server_received += 1;
                        if discard_flight.load(Ordering::Acquire) || discard_server.swap(false, Ordering::AcqRel) {
                            counts.server_discarded += 1;
                            observed_server.notify_one();
                            continue;
                        }
                        let to = client.ok_or_else(|| std::io::Error::other("response before proxy client"))?;
                        front.send_to(&outgoing[..len], to).await?;
                        counts.server_forwarded += 1;
                    }
                }
            }
        });
        Self {
            address,
            armed,
            blocked,
            release,
            discard_held,
            discard_next_server,
            discard_server_flight,
            server_discarded,
            stop,
            task: Some(task),
        }
    }
    fn open(&self) {
        self.armed
            .store(false, std::sync::atomic::Ordering::Release);
        self.release.notify_one();
    }
}
impl Drop for AckGate {
    fn drop(&mut self) {
        self.open();
        self.stop.cancel();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn finish_retains_initiator_until_the_gated_responder_fin_is_acknowledged() {
    for discard in [false, true] {
        gated_finish(discard).await;
    }
}

async fn gated_finish(discard: bool) {
    let (server_identity, client_identity) = identities();
    let local = "127.0.0.1:0".parse().unwrap();
    let listener = Listener::bind_objects(local, server_identity).unwrap();
    let mut proxy = AckGate::start(listener.local_addr().unwrap()).await;
    proxy
        .discard_held
        .store(discard, std::sync::atomic::Ordering::Release);
    let deadline = Instant::now() + Duration::from_secs(5);
    let (server, client) = tokio::join!(
        listener.accept(deadline),
        Peer::connect_objects(
            local,
            proxy.address,
            "gateway.test",
            client_identity,
            deadline
        )
    );
    let (server, client) = (server.unwrap(), client.unwrap());
    let armed = proxy.armed.clone();
    let responder = tokio::spawn(async move {
        let cancel = CancellationToken::new();
        let mut channel = server
            .accept_object_channel(deadline, &cancel)
            .await
            .unwrap();
        let result = async {
            if channel.recv_batch(deadline, &cancel).await?.is_some() {
                return Err(Error::UnexpectedEnd);
            }
            armed.store(true, std::sync::atomic::Ordering::Release);
            channel.acknowledge_finish(deadline, &cancel).await
        }
        .await;
        (result, channel)
    });
    let mut initiator = tokio::spawn(async move {
        let cancel = CancellationToken::new();
        let mut channel = client.open_object_channel(deadline, &cancel).await.unwrap();
        let result = channel.finish(deadline, &cancel).await;
        (result, channel)
    });
    let blocked = tokio::time::timeout_at(deadline, proxy.blocked.notified())
        .await
        .is_ok();
    // A bounded negative observation, not a sleep or product grace allowance.
    let early = tokio::time::timeout(Duration::from_millis(200), &mut initiator)
        .await
        .ok();
    let premature_success = early
        .as_ref()
        .is_some_and(|value| value.as_ref().is_ok_and(|(result, _)| result.is_ok()));
    let responder_pending = !responder.is_finished();
    // Complete cleanup before semantic assertions, including the RED path.
    proxy.open();
    let completed = match early {
        Some(value) => Some(value),
        None => tokio::time::timeout_at(deadline, &mut initiator).await.ok(),
    };
    let responder = tokio::time::timeout_at(deadline, responder).await;
    proxy.stop.cancel();
    let traffic = tokio::time::timeout(Duration::from_secs(1), proxy.task.take().unwrap()).await;
    if let Some(directory) = std::env::var_os("SPIRALCORE_OBJECT_FINISH_EVIDENCE_DIR") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(std::path::PathBuf::from(directory).join(if discard {
                "lost-fin-ack.json"
            } else {
                "delayed-fin-ack.json"
            }))
            .unwrap();
        let counts = traffic
            .as_ref()
            .ok()
            .and_then(|r| r.as_ref().ok())
            .and_then(|r| r.as_ref().ok());
        serde_json::to_writer_pretty(file, &serde_json::json!({
            "scope":"real pinned-mTLS QUIC localhost UDP gate; no WAN claim",
            "held_observed":blocked,"premature_initiator_success":premature_success,
            "discard_held_flight":discard,
            "responder_pending_while_gated":responder_pending,"traffic":counts,
            "initiator_result":format!("{:?}",completed.as_ref().map(|r|r.as_ref().map(|(r,_)|r))),
            "responder_result":format!("{:?}",responder.as_ref().map(|r|r.as_ref().map(|(r,_)|r)))
        })).unwrap();
    }
    assert!(
        blocked,
        "the real transport must actually withhold datagrams"
    );
    assert!(
        responder_pending,
        "the responder must still require its terminal ACK"
    );
    assert!(
        !premature_success,
        "initiator reported success before responder FIN acknowledgment"
    );
    let (client_result, client) = completed.unwrap().unwrap();
    let (server_result, server) = responder.unwrap().unwrap();
    client_result.unwrap();
    server_result.unwrap();
    let traffic = traffic.unwrap().unwrap().unwrap();
    assert!(traffic.held_datagrams > 0 && traffic.held_bytes > 0);
    assert_eq!(
        traffic.client_received,
        traffic.client_forwarded + traffic.client_discarded
    );
    if discard {
        assert_eq!(traffic.client_discarded, traffic.held_datagrams);
    } else {
        assert_eq!(traffic.client_discarded, 0);
    }
    for channel in [&client, &server] {
        assert_eq!(channel.counters().framing_bytes_sent, 4);
        assert_eq!(channel.counters().framing_bytes_received, 4);
        assert_eq!(channel.counters().batches_sent, 0);
        assert_eq!(channel.counters().batches_received, 0);
    }
}

fn raw_object_listener(identity: Identity) -> quinn::Endpoint {
    use std::sync::Arc;
    let mut roots = rustls::RootCertStore::empty();
    for certificate in identity.trusted_roots {
        roots.add(certificate).unwrap();
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
        Arc::new(roots),
        provider.clone(),
    )
    .build()
    .unwrap();
    let mut tls = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_client_cert_verifier(verifier)
        .with_single_cert(identity.certificate_chain, identity.private_key)
        .unwrap();
    tls.alpn_protocols = vec![spiralcore_gateway::transport::OBJECT_ALPN.to_vec()];
    tls.max_early_data_size = 0;
    tls.send_tls13_tickets = 0;
    let crypto = quinn::crypto::rustls::QuicServerConfig::try_from(tls).unwrap();
    let mut config = quinn::ServerConfig::with_crypto(Arc::new(crypto));
    let mut transport = quinn::TransportConfig::default();
    transport
        .max_concurrent_bidi_streams(1u8.into())
        .max_concurrent_uni_streams(0u8.into())
        .datagram_receive_buffer_size(None)
        .datagram_send_buffer_size(0);
    config.transport_config(Arc::new(transport));
    quinn::Endpoint::server(config, "127.0.0.1:0".parse().unwrap()).unwrap()
}

#[derive(Clone, Copy, Debug)]
enum TerminalFault {
    EarlyZero,
    Nonzero,
    MissingClose,
    Cancel,
    ClosePhaseDrop,
    LostTerminalFlight,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn terminal_close_must_follow_fin_and_preserve_original_expiry_and_cancellation() {
    for fault in [
        TerminalFault::EarlyZero,
        TerminalFault::Nonzero,
        TerminalFault::MissingClose,
        TerminalFault::Cancel,
    ] {
        terminal_fault(fault).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn terminal_flight_loss_never_substitutes_local_fin_for_observed_remote_close() {
    for fault in [
        TerminalFault::ClosePhaseDrop,
        TerminalFault::LostTerminalFlight,
    ] {
        terminal_fault(fault).await;
    }
}

async fn terminal_fault(fault: TerminalFault) {
    let (server_identity, client_identity) = identities();
    let endpoint = raw_object_listener(server_identity);
    let mut proxy = AckGate::start(endpoint.local_addr().unwrap()).await;
    let watchdog = Instant::now() + Duration::from_secs(5);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let discard = proxy.discard_next_server.clone();
    let discard_flight = proxy.discard_server_flight.clone();
    let responder = tokio::spawn(async move {
        let mut release_rx = release_rx;
        let connection = endpoint.accept().await.unwrap().await.unwrap();
        let (mut send, mut receive) = connection.accept_bi().await.unwrap();
        let mut prefix = [255; 4];
        receive.read_exact(&mut prefix).await.unwrap();
        assert_eq!(prefix, [0; 4]);
        if !matches!(fault, TerminalFault::EarlyZero) {
            assert!(receive.read(&mut [0; 1]).await.unwrap().is_none());
            send.write_all(&[0; 4]).await.unwrap();
            send.finish().unwrap();
            assert_eq!(send.stopped().await.unwrap(), None);
        }
        let _ = ready_tx.send(());
        if matches!(fault, TerminalFault::MissingClose | TerminalFault::Cancel) {
            let _ = (&mut release_rx).await;
        }
        if matches!(fault, TerminalFault::ClosePhaseDrop) {
            discard.store(true, std::sync::atomic::Ordering::Release);
        }
        if matches!(fault, TerminalFault::LostTerminalFlight) {
            discard_flight.store(true, std::sync::atomic::Ordering::Release);
        }
        connection.close(
            if matches!(fault, TerminalFault::Nonzero) {
                7u8
            } else {
                0u8
            }
            .into(),
            b"owned terminal fixture",
        );
        endpoint.wait_idle().await;
        if !matches!(fault, TerminalFault::MissingClose | TerminalFault::Cancel) {
            let _ = release_rx.await;
        }
    });
    let client = Peer::connect_objects(
        "127.0.0.1:0".parse().unwrap(),
        proxy.address,
        "gateway.test",
        client_identity,
        watchdog,
    )
    .await
    .unwrap();
    let cancel = CancellationToken::new();
    let mut channel = client.open_object_channel(watchdog, &cancel).await.unwrap();
    let original = Instant::now() + Duration::from_millis(600);
    let signal = cancel.clone();
    let initiator = tokio::spawn(async move {
        let result = channel.finish(original, &cancel).await;
        let finished_at = Instant::now();
        (result, channel, finished_at)
    });
    let ready = tokio::time::timeout_at(watchdog, ready_rx).await;
    let close_discarded = if matches!(
        fault,
        TerminalFault::ClosePhaseDrop | TerminalFault::LostTerminalFlight
    ) {
        tokio::time::timeout_at(watchdog, proxy.server_discarded.notified())
            .await
            .is_ok()
    } else {
        false
    };
    if matches!(fault, TerminalFault::Cancel) {
        signal.cancel();
    }
    let completed = tokio::time::timeout_at(watchdog, initiator).await;
    // Keep the raw endpoint/socket alive until the proxy has stopped, including
    // the negative path's client abort; otherwise Linux can inject ICMP refusal.
    proxy.stop.cancel();
    let traffic = tokio::time::timeout(Duration::from_secs(1), proxy.task.take().unwrap()).await;
    let _ = release_tx.send(());
    let responder = tokio::time::timeout_at(watchdog, responder).await;
    let (result, mut channel, finished_at) = completed.unwrap().unwrap();
    let traffic = traffic.unwrap().unwrap().unwrap();
    if let Some(directory) = std::env::var_os("SPIRALCORE_OBJECT_FINISH_EVIDENCE_DIR") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(std::path::PathBuf::from(directory).join(format!("terminal-{fault:?}.json")))
            .unwrap();
        serde_json::to_writer_pretty(
            file,
            &serde_json::json!({
                "scope":"real pinned-mTLS QUIC controlled terminal peer; no WAN claim",
                "fault":format!("{fault:?}"),"result":format!("{result:?}"),"traffic":traffic,
            "close_phase_datagram_discard_observed":close_discarded,
                "framing_sent":channel.counters().framing_bytes_sent,
                "framing_received":channel.counters().framing_bytes_received,
                "original_cap_ms":600,"completion_after_original_cap":finished_at>=original,
                "owned_responder_joined":responder.is_ok()
            }),
        )
        .unwrap();
    }
    ready.unwrap().unwrap();
    responder.unwrap().unwrap();
    if matches!(
        fault,
        TerminalFault::ClosePhaseDrop | TerminalFault::LostTerminalFlight
    ) {
        assert!(close_discarded);
        assert!(traffic.server_discarded > 0);
        assert_eq!(
            traffic.server_received,
            traffic.server_forwarded + traffic.server_discarded
        );
        if matches!(fault, TerminalFault::ClosePhaseDrop) {
            assert_eq!(traffic.server_discarded, 1);
        }
    }
    match fault {
        TerminalFault::MissingClose | TerminalFault::LostTerminalFlight => {
            assert!(matches!(result, Err(Error::Deadline)), "{result:?}")
        }
        TerminalFault::Cancel => assert!(matches!(result, Err(Error::Cancelled)), "{result:?}"),
        TerminalFault::EarlyZero => assert_eq!(
            result.as_ref().err().and_then(application_close_code),
            Some(0)
        ),
        TerminalFault::Nonzero => assert_eq!(
            result.as_ref().err().and_then(application_close_code),
            Some(7)
        ),
        TerminalFault::ClosePhaseDrop => assert!(
            result.is_ok(),
            "a later valid close must actually be observed: {result:?}"
        ),
    }
    assert!(finished_at <= original + Duration::from_millis(150));
    assert_eq!(channel.counters().framing_bytes_sent, 4);
    if !matches!(fault, TerminalFault::EarlyZero) {
        assert_eq!(channel.counters().framing_bytes_received, 4);
    }
    assert!(matches!(
        channel
            .recv_batch(watchdog, &CancellationToken::new())
            .await,
        Err(Error::Poisoned)
    ));
}

fn application_close_code(error: &Error) -> Option<u64> {
    match error {
        Error::Transport(spiralcore_gateway::transport::Error::Connection(
            quinn::ConnectionError::ApplicationClosed(close),
        ))
        | Error::Stopped(quinn::StoppedError::ConnectionLost(
            quinn::ConnectionError::ApplicationClosed(close),
        ))
        | Error::Read(quinn::ReadExactError::ReadError(quinn::ReadError::ConnectionLost(
            quinn::ConnectionError::ApplicationClosed(close),
        )))
        | Error::End(quinn::ReadError::ConnectionLost(
            quinn::ConnectionError::ApplicationClosed(close),
        )) => Some(close.error_code.into_inner()),
        _ => None,
    }
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
