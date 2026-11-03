# Focused line coverage at product revision 7531985

Coverage source: `753198531d4f3ef87e8a79c63d44f4f081ce4077`.
Changed-line baseline: `b154fb75f6e38e5260853d67d08e7dcdac78b402`.
The remote exported source passed all 272 retained crate-file hashes before the
instrumented build and again before the resumed test execution. Normal binaries
were protected separately and supplied all performance/native measurements;
instrumented execution supplies no performance evidence.

Three fresh release test binaries from the successful existing build were run
serially, with one profile directory, merged profdata and warning-free LCOV
export per binary. All **40 tests passed**: `host_budget` 6, live library 25 and
`campaign` 9. LLVM is `22.1.6-rust-1.97.1-stable`. `resume.stderr` and each remote
`export.stderr` were empty. Raw profiles, LCOV exports and build artifacts remain
on Wick; the Mac packet retains compact line hits and execution identities.

| Declared scope | Covered executable lines | Executable lines | Coverage |
|---|---:|---:|---:|
| New `host_budget.rs` production code | 179 | 198 | **90.40%** |
| Changed production lines, all three reviewed files | 315 | 362 | **87.02%** |
| Changed `campaign/canonical.rs` production lines | 35 | 38 | 92.11% |
| Changed `orchestrator.rs` production lines | 101 | 126 | 80.16% |
| All production lines in the three reviewed files | 706 | 1,092 | **64.65%** |
| All `orchestrator.rs` production lines | 210 | 552 | 38.04% |

The new-module and changed-production scopes exceed the declared 80% gate.
The broader three-file production scope remains below 80%; unchanged native
activation paths are not exercised by these unit/integration binaries. These
are line scopes, not branch, whole-live-crate or whole-repository coverage.
Native execution used normal binaries and does not contribute instrumented hits.

`derive.sh` reproduces the table from `line-hits.tsv` and `changed-source.diff`.
It intersects added/changed lines from the exact Git diff with LLVM `DA` lines,
then excludes each file's complete test-module suffix. At this exact source,
the first `#[cfg(test)]` lines are 328 in `host_budget.rs`, 432 in
`campaign/canonical.rs` and 683 in `orchestrator.rs`. Denominators are executable
LLVM lines, not physical source lines. Repeated exports merge by maximum line
hit count; covered means a positive hit. `uncovered-changed-lines.tsv` retains
every zero-hit changed executable production line.

## Parser recovery and execution custody

The initial build succeeded. The next command failed because `jq -rs` had
already parsed/slurped JSON into an array, while its filter called `split` on
that array. `coverage.sh` now uses `jq -s` and selects directly from `.[]`.
`resume.sh` records the actual recovery commands: source verification, corrected
artifact selection, the three existing binaries, isolated exports and line-hit
derivation. It contains no Cargo, clean or rebuild command. The original parser
used `jq -rs` with a filter beginning `[split("\n")[] | select(length>0) |
fromjson | ...]`; the initial parser stderr was not separately retained by this
recovery. No original test failure or performance loss is hidden by this parsing
correction.

Compact artifact identities:

- `line-hits.tsv`: `37fb8b7057eb8b6addd75f383d320375b4bd69455fa909265011984a7ec8bd80`
- `resume.sh`: `5f6625dfbcad575fb6c427050422c25093155f57d87d42f44af66995e0d510a2`
- Test executable hashes and source target paths: `binaries.sha256` and
  `binaries.json`; actual test totals: `resume.stdout`.

No payload collection, VM disk or raw profile was copied to the Mac. No service,
compiler installation, guest allocation or normal-binary performance run was
started by the recovery.
