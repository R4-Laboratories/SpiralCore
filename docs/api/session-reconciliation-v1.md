# Incomplete-session reconciliation — Rust embedding contract v1

Date: 2026-11-03. Lead-approved contract before RED/code, based on main
`e984d2d64cdf0483d50d6d59319276c85e7df644`; not an implemented recovery claim.
Depends on [registered Call retry](registered-call-retry-v1.md),
[receive idle](stream-receive-idle-v1.md) and the
[fixed-deadline send view](stream-send-deadline-v1.md), plus the
[session state authority](../specs/2026-11-03-persistent-session-state.md).
Delivery remains partial C8 under [#43](https://github.com/R4-Laboratories/SpiralCore/issues/43)
and [#49](https://github.com/R4-Laboratories/SpiralCore/issues/49).

## Choice and invocation

Add this public development-witness embedding API in
`spiralcore_mvm_live::campaign`, reusing existing Bootstrap/BrokerContext and wire:

```rust
pub struct ReconciliationSignal { /* private SyncSender<()> */ }
impl ReconciliationSignal {
    pub fn request_retry(&self)
        -> std::result::Result<(), std::sync::mpsc::TrySendError<()>>;
}

pub fn sender_with_broker_and_reconciliation<S: SocketIo + 'static>(
    bootstrap: &Bootstrap,
    broker: &BrokerContext,
    socket: S,
    role: HandshakeRole,
    out: &mut impl Write,
    on_call: &mut impl FnMut(ReconciliationSignal) -> Result<()>,
) -> Result<Value>;
```

The real callsite is `campaign::Client::call`: in the new public entry's opt-in
branch, create a fresh bounded pair for each Call, including the Status probe;
deliver only its signal to `on_call` outside broker dispatch, then move requests
into the canonical method below. The callback receives no request payload,
bootstrap material, authorization, Session or socket. It hands the signal to its
embedding-owned controller and returns promptly. Session creates no controller
thread. Callback errors propagate; the unused requests endpoint is dropped.
Recheck original lifetime after any embedding work before authorized I/O.

```rust
// Internal only: private fields, no arbitrary-channel conversions.
pub(super) struct RetryRequests { /* private Receiver<()> */ }
fn bounded_requests() -> (ReconciliationSignal, RetryRequests); // sync_channel(1)

// Inside the private canonical Session implementation:
pub(super) fn call_with_reconciliation<S: SocketIo + 'static>(
    &mut self,
    request: &Frame,
    stream: &StreamTransport<S>,
    cancel: &CancellationToken,
    retry_requests: RetryRequests,
) -> Result<(Context, Frame)>;
```

The sole private factory creates `sync_channel::<()>(1)` and opaque wrappers per
invocation. Their fields stay private; expose neither a raw-channel constructor
nor receiver extraction/conversion. `RetryRequests` is not Clone and moves into
the call. The controller retains ReconciliationSignal and invokes its nonblocking
`request_retry()` while `call` owns `&mut Session`, without borrowing Session or
the socket. Full means one request is already queued and adds no retry;
Disconnected means this invocation no longer accepts requests. The receiver
handles sender disconnection as no future requests. Dropping the invocation's
receiver prevents stale signals affecting the next identity. This makes capacity
one enforceable by construction rather than trusting an arbitrary Receiver.
Session creates no helper thread. The channel contains no payload or authority.

Keep existing sender/sender_with_broker, Client strict branch and `Session::call`
signatures and strict blocking receive unchanged. The strict branch creates no
channel, invokes no callback and performs no Idle polling; do not enable them to
silence dead-code warnings. The new public embedding entry calls the private
opt-in source path in normal library builds, without dead-code allowances or
test-only gates. Normal guest entry points remain strict until an independently
qualified caller explicitly chooses the new Rust API. No guest port, privileged
host signal, bootstrap/profile serialization or transport version is added.

`lib.rs` exports campaign without a host-backend cfg; host/orchestrator exports
are separately limited to macOS ARM or Linux with linux-host. This does not prove
Windows support for the live crate/SocketIo stack. Qualify the embedding entry on
its supported build/socket targets separately; no native guest opt-in execution,
Windows live consumer or new platform/backend acceptance is claimed by this design.

Initial executed consumer qualification targets the full Unix TCP embedding
fixture. Windows new-consumer qualification is **NOT_RUN / blocked**: the current
live manifest unconditionally enables hardware SHA/sha2-asm, and the production
receiver's private object directory rejects non-Unix platforms as Unsupported.
The existing 92 adapter checks per native-client row do not cover this consumer.
Compile-profile and storage portability remain separate [#46](https://github.com/R4-Laboratories/SpiralCore/issues/46)
gates; this slice changes neither Cargo features nor workflows to bypass them.
Any future profile change must preserve actual existing build-call feature
closures, including static guests built with `--no-default-features`, which
currently still receive dependency-enabled hardware SHA. Moving that feature
only into defaults would silently disable acceleration for those guest builds.
Verify intended closures and source/binary identities before crediting a change.
A later seven-row ephemeral Status-probe test may qualify only its explicitly
labelled partial consumer exchange, not full object publication or Windows MVM.

| Option | Tradeoff | Decision |
| --- | --- | --- |
| Per-invocation explicit signal | Small addition to synchronous ownership; no unsolicited retry bytes | Chosen |
| Begin/drive/reconcile step API | Single-thread application control; requires retaining more local phase/context state | Defer |
| Retry automatically after Idle | Simple policy, but duplicates traffic on a slow reliable stream | Defer |

## Caller state machine and bounds

Use `receive_or_idle(cancel, original_deadline, 250 ms)` only in the opt-in path.
An Idle releases the transport mutex and is reader silence, not evidence of loss.
On Idle, check original lifetime with fresh canonical time, then run the ordinary
completion gate BEFORE inspecting the request queue. If current receipt, Reply
observation and successful ACK-write facts are present, canonical `poll_call`
may extract the payload for completion. If it returns Pending, wait for FIFO
predecessors without dequeuing a request or resending the current Call. Keep the
queued request and recheck completion on later Idle/Frame events against the same
deadline; terminal/consumed statuses retain existing typed errors. This permitted
completion extraction differs from a destructive retry-eligibility probe.
Only when current completion facts are missing poll the request wrapper without
waiting; Empty/disconnected emits zero bytes. A Frame follows existing authenticated
demultiplexing and consumes one incoming event. Idle and local commands do not.

When still incomplete and one request is dequeued:

1. Under short broker dispatch, check the original lifetime, expiry maintenance,
   current handles and that fewer than eight Call-send invocations have begun.
   Re-register only the retained exact encoded request with the existing tracker;
   require the returned SendHandle equals the retained handle. Preserve receipt,
   Reply-observed, ACK and FIFO facts; never use destructive `poll_call` to test
   retry eligibility. No new ordering position, nonce, envelope or Call registration.
2. End dispatch. Burn one Call-send invocation immediately before entering
   `service.retry_call_prepared` with the retained prepared request and original
   CallHandle, passing `&stream.before(original_deadline)` as its PeerTransport.
   This fixed view must bound connect, mutex waits and the channel's entire write;
   passing the unbounded-parent StreamTransport would start another timeout and
   violates this retry contract even with fresh pre-invocation checks.
   The original send burns the first of eight. Invoked pre-connect
   refusals also consume that invocation; do not refund on errors or short writes.
   The adapter rechecks generation, semantic commitment, authorization, original
   mono/Unix expiry and cancellation through its existing policy-locked send path.
   No broker lease spans I/O and no new outer policy lock is added.
   All canonical exchange-owned writes use the same bounded-view mechanism:
   caller initial Call and Reply-ACK use its original Instant; receiver initial
   receipt/Reply and duplicate receipt/cached Reply use ExchangeDeadline.instant().
   Preamble historical receipts use its fixed invocation deadline capped by that
   old request's signed remaining lifetime. Do not renew any bound after a send.
   The view adds no authority to their shared PeerTransport interface; default
   adapter connect and reusable service APIs retain existing behavior.
3. Continue receiving against the same fixed absolute Instant and original owner
   mono/Unix expiry. Preserve the cumulative sixteen-frame bound and eight-send
   bound across all local requests. Completion still requires eligible current
   receipt, current authenticated Reply, successful Reply-ACK write and final
   canonical FIFO payload extraction. Duplicate never invents an acceptance level.

Use the same finite PendingCall, exact request/prepared/encoded buffers and at
most one bounded authenticated Reply wire. Retain charged Reply plaintext inside
the canonical service until final completion. No borrowed permit is persisted.
The invocation's absolute Instant cannot restart on Idle, control, replay or
retry; a new method call cannot resume a previously invoked unresolved send.
Original fixed owner-monotonic and signed Unix bounds remain authoritative.

`EventLimit` and `SendLimit` are typed terminal failures; service refusal, clock,
expiry and authority failures retain their existing typed errors. Transport/
partial-framing error or uncertain write fences the Session and retains original
identity; subsequent calls consume no new frame and allocate no replacement Call.
Do not treat timeout/cancel as Idle or attempt reconnect/resume. Idle does not
strengthen transport closure guarantees for failures without an acquired mutex.

## Receiver preamble and active replay

Extend the receiver's bounded preamble before a fresh application Call. Preserve
the top-level 2,048-call guard and no-resume latch. Set the latch before reading;
start one absolute deadline before the first preamble read. Count every complete
incoming preamble/control/Call frame toward the same sixteen-event invocation
limit, including the eventual new Call and its ACK/replay events. Do not reset
the bound or deadline after handling an old event. Admit no fresh Call after the
event allowance is exhausted. Signed admission caps the fixed deadline further.

- For a provisioned control key, canonical tracker observation authenticates the
  earlier receipt. Retained valid older controls contribute no current facts and
  produce no receipt-of-receipt. Unknown/invalid/expired/conflicting controls keep
  canonical errors; key ID alone grants no authority.
- In the inactive preamble, check its fixed deadline before any admission. With
  fewer than sixteen events consumed after this normal frame (room for a later
  ACK), call `returns.admit` first. Only AcceptedEphemeral enters the fresh proof/
  application path. This uses one canonical verification for an ordinary fresh
  Call. The private no-resume latch excludes an unresolved active exchange; do
  not use this ordering while awaiting a current Reply's receipt.
- For actual Duplicate admission, require `retry_reply_prepared` authenticate
  retained original ownership/lifetime and return `Ok(None)` before receipt
  reservation. The duplicate admission creates no fresh application entry. None
  proves known identity/no cache, not application completion. Send only its actual
  canonical Duplicate receipt; never receive/object work, create Reply/cache,
  increment application calls or return another Handled. Re-run the lookup with
  fresh time after reservation immediately before bounded I/O, retaining fixed
  preamble/signed-expiry caps and old owner-monotonic checks. Cached Some or any
  lookup error fences without sending; other admission outcomes refuse.
- At the sixteenth normal frame, lookup must precede admission: fresh UnknownHandle
  returns typed EventLimit before new application state/control reservation can
  be allocated with no room for its ACK. Only authenticated known `Ok(None)` may
  proceed to actual Duplicate admission/receipt-only handling, with the same fresh
  original-owner checks. Do not reset event/deadline bounds; exhaustion fences.
- For a fresh accepted Call, retain its admitted proof and fixed original
  Instant/Unix/owner-monotonic deadline. Execute the object once and use the
  existing active receiver loop: exact current cached Reply comparison before
  admitting a repeated Call and obtaining its actual Duplicate receipt; fresh both-binding checks,
  same tracker handle, exact cached Reply, at most eight total Reply invocations.
  Only the matching authenticated eligible Reply receipt releases the cache.

Active cached replay sends the exact cached Reply **before** its actual Duplicate
Call receipt. After that blocking write, sample fresh broker time and recheck both
bindings, original lifetime, exact cached identity and retained tracker handle
before reserving/sending the receipt outside dispatch under the same fixed deadline.
Burn the Reply invocation before its write; the eight-invocation bound is unchanged.
The caller writes its ordinary ACK for the replay before reading a receipt that
can close a missing-receipt Call; its ACK precedes any next Call on the same stream.
With an original Call receipt already retained, completion may leave the trailing
Duplicate control for existing authenticated demultiplexing. Qualify both C8 peers
together; no wire/version, proactive extra frame, ACK kind, timer or counter is added.
Inactive cache-None handling remains actual Duplicate-receipt-only; ACK write alone
still does not prove remote observation or general recovery after caller completion.

Old-Call lookup enforces that old identity's original canonical lifetime even
before fresh application admission. Control reservations and all failed sends
remain subject to existing signed rates/budgets. Clear the receiver latch only
after its current application completion; errors preserve canonical state/fence.
Preamble-only traffic never fabricates application completion.
The active prelookup prevents a different Call gaining state after active credit
has been released; it is mandatory even though the inactive fresh path avoids it.
Its Duplicate receipt cannot complete the caller without that caller's own
retained authenticated Reply and successful ACK-write facts plus FIFO extraction.

## Accounting and explicit limits

Record actual stream frames/bytes in both directions, Call/Reply send invocations,
explicit commands consumed, Idle outcomes, retransmitted Call/Reply frames and
preamble controls/duplicate receipts. Use checked counters. Fixed four-frame
normal-path expectations must not be reported as actual recovery traffic.
Include preamble authentication/reservation/send processing in receiver work CPU
and wall meters, accumulating it into the next Handled or an explicit failure/
unfinished observation. Do not drop it when the application meter starts. Preserve
whole-invocation/lifecycle time separately; idle wait is excluded only from the
already defined receiver processing interval, not end-to-end time. Report failures
and partial counters; do not infer energy or a performance improvement.

The JSON session report is explicitly `session_report: "canonical-session-report/v2"`.
Rename static expectations to `normal_frames_per_call` and
`normal_encrypted_destination_receipts_per_call`; retain their normal values, never
label them actual traffic. Actual bytes/frames remain the transport observations.
Add checked-u64 `recovery` counters with these event meanings:

| Field | Increment |
| --- | --- |
| idle_outcomes | Each actual opt-in ReceiveOutcome::Idle, including one followed by refusal |
| commands_consumed | A queued explicit command is actually dequeued, even if preparation later fails |
| call_retry_invocations | Invocation burned immediately before the authoritative retry call, including pre-connect/write failure |
| historical_control_events | Authenticated earlier receipt controls handled in the inactive preamble; normal historical Calls use the receipt counters below |
| historical_receipt_invocations | Immediately before invoking an actual historical-Call Duplicate receipt send |
| historical_receipts_sent | That send returns success; this is not proof of remote observation |
| cached_reply_retry_invocations | Immediately before invoking an exact cached Reply resend |
| reply_send_invocations | All receiver Reply invocations, initial plus cached; cached retries are a subset |
| receiver_failed_invocations | Each receiver handle invocation returning Err, including cancellation after preamble-only work |

Add cumulative `receiver_processing { cpu_us, wall_us, segments }`: begin an owned
Session phase only after a complete incoming frame, finish before the next receive
wait or success, and finish any outstanding phase on Err in the handle wrapper.
Use one Option<(ProcessTime, Instant)>, not an Arc/global clock/thread. Every phase
includes its authentication, reservation, object and outbound-send work as applicable,
including preamble and failed work; socket receive waits/partial reads are excluded
from this processing domain but remain lifecycle/transport observations. Checked
duration conversion/addition refuses overflow rather than truncating/resetting.
ProcessTime measures process CPU during those intervals, not exclusive receiver-thread CPU.
Per-Handled meters include preceding preamble work when an application follows;
they overlap the cumulative domain and must not be added to it. Snapshot only after
phase closure. The lost-Call-receipt fixture's second-handle Cancelled cleanup must
retain one historical receipt invocation/success, one failed invocation and three
cumulative segments across both handles, despite no second Handled. These are
processing observations, not lifecycle, energy or comparative CPU performance claims.

This slice covers an incomplete caller missing its Call receipt or Reply, and
lost Reply-ACK while the Call receipt is also missing. It does not cover ACK loss
after caller completion, general receiver resume, closed-stream recovery, automatic
timers, longer retention, cap relaxation or full C8. The existing cap may prevent
reconciliation after the receiver's final permitted Call; a future clean boundary
must resolve that separately. No new wire/version negotiation or host budget is
introduced. Product integration/measurement requires the plan's independent gates.
