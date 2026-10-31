#!/bin/bash
set -euo pipefail
cd /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
root=/Users/casey.allard/spiralcore/local/unrelated-cpu-20261031/run-01
evidence=docs/network/evidence/2026-10-31-unrelated-cpu/wan-pilot-01
repo=R4-Laboratories/SpiralCore
source_sha=$(jq -r '.source_commit' "$evidence/plan-0.json")
[[ $(git rev-parse HEAD) == "$source_sha" ]]
git diff --exit-code
# Only the diagnostic workflow declaration and its test changed after freeze.
awk '$2 != ".github/workflows/spiralcore-unrelated-cpu.yml" && $2 != "crates/spiralcore-transfer/tests/unrelated_cpu_workflow.rs"' docs/network/evidence/2026-10-31-unrelated-cpu/qualification/frozen-phase2/source-files.sha256 > "$evidence/unchanged-source-files.sha256"
shasum -a 256 -c "$evidence/unchanged-source-files.sha256" > "$evidence/source-before-launch.txt"
shasum -a 256 .github/workflows/spiralcore-unrelated-cpu.yml crates/spiralcore-transfer/tests/unrelated_cpu_workflow.rs > "$evidence/diagnostic-declaration.sha256"
jq -e '.advance_to_wan == false and .operations == 700 and .status == "EXACT_PAIRED_TRANSFERS_PASS" and (.local_gates|length==2) and (.local_gates|all(.warm_passed and .lifecycle_passed)) and .local_gates[0].replacement_passed == true and .local_gates[1].replacement_passed == false' docs/network/evidence/2026-10-31-unrelated-cpu/local-phase2/summary.json > "$evidence/failed-local-gate-confirmed.txt"
test -f docs/network/evidence/2026-10-31-unrelated-cpu/wan-diagnostic-contract.md
receiver=$(shasum -a 256 /Users/casey.allard/spiralcore/local/unrelated-cpu-20261031/frozen-phase2/spiralcore-wan | awk '{print $1}')
campaign=$(cat "$root/run-id.txt")
urls=()
for shard in 0 1 2 3 4 5; do
  url=$(rg -o 'https://[a-z0-9-]+\.trycloudflare\.com' "$root/tunnel-$shard.log" | head -1)
  [[ "$url" =~ ^https://[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?\.trycloudflare\.com$ ]]
  urls+=("$url")
  response=$(curl --silent --show-error --max-time 20 -o /dev/null -w '%{http_code}' "$url/")
  printf '%s %s %s\n' "$shard" "$url" "$response" >> "$evidence/preflight.txt"
  [[ "$response" == 400 ]]
done
gh variable set SPIRALCORE_UNRELATED_SOURCE --repo "$repo" --body "$source_sha"
gh variable set SPIRALCORE_UNRELATED_RUN --repo "$repo" --body "$campaign"
gh variable set SPIRALCORE_UNRELATED_RECEIVER_SHA256 --repo "$repo" --body "$receiver"
for shard in 0 1 2 3 4 5; do
  gh variable set "SPIRALCORE_UNRELATED_URL_$shard" --repo "$repo" --body "${urls[$shard]}"
done
gh secret set SPIRALCORE_UNRELATED_TOKEN --repo "$repo" < "$root/capability.txt"
jq -n --arg source "$source_sha" --arg run "$campaign" --arg receiver "$receiver" \
  --arg url0 "${urls[0]}" --arg url1 "${urls[1]}" --arg url2 "${urls[2]}" \
  --arg url3 "${urls[3]}" --arg url4 "${urls[4]}" --arg url5 "${urls[5]}" \
  '{schema:"spiralcore.unrelated-cpu.launch.v1",source:$source,run:$run,receiver_executable_sha256:$receiver,urls:[$url0,$url1,$url2,$url3,$url4,$url5],source_attempt:1,operations:420,paired_trials_per_size:6,local_advancement:false,designation:"one six-pair WAN diagnostic; local gates failed; not accepted",preflight:"one unauthenticated GET rejected400 at each origin; before timed campaign",limits:"relayed WAN diagnostic; HTTP-body bytes, not complete network traffic"}' > "$evidence/launch.json"
git push -u origin codex/spiralcore-unrelated-cpu
date -u +%FT%TZ > "$evidence/pushed-utc.txt"
