#!/usr/bin/env bash
# Compact receipts only; profiles, profdata, LCOV and binaries remain on Wick.
set -euo pipefail
root=/var/tmp/spiralcore-session-idle-20261103
out="$root/coverage"
cd "$root/source"
sha256sum crates/spiralcore-mvm-adapter/src/live.rs > "$out/source-files.sha256"
sha256sum ../qualified-source-sha256.txt > "$out/source-manifest.sha256"
/home/callard/.cargo/bin/rustc +1.97.1 -Vv > "$out/rustc.txt"
[[ $(cat "$root/coverage-run.exit") == 0 ]]
[[ $(wc -l < "$out/source-verification.txt") == 285 ]]
[[ $(wc -l < "$out/source-verification-after.txt") == 285 ]]
cmp "$out/normal-binaries-before.sha256" "$out/normal-binaries-after.sha256"
sha256sum -c "$out/binaries-retained.sha256" > "$out/binary-preservation-verification.txt"
for name in adapter_live service_tcp spiralcore_mvm_live campaign; do
  dir="$out/$name"
  {
    printf '%s\n' "$name"
    rg '^test result:' "$dir/tests.txt"
    wc -c "$dir/tests.stderr" "$dir/export.stderr"
  } >> "$out/per-binary-tests.txt"
  sha256sum "$dir/current.profdata" "$dir/source.lcov" "$dir"/profiles/*.profraw >> "$out/remote-coverage-artifacts.sha256"
done
sha256sum "$out"/{adapter-build.jsonl,adapter-build.stderr,live-build.jsonl,live-build.stderr,combined.lcov,line-hits.tsv,branch-hits.tsv,collect.sh,derive.sh,finalize.sh,changed-source.diff} > "$out/remote-build-artifacts.sha256"
printf 'source_manifest_before=285/285\nsource_manifest_after=285/285\nnormal_binary_hashes_unchanged=true\ncoverage_exit=0\nselective_instrumentation=adapter,live\n' > "$out/verification.txt"
