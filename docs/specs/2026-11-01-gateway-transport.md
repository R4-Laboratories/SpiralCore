# Gateway QUIC and Linux TUN contract

This is the transport boundary for the shared Rust gateway. The packet codec and
bounded fragment reassembly are owned by `spiralcore-gateway-core`; this boundary
does not decode packets, execute recipes, administer interfaces or change routes.

QUIC uses pinned Quinn 0.11.12 and rustls 0.23 with the explicit ring provider,
TLS 1.3 and ALPN `spiralcore-gateway/1`. Both ends use the standard rustls
certificate verifier, require a peer certificate chained to configured private
trust anchors, and require its exact leaf DER SHA-256 pin before exposing any
application data. The client also verifies its configured server name. There is
no allow-any verifier, anonymous client mode, early data, resumption or 0-RTT.
Private keys are owned values and never implement Debug through this API.

The public `transport` API is:

```rust
pub struct Identity {
    pub certificate_chain: Vec<rustls::pki_types::CertificateDer<'static>>,
    pub private_key: rustls::pki_types::PrivateKeyDer<'static>,
    pub trusted_roots: Vec<rustls::pki_types::CertificateDer<'static>>,
    pub peer_leaf_sha256: [u8; 32],
}
pub struct UdpCounters {
    pub sent_datagrams: u64,
    pub sent_bytes: u64,
    pub received_datagrams: u64,
    pub received_bytes: u64,
}
pub struct Listener { /* private */ }
impl Listener {
    pub fn bind(local: std::net::SocketAddr, identity: Identity) -> Result<Self>;
    pub fn local_addr(&self) -> Result<std::net::SocketAddr>;
    pub async fn accept(self, deadline: tokio::time::Instant) -> Result<Peer>;
}
pub struct Peer { /* private; owns its endpoint */ }
impl Peer {
    pub async fn connect(local: std::net::SocketAddr,
        remote: std::net::SocketAddr, server_name: &str, identity: Identity,
        deadline: tokio::time::Instant) -> Result<Self>;
    pub fn max_datagram_size(&self) -> usize;
    pub async fn send_datagram(&self, bytes: bytes::Bytes,
        deadline: tokio::time::Instant) -> Result<()>;
    pub async fn receive_datagram(&self,
        deadline: tokio::time::Instant) -> Result<bytes::Bytes>;
    pub fn udp_counters(&self) -> UdpCounters;
    pub fn close(&self);
}
```

`Result` uses a typed transport error: configuration, I/O, TLS, QUIC connect or
connection failure, deadline, certificate-pin mismatch, unavailable/undersized
datagrams, or datagram length violation. Error messages never contain key bytes.
Deadline expiry cancels the pending operation; an incomplete handshake closes its
owned endpoint. A pin/protocol-size failure closes the connection before use.
An accepted send means queued for QUIC transmission, not acknowledged delivery.
Datagrams may be lost or reordered. No application reliability is invented here.
Explicit normal `Peer::close` uses application close code 0. Authentication or
malformed-datagram refusal uses nonzero code 1 and is not a graceful completion.

Only one peer connection is exposed. The listening endpoint admits at most one
pending handshake, with at most 64 KiB pending-handshake buffering; accepting the
attempt disables further incoming connections on that endpoint. A failed attempt
ends that listener instance. Uni- and bidirectional streams are disabled in this
packet-only version. Datagram send and receive queues are each capped at 128 KiB;
idle timeout is 30 seconds. No arbitrary remote rate or queue setting is accepted.

The core contract fixes an IP MTU of 1280, a 56-byte fragment header and at most
two fragments. Each QUIC application datagram is 1..1336 bytes, and the negotiated
maximum must be at least 700 bytes. `max_datagram_size()` is the current Quinn
limit capped at 1336; the encoder uses it when splitting packets. A later path
limit reduction is checked again at send time. This preserves IPv6's 1280-byte
minimum rather than quietly shrinking the IP MTU to the QUIC payload ceiling.

UDP counters are snapshots of Quinn's `ConnectionStats.udp_tx/udp_rx`, including
handshake and other QUIC traffic attributed to that connection. They count bytes
inside UDP datagrams, excluding UDP/IP/link headers and stateless traffic that is
not attributed to the accepted connection. Application byte counts and queue
admission are separate. Sampling before close is not a claim to include later
close/drain packets; the runner must label its measurement interval.

Linux-only `tun` exports:

```rust
pub const MTU: usize = 1280;
pub struct Tun { /* private AsyncFd<OwnedFd> */ }
impl Tun {
    pub fn open(name: &str) -> std::io::Result<Self>;
    pub fn from_owned_fd(fd: std::os::fd::OwnedFd) -> std::io::Result<Self>;
    pub fn name(&self) -> &str;
    pub async fn read_packet(&self) -> std::io::Result<Vec<u8>>;
    pub async fn write_packet(&self, packet: &[u8]) -> std::io::Result<()>;
}
```

The interface is configured externally in an isolated Linux network namespace,
with MTU 1280. `open` attaches `/dev/net/tun` using `IFF_TUN | IFF_NO_PI` and a
validated explicit name; `from_owned_fd` owns and verifies an existing TUN FD.
Both establish nonblocking and close-on-exec behavior. The adapter checks the
actual interface MTU and TUN/no-PI flags, and does not create addresses or routes.
Reads reserve at most 1281 bytes so oversized packets are rejected. Writes accept
only 1..1280 bytes and must complete as one packet; a short write fails instead
of sending a suffix as a second packet. Tokio readiness handles WouldBlock and
interruption without blocking the executor. Packet validity, authorization,
fragment expiry and final exact-byte validation belong to the core/service.

Tests first prove a real loopback UDP connection with mutual certificate
authentication, exact datagram exchange and nonzero handshake-inclusive counters.
Then reject wrong peer pins, missing/untrusted client identity and invalid or
oversized datagrams, and exercise deadlines. Linux TUN tests are explicitly
separated from portable transport tests; actual TUN qualification needs the
isolated privileged namespace runner, never changes to macOS routes. This is not
FIPS validation, a production authorization policy or an unrestricted IP router.
