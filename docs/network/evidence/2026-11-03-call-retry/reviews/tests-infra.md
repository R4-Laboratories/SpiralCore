# Registered Call retry: frozen tests and portable minimum

2026-11-03. **CODE/RUST and scoped INFRA PASS — no blocking test-source or
manifest finding.** This supplements [production review](source-rust.md), not a
new audit. Execution of the expanded frozen target was in progress at review;
its eventual GREEN, release/Clippy and hosted source qualification remain
separate records. No test or build was run by this reviewer.

```text
5563e59811266047b582651aa6e08682ff3c368d417c2aabdbee8930a1217794  crates/spiralcore-mvm-adapter/src/service.rs
a81c0ff09047261e0430af853ad030a10fa67f0a574a143cbaade9d393d991fb  crates/spiralcore-mvm-adapter/tests/call_send.rs
11c07ee150221854383254f13730e771fa1b013b3eb0d0ed42335ca5bc466cac  docs/platforms/mvm-mailbox-portable.json
```

The target contains six test functions: two original strict-send tests, the
primary real-Reply retry test and three expanded Pending/validation/fault tests.
The primary fixture creates a separate canonical receiver, actually admits the
Call, obtains its delivery proof, prepares an authorized Reply and admits that
Reply to the caller. There is no synthetic insertion into private Call state.
It asserts exact encoded Reply charge, one queued message and zero active Calls;
strict send refuses, retry transmits the exact original wire, and service/FIFO/
budget snapshots remain unchanged. One poll yields the exact binary payload and
releases bytes/messages; the next yields Replied(None), whose retry refuses.
Strict-send/consumed-Reply refusals assert no output; their pre-connect placement
also follows the reviewed production helper, not an instrumented count in that
primary fixture.

The Pending test checks exact original bytes at two times, unchanged service and
aggregate budget snapshots, two actual connections/sends and a later mutable
operation at an earlier time. The shared service is deliberately advanced first;
both it and the caller can subsequently operate at time 2 after retry at time 3.
This tests accounting-clock independence instead of relying only on snapshots.

The validation matrix has 14 Pending and 13 unconsumed-Reply cases. It covers
generation replacement, new request, changed payload under original options,
foreign binding/service, cancelled token, lifecycle cancellation/shutdown,
original Unix and monotonic expiry boundaries/past-boundaries, service clock
regression and negative Unix time. The Reply fixture advances its own recorded
time to 107; its regression probe uses 106, so the later Reply is not tested with
a falsely valid clock. All 27 cases assert the typed error, zero connect and send
counts, empty output and unchanged retained accounting. The cancelled-Call case
is deliberately skipped for an already replied Call because the existing cancel
API performs a consuming poll there; it would not create a Cancelled state.
Snapshots are taken after deliberate fixture mutations, so their preservation
claims describe the retry invocation, not the cancellation/policy setup.

Five transport cases cover refused and partitioned connection, backpressure,
send cancellation and an error after a completed write. The counted mock accepts
only AuthorizedRoute and AuthorizedMailboxFrame, delegates successful connection
and writes to the real InMemoryPeerTransport and introduces no raw-byte bypass.
It counts calls before fault injection. The uncertain case first invokes the
inner authorized write, verifies exact original bytes and then returns the error;
it proves possible full-write uncertainty, not partial socket-write behavior.
All cases check mapped errors and unchanged charged Reply/accounting, then clear
the fault, resend the original exact request and extract the exact original Reply
once. No mock edits registry or private authorization state. These fixtures have
no worker threads or asynchronous cleanup/join obligations.

The complete portable-manifest diff is call_send.minimum_count 2 to 6, matching
the six actual test functions. A normalized JSON comparison after applying just
that expected change is identical. Seven native OS/architecture rows, toolchain,
dependency pins, target lists, other minima, storage scope, workflows, permissions,
timeouts, resources and concurrency are unchanged. The seven hosted client rows
must run this larger target; no performance claim or native campaign consumer is
added. Cases grouped inside tests are not reported as additional test-function
counts or independent platform proofs.

Only this review file was written. JSON parsing, exact minimum/count comparison
and diff whitespace checks passed. Product hash remains unchanged. Expanded
source/test approval does not substitute for the pending execution records or
close C8, enable an automatic retry consumer, or qualify reconnect/resume/WAN.
