#!/usr/bin/env bash
# Adapted from duplex coverage-final: selective instrumentation, isolated exports.
set -euo pipefail
root=/var/tmp/spiralcore-session-events-20261103
out="$root/coverage"
target=/var/tmp/spiralcore-mvm-service-20261102/target
cargo=/home/callard/.cargo/bin/cargo
llvm=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin
[[ ! -e "$out" ]]
mkdir -m 700 "$out"
mkdir "$out/bin"
cd "$root/source"
sha256sum -c ../qualified-source-sha256.txt > "$out/source-verification.txt"
sha256sum "$root"/bin/{host,guest,broker_objects} > "$out/normal-binaries-before.sha256"
stat -c '%a %n' "$root"/bin/{host,guest,broker_objects} > "$out/normal-binary-modes.txt"
[[ $(awk '$1!=500 {bad++} END {print bad+0}' "$out/normal-binary-modes.txt") == 0 ]]
ps -eo pid,comm | awk '$2=="cargo" || $2=="rustc"' > "$out/active-builds-before.txt"
[[ ! -s "$out/active-builds-before.txt" ]]
export CARGO_TARGET_DIR="$target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER
"$cargo" +1.97.1 clean --manifest-path crates/spiralcore-mvm-live/Cargo.toml --release -p spiralcore-mvm-live -p spiralcore-mvm-adapter > "$out/owned-clean.txt" 2>&1
export RUSTC_WRAPPER=/var/tmp/spiralcore-sustained-20261102/coverage-d940203/coverage-wrapper.sh
sha256sum "$RUSTC_WRAPPER" > "$out/wrapper.sha256"
cp "$RUSTC_WRAPPER" "$out/coverage-wrapper.sh"
"$cargo" +1.97.1 test --release --locked --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml --features live --test receipt --test service_tcp --no-run --message-format=json > "$out/adapter-build.jsonl" 2> "$out/adapter-build.stderr"
"$cargo" +1.97.1 test --release --locked --manifest-path crates/spiralcore-mvm-live/Cargo.toml --features linux-host --lib --test campaign --no-run --message-format=json > "$out/live-build.jsonl" 2> "$out/live-build.stderr"
jq -s '[.[] | select(.reason=="compiler-artifact" and .executable!=null and .profile.test==true and ((.manifest_path|endswith("/crates/spiralcore-mvm-adapter/Cargo.toml")) or (.manifest_path|endswith("/crates/spiralcore-mvm-live/Cargo.toml")))) | {target:.target.name,fresh,executable}]' "$out/adapter-build.jsonl" "$out/live-build.jsonl" > "$out/binaries.json"
jq -e 'length==4 and all(.[];.fresh==false) and ([.[].target]|sort)==(["receipt","service_tcp","spiralcore_mvm_live","campaign"]|sort)' "$out/binaries.json" >/dev/null
# Retain the exact instrumented bytes BEFORE executing them or reusing Cargo.
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
  case "$name" in receipt) expected=21 ;; service_tcp) expected=4 ;; spiralcore_mvm_live) expected=57 ;; campaign) expected=9 ;; *) exit 1 ;; esac
  [[ $(rg -c '^test result:' "$dir/tests.txt") == 1 ]]
  rg -q "^test result: ok\. $expected passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;" "$dir/tests.txt"
  jq -n --arg target "$name" --argjson passed "$expected" '{target:$target,passed:$passed,failed:0,ignored:0,filtered:0}' >> "$out/test-counts.jsonl"
  "$llvm/llvm-profdata" merge -sparse "$dir"/profiles/*.profraw -o "$dir/current.profdata"
  "$llvm/llvm-cov" export -format=lcov -instr-profile="$dir/current.profdata" -object "$binary" > "$dir/source.lcov" 2> "$dir/export.stderr"
  [[ ! -s "$dir/export.stderr" ]]
  rg '^test result:' "$dir/tests.txt"
done
jq -s '{binaries:length,passed:(map(.passed)|add),failed:(map(.failed)|add),ignored:(map(.ignored)|add),filtered:(map(.filtered)|add)}' "$out/test-counts.jsonl" > "$out/test-counts.json"
jq -e '.binaries==4 and .passed==91 and .failed==0 and .ignored==0 and .filtered==0' "$out/test-counts.json" >/dev/null
cat "$out"/{receipt,service_tcp,spiralcore_mvm_live,campaign}/source.lcov > "$out/combined.lcov"
awk '
 /^SF:/ {f=substr($0,4); sub(/^.*\/crates\//,"crates/",f); next}
 /^DA:/ && f ~ /^crates\/spiralcore-mvm-(live|adapter)\/src\// {
   split(substr($0,4),a,","); k=f SUBSEP a[1];
   if(!(k in hits) || a[2]+0>hits[k]) hits[k]=a[2]+0;
 }
 END {OFS="\t"; for(k in hits) {split(k,a,SUBSEP); print a[1],a[2],hits[k]}}
' "$out/combined.lcov" | sort -k1,1 -k2,2n > "$out/line-hits.tsv"
awk '
 /^SF:/ {f=substr($0,4); sub(/^.*\/crates\//,"crates/",f); next}
 /^BRDA:/ && f ~ /^crates\/spiralcore-mvm-(live|adapter)\/src\// {
   split(substr($0,6),a,","); k=f SUBSEP a[1] SUBSEP a[2] SUBSEP a[3];
   if(!(k in hits) || a[4]+0>hits[k]) hits[k]=a[4]+0;
 }
 END {OFS="\t"; for(k in hits) {split(k,a,SUBSEP); print a[1],a[2],a[3],a[4],hits[k]}}
' "$out/combined.lcov" | sort -k1,1 -k2,2n > "$out/branch-hits.tsv"
for file in crates/spiralcore-mvm-adapter/src/receipt.rs crates/spiralcore-mvm-live/src/campaign/canonical.rs crates/spiralcore-mvm-live/src/campaign/session.rs; do
  cutoff=$(awk '/^#\[cfg\(test\)\]/ {print NR; found=1; exit} END {if(!found) print 1000000}' "$file")
  printf '%s\t%s\n' "$file" "$cutoff" >> "$out/production-cutoffs.tsv"
done
"$llvm/llvm-cov" --version > "$out/llvm-version.txt"
sha256sum "$root"/bin/{host,guest,broker_objects} > "$out/normal-binaries-after.sha256"
cmp "$out/normal-binaries-before.sha256" "$out/normal-binaries-after.sha256"
sha256sum -c ../qualified-source-sha256.txt > "$out/source-verification-after.txt"
printf 'PASS: 91 tests; four fresh protected instrumented binaries; isolated warning-free exports.\n'
