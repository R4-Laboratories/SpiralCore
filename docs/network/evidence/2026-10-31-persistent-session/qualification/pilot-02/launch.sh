#!/bin/bash
set -euo pipefail
cd /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
root=/Users/casey.allard/spiralcore/local/persistent-session-20261031/run-02
evidence=docs/network/evidence/2026-10-31-persistent-session/wan-pilot-02
repo=R4-Laboratories/SpiralCore
source_sha=9a6d7f758fb724b848ca1f24c1c27f5088da671d
[[ $(git rev-parse HEAD) == "$source_sha" ]]
git diff --exit-code
jq -e '.advance_to_wan == true and .operations == 660' docs/network/evidence/2026-10-31-persistent-session/local/summary.json > /dev/null
receiver=$(shasum -a 256 /Users/casey.allard/spiralcore/local/persistent-session-20261031/diagnostic-transport-frozen/spiralcore-wan | awk '{print $1}')
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
gh variable set SPIRALCORE_PERSISTENT_SOURCE --repo "$repo" --body "$source_sha"
gh variable set SPIRALCORE_PERSISTENT_RUN --repo "$repo" --body "$campaign"
gh variable set SPIRALCORE_PERSISTENT_RECEIVER_SHA256 --repo "$repo" --body "$receiver"
for shard in 0 1 2 3 4 5; do
  gh variable set "SPIRALCORE_PERSISTENT_URL_$shard" --repo "$repo" --body "${urls[$shard]}"
done
gh variable set SPIRALCORE_PERSISTENT_PURPOSE --repo "$repo" --body pilot
gh secret set SPIRALCORE_PERSISTENT_TOKEN --repo "$repo" < "$root/capability.txt"
jq -n --arg source "$source_sha" --arg run "$campaign" --arg receiver "$receiver" \
  --arg url0 "${urls[0]}" --arg url1 "${urls[1]}" --arg url2 "${urls[2]}" \
  --arg url3 "${urls[3]}" --arg url4 "${urls[4]}" --arg url5 "${urls[5]}" \
  '{schema:"spiralcore.persistent-session.launch.v1",source:$source,run:$run,receiver_executable_sha256:$receiver,urls:[$url0,$url1,$url2,$url3,$url4,$url5],source_attempt:1,operations:396,paired_trials_per_size:6,preflight:"one unauthenticated GET rejected400 at each origin; before timed campaign",limits:"relayed WAN pilot; HTTP-body bytes, not complete network traffic"}' > "$evidence/launch.json"
git push -u origin codex/spiralcore-persistent-session
date -u +%FT%TZ > "$evidence/pushed-utc.txt"
