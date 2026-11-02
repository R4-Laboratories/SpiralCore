#!/usr/bin/env bash
set -euo pipefail
here=$(cd -- "$(dirname -- "$0")" && pwd)
repo=$(git -C "$here" rev-parse --show-toplevel)
# Validator fixture only: duplicate the retained baseline under two labels.
# These synthetic labels are never measurement evidence.
script="$repo/crates/spiralcore-mvm-live/scripts/v1-comparison.jq"
input=$(mktemp)
trap 'rm -f "$input"' EXIT
jq -s '[.[] | select(.session.arm=="combined-cpu") | . as $x | ["legacy","v1"][] as $i | $x | .session.implementation=$i | .session.actual_frames_sent=1 | .session.actual_frames_received=1 | .operation_rows |= map(.implementation=$i | .actual_frames_sent=1 | .actual_frames_received=1)]' "$repo"/docs/network/evidence/2026-11-02-linux-mvm/pilot36/*/verified-summary.json > "$input"
jq -e -f "$script" "$input" >/dev/null
if jq '.[0].operation_rows[0].planned_application_bytes += 1' "$input" | jq -e -f "$script" >/dev/null 2>&1; then
  printf 'FAIL: differing wire frame bytes accepted\n' >&2
  exit 1
fi
if jq '.[0].operation_rows[0].object_id="poisoned"' "$input" | jq -e -f "$script" >/dev/null 2>&1; then
  printf 'FAIL: differing object identity accepted\n' >&2
  exit 1
fi
if jq '.[0]=.[1]' "$input" | jq -e -f "$script" >/dev/null 2>&1; then
  printf "FAIL: duplicate/missing registration cell accepted\n" >&2
  exit 1
fi
if jq '.[0].operation_rows[0].actual_frames_sent += 1' "$input" | jq -e -f "$script" >/dev/null 2>&1; then
  printf "FAIL: differing actual frame counts accepted\n" >&2
  exit 1
fi
printf 'PASS: valid synthetic legacy/v1 fixture; rejects altered frame bytes, identities and duplicate cells\n'
