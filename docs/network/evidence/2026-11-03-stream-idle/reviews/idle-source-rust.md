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

## Frozen negative-test and manifest resolution

Reviewed final test/manifest state at HEAD
`960e73c59d7eab62f15a16667f95e594e1cabc8b` on 2026-11-03. **CODE/RUST and scoped
INFRA PASS — no blocking findings.** The product identity above is unchanged.
This completes the earlier in-progress test-source review, not the future hosted
qualification or consumer review.

```text
4644f962641862ba700402b1e58b4d2fbffd7afef285177ba4d6bfbd71f24ee7  crates/spiralcore-mvm-adapter/tests/service_tcp/idle.rs
ed96343399b0a3402af8a75fcdfbbf5b4be84974a51c7507170b53c23eb26491  crates/spiralcore-mvm-adapter/tests/service_tcp.rs
cfa21562f617d07aa8d14ce63de7285b43d062d7dd1a116742470bd41bf939c4  docs/platforms/mvm-mailbox-portable.json
```

Seven idle tests plus the four existing TCP tests are present. The inspected
GREEN receipt reports 11 passed, zero failed/ignored/measured/filtered. Its prior
10-pass/one-failure receipt remains: the real-TCP fixture expected six bytes for
the seven-byte `\0later\0` payload. The correction uses the exact payload length;
it neither changes product behavior nor weakens byte/counter comparisons.
Qualification cases added after implementation are not relabeled semantic REDs.

The scripted SocketIo fixture releases its own mutex before waits, supplies
retryable errors without pretending they consume a positive window, and retains
actual data returned after a scheduled wait. Split-prefix/body cases require
exact payload/counters after the idle boundary, or absolute-deadline failure with
partial counters and zero completed frames. Fatal cases check typed EOF,
cancellation/deadline precedence, malformed lengths, read/configuration failure,
subsequent Closed/Shutdown refusal, unchanged counters and no extra read calls.
Invalid arguments precede state/cancellation checks without shutdown; valid
window endpoints and later authorized operations remain exercised.

Lock tests use an explicit held-reader signal and bounded channel waits. A losing
contender cannot claim Idle or globally close the current reader; after eventual
acquisition the caller must perform an actual read before Idle. HeldReader's
normal and Drop paths release and join the worker. The real TCP fixture withholds
peer data by a channel until after Idle, then checks exact later bytes, a canonical
authorized reverse send and matching directional counters. Its owning raw socket
handles and Drop guard wake and join on failure; normal completion also joins.
Short scripted deadlines are functional boundary probes, not latency evidence.

The entire manifest diff against the frozen product checkpoint is solely
`service_tcp.minimum_count: 4 -> 11`. JSON parses, the value matches the actual
target count, and all seven OS/architecture rows, toolchain, dependency pins,
other target minima and storage scopes are unchanged. There is no workflow,
permission, concurrency, timeout, runner-resource or secret-handling change.
The existing native workflow must execute the new cases rather than silently
accept its former four-test minimum.

Only this review resolution was appended. No source/test/manifest edits, builds,
tests, CI launches, new audits or performance runs were performed by this reviewer.
The root's reported full release/Clippy success is separate execution evidence.
Frozen source is approved for the planned hosted CI and normal legacy-compatibility
confirmation, with no timer/retry/resume consumer or full C8 claim implied.

## Step 2 final CODE/RUST confirmation after procedural PASS

2026-11-03: **PASS — no blocking CODE/RUST finding for frozen source
960e73c59d7eab62f15a16667f95e594e1cabc8b.** This reuses the approved source/test
reasoning above and follows the task-specific Step 1 PROCEDURAL PASS retained
in `../coverage/procedural-gate.md` and local AUDIT. It is the code/Rust portion
of Step 2; the independent security conclusion and root's publication/merge
work remain separate. No new source audit or additional execution was introduced.

Fresh hash checks confirm live.rs `29010dded67185207d677dca8bcbf10a447336c96b689e0e5308aac214ec9eac`,
idle.rs `4644f962641862ba700402b1e58b4d2fbffd7afef285177ba4d6bfbd71f24ee7`
and portable manifest `cfa21562f617d07aa8d14ce63de7285b43d062d7dd1a116742470bd41bf939c4`.
[Hosted custody verification](../qualification/portable/verification.md) now
independently verifies seven original ZIP/API digests and all 236 inner hashes,
actual native OS/architecture/Rust-host identities, TCP11/receipt21 and 88 tests
per row (616 total), plus eight separate Unix storage executions. The actual
merge checkout 1da75b1aa4a6aacda5f47706caeeeaa213f0e99a has the exact reviewed
head tree 7d4d1568fd6f2143992b145abf68a790b1b1d4ae. The Linux original log confirms
92 live release and 131 shared (8 object-v1 + 123 adapter), all zero failure,
ignored, measured and filtered counts; seven additive TCP tests change shared
only. Strict Clippy, default guest compile and both synthetic analyzers succeed.
Zeros, overlapping execution scopes, Windows CRLF source projections and absent
Windows storage are retained explicitly.

Step 1's selective coverage receipt reports changed live.rs lines 96/97 (98.97%)
and broader mapped file 673/733 (91.81%), branch NA. Those are their stated
line denominators, not whole-project or branch coverage. Normal strict-transfer
measurements and root's interpretation remain distinct from idle-consumer proof.
This approval neither enables retry/resume/timers nor closes C8, WAN, native
Windows MVM, fleet or full-goal acceptance. Reviewer writes are limited to this
review and the owned new qualification packet; no tests/builds/CI, product edits
or broader audit were performed.
