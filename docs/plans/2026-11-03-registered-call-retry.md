# Plan: registered Call retry seam

Date: 2026-11-03. Base `a8a4a9a8daba67c00447a067b18632ed515d95b2`.
Lead-approved design: [registered Call retry v1](../api/registered-call-retry-v1.md).
Delivery authority: [#43](https://github.com/R4-Laboratories/SpiralCore/issues/43),
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49) and parent
[#42](https://github.com/R4-Laboratories/SpiralCore/issues/42).
This is one incomplete C8 prerequisite, following the
[persistent service plan](2026-11-03-persistent-mvm-service.md).

## Choice and boundary

Add `LocalMailboxService::retry_call_prepared` for a retained original Call in
`Pending` or unconsumed `Replied(Some)` state. This addresses the narrow authority
gap where a Reply is locally available but the original destination receipt may
still be missing. Preserve the original strict API and all canonical validation.
The first source slice has no campaign consumer or receiver changes.

Three options were considered:

| Option | Benefit | Cost / disposition |
| --- | --- | --- |
| Explicit registered retry API | Reuses original authority and retained payload; sends only when invoked | Chosen prerequisite; alone gives no end-to-end recovery |
| Automatic resend after receive Idle | Could drive recovery without another application action | Silence on slow reliable TCP can generate needless duplicate bytes; requires consumer bounds and completed-receiver handling; deferred |
| Reconnect/resume with owned receiver state | Could recover a closed exchange | Larger ownership, fencing and authorization lifecycle; deferred |

The API introduces no attempt/event counters or timer. Existing session bounds,
tracker registration and exact original-envelope ownership stay with the producer.
The retained Reply stays charged until ordinary canonical extraction/retention;
the retry path neither takes it nor refreshes its original Call deadline.

## Small implementation sequence

1. **Focused RED.** Extend
   `crates/spiralcore-mvm-adapter/tests/call_send.rs` using existing canonical
   fixtures. Admit a real Reply, leave it unconsumed, require strict send to
   refuse, then call the missing retry API. Record the missing-API RED before
   implementation; after it exists, assert exact transmitted original Call,
   unchanged Reply reservation and exact one-time payload extraction. Consumed
   `Replied(None)` must refuse retry with no transmission.
2. **Surgical production seam.** In
   `crates/spiralcore-mvm-adapter/src/service.rs`, extract the current immutable
   validation into a private immutable helper returning `SendExpectation`, with
   explicit unconsumed-Reply permission. Existing send passes false; additive retry
   passes true, then both use existing `send_prepared_inner`. Only the status gate
   gains `Replied(Some)` eligibility. Avoid a second policy lock or eight-argument
   helper. Keep validation/error precedence and existing verification/transport. Aim for
   roughly 50 changed production lines; split only if needed for clear review.
3. **Negative and compatibility gates.** Test Pending, consumed Reply, original
   mono/Unix expiry, clock regression, original-generation/request/ownership
   mismatch, cancellation/shutdown and shared accounting-clock independence.
   Reuse existing lifecycle-negative cases; keep the focused set below ten test
   functions rather than duplicating fixtures. Strict send retains its behavior.
   Inspect payload/budget
   preservation without consuming it as an eligibility probe. Independent review
   checks the exact diff and hashes before release validation.
4. **Validate and deliver.** Run focused adapter tests, affected locked release
   checks, formatting/Clippy and existing portable/Linux CI as appropriate to the
   source change. Retain RED/failure records. Root updates issue checkpoints and
   opens the reviewed source PR; do not close C8/#43/#49 for this prerequisite.

## Evidence and follow-on gates

Normal product/measurement engines do not invoke the new API in this delivery.
No performance claim or repeat native campaign is warranted for an unused API;
record its source identity, focused semantic results and compatibility checks.
Retain current cap 2,048, pipeline one, signed budgets and canonical wire.

A later explicit reconciliation consumer must preserve original PendingCall
facts and cumulative bounds, decide when an intentional exact retry is useful,
and qualify missing receipt/Reply/ACK cases. Completed-receiver handling of an
authenticated old Duplicate after Reply-cache release is a separate contract
revision; no cache synthesis, acceptance invention or object reexecution is
authorized here. Closed-stream receiver resume, automatic timers, long-session
retention/scale, supported platform matrix and real-WAN acceptance remain open.
