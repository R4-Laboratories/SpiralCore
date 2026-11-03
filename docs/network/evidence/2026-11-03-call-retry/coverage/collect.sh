#!/usr/bin/env bash
# Selective adapter instrumentation; retain each executable before execution.
set -euo pipefail
root=/var/tmp/spiralcore-call-retry-20261103
out="$root/coverage"
target=/var/tmp/spiralcore-mvm-service-20261102/target
cargo=/home/callard/.cargo/bin/cargo
llvm=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin
[[ ! -e "$out" ]]
mkdir -m 700 "$out"
mkdir "$out/bin"
cd "$root/source"
sha256sum -c ../qualified-source-sha256.txt > "$out/source-verification.txt"
ps -eo pid,comm | awk '$2=="cargo" || $2=="rustc"' > "$out/active-builds-before.txt"
[[ ! -s "$out/active-builds-before.txt" ]]
export CARGO_TARGET_DIR="$target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER
"$cargo" +1.97.1 clean --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml --release -p spiralcore-mvm-adapter > "$out/owned-clean.txt" 2>&1
cp "$root/coverage-scripts/coverage-wrapper.sh" "$out/coverage-wrapper.sh"
chmod 700 "$out/coverage-wrapper.sh"
export RUSTC_WRAPPER="$out/coverage-wrapper.sh"
sha256sum "$RUSTC_WRAPPER" > "$out/wrapper.sha256"
"$cargo" +1.97.1 test --release --locked --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml --features live,conformance --lib --tests --no-run --message-format=json > "$out/build.jsonl" 2> "$out/build.stderr"
jq -s '[.[] | select(.reason=="compiler-artifact" and .executable!=null and .profile.test==true and (.manifest_path|endswith("/crates/spiralcore-mvm-adapter/Cargo.toml"))) | {target:.target.name,fresh,executable}]' "$out/build.jsonl" > "$out/binaries.json"
jq -e 'length>0 and all(.[];.fresh==false) and ([.[].target]|unique|length)==length' "$out/binaries.json" >/dev/null
jq -r '.[].executable' "$out/binaries.json" | while IFS= read -r binary; do
  copy="$out/bin/$(basename "$binary")"
  cp -p "$binary" "$copy"
  chmod 500 "$copy"
  cmp "$binary" "$copy"
  sha256sum "$binary" >> "$out/binaries.sha256"
  sha256sum "$copy" >> "$out/binaries-retained.sha256"
done
jq --arg out "$out" '[.[] | . + {retained_executable: ($out+"/bin/"+(.executable|split("/")|last))}]' "$out/binaries.json" > "$out/executed-binaries.json"
jq -r '.[]|[.target,.retained_executable]|@tsv' "$out/executed-binaries.json" | while IFS=$'\t' read -r name binary; do
  dir="$out/$name"
  mkdir -p "$dir/profiles"
  LLVM_PROFILE_FILE="$dir/profiles/%m-%p.profraw" "$binary" --test-threads=1 > "$dir/tests.txt" 2> "$dir/tests.stderr"
  [[ ! -s "$dir/tests.stderr" ]]
  [[ $(rg -c '^test result:' "$dir/tests.txt") == 1 ]]
  rg -q '^test result: ok\. [0-9]+ passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;' "$dir/tests.txt"
  passed=$(awk '/^test result:/ {print $4}' "$dir/tests.txt")
  jq -n --arg target "$name" --argjson passed "$passed" '{target:$target,passed:$passed,failed:0,ignored:0,filtered:0}' >> "$out/test-counts.jsonl"
  "$llvm/llvm-profdata" merge -sparse "$dir"/profiles/*.profraw -o "$dir/current.profdata"
  "$llvm/llvm-cov" export -format=lcov -instr-profile="$dir/current.profdata" -object "$binary" > "$dir/source.lcov" 2> "$dir/export.stderr"
  [[ ! -s "$dir/export.stderr" ]]
  rg '^test result:' "$dir/tests.txt"
done
jq -s '{binaries:length,passed:(map(.passed)|add),failed:(map(.failed)|add),ignored:(map(.ignored)|add),filtered:(map(.filtered)|add)}' "$out/test-counts.jsonl" > "$out/test-counts.json"
cat "$out"/*/source.lcov > "$out/combined.lcov"
awk '
 /^SF:/ {f=substr($0,4); sub(/^.*\/crates\//,"crates/",f); next}
 /^DA:/ && f ~ /^crates\/spiralcore-mvm-adapter\/src\// {
   split(substr($0,4),a,","); k=f SUBSEP a[1];
   if(!(k in hits) || a[2]+0>hits[k]) hits[k]=a[2]+0;
 }
 END {OFS="\t"; for(k in hits) {split(k,a,SUBSEP); print a[1],a[2],hits[k]}}
' "$out/combined.lcov" | sort -k1,1 -k2,2n > "$out/line-hits.tsv"
awk '
 /^SF:/ {f=substr($0,4); sub(/^.*\/crates\//,"crates/",f); next}
 /^BRDA:/ && f ~ /^crates\/spiralcore-mvm-adapter\/src\// {
   split(substr($0,6),a,","); k=f SUBSEP a[1] SUBSEP a[2] SUBSEP a[3];
   if(!(k in hits) || a[4]+0>hits[k]) hits[k]=a[4]+0;
 }
 END {OFS="\t"; for(k in hits) {split(k,a,SUBSEP); print a[1],a[2],a[3],a[4],hits[k]}}
' "$out/combined.lcov" | sort -k1,1 -k2,2n > "$out/branch-hits.tsv"
# service.rs contains production only; do not exclude newly moved validator lines.
printf 'crates/spiralcore-mvm-adapter/src/service.rs\t1000000\n' > "$out/production-cutoffs.tsv"
"$llvm/llvm-cov" --version > "$out/llvm-version.txt"
sha256sum -c ../qualified-source-sha256.txt > "$out/source-verification-after.txt"
cat "$out/test-counts.json"
