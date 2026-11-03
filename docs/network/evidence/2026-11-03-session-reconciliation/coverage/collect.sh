#!/usr/bin/env bash
# Run only after root releases the frozen source following normal execution.
set -euo pipefail
root=/var/tmp/spiralcore-reconciliation-20261103
out="$root/coverage"
target=/var/tmp/spiralcore-mvm-service-20261102/target
cargo=/home/callard/.cargo/bin/cargo
llvm=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin
[[ ! -e "$out" ]]
mkdir -m 700 "$out"
mkdir "$out/bin"
cp "$root"/coverage-scripts/{collect.sh,derive.sh,finalize.sh,run.sh,compact.sh,coverage-wrapper.sh,production-cutoffs.tsv,changed-source.diff,source-pins.txt} "$out/"
cd "$root/source"
sha256sum -c ../qualified-source-sha256.txt > "$out/source-verification.txt"
[[ $(sha256sum ../qualified-source-sha256.txt | awk '{print $1}') == 93ca8300a46ed721fa06a87c52718d642c7dd1a52964eb80065816fbb63f7f8b ]]
sha256sum "$root/bin/broker_objects" > "$out/normal-binary-before.sha256"
[[ $(awk '{print $1}' "$out/normal-binary-before.sha256") == fe7937f2fb348ff0a3c517cf856f2ad132883346a8aa05020e6dbd0e1d62b6d7 ]]
stat -c '%a %n' "$root/bin/broker_objects" > "$out/normal-binary-mode.txt"
[[ $(awk '{print $1}' "$out/normal-binary-mode.txt") == 500 ]]
ps -eo pid,comm | awk '$2=="cargo" || $2=="rustc"' > "$out/active-builds-before.txt"
[[ ! -s "$out/active-builds-before.txt" ]]
while IFS=$'\t' read -r file cutoff; do
  actual=$(awk '/^#\[cfg\(test\)\]/ {print NR; found=1; exit} END {if(!found) print 1000000}' "$file")
  [[ "$actual" == "$cutoff" ]]
done < "$out/production-cutoffs.tsv"
export CARGO_TARGET_DIR="$target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER
# Normal observation receipts must never acquire instrumented CPU timings.
unset SPIRALCORE_RECONCILIATION_EVIDENCE_DIR
"$cargo" +1.97.1 clean --manifest-path crates/spiralcore-mvm-live/Cargo.toml --release -p spiralcore-mvm-adapter -p spiralcore-mvm-live > "$out/owned-clean.txt" 2>&1
chmod 700 "$out/coverage-wrapper.sh"
export RUSTC_WRAPPER="$out/coverage-wrapper.sh"
sha256sum "$RUSTC_WRAPPER" > "$out/wrapper.sha256"
"$cargo" +1.97.1 test --release --locked --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml --features live,conformance,hardware-sha256 --lib --tests --no-run --message-format=json > "$out/adapter-build.jsonl" 2> "$out/adapter-build.stderr"
"$cargo" +1.97.1 test --release --locked --manifest-path crates/spiralcore-mvm-live/Cargo.toml --features linux-host --lib --tests --no-run --message-format=json > "$out/live-build.jsonl" 2> "$out/live-build.stderr"
jq -s '[.[] | select(.reason=="compiler-artifact" and .executable!=null and .profile.test==true and
 ((.manifest_path|endswith("/crates/spiralcore-mvm-adapter/Cargo.toml")) or (.manifest_path|endswith("/crates/spiralcore-mvm-live/Cargo.toml")))) |
 {package:(if (.manifest_path|endswith("/crates/spiralcore-mvm-adapter/Cargo.toml")) then "adapter" else "live" end),target:.target.name,fresh,executable} |
 . + {label:(.package+"__"+.target)}] | unique_by(.executable)' "$out/adapter-build.jsonl" "$out/live-build.jsonl" > "$out/binaries.json"
jq -e 'length>0 and all(.[];.fresh==false) and ([.[].label]|unique|length)==length and ([.[].package]|unique|sort)==["adapter","live"]' "$out/binaries.json" >/dev/null
# Copy ALL executable objects before executing ANY test, outside Cargo custody.
jq -r '.[]|[.label,.executable]|@tsv' "$out/binaries.json" | while IFS=$'\t' read -r label binary; do
  copy="$out/bin/$label-$(basename "$binary")"
  cp -p "$binary" "$copy"
  chmod 500 "$copy"
  cmp "$binary" "$copy"
  sha256sum "$binary" >> "$out/binaries.sha256"
  sha256sum "$copy" >> "$out/binaries-retained.sha256"
done
jq --arg out "$out" '[.[] | . + {retained_executable: ($out+"/bin/"+.label+"-"+(.executable|split("/")|last))}]' "$out/binaries.json" > "$out/executed-binaries.json"
jq -r '.[]|[.label,.retained_executable]|@tsv' "$out/executed-binaries.json" | while IFS=$'\t' read -r label binary; do
  dir="$out/$label"
  mkdir -p "$dir/profiles"
  LLVM_PROFILE_FILE="$dir/profiles/%m-%p.profraw" "$binary" --test-threads=1 > "$dir/tests.txt" 2> "$dir/tests.stderr"
  [[ ! -s "$dir/tests.stderr" ]]
  [[ $(rg -c '^test result:' "$dir/tests.txt") == 1 ]]
  rg -q '^test result: ok\. [0-9]+ passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;' "$dir/tests.txt"
  passed=$(awk '/^test result:/ {print $4}' "$dir/tests.txt")
  jq -n --arg label "$label" --argjson passed "$passed" '{label:$label,passed:$passed,failed:0,ignored:0,filtered:0}' >> "$out/test-counts.jsonl"
  "$llvm/llvm-profdata" merge -sparse "$dir"/profiles/*.profraw -o "$dir/current.profdata"
  "$llvm/llvm-cov" export -format=lcov -instr-profile="$dir/current.profdata" -object "$binary" > "$dir/source.lcov" 2> "$dir/export.stderr"
  [[ ! -s "$dir/export.stderr" ]]
  printf '%s: ' "$label"
  rg '^test result:' "$dir/tests.txt"
done
jq -s '{binaries:length,passed:(map(.passed)|add),failed:(map(.failed)|add),ignored:(map(.ignored)|add),filtered:(map(.filtered)|add)}' "$out/test-counts.jsonl" > "$out/test-counts.json"
cat "$out"/*/source.lcov > "$out/combined.lcov"
awk '
 /^SF:/ {f=substr($0,4); sub(/^.*\/crates\//,"crates/",f); next}
 /^DA:/ && f ~ /^crates\/spiralcore-mvm-(adapter|live)\/src\// {
   split(substr($0,4),a,","); k=f SUBSEP a[1];
   if(!(k in hits) || a[2]+0>hits[k]) hits[k]=a[2]+0;
 }
 END {OFS="\t"; for(k in hits) {split(k,a,SUBSEP); print a[1],a[2],hits[k]}}
' "$out/combined.lcov" | sort -k1,1 -k2,2n > "$out/line-hits.tsv"
awk '
 /^SF:/ {f=substr($0,4); sub(/^.*\/crates\//,"crates/",f); next}
 /^BRDA:/ && f ~ /^crates\/spiralcore-mvm-(adapter|live)\/src\// {
   split(substr($0,6),a,","); k=f SUBSEP a[1] SUBSEP a[2] SUBSEP a[3];
   if(!(k in hits) || a[4]+0>hits[k]) hits[k]=a[4]+0;
 }
 END {OFS="\t"; for(k in hits) {split(k,a,SUBSEP); print a[1],a[2],a[3],a[4],hits[k]}}
' "$out/combined.lcov" | sort -k1,1 -k2,2n > "$out/branch-hits.tsv"
"$llvm/llvm-cov" --version > "$out/llvm-version.txt"
sha256sum -c ../qualified-source-sha256.txt > "$out/source-verification-after.txt"
sha256sum "$root/bin/broker_objects" > "$out/normal-binary-after.sha256"
cmp "$out/normal-binary-before.sha256" "$out/normal-binary-after.sha256"
cat "$out/test-counts.json"
