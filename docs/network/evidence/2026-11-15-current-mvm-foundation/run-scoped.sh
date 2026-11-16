#!/usr/bin/env bash
set -euo pipefail
q=/var/tmp/sc-direct-flowmux.iptmjZre
out="$q/privacy-compile5"
manifest="$out-frozen-inputs.sha256"
umask 0022
umask > "$out-umask.txt"
export PATH="$q/qualification-tools/uv/bin:/home/callard/.cargo/bin:/usr/bin:/bin" RUSTUP_TOOLCHAIN=1.97.1
export CARGO_HOME="$q/source/.mvm-test/cargo" CARGO_TARGET_DIR="$q/source/.mvm-test/target"
export MVM_HOME="$out-state" MVM_DEV_ENV_KEEP_INHERITED=1
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export MVM_GPU_TEST_EXPECT_NATIVE=1 PYTHONDONTWRITEBYTECODE=1
export UV_CACHE_DIR="$q/qualification-cache/uv" npm_config_cache="$q/qualification-cache/npm"
mkdir -m700 "$MVM_HOME"
export TMPDIR
TMPDIR=$(mktemp -d /tmp/sc-pc.XXXXXX)
printf '%s\n' "$TMPDIR" > "$out-private-tmp.txt"
ulimit -Sn 4096
ulimit -S -s 8192
cd "$q/flowmux-credit-mvm-work"
source scripts/dev-env.sh
sha256sum -c "$manifest" > "$out-inputs-before.txt"
find . -path ./.git -prune -o -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum > "$out-file-set-before.sha256"
cmp "$manifest" "$out-file-set-before.sha256"
finish() {
  original=$?
  trap - EXIT
  set +e
  sha256sum -c "$manifest" > "$out-inputs-after.txt"
  hashes=$?
  find . -path ./.git -prune -o -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum > "$out-file-set-after.sha256"
  files=$?
  cmp "$manifest" "$out-file-set-after.sha256"
  comparison=$?
  final=$original
  if [ "$hashes" -ne 0 ] || [ "$files" -ne 0 ] || [ "$comparison" -ne 0 ]; then final=90; fi
  printf '{"original_exit":%s,"source_hash_exit":%s,"file_enumeration_exit":%s,"file_set_comparison_exit":%s,"final_exit":%s}\n' "$original" "$hashes" "$files" "$comparison" "$final" > "$out-outcome.json"
  printf '%s\n' "$final" > "$out-exit.txt"
  echo "PRIVACY_COMPILE5_TERMINAL original=$original source_hash=$hashes files=$comparison final=$final"
  exit "$final"
}
trap finish EXIT
printf '5819ec8dc2d2ed897cc4716aa242131acef9f5c5\n' > "$out-source.txt"
systemctl --user show sc-privacy-compile5 -p InvocationID -p MemoryMax -p MemorySwapMax -p CPUQuotaPerSecUSec -p TasksMax -p LimitNOFILE -p LimitSTACK -p RuntimeMaxUSec -p UMask > "$out-limits.txt"
rustc -Vv > "$out-toolchain.txt"
cargo metadata --locked --offline --format-version=1 > "$out-metadata.json"
jq -e --arg source 'git+https://github.com/R4-Laboratories/SpiralCore?rev=8994951b7c13fe5acf6f5ea3060ad4e51dc473c2#8994951b7c13fe5acf6f5ea3060ad4e51dc473c2' '[.packages[] | select(.name|test("^(spiralcore-|uor-r4-object-store)"))] as $core | ($core|length)==5 and all($core[];.source==$source)' "$out-metadata.json" > "$out-pin-check.txt"
cargo fmt --all -- --check > "$out-format.txt" 2>&1
record() {
  label=$1
  shift
  set +e
  /usr/bin/time -v -o "$out-$label-time.txt" "$@" > "$out-$label.txt" 2>&1
  result=$?
  set -e
  printf '%s\n' "$result" > "$out-$label-exit.txt"
  echo "PRIVACY_COMPILE5_PHASE $label exit=$result"
}
record footprint cargo run --locked --offline -p xtask --bin xtask -- check-rust-footprint
if [ "$(cat "$out-footprint-exit.txt")" -ne 0 ]; then exit 101; fi
record closure cargo run --locked --offline -p xtask --bin xtask -- check-closure-budget
record feature cargo run --locked --offline -p xtask --bin xtask -- check-feature-closure-budget
record clippy cargo clippy --locked --offline -p mvm-runtime -p mvm-hostd --tests -- -D warnings
if [ "$(cat "$out-clippy-exit.txt")" -ne 0 ]; then exit 101; fi
build_and_run() {
  label=$1
  package=$2
  filter=$3
  set +e
  /usr/bin/time -v -o "$out-$label-build-time.txt" cargo test --locked --offline -p "$package" --lib --no-run --message-format=json > "$out-$label-build.jsonl" 2> "$out-$label-build.stderr"
  result=$?
  set -e
  printf '%s\n' "$result" > "$out-$label-build-exit.txt"
  if [ "$result" -ne 0 ]; then return "$result"; fi
  jq -s -e --arg source "$q/flowmux-credit-mvm-work/crates/$package/src/lib.rs" '[.[] | select(.reason=="compiler-artifact" and .profile.test==true and .target.src_path==$source and .executable!=null)] | if length==1 then .[0] else error("exact current-root library test artifact missing") end' "$out-$label-build.jsonl" > "$out-$label-selected-artifact.json"
  if jq -e '.fresh==false' "$out-$label-selected-artifact.json" > "$out-$label-fresh-check.txt"; then
    printf 'fresh compiler artifact\n' > "$out-$label-codegen-policy.txt"
  else
    test "$label" = hostd
    cmp "$manifest" "$q/privacy-compile4-frozen-inputs.sha256"
    cmp "$out-source.txt" "$q/privacy-compile4-source.txt"
    prior=$(jq -er .executable "$q/privacy-compile4-hostd-selected-artifact.json")
    jq -e --arg prior "$prior" '.executable==$prior' "$out-$label-selected-artifact.json" > "$out-$label-same-artifact-path.txt"
    sha256sum -c "$q/privacy-compile4-hostd-executable-before.sha256" > "$out-$label-same-source-reuse-check.txt"
    printf 'verified same-head/current-root artifact reused; source manifest and prior executable SHA identical\n' > "$out-$label-codegen-policy.txt"
  fi
  executable=$(jq -er .executable "$out-$label-selected-artifact.json")
  case "$executable" in "$CARGO_TARGET_DIR"/debug/deps/*) ;; *) return 90 ;; esac
  sha256sum "$executable" > "$out-$label-executable-before.sha256"
  record "$label" "$executable" "$filter" --test-threads=1
  sha256sum -c "$out-$label-executable-before.sha256" > "$out-$label-executable-after.txt"
}
build_and_run hostd mvm-hostd plan_admission::
if [ "$(cat "$out-hostd-exit.txt")" -ne 0 ]; then exit 101; fi
build_and_run runner mvm-runtime workload_runner::runner
for label in closure feature runner; do
  if [ "$(cat "$out-$label-exit.txt")" -ne 0 ]; then exit 101; fi
done
echo PRIVACY_COMPILE5_SCOPED_GATES_PASS
