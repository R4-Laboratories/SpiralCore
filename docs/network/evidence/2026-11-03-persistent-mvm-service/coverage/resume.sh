#!/usr/bin/env bash
# Resume only after the successful coverage build; no clean or Cargo invocation.
# Parser diagnosis: jq -rs slurps JSON values into an array, so split was invalid.
# Correct parsing uses jq -s and selects artifact values directly from .[].
set -euo pipefail
root=/var/tmp/spiralcore-persistent-20261103
out="$root/coverage"
llvm=/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin
cd "$root/source"
sha256sum --status -c ../source-sha256.txt
[[ -s "$out/build.jsonl" && ! -e "$out/binaries.sha256" ]]
jq -s '[.[] | select(.reason=="compiler-artifact" and .executable!=null and .profile.test==true and (.manifest_path|endswith("/crates/spiralcore-mvm-live/Cargo.toml"))) | {target:.target.name,fresh,executable}]' "$out/build.jsonl" > "$out/binaries.json"
jq -e 'length==3 and all(.[];.fresh==false)' "$out/binaries.json" >/dev/null
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
 /^DA:/ && f ~ /^crates\/spiralcore-mvm-live\/src\// {
   split(substr($0,4),a,","); k=f SUBSEP a[1];
   if(!(k in hits) || a[2]+0>hits[k]) hits[k]=a[2]+0;
 }
 END {OFS="\t"; for(k in hits) {split(k,a,SUBSEP); print a[1],a[2],hits[k]}}
' "$out/combined.lcov" | sort -k1,1 -k2,2n > "$out/line-hits.tsv"
"$llvm/llvm-cov" --version > "$out/llvm-version.txt"
printf 'PASS: three isolated test binaries and warning-free exports; derive scoped production/changed coverage separately.\n'
