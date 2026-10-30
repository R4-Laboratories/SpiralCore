# Native qualification before shared-runtime measurements

These are actual command receipts, retained at the user's request. They are not
timed acceptance observations. Source and executable identities are frozen
separately before the registered campaign.

- `all-targets-final-attempt2.txt` / `.exit`: release, native
  `aarch64-apple-darwin`, Rust 1.97.1, `hardware-sha256`; **220 tests pass across
  26 targets**, with two explicitly ignored historical large HTTP qualifications.
- `doctests.txt`: one compile-fail doctest passes.
- `object-store.txt`: all **71** imported-store tests pass across nine targets.
- `build.txt`: native release build of all targets succeeds.
- `format-final.txt`: format check passes for the five new Rust source/test/example
  files. `transfer-format-final.txt` reruns all seven transfer tests successfully
  after formatting the final added test.
- `../reference/review-all-target-check.txt` and
  `../reference/review-all-target-clippy.txt`: all-target check and Clippy with
  warnings denied succeed. Source-pinned independent reviews are separate.
- `coverage.txt` / `coverage.json`: a separate instrumented target runs all
  22 runtime, seven transfer and five harness tests. The runtime covers 428/437
  lines (97.94%), 31/31 LLVM functions and 787/807 regions. The transfer helper
  covers 50/54 lines (92.59%), 2/2 functions and 101/109 regions. Branch, MC/DC
  and statement coverage are **not measured**. This is focused coverage of the
  new modules, not a whole-repository coverage percentage.
- `release-before-coverage.sha256`: both ordinary release binaries verified
  unchanged after instrumentation. Instrumented timings are never used as
  performance results.

The common Cargo arguments were:

```sh
CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target \
  /Users/casey.allard/.cargo/bin/cargo +1.97.1 COMMAND --release --locked \
  --manifest-path crates/spiralcore-transfer/Cargo.toml \
  --target aarch64-apple-darwin --features hardware-sha256
```

The coverage command used `cargo llvm-cov`, a separate
`local/shared-state-20261030/coverage-target`, and `--test shared_runtime --test
shared_transfer --example shared_runtime --json --summary-only`. Platform,
compiler, dependency features, CPU model and physical memory are retained here.

## Failed attempts and corrections

`transfer-red` precedes the implementation and fails because the runtime transfer
module/API/mode do not yet exist. `transfer-green-attempt1` then passes its initial
six tests. The seventh test adds revocation and changed-epoch coverage.

`all-targets-attempt1` was invoked while the measurement example was still under
construction; it fails on absent helper functions. It is not an all-target pass.
`all-targets-final` later finds a legacy test that assumed mode byte 5 was
unassigned. This implementation assigns 5 to RuntimeV1. The test now uses
unassigned byte 255, preserving unsupported-mode rejection without weakening
the decoder. Its focused regression and the full successful rerun are retained.

`format.txt` records the first format check finding the newly added transfer test
unformatted. Only that new test file was formatted; the focused tests and format
check were rerun. Earlier engine/Clippy and harness failures are retained in
`../reference/` and `harness/`, including their explanations and exact outcomes.

The reference checksum list uses paths relative to `reference/`. An operator
attempt from the repository root therefore reported missing files and compared
the wrong README; rerunning from `reference/` verified all 56 entries. No packet
content was changed to make that check pass.

The pinned Python extraction can be rerun only with the preserved original local
sources. Bundled vectors/excerpts and the Rust qualification are reviewable from
the repository; full Python extraction is not claimed to be self-contained.
