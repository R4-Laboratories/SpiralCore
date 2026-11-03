# Final-source duplex relay coverage

Executed on Wick against exact qualification source
`fdfbd2f72276589e1c1c8ad8d4908b97f94e9647`. The new 278-file
`../qualification-source-sha256.txt` verified before and after execution.
Only the tracked TCP test fixture was synced into the retained remote source;
the original `source-sha256.txt` and `final-source-sha256.txt` were preserved.
The untracked, unwired next-peer test was excluded from this tracked manifest.
`source-files.sha256` pins the reviewed production files and exact final TCP
fixture; `source-manifest.sha256` pins the qualification manifest.

All **62 tests passed** in exactly four freshly emitted release test binaries:
`spiralcore_mvm_live` lib 38, `campaign` 9, `adapter_live` 11, and `service_tcp` 4.
Each binary had a separate raw-profile directory, merged profile and isolated
LCOV export. All four test stderr files and all four export stderr files were
zero bytes. `binaries.json`, `binaries.sha256` and `per-binary-tests.txt` retain
the executable identities and results. Rust 1.97.1 / LLVM 22.1.6 were used.

| Scope: executable production lines | Covered / total | Percent |
|---|---:|---:|
| Changed adapter `live.rs` | 30 / 36 | 83.33% |
| New `orchestrator/duplex.rs` module | 157 / 174 | 90.23% |
| Combined changed production | 187 / 210 | 89.05% |
| EOF helper `live.rs:163-199` | 26 / 29 | 89.66% |
| All production in the three reviewed files | 856 / 1,277 | 67.03% |

Both changed-production targets and the new module pass the 80% line gate. The
parent adds no executable production lines: its forwarding wrapper is test-only
and excluded. All **23 uncovered changed executable lines** remain listed in
`uncovered-changed-lines.tsv`, including the explicit Darwin shutdown fallback
which these Linux tests did not execute. There is no denominator exception for
that fallback and no transfer of macOS execution into Linux hit counts.

The broader complete production-file denominators are adapter `live.rs` 582/650
(89.54%), `duplex.rs` 157/174 (90.23%), and `orchestrator.rs` 117/453 (25.83%).
The latter includes native lifecycle paths not executed by these instrumented
tests. These are executable LCOV DA-line denominators for three files, not
whole-crate or semantic-path coverage. Normal native runs remain separate.

The export contains **zero mapped branch counters** for the EOF helper;
`branch-hits.tsv` is empty. Branch coverage is consequently **unavailable**, as
recorded in `branch-summary.tsv`, rather than zero-of-zero 100%. These line
results do not establish startup/panic injection or a real 30-second blocked
write deadline test. The blocked-output test proves failure wakeup after a
partially transmitted output is closed.

The two final fixture changes move probe read-timeout setup before terminal
socket actions. The test assertions and production bytes are unchanged from
`62cb6042`; `fixture-only.diff` records the exact delta. Final normal host/guest
binaries remain from that production-equivalent source. Their mode-500 copies
in `bin-final` matched these hashes before and after coverage:

```text
0b851e9df95cdfb1f7f999035b4e551707fd29ca3744d532041cb2bcadc991e6  host
c9fba260cbbbb16fb18d9b9bf7e0f87df26ceacdb8d5cab037699094c70b5431  guest
```

Coverage began after the root agent reported all six native measurements done
and explicitly released the target. Preflight found no Cargo/rustc or target
lock holder. `collect.sh` retains the exact selective-wrapper build/test/export
commands: two jobs, release/locked, existing shared target, and package-only
clean of `spiralcore-mvm-live` and `spiralcore-mvm-adapter`. No product, VM,
native payload, protected normal binary or other lab was modified. Instrumented
timing supplies no performance evidence; no CPU isolation claim is made.

## Binary custody

The initial `coverage/` packet and its observed exports, profiles, hashes and
derivation remain preserved. Before this final clean, its four shared-target
executable paths still existed but all four hashes already differed from the
initial recorded identities following intervening normal builds. They cannot be
represented as retained original instrumented executables. The comparison is
explicit in `initial-binary-comparison.txt`. Their then-current pre-clean bytes
were copied on Wick and pinned in `pre-clean-binaries.sha256`.

The request to protect the final instrumented binaries arrived after their
execution and exports completed. Before releasing the target, all four target
files were checked against the hashes recorded immediately after each isolated
export, copied into owned `coverage-final/bin`, made mode 500, and byte-compared
with their source files. All four preserved hashes verified. The exact commands
are in `finalize.sh`; `binary-preservation-verification.txt` and
`binaries-retained.sha256` retain the checks. Thus this final packet retains the
actual executed bytes independently of subsequent shared-target reuse, with
copy-after-execution timing explicitly disclosed.

Profiles, merged profiles, raw LCOV, build JSON and protected executable copies
remain only on Wick under `/var/tmp/spiralcore-duplex-20261103/coverage-final`.
Only compact receipts, hashes, line hits and derivation were copied to the Mac.
The two `remote-*-artifacts.sha256` files pin the retained raw artifacts. This
run does not establish full C8 endpoint retry, WAN, retention or fleet gates.

To reproduce the local derivation from the repository root:

```sh
git diff --unified=0 7c480a9 fdfbd2f -- crates/spiralcore-mvm-adapter/src/live.rs crates/spiralcore-mvm-live/src/orchestrator.rs crates/spiralcore-mvm-live/src/orchestrator/duplex.rs > docs/network/evidence/2026-11-03-duplex-relay/coverage-final/changed-source.diff
bash docs/network/evidence/2026-11-03-duplex-relay/coverage-final/derive.sh
```

`derive.sh` excludes the parent's `cfg(test)` helper at 299..319 and test suffix
starting at 592. Per-line hit counts are maxima across isolated exports;
positive-versus-zero coverage is their union, not an execution-count sum.
