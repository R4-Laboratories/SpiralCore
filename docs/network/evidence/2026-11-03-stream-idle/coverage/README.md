# Targeted stream-idle coverage

PASS for the required 80% changed executable-line gate at qualified source
`960e73c59d7eab62f15a16667f95e594e1cabc8b`, compared with base
`62692428cd1e9effc51c7930f7b1f34cc1519277`. Scope is only production
`crates/spiralcore-mvm-adapter/src/live.rs`; the file has no inline test suffix
to exclude. Its SHA-256 is
`29010dded67185207d677dca8bcbf10a447336c96b689e0e5308aac214ec9eac`.

| Scope | Hit / mapped executable lines | Coverage |
| --- | ---: | ---: |
| Changed production lines | 96 / 97 | 98.97% |
| Entire production file | 673 / 733 | 91.81% |

The union comes from four fresh, separately exported instrumented test binaries:
live library 57, campaign 9, adapter_live 11 and service_tcp 11: **88 passed**,
zero failed/ignored/filtered. Test and export stderr are empty for all four.
This is mapped line coverage, not whole-crate coverage. No branch records were
mapped (zero BRDA); branch coverage is **NA**, with no branch or MC/DC claim.

`uncovered-changed-lines.tsv` retains the one unhit changed line (186), and
`uncovered-production-lines.tsv` retains all 60 unhit production lines.
`summary.tsv`, `summary.json`, `focused-lines.tsv` and `branch-summary.tsv`
provide the complete compact accounting. `changed-source.diff` records the
exact source comparison.

The existing Wick target was reused with Rust 1.97.1, locked release builds,
two jobs and instrumentation selective to adapter/live. Only those two owned
packages were cleaned. Each fresh test executable was copied outside Cargo's
target, compared, hashed and protected mode 500 before execution; profiles and
exports were isolated by executable. See `collect.sh`, `derive.sh`,
`finalize.sh`, `coverage-wrapper.sh`, `executed-binaries.json`,
`binaries-retained.sha256` and `per-binary-tests.txt`.

All **285 source entries** verified before and after collection. The protected
normal host, guest and broker_objects binary hashes remained unchanged; see
`verification.txt`, `source-verification*.txt` and
`normal-binaries-{before,after}.sha256`. Normal performance completed before
instrumentation. These test timings are not performance measurements.

Raw profiles, profdata, LCOV, build JSON and executed instrumented binaries are
retained only on Wick under
`/var/tmp/spiralcore-session-idle-20261103/coverage`; the local packet contains
compact receipts and scripts. `remote-coverage-artifacts.sha256` and
`remote-build-artifacts.sha256` identify remote raw custody. The transferred
compact archive SHA-256 is
`477f5f8478716a49c537c14b29c61294669a2d3b1bc5a4191de419b6ff67bf66`.
No global process arguments or shell history were captured.
