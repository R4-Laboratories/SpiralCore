#!/usr/bin/env bash
set -euo pipefail
umask 077
base=/var/tmp/spiralcore-installed-supervisor-20261103
out="$base/coverage-selective"
if [ -e "$out" ]; then exit 95; fi
mkdir -p "$out/bin" "$out/profiles" "$out/exports"
export PATH=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin:$PATH
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="$base/coverage-target"
unset RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER CARGO_ENCODED_RUSTFLAGS
unset RUSTFLAGS
export RUSTC_WRAPPER="$base/coverage-wrapper-owned.sh"
export LLVM_PROFILE_FILE="$out/profiles/%m-%p.profraw"
cd "$base/source"
find crates/spiralcore-mvm-live crates/spiralcore-mvm-adapter crates/spiralcore-transfer crates/spiralcore-gateway-core crates/uor-r4-object-store -type f \( -name '*.rs' -o -name Cargo.toml -o -name Cargo.lock \) ! -path '*/target/*' -print0 | sort -z | xargs -0 sha256sum > "$out/source-before.sha256"
cargo test --release --locked --manifest-path crates/spiralcore-mvm-live/Cargo.toml --features linux-host --all-targets --no-run --message-format=json > "$out/build.json" 2> "$out/build.stderr"
jq -r 'select(.reason=="compiler-artifact" and .profile.test==true and .executable!=null)|.executable' "$out/build.json" > "$out/executables.txt"
while IFS= read -r executable; do
 name=$(basename "$executable")
 cp "$executable" "$out/bin/$name"
 chmod 500 "$out/bin/$name"
 sha256sum "$out/bin/$name" >> "$out/executed-binaries.sha256"
 "$out/bin/$name" --test-threads=1 > "$out/$name.tests.txt" 2> "$out/$name.tests.stderr"
done < "$out/executables.txt"
cargo build --release --locked --manifest-path crates/spiralcore-mvm-live/Cargo.toml --features linux-host --bin spiralcore-mvm-live > "$out/host-build.txt" 2>&1
cp "$CARGO_TARGET_DIR/release/spiralcore-mvm-live" "$out/bin/host-coverage"
chmod 500 "$out/bin/host-coverage"
sha256sum "$out/bin/host-coverage" >> "$out/executed-binaries.sha256"
sha256sum -c "$out/source-before.sha256" > "$out/source-after-check.txt"
printf 'PASS: instrumented tests and host built; native coverage campaign and export still required.\n'
