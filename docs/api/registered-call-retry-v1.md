# Registered Call retry — additive Rust contract v1

Date: 2026-11-03. Architect contract approved for focused RED/code by the lead.
Baseline: `a8a4a9a8daba67c00447a067b18632ed515d95b2`; adapter `src/service.rs`
SHA-256 `7a58e929401c68f40461a762b72f88b46ef221bc83e33875013005f394e10fcd`.
Implementation and independent source/test qualification remain pending.
Authority: [persistent session state](../specs/2026-11-03-persistent-session-state.md),
with delivery under [#43](https://github.com/R4-Laboratories/SpiralCore/issues/43)
and [#49](https://github.com/R4-Laboratories/SpiralCore/issues/49).

## Decision and public surface

Add one method to `spiralcore_mvm_adapter::service::LocalMailboxService`:

```rust
pub fn retry_call_prepared(
    &self,
    binding: &MailboxBinding,
    call: &CallHandle,
    prepared: &PreparedMailboxEnvelope,
    transport: &impl PeerTransport,
    cancel: &CancellationToken,
    now: ServiceTime,
) -> Result<usize, ServiceError>;
```

The implementation may spell the return type using the module's existing
`Result<usize>` alias. This is an additive local Rust API: no wire, crypto,
codec, receipt, public error variant or canonical admission change.
Keep `send_call_prepared`'s signature and Pending-only behavior unchanged.
Both methods use one private immutable validator, for example
`registered_call_send_expectation(binding, call, now, allow_unconsumed_reply)`
returning `Result<SendExpectation>`, then the same `send_prepared_inner` path.
Pass false for send and true for retry. This avoids an eight-argument shared send
helper, duplicated validation or a Clippy allowance. Add no outer policy lock.

## Eligibility, validation and effects

Preserve the original send method's validation order and error mappings:

1. Reject service shutdown with `ShuttingDown`; negative Unix time or regression
   against this service's last recorded monotonic time with `Clock`.
2. Require this service and binding's handle, retained Call ID and exact retained
   handle. Foreign ownership is `Unauthorized`; absent retained ID is
   `UnknownHandle`, with existing precedence preserved.
3. Require `now.monotonic_ms < original_deadline` and
   `now.unix_ms < original_header.expires_at_unix_ms`, otherwise `Expired`.
   A retained Reply never refreshes or waives either original deadline.
4. Apply the following status gate without consuming or cloning the payload:

| Retained status | Existing send | New retry |
| --- | --- | --- |
| `Pending` | Eligible | Eligible |
| `Replied(Some(_))` | `Conflict` | Eligible |
| `Replied(None)` | `Conflict` | `Conflict` |
| `Cancelled` | `Cancelled` | `Cancelled` |
| `TimedOut` | `Expired` | `Expired` |
| `Unavailable` | `Unavailable` | `Unavailable` |
| `ShuttingDown` | `ShuttingDown` | `ShuttingDown` |

Then invoke existing `MailboxBinding::send_prepared_inner` with the original
Call generation and retained request commitment. Preserve its cancellation,
single canonical verification, policy-lock generation/commitment comparison,
route/capability/outbound authorization, encoded-size limits and transport error
mapping before/through transmission. A valid but different request commitment
is `Conflict`; changed policy generation is `Unauthorized`. Malformed/expired/
unauthorized material retains existing verification errors and their precedence.
No second verification or alternate transport authorization is introduced.

This immutable phase runs outside broker dispatch. It neither advances aggregate
accounting clocks nor mutates Call status, FIFO position, deadlines, registration,
receipt facts, active credits or retained Reply reservation. In particular, do
not call `poll_call`: that takes `Replied(Some)` and releases its payload budget.
No additional retained payload or retry-state allocation is introduced; encoding
and transport allocation remain those of the existing send path. The immutable
borrow prevents same-service local lifecycle mutation during I/O; existing binding
policy lock semantics remain authoritative, without a new distributed guarantee.

## Producer obligations and outcome

The producer supplies its retained original prepared request and tracker attempt;
it preserves message ID, semantic scope, sequence, correlation, causation, nonce,
delivery contract, issue/expiry, payload and encoded envelope. The shared service
pins the original semantic request commitment and generation; it does not add a
second outer-envelope identity store or create/re-register an attempt. Retain the
existing tracker authority and the session contract's lifetime/event/send bounds.
This API owns no timer, attempt counter or event state and cannot prove that an
earlier send was invoked. `Pending` was already eligible for repeated strict sends.

Success returns the existing encoded transmitted length, not authenticated
destination acceptance, Reply receipt, durable commit or application completion.
A transport failure can have an unknown/partial-send outcome. Preserve original
identity and uncertain state; this method neither rolls back nor retries itself.

## Scope and focused gates

The first delivery adds this API and tests only. The normal campaign keeps its
strict send path. No receive-idle consumer, automatic resend, receiver change,
new rate allowance, reconnect or resume is enabled.

Required RED/GREEN: a real canonical Reply leaves `Replied(Some)` charged; strict
send refuses it; retry sends the original encoded Call and leaves the exact Reply
available once to `poll_call`; retry after consumption refuses with no bytes.
Also cover eligible Pending, unchanged original mono/Unix expiry, time regression,
generation/request/owner mismatch, cancellation/shutdown and unchanged shared
accounting-clock behavior. Preserve existing strict-send tests and typed errors.
Use the existing adapter test fixtures and transport, with no benchmark engine.

End-to-end explicit recovery remains a later consumer contract: a healthy stream
idle is silence rather than proof of loss; a completed receiver currently rejects
an old Call at initial admission. A future authenticated Duplicate-only response
after cache release requires its own reviewed change. Closed-stream resume also
needs retained receiver ownership/state. This API alone completes none of those
gates; full C8 and issues #43/#49 remain open.
