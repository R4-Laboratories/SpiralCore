//! A single TLS1.3 session on an already admitted TCP connection.
use rustls::{
    DigitallySignedStruct, DistinguishedName, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{CryptoProvider, verify_tls13_signature_with_raw_key},
    pki_types::{
        CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, SubjectPublicKeyInfoDer,
        UnixTime,
    },
    server::danger::{ClientCertVerified, ClientCertVerifier},
    sign::CertifiedKey,
};
use sha2::{Digest, Sha256};
use std::{
    fmt,
    io::{self, Read, Write},
    net::{Shutdown, SocketAddr, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
const ALPN: &[u8] = b"spiralcore-flowmux/object-stream/1";
const READY: &[u8; 8] = b"SCREADY1";
fn error(message: impl ToString) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}
fn timeout() -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, "TLS carrier deadline")
}
fn transient(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted | io::ErrorKind::TimedOut
    )
}
fn pause(until: Instant) -> io::Result<()> {
    let remaining = until.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(timeout());
    }
    std::thread::park_timeout(remaining.min(Duration::from_millis(1)));
    Ok(())
}
#[derive(Clone)]
pub struct PinnedIdentity {
    key: Arc<CertifiedKey>,
    pin: [u8; 32],
    provider: Arc<CryptoProvider>,
}
impl fmt::Debug for PinnedIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PinnedIdentity")
            .field("spki_sha256", &self.pin)
            .finish_non_exhaustive()
    }
}
impl PinnedIdentity {
    pub fn from_pkcs8(bytes: Vec<u8>) -> io::Result<Self> {
        Self::from_private_key(PrivatePkcs8KeyDer::from(bytes).into())
    }
    pub fn from_private_key(key: PrivateKeyDer<'static>) -> io::Result<Self> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let signing = provider.key_provider.load_private_key(key).map_err(error)?;
        let spki = signing
            .public_key()
            .ok_or_else(|| error("provider has no public SPKI"))?
            .to_vec();
        let pin = Sha256::digest(&spki).into();
        Ok(Self {
            key: Arc::new(CertifiedKey::new(vec![CertificateDer::from(spki)], signing)),
            pin,
            provider,
        })
    }
    pub fn spki_sha256(&self) -> [u8; 32] {
        self.pin
    }
}
#[derive(Debug)]
struct Pin {
    digest: [u8; 32],
    provider: Arc<CryptoProvider>,
}
impl Pin {
    fn check(
        &self,
        cert: &CertificateDer<'_>,
        chain: &[CertificateDer<'_>],
    ) -> Result<(), rustls::Error> {
        let actual: [u8; 32] = Sha256::digest(cert.as_ref()).into();
        if !chain.is_empty() || actual != self.digest {
            return Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            ));
        }
        Ok(())
    }
    fn signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature_with_raw_key(
            message,
            &SubjectPublicKeyInfoDer::from(cert.as_ref()),
            signature,
            &self.provider.signature_verification_algorithms,
        )
    }
    fn schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}
impl ServerCertVerifier for Pin {
    fn verify_server_cert(
        &self,
        cert: &CertificateDer<'_>,
        chain: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        self.check(cert, chain)?;
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::General("TLS1.2 disabled".into()))
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.signature(message, cert, signature)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes()
    }
    fn requires_raw_public_keys(&self) -> bool {
        true
    }
}
impl ClientCertVerifier for Pin {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }
    fn verify_client_cert(
        &self,
        cert: &CertificateDer<'_>,
        chain: &[CertificateDer<'_>],
        _: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        self.check(cert, chain)?;
        Ok(ClientCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::General("TLS1.2 disabled".into()))
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.signature(message, cert, signature)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes()
    }
    fn requires_raw_public_keys(&self) -> bool {
        true
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TlsCounters {
    pub received: u64,
    pub sent: u64,
}
struct Session {
    tls: rustls::Connection,
    socket: TcpStream,
}
struct Shared {
    session: Mutex<Session>,
    control: TcpStream,
    stopped: AtomicBool,
    received: AtomicU64,
    sent: AtomicU64,
    exporter: [u8; 32],
    local_pin: [u8; 32],
    peer_pin: [u8; 32],
    role: crate::Role,
}
impl Shared {
    fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
        let _ = self.control.shutdown(Shutdown::Both);
    }
    fn check(&self) -> io::Result<()> {
        if self.stopped.load(Ordering::Acquire) {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "TLS carrier closed",
            ))
        } else {
            Ok(())
        }
    }
}
impl Drop for Shared {
    fn drop(&mut self) {
        let _ = self.control.shutdown(Shutdown::Both);
    }
}
pub struct TlsCarrier {
    shared: Arc<Shared>,
    read_timeout: Duration,
    write_timeout: Duration,
}
impl fmt::Debug for TlsCarrier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TlsCarrier")
            .field("counters", &self.counters())
            .finish_non_exhaustive()
    }
}
fn add(counter: &AtomicU64, n: usize) -> io::Result<()> {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(n as u64)
        })
        .map(|_| ())
        .map_err(|_| error("TLS counter overflow"))
}
fn pump(session: &mut Session, received: &AtomicU64, sent: &AtomicU64) -> io::Result<()> {
    let Session { tls, socket } = session;
    while tls.wants_write() {
        match tls.write_tls(socket) {
            Ok(0) => return Err(io::Error::new(io::ErrorKind::WriteZero, "TLS write zero")),
            Ok(n) => add(sent, n)?,
            Err(e) if transient(&e) => break,
            Err(e) => return Err(e),
        }
    }
    if tls.wants_read() {
        match tls.read_tls(socket) {
            Ok(0) if tls.is_handshaking() => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "TLS peer closed before handshake completed",
                ));
            }
            Ok(n) => {
                add(received, n)?;
                tls.process_new_packets().map_err(error)?;
            }
            Err(e) if transient(&e) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
impl TlsCarrier {
    pub fn connect(
        socket: TcpStream,
        identity: &PinnedIdentity,
        peer_pin: [u8; 32],
        server_name: &str,
        until: Instant,
    ) -> io::Result<Self> {
        let verifier = Arc::new(Pin {
            digest: peer_pin,
            provider: identity.provider.clone(),
        });
        let mut config = rustls::ClientConfig::builder_with_provider(identity.provider.clone())
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(error)?
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_client_cert_resolver(Arc::new(
                rustls::client::AlwaysResolvesClientRawPublicKeys::new(identity.key.clone()),
            ));
        config.alpn_protocols = vec![ALPN.to_vec()];
        config.resumption = rustls::client::Resumption::disabled();
        config.enable_early_data = false;
        let name = ServerName::try_from(server_name.to_owned()).map_err(error)?;
        let tls = rustls::ClientConnection::new(Arc::new(config), name).map_err(error)?;
        let mut stream = Self::handshake(
            socket,
            tls.into(),
            until,
            identity.pin,
            peer_pin,
            crate::Role::Initiator,
        )?;
        let mut marker = [0; 8];
        if let Err(e) = stream.read_exact_until(&mut marker, until) {
            stream.shared.stop();
            return Err(e);
        }
        if &marker != READY {
            stream.shared.stop();
            return Err(error("TLS profile readiness mismatch"));
        }
        Ok(stream)
    }
    pub fn accept(
        socket: TcpStream,
        identity: &PinnedIdentity,
        peer_pin: [u8; 32],
        until: Instant,
    ) -> io::Result<Self> {
        let verifier = Arc::new(Pin {
            digest: peer_pin,
            provider: identity.provider.clone(),
        });
        let mut config = rustls::ServerConfig::builder_with_provider(identity.provider.clone())
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(error)?
            .with_client_cert_verifier(verifier)
            .with_cert_resolver(Arc::new(
                rustls::server::AlwaysResolvesServerRawPublicKeys::new(identity.key.clone()),
            ));
        config.alpn_protocols = vec![ALPN.to_vec()];
        config.send_tls13_tickets = 0;
        config.max_early_data_size = 0;
        config.session_storage = Arc::new(rustls::server::NoServerSessionStorage {});
        let tls = rustls::ServerConnection::new(Arc::new(config)).map_err(error)?;
        let mut stream = Self::handshake(
            socket,
            tls.into(),
            until,
            identity.pin,
            peer_pin,
            crate::Role::Acceptor,
        )?;
        if let Err(e) = stream
            .write_all_until(READY, until)
            .and_then(|()| stream.flush_until(until))
        {
            stream.shared.stop();
            return Err(e);
        }
        Ok(stream)
    }
    fn handshake(
        socket: TcpStream,
        mut tls: rustls::Connection,
        until: Instant,
        local_pin: [u8; 32],
        peer_pin: [u8; 32],
        role: crate::Role,
    ) -> io::Result<Self> {
        if Instant::now() >= until
            || until.saturating_duration_since(Instant::now()) > Duration::from_secs(30)
        {
            return Err(timeout());
        }
        socket.set_nonblocking(true)?;
        socket.set_nodelay(true)?;
        tls.set_buffer_limit(Some(65536));
        let mut session = Session { tls, socket };
        let received = AtomicU64::new(0);
        let sent = AtomicU64::new(0);
        let outcome = (|| {
            loop {
                if Instant::now() >= until {
                    return Err(timeout());
                }
                pump(&mut session, &received, &sent)?;
                if !session.tls.is_handshaking() && !session.tls.wants_write() {
                    break;
                }
                pause(until)?;
            }
            if session.tls.alpn_protocol() != Some(ALPN)
                || session.tls.protocol_version() != Some(rustls::ProtocolVersion::TLSv1_3)
            {
                return Err(error("TLS required profile not negotiated"));
            }
            session
                .tls
                .export_keying_material([0; 32], b"EXPORTER-SpiralCore-FlowMux-v1", Some(ALPN))
                .map_err(error)
        })();
        let exporter = match outcome {
            Ok(value) => value,
            Err(e) => {
                let _ = session.socket.shutdown(Shutdown::Both);
                return Err(e);
            }
        };
        let control = session.socket.try_clone()?;
        Ok(Self {
            shared: Arc::new(Shared {
                session: Mutex::new(session),
                control,
                stopped: AtomicBool::new(false),
                received,
                sent,
                exporter,
                local_pin,
                peer_pin,
                role,
            }),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
        })
    }
    pub fn local_spki_sha256(&self) -> [u8; 32] {
        self.shared.local_pin
    }
    pub fn peer_spki_sha256(&self) -> [u8; 32] {
        self.shared.peer_pin
    }
    pub fn role(&self) -> crate::Role {
        self.shared.role
    }
    pub fn exporter(&self) -> [u8; 32] {
        self.shared.exporter
    }
    pub fn counters(&self) -> TlsCounters {
        TlsCounters {
            received: self.shared.received.load(Ordering::Relaxed),
            sent: self.shared.sent.load(Ordering::Relaxed),
        }
    }
    pub fn try_clone(&self) -> io::Result<Self> {
        self.shared.check()?;
        Ok(Self {
            shared: self.shared.clone(),
            read_timeout: self.read_timeout,
            write_timeout: self.write_timeout,
        })
    }
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.shared.control.peer_addr()
    }
    pub fn peek(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        use std::io::BufRead;
        if buffer.is_empty() {
            return Ok(0);
        }
        let until = Instant::now() + self.read_timeout;
        loop {
            if Instant::now() >= until {
                return Err(timeout());
            }
            let result = self.with_session(|session| {
                pump(session, &self.shared.received, &self.shared.sent)?;
                let mut reader = session.tls.reader();
                let available = reader.fill_buf()?;
                let n = available.len().min(buffer.len());
                buffer[..n].copy_from_slice(&available[..n]);
                Ok(n)
            });
            match result {
                Err(e) if transient(&e) => pause(until)?,
                other => return other,
            }
        }
    }
    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.read_timeout = budget(timeout)?;
        Ok(())
    }
    pub fn set_write_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.write_timeout = budget(timeout)?;
        Ok(())
    }
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        match how {
            Shutdown::Both => {
                self.shared.stop();
                Ok(())
            }
            Shutdown::Read => self.shared.control.shutdown(how),
            Shutdown::Write => {
                let until = Instant::now() + self.write_timeout;
                loop {
                    if Instant::now() >= until {
                        return Err(timeout());
                    }
                    match self.with_session(|session| {
                        session.tls.send_close_notify();
                        Ok(())
                    }) {
                        Err(e) if transient(&e) => pause(until)?,
                        outcome => {
                            outcome?;
                            break;
                        }
                    }
                }
                self.flush_until(until)?;
                self.shared.control.shutdown(how)
            }
        }
    }
    fn with_session<T>(
        &self,
        operation: impl FnOnce(&mut Session) -> io::Result<T>,
    ) -> io::Result<T> {
        self.shared.check()?;
        let mut guard = match self.shared.session.try_lock() {
            Ok(value) => value,
            Err(std::sync::TryLockError::WouldBlock) => {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            Err(std::sync::TryLockError::Poisoned(_)) => return Err(error("TLS poisoned")),
        };
        let result = operation(&mut guard);
        drop(guard);
        if let Err(e) = &result
            && !transient(e)
        {
            self.shared.stop();
        }
        result
    }
    fn read_until(&mut self, buffer: &mut [u8], until: Instant) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        loop {
            if Instant::now() >= until {
                return Err(timeout());
            }
            let result = self.with_session(|session| {
                match session.tls.reader().read(buffer) {
                    Ok(n) => return Ok(n),
                    Err(e) if transient(&e) => {}
                    Err(e) => return Err(e),
                };
                pump(session, &self.shared.received, &self.shared.sent)?;
                session.tls.reader().read(buffer)
            });
            match result {
                Err(e) if transient(&e) => pause(until)?,
                other => return other,
            }
        }
    }
    fn read_exact_until(&mut self, mut buffer: &mut [u8], until: Instant) -> io::Result<()> {
        while !buffer.is_empty() {
            let n = self.read_until(buffer, until)?;
            if n == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            buffer = &mut buffer[n..];
        }
        Ok(())
    }
    fn write_until(&mut self, buffer: &[u8], until: Instant) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        loop {
            if Instant::now() >= until {
                return Err(timeout());
            }
            let result = self.with_session(|session| {
                pump(session, &self.shared.received, &self.shared.sent)?;
                let n = session.tls.writer().write(buffer)?;
                pump(session, &self.shared.received, &self.shared.sent)?;
                if n == 0 {
                    return Err(io::ErrorKind::WouldBlock.into());
                }
                Ok(n)
            });
            match result {
                Err(e) if transient(&e) => pause(until)?,
                other => return other,
            }
        }
    }
    fn write_all_until(&mut self, mut buffer: &[u8], until: Instant) -> io::Result<()> {
        while !buffer.is_empty() {
            let n = self.write_until(buffer, until)?;
            buffer = &buffer[n..];
        }
        Ok(())
    }
    fn flush_until(&self, until: Instant) -> io::Result<()> {
        loop {
            if Instant::now() >= until {
                return Err(timeout());
            }
            let result = self.with_session(|session| {
                pump(session, &self.shared.received, &self.shared.sent)?;
                if session.tls.wants_write() {
                    Err(io::ErrorKind::WouldBlock.into())
                } else {
                    Ok(())
                }
            });
            match result {
                Err(e) if transient(&e) => pause(until)?,
                other => return other,
            }
        }
    }
}
fn budget(value: Option<Duration>) -> io::Result<Duration> {
    let value = value.unwrap_or(Duration::from_secs(30));
    if value.is_zero() || value > Duration::from_secs(30) {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "TLS timeout range",
        ))
    } else {
        Ok(value)
    }
}
impl Read for TlsCarrier {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.read_until(buffer, Instant::now() + self.read_timeout)
    }
}
impl Write for TlsCarrier {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.write_until(buffer, Instant::now() + self.write_timeout)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.flush_until(Instant::now() + self.write_timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn required_alpn_refuses_a_different_authenticated_tls_profile() {
        let make = || {
            let bytes =
                ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new())
                    .expect("key");
            PinnedIdentity::from_pkcs8(bytes.as_ref().to_vec()).expect("identity")
        };
        let client = make();
        let server = make();
        let client_pin = client.pin;
        let server_pin = server.pin;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener");
        let mut socket =
            TcpStream::connect(listener.local_addr().expect("address")).expect("connect");
        let (peer, _) = listener.accept().expect("accept");
        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("read timeout");
        socket
            .set_write_timeout(Some(Duration::from_secs(1)))
            .expect("write timeout");
        std::thread::scope(|scope| {
            let peer = scope.spawn(move || {
                TlsCarrier::accept(
                    peer,
                    &server,
                    client_pin,
                    Instant::now() + Duration::from_secs(1),
                )
            });
            let mut config = rustls::ClientConfig::builder_with_provider(client.provider.clone())
                .with_protocol_versions(&[&rustls::version::TLS13])
                .expect("versions")
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(Pin {
                    digest: server_pin,
                    provider: client.provider.clone(),
                }))
                .with_client_cert_resolver(Arc::new(
                    rustls::client::AlwaysResolvesClientRawPublicKeys::new(client.key),
                ));
            config.alpn_protocols = vec![b"different-required-profile/1".to_vec()];
            let mut tls = rustls::ClientConnection::new(
                Arc::new(config),
                ServerName::try_from("peer.example").expect("name"),
            )
            .expect("client");
            assert!(tls.complete_io(&mut socket).is_err());
            assert!(peer.join().expect("join").is_err());
        });
    }
}
