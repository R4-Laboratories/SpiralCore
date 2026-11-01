#!/bin/bash
set -euo pipefail
evidence=$(cd "$(dirname "$0")" && pwd)
for phase in before-standard after-standard after-experimental64; do
  for result in "$evidence/$phase"/*/result.json; do
    [ -f "$result" ] || continue
    receipt=$(dirname "$result")
    [ "$(cat "$receipt/run.exit")" = 0 ]
    [ -f "$receipt/cleanup.json" ]
    jq --arg phase "$phase" --arg source "${result#"$evidence/"}" --slurpfile environment "$receipt/environment.json" -f "$evidence/extract.jq" "$result" > "$receipt/extracted.json"
  done
done
jq -cs '.[]|.session' "$evidence"/*/two-*/extracted.json > "$evidence/sessions.jsonl"
jq -cs '.[]|.operations[]' "$evidence"/*/two-*/extracted.json > "$evidence/operations.jsonl"
jq -s '{sessions:length,operations:(map(.operations)|add),exact_reopened:all(.[];.exact_reopened),exact_keyless_reboot:all(.[];.exact_keyless_reboot),by_phase:(group_by(.phase)|map({phase:.[0].phase,sessions:length,operations:(map(.operations)|add)}))}' "$evidence/sessions.jsonl" > "$evidence/completion.json"
jq -se 'group_by([.size,.trial,.kind])|all(.[]; ([.[].object_id]|unique|length)==1 and ([.[].object_bytes]|unique|length)==1)' "$evidence/operations.jsonl" > "$evidence/fixture-pairing.txt"
jq -n --slurpfile ops "$evidence/operations.jsonl" --slurpfile sessions "$evidence/sessions.jsonl" -f "$evidence/analyze.jq" > "$evidence/summary.json"
