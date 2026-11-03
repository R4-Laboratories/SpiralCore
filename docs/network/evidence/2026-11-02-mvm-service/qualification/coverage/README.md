# Final source coverage and independent review

Measured source: `0cf256f2e691849e5c9c019ac2fda7eb97287938`, with canonical MVM
contribution dependency `ae12e9deb41cc9a3264196ce22ebc72d4042883d`.
Changed lines are additions relative to PR base
`2e941e30552ee9bcc8714f339d52f0964eee8e3a`.

The final, corrected measurement passes the 80% aggregate target in the declared
scope: **5,758/6,672 compiled production lines (86.30%)**. Changed production
lines independently measure **3,414/3,711 (92.00%)**. Including inline test
blocks, all compiled source lines measure **6,519/7,434 (87.69%)**, with changed
source lines **3,802/4,100 (92.73%)**.

This scope includes all twenty compiled source files in the two owned
adapter/live crates, including legacy code, fixtures, main, host and
orchestrator. Production removes only the trailing inline test-module blocks
(`#[cfg(test)] mod tests`) listed in `test-cutoffs.tsv`; no source files are excluded.
Linux-only backend bodies are not compiled on this macOS arm64 run. The compiled
`linux_host.rs` coverage describes only its portable helpers. This is not a
whole-repository, native Linux backend or cross-platform coverage gate.

| Module | Covered/count distinct source lines | Coverage |
|---|---:|---:|
| Shared service | 1,758/1,930 | 91.09% |
| Signed receipts/tracker | 719/759 | 94.73% |
| Issuance policy | 72/72 | 100.00% |
| FIFO ordering | 304/306 | 99.35% |
| Object bridge | 90/100 | 90.00% |
| Prepared sender | 181/189 | 95.77% |
| Native canonical consumer | 289/314 | 92.04% |
| Historical adapter live transport | 565/620 | 91.13% |
| Native host | 353/479 | 73.70% |
| Native orchestrator | 222/564 | 39.36% |

The aggregate pass does not mean every module passes 80%. Host and orchestrator
lack instrumented VM/process supervision coverage here. Existing native guest
measurements remain separate evidence and are not counted as LLVM coverage.
Uncovered lifecycle cleanup and refusal branches remain useful future coverage
work. No tests were added, skipped or changed to improve this metric.

## Executed commands and corrected collection

The two release all-target commands passed **82 adapter tests** and **48 live
tests**, with no failures, ignored or filtered tests. Their twenty-one exact
executable artifacts were then each run separately with the same single-thread
setting: **130 passed**, zero failures/ignored/filtered. `per-binary-tests.txt`
retains each result. This includes the eight campaign tests and large mixed
session. Instrumented durations are not performance measurements.

```sh
export CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target
export CARGO_BUILD_JOBS=2
export CARGO_INCREMENTAL=0
export RUSTC_WRAPPER=/Users/casey.allard/spiralcore/local/mvm-service-20261102/coverage-final/coverage-wrapper-all-owned.sh
export LLVM_PROFILE_FILE=/Users/casey.allard/spiralcore/local/mvm-service-20261102/coverage-final/profiles-v3/%m-%p.profraw

/Users/casey.allard/.cargo/bin/cargo +1.97.1 test --release --locked \
  --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml \
  --features live,conformance,hardware-sha256 --all-targets \
  --message-format=json -- --test-threads=1

/Users/casey.allard/.cargo/bin/cargo +1.97.1 test --release --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml --all-targets \
  --message-format=json -- --test-threads=1
```

The retained `coverage-wrapper.sh` instruments every compilation unit whose
`CARGO_MANIFEST_DIR` identifies either owned crate, including all integration
callers and examples. Its SHA-256 is
`449138a815dbff9b08023b92725f9ed26e6a8b9755ec4b7a4dd3387c16fa1567`.
Upstream dependencies are not instrumented. Changing the wrapper alone reused
old Cargo artifacts, so only these two owned packages were invalidated using
`cargo clean --release -p PACKAGE --manifest-path MANIFEST`. Both rebuilt test
artifact lists show `fresh=false`; upstream caches and frozen native binaries
were preserved. Jobs stayed bounded at two and free space stayed above 12 GiB.

Combining profiles from different binaries/configurations into a single LLVM
export produced 41 mismatched-function warnings. The final gate does not use
that warningful export. `run-per-binary.sh` runs each exact emitted test
executable in its own new profile directory, merges only that executable's
profiles and exports against only that executable. All twenty-one exports
completed with **empty warning/error output**. These exports are aggregated by
maximum observed `DA` count for each distinct owned source file/line. Thus a
covered generic method emitted into any integration caller is counted, without
mixing incompatible function hashes. This is a union of the same test suite,
not an expansion or a different denominator selected after observing results.

`source.lcov` retains the final deduplicated owned-source line hits; `lines.tsv`
and `summary.json` retain per-file and aggregate metrics. LLVM expansion-context
summary percentages are not substituted for these distinct-line metrics. No
branch or MC/DC coverage was collected.
`test-binaries.sha256` binds the exact twenty-one instrumented executables.

To regenerate changed lines, use `git diff --unified=0 BASE SOURCE --
crates/spiralcore-mvm-adapter/src crates/spiralcore-mvm-live/src`. Advance the
hunk's new-line number on context/additions, selecting added lines.
`summarize.awk` intersects those selections with LCOV `DA` entries and the
explicit test cutoffs. The base and source identities above make that selection
reproducible. The compact packet retains only owned line hits, not payloads or
upstream build products.

## Retained measurement corrections

`attempt1-incomplete-wrapper.json` retains the first result, explicitly invalid
for the final gate: the historical wrapper missed new integration compilation
units, undercounting generic methods. Its reported below-target aggregate must
not be mistaken for the corrected result. A subsequent aggregate export also
had incompatible-function warnings; isolated per-binary collection replaced it.
The local raw outputs and profiles remain in the owned `coverage-final`
directory for inspection. No product or test source changed during corrections.

## Independent code/security and harness gate

Roster verification `d1927d9e7ce51354` and active checks for `harness-optimizer`
and `code-reviewer` were executed for this review. The working tree was clean
apart from preexisting untracked `.serena/` before this evidence was written.
No production source edits were made by this task.

The focused reviews in this session covered shared policy/owner fencing,
ordered Call/Reply state transitions, signed receipt resource reservations and
identity matching, cross-destination epoch handling, immutable sender headers,
bounded issuance, exact object bridging, native profile admission/limits,
descriptor custody and explicit Firecracker state authority. Previously
approved source identities remain unchanged at this checkpoint. No unresolved
blocking source/security finding was identified in that bounded scope.

Actual test/build tool receipts and component qualification records support the
verification claims; execution was not inferred from a green badge or agent
assertion. The legacy `.agent-sync` directory has no DAILY/task-results ledger
for this milestone, so no audit of missing historical task logs is claimed.
This packet and existing component records provide reviewable evidence. The
unavailable `/quality-gate` command is not reported as executed; equivalent
independent code/security review is recorded instead.

**PASS for the reviewed source and declared compiled-production coverage scope.**
Outstanding native backend, platform, WAN, upstream acceptance and certification
gates retain their separate requirements.
