#!/usr/bin/env bash
# Selective instrumentation; performance comes only from protected normal bins.
set -euo pipefail
root=/var/tmp/spiralcore-session-20261103
out="$root/coverage"
target=/var/tmp/spiralcore-mvm-service-20261102/target
cargo=/home/callard/.cargo/bin/cargo
llvm=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin
[[ ! -e "$out" ]]
mkdir -m 700 "$out"
cd "$root/source"
sha256sum -c ../source-sha256.txt > "$out/source-verification.txt"
sha256sum "$root"/bin/* > "$out/normal-binaries-before.sha256"
ps -eo pid,comm,args > "$out/processes-before.txt"
# Before invoking, independently inspect active cargo/rustc paths: no build may
# use this shared SpiralCore target. Unrelated UOR builds use separate targets.
export CARGO_TARGET_DIR="$target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER
"$cargo" +1.97.1 clean --manifest-path crates/spiralcore-mvm-live/Cargo.toml --release -p spiralcore-mvm-live -p spiralcore-mvm-adapter > "$out/owned-clean.txt" 2>&1
export RUSTC_WRAPPER=/var/tmp/spiralcore-sustained-20261102/coverage-d940203/coverage-wrapper.sh
sha256sum "$RUSTC_WRAPPER" > "$out/wrapper.sha256"
"$cargo" +1.97.1 test --release --locked --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml --features live --test call_send --test receipt_send --no-run --message-format=json > "$out/adapter-build.jsonl" 2> "$out/adapter-build.stderr"
"$cargo" +1.97.1 test --release --locked --manifest-path crates/spiralcore-mvm-live/Cargo.toml --features linux-host --lib --test campaign --no-run --message-format=json > "$out/live-build.jsonl" 2> "$out/live-build.stderr"
jq -s '[.[] | select(.reason=="compiler-artifact" and .executable!=null and .profile.test==true and ((.manifest_path|endswith("/crates/spiralcore-mvm-adapter/Cargo.toml")) or (.manifest_path|endswith("/crates/spiralcore-mvm-live/Cargo.toml")))) | {target:.target.name,fresh,executable}]' "$out/adapter-build.jsonl" "$out/live-build.jsonl" > "$out/binaries.json"
jq -e 'length==4 and all(.[];.fresh==false)' "$out/binaries.json" >/dev/null
jq -r '.[].executable' "$out/binaries.json" > "$out/objects.txt"
while IFS= read -r binary; do
  dir="$out/$(basename "$binary")"
  mkdir -p "$dir/profiles"
  LLVM_PROFILE_FILE="$dir/profiles/%m-%p.profraw" "$binary" --test-threads=1 > "$dir/tests.txt" 2> "$dir/tests.stderr"
  "$llvm/llvm-profdata" merge -sparse "$dir"/profiles/*.profraw -o "$dir/current.profdata"
  "$llvm/llvm-cov" export -format=lcov -instr-profile="$dir/current.profdata" -object "$binary" > "$dir/source.lcov" 2> "$dir/export.stderr"
  [[ ! -s "$dir/export.stderr" ]]
  sha256sum "$binary" >> "$out/binaries.sha256"
  rg '^test result:' "$dir/tests.txt"
done < "$out/objects.txt"
cat "$out"/*/source.lcov > "$out/combined.lcov"
awk '
 /^SF:/ {f=substr($0,4); sub(/^.*\/crates\//,"crates/",f); next}
 /^DA:/ && f ~ /^crates\/spiralcore-mvm-(live|adapter)\/src\// {
   split(substr($0,4),a,","); k=f SUBSEP a[1];
   if(!(k in hits) || a[2]+0>hits[k]) hits[k]=a[2]+0;
 }
 END {OFS="\t"; for(k in hits) {split(k,a,SUBSEP); print a[1],a[2],hits[k]}}
' "$out/combined.lcov" | sort -k1,1 -k2,2n > "$out/line-hits.tsv"
"$llvm/llvm-cov" --version > "$out/llvm-version.txt"
sha256sum "$root"/bin/* > "$out/normal-binaries-after.sha256"
cmp "$out/normal-binaries-before.sha256" "$out/normal-binaries-after.sha256"
sha256sum -c ../source-sha256.txt > "$out/source-verification-after.txt"
printf 'PASS: four isolated test binaries and warning-free exports; derive scoped production coverage separately.\n'
