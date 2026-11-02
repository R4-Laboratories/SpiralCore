# SpiralCore v1 object protocol and endpoint contract

Status: normative project protocol contract, 2026-11-02. This consolidates the
implemented object-transfer family at source `06a4bdf3cb6f7de3ce7c134a9f6440274ebee6fc`.
It is a versioned project standard, not an IETF standard or production security
qualification. MUST, MUST NOT and SHOULD state requirements for implementations
claiming this profile. Historical experiment status in older documents does not
change the current wire definition.

## One protocol, explicit endpoint mappings

The v1 protocol identity is `spiralcore-object`, version **1.0**, profile **1**
(`object-v1`). It transports immutable byte strings through a shared sender and
receiver state machine. MVM canonical mailbox, reliable QUIC object service and
in-process test I/O are carriers of this same application protocol; they do not
create new object encodings or content identities.

The existing independent-packet TUN service is not an `object-v1` endpoint.
Transparent stream and packet mappings remain unstandardized and MUST NOT be
advertised as this profile. Defining such a mapping requires a published profile
identifier and conformance tests; it must not silently reinterpret object ACKs,
message boundaries, ordering, cancellation or authorization. Guest management
and development shells retain separate admitted service semantics.

MVM owns its canonical mailbox envelope, cryptographic domains, capabilities,
transport bindings and delivery receipts. SpiralCore frames are the payload of
`application/spiralcore-object`, content-type version **1**. A carrier authenticates
and decrypts before dispatch. Host relays forward canonical ciphertext unchanged.
SpiralCore object publication, MVM mailbox admission, transport delivery and
application processing are distinct acknowledgments. No exactly-once application
execution guarantee is implied.

## Version and capability policy

The first canonical MVM mapping uses an explicitly pinned **full object-v1**
profile: all mode IDs 0–9, a 65,536-byte frame ceiling and a 67,108,864-byte receiver
object ceiling. This is prior agreement, not an on-wire discovery handshake.
No extra bytes, new frame kinds, cipher or mailbox envelope are introduced.
Existing `SCMOBJ01` bytes and compatibility vectors MUST remain unchanged.

A shared `Capabilities::negotiate` API accepts local capabilities and peer
capabilities obtained from authenticated carrier metadata or explicit trusted
configuration. An arbitrary received JSON document is not authenticated metadata.
The adapter MUST authenticate/bind both advertisements and the resulting
selection to the same peer, tenant/domain/recipient, execution and policy epochs
before treating the selection as peer agreement. The protocol library does not
perform this authentication. A transport reconnect or policy/epoch change MUST
revalidate the binding. The pinned MVM profile MUST NOT be called dynamic wire
negotiation.

The machine-readable descriptor is [spiralcore-v1.schema.json](../api/spiralcore-v1.schema.json).
It is an API/control-plane descriptor, not a replacement data-plane encoding.

- `major` MUST be 1, `minor` MUST be 0, and `profile` MUST be 1 for this implementation.
  Unknown versions/profiles fail `Unsupported`; they do not fall back silently.
- `max_frame_bytes` MUST be 65,536. Existing senders use fixed chunk bounds; a
  smaller-frame profile requires a separate compatible batching specification.
- `max_object_bytes` is a receiver ceiling in 1–67,108,864; the negotiated value
  is the minimum of both advertisements. Empty byte strings remain valid objects.
- `codec_mask` is a nonzero unsigned 16-bit set containing only bits 0–9; bit 0
  (Raw) is mandatory. Unknown bits fail `Unsupported`. The negotiated set is the
  intersection, which MUST retain Raw.
- Unadvertised codec modes MUST be rejected before any request is emitted or
  offered to the underlying receiver. Narrowing is not permission to reinterpret
  a codec or seed a base automatically. The current Combined selector is pinned
  full-profile; rejecting its unsupported selected offer does not imply it has
  dynamically pruned all candidates for a narrower peer.

A breaking wire, codec, identity or ACK change requires a new version/profile.
A new codec receives a new stable ID and its own bounded decoder definition and
vectors. Existing mode IDs and hash domains MUST NOT be reassigned. Negotiation
is capability intersection, not authorization or distributed consensus.

## Byte identity and canonical framing

All integers below are unsigned little-endian except stated floating-point
values. SHA-256 domains include the terminal NUL byte.

`ObjectId = SHA256("spiralcore.object.bytes.v1\0" || byte_len:u64 || exact_bytes)`.
Length is part of identity; embedded/trailing zero bytes are data. Object identity
is independent of DAG shape, cache state, carrier and selected codec. No receiver
may advertise a staged prediction or digest assertion as a verified publication.
Legacy UOROBJ01 96-bit record identities are not object-v1 cache acknowledgments.

An application frame is exactly:

| Offset | Bytes | Meaning |
|---:|---:|---|
| 0 | 8 | ASCII `SCMOBJ01` |
| 8 | 1 | Kind |
| 9 | 3 | Zero flags/reserved bytes |
| 12 | 32 | Transfer ID |
| 44 | 8 | Request sequence |
| 52 | 4 | Body byte length |
| 56 | body length | Body |

Total frame length MUST be at most 65,536. Decode MUST reject bad magic,
unsupported kind, nonzero reserved bytes, integer overflow, truncation and
trailing bytes. Transfer IDs are application identifiers, not crypto nonces.
Replies repeat the request transfer ID and sequence.

The Offer body is `mode:u8 || original_len:u64 || ObjectId32 || manifest_len:u32 ||
manifest || base_present:u8 || optional_base_id32`. Manifest bytes are at most
1,024; base flag is exactly 0 or 1. A descriptor ID is
`SHA256("spiralcore.object.transfer.v1\0" || canonical_Offer)`.
Raw/Zstd/Whole prohibit a manifest and base; DAG requires its matching manifest;
other base-dependent modes require the full published base/artifact object ID.
DAG's optional base names a verified base manifest. No field grants access.

## Operation/state machine

| Kind | Body | Successful reply |
|---:|---|---|
| 1 Begin | Offer | 101 Ready or 103 Published |
| 2 Data | Mode-specific bounded body | 102 Accepted |
| 3 Finish | Empty | 103 Published or 104 Need |
| 4 Resume | DescriptorId32 plus exact Offer | 101 Ready or 103 Published |
| 5 Status | Empty | 105 State |
| 6 Cancel | Empty | 105 State |
| 101 Ready | Mode:u8, base_pinned:u8, next_sequence:u64 | Response only |
| 102 Accepted | accepted_input_bytes:u64 | Response only |
| 103 Published | ObjectId32 | Response only |
| 104 Need | count:u16 plus missing NodeId32 values | Response only |
| 105 State | state:u8 plus accepted_input_bytes:u64 | Response only |
| 127 Error | error_code:u16 | Response only |

Begin starts at sequence 0. The next accepted request uses sequence 1 and then
increments exactly once per successful request. Exact retries of an already
accepted sequence/body return its recorded reply. Conflicting reuse fails
`Sequence`; gaps fail without advancing. Keep a bounded, charged ledger of at
most 4,096 accepted requests per transfer. Failed admission consumes no active
transfer slot. Validation errors can reject and clear the attempt.

States are receiving=0, published=1, cancelled=2 and rejected=3. Idle expiration
is an error, not another State value. Cancel clears uncommitted work; published
objects remain immutable. Unknown/expired attempts must not be inferred from
filesystem membership. Resume with sequence 0 after restart can recover an
already verified publication; partial-progress staging is volatile and otherwise
returns `Unknown`. Resume across a changed namespace/epoch is a new authorized
context, not continuation of the old attempt.

Ready identifies the offered encoding, not an implicit alternative. Accepted
reports received/decoded or encoded progress according to the mode; it is not a
verified-cache or durable-publication acknowledgment. Finish MUST reconstruct,
check exact original length and ObjectId, recheck current authority and durably
publish before Published. DAG Need requests complete missing descendant closure,
up to 1,024 IDs and two repair passes; further misses fail `Missing`. An authorized
existing exact object may produce Published at Begin or Resume after revalidation.

## Stable codec registry

| ID | Encoding | Base and bound |
|---:|---|---|
| 0 | Raw | No base; receiver object ceiling |
| 1 | Independent zstd chunks with raw fallback | No base; receiver object ceiling |
| 2 | Ternary SHA-256 DAG records | Matching manifest; optional verified manifest base |
| 3 | SparseV1 COPY/LITERAL | Full base ObjectId; receiver object ceiling |
| 4 | ZstdPrefixV1 | Full base ObjectId; output at most 16 MiB |
| 5 | RuntimeV1 checkpoint/recipe plus exact residual | Full checkpoint ObjectId; output at most 16 MiB |
| 6 | SchemaV1 exact template/value stream | Full schema ObjectId; output at most 16 MiB |
| 7 | ZstdDictionaryV1 | Full dictionary artifact ObjectId; output at most 16 MiB |
| 8 | SchemaZstdV1 | Full schema ObjectId; output at most 16 MiB |
| 9 | ZstdWholeV1 | No base; output at most 16 MiB |

The receiver MAY impose lower resource ceilings; advertising a mode does not
promise arbitrary concurrent objects fit. The delivered batched sender is
limited to 16 MiB even where the receiver's Raw/DAG/Sparse parser accepts 64 MiB.
A sender must enforce its own stronger bound before any wire activity.

Raw/chunk-zstd Data is `encoding:u8 || decoded_len:u32 || payload`; decoded bytes
are at most 61,440 per chunk. Encoding 0 is literal exact bytes. Encoding 1 is
one bounded zstd frame only in mode 1, with no concatenation/trailing bytes and
at most 1 MiB window. DAG uses encoding 2, a record count:u16 and contiguous
records; decoded length is the exact record-byte total. Leaf records are ID32,
type=0, scale=0, data32 (66 bytes); parent records are ID32, type=1, scale 1–13,
ordered three child IDs (130 bytes). Child scale MUST be parent scale minus one.
Node and manifest domains are `spiralcore.object.node.v1\0` and
`spiralcore.object.manifest.v1\0`; final partial leaf zero-padding and exact
ternary root shape are mandatory.

The following existing format documents are normative subordinate definitions;
their old experiment/rollout status is historical, while their explicitly
frozen codec byte definitions remain binding:

- [SparseV1](../network/sparse-v1.md): bounded COPY/LITERAL frames, append,
  truncate, insertion/deletion and literal fallback; at most 8,192 operations.
- [RuntimeV1](2026-10-30-shared-runtime-design.md): canonical `SCRUN001` checkpoints,
  `SCRCP001` recipes and `SCTRAC01` traces. The transition math and software
  numerical behavior are version-bound; hardware clock synchronization is not
  correctness authority. At most 4,096 steps/width, 64-KiB checkpoint and 16-MiB
  output; prediction identity is verified before residual application.
- [Schema/dictionary](2026-10-30-schema-fitting-design.md): `SCSCH001`, `SCVAL001`,
  `SCZDICT1`, exact raw lexical values and literal fallback.
- [Compressed schema](2026-10-30-schema-reuse-design.md): one bounded checksummed
  zstd frame over the exact schema value stream.
- [Whole-object zstd](2026-10-31-persistent-session-design.md) and
  [warm-selection codec definition](2026-10-30-warm-selection-design.md): strict
  one-frame output and prefix reuse. The source-pinned compatibility vectors
  remain required where those definitions refer to fixed binary decoder rules.

Modes 6–9 use buffered Data `mode:u8 || chunk_len:u32 || profile:u8=1 || chunk`,
with nonempty chunks at most 61,440 bytes, at most 512 chunks, and the codec's
aggregate encoded cap. Runtime's first Data uses opcode 5, zero decoded output,
profile u16=1, recipe_len:u16, recipe, prediction_len:u32 and prediction_id32;
remaining Data uses SparseV1 against that verified prediction. Artifacts and
recipes are inert data selecting built-in transitions; received source code or
executables MUST NOT run. Unrelated/unpredictable bytes require explicit residual
or literal bytes. No mechanism removes independent information.

## Shared endpoint API

The authoritative transport-neutral entry point belongs in `spiralcore-transfer::v1`.
There is one sender implementation, exported compatibly from gateway-core rather
than copied into each carrier. The shared Rust surface has the following public
signatures. `Capabilities` has
six public integer fields matching the schema (u16 major/minor/profile/codec_mask,
u32 max_frame_bytes, u64 max_object_bytes); validation occurs before agreement.
`NegotiatedProfile` fields and the endpoint's owned receiver remain private.

```rust
impl Capabilities {
    pub fn full() -> Self;
    pub fn negotiate(&self, peer: &Self) -> Result<NegotiatedProfile>;
}
pub trait FrameEndpoint {
    fn handle(&mut self, context: &Context, frame: &Frame, now: u64) -> Result<Frame>;
}
impl<A: Authority> ObjectEndpoint<A> {
    pub fn open(root: &Path, limits: Limits, authority: A,
                profile: NegotiatedProfile) -> Result<Self>;
    pub fn open_with_policy(root: &Path, limits: Limits, authority: A,
                            profile: NegotiatedProfile,
                            policy: ColdGraphPolicy) -> Result<Self>;
    pub fn read_object(&mut self, context: &Context, id: Id) -> Result<Vec<u8>>;
}
```

These are API signatures, not standalone executable Rust. Constructors MUST
validate capabilities before producing NegotiatedProfile. A new ObjectEndpoint
opens a fresh receiver under that immutable profile; it MUST NOT rewrap an
already-active legacy endpoint to disguise old attempts under narrowed
capabilities. `handle` invokes the underlying Authority before mode/limit
rejections, then validates context namespace shape, frame bound and Begin/Resume
Offer mode/length before receiver admission. Frame bounds MUST be checked without
allocating a second encoded copy merely to inspect its length. All other
frame/state/codec checks delegate to the existing single receiver implementation.
The wrapper does not authenticate context; the underlying Authority MUST still
run on every operation and before publication/cache disclosure. A legacy endpoint
may implement FrameEndpoint for source compatibility, but an adapter claiming v1
profile enforcement MUST use the profile-enforcing ObjectEndpoint path.

The negotiated object ceiling constrains inbound transfer offers. It does not
retroactively shrink previously stored objects or promise a smaller pre-read
allocation limit for `read_object`; stored reads retain the core's 64 MiB global
ceiling. A future bounded-read API must check length before allocation. The first
MVM mapping pins the full 64 MiB profile.

Read and configuration methods required by existing receiver callers can forward
to the owned receiver without duplicating storage. They MUST preserve authority,
resource and durable publication checks. A public mutable-inner escape that lets
an adapter bypass the enforced profile is not part of this API contract.

The shared sender retains existing selection/seed/send receipts and prepared
state, with the following profile-bound constructors:

```rust
impl ObjectSender {
    pub fn new_with_profile(mode: ObjectMode, context: &Context,
                            schema: Option<&[u8]>,
                            profile: NegotiatedProfile) -> Result<Self>;
    pub fn with_runtime_profile(context: &Context, prediction: PreparedPrediction,
                                profile: NegotiatedProfile) -> Result<Self>;
    pub fn with_combined_policy_and_profile(context: &Context,
                                            prediction: PreparedPrediction,
                                            schema: &[u8], policy: SelectionPolicy,
                                            profile: NegotiatedProfile) -> Result<Self>;
}
```

The sender owns its immutable negotiated profile. Existing constructors retain
source compatibility by pinning the full v1 profile. There is no operation that
changes an active sender's profile. Every seed/send exchange callback MUST be
wrapped by the same profile guard. Before invoking the carrier callback, the guard
checks every outbound frame's bound without encoding a second copy and validates
Begin/Resume Offers against the profile's codec and object ceilings. Resume's
Offer follows its 32-byte descriptor ID. If an outbound batch is inadmissible,
the guard rejects that batch before invoking its carrier callback. Reply frame
bounds MUST also be validated before returning replies to the sender state machine.
An unsupported selected target Offer MUST fail `Unsupported` with zero carrier
callback invocations for that transfer attempt; oversized offers fail `Limit`.

A narrowed Combined profile MAY reject its selected Offer before any wire
activity; this milestone does not promise dynamic pruning of all candidate
encoders. The caller may explicitly choose the mandatory Raw mode as fallback.
It MUST NOT silently emit a codec outside the negotiated set. Seed publications
are transfers subject to these same profile checks, even when their codec differs
from a later target's codec.

Carrier callbacks transport the same frames; they do not reinterpret codec IDs,
discard errors, mint authorization or count unsent bytes as transmitted. The
gateway-core object facade remains a compatibility re-export, not a fork.

## Authorization, resources, durability and errors

The caller supplies authenticated context and trusted monotonic elapsed seconds.
Namespace binds seven UTF-8 fields (tenant, domain, sender, recipient, mailbox,
capability, authorization_scope), each length-prefixed u16 and 1–256 bytes, plus
policy_epoch:u64 and execution_epoch:u64, under
`spiralcore.object.namespace.v1\0`. No normalization or cache membership grants
rights. Reconnect/resume MUST revalidate current authorization, recipient and
epochs; cache bases cannot cross namespaces by digest alone.

Default receiver ceilings are four active transfers, 262,144 staged nodes,
512-MiB resident reservation, 512-MiB namespace disk quota, 1-GiB aggregate cache,
128 transfer states and 4,096 publications per endpoint instance. Defaults are
not independent allocations per peer: adapters need one shared budget/instance
or an explicit aggregate admission owner. Idle timeout is 30 seconds and terminal
retention 300 seconds. Prepared codecs/graphs are charged inside, not in addition
to, resident limits. Output, encoded intermediates, base bytes, traversal work,
codec scratch and request ledgers require checked admission before allocation.

Staging is inaccessible to ordinary reads. The writer validates canonical
encoding, complete DAG closure/scales/padding, exact length and ObjectId, checks
authority again, writes/syncs a temporary publication, atomically renames and
syncs its directory before Published. A storage uncertainty returns error; a
retry can discover a verified committed object. Thus loss of an acknowledgment
is not a proof that publication did not occur. Restart revalidates content and
current authority. The current storage implementation is Unix-specific; Windows
MUST NOT claim this profile without equivalent private/durable storage semantics.

Error codes are fixed: 1 Malformed, 2 Unauthorized, 3 Limit, 4 Integrity,
5 Missing, 6 Sequence, 7 Unknown, 8 Expired, 9 Storage, 10 Unsupported.
Adapters MAY wrap these in carrier errors but MUST preserve their meaning. Error
127 carries only its u16 code, never payload contents, keys or private diagnostics.

## Conformance and measurement

A v1 release MUST retain existing frame/identity vectors and add descriptor,
codec-subset/version refusal, shared-endpoint and canonical-MVM dispatch tests.
Exact bytes and lengths are checked for empty/binary/trailing-zero inputs,
boundaries, edits, append/truncate and missing/evicted bases. Unknown versions,
unsupported modes, malformed frames, invalid authorization, stale epochs,
cancellation, replay/conflict, resource limits and interrupted publication fail
closed. Transport authentication and MVM receipt semantics are tested separately.

Record setup plus sender/receiver CPU and elapsed transfer/verified-commit time;
count actual request/reply frame bytes separately from envelope, stream/device
and physical-IP bytes. State the layer and exclusions. Legacy batched sender
`Stats::request_bytes` can include planned frames skipped by a carrier after an
early Published reply; these totals MUST be labelled planned, not transmitted.
Actual carrier counters govern transmitted-byte claims. Charge negotiation when
it is actually performed; pinned configuration does not have invented handshake
bytes. Keep warm/cold, seed cost, retries, receiver durability, Linux/HVF/QUIC,
local/WAN and direct/relay observations distinguishable. A conformance run or
published contract alone is not a new WAN, production mailbox, energy or
cryptographic-module validation result.
