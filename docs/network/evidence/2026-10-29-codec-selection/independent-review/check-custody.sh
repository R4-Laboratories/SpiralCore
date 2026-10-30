#!/bin/bash
# Run from the repository root after the registered run completes.
set -eu
review=docs/network/evidence/2026-10-29-codec-selection/independent-review
evidence=docs/network/evidence/2026-10-29-codec-selection
source_revision=850272f01d580d1cf7564d25fb628fd50f58d956
binary=/Users/casey.allard/spiralcore/local/codec-selection-20261029/bin/codec_selection
git diff --exit-code "$source_revision" -- crates
shasum -a 256 -c "$evidence/source-files.sha256" > "$review/source-verification.txt"
printf '0\n' > "$review/source-verification.exit"
test "$(wc -l < "$evidence/source-files.sha256" | tr -d ' ')" = 60
test "$(shasum -a 256 "$binary" | cut -d ' ' -f 1)" = c165a97205f858578f3c25d02464a725503660e21cf7c318811201f548c4d3fd
: > "$review/cache-roots.jsonl"
for lane in smoke measurement; do
  root=/Users/casey.allard/spiralcore/local/codec-selection-20261029/${lane}-cache
  if test "$lane" = smoke; then data="$evidence/smoke.jsonl"; expected=120; else data="$evidence/measurements.jsonl"; expected=1200; fi
  owner=$(cat "$root/.selector-owner")
  start=$(head -1 "$data" | jq -r .started_unix_nanos)
  count=$(find "$root" -mindepth 1 -maxdepth 1 -print | wc -l | tr -d ' ')
  test "$owner" = "$start"
  test "$count" = 1
  actual=$(jq -s '[.[] | select(.event=="cleanup" and .status=="removed-after-all-phase-reopen-checks")]|length' "$data")
  test "$actual" = "$expected"
  jq -nc --arg root "$root" --arg owner "$owner" --arg lane "$lane" --argjson count "$count" --argjson cleanup_rows "$actual" \
    '{lane:$lane,root:$root,entries:$count,only_entry:".selector-owner",owner:$owner,owner_matches_registration:true,successful_cleanup_rows:$cleanup_rows,status:"PASS"}' >> "$review/cache-roots.jsonl"
done
jq -n --arg version "$(jq --version)" --arg source "$source_revision" \
  --slurpfile roots "$review/cache-roots.jsonl" \
  '{schema:"spiralcore.selector.custody-review.v1",status:"PASS",source_revision:$source,source_manifest_files:60,source_manifest_check_exit:0,runtime_diff_exit:0,executable_sha256:"c165a97205f858578f3c25d02464a725503660e21cf7c318811201f548c4d3fd",jq:$version,cache_roots:$roots}' > "$review/custody.json"
