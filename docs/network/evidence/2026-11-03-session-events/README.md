# Bounded canonical mailbox event checkpoint

[Public report](../../mailbox-session-events-results-2026-11-03.md) ·
[Full observations and losses](measurement-review.md) ·
[Source/binary pins](execution-pins.json) · [Summary](summary.json).

The reviewed `5c41d6a` source completed 22 matched host cells (192 publications
and independent readbacks) and four native Linux/Firecracker cells (36
publications and 36 extra keyless readbacks). Every length and byte comparison
passed. Current native Combined/zstd: 62,547,228/107,299,202 logical bytes,
3.005620/4.384472 seconds timed sender CPU and 6.385582/8.652877 seconds full
lifecycle. These are local exploratory mixed sequences, not WAN or universal
speedup claims. All individual losses and timer domains remain in the report.

Exact-source release/Clippy, seven native client rows, isolated scoped coverage
and independent reviews are recorded under `validation/`, `qualification/`,
`coverage/` and `reviews/`. Coverage is 92.88% changed production and 94.50%
across the three files; branches are unmapped. Both original measurement archive
digests and all 73 members verified before deletion of only four successful
owned disks totaling 1 GiB apparent length. Instrumented executables/profiles
remain remote, separated from protected normal benchmark binaries.

The following history preserves each functional RED and qualification boundary.

RED source `8648e5dd` adds two real direct-stream regressions to product
`fdfbd2f`. Both fail with `missing authenticated Call admission`: duplicate
receipts before Reply and an older queued receipt before a second Call.

The initial bounded-reader fix distinguishes an authenticated `Pending`
observation from current verified/expired state, allows at most 16 incoming
frames, and stops after current admission plus Reply acknowledgment. Seven
focused canonical tests pass in release mode; original pending identity remains
on transport failure without a Reply. Exact byte/frame counters are asserted.
The initial test-only E0433 missing type qualification and subsequent corrected
build are retained separately in `validation/`.

The next semantic RED (`2b43f426`) injects a deterministic Reply receipt write
failure. Original request identity remains, but the old caller has already
consumed the only service-owned Reply payload (`Replied(None)`). Deferring that
consumption until all required observations/receipt transmission, with a fresh
original-deadline check, makes the regression pass. Eight focused canonical
tests pass in `ack-retention-green.txt`; the charged exact payload remains in the
canonical service after failure. Reply-wire/ack progress retention is still the
next implementation at that checkpoint, and no automatic retry is enabled.

Subsequent missing-API RED and GREEN records now establish retained receipt
disposition, cumulative event count, exact authenticated Reply wire and
Needed/InvokedUncertain/Sent acknowledgment state. Twelve focused campaign tests
passed that initial integration. A real-stream receipt-flood gate sends 17
unchanged authenticated receipts and confirms that only 16 are read, the
original Call remains, and no extra request or ACK is emitted.

An expanded state test initially refused its second Call with `Full`, correctly:
the normal signed grant allows one outstanding Call. The foreign-handle unit
fixture now uses another local service/tracker owner rather than increasing the
grant. Its original failure remains. The generic FIFO regression deliberately
uses a separately named capacity-two grant signed by a new owned test authority,
with all other limits unchanged. It reproduced observation-ready but FIFO-Pending
premature failure. Polling under fresh original-deadline checks inside the
bounded event loop now waits for the preceding Reply and returns only the exact
current payload. All **17 focused campaign tests pass** in
`fifo-and-progress-green.txt`. This synthetic two-Call fixture does not qualify
native pipelining or raise production limits. Independent source reviews record
the FIFO correction and retained-state boundaries.

Exact cached Reply replay initially failed with `Reply admission refused`.
Its canonical Duplicate disposition now sends the actual duplicate receipt,
retains current acceptance facts, and releases peer cache only once. The
additional optional registered-Reply observer contract has four focused RED
cases and 21 passing receipt tests; genuinely unknown authenticated identities
return None while known mismatch, expiry, conflict and verification errors stay
errors. Existing strict callers retain their original behavior.

The next real-stream RED sends an unsolicited authenticated Late Reply and
replays its exact ciphertext before delivering the current Reply. The caller
now sends the actual Late then Duplicate receipts without advancing current
facts. All **19 focused campaign tests pass** in `late-duplicate-green.txt`
after product commit `5ae3653`. Byte/frame counts and exact current payload are
asserted. These are functional tests, not performance measurements.

Receiver replay RED `0292ecc` reproduces the fixed-step rejection of an identical
Call while awaiting a Reply receipt. Product `976ebaf` replaces it with a bounded
event loop: current cache identity is checked before candidate admission, actual
Duplicate receipts precede byte-identical cached Reply sends, and no object work
or new Reply sequence runs on replay. A receiver latch fences any incomplete
invocation before a later read. Four real-stream qualification tests check exact
cached bytes, earlier-control isolation, unchanged capacity-one grants, eight
Reply sends with ninth refusal, retained cache and zero-read fenced reentry.
Their passing records are functional evidence, not timing measurements.

Independent review found an initial Reply path deadline gap in that first patch.
`f2b1cc8` adds one immutable Instant/Unix/owner-monotonic exchange deadline,
guarding initial/post-work sends as well as later replay and cache release.
The missing-API RED and three boundary tests remain recorded; **24 campaign tests
pass** in `exchange-deadline-green.txt` (the receiver guard extensions were run
separately afterward). Original review finding and resolution are both retained.

This is not complete C8: automatic retry and explicit resume remain.
Automatic retry stays disabled, and
the 2,048-Call guard/pipeline one remain. Independent final source review, full
qualification and performance measurements remain before delivery. No native
MVM/WAN or performance claim follows from these focused tests.
