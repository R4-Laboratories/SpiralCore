# Restored FIN barrier: independent code/Rust/security approval

PASS for exact duplex.rs SHA b5df190a58995ee6ff6291b975b30c3887714132faa46c2cf0a935cfb5f45e2c, conditional on fresh exact-source native matrix before merge. This supersedes the missed synchronization effect recorded in delivery-fence-review-correction.md; the original review/failure remain retained.

The original Tx mutex is restored before observing local_closed and Pending in the post-FIN truncation branch. half_close holds that same mutex through FIN flush and closed-flag publication. exchange releases Pending before acquiring Tx; this branch preserves original Tx-to-Pending order. Explicit drop(tx) precedes complete(), preventing self-deadlock when the helper snapshots Tx counters. Fatal monotonic delivery fencing and ordinary buffered-drain semantics remain unchanged. No wire/codec/capacity/deadline change.

Author supplied fresh34-test and strict Clippy successes and34-test coverage from9 protected objects,1954/2145 mapped lines91.10%,159 source checks before/after. These are supplied executed receipts, not an independent rerun. The previous failed instrumented attempt and earlier34 ordinary pass remain separate. Native MVM/WAN/installed budget qualification remains open.
