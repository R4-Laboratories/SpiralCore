# Zero-prefix idle API: independent CODE/RUST review

Date: 2026-11-03. Verdict: **PASS — no blocking source/Rust finding in this
bounded additive API slice.** Qualification, negative portable tests and any
consumer behavior remain separate. This source is pending on
`codex/spiralcore-session-idle`, not asserted to be delivered on main.

Reviewed product commit `bc854e062082adfbadda599136976c77e97cf13f` against the
approved `docs/api/stream-receive-idle-v1.md` contract. Exact product identity:

```text
29010dded67185207d677dca8bcbf10a447336c96b689e0e5308aac214ec9eac  crates/spiralcore-mvm-adapter/src/live.rs
2dcc00c26cbbb745d4531dadc3f38c8d2e8617fa77f61a5eb46a3e4cd9cff273  docs/api/stream-receive-idle-v1.md
```

## Source conclusions

- ReceiveOutcome is additive local metadata. LiveError, wire encoding,
  authentication authority and existing strict receive/raw-helper interfaces
  remain unchanged. Frame remains ciphertext requiring canonical verification.
- Invalid idle windows are rejected as existing typed Io(InvalidInput) before
  locking, cancellation/state inspection or socket I/O. This path changes no
  state or counters and does not fence a ready transport.
- The effective frame deadline is fixed at valid method entry and capped by
  the caller deadline, transport timeout and 30-second operation bound. Mutex
  acquisition uses that deadline; only after ready/closed validation does the
  positive idle window start. Lock waiting cannot produce Idle or refresh the
  frame deadline. Existing contention, cancellation and poison behavior stands.
- read_exact_progress may yield Idle only at local consumed-prefix offset zero.
  Every idle decision checks cancellation and absolute expiry first. Early
  timeouts/interruptions keep both original timers. A positive read retains and
  counts its bytes, disables idle for the remainder of that prefix, and uses
  the same absolute deadline through prefix and body completion.
- The shared payload parser validates the unchanged nonzero/262,144-byte limit
  before allocation. Idle uses only a stack prefix, retains no partial frame,
  leaves counters/ready state unchanged and returns after dropping the mutex.
  Actual byte counters and completed-frame counts retain their strict behavior;
  partial-read failures preserve their byte observations.
- Strict helpers call the progress reader with no idle deadline, so Idle is not
  reachable there. Endpoint EOF remains fatal; only the existing separate relay
  helper permits clean EOF. Framing, read/timeout-configuration, cancellation,
  deadline and counter errors close/fence the owning read path using the existing
  shutdown fallback. No error is interpreted as recoverable silence.
- New private helpers centralize progress/payload parsing instead of duplicating
  a framing codec. Ownership remains one serialized transport, with no new queue,
  socket clone, background timer, stored borrowed state or unsafe Rust.

The implementation run's retained primary/strict-TCP record reports five passing
tests. Negative/portable extensions in `tests/service_tcp/idle.rs` were being
written concurrently and are outside this frozen product-source verdict until
their own completed source identity and execution record exist. In particular,
partial-prefix/body stalls, boundary data, early sliced timeouts, lock contention,
argument validation and cancellation/deadline precedence remain the approved
contract's executable gates, not inferred coverage from this manual review.

No product edits, builds, tests, coverage, new audits or experiments were run by
this reviewer. Only this review record was written; whitespace checks passed
for the frozen product diff. Idle means zero next-prefix bytes were consumed,
not that a peer lost a frame or that retry is safe. There is no timer/retry/resume
consumer here, no signed lifetime extension, and no full C8/WAN/release claim.
