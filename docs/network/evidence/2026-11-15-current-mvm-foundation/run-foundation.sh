#!/usr/bin/env bash
set -euo pipefail
q=/var/tmp/sc-direct-flowmux.iptmjZre
out="$q/privacy-mvm-full1"
umask 0022
printf '5819ec8dc2d2ed897cc4716aa242131acef9f5c5\n' > "$out-source.txt"
umask > "$out-umask.txt"
export PATH="$q/qualification-tools/uv/bin:/home/callard/.cargo/bin:/usr/bin:/bin" RUSTUP_TOOLCHAIN=1.97.1
export CARGO_HOME="$q/source/.mvm-test/cargo" CARGO_TARGET_DIR="$q/source/.mvm-test/target"
export MVM_HOME="$q/privacy-mvm-full1-state" MVM_DEV_ENV_KEEP_INHERITED=1
mkdir -m700 "$MVM_HOME"
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export MVM_GPU_TEST_EXPECT_NATIVE=1 PYTHONDONTWRITEBYTECODE=1
export UV_CACHE_DIR="$q/qualification-cache/uv" npm_config_cache="$q/qualification-cache/npm"
export TMPDIR
TMPDIR=$(mktemp -d /tmp/sc-oq.XXXXXX)
printf '%s\n' "$TMPDIR" > "$out-private-tmp.txt"
ulimit -Sn 4096
ulimit -S -s 8192
cd "$q/flowmux-credit-mvm-work"
source scripts/dev-env.sh
sha256sum -c "$q/privacy-mvm-full1-frozen-inputs.sha256" > "$out-inputs-before.txt"
find . -path ./.git -prune -o -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum > "$out-file-set-before.sha256"
cmp "$q/privacy-mvm-full1-frozen-inputs.sha256" "$out-file-set-before.sha256"
finish() {
  original=$?
  trap - EXIT
  set +e
  sha256sum -c "$q/privacy-mvm-full1-frozen-inputs.sha256" > "$out-inputs-after.txt"
  hashes=$?
  find . -path ./.git -prune -o -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum > "$out-file-set-after.sha256"
  files=$?
  cmp "$q/privacy-mvm-full1-frozen-inputs.sha256" "$out-file-set-after.sha256"
  comparison=$?
  final=$original
  if [ "$hashes" -ne 0 ] || [ "$files" -ne 0 ] || [ "$comparison" -ne 0 ]; then final=90; fi
  printf '{"original_exit":%s,"source_hash_exit":%s,"file_enumeration_exit":%s,"file_set_comparison_exit":%s,"final_exit":%s}\n' "$original" "$hashes" "$files" "$comparison" "$final" > "$out-outcome.json"
  printf '%s\n' "$final" > "$out-exit.txt"
  echo "PRIVACY_FULL_TERMINAL original=$original source_hash=$hashes file_set=$comparison final=$final"
  exit "$final"
}
trap finish EXIT
printf 'same physical root; source285 target set derives fromB2: two crypto additions and zero-test BDD helper relocation; reuse requires a prior matching hash\n' > "$out-codegen-policy.txt"
{ printf 'soft=%s\nhard=%s\n' "$(ulimit -Sn)" "$(ulimit -Hn)"; stat -c '%U %a %n' "$TMPDIR"; cat /proc/driver/nvidia/version; } > "$out-prerequisites.txt"
rustc -Vv > "$out-toolchain.txt"
systemctl --user show sc-privacy-mvm-full1 -p InvocationID -p MemoryMax -p MemorySwapMax -p CPUQuotaPerSecUSec -p TasksMax -p LimitNOFILE -p LimitSTACK -p UMask > "$out-limits.txt"
cargo metadata --locked --offline --format-version=1 > "$out-metadata.json"
jq -e --arg source 'git+https://github.com/R4-Laboratories/SpiralCore?rev=8994951b7c13fe5acf6f5ea3060ad4e51dc473c2#8994951b7c13fe5acf6f5ea3060ad4e51dc473c2' '[.packages[] | select(.name|test("^(spiralcore-|uor-r4-object-store)"))] as $core | ($core|length)==5 and all($core[];.source==$source)' "$out-metadata.json" > "$out-pin-check.txt"
cargo fmt --all -- --check > "$out-format.txt" 2>&1
/usr/bin/time -v -o "$out-clippy-time.txt" cargo clippy --locked --offline --workspace -- -D warnings > "$out-clippy.txt" 2>&1
/usr/bin/time -v -o "$out-check-time.txt" cargo check --locked --offline --workspace --all-targets > "$out-check.txt" 2>&1
/usr/bin/time -v -o "$out-bdd-time.txt" cargo check --locked --offline -p mvm-conformance --all-targets --features bdd > "$out-bdd.txt" 2>&1
/usr/bin/time -v -o "$out-footprint-time.txt" cargo run --locked --offline -p xtask --bin xtask -- check-rust-footprint > "$out-footprint.txt" 2>&1
/usr/bin/time -v -o "$out-policy-time.txt" cargo run --locked --offline -p xtask --bin xtask -- check-all > "$out-policy.txt" 2>&1
/usr/bin/time -v -o "$out-test-build-time.txt" cargo test --locked --offline --workspace --no-run --message-format=json > "$out-test-build.jsonl" 2> "$out-test-build-stderr.txt"
python3 "$q/sc-privacy-full-executables.py" "$out-test-build.jsonl" "$out" "$q/flowmux-credit-mvm-work" "$q/privacy-full-prior-oracle.json" "$q/privacy-full-expected-targets.json"
/usr/bin/time -v -o "$out-tests-time.txt" cargo test --locked --offline --workspace -- --test-threads=1 > "$out-tests.txt" 2>&1
python3 "$q/sc-privacy-runtime-executable-set.py" "$out-tests.txt" "$out-executables-before.json" "$out-runtime-executable-set.json"
sha256sum -c "$out-executables-before.sha256" > "$out-executables-after.txt"
