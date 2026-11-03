# Prepared selective reconciliation coverage gate

**PASS — executed at frozen source26dc93a1 after normal release and performance.**
The run passes239 tests in29 fresh objects with zero failed, ignored or filtered.
Changed mapped executable lines are350/377 (92.84%); all five mapped production
files are2,155/2,308 (93.37%). Branch mapping is NA. Every uncovered line remains
recorded. Source checks249/249 pass before/after; normal binary hash/mode stay
unchanged; compiler warnings and per-binary test/export stderr are zero.
Collection, derivation and finalization exit0. Root verified original compact
archive SHA5dd5a75294a65540d9aa4a0dc90fd7ba35e14dc024f55bf0dc00d151229b81b6.
The following retains the prepared procedure and scope. No run was made during
script preparation; root staged and executed it afterward. The active local roster lists the
harness-optimizer role; this is the same narrow selective-coverage procedure as
the preceding registered-Call retry and stream-idle packets.

The new-side source diff is pinned to base
`e984d2d64cdf0483d50d6d59319276c85e7df644` through
`26dc93a17146693af43529c09db78c7906d47b37`. Its five-product-file diff is
identical to the initially requested `afe3d0a2` diff. It includes all mapped executable
new/moved production lines in five files:

| File | Production line range |
| --- | --- |
| adapter `src/live.rs` | Entire file |
| live `src/campaign.rs` | Entire file |
| live `src/campaign/canonical.rs` | Before first test attribute at 866 |
| live `src/campaign/session.rs` | Before test attribute at 414 |
| live `src/campaign/reconciliation.rs` | Before test attribute at 83 |

The collector verifies these boundaries against the actual frozen source.
Orchestrator changes are a test-only relay re-export and are explicitly outside
the production metric. Other files may be instrumented because crate-level
instrumentation is selective, but the reported metric is restricted to this
table. No other exclusions or whole-project coverage claim are implied.

Root supplied the final 249-entry source manifest, SHA-256
`93ca8300a46ed721fa06a87c52718d642c7dd1a52964eb80065816fbb63f7f8b`.
The collector checks that identity and every source before and after. The
normal protected `bin/broker_objects` binary remains mode 500 and SHA-256
`fe7937f2fb348ff0a3c517cf856f2ad132883346a8aa05020e6dbd0e1d62b6d7`;
its before/after hashes are compared without rebuilding or running it.

The existing target `/var/tmp/spiralcore-mvm-service-20261102/target` is reused,
Rust 1.97.1 locked release with two jobs and incremental compilation disabled.
Only owned adapter/live packages are cleaned and instrumented. Adapter selection
is `live,conformance,hardware-sha256 --lib --tests`; live selection is
`linux-host --lib --tests`. Every fresh executable is copied outside Cargo's
target, compared/hashed and protected mode 500 before any is executed.
Package-qualified labels prevent target-name collisions. Each binary receives
isolated profiles/profdata and its own LLVM export against that retained object.
Actual binary/test counts are derived from receipts; none are assumed here.
The normal-observation environment variable is unset during coverage so that
instrumented CPU/wall values cannot overwrite normal runtime evidence.

DA hit records are unioned by file/line using the maximum hit count. The changed
line gate is at least 80% over the five-file union; per-file changed and entire
production scopes are separately reported, together with every uncovered mapped
line. An aggregate PASS must not be described as every file independently meeting
80%. BRDA records are handled separately; absent mapping is **NA**, never 100%
or a branch/MC/DC claim. Even a below-threshold derivation retains custody and
uncovered-line receipts. Failed builds/tests stop collection without erasing
earlier logs; do not rerun into an existing coverage directory or hide failures.

Root execution instructions, after explicit release:

1. Pin current crate sources at
   `/var/tmp/spiralcore-reconciliation-20261103/source` and place the exact
   `qualified-source-sha256.txt` one level above it. A later fixture-only lint
   correction is included at the final source pin above. Any later source
   change requires updating the manifest identity and reviewing the five-file
   diff and test cutoffs before collection.
2. Copy `collect.sh`, `derive.sh`, `finalize.sh`, `run.sh`, `compact.sh`,
   `coverage-wrapper.sh`, `production-cutoffs.tsv`, `changed-source.diff` and
   `source-pins.txt` to that root's `coverage-scripts/` directory. Keep the
   output `coverage/` and `coverage-run.txt` absent for the first invocation.
3. Run `bash /var/tmp/spiralcore-reconciliation-20261103/coverage-scripts/run.sh`.
   The launcher records each exit. It copies scripts into remote custody and
   runs collect, derive and finalize sequentially. Inspect `coverage-run.txt`,
   `test-counts.json`, `summary.tsv`, `branch-summary.tsv` and all exit receipts.
   Source manifests verify before/after; only PID/comm names are checked for
   active Cargo/rustc, never global arguments or history.
4. Run `bash /var/tmp/spiralcore-reconciliation-20261103/coverage-scripts/compact.sh`.
   Copy the resulting compact archive and verify its printed SHA-256 before
   extracting locally. It contains receipts/scripts/test logs and all uncovered
   lines. Raw build JSON, instrumented binaries, raw profiles, profdata and LCOV
   remain on Wick, identified by retained checksums. No payload store, VM disk,
   global process dump or new Mac target is copied.

Do not infer performance from coverage timings. The 239 tests overlap normal release/adapter scopes; this is line coverage, not branch/MC/DC or whole-project coverage.
