# Plan: explicit incomplete-session reconciliation

Date: 2026-11-03. Lead-approved implementation plan before RED/code. Base main
`e984d2d64cdf0483d50d6d59319276c85e7df644`.
Contract: [session reconciliation v1](../api/session-reconciliation-v1.md).
Dependencies: qualified [registered Call retry](2026-11-03-registered-call-retry.md),
receive-idle API, existing caller demultiplexing and receiver cached replay.
The explicit retry also requires the reviewed
[fixed-deadline stream-send contract](../api/stream-send-deadline-v1.md).
This continues partial C8 under issues #43/#49; it does not complete either issue.

## Owned files, order and risk

| Step | Exact files | Dependency / risk |
| --- | --- | --- |
| Public-entry RED and direct fault fixture | `crates/spiralcore-mvm-live/tests/reconciliation.rs`; `crates/spiralcore-mvm-live/src/campaign/canonical/reconciliation_tests.rs` | Approved contract; high |
| Bounded signal and public embedding wiring | `crates/spiralcore-mvm-live/src/campaign/reconciliation.rs`; `crates/spiralcore-mvm-live/src/campaign.rs` | Recorded missing-API RED; medium |
| Retained attempt state and caller loop | `crates/spiralcore-mvm-live/src/campaign/session.rs`; `crates/spiralcore-mvm-live/src/campaign/session/tests.rs`; `crates/spiralcore-mvm-live/src/campaign/canonical.rs` | Signal and state REDs; high |
| Fixed canonical I/O deadline | `crates/spiralcore-mvm-adapter/src/live.rs`; `crates/spiralcore-mvm-adapter/tests/service_tcp.rs`; `crates/spiralcore-mvm-live/src/campaign/canonical.rs`; `crates/spiralcore-mvm-live/tests/reconciliation.rs` or its `tests/common/` fixture | Reviewed send-view contract, missing-API and blocked-write RED; high |
| Historical-Call receiver preamble | `crates/spiralcore-mvm-live/src/campaign/canonical.rs`; `crates/spiralcore-mvm-live/src/campaign/canonical/reconciliation_tests.rs` | Caller GREEN and receiver semantic RED; high |
| Actual relay fixture access | `crates/spiralcore-mvm-live/src/orchestrator.rs`; the same reconciliation test module | Direct semantic GREEN; medium; test-only re-export of existing relay |
| Qualification and publication | `docs/network/evidence/2026-11-03-session-reconciliation/`; `docs/network/session-reconciliation-results-2026-11-03.md`; parent plan and README checkpoint | Frozen reviewed source; medium |

Within a step, make bounded commits around one state transition (roughly 50
production lines) and run its focused gate before the next transition. Root owns
shared `canonical.rs` changes sequentially; test ownership is separate. Do not
edit caller and receiver paths concurrently in that file.

## Small sequence

1. **Public entry and direct semantic RED.** First compile a real embedding test
   importing campaign::sender_with_broker_and_reconciliation and opaque public
   ReconciliationSignal; retain its missing-API RED. Then, in the signed TCP fixture,
   withhold the initial Call receipt but deliver the valid Reply and its ACK.
   Receiver completes and cache releases; caller remains incomplete with its
   plaintext Reply charged. Controller explicitly queues one retry. Require exact
   original Call bytes, actual canonical Duplicate receipt only from completed
   receiver, exact application result once and one object execution. Current
   source lacks the opt-in entry/preamble and cannot satisfy this gate. Keep the
   failure and distinguish API-absence RED from the semantic failure.
2. **Public embedding and bounded caller seam.** Add the public development-witness
   sender entry and opaque public signal; keep requests and bounded factory private.
   Wire its real campaign::Client::call branch to the canonical opt-in method:
   one fresh pair per Call, on_call(signal) outside broker dispatch, then move the
   requests endpoint. Callback receives no payload/authorization and transfers the
   signal to an embedding-owned controller thread; Session spawns none. Signals
   disconnect on invocation exit. Accept no arbitrary raw Receiver. Existing sender
   and guest strict branches allocate no channel, call no callback and perform no
   Idle polling. Do not add dead-code allowances or test-only recovery source.
   Persist eight-invocation count
   in PendingCall and reuse its existing sixteen-event/original-identity state.
   On 250 ms zero-prefix Idle, run original-lifetime and ordinary completion/FIFO
   gates first. Complete if ready; if all observation/ACK facts exist but canonical
   poll is Pending, retain any queued command and wait without resending current
   Call. Otherwise dequeue at most one explicit request. Reuse the
   canonical tracker and retry API; no new registration, payload consumption or
   borrowed permit. Add deterministic state tests before production changes.
   Before accepting retry I/O, expose the adapter's existing before view as opaque
   PeerTransport with precise `use<S>` capture under the separate send-view
   contract. Retain external missing-API RED and a channel/write blocked past a
   short original deadline despite a longer parent timeout; merely checking time
   before invoking retry does not prove this bound. Wire all canonical exchange
   sends to before(original_deadline): caller original/retry/Reply-ACK and receiver
   original receipt/Reply/duplicate receipt/cached Reply, using its fixed
   ExchangeDeadline. Historical preamble receipts use the fixed preamble Instant
   capped by the authenticated old request's signed remaining lifetime. Reproduce
   late retry and ACK/replay backpressure through the actual consumer; test
   repeated views/connections with the same original cap, partial-write expiry,
   channel acquired before expiry and existing route/cancel/fence precedence.
   Default adapter connect, wire/codec and API #64 implementation remain unchanged.
3. **Historical receiver preamble.** Add bounded authenticated older-control and
   retained old-Call handling. Inactive path checks fixed deadline then calls
   returns.admit first while room remains for a later ACK: AcceptedEphemeral
   obtains proof/object work once; Duplicate must pass original-owner retry lookup
   None before reservation and again immediately before bounded send. Other
   outcomes, cached Some or lookup errors refuse/fence. This avoids a second
   crypto verification on normal fresh Calls without authorizing historical send
   from a shared duplicate disposition alone. At the sixteenth normal frame,
   prelookup first: UnknownHandle→EventLimit before allocating fresh state with
   no ACK slot; only authenticated known None may get its actual Duplicate receipt.
   Preserve mandatory prelookup/exact-cache comparison in the active ACK-wait loop.
   None never infers completion; Duplicate alone cannot complete a caller without
   retained Reply/ACK facts. Preserve active cached replay, no-resume latch,
   original deadlines, event/send ceilings and top-level cap. Count preamble work
   and traffic. One control-only exchange cannot return a second Handled.
4. **Fault combinations and relay GREEN.** Apply the focused gates below through
   the real duplex relay as well as direct sockets. Use bounded deterministic
   forwarding/withholding of complete canonical frames, never a competing codec,
   fabricated receipt or plaintext inspection by the relay. Controller signaling
   is explicit test/application intent, not an automatic timer. Verify retained
   failures, authority and resource bounds before broader qualification.
5. **Review and qualification.** Independent exact-source/security review, focused
   and affected release tests, formatting/Clippy and normal platform CI. Register
   any added runtime comparison before executing it; include opt-in idle/wake,
   control/replay traffic and lifecycle costs if measured. Previous strict-path
   campaigns do not measure this consumer. Root updates issue/PR checkpoints and
   keeps unsupported recovery/platform/scale gates open.

## Focused end-to-end acceptance gates

Use existing fixtures and small payloads; share direct/relay cases where possible.
Do not use 30-second sleeps to prove deadline boundaries.

| Gate | Required observation |
| --- | --- |
| Real public embedding | Invoke the public sender with a shared broker and actual receiver, receive a fresh opaque signal per Call/Status probe, request recovery through an embedding-owned controller, complete exact transfer; expired signals return Disconnected and cannot affect the next Call |
| Silence and slow peer | Several real Idle outcomes without a command transmit zero additional frames; strict call has no Idle path |
| Missing Call receipt, receiver completed | Deliver Reply and ACK, withhold Call receipt; one explicit retry gets only actual Duplicate receipt, exact payload returns once, cache stays absent |
| Missing Reply while receiver waits | Deliver initial Call receipt, withhold Reply; exact Call retry gets Duplicate plus exact cached Reply; caller ACK releases cache; object executes once |
| Missing receipt plus missing ACK | Withhold initial Call receipt and first Reply-ACK; explicit retry while both sides incomplete gets cached Reply and fresh actual receipts, finishes once |
| Leftover earlier controls | Replay completion leaves old Reply/ACK or an old Call ahead of the next legitimate Call; preamble demuxes control-only work without new Handled, next exact Call succeeds |
| Command and resource bounds | Capacity one cannot queue a burst; seven retries plus original exhaust eight invocations, ninth refuses before I/O; sixteen frames cannot reset through Idle/preamble/retry |
| FIFO wait with queued retry | All current receipt/Reply/ACK facts exist but predecessor blocks canonical delivery; Idle polls completion first, retains queued command and sends zero retries; predecessor progress completes without a new current frame |
| Historical identity without cache | Canonical known identity plus None does not imply application completed; actual Duplicate receipt cannot complete a caller with missing Reply/ACK facts |
| Inactive admission and final event | Fresh ordinary Call uses admit-first; Duplicate requires local original-owner lookup; sixteenth fresh Call refuses before new state/credits, while known None may receive only actual Duplicate receipt; active different Call still gains no state |
| Original lifetime and authority | Deadline equality, Unix rollback/original mono expiry, policy generation change and different request refuse with no replacement identity/sequence; service Reply remains charged on refusal |
| Partial/uncertain transport | Mid-prefix timeout/truncation and invoked send failure fence retained current identity; a second call reads/sends no new frame; no reconnect/resume |
| Retry I/O cap | Late explicit retry uses the fixed original deadline through connect and blocked/partial write; a preexisting channel cannot gain a fresh parent timeout; actual counters and fence survive refusal |

Run the three loss scenarios over both direct and real duplex-relayed streams.
At least the missing-Call-receipt case must traverse the public sender entry and
Client wiring, not solely private Session fixtures. The strict public sender must
retain its no-channel/no-Idle branch. The live crate is a development witness;
public source reachability does not establish native guest opt-in execution.
Record exact request/Reply encodings and payload lengths/readbacks, actual bytes/
frames, tracker handles, object execution count, cache release and charged Reply
ownership. Fault injection must preserve failures and say which frame was withheld;
reliable TCP delivery is not itself a packet-loss benchmark. Assertions must not
claim that one successful ACK write proves remote receipt.

Use a bounded complete-frame fault gate adjacent to an endpoint, activated after
the existing handshake. Select direction/ordinal without plaintext inspection;
forward unchanged or deliberately withhold the one registered frame. Record
actual read/write bytes for each leg and the withheld frame length/hash. A frame
read by the gate was transmitted on its incoming leg; a withheld outgoing frame
contributes zero bytes there. Do not use a socket shim that swallows writes while
ordinary counters falsely imply delivery. Reuse the actual duplex relay through
a test-only boundary re-export rather than copying its implementation.

The invocation remains bounded by its original absolute Instant, signed Unix and
owner-monotonic deadline; 250 ms is polling granularity, never renewed lifetime.
Receiver preamble processing must appear in accumulated receiver work and failed
observations; exclude only defined idle wait from processing CPU/wall. Whole
lifecycle and actual recovery control traffic remain visible.

## Deferred boundaries

No recovery after caller completion when its final ACK is lost; that needs
bounded completed-tail ownership or a separately reviewed receiver-driven policy.
No persistent PendingReply, receiver resume, reconnect, automatic resend, wire
negotiation, cap-2,048 relaxation or larger host allowance. Last-cap reconciliation
can still fail at the existing receiver guard. Long-lived retention, fairness,
capacity/tail latency, native Windows and real-WAN qualification remain open.
The campaign module export is unconditional in lib.rs, while host/orchestrator
backends have their own cfgs. Qualify actual supported embedding build/socket
targets; do not infer a Windows live consumer or add a new bootstrap/wire mapping.

Initial full public-consumer TCP qualification is Unix-only. Windows new-consumer
coverage is **NOT_RUN / blocked** by unconditional hardware-SHA assembly features
and non-Unix private-directory Unsupported; adapter's existing 92 checks do not
exercise the new entry. Track compile-profile/storage portability separately in
[#46](https://github.com/R4-Laboratories/SpiralCore/issues/46). No Cargo/workflow
portability edits belong to this first reconciliation slice. Preserve all existing
build-call feature closures in any later change: static guests already use
`--no-default-features` while dependency features still enable hardware SHA, so
moving acceleration into defaults alone is not preservation. Qualify any explicit
feature/cfg solution and its identities before changing platform claims. A future
seven-row partial Status-probe fixture must be labelled partial, without claiming
full publication or native Windows MVM. Do not add dead-code allowances.
