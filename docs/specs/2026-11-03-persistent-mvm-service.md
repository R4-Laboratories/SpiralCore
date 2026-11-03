# Persistent canonical MVM service and host resource ownership

Date: 2026-11-03. Status: proposed implementation contract; independent design,
code and security review and executed qualification are required. Baseline:
SpiralCore `b154fb7`. Delivery authority: [#49](https://github.com/R4-Laboratories/SpiralCore/issues/49),
[#43](https://github.com/R4-Laboratories/SpiralCore/issues/43), and parent
[#42](https://github.com/R4-Laboratories/SpiralCore/issues/42). The live upstream
check identified MVM main `ace537b0a07fd48322813242ceb3f19cc6ddc3ff` and open
MVM #4629; this document does not represent upstream acceptance.

Independent planner review on 2026-11-03 accepted the first-slice authority
boundary and private application-phase seam without a blocking concern. Exact
implementation code/security review and runtime qualification remain required;
that design review does not close any roadmap acceptance criterion.

## Decision and delivery sequence

Keep canonical MVM envelopes, signed grants, receipt authority and object-v1.
Deliver three separately reviewed increments:

1. Move object/storage/codec work out of the shared broker dispatch gate; add
   one host transport owner used by all admitted guest links within the owning
   orchestrator, including links to separate child processes. Qualify actual
   aggregate relay saturation and release. Keep pipeline one.
2. Qualify long sessions beyond 2,048 completed Calls under unchanged finite
   ceilings and retention, with periodic expiry and safe retry identities.
3. Add explicitly negotiated bounded pipelining only after the first two gates,
   preserving the same protocol and endpoint authority.

A persistent process is not a durable mailbox. The first slice issues only
`AcceptedEphemeral`; object durable publication/readback remains a separate
fact. No encrypted acceptance journal, exactly-once application execution across
power loss, general daemon IPC, FlowMux forwarding, owner access, guest NIC,
new MVM envelope, or upstream activation is introduced by this decision.

## Source and authority map

| Source | Present responsibility | Proposed seam |
|---|---|---|
| `crates/spiralcore-mvm-adapter/src/broker.rs` | Process-local `BrokerContext`, serialized trusted time and `HostBudget` | Preserve short accounting phases; no object work under `with_time` |
| `crates/spiralcore-mvm-adapter/src/service.rs` | Canonical verification, plaintext queue/calls, transactional replay admission, owner/epoch and retained identities | Preserve authority and retention; run explicit bounded expiry |
| `crates/spiralcore-mvm-adapter/src/service/ordering.rs` | FIFO cursor per address/sender/execution epoch/scope, boot-independent | Preserve cursor and unknown gaps; no empty-scope eviction |
| `crates/spiralcore-mvm-adapter/src/receipt.rs` | Canonical encrypted receipt carrier, source observations and retained attempts | Preserve deadline plus retention and at most eight attempt bindings |
| `crates/spiralcore-mvm-adapter/src/service_object.rs` | Private admitted proof, binding policy checks and shared object endpoint | Run on bounded endpoint worker outside broker dispatch |
| `crates/spiralcore-mvm-live/src/campaign/canonical.rs` | Stable sender order, Call/Reply and receipt exchange, 2,048-Call witness cap | Split handler phases, then steady-state retention and retry |
| `crates/spiralcore-mvm-live/src/orchestrator.rs` | Owned worker spawn/reap and keyless two-guest relay | Own one transport budget; reserve every link before use |
| `crates/spiralcore-mvm-adapter/src/live.rs` | Bounded ciphertext frame read/write, existing framing and counters | Acquire transport buffer credit before payload allocation/read |

`HostBudget` is an `Arc<Mutex<_>>` shared only within its process. Its internal
mailbox, capability, replay and receipt meanings cannot be exported as trusted
host observations by accepting child-supplied counters. A child bootstrap,
transport connection or host lease cannot confer destination signing authority.

## Two resource domains

The host transport owner owns ciphertext buffer and link resources across all
its admitted links, including links terminating in different guest processes.
It receives trusted route/tenant classification from local provisioning, never
from an encrypted inner header, client-supplied tenant string or claimed receipt
class. It has no decryption or signing keys and cannot observe plaintext object
identity, cache membership or canonical message kind. The exact typed contract
and report schema are in [persistent-host-budget-v1](../api/persistent-host-budget-v1.md).

Endpoint `HostBudget`, `LocalMailboxService`, `ReceiptTracker`, FIFO state,
plaintext codec workspace and cache remain in their existing endpoint/guest.
Their signed per-tenant/mailbox/sender contributions remain enforced there.
Fixed VM memory, process counts and transport credits are useful independent
host limits; none measures actual guest plaintext/cache occupancy. The first
host transport ledger therefore does **not** complete #43's aggregate canonical
plaintext admission criteria or #49's host/cache capacity program. A separately
reviewed authoritative shared plaintext service or bounded partition contract
would be needed to establish that broader aggregation.

A single orchestrator budget is aggregate for its links. Calling that budget
host-wide across independently started orchestrators additionally requires a
host installation owner lock/supervisor and refusal of competing owners. Until
that enforcement and its crash test exist, reports must say
`scope=orchestrator` and the host-wide gate remains `NOT_RUN`. There is no
per-run path or PID string that by itself proves host-wide exclusivity.

## First implementable transport slice

Instantiate one `HostTransportBudget` before workers/relays start. Inject clones
of that owner into every link; no link constructs its own maximum. Use bounded
trusted route slots to aggregate tenant totals without storing guest identities.
Add `session_with_budget(..., budget: &HostTransportBudget)` as the supervisor
integration seam. A convenience `session(...)` may construct a dedicated owner,
but must identify that isolated owner scope and cannot claim to aggregate other
independently launched sessions. Place the ledger in the live crate's
`host_budget.rs`; it is host transport policy, not canonical mailbox authority.
The owner accounts active links, simultaneously retained serialized ciphertext
buffer capacity and frame slots. Configure host and tenant ceilings and reserve
all contributions atomically with checked arithmetic. Configuration is explicit
and immutable during a run. Failed reservations do not alter usage.

Reserve one bounded frame slot and the maximum permitted serialized buffer
capacity before calling an existing allocating frame read. A later exact-length
reader may reserve after the fixed-size length prefix but before allocation; its
prefix remains bounded and length is checked against the existing canonical
transport limit. The initial pessimistic full-frame reservation avoids adding
an alternate frame format. Hold the reservation through route validation and
all writes using that buffer; release only after the buffer is destroyed and no
retry owns it. A failed or partial write leaves destination acceptance unknown.
Drop owned buffers on terminal failure; a retry must reacquire credit and preserve
its canonical request identity. Never issue a destination receipt from the relay.

Count both directions, and ensure every simultaneously live buffer is charged.
Avoid retaining a previously forwarded buffer into the next read. `decode_envelope`
may allocate a separate bounded representation: serialized buffer accounting is
not total allocator/RSS accounting. Record that limitation, audit temporary copy
bounds, and qualify RSS/CPU separately before making a host-memory bound claim.
Reserve link state before socket/worker activation; release link credit only when
its I/O tasks and buffers are gone. A worker/VM budget is a separate optional
fixed allocation and is released only after confirmed exit/cleanup, not socket
EOF alone. This slice adds no interservice IPC or new remote authorization.

Transport congestion cannot borrow FlowMux, machine-control or telemetry queues.
A keyless relay cannot safely infer reserved receipt traffic from untrusted
claims. The first pipeline-one relay reserves enough link-owned capacity for its
bounded exchange and forwards each frame before reading another; endpoint
receipt-control reserves remain independent. A future concurrent scheduler must
prove that ordinary data cannot prevent control progress, without accepting an
unauthorized reserved-class marker. FIFO and signed endpoint rate limits still
apply after transport admission.

## Short broker phases and object worker

Keep `BrokerContext::with_time` as the serialization boundary for mutable service,
receipt, ordering and shared endpoint accounting actions. Its sampled time is
valid for that phase only. Socket I/O, worker waits, object reconstruction,
compression, disk commit and exact object readback must execute outside it.

The first handler split is:

1. Under dispatch, sample trusted time, verify/admit the request and obtain the
   existing private `AdmittedMessage`. Prepare/reserve receipt ownership.
2. Outside dispatch, send the destination receipt. Run `ObjectMailbox::context`
   and `handle` using a fresh trusted time sample and the admitted proof. Preserve
   `AdmittedMessage::with_verified` and binding policy/owner checks, including
   their per-binding locks. Do not copy its Context into a reusable authority.
3. Reenter dispatch, obtain a fresh time, revalidate current binding generation,
   epoch, proof, deadline and cancellation, then create the canonical Reply and
   register its source tracker. Reply preparation and the cached canonical reply
   commit remain one bounded control phase.
4. Outside dispatch, send Reply and wait for its receipt; reenter dispatch with
   fresh time to verify/acknowledge it and release reply-cache ownership.

The broker may sample a trusted work time in a short phase, but no pre-worker
`ServiceTime` may be reused for post-worker accounting. Existing policy locks
serialize revocation against a sensitive object operation; taking the operation
out of the broker gate must not remove those locks. If deeper lock splitting is
needed, introduce a reviewed non-clone work permit with checked binding generation,
owner/epoch, deadline and bounded workspace, and recheck at each sensitive commit.
An expired/revoked result cannot be sent as an authorized Reply. A completed
object side effect may already exist: timeout/cancellation is not rollback.

The approved minimal seam is a private helper in `campaign/canonical.rs`:

```rust
fn application_phase<T>(
    broker: &BrokerContext,
    work: impl FnOnce(ServiceTime) -> Result<T>,
) -> Result<T>;
```

It samples time in a short `with_time`, returns from that closure to release
dispatch, then invokes `work(now)`. Only the existing proof-checked object
context/handler operations use this snapshot; no service, tracker, ordering or
aggregate budget mutation may use it. Keep this restricted seam private rather
than adding a public arbitrary-work escape hatch to `BrokerContext`. Existing
per-binding/live/owner guards remain intact. A worker waiting for such a guard
may still block control for the same owner; the first regression and claimed
progress apply to independently owned mailbox addresses. Removing that remaining
contention or strengthening deadline checks inside a long storage operation
requires a separate reviewed change.

Bound workers, per-endpoint outstanding object jobs, workspace, storage and
queued results before dispatch. Start with one object job per endpoint. That
preserves object/cache ordering while allowing unrelated endpoint control work
to proceed. This is no guarantee of preemption inside one object operation or
of strict fairness from `std::sync::Mutex`.

## Safe sessions beyond 2,048 Calls

The existing 2,048 cap is a witness bound backed by larger identity ceilings;
removing the comparison alone is unsafe. Keep fixed ceilings and the existing
300,000-ms terminal extension and 60-second rate histories. Tick service and
tracker expiry on active and idle sessions with the owner's current clock.
Retained terminal handles remain unforgeable, and late replies cannot satisfy
a new correlation. Signed grant/binding expiry still terminates authorization;
a longer session needs explicit current renewal rather than extended fixture TTL.

Admission is limited by concurrent retained state, not lifetime operation count.
Qualify more than 2,048 complete Calls across multiple full retention cycles with
one service, tracker, FIFO stream and owner, unchanged ceilings and no hidden
restart. Report peak and post-expiry counts, rejected calls, latency and exact
readbacks; do not claim arbitrary offered rate. Active-call credit can be freed
on terminal status while replay/correlation/receipt identities stay retained.

Allocate `MessageOptions` once per logical request. If registration/admission is
refused before sending, retain the same options, stable identity, correlation,
sequence and expiry and retry only while they remain authorized. Do not allocate
a replacement sequence or create a fresh scope to disguise an unknown FIFO gap.
The current sequential register-call then register-send path may leave one
reservation committed when the second refuses; reconcile that same identity or
provide reviewed transactional preflight/rollback before claiming general retry.

If an unadmitted sequence's identity expires, do not infer the receiver knew it
or skip to the next position. Mark the stream fenced/unavailable and require an
explicit authorized owner/epoch recovery. Known accepted terminal entries may
advance only through known contiguous positions. Persistent empty cursors and
terminal entries behind unknown gaps remain charged. Scope churn has a finite
ceiling and explicit `Full`, not silent LRU reset. A new scope is a new stream,
never evidence of same-stream recovery.

## Failure, ownership and recovery

| Event | Required result |
|---|---|
| Full before allocation | No new buffer, frame or acceptance; bounded backpressure/refusal |
| Partial read/write | Destroy owned buffers on exit, report actual counters and uncertain remote acceptance |
| Worker error/cancel | Close owned I/O, join tasks, drop buffers; terminate/reap only owned child if required |
| Lease deadline or disconnect | Fence further operations; do not reclaim resources still usable/owned by a live old holder |
| Host owner crash | Ephemeral links stop; no durable mailbox claim or automatic successful resume |
| New host owner with unknown old resources | Refuse admission until supervised ownership/reconciliation proves safe bounds |
| Endpoint restart | Fresh authorization and explicit epoch policy; no reconstruction of old FIFO from a new budget |
| Clock/policy/poison failure | Fail closed and keep uncertainty visible |

The first in-process RAII owner has no expiring distributed leases. Dropping a
client handle does not make remotely held canonical work disappear. Future IPC
requires authenticated local peer binding, bounded requests before allocation,
owner incarnation and non-reusable lease IDs, per-client quotas, explicit fencing,
and crash-safe orphan reconciliation. Lease timeout alone cannot release live
client memory and make it safe for a new client to reuse the same capacity.

## Pipelining contract and deferred gates

Pipeline remains one in the first slices. A later mutually pinned/negotiated
window may add bounded outstanding Calls to the existing canonical envelopes;
all messages keep stable IDs, correlation and ordering. Do not append a private
ACK or overload receipts into application replies. Negotiation must be covered
by canonical trusted policy/version binding, and absence means window one.
The present fixed four-frame relay choreography must be replaced by a bounded
bidirectional pump/demultiplexer before window greater than one: a receipt and
Reply can arrive in either order. Reserve the complete worst-case window,
reply/control/cache/attempt retention and per-tenant shares first. No auto-raise
of host limits or activation of a second protocol is allowed.

Required evidence includes focused accounting boundary/overflow/drop tests,
two separate worker processes sharing one owner, slow/cancelled/malformed tenant
beside healthy traffic, object work blocked while unrelated control progresses,
revocation during worker execution, idle expiry, greater-than-2,048 same-owner
sessions and unknown-gap refusal. Measure per-tenant p50/p95/p99, high-water use,
actual bidirectional traffic, setup and endpoint CPU, exact lengths/readbacks,
and object durable completion. No timing or throughput claim follows from source
review alone.

Host installation exclusivity/crash recovery, total host RSS/cache/workspace
bounds, canonical aggregate plaintext service, churn/fair scheduling envelope,
negotiated pipeline, native macOS/Linux backend runs and direct/relay canonical
WAN remain separate gates. Native Windows durable gateway remains unqualified;
WSL or cross-compilation does not supply native evidence. Keep #42/#43/#49 open
for all unexecuted criteria. Use compact evidence and the existing shared Cargo
target; no new Mac VM image or large local dataset is needed for this design.
