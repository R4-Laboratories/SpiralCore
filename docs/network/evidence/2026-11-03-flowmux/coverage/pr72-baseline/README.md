# Baseline33 selective coverage

Frozen PR72 head57c09d92, tree38fc292e, was exported with exactly four dependency
crates and its frozen contracts. All159 input hashes match before/after.
Only spiralcore-mvm-flowmux was instrumented; dependency/crypto/storage/core
code was not included in this coverage scope. Nine fresh release objects were
copied outside Cargo and protected mode0500 before execution. All33 tests pass,
with0 failed/ignored/filtered; each object's profiles/export were isolated and
export stderr is empty. Line hits union by maximum across objects, not sum.

Mapped production scope is eight new files versus main base7e747983, excluding
inline cfg(test) suffixes (tcp606/tls639). Result:1928/2116 executable mapped
lines,91.12%. Because the crate is new in that base diff, changed and whole
production scope coincide. All188 uncovered mapped lines remain listed.
LLVM supplied no mapped branches: branch coverage is NA, not100% or proof of
all error conditions. The compact production-only LCOV preserves line evidence;
raw per-object LCOV/profiles/binaries remain on Wick under the owned PR72 root.

This is baseline evidence only. The source's native33 tests and coverage missed
a fatal buffered-delivery bug. The successor and its failed intermediate
instrumented run remain separate; this percentage cannot be applied to them.
No timing/performance, native-MVM, shared-budget, Windows-coverage or WAN claim.
