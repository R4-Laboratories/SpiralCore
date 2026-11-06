#!/usr/bin/env bash
set -euo pipefail
umask 077
base=/var/tmp/spiralcore-installed-supervisor-20261103
out="$base/resources-coverage"
test ! -e "$out"
mkdir -p "$out/bin" "$out/profiles"
export PATH=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin:$PATH
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="$base/coverage-target"
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC_WORKSPACE_WRAPPER
export RUSTC_WRAPPER="$base/resources-coverage-wrapper.sh"
export LLVM_PROFILE_FILE="$out/profiles/%m-%p.profraw"
cd "$base/source"
find crates/spiralcore-mvm-live/tests/installed-supervisor-harness -type f \( -name '*.rs' -o -name Cargo.toml -o -name Cargo.lock \) -print0 | sort -z | xargs -0 sha256sum > "$out/source-before.sha256"
sha256sum crates/spiralcore-mvm-live/src/installed_resources.rs crates/spiralcore-mvm-live/src/installed_supervisor.rs crates/spiralcore-mvm-live/src/installed_owner.rs crates/spiralcore-mvm-live/src/host_budget.rs >> "$out/source-before.sha256"
cargo test --release --locked --manifest-path crates/spiralcore-mvm-live/tests/installed-supervisor-harness/Cargo.toml --all-targets --no-run --message-format=json > "$out/build.json" 2> "$out/build.stderr"
jq -r 'select(.reason=="compiler-artifact" and .profile.test==true and .executable!=null)|.executable' "$out/build.json" > "$out/executables.txt"
while IFS= read -r executable; do
  name=$(basename "$executable")
  cp "$executable" "$out/bin/$name"
  chmod 500 "$out/bin/$name"
  sha256sum "$out/bin/$name" >> "$out/executed-binaries.sha256"
  "$out/bin/$name" --test-threads=1 > "$out/$name.tests.txt" 2> "$out/$name.tests.stderr"
done < "$out/executables.txt"
llvm=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin
"$llvm/llvm-profdata" merge -sparse "$out"/profiles/*.profraw -o "$out/merged.profdata"
objects=()
for executable in "$out"/bin/*; do objects+=(-object "$executable"); done
"$llvm/llvm-cov" export "${objects[@]}" -instr-profile="$out/merged.profdata" -format=lcov > "$out/owned.lcov"
"$llvm/llvm-cov" report "${objects[@]}" -instr-profile="$out/merged.profdata" > "$out/report.txt"
sha256sum -c "$out/source-before.sha256" > "$out/source-after-check.txt"
sha256sum "$out/owned.lcov" "$out/report.txt" "$out/merged.profdata" > "$out/exports.sha256"
grep -E 'installed_resources|installed_supervisor|TOTAL' "$out/report.txt"
