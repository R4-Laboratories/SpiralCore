# Isolated C7 coverage verification

Executed on Wick against exact product source
`91baf28efa56da94336d413ca8e912266079f654`. The 276-file source manifest
verified before and after execution. The three reviewed production source hashes
are in `source-files.sha256`; the parent manifest is pinned in
`source-manifest.sha256`.

All **46 tests passed** in exactly four freshly emitted release test binaries:
`call_send` 2, `receipt_send` 3, `campaign` 9, and `spiralcore_mvm_live` lib 32.
Each binary had a separate raw profile directory, merged profile and isolated
LCOV export. All four test stderr files and all four export stderr files were
zero bytes. `binaries.json`, `binaries.sha256` and `per-binary-tests.txt` retain
the identities and test results. Rust 1.97.1 and LLVM 22.1.6 were used.

| Scoped executable production lines | Covered / total | Percent |
|---|---:|---:|
| Changed `service.rs` | 60 / 65 | 92.31% |
| Changed `canonical.rs` | 54 / 58 | 93.10% |
| New `session.rs` module | 137 / 152 | 90.13% |
| Combined changed production lines | 251 / 275 | 91.27% |
| All instrumented production lines in these three files | 2,243 / 2,669 | 84.04% |

Each changed-file target and the new module exceeds the 80% line gate. These
are executable LCOV DA-line denominators, with the `cfg(test)` suffix excluded
from `canonical.rs` and `session.rs`. They are not whole-crate, branch or
semantic-path coverage. `summary.tsv` reports each complete production-file
denominator, including `service.rs` 1,762/2,148 (82.03%), `canonical.rs` 344/369
(93.22%), and `session.rs` 137/152 (90.13%). `uncovered-changed-lines.tsv` retains
all 24 uncovered changed executable lines; no exception removes these lines
from a denominator.

`collect.sh` contains the exact remote build, test and export commands. It used
the existing selective adapter/live wrapper, two build jobs, release/locked
Cargo and the existing owned shared target. Before the package-only clean, live
process argv/cwd checks showed active unrelated UOR Cargo/rustc jobs under
`/home/callard/projects/codex-playground/uor-r4/.worktrees/deepseek-discrete-k4-corpus`;
their rustc output paths were separate UOR targets. No SpiralCore command or
shared-target lock holder was present. Only `spiralcore-mvm-live` and
`spiralcore-mvm-adapter` release package artifacts were cleaned. UOR work was
left untouched. This coexistence does not establish CPU isolation.

Protected normal host/guest/object binaries in
`/var/tmp/spiralcore-session-20261103/bin` had identical hashes before and after
coverage. Coverage instrumentation supplied no performance result and started
no native guest. Product sources were not changed. C8/C9, actual soak, >2,048
Calls, WAN and remaining platform gates are not established by this run.

Profiles, merged profiles, build JSON and raw LCOV remain on Wick under
`/var/tmp/spiralcore-session-20261103/coverage`. Their hashes are retained in the
two `remote-*-artifacts.sha256` files. Only compact receipts, line hits and
derivation files were copied to the Mac.

Reproduce the local derivation from the repository root:

```sh
git diff --unified=0 cc94f31 91baf28 -- crates/spiralcore-mvm-adapter/src/service.rs crates/spiralcore-mvm-live/src/campaign/canonical.rs crates/spiralcore-mvm-live/src/campaign/session.rs > docs/network/evidence/2026-11-03-persistent-session/coverage/changed-source.diff
bash docs/network/evidence/2026-11-03-persistent-session/coverage/derive.sh
```

`derive.sh` joins that new-side diff with `line-hits.tsv`. Hit counts are the
maximum per source line across isolated exports; positive-versus-zero coverage
is their union. A maximum is not an execution-count total. `finalize.sh` records
the remote artifact hashes and compact verification receipt after the run.
