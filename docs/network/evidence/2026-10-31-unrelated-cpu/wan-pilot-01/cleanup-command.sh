#!/bin/bash
set -euo pipefail
cd /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
root=/Users/casey.allard/spiralcore/local/unrelated-cpu-20261031/run-01
evidence=docs/network/evidence/2026-10-31-unrelated-cpu/wan-pilot-01
jq -e '.status=="EXACT_PAIRED_TRANSFERS_PASS" and .operations==420 and .advance_to_wan==false' "$evidence/summary.json" >/dev/null
for file in "$root"/*.pid; do
  pid=$(cat "$file")
  if kill -0 "$pid" 2>/dev/null; then exit 1; fi
done
for shard in 0 1 2 3 4 5; do
  cache="$root/cache-$shard"
  [[ -d "$cache" && ! -L "$cache" ]]
  kib=$(du -sk "$cache" | awk '{print $1}')
  rm -rf -- "$cache"
  [[ ! -e "$cache" ]]
  jq -n --arg path "$cache" --argjson kib "$kib" '{owned_store:$path,removed:true,allocated_kib:$kib,objects:70,only_after_exact_reopen:true}' >> "$evidence/cleanup.jsonl"
done
gh variable list --repo R4-Laboratories/SpiralCore --json name | jq -e '[.[]|select(.name|startswith("SPIRALCORE_UNRELATED_"))]|length==0' > "$evidence/temporary-variables-absent.txt"
gh secret list --repo R4-Laboratories/SpiralCore --json name | jq -e '[.[]|select(.name=="SPIRALCORE_UNRELATED_TOKEN")]|length==0' > "$evidence/temporary-secret-absent.txt"
rm -- "$root/capability.txt"
[[ ! -e "$root/capability.txt" ]]
date -u +%FT%TZ > "$evidence/cleanup-completed-utc.txt"
df -k "$root" > "$evidence/disk-after.txt"
