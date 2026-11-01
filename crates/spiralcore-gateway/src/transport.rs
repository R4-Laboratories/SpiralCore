//! One pinned, mutually authenticated QUIC peer carrying bounded datagrams.
use bytes::Bytes;
use quinn::{
    crypto::rustls::{QuicClientConfig, QuicServerConfig},
    Endpoint,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use sha2::{Digest, Sha256};
use spiralcore_gateway_core::packet::{MAX_DATAGRAM, MIN_DATAGRAM};
use std::{net::SocketAddr, sync::Arc};
use tokio::time::{timeout_at, Instant};

pub const ALPN: &[u8] = b"spiralcore-gateway/1";
const QUEUE_BYTES: usize = 128 * 1024;
const HANDSHAKE_BYTES: u64 = 64 * 1024;

/// Secrets are owned and intentionally have no Debug implementation.
pub struct Identity {
    pub certificate_chain: Vec<CertificateDer<'static>>,
    pub private_key: PrivateKeyDer<'static>,
    pub trusted_roots: Vec<CertificateDer<'static>>,
    pub peer_leaf_sha256: [u8; 32],
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid QUIC configuration: {0}")]
    Configuration(&'static str),
    #[error("QUIC socket I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("TLS configuration: {0}")]
    Tls(#[from] rustls::Error),
    #[error("QUIC connect: {0}")]
    Connect(#[from] quinn::ConnectError),
    #[error("QUIC connection: {0}")]
    Connection(#[from] quinn::ConnectionError),
    #[error("QUIC datagram send: {0}")]
    Send(#[from] quinn::SendDatagramError),
    #[error("transport deadline expired")]
    Deadline,
    #[error("peer certificate pin mismatch")]
    PinMismatch,
    #[error("gateway ALPN was not negotiated")]
    Protocol,
    #[error("peer datagram capability is unavailable or below the required minimum")]
    DatagramUnavailable,
    #[error("datagram length exceeds the negotiated or protocol bound")]
    DatagramLength,
}
pub type Result<T> = std::result::Result<T, Error>;

/// Bytes inside UDP datagrams, including connection handshake/control traffic.
/// UDP/IP headers and endpoint-stateless traffic are outside these counters.
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct UdpCounters {
    pub sent_datagrams: u64,
    pub sent_bytes: u64,
    pub received_datagrams: u64,
    pub received_bytes: u64,
}

fn transport_config() -> quinn::TransportConfig {
    let mut config = quinn::TransportConfig::default();
    config
        .max_concurrent_uni_streams(0u8.into())
        .max_concurrent_bidi_streams(0u8.into())
        .max_idle_timeout(Some(quinn::VarInt::from_u32(30_000).into()))
        .datagram_receive_buffer_size(Some(QUEUE_BYTES))
        .datagram_send_buffer_size(QUEUE_BYTES);
    config
}

fn roots(certificates: Vec<CertificateDer<'static>>) -> Result<rustls::RootCertStore> {
    if certificates.is_empty() || certificates.len() > 4 {
        return Err(Error::Configuration("one to four trust anchors required"));
    }
    let mut roots = rustls::RootCertStore::empty();
    for certificate in certificates {
        if certificate.len() > 16 * 1024 {
            return Err(Error::Configuration("trust anchor exceeds 16 KiB"));
        }
        roots.add(certificate)?;
    }
    Ok(roots)
}

fn check_chain(chain: &[CertificateDer<'static>]) -> Result<()> {
    if chain.is_empty() || chain.len() > 4 || chain.iter().any(|c| c.len() > 16 * 1024) {
        return Err(Error::Configuration("certificate chain exceeds bounds"));
    }
    Ok(())
}

fn server_config(identity: Identity) -> Result<(quinn::ServerConfig, [u8; 32])> {
    check_chain(&identity.certificate_chain)?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
        Arc::new(roots(identity.trusted_roots)?),
        Arc::clone(&provider),
    )
    .build()
    .map_err(|_| Error::Configuration("client certificate verifier"))?;
    let mut tls = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_client_cert_verifier(verifier)
        .with_single_cert(identity.certificate_chain, identity.private_key)?;
    tls.alpn_protocols = vec![ALPN.to_vec()];
    tls.max_early_data_size = 0;
    tls.send_tls13_tickets = 0;
    let crypto = QuicServerConfig::try_from(tls)
        .map_err(|_| Error::Configuration("TLS 1.3 QUIC server cipher"))?;
    let mut config = quinn::ServerConfig::with_crypto(Arc::new(crypto));
    config
        .transport_config(Arc::new(transport_config()))
        .max_incoming(1)
        .incoming_buffer_size(HANDSHAKE_BYTES)
        .incoming_buffer_size_total(HANDSHAKE_BYTES);
    Ok((config, identity.peer_leaf_sha256))
}

fn client_config(identity: Identity) -> Result<(quinn::ClientConfig, [u8; 32])> {
    check_chain(&identity.certificate_chain)?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut tls = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_root_certificates(roots(identity.trusted_roots)?)
        .with_client_auth_cert(identity.certificate_chain, identity.private_key)?;
    tls.alpn_protocols = vec![ALPN.to_vec()];
    tls.enable_early_data = false;
    tls.resumption = rustls::client::Resumption::disabled();
    let crypto = QuicClientConfig::try_from(tls)
        .map_err(|_| Error::Configuration("TLS 1.3 QUIC client cipher"))?;
    let mut config = quinn::ClientConfig::new(Arc::new(crypto));
    config.transport_config(Arc::new(transport_config()));
    Ok((config, identity.peer_leaf_sha256))
}

fn before_deadline(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        Err(Error::Deadline)
    } else {
        Ok(())
    }
}

fn stop(endpoint: &Endpoint) {
    endpoint.close(quinn::VarInt::from_u32(1), b"gateway closed");
}

pub struct Listener {
    endpoint: Endpoint,
    pin: [u8; 32],
}
impl Listener {
    pub fn bind(local: SocketAddr, identity: Identity) -> Result<Self> {
        let (config, pin) = server_config(identity)?;
        Ok(Self {
            endpoint: Endpoint::server(config, local)?,
            pin,
        })
    }
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.endpoint.local_addr()?)
    }
    /// Consume the single-peer listener; any failure closes its endpoint.
    pub async fn accept(self, deadline: Instant) -> Result<Peer> {
        let result = async {
            before_deadline(deadline)?;
            let incoming = timeout_at(deadline, self.endpoint.accept())
                .await
                .map_err(|_| Error::Deadline)?
                .ok_or(Error::Configuration("listener closed"))?;
            self.endpoint.set_server_config(None);
            let connection = timeout_at(deadline, incoming)
                .await
                .map_err(|_| Error::Deadline)??;
            before_deadline(deadline)?;
            verify_peer(&connection, self.pin)?;
            Ok(connection)
        }
        .await;
        match result {
            Ok(connection) => Ok(Peer {
                endpoint: self.endpoint,
                connection,
            }),
            Err(error) => {
                stop(&self.endpoint);
                Err(error)
            }
        }
    }
}

pub struct Peer {
    endpoint: Endpoint,
    connection: quinn::Connection,
}
impl Peer {
    pub async fn connect(
        local: SocketAddr,
        remote: SocketAddr,
        server_name: &str,
        identity: Identity,
        deadline: Instant,
    ) -> Result<Self> {
        before_deadline(deadline)?;
        let (config, pin) = client_config(identity)?;
        let mut endpoint = Endpoint::client(local)?;
        endpoint.set_default_client_config(config);
        let result = async {
            let connecting = endpoint.connect(remote, server_name)?;
            let connection = timeout_at(deadline, connecting)
                .await
                .map_err(|_| Error::Deadline)??;
            before_deadline(deadline)?;
            verify_peer(&connection, pin)?;
            Ok(connection)
        }
        .await;
        match result {
            Ok(connection) => Ok(Self {
                endpoint,
                connection,
            }),
            Err(error) => {
                stop(&endpoint);
                Err(error)
            }
        }
    }
    pub fn max_datagram_size(&self) -> usize {
        self.connection
            .max_datagram_size()
            .unwrap_or(0)
            .min(MAX_DATAGRAM)
    }
    pub async fn send_datagram(&self, bytes: Bytes, deadline: Instant) -> Result<()> {
        before_deadline(deadline)?;
        let limit = self.max_datagram_size();
        if limit < MIN_DATAGRAM {
            return Err(Error::DatagramUnavailable);
        }
        if bytes.is_empty() || bytes.len() > limit {
            return Err(Error::DatagramLength);
        }
        timeout_at(deadline, self.connection.send_datagram_wait(bytes))
            .await
            .map_err(|_| Error::Deadline)??;
        Ok(())
    }
    pub async fn receive_datagram(&self, deadline: Instant) -> Result<Bytes> {
        before_deadline(deadline)?;
        let bytes = timeout_at(deadline, self.connection.read_datagram())
            .await
            .map_err(|_| Error::Deadline)??;
        before_deadline(deadline)?;
        if bytes.is_empty() || bytes.len() > MAX_DATAGRAM {
            stop(&self.endpoint);
            return Err(Error::DatagramLength);
        }
        Ok(bytes)
    }
    pub fn udp_counters(&self) -> UdpCounters {
        let stats = self.connection.stats();
        UdpCounters {
            sent_datagrams: stats.udp_tx.datagrams,
            sent_bytes: stats.udp_tx.bytes,
            received_datagrams: stats.udp_rx.datagrams,
            received_bytes: stats.udp_rx.bytes,
        }
    }
    pub fn close(&self) {
        self.endpoint
            .close(quinn::VarInt::from_u32(0), b"gateway closed");
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.close();
    }
}

fn verify_peer(connection: &quinn::Connection, pin: [u8; 32]) -> Result<()> {
    let certificates = connection
        .peer_identity()
        .and_then(|id| id.downcast::<Vec<CertificateDer<'static>>>().ok())
        .ok_or(Error::PinMismatch)?;
    let leaf = certificates.first().ok_or(Error::PinMismatch)?;
    let actual: [u8; 32] = Sha256::digest(leaf.as_ref()).into();
    if actual != pin {
        return Err(Error::PinMismatch);
    }
    let handshake = connection
        .handshake_data()
        .and_then(|data| data.downcast::<quinn::crypto::rustls::HandshakeData>().ok())
        .ok_or(Error::Protocol)?;
    if handshake.protocol.as_deref() != Some(ALPN) {
        return Err(Error::Protocol);
    }
    if connection
        .max_datagram_size()
        .is_none_or(|limit| limit < MIN_DATAGRAM)
    {
        return Err(Error::DatagramUnavailable);
    }
    Ok(())
}
