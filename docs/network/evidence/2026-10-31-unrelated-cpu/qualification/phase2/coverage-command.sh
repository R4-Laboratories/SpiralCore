#!/bin/bash
set -euo pipefail
cd /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
owned=/Users/casey.allard/spiralcore/local/unrelated-cpu-20261031/coverage-phase2
evidence=docs/network/evidence/2026-10-31-unrelated-cpu/qualification/phase2
mkdir "$owned" "$evidence"
export CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/local/shared-state-20261030/coverage-target
export CARGO_INCREMENTAL=0
export RUSTFLAGS='-C instrument-coverage'
export LLVM_PROFILE_FILE="$owned/test-%p-%m.profraw"
/Users/casey.allard/.cargo/bin/cargo +1.97.1 test --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --target aarch64-apple-darwin --features hardware-sha256 --lib --test unrelated_delta --test sparse --test schema_sender --test schema_codec --test schema_prepared --test schema_reuse --test warm_selection --message-format=json > "$owned/cargo-output.jsonl" 2> "$evidence/coverage-stderr.txt"
llvm=/Users/casey.allard/.rustup/toolchains/1.97.1-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin
"$llvm/llvm-profdata" merge -sparse "$owned"/*.profraw -o "$owned/current.profdata"
objects=()
while IFS= read -r executable; do objects+=(-object "$executable"); done < <(jq -Rr 'fromjson? | select(.reason=="compiler-artifact" and .profile.test==true and .executable!=null) | .executable' "$owned/cargo-output.jsonl" | sort -u)
[[ ${#objects[@]} -gt 0 ]]
"$llvm/llvm-cov" export -summary-only -instr-profile="$owned/current.profdata" "${objects[@]}" > "$evidence/coverage.json"
jq '[.data[].files[] | select(.filename|endswith("/src/delta.rs") or endswith("/src/schema.rs")) | {filename,summary}]' "$evidence/coverage.json" > "$evidence/coverage-changed-files.json"
rg '^test result:' "$owned/cargo-output.jsonl" > "$evidence/coverage-tests.txt"
shasum -a 256 crates/spiralcore-transfer/src/delta.rs crates/spiralcore-transfer/src/schema.rs "$owned"/*.profraw > "$evidence/coverage-sources-profiles.sha256"
for ((i=1;i<${#objects[@]};i+=2)); do shasum -a 256 "${objects[$i]}"; done > "$evidence/coverage-objects.sha256"
cat "$evidence/coverage-changed-files.json"
