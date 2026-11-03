#!/usr/bin/env bash
set -euo pipefail
root=/var/tmp/spiralcore-reconciliation-20261103
out="$root/coverage"
cd "$root/source"
[[ $(cat "$root/coverage-run.exit") == 0 ]]
expected=$(wc -l < ../qualified-source-sha256.txt)
[[ $(wc -l < "$out/source-verification.txt") == "$expected" ]]
[[ $(wc -l < "$out/source-verification-after.txt") == "$expected" ]]
cmp "$out/normal-binary-before.sha256" "$out/normal-binary-after.sha256"
while IFS=$'\t' read -r file _; do sha256sum "$file"; done < "$out/production-cutoffs.tsv" > "$out/source-files.sha256"
sha256sum ../qualified-source-sha256.txt > "$out/source-manifest.sha256"
/home/callard/.cargo/bin/rustc +1.97.1 -Vv > "$out/rustc.txt"
sha256sum -c "$out/binaries-retained.sha256" > "$out/binary-preservation-verification.txt"
jq -r '.[].label' "$out/executed-binaries.json" | while IFS= read -r label; do
  dir="$out/$label"
  { printf '%s\n' "$label"; rg '^test result:' "$dir/tests.txt"; wc -c "$dir/tests.stderr" "$dir/export.stderr"; } >> "$out/per-binary-tests.txt"
  sha256sum "$dir/current.profdata" "$dir/source.lcov" "$dir"/profiles/*.profraw >> "$out/remote-coverage-artifacts.sha256"
done
sha256sum "$out"/{adapter-build.jsonl,adapter-build.stderr,live-build.jsonl,live-build.stderr,combined.lcov,line-hits.tsv,branch-hits.tsv,collect.sh,derive.sh,finalize.sh,run.sh,compact.sh,changed-source.diff,production-cutoffs.tsv,source-pins.txt} > "$out/remote-build-artifacts.sha256"
awk '/warning:/ {n++} END {print n+0}' "$out/adapter-build.stderr" "$out/live-build.stderr" > "$out/compiler-warning-count.txt"
printf 'source_manifest_before=%s/%s\nsource_manifest_after=%s/%s\nnormal_binary_hash_unchanged=true\ncollection_exit=0\nchanged_line_gate_exit=%s\nselective_instrumentation=adapter,live\n' "$expected" "$expected" "$expected" "$expected" "$(cat "$out/derive.exit")" > "$out/verification.txt"
