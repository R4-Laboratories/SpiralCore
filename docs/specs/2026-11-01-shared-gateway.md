# Shared Rust gateway and ordinary host traffic

The first delivery connects two ordinary hosts through an authenticated peer link
and connects the existing MVM object adapter to the same Rust object facade.
Linux TUN is a first-class adapter for selected IPv4/IPv6 traffic. The existing
MVM mailbox and FlowMux boundaries remain separate adapters; no guest NIC or
alternate MVM policy path is introduced. This implements the user's approved
gateway direction, with bounded tests and retained bytes, CPU and wall-time
measurements. It does not change this workstation's routes or create a local
Linux VM disk.

## Shared object interface

`spiralcore-gateway-core` is independent of the asynchronous network runtime. It
reuses `spiralcore-transfer` and contains no second mathematical engine, crypto
provider or durable object store. Its `object` module exposes:

```rust
enum ObjectMode { Raw, Whole, Warm }
struct ObjectSender { /* private scoped prepared state */ }

impl ObjectSender {
    fn new(mode: ObjectMode, context: &Context, schema: Option<&[u8]>)
        -> spiralcore_transfer::Result<Self>;
    fn seed(&mut self, context: &Context, bytes: Vec<u8>, transfer: Id,
        exchange: impl FnMut(&[Frame]) -> Result<Vec<Frame>>) -> Result<Stats>;
    fn seed_schema(&mut self, context: &Context, transfer: Id,
        exchange: impl FnMut(&[Frame]) -> Result<Vec<Frame>>) -> Result<Stats>;
    fn send(&mut self, context: &Context, bytes: Vec<u8>, transfer: Id,
        warm_probe: bool,
        exchange: impl FnMut(&[Frame]) -> Result<Vec<Frame>>) -> Result<Stats>;
}
```

All `Result`, `Id`, `Context`, `Frame` and `Stats` above are the existing transfer
types. `Warm` requires a schema artifact; `Raw` and `Whole` reject an unexpected
artifact. `seed` uses Raw in every arm; Warm records only its verified publication
as the base. `seed_schema` is supported only by Warm and preserves its rolling
object base. `send` delegates to the existing Raw sender, prepared whole-object
zstd sender with literal fallback, or `WarmSession`. Existing exact-repeat,
schema, ternary and residual behavior is preserved. Runtime checkpoint/prediction
transfers continue using the existing `RuntimeSession`, with no copy of that
engine. Objects remain bounded by the current 16 MiB batched sender ceiling.

The sender binds the complete existing context namespace at construction. A
different or malformed context refuses before exchange and discards prepared
state; callers construct a new sender for a new authorization scope. A namespace
check is not authentication. Network adapters must derive context from their
authenticated peer and configured authorization policy; MVM continues deriving
it from the verified canonical envelope and live capability. Receiver authority
remains `Endpoint<A>` with the existing current `Authority` check on every frame.
No public transport can turn asserted context fields into authority.

The existing SCMOBJ01 application frames, exact original length, whole-object
identity, reconstruction checks and durable publication contract are unchanged.
Object publication, mailbox acceptance and application processing remain distinct
events. A reliable stream provides framing and delivery; it does not manufacture
an application commit acknowledgment.

## Packet and trust boundaries

The packet lane carries selected IP traffic after application encryption. It can
carry TLS traffic exactly; it cannot reliably deduplicate or compress arbitrary
TLS ciphertext. A content API or explicitly authorized plaintext connector is
needed to exploit content before that encryption. The initial packet codec uses
independent per-packet literal or zstd encoding, without cross-packet cache
references; losing one packet cannot corrupt subsequent packets. Warm object
reuse remains on the reliable object lane.

IP packet transport uses authenticated QUIC datagrams rather than a reliable
outer TCP stream. Transport v1 disables unidirectional and bidirectional QUIC
streams. The reliable object facade is exercised through MVM's existing canonical
vsock channel and the durable object test harness; a reliable QUIC object service
is a later adapter, not delivered by this milestone. Both peers must agree on the
version and bounded framing before accepting data. A missing
peer does not inject encoded bytes into the Internet: an explicitly configured
ordinary forwarding path may be used only where policy authorizes it; otherwise
the adapter refuses. The first witness need not install that forwarding path.

Packet addresses and peer routes need explicit allowlists. A peer identity does
not grant arbitrary routing; MVM's FlowMux DNS, egress, transformation, rate and
audit checks remain its sole socket-opening policy boundary. Queue and cache
budgets are separate for packets and objects and partitioned by authorization
domain. The gateway is an explicitly trusted process for ordinary host plaintext;
the mailbox relay remains opaque unless separately authorized as a plaintext
endpoint. No production, WAN, FIPS/CUI or physical-energy claim follows from this
development witness.

## Packet v1 wire contract

The IP MTU is 1280 bytes for IPv4 and IPv6. QUIC's negotiated datagram payload
capacity must be at least 700 bytes; the encoder caps it at 1336. A packet is
encoded independently as Raw or the existing prepared zstd level 3 profile with a literal fallback
unless compression is strictly smaller. The encoded body is at most 1280 bytes.
There are at most two fragments. Every fragment starts with this 56-byte header:

| Offset | Size | Meaning |
| --- | --- | --- |
| 0 | 8 | ASCII `SCGPKT01` |
| 8 | 8 | Packet sequence, unsigned big-endian |
| 16 | 2 | Original IP packet length, unsigned big-endian |
| 18 | 2 | Encoded body length, unsigned big-endian |
| 20 | 1 | Codec: 0 Raw, 1 zstd |
| 21 | 1 | Zero-based fragment index |
| 22 | 1 | Fragment count: 1 or 2 |
| 23 | 1 | Reserved, must be zero |
| 24 | 32 | SHA-256 over domain `spiralcore.gateway.packet.v1\0`, original length as u16 big-endian, then exact original packet |
| 56 | variable | Fragment body, concatenated by index |

The schema in `docs/contracts/spiralcore-gateway-packet-v1.schema.json` documents
the typed header projection; this table defines its binary encoding. Header
fields must agree across fragments. Raw requires encoded length equal to original
length; zstd requires strictly smaller encoded length. Reject unknown versions,
reserved fields, empty fragments, invalid lengths, conflicting duplicates,
trailing compressed frames, and a reconstruction/digest/IP-length mismatch.
No reconstructed packet is returned for injection until all checks pass.

`packet::PacketEncoder::new(PacketMode::{Raw,Zstd})` prepares reusable state.
`encode(&mut self, packet: &[u8], sequence: u64, max_datagram: usize)` returns
`Result<Vec<Vec<u8>>>`. The adapter assigns unique increasing sequence numbers
per authenticated connection and refuses overflow. `PacketDecoder::new()` returns
`Result<Self>`; `receive(&mut self, wire: &[u8], now: Duration)` returns
`Result<Option<Vec<u8>>>`. `None` includes incomplete packets, exact duplicates
and replay drops; errors are malformed, integrity, resource or clock-order
refusals. `counters()` returns copied `PacketCounters` with accepted packets,
duplicate fragments, conflicting fragments, expired packets, capacity drops,
replay drops and invalid packets. `inspect_ip(&[u8])` returns validated source and
destination `IpAddr` values for adapter allowlists.

Receiver state is per authenticated connection: at most 64 incomplete packets,
128 KiB charged retained reassembly storage (declared encoded bytes plus a
conservative 256-byte metadata/map allowance per entry, not a measured process
RSS ceiling), a two-second monotonic TTL, and a
64-sequence completed-packet replay window. No packet loss recovery, ordering or
durability is promised. Reordered fragments may complete; incomplete packets
expire. Authentication, policy and rate checks remain outside this codec. The
packet lane never borrows mailbox acknowledgment or object-cache authority.
Prepared compression/decompression uses the existing whole-object codec and its
separate conservative workspace allowances (128 MiB encoder, 4 MiB decoder),
not an allocation allowance multiplied by packet count. Adapters admit one
bounded peer initially; several peers require a shared outer memory budget.

Packet transport keeps IPv6's 1280-byte minimum MTU instead of lowering it to fit
the QUIC minimum datagram. It does not yet provide transparent warm TCP-stream
deduplication; the reliable object API is where existing warm/shared-runtime
mechanisms are available.
