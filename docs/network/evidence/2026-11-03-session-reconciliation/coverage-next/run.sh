#!/usr/bin/env bash
# Root starts this ONLY after normal release/performance is complete.
set -euo pipefail
root=/var/tmp/spiralcore-reconciliation-20261103
[[ ! -e "$root/coverage-next-run.txt" && ! -e "$root/coverage-next" ]]
set +e
bash "$root/coverage-next-scripts/collect.sh" > "$root/coverage-next-run.txt" 2>&1
collected=$?
printf '%s\n' "$collected" > "$root/coverage-next-run.exit"
if [[ "$collected" != 0 ]]; then exit "$collected"; fi
bash "$root/coverage-next/derive.sh" > "$root/coverage-next/derive-output.txt" 2>&1
derived=$?
printf '%s\n' "$derived" > "$root/coverage-next/derive.exit"
# Preserve custody even when the derived changed-line gate is below 80%.
bash "$root/coverage-next/finalize.sh" > "$root/coverage-next/finalize-output.txt" 2>&1
finalized=$?
printf '%s\n' "$finalized" > "$root/coverage-next/finalize.exit"
cat "$root/coverage-next/test-counts.json" "$root/coverage-next/summary.tsv" "$root/coverage-next/branch-summary.tsv"
if [[ "$derived" != 0 ]]; then exit "$derived"; fi
exit "$finalized"
