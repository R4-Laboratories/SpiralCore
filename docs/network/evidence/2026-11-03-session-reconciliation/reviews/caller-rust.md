# Opt-in caller/public-entry delta: independent CODE/RUST review

2026-11-03. **BLOCK for the committed caller delta: one fixed-I/O-deadline
finding.** Reviewed committed source `f95c0f4076f2d8ca442124fbbe857f56f2cf4179`
against `fdc89183149699c6d8b7d49247f4212494e7f5c8`; concurrent working-tree
receiver/test changes are excluded. The previously approved PendingCall state
helpers are not re-audited. No product/test edit, build or test was performed.

```text
27a6827d592847a3c560eb4187660ee3dae57e3f583b9964eecbba3c4c672ece  committed crates/spiralcore-mvm-live/src/campaign.rs
dd59442c9cb662060ba9866c2f0152be7aa7ef48e05fcef1c3b6de52c6346a69  committed crates/spiralcore-mvm-live/src/campaign/canonical.rs
92da1719c1a45e9cbbcd33b1b9a297b00e68e9749556b76c1f5b897f5a8ba39c  prerequisite campaign/reconciliation.rs
```

## Blocking finding

**P1 — retain the exchange deadline during the retry write.** At canonical.rs
lines 312-322 (raw stream argument at 319), retry_call_prepared receives the
ordinary StreamTransport. Its PeerTransport::connect creates a deadline of
Instant::now() plus the transport timeout. Therefore an explicitly requested
retry near the end of the original exchange can connect/write for another full
timeout after the original fixed Instant has expired. The two fresh service-time/
Instant checks before invocation prevent a late start, but do not bound subsequent
blocking I/O. The adapter authorizes using supplied time at entry and cannot
repair this transport deadline reset. This violates the reviewed fixed lifetime
contract and can transmit beyond that invocation's permitted interval.

Use a contract-approved transport view bounded by the existing absolute deadline
for that invocation. The adapter already has a DeadlineTransport/before pattern
internally, but both are private to adapter/live.rs. My initial message suggested
calling stream.before directly and incorrectly said no API exposure was needed;
the concrete visibility check corrected that advice before implementation. Any
public exposure needs the project's contract-first step, or an equivalent existing
authorized bounded surface. No ad hoc competing authorization path is warranted.
A focused blocked-write boundary must qualify the eventual fix; healthy TCP
completion does not exercise this failure. The finding is against the pinned
snapshot and must remain in review history after resolution.

## Other source conclusions within this delta

The public embedding API reaches the normal library Client path, without a
dead-code allowance or test-only implementation. A fresh opaque capacity-one
signal/request pair is created per Client::call, including the Status probe.
The callback receives only the signal and executes before canonical broker
dispatch; errors drop the unused receiver. Requests move into one invocation;
drop disconnects stale signals. Session creates no controller thread, exposes
no raw-channel constructor and stores no borrowed permit or authorization in
the channel.

Existing sender/sender_with_broker choose None, so their Client branch creates
no channel, invokes no callback and calls the unchanged strict receive branch.
The new opt-in branch uses 250-ms zero-prefix Idle. Local commands and Idle do
not increment the incoming event count. On Idle it checks original lifetime
and absolute Instant, then completion readiness before touching the request
queue. If canonical FIFO poll returns Pending, the queued request stays queued
and no current Call is resent. Missing completion facts permit one nonblocking
request dequeue; Empty/disconnection sends nothing.

Preparation occurs under a short broker dispatch; a separate second broker
sample follows it before start_retry. The invocation is burned immediately
before the immutable authoritative adapter call, outside dispatch. No new outer
policy guard or broker lease spans I/O. Original handles/prepared bytes and the
same retained event/send counters are reused. The existing invoked-send latch
prevents reentry from issuing another initial send or reading another frame after
an unresolved failure; different payloads retain Busy rather than replacement.

The extracted poll_ready_reply keeps the prior canonical remaining-time check,
FIFO Pending behavior, one-time Replied(Some) decoding and explicit
SessionError::Service namespace for consumed/terminal status errors. It does not
turn tracker observation readiness into application completion on its own.
The production diff's whitespace check passed.

## Pending qualification and accounting

Root reports healthy public TCP 1/1, signal 2/2 and state 2/2 GREEN; these were
not rerun here. Loss/reconciliation semantics, FIFO waiting with a queued command,
silence without commands, expiry/write-boundary fencing and direct/relay fault
paths are not yet proven by this snapshot. Receiver changes and final complete
source/release/Clippy/CI gates remain pending. This is not a release approval.

Session.report still contains frames_per_call 4 and its normal-path envelope/
receipt fields. Those are nominal normal-exchange figures, not actual recovery
traffic. Existing StreamCounters record actual writes/reads, but dedicated checked
retry invocation, command, Idle, retransmission, preamble and failed-observation
fields have not yet been added. Recovery accounting and failure-meter retention
remain required qualification work before interpreting or publishing consumer
measurements. No Idle-consumer performance, native Windows live consumer, WAN,
resume, cap increase or full C8 completion is established.

Only this review document was written. Scope stayed on the committed caller
delta and its immediate transport deadline boundary; no broader audit ran.
