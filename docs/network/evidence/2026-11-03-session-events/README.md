# C8 endpoint work in progress

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
next implementation, and no automatic retry is enabled.

This is not complete C8: receipt/Reply/ack progress still needs retained bounded
state, earlier/duplicate Reply handling and both-peer cached Reply recovery.
Automatic retry stays disabled, the receiver choreography is unchanged, and
the 2,048-Call guard/pipeline one remain. Independent final source review, full
qualification and performance measurements remain before delivery. No native
MVM/WAN or performance claim follows from these focused tests.
