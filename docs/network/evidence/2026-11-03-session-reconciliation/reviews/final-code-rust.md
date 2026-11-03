# Final explicit reconciliation CODE/RUST source review

2026-11-03. **SOURCE PASS — zero CRITICAL or HIGH findings in this frozen scope.**
Reviewed product changes from `e984d2d64cdf0483d50d6d59319276c85e7df644` through
`07e1bf837458af0b5d3bb7d050d7749a2071263d`, then the two fixture-only corrections
`b80bfdcd35e5674b1549efc4a4eafec4ae9b831f` and
`afe3d0a2bdea6aaa379c0cf98b5a235dc71b51a3`. The six changed source files are
identical between 07e1 and afe3. Required local code/Rust reviewer instructions
were read; the local File Claims table has no in-progress conflict. Explicit
read-only review instructions take precedence over template build commands.
Independent security and final exact-source qualification remain separate.

```text
d55ca44a3812d80c88d94811c7889fbf142340f16776a68d1b8129702a4318b2  crates/spiralcore-mvm-adapter/src/live.rs
27a6827d592847a3c560eb4187660ee3dae57e3f583b9964eecbba3c4c672ece  crates/spiralcore-mvm-live/src/campaign.rs
7ff5e5ec5ed907334fbfe200f37b3ed6f3e537f43a8da955a25b6196accb1b86  crates/spiralcore-mvm-live/src/campaign/canonical.rs
601b40bad57fe4b1f39f04db531aa73c95dd862d965e9e0ad7d4ab96b3797708  crates/spiralcore-mvm-live/src/campaign/reconciliation.rs
20ae5413ab61cf015dab8c833bed25ee6f033eaff5563e186ef642c42e6342de  crates/spiralcore-mvm-live/src/campaign/session.rs
d22755584c91af51eb4532bc7fe84fde26cfa115fb72950fb30c58985eb7c1a1  crates/spiralcore-mvm-live/src/orchestrator.rs
0c89489a78a875f71996a10e6687465f48f46891916168de306f539792684980  final campaign/canonical/write_deadline_tests.rs
```

## Source conclusions

This confirmation reuses the [state](state-rust.md), [caller](caller-rust.md) and
[receiver/metrics](receiver-metrics-rust.md) reviews and the contract-first
Reply-first amendment. No new wire, codec, receipt authority, capability/rate
allowance, pipeline size, retention or default guest opt-in is introduced.

The actual public embedding API creates one private capacity-one channel per
Call/Status probe; the opaque signal contains no payload/authority. Callback and
I/O stay outside broker dispatch. The strict entry takes None, allocates no
channel, invokes no callback and keeps blocking strict receive. Signals disconnect
when their invocation ends and cannot address the next identity. Session starts
no controller thread. Its original send latch prevents unresolved reentry from
reading/sending a replacement; retry uses the same handles/prepared/encoded Call.
The original plus seven explicit starts exhaust eight invocations; every complete
incoming frame counts toward the same sixteen-event bound. FIFO completion is
checked before consuming a queued command; ready-but-Pending retains that command.

All eight exchange-owned writes use the original absolute deadline view. It owns
only the existing transport Arc/route, shares fence/counters and adds no send
authority. Fresh owner-monotonic/Unix checks, original generation/commitment and
existing policy-locked send authorization remain required. No outer policy lock
or broker lease spans I/O. The prior raw-stream P1 is closed in source and now
has a controlled old-caller RED/current-caller GREEN writer-loop regression.

Active replay authenticates the current cached identity before Duplicate admission,
sends the exact cached Reply first, then freshly rechecks original deadline,
owner/bindings, exact cache and tracker handle before actual Duplicate receipt
reservation/send. Existing ACK processing precedes caller completion on that
closing receipt; no proactive extra frame is added. Inactive cache-None remains
actual receipt-only, without object work/cache creation/another Handled. Fresh
inactive admission verifies once. The sixteenth normal preamble frame performs
lookup before admission and refuses a fresh identity without allocating its state.
Preamble and active loop share the original deadline/event ceiling and no-resume
latch. Only authenticated eligible current Reply receipts release cached state.

Checked-u64 recovery observations separate commands, Idle, attempts and successful
historical writes. Versioned v2 labels static expectations as normal, while
StreamCounters remain actual traffic. An owned post-frame processing phase closes
before receive waits and on Err, retaining unfinished historical work. Per-Handled,
cumulative, process CPU and lifecycle domains overlap and are explicitly labelled;
they establish no exclusive thread CPU, energy or comparative performance result.

## Test integrity and observed gate boundaries

The fault gate consumes real canonical ciphertext frames and records actual input
and output legs separately; withholding does not pretend the outgoing write
succeeded. Bounded captures, cancellation/shutdown handles and joined workers
preserve failure cleanup. The actual duplex relay is reached through a test-only
thin re-export, not a copied relay or runtime alternate. Public tests traverse
Client wiring and exact object publication/reopen. Signed capacity two exists only
in the FIFO fixture and does not alter the production pipeline-one grant.

The FIFO test proves an actually full queued command survives real Idle while
authenticated predecessor progress releases delivery, with zero retry invocations.
The final-event fixture uses fifteen authentic old ACKs then a fresh sixteenth
Call and checks EventLimit, unchanged application state, zero outgoing frames and
fenced reentry. The generation/deadline seam counts zero connections and retains
the burned invocation rather than pretending preparation is a send permit.

Scripted deadline writers return success only for bytes actually accepted into
their bounded ledger, then return retryable stalls; canonical StreamTransport
produces all frame/byte counters. Labels correctly say SocketIo writer-loop
conformance, not OS TCP backpressure. The retained f95-only caller substitution
hits the 1.5-second watchdog/Cancelled; fixed source refuses near the original
900-ms cap with partial payload counted, original identity retained and no further
read/write on reentry. Reported differing envelope lengths across runs are not
treated as identical cross-run ciphertext. Other writer cases exercise ACK,
initial Reply/cached Reply, silence and eight exact send invocations.

The fixture corrections preserve strong oracles: b80 obtains the expected wire
from real retained registration instead of separately sealing a different nonce;
afe3 checks ACK failure through ReceiptSendError::Service(Unavailable), retaining
direct ServiceError::Unavailable for Reply writes. No byte, deadline, charge,
sequence, fence, event/send ceiling or peer/cache assertion is weakened.
The earlier four receipt-first compatibility fixtures were adapted to the
approved Reply-first contract; their retained 21-pass/four-fail run is a fixture
ordering failure, not an invented new behavioral RED. The subsequent retained
compatibility log has 25 passed/43 filtered; the single fixed late-writer log has
one passed/67 filtered. Root reports six direct/actual-relay fault cases and three
public full campaigns passing, plus final affected release 111 passed: 73 library,
11 adapter-live, 1 broker, 4 concurrent, 9 persistent, 1 profile, 6 owner, 3 worker
and 3 public reconciliation, with two explicit zero-test scopes. Those execution
scopes overlap and are not added as independent coverage; no tests were rerun here.

Final Clippy/default/shared gates, coverage, exact-source hosted qualification,
registered strict-transfer measurement interpretation and protected merge remain
pending at this source verdict. The strict benchmark does not measure opt-in
recovery overhead. Windows live consumer/native guest opt-in, general ACK loss
after caller completion, closed-stream resume, installed ownership, scale/WAN and
full C8/goal acceptance remain open.

Only this review file was written. Source-diff whitespace checks passed. No builds,
tests, product/test edits, commits, pushes, new measurements or broader audit ran.
