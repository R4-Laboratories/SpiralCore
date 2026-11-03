# Receive-idle source security and architecture review

Date: 2026-11-03. Verdict: **PASS for the narrow source/API implementation**;
no blocking case found in this review. Focused negative and compatibility
execution gates remain required. No builds, tests, measurements or external
actions were performed by this reviewer.

Reviewed commit: `bc854e06`, against main `6269242`. Production file:
`crates/spiralcore-mvm-adapter/src/live.rs`, SHA-256
`29010dded67185207d677dca8bcbf10a447336c96b689e0e5308aac214ec9eac`.
Scope: additive receive-idle implementation and its shared framing refactor,
against [stream receive-idle v1](../../../../api/stream-receive-idle-v1.md).

- `read_exact_progress` returns Idle only while its local offset is zero and
  the positive idle deadline elapsed. It rechecks cancellation/original frame
  expiry before returning Idle. Interrupted/early sliced timeouts do not refresh
  either deadline. A positive prefix read irrevocably disables Idle for that
  prefix; payload reading receives no idle deadline. No partial bytes are dropped.
- The effective frame deadline is fixed once at method entry and capped by the
  supplied absolute deadline, existing transport timeout and 30-second maximum.
  Lock waiting, first-byte arrival and payload progress never extend it. The idle
  clock begins only after successful lock and ready/closed checks.
- Invalid idle durations outside 1 ms through 1 s return existing Io/InvalidInput
  before lock/I/O and do not fence. No LiveError variant is added. Valid calls
  retain existing lock semantics: acquired-guard deadline/cancel failure fences;
  a contended-lock error without ownership does not promise physical closure.
  Unready/closed state never becomes Idle.
- Idle returns without a heap frame, byte/frame counter change, retained parser
  state or escaped mutex guard. A racing queued frame is not discarded; Idle is
  zero consumed prefix bytes, not evidence that a peer lost data. An authorized
  writer can acquire the same mutex after return; this adds no fairness or
  simultaneous reader/writer guarantee during a partial frame.
- Shared `read_frame_payload` retains the nonzero/maximum length check before
  allocation, original payload deadline and completed-frame increment. Partial
  counters remain available on failure. Strict readers pass no idle deadline;
  optional relay EOF behavior and strict endpoint EOF conversion remain intact.
  Fatal owned framing/read errors use the existing closed fence and Darwin-aware
  shutdown fallback, preserving the original error.

Canonical framing, ciphertext verification responsibility, route authorization,
resource ceilings and endpoint consumers are unchanged. There is no timer,
automatic retransmission, resume, grant/TTL extension or new acceptance fact.

The implementation owner reports the missing-API RED and five TCP checks GREEN,
including healthy Idle followed by authorized writing and an exact later frame.
This reviewer inspected source, not their execution. Before release retain the
separately owned gates for split prefix/body beyond idle but within absolute
deadline; partial absolute timeout and counters; cancellation, malformed length
and EOF; invalid range/state precedence; shorter absolute deadline; and contended
lock timing without a false Idle. Existing strict/Darwin/canonical exchange
compatibility remains required. This source approval does not claim those
pending executions, consumer retry correctness, full C8, WAN or retention proof.
