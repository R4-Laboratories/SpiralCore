# Restored delivery-fence successor coverage

Final candidate`1e69a99d27691d1db9921568d834a98814defc81`, tree
`94426f89602dc2714842198f9bc4dd7fa4fa8d8b`, exactly matches all159 source/contract
inputs in the executed isolated snapshot. Its complete selected archive diff is
empty. This source contains the reviewed delivery-fence repair and restored
original FIN writer synchronization; it excludes later TLS/Mio/Small/Ordered WIP.

Nine fresh selective instrumented release objects were protected outside Cargo
BEFORE execution, with separate per-object profiles and exports. All34 tests
pass;0 failed/ignored/filtered, empty test/export stderr. Only the pure component
was instrumented. All159 source inputs and protected object identities verify
before/after; the9 normal objects also remain protected and unchanged. After
export, only the owned pure package's instrumented Cargo cache was removed
(29.1MiB); protected normal/instrumented binaries, raw profiles and LCOV remain
on Wick. No new target or VM was created.

Changed/new whole-component production mapping versus original main7e747983:
**1954/2145 lines,91.10%** across8 files. The narrower repair-only new-side diff
versus57c09d92 is **48/49 lines,97.96%**. Its sole uncovered mapped new line is
JoinGuard's missing-child fallback (duplex441), not an executed failure proof.
All191 whole-production uncovered mapped lines remain listed. LLVM exported no
mapped branch counters, so branch coverage is explicitly NA. Coverage concerns
mapped executable lines, not every syntactic source line or every failure branch.
Line hits union by max across objects; profiles from the earlier failed attempt
are excluded. Compact LCOV and hit tables are retained; raw data stays remote.

The original33 source covered91.12%; the first34 instrumented attempt FAILED
Combined half-close when a refactor removed a Tx mutex ordering barrier. That
failure and superseded review remain in the cancellation packet. The original
barrier is restored before inspecting local_closed/pending and released before
complete() reacquires Tx. Normal34 and strict all-target Clippy pass on this
restored source; final independent Root reviews are retained alongside the fix.

These are Linux component coverage/functional gates. Fresh seven-platform
34-test CI/custody remains required before merge; old33 native artifacts cannot
substitute. No native-MVM activation, aggregate-installed-budget, sustained/WAN,
energy or whole-goal completion claim follows.
