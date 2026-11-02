# Object-v1 source coverage

Executed on macOS arm64 using Rust 1.97.1 and LLVM 22.1.6. The measured
`src/v1/mod.rs` and `src/v1/sender.rs` match commit `f45c6c2` exactly; this run
changed no Rust runtime or test source. Machine-readable counts, source/blob
identities and raw-artifact hashes are in [v1-source-coverage.json](v1-source-coverage.json).

| Source | Lines | Functions | Regions |
|---|---:|---:|---:|
| `src/v1/mod.rs` | 129/150 (86.00%) | 16/22 (72.73%) | 158/199 (79.40%) |
| `src/v1/sender.rs` | 173/200 (86.50%) | 19/20 (95.00%) | 239/305 (78.36%) |
| **Combined** | **302/350 (86.29%)** | **35/42 (83.33%)** | **397/504 (78.77%)** |

The finishing threshold is **at least 80% line coverage**, which passes.
Regions remain below 80%; no claim is made that a region threshold passed.
Stable Rust instrumentation supplied no branch counters: branch coverage is
unavailable, rather than a measured zero percent. Generic instantiation coverage
is recorded separately in JSON. Coverage indicates execution and does not prove
correctness or security.

The instrumented campaign passed 16 tests: transfer `v1_contract` (8), gateway
`core` (4), `object_combined` (1) and `object_runtime` (3). No failures or ignored
tests. Instrumented elapsed times are not performance results.

## Executed procedure

Temporary ownership root: `/tmp/spiralcore-v1-coverage-60BCiL`.
Existing shared build target:
`/Users/casey.allard/spiralcore/local/shared-state-20261030/coverage-target`.
Only the two owned package products were cleaned to force recompilation;
upstream dependency products were retained.

The initial wrapper instrumented only crate-name `spiralcore_transfer`. That
missed generic methods instantiated in the integration-test crates. The corrected
wrapper below instruments the two owned packages, including their integration
tests, while leaving upstream dependencies uninstrumented. All reported final
numbers include the corrected instrumented executions.

```sh
#!/bin/sh
compiler=$1
shift
case "$CARGO_PKG_NAME" in
  spiralcore-transfer|spiralcore-gateway-core)
    exec "$compiler" "$@" -C instrument-coverage ;;
  *) exec "$compiler" "$@" ;;
esac
```

Commands used the following environment and executable:

```sh
export RUSTC_WRAPPER=/tmp/spiralcore-v1-coverage-60BCiL/rustc-wrapper-v2.sh
export LLVM_PROFILE_FILE=/tmp/spiralcore-v1-coverage-60BCiL/%p-%m.profraw
export CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/local/shared-state-20261030/coverage-target
export CARGO_BUILD_JOBS=2
export CARGO_INCREMENTAL=0

/Users/casey.allard/.cargo/bin/cargo +1.97.1 clean \
  --manifest-path crates/spiralcore-gateway-core/Cargo.toml \
  -p spiralcore-transfer -p spiralcore-gateway-core

/Users/casey.allard/.cargo/bin/cargo +1.97.1 test \
  --manifest-path crates/spiralcore-transfer/Cargo.toml \
  --test v1_contract --message-format=json

/Users/casey.allard/.cargo/bin/cargo +1.97.1 test \
  --manifest-path crates/spiralcore-gateway-core/Cargo.toml \
  --test core --test object_runtime --test object_combined --message-format=json
```

Cargo stdout and stderr were redirected into the owned temporary root as
`transfer-final-cargo.json`, `transfer-final-build.log`,
`gateway-final-cargo.json` and `gateway-final-build.log`. The Cargo JSON artifact
records identified these four actual binaries:

```text
coverage-target/debug/deps/v1_contract-8d203c588226a835
coverage-target/debug/deps/core-6245a3eaa4ba4ddc
coverage-target/debug/deps/object_combined-cbf9f84c92667458
coverage-target/debug/deps/object_runtime-acff04db363f6ab9
```

Using `llvm-profdata` and `llvm-cov` from the Rust toolchain's
`lib/rustlib/aarch64-apple-darwin/bin`, profiles were merged with
`llvm-profdata merge -sparse <owned *.profraw> -o merged-final.profdata`.
`llvm-cov export -summary-only -instr-profile=merged-final.profdata` used the
first binary and three additional `-object <binary>` arguments. The retained
JSON extracts only the two v1 source-file summaries from that export.

Compact hashes were retained before deleting the owned temporary profiles,
merged coverage database and debug exports. No raw coverage output is committed;
the shared build target and its warmed dependency products remain available.
