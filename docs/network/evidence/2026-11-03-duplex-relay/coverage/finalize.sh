#!/usr/bin/env bash
# Compact receipts only. Profiles and LCOV stay on Wick.
set -euo pipefail
root=/var/tmp/spiralcore-duplex-20261103
out="$root/coverage"
cd "$root/source"
sha256sum crates/spiralcore-mvm-adapter/src/live.rs crates/spiralcore-mvm-live/src/orchestrator.rs crates/spiralcore-mvm-live/src/orchestrator/duplex.rs > "$out/source-files.sha256"
sha256sum ../source-sha256.txt > "$out/source-manifest.sha256"
/home/callard/.cargo/bin/rustc +1.97.1 -Vv > "$out/rustc.txt"
[[ $(cat "$root/coverage-run.exit") == 0 ]]
[[ $(wc -l < "$out/source-verification.txt") == 278 ]]
[[ $(wc -l < "$out/source-verification-after.txt") == 278 ]]
cmp "$out/normal-binaries-before.sha256" "$out/normal-binaries-after.sha256"
for dir in "$out"/*/; do
  if [[ -f "$dir/tests.txt" ]]; then
    {
      basename "$dir"
      rg '^test result:' "$dir/tests.txt"
      wc -c "$dir/tests.stderr" "$dir/export.stderr"
    } >> "$out/per-binary-tests.txt"
    sha256sum "$dir/current.profdata" "$dir/source.lcov" "$dir"/profiles/*.profraw >> "$out/remote-coverage-artifacts.sha256"
  fi
done
sha256sum "$out"/adapter-build.jsonl "$out"/adapter-build.stderr "$out"/live-build.jsonl "$out"/live-build.stderr "$out"/combined.lcov "$out"/line-hits.tsv "$root"/coverage-collect.sh > "$out/remote-build-artifacts.sha256"
printf 'source_manifest_before=278/278\nsource_manifest_after=278/278\nnormal_binary_hashes_unchanged=true\ncoverage_exit=0\n' > "$out/verification.txt"
