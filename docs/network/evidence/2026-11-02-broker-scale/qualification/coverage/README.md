# Broker source coverage checkpoint

**PASS in the declared compiled-production source-line scope:**
**6,022 / 6,938 lines (86.80%)**. Additions relative to PR base
`6bf85f97bd2f22c055ab30683f8a3013a28ef267` measure
**417 / 441 changed production lines (94.56%)**.

Including inline test blocks, all compiled source lines measure
**6,783 / 7,700 (88.09%)**. Changed source and changed production both count
441 executable lines in this milestone; no changed inline test lines contribute
to that denominator.

Production source matches `65abaa0b68a1a0e28b8e3aa713dfb4c8addf29dc` and public MVM
dependency `ae12e9deb41cc9a3264196ce22ebc72d4042883d`. Collection started with the
portable capacity test's full-error diagnostic assertion already in the working
tree. That sole test-only delta was committed as `0ee32ab` during collection;
the production source did not change. The exact test hash is in `summary.json`.
The packet inventories source and instrumented executable identities; no binary
copies or VM images are stored here.

## Scope and practical limits

The same scope as the preceding service coverage checkpoint includes all
**21 compiled source files** under the two owned crates' `src/` directories.
It includes legacy production paths, fixtures, host and orchestrator. Only the
explicit trailing inline test modules listed in `test-cutoffs.tsv` are removed
from the production metric. Integration tests and measurement examples outside
`src/` are not part of this product-source metric. No compiled production source
file is excluded.

This is macOS ARM64 compilation, not native Linux, Windows or whole-repository
coverage. Linux-only backend bodies are uncompiled here; `linux_host.rs` covers
only its portable helpers. Branch, function and MC/DC percentages were not
collected and cannot be inferred from these line percentages. Native MVM and
WAN qualification remain separate gates.

| Production module | Covered / counted executable lines | Coverage |
|---|---:|---:|
| Shared broker | 28 / 28 | 100.00% |
| Sender metadata and stable ordering | 292 / 305 | 95.74% |
| Receipts, tracker and send permit | 758 / 796 | 95.23% |
| Native canonical consumer | 307 / 332 | 92.47% |
| Native host | 190 / 315 | 60.32% |
| Native orchestrator | 133 / 475 | 28.00% |

The aggregate pass does not mean every module exceeds 80%. Native host/process
supervision and orchestration are the main lower-coverage areas. Actual VM runs
are separate evidence, not counted as LLVM execution here.

## Executed collection

All **94 adapter and 49 live tests passed** in instrumented release all-target
runs, with zero failures, ignored or filtered tests. Each of the **26 exact
emitted test executables** was then rerun separately: **143 passing tests**,
again with zero failures, ignored or filtered tests. Zero-test harnesses add no
tests to those totals. No tests were created, skipped or changed for coverage.
Instrumented durations are not transfer-performance measurements.

```sh
export CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target
export CARGO_BUILD_JOBS=2
export CARGO_INCREMENTAL=0
export RUSTC_WRAPPER=/Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore/docs/network/evidence/2026-11-02-broker-scale/qualification/coverage/coverage-wrapper.sh
export LLVM_PROFILE_FILE=/Users/casey.allard/spiralcore/local/broker-scale-20261102/coverage/profiles-initial/%m-%p.profraw

/Users/casey.allard/.cargo/bin/cargo +1.97.1 test --release --locked \
  --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml \
  --all-features --all-targets --message-format=json -- --test-threads=1

/Users/casey.allard/.cargo/bin/cargo +1.97.1 test --release --locked \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --all-targets --message-format=json -- --test-threads=1
```

The retained wrapper instruments every compilation unit owned by either package
through `CARGO_MANIFEST_DIR`, including integration callers of generic functions.
Upstream packages are uninstrumented. Before rebuilding, only those two owned
release packages were cleaned with `cargo clean --release -p PACKAGE`, reclaiming
138.2 MiB while retaining upstream caches. All emitted owned executable artifacts
record `fresh: false`. The shared Cargo target was left instrumented afterward;
normal builds must omit the wrapper before any new performance measurement.

`run-per-binary.sh` reruns each exact executable with its own fresh profile
directory, merges only that executable's profiles, and exports LCOV against
only that executable. All **26 LLVM exports had empty warning/error output**;
all per-executable test stderr files were empty. This avoids incompatible
function-hash mixtures between independently linked binaries/configurations.

The final source-line union takes the maximum observed `DA` count per owned
file/line across these isolated exports. `source.lcov` preserves those line
hits; `lines.tsv` and `summary.json` preserve all per-file and aggregate totals.
The retained AWK summarizer independently reproduced every per-file total from
that LCOV; sorted outputs compared byte for byte equal. Changed executable
lines come from `git diff --unified=0 BASE --` the two `src/` directories, selecting
added new-side lines before intersecting them with the LCOV data.

Compact evidence occupies approximately 160 KiB. The owned local collection
directory occupies approximately 11 MiB, including raw profiles and unfiltered
exports. No copied executables, payload datasets, build trees or VM disks were
created for this coverage task. Available local storage stayed above 27 GiB.
