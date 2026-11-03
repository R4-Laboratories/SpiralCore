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

This is not complete C8: receipt/Reply/ack progress still needs retained bounded
state, earlier/duplicate Reply handling and both-peer cached Reply recovery.
Automatic retry stays disabled, the receiver choreography is unchanged, and
the 2,048-Call guard/pipeline one remain. Independent final source review, full
qualification and performance measurements remain before delivery. No native
MVM/WAN or performance claim follows from these focused tests.
