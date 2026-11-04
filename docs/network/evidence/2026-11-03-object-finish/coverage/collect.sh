#!/bin/bash
# One selective gateway collection. No dependency instrumentation or new target.
set -euo pipefail
root=/var/tmp/spiralcore-object-finish-20261103
out=$root/coverage
target=/var/tmp/spiralcore-mvm-service-20261102/target
export PATH=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin:/usr/bin:/bin
llvm=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin
test ! -e "$out"
mkdir -m 700 "$out"
mkdir "$out/normal-bin" "$out/bin"
cd "$root/source"
sha256sum -c ../validation/finish-final2-before.sha256 > "$out/normal-source-verification.txt"
if fuser "$target/release/.cargo-lock" > "$out/owned-target-open-pids.txt" 2>/dev/null; then
  echo 'Owned Cargo target is in use; no cleanup/build performed.' >&2
  exit 88
fi
find crates -type f -not -name '._*' -not -path '*/target/*' -print0 | sort -z | xargs -0 sha256sum > "$out/source-before.sha256"
# Preserve all actually executed normal test objects and normal gateway bins.
sed -n 's/^     Running .* (\(.*\))$/\1/p' ../validation/finish-release2.txt > "$out/normal-executed-paths.txt"
for name in spiralcore-gateway spiralcore-gateway-object spiralcore-gateway-probe; do
  test ! -x "$target/release/$name" || printf '%s\n' "$target/release/$name" >> "$out/normal-executed-paths.txt"
done
while IFS= read -r binary; do
  cp -p "$binary" "$out/normal-bin/$(basename "$binary")"
  chmod 500 "$out/normal-bin/$(basename "$binary")"
  cmp "$binary" "$out/normal-bin/$(basename "$binary")"
  sha256sum "$out/normal-bin/$(basename "$binary")" >> "$out/normal-binaries.sha256"
done < "$out/normal-executed-paths.txt"
export CARGO_TARGET_DIR="$target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTDOCFLAGS RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER LLVM_PROFILE_FILE
cargo clean --manifest-path crates/spiralcore-gateway/Cargo.toml --release -p spiralcore-gateway > "$out/owned-clean.txt" 2>&1
cp "$root/coverage-scripts/coverage-wrapper.sh" "$out/coverage-wrapper.sh"
chmod 700 "$out/coverage-wrapper.sh"
export RUSTC_WRAPPER="$out/coverage-wrapper.sh"
sha256sum "$RUSTC_WRAPPER" > "$out/wrapper.sha256"
cargo test --release --locked --manifest-path crates/spiralcore-gateway/Cargo.toml --all-targets --no-run --message-format=json > "$out/build.jsonl" 2> "$out/build.stderr"
jq -s '[.[]|select(.reason=="compiler-artifact" and .executable!=null and .profile.test==true and (.manifest_path|endswith("/crates/spiralcore-gateway/Cargo.toml")))|{target:.target.name,fresh,executable}]' "$out/build.jsonl" > "$out/binaries.json"
jq -e 'length==14 and all(.[];.fresh==false)' "$out/binaries.json" >/dev/null
jq -r '.[].executable' "$out/binaries.json" | while IFS= read -r binary; do
  copy="$out/bin/$(basename "$binary")"
  cp -p "$binary" "$copy"
  chmod 500 "$copy"
  cmp "$binary" "$copy"
  sha256sum "$copy" >> "$out/binaries-retained.sha256"
done
jq --arg out "$out" '[.[]|.+{retained_executable:($out+"/bin/"+(.executable|split("/")|last))}]' "$out/binaries.json" > "$out/executed-binaries.json"
jq -r '.[]|[.target,.retained_executable]|@tsv' "$out/executed-binaries.json" | while IFS=$'\t' read -r name binary; do
  dir="$out/$(basename "$binary")"
  mkdir -p "$dir/profiles" "$dir/evidence"
  LLVM_PROFILE_FILE="$dir/profiles/%m-%p.profraw" SPIRALCORE_OBJECT_FINISH_EVIDENCE_DIR="$dir/evidence" "$binary" --test-threads=1 > "$dir/tests.txt" 2> "$dir/tests.stderr"
  test ! -s "$dir/tests.stderr"
  grep -Eq '^test result: ok\. [0-9]+ passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;' "$dir/tests.txt"
  passed=$(awk '/^test result:/ {print $4}' "$dir/tests.txt")
  jq -n --arg target "$name" --argjson passed "$passed" '{target:$target,passed:$passed,failed:0,ignored:0,filtered:0}' >> "$out/test-counts.jsonl"
  "$llvm/llvm-profdata" merge -sparse "$dir"/profiles/*.profraw -o "$dir/current.profdata"
  "$llvm/llvm-cov" export -format=lcov -instr-profile="$dir/current.profdata" -object "$binary" > "$dir/source.lcov" 2> "$dir/export.stderr"
  test ! -s "$dir/export.stderr"
  grep '^test result:' "$dir/tests.txt"
done
jq -s '{binaries:length,passed:(map(.passed)|add),failed:(map(.failed)|add),ignored:(map(.ignored)|add),filtered:(map(.filtered)|add)}' "$out/test-counts.jsonl" > "$out/test-counts.json"
cat "$out"/*/source.lcov > "$out/combined.lcov"
awk '
 /^SF:/ {f=substr($0,4); sub(/^.*\/crates\//,"crates/",f); next}
 /^DA:/ && f ~ /^crates\/spiralcore-gateway\/src\// {
   split(substr($0,4),a,","); k=f SUBSEP a[1]; if(!(k in hits)||a[2]+0>hits[k])hits[k]=a[2]+0
 }
 END {OFS="\t";for(k in hits){split(k,a,SUBSEP);print a[1],a[2],hits[k]}}
' "$out/combined.lcov" | sort -k1,1 -k2,2n > "$out/line-hits.tsv"
awk '
 /^SF:/ {f=substr($0,4); sub(/^.*\/crates\//,"crates/",f); next}
 /^BRDA:/ && f ~ /^crates\/spiralcore-gateway\/src\// {
   split(substr($0,6),a,","); k=f SUBSEP a[1] SUBSEP a[2] SUBSEP a[3]; if(!(k in hits)||a[4]+0>hits[k])hits[k]=a[4]+0
 }
 END{OFS="\t";for(k in hits){split(k,a,SUBSEP);print a[1],a[2],a[3],a[4],hits[k]}}
' "$out/combined.lcov" | sort -k1,1 -k2,2n > "$out/branch-hits.tsv"
for file in object_channel.rs transport.rs object_service.rs; do printf 'crates/spiralcore-gateway/src/%s\t1000000\n' "$file"; done > "$out/production-cutoffs.tsv"
cp "$root/coverage-scripts/changed-source.diff" "$out/changed-source.diff"
cp "$root/coverage-scripts/derive.sh" "$out/derive.sh"
bash "$out/derive.sh" > "$out/derive-output.txt" 2>&1
sha256sum -c "$out/source-before.sha256" > "$out/source-after-verification.txt"
sha256sum -c "$out/normal-binaries.sha256" > "$out/normal-binaries-after-verification.txt"
"$llvm/llvm-cov" --version > "$out/llvm-version.txt"
cat "$out/test-counts.json"
cat "$out/derive-output.txt"
