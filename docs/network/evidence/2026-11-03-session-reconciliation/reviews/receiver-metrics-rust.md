# Receiver, bounded writes and recovery metrics: source confirmation

2026-11-03. **CODE/RUST and scoped security source PASS — no blocking finding in
this committed delta.** Reviewed `767ea4cef66aa640a0924500809c25031bde0c74`
against the earlier caller snapshot `f95c0f4076f2d8ca442124fbbe857f56f2cf4179`,
reusing the [caller](caller-rust.md) and [state](state-rust.md) reasoning. The
earlier P1 deadline reset is resolved in source as described below; its actual
late-write consumer regression and complete-feature qualification remain pending.
This is not release approval or a wider security audit. Concurrent uncommitted
fault-matrix/late-writer work is outside the pinned source verdict.

```text
d55ca44a3812d80c88d94811c7889fbf142340f16776a68d1b8129702a4318b2  committed adapter/src/live.rs
d0cc0795c385cf2aa9d6dd7cedba82af36b086c85608e099e03d99cd0f9d613d  committed live/src/campaign/canonical.rs
601b40bad57fe4b1f39f04db531aa73c95dd862d965e9e0ad7d4ab96b3797708  committed live/src/campaign/reconciliation.rs
f07c30bb8036b41a754da026a6e38a41b74b049f4ea11693674101deba992e14  committed docs/api/session-reconciliation-v1.md
e27e475700a2879cb02448da363155af508a194ad2e3676abf2ef40fd678ae76  committed docs/api/stream-send-deadline-v1.md
```

## Earlier P1 source resolution

The contract-first additive before(Instant) method exposes the existing private
deadline view through an opaque PeerTransport return with precise socket-type
capture. The returned object owns the existing Arc clone, not a mutex guard or
borrowed permit; route/ready/fence/counters and canonical route/frame authority
stay shared. The effective cap is fixed at creation; connect, later channel use,
mutex waits and writes retain it. Default StreamTransport connect behavior and
the implementation of DeadlineTransport/SocketChannel remain unchanged.

All eight canonical write sites now use the retained deadline: caller initial
Call, explicit retry and Reply-ACK; receiver initial receipt/Reply, active
duplicate receipt/cached Reply, and historical preamble receipt. Caller writes
use the original invocation Instant; receiver writes use its ExchangeDeadline,
with historical receipts capped by the preamble Instant and authenticated signed
remaining lifetime. Recreating a view always passes that original cap rather
than now plus a fresh interval. This removes the raw-stream timeout reset found
in f95 without weakening existing authorization or adding an outer policy lock.
Fresh service-time checks and the existing original-owner lookup remain required
at their authorization points; the view is an I/O bound, not extra authority.
Actual blocked/partial-write and late-consumer regression evidence is still a
required final gate, not inferred from healthy TCP or this source inspection.

## Inactive preamble and active replay

The existing receiver cap and no-resume latch precede socket reads. One preamble
Instant and sixteen-event allowance cover old controls, historical Calls, eventual
fresh Call and its active ACK/replay loop. Normal event sixteen performs the
authenticated retained lookup before admission: unknown fresh identity refuses
EventLimit before allocating application state with no remaining ACK slot. The
active-loop bound begins at the already consumed event count rather than resetting.

With room left, inactive fresh admission goes through returns.admit once, retaining
the existing canonical inner/outer verification and AcceptedEphemeral proof path.
Only that proof reaches application/object work. Provisioned key IDs select a
parser path but grant no authority: controls use tracker authentication; ordinary
frames require canonical admission. Controls update retained tracker facts only,
produce no receipt-of-receipt and create no new application result.

Historical normal handling requires an actual Duplicate disposition, then the
canonical retained original-owner/generation/commitment/lifetime lookup returning
None. The outer expiry used for the cap is read only after canonical admission
has authenticated its equality with signed inner expiry; it never substitutes
for retained owner checks. Lookup/checks run before send-rate reservation and
again with fresh time after it, before bounded I/O. Cached Some or lookup error
refuses/fences without receipt transmission. None means known identity/no cache,
not completed object work. There is no receive/object call, cache synthesis,
application-call increment or second Handled on this historical branch.

The active Reply-wait branch retains its original authenticated exact-cache
prelookup before admitting a repeated Call, actual Duplicate receipt, fresh
post-receipt checks, exact original Reply encoding and same tracker handle. A
different fresh Call cannot acquire state through that active branch. Only its
matching authenticated eligible Reply receipt releases the cache. The eight
Reply-invocation and sixteen-event ceilings stay finite; errors retain the latch.
Broker dispatch ends before all potentially blocking writes and object work.
No wire, alternate codec, route/capability bypass, new allowance or network
inspection authority is introduced by these changes.

## Metrics and failure accounting

Recovery fields are checked-u64 counts with explicit event meanings. Idle counts
actual opt-in outcomes, commands count actual dequeues, retry counts burned API
invocations including failures, and historical send attempts/successes remain
distinct. Reply total invocations include cached retries as a subset. No success
counter claims remote receipt. Existing StreamCounters remain actual bytes/frames;
normal four-frame expectations are explicitly renamed and versioned under
canonical-session-report/v2 rather than presented as recovery traffic.

Receiver processing starts only after a complete incoming frame. One owned
optional phase spans authentication/reservation/application/outbound work and
ends before the next receive wait. Completed preamble segments accumulate into
the next Handled meter. The outer handle wrapper closes an outstanding phase on
Err as well as success, so failed/preamble-only work persists in cumulative totals
even when no Handled exists. Checked duration conversion/addition refuses overflow;
there is no truncation, global timer, helper thread or forgotten borrowed guard.
Report snapshots after returned handle calls see closed phases.

ProcessTime is process CPU during each interval, not exclusive receiver-thread
CPU. Per-Handled and cumulative processing domains overlap; receive waits and
partial reads are excluded from this processing domain while lifecycle/transport
observations retain their own boundaries. The committed labels and retained
JSON make those limitations explicit, without energy/performance inference.

The inspected focused GREEN log reports one missing-Call-receipt direct test
passing (61 filtered). Its retained metrics-green JSON reports one historical
receipt invocation/success, one cancelled second-handle failure, empty cache,
no second Handled and three cumulative processing segments; the cumulative
processing exceeds the first Handled's processing. These observations agree
with the new wrapper/counters and describe one Unix TCP fixture, not the still
pending full fault/relay/platform matrix. The earlier metrics RED remains intact.

## Remaining gates and scope

Whole-feature loss combinations, authority/generation/lifetime failures, FIFO
waiting with queued commands, signal capacity, cumulative events/sends, blocked
canonical writes and direct/actual-relay cases still need their frozen execution
records. Strict compatibility, full affected release/Clippy/coverage/CI and exact
final-source review remain separate. No native Windows live consumer, native
guest opt-in, WAN, general ACK loss after caller completion, resume, retention or
cap increase, comparative performance or full C8 completion is approved here.

Only this review file was written. Production diff whitespace checks passed.
No source/test edits, builds, tests, new experiment or broader audit were run.
