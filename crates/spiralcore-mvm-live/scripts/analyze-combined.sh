#!/usr/bin/env bash
set -euo pipefail
[[ $# == 1 ]] || { printf 'usage: analyze-combined.sh RUN_ROOT\n' >&2; exit 2; }
here=$(cd -- "$(dirname -- "$0")" && pwd)
root=$1
[[ -f "$root/schedule.json" ]] || { printf 'missing schedule\n' >&2; exit 2; }
jq -e 'length==36 and all(.[]; (.name|test("^(one|two)-(1048576|16777216)-t[012]-(combined|combined-cpu|whole-mixed)$")))' "$root/schedule.json" >/dev/null
scratch=$(mktemp "${TMPDIR:-/tmp}/spiralcore-combined-rows.XXXXXX")
trap 'rm -f -- "$scratch"' EXIT
while IFS= read -r item; do
    name=$(jq -r .name <<< "$item")
    jq -n --argjson expected "$item" --arg source "$name/result.json" \
       --slurpfile r "$root/$name/result.json" --slurpfile host "$root/$name/host-events.jsonl" \
       -f "$here/combined-session.jq" >> "$scratch"
done < <(jq -c '.[]' "$root/schedule.json")
jq -s -f "$here/combined-analysis.jq" "$scratch"
