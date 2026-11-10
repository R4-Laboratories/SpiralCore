# FlowMux SpiralCore payload mapping v1 — draft

This is the current implementation contract for an additive MVM contribution,
not a formal standards claim or a completed release. Canonical activation is
under construction. The existing roadmap remains
[SpiralCore issue44](https://github.com/R4-Laboratories/SpiralCore/issues/44).
MVM owns its authenticated NetworkFlow carrier, signed-plan admission, DNS pins,
tool/transform decisions, rate/window budgets, audit and teardown. SpiralCore
retains one shared object/codec core. No NIC, alternate egress route, mailbox
semantics or owner-access grant is introduced by this mapping.

The [checkpoint20 packet](../network/evidence/2026-11-03-flowmux/component20.md)
records twenty passing socket tests for the earlier transport prototype. Its
72-byte/numeric-epoch draft is historical component evidence and is not this
canonical MVM binding. The new optional policy DTO and admitted-plan forwarding
have focused tests; actual authenticated canonical mapping, platform and WAN
qualification remain required. Source review or a passing component is not
whole issue44 completion.

## Placement and signed selection

A selected cooperating application peer receives the mapping on the SAME
already-admitted TCP socket, at the admitted IP/port. A TCP grant does not enable
UDP, QUIC, another proxy or a delegated exit. A tuple, digest, successful probe
or peer payload assertion is not authentication or permission. Ordinary
unselected destinations retain canonical behavior. A selected required peer
that cannot authenticate or speak the profile refuses without raw fallback.

Canonical NetworkPolicy Preset/AllowList gain optional payload_mapping. None is
serde-default and omitted when serialized, preserving old policy bytes and
policy-asset SHA exactly. CooperatingPayloadPolicy has explicit version1 and
1..32 exact CooperatingTcpPeer rules. Each contains canonical host, nonzero TCP
port, TLS server_name, full nonzero32-byte SPKI SHA-256 pin and explicit
SpiralCoreObjectStreamV1 profile. Unknown fields/version/profile, invalid names,
wildcards, empty/oversized/duplicate destinations and zero ports/pins reject.
Selection never grants a route; canonical destination admission still comes first.

The field follows the existing complete-policy content_sha256, signed policy
asset identity, bind_claim_policy_to_plan and EndpointConfig.network_policy
materialization path. Typed HTTP substitution/redaction stays in its canonical
pipeline. A selected mapping must refuse an unsupported transform/typed-path
combination rather than silently bypassing the transformation or required mapping.
Opaque ciphertext gains are not assumed.

## Full admitted facts and authority

AdmittedNetworkPlan carries the actual complete plan_id, plan_version,
plan_nonce, bundle_sha256, tenant_id, instance_id and full policy_sha256. The
runtime forwards the actual already-admitted signed plan; no field is fabricated
from a truncated digest, resource-owner generation or absent numeric epoch.
SubstitutionSpawnParams and EndpointConfig carry this DTO optionally. None adds
no JSON key. Selected mapping plus missing/mismatched facts refuses.

The DTO itself is not proof of admission. Existing trusted spawn/config admission
and canonical policy-hash checks remain the authority boundary. MVM supplies the
actual authenticated NetworkFlow session/guest identity, stream ID, requested
host/logical port, admitted protocol and actual connected IP/port/route. Peer
routing retains both the requested logical port and the gate's translated actual
socket port. Full facts remain available to current authority checks.

A connection binding additionally includes actual authenticated TLS endpoint
identities, trusted initiator/acceptor role, negotiated profile and a fresh
TLS-bound connection nonce. The per-direction32-byte flow identifier hashes a
versioned domain plus unambiguous FULL admitted facts, role and fresh nonce.
The connection/object namespace binds those full facts. Separate connections
cannot reset block index0 under a static peer namespace. A commitment alone is
not authorization; peer-supplied scope must be checked against canonical trusted
facts before codec state is allocated.

The shared object-v1 Context's legacy numeric codec epoch slots are reserved0 in
this application profile, with NO MVM epoch authority. Complete canonical policy,
plan, run, session and authenticated peer facts bind its application namespace
and authorization. Mailbox epochs and resource-owner lifetime generations retain
their separate domains.

## Authenticated carrier

### Authenticated scope versions and ordered Small retention

`ScopeFacts.object_profile` selects the scope serialization explicitly. General
object profile 1 retains the published `spiralcore.flow.scope.v1\0` canonical
bytes and `spiralcore.flow.scope.commit.v1\0` hash domain for IPv4 and unscoped
IPv6. General rejects IPv6 addresses with nonzero flow information or scope ID,
because its existing encoding cannot represent those fields.

Small object profile 2 uses `spiralcore.flow.scope.v2\0` and
`spiralcore.flow.scope.commit.v2\0`. Its canonical facts additionally bind the
big-endian object profile and complete IPv6 flow information and scope ID.
The authenticated object capability must match that scope's profile before
the object channel is created. There is no implicit downgrade or fallback.
Existing experimental Small peers must update together. These scope versions
do not change the object-v1 frames or introduce a second codec protocol.

Only an explicitly selected Small endpoint uses ordered retention. Application
and artifact lanes have independent, strictly increasing ordinals and immutable
direction/scope-derived transfer IDs. The next admitted ordinal retires the
previous completed attempt in that lane. Replays of retired ordinals and wrap
to zero refuse; this is a live ordered connection contract, not reconnect or
final-acknowledgment recovery. General endpoints keep their existing retention.

The bounded cache pins at most two published artifacts, the current lane objects
and active bases. Space planning selects unpinned LRU victims without deleting
files or aging cache entries. Canonical Begin admission validates the offer and
aggregate limits using projected space credits. Pre-admission refusal leaves
verified publications and LRU state intact. Only after successful admission are
the verified victims removed. A post-admission storage or authorization failure
can leave a partially evicted cache: the facade fences, sends no success reply
and requires owner-controlled recovery. This operation provides no transactional
rollback or durable cache-eviction acknowledgment. Object publication still
requires the existing exact-byte and length verification.

The cooperating profile uses mutual TLS1.3 with RFC7250 raw public keys, full
SPKI SHA-256 pins and ALPN spiralcore-flowmux/object-stream/1. Both actual
handshake signatures use rustls's existing standard crypto provider and raw-key
verification facilities. No private crypto, handwritten X.509 parser, unsigned
pin environment, opportunistic upgrade or second dial is permitted.

Wrong pins/keys/ALPN, missing authenticated identities, TLS1.2 or an unavailable
required profile refuse before application/object bytes are sent. Early data
and resumption cannot bypass scope setup. Scope/profile capability negotiation
is authenticated and bounded; it grants no new authority. Both payload directions
share ONE TLS connection state, not TLS sessions made from cloned TCP FDs.
Standard TLS is the carrier, not another object codec or MVM envelope.

The original setup Instant deadline includes canonical TCP connect, TLS and
profile/scope preparation; partial progress does not refresh it. Local operation
deadlines remain monotonic and bounded at30s. Fresh deadline and current scope
checks occur before invoked I/O. No syscall or child join holds a ledger/map lock.

## Canonical host interface

PayloadMappingConfig::new(&NetworkPolicy, AdmittedNetworkPlan) validates the
immutable complete policy/scope binding. with_factory supplies the trusted
AuthenticatedTcpPayloadFactory. FlowMuxAccept::with_payload_mapping retains the
old None path. The factory consumes the SAME owned connected TcpStream and an
AdmittedTcpPayload minted after canonical gate/tool/transform decisions. A guest
cannot mint that host fact token. The factory authenticates before constructing
codecs. It returns Read/Write/clone/shutdown payload I/O retaining the original
TCP and worker custody. Failure retires the stream reservation and emits the
canonical refusal/audit without fallback.

Physical FlowSocket backings remain admitted upstream TCP or the existing local
terminated pair. A payload decorator is not a third route. Mapped I/O, shutdown
and joins occur outside registry/stream-map locks. Ordinary None I/O remains on
its existing path. Real factory activation, credentials and artifact/plan binding
must be implemented and independently reviewed before native delivery is claimed.

## Versioned record and shared object mapping

Canonical draft header length is120 bytes:

| Offset | Field | Bytes |
| --- | --- | ---: |
| 0 | SCFLW001 magic | 8 |
| 8 | kind | 1 |
| 9 | sender role: initiator0, acceptor1 | 1 |
| 10 | reserved zero | 2 |
| 12 | directional flow identifier | 32 |
| 44 | full canonical policy SHA-256 | 32 |
| 76 | full admitted-scope commitment | 32 |
| 108 | sequence, big-endian | 8 |
| 116 | body length, big-endian | 4 |

Sequences start0 independently per direction, must be exact and never wrap.
The receiver verifies actual peer role/binding, version and limits. Wrong role,
policy/scope/flow, reserved bytes, sequence, malformed body, identity/length
mismatch or truncation fences the endpoint. No retry/reorder/repair buffer is
inferred from this mapping.

Kinds1/2 are the earlier bounded baseline TCPData/transport FIN; kinds3/4 are
Reset/Cancel; kind5 is separately admitted UDPDatagram; kind6 is application
ObjectFrame; kind7 is artifact ObjectFrame; kind8 is ApplicationHalfClose.
Profiles are explicitly selected, never inferred by probing an ordinary target.
ObjectFrame carries unmodified shared protocol::Frame bytes, bounded by the
negotiated shared-core maximum65536. Controls have zero body. Header/body bounds
are checked before allocation/decompression.

The baseline uses the existing Offer encoding/object_id and prepared whole-zstd
profile. Its data body is u16 Offer length plus canonical Offer plus payload;
body cap derives from actual Offer::encode length plus16384, not a guessed48-byte
allowance. Raw/whole baseline alone is not the completed profile.

The object profile uses existing shared Capabilities negotiation and
ObjectSender/ObjectEndpoint Warm, Runtime and Combined paths, including existing
runtime/ternary-CA artifacts. Application blocks are1..16384 bytes; explicit
base/schema/checkpoint publication is bounded at65536. A batch has at most8
canonical frames. Only the existing verified Published response advances sender
receiver-readiness/warm state. FlowMux credit, write completion, application FIN
or profile setup is not object publication. No copied codec, new object ACK or
parallel object authority is introduced.

## Duplex delivery, backpressure and lifecycle

One owned reader/demultiplexer handles incoming requests while a local operation
awaits its response. A local operation admits at most32 incoming frames under one
original deadline, including preparation and all exchanges. One local response
is queued; delivery retains at most4 decoded blocks plus1 pending block. A full
queue applies backpressure before the core Published response and observes its
original deadline/cancellation. It does not discard bytes or mint window credit.

Only actual verified endpoint publication can deliver application Data. Artifact
frames never become stream bytes. Application transfer IDs are domain-separated
hashes of the full connection namespace, explicit sender role and monotonic block
index0 onward; Published advances an index once. Replay/old IDs refuse without
redelivery. Artifact IDs use a separate domain/role/index. Existing endpoint
storage/state limits and an aggregate admitted resource budget still apply.

ApplicationHalfClose forbids future requests in that direction but keeps physical
TCP Write available for canonical responses to reverse traffic. Physical closure
follows both application half-closes and no pending local response. Repeated EOF
reads return0. A pre-I/O write-after-FIN refusal returns BrokenPipe while retaining
reverse traffic. Invoked operation errors/timeouts fence the whole mapping and
invalidate sender receiver-readiness; no automatic retry occurs.

MappedStream consumes a cohesive Duplex and exposes Read/Write, short reads,
clones, flush, explicit artifact seeds and shutdown for the relay. Empty reads/
writes return0 without a record. Nonempty Write accepts at most16384 bytes and
succeeds only after canonical Published. Read/Both shutdown cancels the mapping.
Clones share direction state and do not create another TLS session or worker.
The last clone cancels and joins its exact worker; explicit joins report outcomes.

## Remaining qualification

Use focused API/semantic RED before implementation, then real paired sockets and
actual canonical endpoint tests: admitted/denied routes, signed-policy tamper,
wrong identities/scopes/roles, unavailable required peers, transform preservation,
exact readbacks, duplex/artifact separation, deadlines, backpressure, cleanup and
no fallback. Keep test threads/children owned and joined. Record actual traffic
both directions, setup/sender/receiver CPU, complete lifecycle and verified wall
/RSS; core selection estimates are not actual carrier counters.

UDP preserves a single datagram boundary including empty payload, has a16384
explicit cap and no application fragmentation, and requires its OWN canonical
UDP association. TCP cannot enable it. Datagram/ingress, supported Linux/FC and
macOS native backends, Windows role distinctions, matched raw/equally-tuned-zstd
measurements, MVM performance limits, real WAN and scale remain separate executed
gates. Socket tests are not booted-MVM capacity or WAN/energy/security certification.
Whole issue44 and the parent delivery goal remain open until those gates pass.

## Explicit first-profile enrollment and original-deadline setup

Mutual TLS proves possession of the pinned peer key; it does not admit a foreign
plan or tenant. The first cooperating endpoint requires a trusted local
controller to provision PinnedScopePolicy with exact full ScopeFacts, peer SPKI,
profile/direction and original expiry Instant before object allocation. This
bounded policy starts empty, permits at most32 enrollments and caps each lifetime
at300s. Revocation advances its local generation; replacing an enrollment cannot
revive an older authority handle. This local revocation generation is not a wire
MVM epoch. ScopeAuthority implements the existing core endpoint::Authority and
checks the exact Context and current enrollment before publication/I/O, including
release of a previously verified buffered application suffix.

The trusted initiating controller derives facts from the actual private admitted
flow token and authenticated TLS session, then explicitly provisions both ends.
The receiver does not boot or re-admit the initiating VM's plan. Received facts
are not grants. Unknown scope, wrong peer/direction/tenant/plan/nonce/exporter,
expiry or revocation refuses. General remote discovery, issuer delegation and
cross-organization enrollment are separate open control-plane gates. ScopeFacts
Debug deliberately omits cryptographic/session material.

The current full-facts Context partitions verified object state per authenticated
flow, including the fresh TLS exporter and session/stream identity. Warm state
reuse is within that flow. Reuse across connections requires an explicit stable
CacheScope capability and fresh full-flow authorization; silently omitting fresh
authentication facts to obtain a cache hit is forbidden. No cross-flow cache
benefit is established by this component.

The additive MappedStream seed_base_until(Vec<u8>, Instant), seed_schema_until
and seed_checkpoint_until APIs reuse the existing exact core publication paths.
They preserve the caller's original monotonic setup deadline through lock waits,
artifact preparation, partial writes and interleaved replies. The supplied future
budget must fit the configured stream timeout, at most30s. An expired attempt
writes no artifact bytes; failure fences reentry. Existing convenience seed APIs
retain their bounded per-operation behavior. Canonical factory setup uses only
the explicit original-Instant APIs, at most one schema, one checkpoint and one
base artifact, with core Published verification before readiness. Artifacts are
never application stream data or synthetic object commits.

The hostd SpiralCorePayloadFactory consumes the same admitted TCP and actual
endpoint SigningKey through standard PKCS8/rustls facilities. Its explicit
PayloadScopeController receives the private admitted request, full TLS-bound
facts, fixed v1 shared NegotiatedProfile and original deadline, and returns an
explicitly enrolled PreparedPayloadSession containing the existing core sender
and endpoint. The factory verifies the exact returned scope, current authority
and actual TLS role/exporter before mapped I/O. The cooperating acceptor requires
its own enrolled prepared session. A controlled canonical NetworkFlow socket test
executes Whole, Warm, Runtime and Combined with real artifact publication and
exact bidirectional half-close readbacks; an authenticated but unenrolled peer
is refused. These are component sockets, not booted MVMs or production daemon
provider provisioning. The full integrated source, production controller,
independent reviews, platform/backend, aggregate scale, matched performance and
real-WAN gates remain required. Issue44 remains OPEN.
