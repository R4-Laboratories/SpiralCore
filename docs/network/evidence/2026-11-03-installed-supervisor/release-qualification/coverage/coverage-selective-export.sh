#!/usr/bin/env bash
set -euo pipefail
base=/var/tmp/spiralcore-installed-supervisor-20261103/coverage-selective
llvm=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin
"$llvm/llvm-profdata" merge -sparse "$base"/profiles/*.profraw -o "$base/exports/merged.profdata"
objects=()
for executable in "$base"/bin/*; do objects+=(-object "$executable"); done
"$llvm/llvm-cov" export "${objects[@]}" -instr-profile="$base/exports/merged.profdata" -format=lcov \
  -ignore-filename-regex='(/\.cargo/|/\.rustup/|/tests/|/examples/|/build/)' > "$base/exports/owned.lcov"
"$llvm/llvm-cov" report "${objects[@]}" -instr-profile="$base/exports/merged.profdata" \
  -ignore-filename-regex='(/\.cargo/|/\.rustup/|/tests/|/examples/|/build/)' > "$base/exports/report.txt"
sha256sum "$base/exports/owned.lcov" "$base/exports/report.txt" "$base/exports/merged.profdata" > "$base/exports/SHA256SUMS"
grep -E 'installed_|orchestrator|TOTAL' "$base/exports/report.txt" |tail -18
