# Persistent session pending identity and authenticated events

Date: 2026-11-03. Design contract for plan C7/C8, against `cc94f31`.
This is a source-derived implementation contract, not executed qualification.
The parent [service specification](2026-11-03-persistent-mvm-service.md) and
[plan C7–C10](../plans/2026-11-03-persistent-mvm-service.md) retain authority.
Keep pipeline one, canonical wire/crypto/proof APIs, existing finite ceilings,
production retention and the 2,048-Call witness guard. C9/C10 remain separate.

## Existing API facts

- `OrderingStream::next_for_binding` consumes a sequence only on success. Keep
  that successful result: another invocation allocates a different identity.
- `LocalMailboxService::register_call` returns the original handle for an
  identical pending Call under the same binding/generation. Terminal reuse
  conflicts. `cancel` terminalizes that Call; it is not registration rollback.
- `ReceiptTracker::register_send` is idempotent for the same delivery ID and
  ciphertext commitment. Same delivery ID with different ciphertext conflicts.
  Changed immutable header/payload or binding/generation/key also fails closed.
  At most eight distinct delivery/ciphertext attempt bindings are retained.
- `take_receipt` consumes an available observation; subsequent duplicate frames
  need not produce another available receipt. `poll_call` similarly consumes
  the payload once, leaving `Replied(None)` on later polls.
- `observe_receipt` authenticates the control carrier, destination signature,
  original identity and registered attempt. It can observe an earlier retained
  operation. `observe_application_reply` returns its authenticated `SendHandle`.
- `MailboxBinding::send_prepared` returns `ServiceError`; transport failures are
  collapsed by `map_transport`, without a reliable written-byte/phase result.
  An error after invoking it cannot generally establish zero transmission.

## Private Rust seam

Private `campaign::session` state is owned with the existing service/tracker/
stream. Separate begin/resume: a different submitted request while pending is
Busy or identity conflict, never a substitute payload. No new adapter authority.

```rust
// Conceptual private types; concrete envelope/context types stay canonical.
struct PendingCall {
    options: MessageOptions,
    prepared: Option<PreparedMailboxEnvelope>,
    encoded: Option<Vec<u8>>,
    call: Option<CallHandle>, send: Option<SendHandle>,
    send_started: bool, sends: u8, events: u16,
    // Fixed deadline/context; bounded Reply bytes, receipt facts, ack state.
}
enum SessionState { Idle, Pending(PendingCall), Fenced }
enum Progress { WaitingForCapacity, WaitingForPeer, Complete }
```

Typed private errors preserve `ServiceError`/transport errors and explicit
`Busy`, `WitnessLimit`, `EventLimit`, `SendUncertain`, `StreamFenced` dispositions.
Distinguish local/remote refusal, timeout/cancel and uncertain transmission;
never parse strings or classify every failure as retryable capacity refusal.

## Registration and retry transaction

1. Before consuming a sequence, validate/encode the bounded application payload,
   check idle state and the witness guard. Allocate options once, set `reply_to`
   once and preserve every inner field: message/correlation/causation IDs,
   scope/sequence, issue/expiry, nonce, content type and route. Retain the pending
   identity immediately; any failure after sequence allocation fences or keeps
   that identity. Preparing/encoding failure cannot silently return to Idle.
2. Prepare once and retain its exact encoded envelope and prepared object.
   Under a short broker phase with fresh owner time, register the Call first;
   save its handle immediately. Register its source tracker second and save
   that handle. Never send without both handles. A tracker `Full` after Call
   success retains the Call reservation and retries only the missing tracker
   registration. A first-registration `Full` retains options/prepared bytes.
3. For known pre-send `Full`/`Backpressured`, return bounded waiting/refusal while
   preserving pending state. Expire service/tracker with fresh owner time before
   retrying. No busy loop, new sequence, scope, correlation, deadline or TTL.
   Conflicting registration/revocation/clock/state failures fence this stream.
4. Set `send_started` before invoking transport outside dispatch. Retain both
   handles and exact envelope across failure; no cancellation as rollback.
   First C7 retransmission reuses the identical prepared envelope, delivery ID
   and ciphertext, with at most eight send invocations per logical request.
   Each invocation revalidates current authorization/time. Do not reseal it.
   A later fresh-delivery retry needs another bounded `register_send` before
   transport; all inner options remain identical. That is not required by C7.
5. Unknown send outcome blocks new Calls; retry/reconcile only this identity
   within its original authorization/deadline. A socket with partial framing is
   not reused as though aligned. No reconnect/resume claim is introduced here.
   Cancellation/expiry may release active local Call credit through the existing
   API, but retains replay/receipt identities and never proves remote rollback.
6. Return to Idle only after the explicit successful completion policy below.
   Refusal/expiry of an allocated, not-known-contiguously-delivered sequence
   fences the stream. Terminal local state does not fill a receiver's unknown
   gap. Recovery needs a separately authorized owner/epoch procedure.

## Bounded authenticated event processing

Replace `for _ in 0..2` with one-reader processing bounded by the original
deadline, cancellation and a fixed 16-frame total allowance per pending Call;
every frame, including duplicates, consumes allowance. Retries do not reset it.
Hold at most one decoded incoming frame and one bounded Reply payload. Read,
send and wait outside dispatch; authenticate/mutate under fresh broker time.

Encryption key ID selects verification, not authentication. Unknown keys,
malformed frames or failed verification fail; no receipt-of-receipt is emitted.

For control frames, call `observe_receipt`, then `take_receipt` for the **current**
send handle. `Pending` after an earlier/duplicate receipt is normal waiting.
Retain already observed current receipt facts when no new observation exists.
Signed `Full`/`Refused`/`Unavailable` is a remote outcome, not local registration
rollback. `Duplicate` alone does not prove ephemeral/durable acceptance level;
keep `ReceiptFacts::accepted` semantics unchanged.

For normal frames, retain canonical authenticated Reply admission and receipt
return through `ReceiptReturnConfig::admit`. `AcceptedEphemeral`, `Duplicate`
and `Late` are distinct dispositions. Correlate using the authenticated handle
returned by `observe_application_reply`; only the current handle can contribute
its payload. Poll only the retained current Call handle and keep the first
`Replied(Some(payload))` result. `Replied(None)` never erases it. A valid earlier
Reply cannot satisfy the current Call. An expired earlier Reply may fail
verification; fail closed rather than interpreting its untrusted header.

Completion requires the current authenticated application Reply and current
signed destination observation (`AcceptedEphemeral`, `AcceptedDurable`, or
`Duplicate`), plus successful transmission of the Reply's required receipt.
Record receipt level separately: Duplicate plus Reply is not durable acceptance.
Reply-before-receipt and receipt-before-Reply both work. Loss of either required
observation ends as incomplete/uncertain, never an invented receipt or success.
Retain Reply payload and ack-needed/ack-uncertain state if receipt sending fails;
never consume/drop them into an error-only return. `PreparedReceipt<'a>` borrows
local return/binding state: do not store a self-referential permit in Session.
Retain bounded authenticated Reply bytes and recreate canonical receipt admission
under fresh time, preserving its actual disposition, or review an owning seam.

C8 covers both peers. While waiting for its Reply receipt, the destination must
authenticate a repeated Call, return its canonical duplicate receipt and use
`retry_reply_prepared` to resend the exact cached Reply, registering that cached
attempt before sending. Never reexecute object work or allocate Reply options.
No cached Reply means explicit incomplete/failure, never reconstructed success.
Control/duplicate events share that pending exchange's finite event/send bounds.
Only the matching authenticated Reply receipt may call `acknowledge_reply` and
release the cache. Replacing only the caller's two-frame loop is not C8.

## Focused executable gates

- Refuse Call admission, then tracker admission after Call success; repair
  capacity and assert one sequence/options/Call handle/Send handle, no send
  before both registrations, unchanged retry envelope bytes and no leak/reset.
- Mutate payload/inner options or reseal with the original delivery ID: assert
  canonical conflict; exact prepared replay remains idempotent and bounded.
- Receipt/Reply in either order; duplicate/earlier frames; consumed observation
  and payload polls; loss/deadline/cancel/event saturation; malformed/unprovisioned
  input and revoked binding. Assert current-only completion and finite state.
- Inject send failure and inspect retained pending identity/handles; attempted
  replacement Call is Busy/fenced. Unknown expired gap never advances sequence
  via a fresh scope, receiver reset or scope checkpoint eviction.
- Retransmit a Call while receiver awaits Reply receipt; exact cached Reply is
  resent, object execution stays one, lost/uncertain ack retains pending state.

Exact-source code/security review precedes merge. >2,048 Calls, retention-cycle
recovery, WAN and platform acceptance remain unestablished by this seam.

## Approved C7 generation-pinned send seam

Architect review approves this bounded public adapter addition before coding:

```rust
pub fn send_call_prepared(
    &self, binding: &MailboxBinding, call: &CallHandle,
    prepared: &PreparedMailboxEnvelope, transport: &impl PeerTransport,
    cancel: &CancellationToken, now: ServiceTime,
) -> Result<usize>;
```

The owning `LocalMailboxService` validates exact retained handle equality,
service/binding identity, Pending status, non-shutdown state, nondecreasing
monotonic time and both original monotonic deadline and header expiry. It then
uses the existing binding send path with a private expected generation and
semantic request commitment from that retained Call. After the existing single
canonical verification, compare both expectations under the existing binding
policy lock before transport connect/send. A different prepared request or
policy generation fails closed without writing; no extra verification, budget
reservation, clock mutation or alternate canonical authority is introduced.

Use this method outside broker dispatch. Its immutable service borrow spans
I/O, preventing local cancellation/shutdown mutation through that same service;
other sessions remain independent. Preserve existing policy-lock linearization
and existing unpinned `send_prepared` behavior; generation-only receipt sends
retain their current check without requiring a Call commitment. The caller
marks send-started first and retains pending state on every send error.
Post-tracker polling still reconciles registration before this final send gate.
Tests reject policy replacement after registration and a substituted prepared
request before any transport bytes, and retain the original handles/identity.
Successful send remains a transport fact, not peer acceptance or a guarantee
that authorization remains valid after blocking I/O. C8/C9 stay unfinished.

## Approved C8 caller Reply ownership seam

Defer payload consumption: this supersedes the earlier instruction to retain
the first `Replied(Some(payload))` in caller state before acknowledgment.
Canonical `admit`/`admit_for_control` already drain FIFO-ready Replies into the
original Call's `Replied(Some(payload))`, retaining the queue reservation as
`reply_budget`. Leave that payload and charge in `LocalMailboxService` until
completion; do not add an uncharged caller payload copy or a new authority API.

`PendingCall` retains current verified receipt disposition, cumulative 16-frame
count, current Reply-observed fact, one authenticated encoded Reply bounded by
`MAX_ENVELOPE_BYTES`, and acknowledgment Needed/InvokedUncertain/Sent state.
Advance current Reply state only when `observe_application_reply` returns the
current `SendHandle`. Preserve Duplicate's distinct acceptance meaning. The
extra retained ciphertext is bounded endpoint overhead, not a transport-budget
or service-payload charge. Prepared receipt/permit borrows remain stack-local.
Mark acknowledgment invoked before transport; failure retains original request
identity/wire, Reply wire and service-owned payload without implicit retry.

Only after current destination observation, current Reply observation and a
successful required Reply-receipt send, sample fresh broker time and require
the original `PendingCall::remaining(now) > 0`, then poll the original handle.
This explicit deadline gate is required: `poll_call` can consume an already
Replied payload after message expiry. Its binding/generation checks still apply.
Pending preserves waiting/FIFO gaps; Replied(None) cannot establish completion.
Do not poll or call `cancel` merely for cleanup: cancel itself delegates to the
consuming poll. Later explicit resume reconstructs receipt admission at fresh
time from retained wire; it neither borrows a stored permit nor resumes a closed
socket implicitly. RED: inject receipt-send failure after a valid Reply and
verify the original Call still yields its exact payload, with identity/wire and
uncertain acknowledgment retained. Receiver replay and automatic retry remain
separate gates; unchanged retention bounds eventual service-owned reclamation.

## Approved C8 optional registered-Reply observation

Add this shared `ReceiptTracker` seam before caller integration:

```rust
pub fn observe_registered_application_reply(
    &mut self, binding: &MailboxBinding, encoded: &[u8], now: ServiceTime,
) -> Result<Option<SendHandle>>;
```

Retain the existing expiry, full canonical verification, Reply-kind and context
checks. A unique complete registered match follows existing deadline/commitment
checks and returns Some; ambiguous complete matches remain Conflict. None means
no registered request identity is known under the authenticated tenant/domain/
peer. If correlation OR causation identifies a retained request there, failed
content/capability/reply-address or other matching checks remain Unauthorized,
not None. Never suppress known expiry/conflict or generic verification errors.
Unknown observation allocates no retained identity, payload or observation state.
The existing strict `observe_application_reply` wraps this method and maps None
to its existing Unauthorized result; canonical verification runs once per call.

Caller Late sends its already prepared canonical Late receipt, skips correlation
and changes no current progress. Duplicate may use the optional observer: None
sends only its actual Duplicate receipt; Some contributes current progress only
when its handle equals the pending SendHandle. AcceptedEphemeral requires Some.
Duplicate never upgrades acceptance level: even an originally Late identity can
produce Duplicate on replay. Canonical admission of a known Late FIFO position
may unblock queued Replies; run the ordinary readiness/poll gate afterward using
only existing verified current facts. No new ACK, wire, borrowed permit or retry.
Test unknown valid Reply, Late then replay, known mismatch/expiry/conflict and
unchanged strict behavior before claiming this broader stale-event handling.

## Approved first receiver event-loop slice: fence without resume

A private `Session::receiver_incomplete` latch suffices for this bounded slice;
an owned resumable PendingReply structure is not required. Set the latch before
the initial receive. Any error or unwind leaves it set, and a subsequent handle
must return StreamFenced before reading another frame. Existing canonical service,
tracker and reply-cache state remain owned. Do not infer cancellation/rollback
from stack-local envelope/result disposal. Clear only after authenticated current
Reply acknowledgment releases the cache and all completion bookkeeping succeeds.

Keep prepared Reply, registered SendHandle, exact encoded Reply, namespace,
application result and metrics local during the active invocation. Bound one
exchange by 16 complete incoming frames including the initial Call, and at most
eight total Reply-send invocations including the first; increment before transport.
Start one Instant deadline before initial read; cap it once at authenticated
admission by the original request's remaining Unix lifetime. Never refresh it on
duplicates or after application work. Before release/completion require that fixed
deadline and fresh original Unix expiry; acknowledgment APIs alone do not enforce
the original Call's monotonic lifetime. Existing canonical checks still apply.

The implemented private `ExchangeDeadline` also retains the checked owner-
monotonic expiry derived once from the initial trusted time and signed remaining
lifetime. Reject owner-monotonic regression and expiry equality. Its guards apply
before initial receipt reservation, application work, post-work Reply preparation,
fresh initial send, event/ACK release, duplicate receipt reservation and replay.
This closes the initial post-work path as well as the later replay paths. It does
not preempt computation or promise that a blocking write completes before expiry.

While awaiting the Reply receipt, authenticate controls with observe_receipt and
take only the current SendHandle. Pending/earlier duplicates continue within the
same event bound; only its verified eligible receipt may acknowledge_reply.
For a normal candidate Call, first use retry_reply_prepared with the incoming
binding and require its exact cached Reply bytes equal the current Reply BEFORE
admission. A different/new Call cannot gain state while active credit from the
original application may already be released. Then canonical admission must yield
Duplicate; send that actual receipt, register the unchanged cached Reply and
require the same SendHandle, then resend within the invocation bound. No receive
of application work, object.handle, next_reply, new identity or resealing occurs.
All I/O stays outside dispatch; permits are local and failures fence the Session.

RED gates: duplicate exact Call during receipt wait yields the same cached Reply
and one object execution; earlier controls cannot release current cache; different
Call is refused without admission; send failure/event saturation/deadline keep
the latch and prevent a later handle from consuming bytes. Explicit resume,
timer-driven request retransmission and full C8 remain separate reviewed slices.
