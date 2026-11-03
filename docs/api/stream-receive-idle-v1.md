# Zero-prefix receive idle — additive Rust contract v1

Date: 2026-11-03. Architect-approved implementation contract; independent source
review and executed gates remain required. Baseline main `62692428cd1e9effc51c7930f7b1f34cc1519277`,
adapter `src/live.rs` SHA-256
`53c29a1724776c2cd60a004b59f46cd16a28600eca9e9345f55f03725b39e500`.
Authority: the [C8 session contract](../specs/2026-11-03-persistent-session-state.md).

## Decision and public surface

Add a local receive outcome and method in `spiralcore_mvm_adapter::live`:

```rust
pub enum ReceiveOutcome {
    Frame(Vec<u8>),
    Idle,
}

impl<S: SocketIo + 'static> StreamTransport<S> {
    pub fn receive_or_idle(
        &self,
        cancel: &CancellationToken,
        frame_deadline: Instant,
        idle_window: Duration,
    ) -> LiveResult<ReceiveOutcome>;
}
```

This is not a REST schema, serialization, wire version, codec, authentication,
receipt or admission change. Preserve the existing `receive` and raw framing
helpers' strict behavior. Keep the existing `LiveError` enum unchanged so callers
with exhaustive matches retain compatibility. No raw public idle helper is needed.

Splitting socket ownership/reader-writer locks adds a larger lifecycle and counter
change. Shortening today's strict receive deadline closes the stream, including
partial framing. The chosen seam yields only at an unconsumed prefix boundary,
using the same serialized transport and existing cancellation-aware sliced I/O.

## Bounds, validation and lock precedence

Validate `1 ms <= idle_window <= 1 s` before locking or I/O. An invalid window
returns existing `LiveError::Io` with `io::ErrorKind::InvalidInput`; it changes no
protocol state, counters or socket state and does not fence the transport. Match
the typed error kind, never an error string. No new error wrapper/variant is added.
This pure argument rejection precedes cancellation/deadline/state inspection.

For valid arguments, fix one effective frame deadline at method entry:
`min(frame_deadline, entry + self.timeout, entry + 30 s)`. It never refreshes after
lock wait, first byte, prefix completion or payload progress. Repeated polls must
receive the consumer's same original absolute exchange deadline; this method
cannot extend signed authorization or the consumer's original monotonic lifetime.

Acquire the existing state mutex using that frame deadline, not the idle window.
Retain current `lock` behavior: cancellation wins its deadline check; an acquired
guard is fenced/shut down when that check fails. A contended-lock timeout/cancel
returns without owning the guard and cannot promise physical socket closure.
Poison remains fatal. None of these cases is Idle. After successful acquisition,
closed or unready state returns Closed without a receive, as the strict API does.

Only after lock and ready/closed validation start the positive idle window. Lock
waiting is never evidence of reader silence. Check cancellation and frame expiry
before any Idle decision; frame expiry wins even if both deadlines are reached.
If the remaining frame lifetime is shorter than the requested idle window, no
Idle is promised: the original frame deadline may fail first. No queue, mutex
lease, socket clone or background timer is introduced.

## Prefix and frame state machine

Start with a four-byte stack prefix and local consumed-prefix offset zero.
Before any prefix byte, each socket timeout is bounded by the existing I/O slice,
remaining idle window and remaining effective frame lifetime. Early TimedOut,
WouldBlock or Interrupted results do not produce Idle or restart either timer.
When the full positive idle window elapses with zero prefix bytes consumed,
return Idle only after cancellation/frame-deadline rechecks.

A positive read irrevocably disables Idle for that invocation. Even if that read
returns near/after the idle boundary, retain its bytes and complete the remaining
prefix and payload against the original effective frame deadline. Never discard
a partial prefix, return Idle for it, restart parsing, or refresh a per-byte or
payload deadline. If absolute expiry/cancellation is observed, fail and fence.
The idle window is not a deadline for completing a frame whose prefix has begun.

Validate the unchanged nonzero length and `MAX_ENVELOPE_BYTES` before allocating
the payload. Allocate no heap buffer for Idle; retain no partial frame between
invocations. Return Frame only after the full bounded payload and final ordinary
cancellation/deadline check. Increment byte counters for actual reads and frame
count only for a successfully completed frame. Preserve partial counters on error.

Observed EOF before a prefix remains fatal UnexpectedEof for the owning endpoint,
not Idle or a relay half-close. Partial-prefix/payload EOF, malformed length,
counter failure, read/timeout-configuration failure and partial-frame deadline are
fatal as usual. Once reader state is owned, set its closed fence and use the
existing shutdown fallback on fatal framing/read failure; preserve the error.

Idle leaves ready/closed state and counters unchanged and releases the mutex
before returning. Subsequent authorized send/receive can acquire it; normal
operations set their own timeouts. Frame bytes remain unauthenticated ciphertext
until existing canonical consumers verify them. Idle means zero bytes consumed,
not proof that the kernel queue is empty, a peer lost a message or retry is safe.
A racing frame remains queued for the next operation. There is no fairness or
concurrent-write guarantee while an actual partial frame is being completed.

## Focused contract gates before consumer retry

- Zero, sub-millisecond and greater-than-one-second windows yield typed local
  InvalidInput without closing, reading, changing counters or preventing a later
  handshake/valid operation. Boundaries 1 ms and 1 s are valid.
- Positive zero-prefix silence yields Idle, unchanged counters and an open ready
  stream; a subsequent authorized send and later exact frame receive succeed.
- Lock contention past the idle interval never yields Idle. Once acquired, a full
  idle window begins; lock timeout/cancellation retains existing error semantics.
- Closed/unready, cancellation and expired absolute deadline never become Idle;
  absolute expiry shorter than/equal to the idle window fails and fences when
  reader state is owned. An already observed EOF remains fatal.
- Deliver one or more prefix bytes before idle expiry and the rest after it but
  before the fixed frame deadline: return the exact Frame, never Idle. Repeat for
  a split prefix/body and verify no timer refresh or extra frame count.
- Stall after a partial prefix or body to absolute expiry: fatal closure with
  exact partial counters; cancellation, bad lengths and truncated EOF stay fatal.
- Boundary data cannot be discarded as Idle; early sliced timeouts do not shorten
  the positive window. Strict receive/EOF, Darwin shutdown, canonical TCP exchange
  and existing byte/frame counters remain compatible.

No automatic retry, reconnect/resume, idle-maintenance daemon, larger pipeline,
grant/TTL/retention increase, new acceptance fact, WAN or full C8 qualification is
enabled by this API. Its consumer needs a separately reviewed bounded retry rule.
