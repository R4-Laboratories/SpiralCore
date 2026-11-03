# Registered Call retry targeted coverage

**PASS** for the required 80% changed executable-line gate. Qualified crate
source: `4d34c6a722ec5223bf3e25431ab0b73e11f39d54`; comparison base:
`a8a4a9a8daba67c00447a067b18632ed515d95b2`. The sole changed production file
is `crates/spiralcore-mvm-adapter/src/service.rs`, SHA-256
`5563e59811266047b582651aa6e08682ff3c368d417c2aabdbee8930a1217794`.

| Mapped executable-line scope | Covered / total | Coverage |
| --- | ---: | ---: |
| All changed production lines, including moved validation | 31 / 31 | 100.00% |
| Entire production `service.rs` | 2,010 / 2,169 | 92.67% |

The denominator includes every new-side diff line with a mapped DA record;
it does not discard moved validator lines or restrict the count to the new API.
The file contains production only, with no test suffix excluded. All **159**
uncovered whole-file mapped lines are in `uncovered-production-lines.tsv`;
`uncovered-changed-lines.tsv` is empty. See `changed-source.diff`,
`focused-lines.tsv`, `summary.tsv` and `summary.json` for full accounting.
No BRDA records were mapped, so branch coverage is **NA**; no branch or MC/DC
percentage is asserted. This is scoped file coverage, not whole-project coverage.

One fresh selective run executed **125 tests across 19 library/integration
binaries**, zero failed/ignored/filtered. The normal release `--all-targets`
gate passed 127 tests; its two example tests are outside the coverage
`--lib --tests` selection. Each instrumented executable was copied outside
Cargo's target, compared, hashed and protected mode 500 **before** execution.
Each had isolated raw profiles, profdata and its own LLVM export against the
same retained object. All test/export stderr files are empty; compiler warning
count is zero. `per-binary-tests.txt`, `test-counts.json`,
`executed-binaries.json` and retained-binary checksums bind the actual run.

Only the adapter crate was instrumented through the selective wrapper; its
dependencies were not instrumented. The existing Wick target was reused with
Rust 1.97.1, locked release, two jobs and incremental compilation disabled.
Only the owned adapter package was cleaned. All **285 source entries** verified
before and after collection. No normal engine build or performance campaign
was run for this unused additive API, and coverage timings are not performance
measurements.

Collection/export/derivation scripts and compact receipts are local. Raw build
JSON, profiles, profdata, LCOV and the exact executed binaries remain only on
Wick under `/var/tmp/spiralcore-call-retry-20261103/coverage`.
`remote-coverage-artifacts.sha256` and `remote-build-artifacts.sha256` retain
their identities. No global process arguments or shell history were captured.

This packet remains untracked for the root-owned results-only publication
after source PR #64; the coverage task did not commit or push it. The scoped
[procedural gate](procedural-gate.md) permits final parallel confirmations,
with hosted qualification and merge recorded separately by root.
