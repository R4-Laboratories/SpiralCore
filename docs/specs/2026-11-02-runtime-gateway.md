# Runtime and warm-cache gateway integration

Authorized by the user's request to integrate the existing ternary/Kuramoto/CA
runtime and warm-cache paths, preserving exact bytes and recording costs against
zstd. Base420e511. Reuse the current isolated worktree. No additional approval
checkpoint is needed for this implementation. This document is committed before
production edits.

## Decision

Expose the existing RuntimeSession alongside WarmSession through ObjectSender;
carry its existing object protocol over a new reliable, pinned-mTLS QUIC profile
inside the gateway. Add a usable file/object service, not merely a codec bench.
Use existing exact publication, cache scoping and replay/residual validation.
Runtime prediction preparation is explicit application knowledge, not an
unbounded search for a recipe. No runtime equations or wire object codecs change.

Alternative packet-by-packet replay cannot express the existing object checkpoint
contract and would repeat setup. A new global optimizer would duplicate policy
and conceal what ran. We instead expose explicit runtime and warm profiles,
with real selected-mode/candidate receipts and a matched whole-zstd profile.
Runtime's selector uses a fixed checkpoint; Warm uses a rolling acknowledged
base and schema. This is not a global minimum across all encodings.

The existing TUN datagram route remains operational and independent. The new
object channel is usable by cooperating applications and the shared MVM facade;
transparent packet-block/TCP reconstruction is not claimed by this object-channel
milestone. Do not describe runtime gains as applying to arbitrary encrypted NIC
traffic. No MVM vsock or mathematics rewrite is required here.

## Shared facade

Add ObjectMode::Runtime, ObjectSender::with_runtime(context, PreparedPrediction),
seed_checkpoint, send_with_receipt and invalidate_receiver_state. ObjectReceipt
contains Stats and ObjectSelection::{Fixed,Warm(WarmSelection),Runtime(Selection)}.
Keep new/send signatures compatible. Runtime needs explicit prediction; generic
new(Runtime) rejects. State namespace changes permanently invalidate the facade.
Expose retained_charge; one explicit profile per sender avoids duplicate large
state. Missing checkpoint/base is an error, not hidden fallback/reseed.
Candidate costs retain their original HTTP-body accounting label; actual channel
and network bytes are measured separately. No claim of QUIC byte optimality.

## Reliable channel contract

ALPN spiralcore-object/1, separate from unchanged spiralcore-gateway/1 packets.
Same TLS1.3 chain+name+exact-peer-pin checks, no 0RTT/resumption. Object transport
allows one bidirectional stream, zero unidirectional streams, no datagrams,
1MiB stream/connection receive windows and4MiB send window. Reference:
https://docs.rs/quinn/0.11.12/quinn/struct.TransportConfig.html

Each stream message is u32LE length followed by existing SCWBAT01 bytes, encoded
and decoded by wan::encode_batch/decode_batch, max20MiB and512frames. Both
requests and responses use that envelope. Zero length is orderly end, acknowledged
by zero length; truncated EOF is always error. Success finishes stream delivery;
transport ACK is not application processing. Deadline/cancel/malformed/partial
I/O poisons the channel, including a dropped in-progress future. No resume at an
unknown frame boundary. ObjectChannel owns Peer; one application stream only.
Expose send_batch, recv_batch->Option<Vec<Frame>>, exchange, finish and
acknowledge_finish, all with absolute deadline and CancellationToken. Counters
separate logical envelope/framing bytes and QUIC UDP bytes; packet capture is
independent. Receiver never executes uploaded source.

## Service/config contract

A new `spiralcore-gateway-object run CONFIG.json` binary uses a bounded JSON config
(max64KiB, deny unknown fields). It runs on macOS/Linux for cooperating objects;
Linux namespace campaign measures actual UDP transport. Shared required fields:
role(listen/connect), bind, peer, server_name, certificate, private_key,
trust_anchor, peer_certificate, scope(nonempty<=128chars), policy_epoch(u64),
store_dir, stats_path, ready_path, duration_secs(1..3600), groups, verify_files.
Credential formats match existing gateway. Each service admits one configured
peer; context derives sender/recipient from actual configured certificate pins,
plus local scope/epoch. Receiver authority accepts only that exact context;
remote payload cannot grant scope. Policy changes take effect on restart.
Report/readiness paths require fresh files in existing writable parents. Reserve
exclusive ownership before readiness or error writes; reject existing targets,
input/credential/store overlap and normalized aliases. Initial publication must
not replace another file; later readiness updates require the held output inode.
Client operations and receiver batches carry matching transfer IDs in local
receipts for exact CPU attribution; this adds no wire fields.

`groups` (connect only,max8) contain label, mode(raw/whole-zstd/warm/runtime),
optional schema_path/checkpoint_path/recipe_path, and operations(max32/group).
An operation has label,path(optional for checkpoint/schema),action(send/seed/
schema/checkpoint),warm_probe(bool). Recipe file is canonical Recipe::encode,
not code. Only one group's sender is retained at a time on the persistent channel.
Use<=16MiB inputs, Runtime MAX_OUTPUT and all Endpoint default shared limits.
`verify_files` (listen,max64) are local expected files for optional exact readback
validation after endpoint close/reopen; they are never sent or trusted as remote
permission. Production listen may leave this empty; digest validation still
precedes Published. Server tracks committed IDs, reopens store and verifies
committed bytes; expected fixture comparison supplies independent exactness.

CPU/wall starts before input loading and encoding for each client operation and
ends after verified publication reply. Group preparation is separately timed,
including checkpoint load/replay/hash and codec preparation; seed/schema/checkpoint
operations are charged. Server handle CPU/wall includes decode and durable commit;
final reopened readback separately reported. No JSON/log output between measured
operations; retain bounded receipts and write at session end. QUIC setup and
whole process CPU/wall separately reported, not added to nested CPU. Each selected
mode and original selection candidates are explicit. Failure never emits an
unqualified successful completion; transport closure is distinct from publication.

## Measurement and acceptance

24 sessions:4profiles x2links(local,80msRTT/10Mbit/s bidirectional)x3trials.
Each has two tiers using width4096:289steps=1,047,202 bytes and4096steps=
14,851,384 bytes (max trace). Initial checkpoint3,382B. Confirm generated lengths
before admission; no padding or runtime-limit changes to invent16MiB output.
Per tier: generated,512bytes of deterministic localized residual edits, exact
repeat of edited object, unrelated same-length opaque input, localized opaque
edits and their exact repeat.288 requested object deliveries; schema/checkpoint
setup separately counted (312 total operations). Same fixture
hashes across arms; equal warm_probe opportunity; prepared contexts reused.
Record generation outside transfer and all preparation/seeding inside cumulative
session cost. Runtime expected to win generated/residual cases, but actual
selected mode governs claims. Warm must demonstrate actual reuse; no forced
codec or dropped losing case. Include initial cold/seed and cumulative1..6
object totals. Require exact reopened bytes, bounds/scope/missing-state/truncated
channel tests, captured both-direction bytes, sender+receiver CPU, full operation
and session wall. Three trials descriptive only. Shaped local is not actual WAN.
Capture full original lengths at one boundary, include authentication/closure,
retain congestion costs; zero capture loss. Reuse finite-queue calibration,
read-only CI token, source/binary hashes, no auto retries,4GiB disk floor.
Large artifacts archived to existing private HF dataset with fresh pinned hash
verification before local cleanup. Formal RED/GREEN and negative results stay in
GitHub per user request. Lean cross-author review; no broad audit/coverage work.

### Pre-measurement fixture amendment

Before any campaign traffic, extend each tier with an opaque object carrying the
same512-byte localized edit pattern, then its exact repeat. This isolates warm
rolling reuse on data that zstd cannot shrink; the generated-runtime fixture is
already compressible and is insufficient as the only warm-change test. No new
size/profile/trial is introduced. Six requested deliveries per tier means288
object deliveries, plus12schema and12checkpoint operations=312operations total.
Eight unique expected files per receiver. Runtime may truthfully fall back on
opaque edits because its base is the fixed checkpoint; do not claim automatic
combined runtime+rolling selection. Count every seed and both additional inputs.

### Scope binding before object traffic

After pinned authentication, the initiator sends one SCWBAT01 batch containing
Frame(kind6,transfer=local namespace ID,seq0,body=SCGNSP01). The receiver compares
it exactly with its locally derived namespace and echoes only on equality. The
initiator requires that exact echo. This control is intercepted by the service
and never passed to Endpoint. Scope/epoch misconfiguration therefore fails before
any cache publication. It conveys no new authorization and is counted in channel
setup traffic. Publication receipts distinguish an acknowledged target-cache hit
from a full reconstruction using the actual reply to Begin/probe, not only the
offered mode; a full batch may already be sent before a cache-hit reply arrives.
