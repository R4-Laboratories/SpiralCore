#!/usr/bin/env bash
# Root starts this ONLY after normal release/performance is complete.
set -euo pipefail
root=/var/tmp/spiralcore-reconciliation-20261103
[[ ! -e "$root/coverage-run.txt" && ! -e "$root/coverage" ]]
set +e
bash "$root/coverage-scripts/collect.sh" > "$root/coverage-run.txt" 2>&1
collected=$?
printf '%s\n' "$collected" > "$root/coverage-run.exit"
if [[ "$collected" != 0 ]]; then exit "$collected"; fi
bash "$root/coverage/derive.sh" > "$root/coverage/derive-output.txt" 2>&1
derived=$?
printf '%s\n' "$derived" > "$root/coverage/derive.exit"
# Preserve custody even when the derived changed-line gate is below 80%.
bash "$root/coverage/finalize.sh" > "$root/coverage/finalize-output.txt" 2>&1
finalized=$?
printf '%s\n' "$finalized" > "$root/coverage/finalize.exit"
cat "$root/coverage/test-counts.json" "$root/coverage/summary.tsv" "$root/coverage/branch-summary.tsv"
if [[ "$derived" != 0 ]]; then exit "$derived"; fi
exit "$finalized"
