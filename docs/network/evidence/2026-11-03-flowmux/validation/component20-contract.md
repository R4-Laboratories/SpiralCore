# FlowMux SpiralCore payload mapping v1

Status: lead-approved TCP baseline implementation direction, with full v1
object integration required below. Contract reviewed before product source.
ADR-042's authenticated NetworkFlow surface, canonical signed admission, DNS
pins, tool/transform decisions, rate/window/audit and teardown remain authoritative.
This mapping creates no NIC, alternate dial, mailbox envelope or boot permission.

## Placement and selection

An explicitly selected cooperating application peer receives this mapping on
the exact already-admitted socket. A TCP grant remains TCP to its admitted IP
and port. No proxy, delegated exit or UDP/QUIC transport is inferred. Selection
must come from trusted signed-plan/profile configuration tied to its policy and
execution epochs. A guessed digest, successful probe or payload declaration is
not permission. Ordinary opaque/unaware destinations retain ordinary canonical
behavior; a required cooperating peer being unavailable refuses, with no raw
fallback. Typed HTTP transforms stay in their existing pipeline.

The first component operates on a caller-owned connected TCP socket and checks
its actual peer against the supplied expected SocketAddr. It never connects,
resolves DNS, authorizes a route or authenticates a remote identity. Its binding
and epoch comparison detect cross-flow/misrouted records, not on-path forgery;
canonical admission and appropriate peer authentication remain caller duties.
Its standalone socket tests are mapping qualification, not an admitted native
MVM endpoint witness. The pinned endpoint currently has no public adapter hook;
an additive reviewed post-admission seam is required before claiming activation.

## Rust component and explicit stream record

New standalone crate `spiralcore-mvm-flowmux` uses the existing transfer core's
Offer encoding/object_id and PreparedEncoder/PreparedDecoder whole-zstd profile.
It copies no codec, parses no IP packet and introduces no second MVM envelope.
The bounded application record below is the explicit endpoint mapping carried
inside the existing admitted TCP connection, not a competing guest carrier.

`Binding { flow:[u8;32], policy_epoch:u64, execution_epoch:u64 }` requires nonzero
flow/epochs and is immutable per connection. Connected TCP construction takes
that binding, exact expected peer and a nonzero operation timeout <=30s. Split
Tx/Rx own independent socket handles, codec state and sequence counters so a
blocked receive cannot hold the send direction. Cancellation is a shared flag
and shuts down the original socket; it creates no network route.

Each record has72 fixed bytes: `SCFLW001`8, kind1, reserved-zero3, flow32,
policy_epoch8, execution_epoch8, sequence8, body_len4 (integers big-endian).
Kinds:1 TCPData,2 TCPFin,3 Reset,4 Cancel,5 UDPDatagram. Sequence starts0 and
must be exact for TCP; overflow refuses without wrap. TCPData body is a2-byte
big-endian Offer length followed by canonical core Offer and encoded payload.
The first transport baseline accepts no-base/no-manifest Raw or ZstdWholeV1
offers. This is not the finished SpiralCore FlowMux profile. Decoded payload
is1..16384bytes; encode attempts the existing prepared codec and chooses Raw
when it does not reduce bytes. Core object_id verifies exact decoded bytes.
The baseline body cap is two bytes plus the actual canonical no-artifact Offer
encoded length plus16384; derive it through Offer::encode, not a guessed48-byte
allowance. Reject oversized lengths before allocation/decompression.
Control records have zero body. Unknown kind/version/reserved/binding/epoch,
wrong sequence, malformed Offer, checksum/length mismatch or truncation fences
the endpoint. No retry, reorder buffer or repair state is introduced.

Tx::send sends one bounded data record; Tx::half_close sends TCPFin then shuts
down only Write. Rx::receive yields Data, HalfClose, Reset or Cancel. TCP EOF
before TCPFin is truncation, including a partial prefix/body. HalfClose leaves
the reverse direction usable. Terminal/error state forbids reentry. Partial
write/read error and explicit cancellation shut down both directions. Each
operation retains one original Instant deadline across partial progress;
poll timeouts do not refresh it. Actual successful read/write byte counts are
retained even on failure, separately from application payload counts. They are
mapping bytes, not canonical FlowMux ciphertext/TCP-IP or CPU measurements.

## Required full object-v1 integration

The completed mapping must negotiate existing shared-core capabilities and
support Combined, warm and runtime/ternary-CA paths through the existing object-v1
sender/endpoint. Only the existing verified Published commit response may advance
receiver-readiness/warm state. FlowMux credit, TCP write success, HalfClose or a
new invented acknowledgement is not object publication. Preserve exact identity,
verified artifact/manifest semantics and the shared decoder; no private codec or
parallel object envelope. Bound each canonical Offer/frame using the actual
negotiated shared-core limits when this profile is connected. Continue from the
baseline into this integration after the canonical seam; issue44 stays open.

The additive `ObjectChannel` transport carries one unmodified canonical
`protocol::Frame::encode()` as kind6 (ObjectFrame), with its existing 65536-byte
maximum. It is a separately selected profile, never autodetected from ordinary
payload. Local/peer `Capabilities` must be obtained through authenticated
configuration or explicit trusted pinning and are negotiated by the shared core;
they grant no route or peer authority. The initial channel serializes one local
object operation and requires the other side to serve it; a simultaneous duplex
FlowMux pump remains required before native activation. A batch contains at most
eight frames, stops on canonical Published/Error, and retains one original
deadline including sender preparation and all exchange calls. Application blocks
remain <=16384, explicit schema/checkpoint/base publication <=65536. The public
send/seed methods check context namespace/epochs and sender profile before any wire I/O;
the receiver checks its core endpoint profile and context epochs before reading.
In this profile `Binding.flow` is exactly the shared core Context namespace,
which includes the explicitly scoped application stream identity. Another
authorized tenant/context with the same epochs cannot reuse the channel.
The existing ObjectEndpoint performs current authority checks and verified commit.
Transport failure fences the socket and invalidates sender receiver-readiness.
Core transfer statistics retain their original cost model; channel counters are
actual canonical-frame mapping bytes, not decoded application bytes or CPU.

### Bounded duplex object stream addition

The selected object-stream-v1 profile reserves kind6 for application ObjectFrame
and kind7 for artifact ObjectFrame, both with exactly one canonical Frame body;
the header's three reserved bytes remain zero. Responses echo the request kind.
This typed distinction affects application delivery only. It neither creates a
new object acknowledgement nor bypasses core authority/artifact verification.
Serial ObjectChannel seed methods use kind7; send_block uses kind6.

`Duplex::spawn(channel, sender, endpoint, context, role)` owns one reader worker
and the same socket. `role` is trusted Initiator/Acceptor configuration, never a
peer assertion. `split()` yields a single non-clone Writer, Reader and join guard.
Writer owns the core sender and exposes send_block, explicit seed methods and
half_close; Reader receives Data/HalfClose. No implicit seeding or retry occurs.
One local response is queued; at most four decoded blocks plus one in-progress
block are retained for delivery, each <=16384. Full delivery queues suspend the
worker before sending the core Published response, retaining its original
operation deadline and observing cancellation, which applies TCP backpressure.
They do not silently discard bytes or create FlowMux window credit.

The worker demultiplexes incoming requests while awaiting a local response, so
simultaneous writes do not deadlock by mistaking a request for a reply. A local
exchange observes at most32 incoming frames and one immutable operation deadline.
Idle polling before the first byte does not create an expiring partial record;
the first byte starts a record deadline. Only core Published authorizes Data
delivery from the verified endpoint object; artifacts never enter that queue.
Application transfer IDs are domain-separated hashes of the immutable namespace,
explicit sender role and monotonic block index (starting0), verified before accepting an application
operation. Published advances that index once; old/duplicate transfer IDs refuse
without delivering duplicate stream bytes. Artifact transfer IDs use a distinct
domain/role/index, and the endpoint's existing limits bound their storage/state.
The send receipt retains that transfer ID/index alongside the unchanged core
ObjectReceipt. Simultaneous identical objects at index0 have distinct IDs.

Kind8 is ApplicationHalfClose with zero body in this explicitly selected duplex
profile. It forbids future requests in that direction, but the same physical
TCP write half remains available for canonical responses to reverse traffic.
Using TCP shutdown(Write) at application half-close would lose those responses.
The worker closes the physical socket after both application half-closes and no
outstanding local response; kind2 remains the serial transport FIN. No control
record serves as an object acknowledgement. Duplicate half-close or request
after it is a protocol failure.

Each public writer/reader operation starts one deadline before enqueue/wait;
timeout/error cancels the whole transport. HalfClose retains reverse traffic.
The guard owns the exact worker; explicit join reports its result, and guard Drop
cancels and joins it. No detached worker or leaked Arc is accepted as cleanup.
Current authority is checked by ObjectEndpoint on every incoming canonical frame
and verified object read; FlowMux admission/rate/window/audit remain outside.

`MappedStream::from_duplex` consumes the cohesive Duplex and exposes standard
Read/Write, cheap clones, flush and shutdown for a canonical relay backing. It
opens no socket and changes no admission. Clones share one independently locked
reader buffer and writer; lock waits retain the original operation deadline.
Write accepts at most16384 bytes and succeeds only after existing core Published;
Read preserves order across short caller buffers and returns0 after application
HalfClose. Empty reads/writes return0 without consuming/sending a record. Explicit
seed_base/schema/checkpoint methods retain the same core artifact path. Write
shutdown sends ApplicationHalfClose; Read/Both shutdown cancels the complete
mapping, since abandoning the protocol reader also abandons response handling.
The final clone cancels and joins its exact worker. A cloned facade is not an
additional physical transport, worker or application queue.
Repeated reads after application EOF return0. A write attempted after successful
local Write shutdown returns BrokenPipe without cancelling still-valid reverse
traffic; this pre-I/O Closed refusal is separate from an invoked operation error.

## Datagram mapping boundary

Kind5 preserves one UDP datagram per record, including empty payload, with the
same16384 decoded cap and no fragmentation. It requires a separately admitted
UDP association and exact admitted peer; TCP selection never enables it.
No ordering/retransmission guarantee is added to UDP. Its first executable API
must have separate admission/integration review; TCP is implemented first.
Canonical full-size datagrams outside this version's explicit cap refuse rather
than silently split or redirect. Ingress remains declared before boot.

## Bounded first qualification and remaining acceptance

Focused RED before implementation, then real paired TCP sockets: exact bytes,
compressible/raw selection, independent reverse direction after half-close,
cross-binding/epoch and sequence refusal, unavailable required peer, oversized
or truncated input, partial progress/deadline/cancel, and actual byte counters.
Do not relabel a missing API compile RED as semantic runtime failure. All test
threads/sockets are owned and joined; no VM or host-network change is needed.

After this component, review the exact canonical MVM seam before editing its
fork. Preserve canonical positive/negative admission, transforms, rate/audit,
window/backpressure and cleanup with actual endpoint tests; then Linux/FC and
supported Mac backends, ingress/UDP, matched raw/equally-tuned-zstd bytes/CPU/
wall/RSS and ADR-042 performance gates. Whole issue44, real WAN, scale and full
Ari delivery remain open until those executed gates pass.
