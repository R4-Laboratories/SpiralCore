# SC-OPT-018: acknowledged rolling sessions

Accepted direction under the user's instruction to implement persistent warm
transfer. Base: `28a373ddb7ab9d236fd4b872db128559d4404cb8`. Production changes
start after the parent commits this design; no further user approval is needed.
This is a design and measurement hypothesis, not a new performance result.

## Decision and alternatives

Add small transport-agnostic `WarmSession` and `RuntimeSession` types in
`src/session.rs`. Callers retain their authenticated transport and pass the
existing batched exchange closure. Continuing to manually compose current calls
can already reuse contexts, but duplicates acknowledged-base bookkeeping and
cannot safely reuse the private base identity. The session owns those invariants.
A broader connection protocol, receiver object cache or unified runtime/schema
selector would introduce unrelated semantics; defer them.

`WarmSession` owns one `PreparedSelector`, one immutable last-acknowledged input
buffer and its cached object ID, receiver-readiness flags, and the namespace
digest from `Context::namespace()`. The base type and ID constructor stay private.
Use an additive crate-private selector seam accepting this verified owned base;
ordinary public selector APIs and exact tie/cost behavior remain unchanged.
No public API accepts an unchecked hash as proof of bytes.

Proposed public surface (exchange retains its current `FnMut(&[Frame])` contract):

```text
WarmSession::new(&Context, artifact: &[u8]) -> Result<Self>
seed_base(&mut self, &Context, Vec<u8>, transfer_id, exchange) -> Result<Stats>
send(&mut self, &Context, Vec<u8>, transfer_id, warm_probe, exchange)
    -> Result<SessionStats { transfer: Stats, selection: WarmSelection }>
seed_schema(&mut self, &Context, transfer_id, exchange) -> Result<Stats>
recover(&mut self, &Context, Vec<u8>, fresh_transfer_id, exchange) -> Result<Stats>
invalidate_receiver_state(&mut self)
base_id(), base_ready(), schema_ready(), retained_bytes(), retained_charge()
```

Own each input `Vec<u8>` before preparation. Cap both its length and capacity at
16 MiB; charge its retained capacity. After validated durable `Published`, move
that same buffer into the base slot and retain the already verified `Stats.object`.
The normal sender hashes the target once; no promotion copy or second hash is
needed. When Sparse wins, the selector uses the old base's cached ID rather than
rehashing it. Failure may consume/drop the offered input; recovery callers supply
the desired bytes again explicitly. One mutable session permits one active send.

## State and failure contract

- A successful base seed or target send promotes the same owned input only after
  the existing client verifies transfer ID, sequence, terminal reply and full
  expected object ID. A schema/checkpoint seed is auxiliary and never replaces
  the application's rolling base.
- A missing/evicted base or any failed network attempt returns its original error,
  does not promote the target and clears receiver-readiness hints. The previous
  acknowledged local bytes/ID may remain inspectable, but are ineligible as a
  receiver base until explicitly re-established. No implicit retry or reoffer.
- Preparation/limit failures before network I/O leave existing readiness intact.
  An unseeded winning Schema returns `Error::Missing` before I/O. The caller may
  explicitly `seed_schema`, then initiate a fresh send; every setup transfer is
  visible and charged. Never treat a readiness flag as receiver authorization.
- `recover` is an explicit Raw identity probe using the existing `warm_probe=true`
  path and a fresh transfer ID. A committed target returns a validated ACK; an
  absent target is sent Raw by this explicit operation. Promotion occurs only
  after that ACK. The same transfer ID with a changed offer is not a valid retry:
  the current receiver ledger correctly rejects it.
- Every operation checks the supplied `Context` namespace digest. Any context,
  capability, policy-epoch or execution-epoch change invalidates scoped state and
  returns an error before network I/O; construct a new session for the new context.
  Explicit invalidation handles transport replacement/restart knowledge. The
  caller must bind the supplied context to its authenticated exchange; receiver
  `Authority` still checks every frame and publication.

Sender state is bounded by a 512 MiB conservative admission charge, not a claim
about process RSS. Warm retention includes selector context charges, artifact,
metadata and base capacity; preparation additionally charges target capacity and
a 128 MiB transient allowance. A separate runtime session avoids retaining unused
selector contexts: charge prediction/checkpoint/setup buffers, metadata and last
acknowledged capacity, plus target capacity and a 384 MiB transient allowance for
its existing codec path. Reject over-budget work before I/O. These allowances
include existing bounded candidate buffers/codec work; receiver quotas remain
unchanged. Applications retaining multiple sessions need a shared outer budget.

## Existing runtime and actual connection reuse

`RuntimeSession::new(&Context, PreparedPrediction)` consumes the immutable
prepared state. Expose `seed_checkpoint`, owned-input `send` returning existing
`AdaptiveStats`, explicit `recover`, invalidation and retention/readiness getters.
It delegates to the existing prepared runtime codec and retains the fixed
prediction across sends; it does not advance logical/physical time, refit a
recipe or replace the current runtime candidate policy. Test exact sequence reuse
and checkpoint loss separately from the main warm-selector benchmark.

The current `HttpClient` already retains a `ureq::Agent`, but the server handles
one request per connection and sends `Connection: close`. Add explicit configured
server entry points while keeping `serve`/`serve_observed` defaults unchanged.
`ServeConfig` carries duration, global request cap and `Close` or bounded
`KeepAlive`: 1–64 requests per connection, idle timeout 1–1000 ms, and the existing
absolute per-request deadline. The campaign uses 32 requests and 500 ms idle.
Reauthorize each request; honor client close, close rejected/truncated requests,
and preserve global quotas. Add connection ordinal/request-on-connection to
observer telemetry; do not claim reuse merely because an Agent is retained.
All measured arms use the same policy. An optional strictly parsed plan setting
enables it; absent configuration keeps the historical close behavior.

Do not bypass fresh receiver publication reads, hashes, corruption checks,
durability or final authorization. First attribute receiver Begin/Data/Finish
cost with external grouped timers: 44 actual objects (two sizes, two clock modes,
eleven objects per sequence). Use attribution only if median instrumentation
overhead is <=1.10; retain both modes and exactness. Any production receiver
optimization needs a material measured opportunity and separate evidence.

## Qualification and experiment

Test seed plus rolling edits, append/truncate, unrelated escape and repeated
objects against existing exhaustive selections and exact serialized results.
Include missing/corrupt bases, denied/changed context, loss before commit, lost
ACK after commit, wrong terminal ID/sequence, explicit recovery, restart and
resource rejection. Failed sends must not advance base identity or retain a
false-ready hint. Qualify runtime prediction ownership/checkpoint reuse and
actual persistent HTTP connections, idle expiry, request caps and close behavior.

Both selector arms fit the identical training data **once per session**. Fresh
constructs a selector from that artifact for each update and manually owns the
rolling acknowledged input; persistent constructs once and uses the session's
cached base identity. Both receive the same owned inputs, and both retain one
HttpClient. Whole likewise reuses its prepared codec and connection. Record fit,
constructor, seed, ownership/hash, each update and total lifecycle without
inflating the comparator with ten fits.

Each session sends one base and ten updates: nine interior 32-byte edits with
600-byte Sparse results and one unrelated escape that must select the true
minimum. Use 1 and 16 MiB sizes, three arms, ten local pairs/size: 660 transfers.
Local advancement requires exactness and cumulative sender CPU for setup plus
ten updates at most 1.05 times the matched fresh selector at both sizes; seed
and full lifecycle are also reported. Then a six-pair/size real-WAN pilot uses
all six arm orders once: 396 transfers in six bounded stores. Label it a pilot,
not a thirty-pair acceptance study. The measurement contract must freeze the
escape position, fixture/source identities and exact per-store byte charge before
measurement. Retain failures/negative results; no automatic timing retries.

Measure body bytes, sender/receiver-handler CPU, durable elapsed and full paid
lifecycle. Exchange elapsed is nested, not pure wire time. Connection telemetry
establishes only the observed hop; relay TLS termination, transport overhead and
energy retain their existing limitations. Runtime results remain a separate
existing-codec profile rather than an asserted all-codec optimum.

## Diagnostic addendum after incomplete WAN attempt 1

The [retained failure diagnosis](../network/evidence/2026-10-31-persistent-session/qualification/wan-attempt1-diagnosis.json)
pins source `58990cec4d190f815fccd51df9cadf94029ab78f` and hosted run
`41167011759`. The first failed 16 MiB localized update selected Sparse at 600
body bytes; the client recorded a 342-byte attempted request, no response body,
and `Storage` after about 500 ms. No admitted receiver observation exists for
that request. Pinned ureq 3.5.0 maps HTTP 4xx/5xx to `Error::StatusCode` by default,
and the current sender maps every send error to `Storage`. Consequently the
existing evidence cannot distinguish HTTP rejection, timeout and pooled-socket
I/O failure. Preserve all attempt-1 outputs and caches as an incomplete failed
campaign. The elapsed time alone does not justify changing the 500 ms policy.

**Decision: add sanitized diagnostics, with unchanged transport policy.** Add
`HttpClient::last_transport_failure() -> Option<TransportFailure>`, cleared at
the start of each `exchange`, including before local frame encoding. The owned
`Copy + Serialize` record has only `stage`, `category`, optional numeric
`http_status`, optional `io_kind`, and optional `timeout_phase`. All string fields
are static values selected by closed match allowlists; unknown variants become
an explicit `other` category. Never format an error or traverse its source chain.
Success and errors before transport starts must not expose a stale failure.
Keep current public error mappings, reply checks, counters and retry behavior.
The harness reads and serializes this diagnostic after its operation timers stop.

Add `serve_diagnosed_configured(listener, endpoint, token, scopes, config,
observer, diagnostic)` with a separate `ConnectionObservation` callback. Existing
serve wrappers and `RequestObservation` remain compatible and produce no extra
diagnostic output unless explicitly opted in. The connection record contains
only `connection_ordinal`, `request_ordinal`, `request_on_connection`,
`request_counted`, `reason`, optional numeric `http_status`, optional allowlisted
`io_kind`, bounded `header_bytes`, and `elapsed_micros`. Elapsed time starts at
the current request-attempt loop, before idle/header waiting; it is not connection
age or pure network time. `request_ordinal` denotes the assigned or next attempted
ordinal, while `request_counted` states whether the existing global counter was
advanced. A diagnostic is never an admitted transfer or publication receipt.

Fixed reasons distinguish `idle_timeout`, `peer_eof`, header timeout/truncation/
I/O/limit, request rejection, body timeout/truncation/I/O, process rejection,
response I/O, and normal close due to client request, connection cap, global cap,
server deadline or default close policy. Preserve the distinction between the
existing silent close for an empty reused-header timeout and HTTP 400 for a
partial-header failure. Count header bytes without retaining or exposing them;
the count cannot exceed the existing header limit. Do not log payload, header
values, credentials, URLs, peer addresses or arbitrary error strings.

Callbacks stream synchronously outside the receiver `process` timer; the library
does not buffer diagnostic history. Their actual cost remains in any enclosing
elapsed interval. An exclusive CLI diagnostic log may impose a row budget, but
exhaustion or observer failure must make diagnostic completeness explicitly fail,
never silently discard records. Keep all deadlines, quotas, authorization,
connection reuse, codec bytes and durable commit rules unchanged. No new
dependencies, implicit retries or timeout relaxation belong to this change.

Qualify actual status/I/O/timeout classification and reset-after-success; server
idle versus partial-header outcomes; rejection and response-write failure;
unchanged legacy observation counts; and absence of sentinel request secrets in
serialized diagnostics. A bounded diagnostic reproduction needs new source and
run identities and retains its first outcome. Diagnostic evidence may motivate a
later separately justified transport correction; it cannot retroactively turn
attempt 1 into a completed paired WAN performance result.
