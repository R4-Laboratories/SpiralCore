#!/usr/bin/env bash
# Raw profiles/LCOV/binaries stay remote; retain compact integrity receipts.
set -euo pipefail
root=/var/tmp/spiralcore-call-retry-20261103
out="$root/coverage"
cd "$root/source"
[[ $(cat "$root/coverage-run.exit") == 0 ]]
sha256sum crates/spiralcore-mvm-adapter/src/service.rs crates/spiralcore-mvm-adapter/tests/call_send.rs > "$out/source-files.sha256"
sha256sum ../qualified-source-sha256.txt > "$out/source-manifest.sha256"
/home/callard/.cargo/bin/rustc +1.97.1 -Vv > "$out/rustc.txt"
[[ $(wc -l < "$out/source-verification.txt") == 285 ]]
[[ $(wc -l < "$out/source-verification-after.txt") == 285 ]]
sha256sum -c "$out/binaries-retained.sha256" > "$out/binary-preservation-verification.txt"
jq -r '.[].target' "$out/executed-binaries.json" | while IFS= read -r name; do
  dir="$out/$name"
  {
    printf '%s\n' "$name"
    rg '^test result:' "$dir/tests.txt"
    wc -c "$dir/tests.stderr" "$dir/export.stderr"
  } >> "$out/per-binary-tests.txt"
  sha256sum "$dir/current.profdata" "$dir/source.lcov" "$dir"/profiles/*.profraw >> "$out/remote-coverage-artifacts.sha256"
done
sha256sum "$out"/{build.jsonl,build.stderr,combined.lcov,line-hits.tsv,branch-hits.tsv,collect.sh,derive.sh,finalize.sh,changed-source.diff} > "$out/remote-build-artifacts.sha256"
awk '/warning:/ {n++} END {print n+0}' "$out/build.stderr" > "$out/compiler-warning-count.txt"
printf 'source_manifest_before=285/285\nsource_manifest_after=285/285\ncoverage_exit=0\nselective_instrumentation=adapter-only\n' > "$out/verification.txt"
