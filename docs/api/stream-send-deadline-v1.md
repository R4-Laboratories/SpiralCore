# Fixed-deadline stream send view — additive Rust contract v1

Date: 2026-11-03. Proposed narrow prerequisite for lead review before RED/code.
Authority: [session reconciliation](session-reconciliation-v1.md). Expose the
existing private fixed-deadline view in adapter `src/live.rs`; canonical wire,
authorization, clocks and default transport behavior remain authoritative.

## Surface and ownership

In the existing `impl<S: SocketIo + 'static> StreamTransport<S>`:

```rust
pub fn before(&self, deadline: Instant) -> impl PeerTransport + use<S>;
```

The hidden return value remains private `DeadlineTransport<S>`. Rust 2024 with
the pinned compiler supports precise return-position opaque capture: `use<S>`
captures its socket type, not the temporary `&self` borrow. The value owns the
existing Arc clone and route; no mutex guard or borrowed service permit escapes.
It may outlive the original handle, while sharing the same ready/closed fence,
socket, counters and explicit close authority. No public concrete transport type,
trait change, error variant, socket clone, queue or new task is introduced.

Creation is pure: no lock, I/O, authorization, clock-accounting mutation or heap
allocation; it only increments the existing Arc reference. Fix the effective deadline
once to `min(deadline, creation_instant + self.timeout)`; timeout is already bounded
by MAX_OPERATION (30 s). Never refresh it at connect, channel creation, mutex wait,
prefix/payload write, interruption or partial progress. Repeated use of the view
retains that same deadline. A supplied past Instant creates an already expired view;
creation itself does not reject or fence. Creating another view cannot prove a
consumer retained its original lifetime: that remains the consumer's obligation.

## Send authority, errors and framing

Reuse existing `DeadlineTransport::connect` and `SocketChannel::try_send` unchanged.
The view is only a time bound, never admission: existing authorized route/frame,
same route ID, channel limits, ready/closed state, canonical capability/policy/
generation checks in the producer, framing and cancellation all remain required.
Both connect's mutex acquisition and the returned channel's later mutex/writes
use the fixed view deadline, including when the channel is obtained earlier and
used after expiry. No default timeout is restored in a channel.

Preserve current precedence/mapping: wrong route is UnknownRoute before locking;
size/limit checks stay at their existing phases. Existing cancellation/deadline
errors map through `transport_error` (deadline to Partitioned); no string matching
or new error type. An acquired mutex observes expiry/cancellation and fences;
contended-lock failure without a guard cannot promise physical closure. Owned
write failure preserves the shared logical fence, existing framing shutdown path
and partial byte counters. A fence prevents later authorized sends on any view.
Do not strengthen existing lock shutdown guarantees or change the Darwin fallback.
Frame count increments only on complete successful framing as before.

The existing `PeerTransport for StreamTransport::connect` still creates its
ordinary per-operation view from `now + timeout`. Making `before` public must not
change that default, strict receive, Idle, handshake or any normal-path producer.
Existing raw framing helpers and public LiveError variants remain unchanged.

## Reconciliation use and focused gates

The new explicit retry must pass `&stream.before(original_exchange_deadline)` to
`LocalMailboxService::retry_call_prepared`. Keep fresh canonical time/Instant checks
and the pre-I/O invocation burn; they do not replace this I/O bound. The service
API stays unchanged. Bind every canonical exchange-owned write, including strict
canonical callers, to its already retained original absolute deadline:

| Canonical write | Required view deadline |
| --- | --- |
| Caller initial Call, explicit Call retry, Reply-ACK | Original caller exchange Instant |
| Receiver initial Call receipt, initial Reply, duplicate Call receipt, cached Reply resend | Receiver's fixed ExchangeDeadline.instant() |
| Receiver historical preamble receipt | Fixed preamble invocation Instant, further capped by that authenticated old request's signed remaining lifetime |

Receipt permits and binding send APIs already accept PeerTransport, so the same
view adds no rights or new authorization mechanism. Create/reuse views as needed,
always passing the same original exchange deadline; never substitute now plus a
new interval. The canonical old-request lookup still enforces its original owner
deadline. This migration tightens canonical write expiry; it changes no wire,
codec, normal frame count or strict/opt-in choice. Adapter default connect and
the reusable API #64 service implementation remain unchanged. This contract does
not preempt arbitrary application/controller work or guarantee scheduler timing;
I/O must fail on observed expiry under existing bounded SocketIo semantics.

Historical expiry needs no new service getter: pinned canonical MVM `ae12e9d`,
`crates/mvm-core/src/mailbox.rs:545`, rejects inner signed expiry unequal to outer
routing expiry after decryption/signature verification. Adapter binding.verify
uses that canonical opener. Only after successful retry_reply_prepared lookup
of the same bytes may decoded outer expiry cap ExchangeDeadline::new using the
fixed preamble Instant and fresh owner time. This derived cap is not the old
retained owner-monotonic deadline: re-run the original canonical lookup with
fresh time immediately before actual historical receipt send, especially after
wall rollback or reservation work. None still means known identity/no cache,
not application completion; neither a parsed outer field nor a new local cap
replaces that lookup's authorization and original owner-lifetime checks.

Required tests in `crates/spiralcore-mvm-adapter/tests/service_tcp.rs`: external
missing-API RED, expired view before connect, channel acquired before deadline then
used after, and canonical authorized blocked-write/partial-progress failure using
the short original cap rather than the parent's longer timeout. Preserve actual
byte/frame counters and shared fence, route/limit/cancel errors, and successful
healthy send. Repeat connections/views using the same original Instant and prove
they never refresh it. Preserve route/cancel precedence: wrong route alone does
not gain a new fencing side effect; cancellation/expiry while owned keeps the
existing fence, and unowned lock failure retains its documented limitation.
Use bounded backpressure, not a 30-second test sleep or raw-authority
bypass. Integration in the live reconciliation test must delay an explicit retry
until little original lifetime remains, force write backpressure and show refusal
under that original cap. Include a blocked canonical ACK/receiver replay write
under its own original exchange deadline. A pre-invocation expiry-only test is insufficient.
Independent exact-source review and affected release gates follow RED/GREEN.

Portable fixture phase clarification: obtain the ordinary canonical authorized
frame and route before starting a short transport-only deadline. Then connect
and exercise the actual channel writer with exact partial-byte and fencing
assertions. Software/debug cryptography may consume a short deadline before I/O;
that correct refusal does not qualify partial-write progress. Full service-path
deadline qualification remains in the live Call/ACK/Reply writer matrix, where
the original signed lifetime still includes authorization work. This changes no
product deadline or authorization check and admits no arbitrary raw frame.
