# Isolated duplex relay coverage

Executed on Wick against exact product source
`f964c034c8ac5d32ef29aa4dfab5771b6867f534`. The 278-file source manifest
verified before and after execution. `source-files.sha256` pins the three
reviewed production files; `source-manifest.sha256` pins the complete manifest.

All **61 tests passed** in exactly four freshly emitted release test binaries:
`spiralcore_mvm_live` lib 38, `campaign` 9, `adapter_live` 11, and `service_tcp` 3.
Each binary used a separate raw-profile directory, merged profile and isolated
LCOV export. All four test stderr files and all four export stderr files were
zero bytes. `binaries.json`, `binaries.sha256` and `per-binary-tests.txt` retain
the executable identities and test results. Rust 1.97.1 / LLVM 22.1.6 were used.

| Scope: executable production lines | Covered / total | Percent |
|---|---:|---:|
| Changed adapter `live.rs` | 29 / 30 | 96.67% |
| New `orchestrator/duplex.rs` module | 157 / 174 | 90.23% |
| Combined changed production | 186 / 204 | 91.18% |
| EOF helper `live.rs:159-195` | 26 / 29 | 89.66% |
| All production in the three reviewed files | 856 / 1,273 | 67.24% |

The changed-production targets and the new module pass the 80% line gate. The
parent `orchestrator.rs` plumbing adds no executable production lines: its
new forwarding wrapper is test-only and is excluded. No exception removes the
18 uncovered changed executable lines listed in `uncovered-changed-lines.tsv`.

The broader complete production-file denominators are `live.rs` 582/646
(90.09%), `duplex.rs` 157/174 (90.23%), and `orchestrator.rs` 117/453 (25.83%).
The latter includes native lifecycle paths not executed by these instrumented
test binaries. These are three-file executable LCOV DA-line denominators,
not whole-crate or semantic-path coverage. The root agent's normal native runs
are separate evidence and were not combined into this coverage result.

The LLVM export contains **zero mapped branch counters** for the EOF helper
(and `branch-hits.tsv` is empty). Consequently branch coverage is **unavailable**,
as recorded in `branch-summary.tsv`; this is not a zero-of-zero 100% result.
The line result and executed EOF/truncation tests do not establish compiler
branch coverage, startup-failure/panic injection or a real 30-second blocked-write
deadline test. The blocked-output regression instead proves failure wakeup after
an output with verified partial transmission is closed.

`collect.sh` retains the exact commands. It used the existing selective
adapter/live instrumentation wrapper, two Cargo jobs, release/locked builds and
the existing `/var/tmp/spiralcore-mvm-service-20261102/target`. Before cleaning,
the process and lock-holder check found no Cargo/rustc job or SpiralCore target
lock holder. Only the authorized adapter/live release packages were cleaned.
No other lab, build target, product source, VM or native payload was modified.

Coverage began only after the root agent reported all four registered native
measurement cells complete and explicitly released the build target. Protected
normal host/guest binaries were mode 500 and matched these hashes before and
after coverage:

```text
1b3e8b00166ca0470b79ddbcb07d30be8fa4234efaffc8d88500dc7554834d50  host
27604d73ad1b7a4ae570d78404efef994ad59990eeab747e1bf14d7631e2204f  guest
```

Instrumented test timing supplies no performance evidence. This run establishes
neither full endpoint C8 retry/reconciliation nor WAN, retention or fleet
acceptance. It starts no native guest and makes no CPU isolation claim.

Profiles, merged profiles, build JSON and raw LCOV remain on Wick under
`/var/tmp/spiralcore-duplex-20261103/coverage`; their hashes are retained in the
two `remote-*-artifacts.sha256` files. Only compact receipts, line hits and
derivation files were copied to the Mac.

To reproduce the local derivation from the repository root:

```sh
git diff --unified=0 7c480a9 f964c03 -- crates/spiralcore-mvm-adapter/src/live.rs crates/spiralcore-mvm-live/src/orchestrator.rs crates/spiralcore-mvm-live/src/orchestrator/duplex.rs > docs/network/evidence/2026-11-03-duplex-relay/coverage/changed-source.diff
bash docs/network/evidence/2026-11-03-duplex-relay/coverage/derive.sh
```

`derive.sh` excludes the parent's `cfg(test)` helper at 299..319 and test suffix
starting at 592. Hit counts are the maximum per source line across isolated
exports; positive-versus-zero coverage is their union. The maximum is not a
sum of execution counts. `finalize.sh` records the remote artifact hashes and
compact source/binary verification receipt after execution.
