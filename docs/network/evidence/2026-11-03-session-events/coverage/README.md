# Frozen session-events coverage

Qualified source: `5c41d6ac2d9b40e6eead62e201d5e9b020f96019`.
Diff base: `231ab32103f58e52c206d6e5917d883de4e3a928`.

**PASS: 287/309 changed executable production lines covered (92.88%)**, exceeding
the 80% gate. Across all production lines in the three reviewed files,
coverage is **1,495/1,582 (94.50%)**. These are distinct scopes; neither is
repository coverage. The canonical/session embedded test suffixes are excluded.
All 22 uncovered changed lines remain listed. Branch mapping is **NA**, with no
mapped branches in these exports; no branch or MC/DC percentage is claimed.

One selective run instruments only adapter/live in the existing Wick target.
Four freshly built test binaries were retained mode 500 outside Cargo before
execution. Each binary's profiles were merged and exported separately to avoid
object/profile mismatches. All **91 tests** passed: receipt21, TCP4, live library57,
campaign9. Test/export stderr is empty and compiler warning count is zero.
The source manifest verifies **284/284 before and after**. Protected normal
host/guest/broker binary hashes are unchanged; this coverage run contributes no
performance measurement.

- [Machine-readable statistics](stats.json), [scope table](summary.tsv),
  [branch availability](branch-summary.tsv).
- [Changed unhit lines](uncovered-changed-lines.tsv),
  [broader unhit production lines](uncovered-production-lines.tsv),
  [union line records](focused-lines.tsv), [test cutoffs](production-cutoffs.tsv).
- [Executed binary identities](executed-binaries.json),
  [retained hashes](binaries-retained.sha256), [per-binary results](per-binary-tests.txt),
  [verification](verification.txt), [source hashes](source-files.sha256).
- [Collection](collect.sh), [derivation](derive.sh), [custody finalization](finalize.sh),
  [scoped procedural gate](procedural-gate.md).

Profiles, profdata, full LCOV/build JSON and the actual executed instrumented
binaries remain **remote only** at
`/var/tmp/spiralcore-session-events-20261103/coverage`. Local files are compact
receipts/scripts/statistics/checksums. The matching compact archive checksum is
`6561bbe307d0dae7dd2a70c558c8f0092bd8426caa4d72fa694fcd06dd49fa18`.
