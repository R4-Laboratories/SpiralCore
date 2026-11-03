#!/usr/bin/env bash
# Export only compact receipts; no raw objects, profiles, profdata or LCOV.
set -euo pipefail
root=/var/tmp/spiralcore-reconciliation-20261103
cd "$root/coverage"
[[ ! -e "$root/coverage-compact.tgz" ]]
files=(collect.sh derive.sh finalize.sh run.sh compact.sh coverage-wrapper.sh production-cutoffs.tsv changed-source.diff source-pins.txt
 source-files.sha256 source-manifest.sha256 source-verification.txt source-verification-after.txt active-builds-before.txt
 owned-clean.txt normal-binary-before.sha256 normal-binary-after.sha256 normal-binary-mode.txt wrapper.sha256 binaries.json executed-binaries.json binaries.sha256 binaries-retained.sha256
 binary-preservation-verification.txt test-counts.json test-counts.jsonl per-binary-tests.txt summary.tsv summary.json
 focused-lines.tsv uncovered-changed-lines.tsv uncovered-production-lines.tsv branch-summary.tsv focused-branches.tsv
 verification.txt remote-coverage-artifacts.sha256 remote-build-artifacts.sha256 compiler-warning-count.txt rustc.txt
 llvm-version.txt adapter-build.stderr live-build.stderr derive-output.txt derive.exit finalize-output.txt finalize.exit)
while IFS= read -r label; do files+=("$label/tests.txt" "$label/tests.stderr" "$label/export.stderr"); done < <(jq -r '.[].label' executed-binaries.json)
tar -czf "$root/coverage-compact.tgz" "${files[@]}"
sha256sum "$root/coverage-compact.tgz"
