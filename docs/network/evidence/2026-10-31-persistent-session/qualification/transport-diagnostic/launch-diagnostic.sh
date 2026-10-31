#!/bin/bash
set -euo pipefail
cd /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
root=/Users/casey.allard/spiralcore/local/persistent-session-20261031/diagnostic-01
frozen=/Users/casey.allard/spiralcore/local/persistent-session-20261031/diagnostic-transport-frozen
evidence=docs/network/evidence/2026-10-31-persistent-session/wan-diagnostic-01
repo=R4-Laboratories/SpiralCore
source_sha=$(cat docs/network/evidence/2026-10-31-persistent-session/qualification/transport-diagnostic/source-revision.txt)
[[ $(git rev-parse HEAD) == "$source_sha" ]]
git diff --exit-code
receiver=$(shasum -a 256 "$frozen/spiralcore-wan" | awk '{print $1}')
campaign=$(cat "$root/run-id.txt")
url=$(rg -o 'https://[a-z0-9-]+\.trycloudflare\.com' "$root/tunnel-0.log" | head -1)
[[ "$url" =~ ^https://[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?\.trycloudflare\.com$ ]]
response=$(curl --silent --show-error --max-time 20 -o /dev/null -w '%{http_code}' "$url/")
printf '%s %s\n' "$url" "$response" > "$evidence/preflight.txt"
[[ "$response" == 400 ]]
gh variable set SPIRALCORE_PERSISTENT_SOURCE --repo "$repo" --body "$source_sha"
gh variable set SPIRALCORE_PERSISTENT_RUN --repo "$repo" --body "$campaign"
gh variable set SPIRALCORE_PERSISTENT_RECEIVER_SHA256 --repo "$repo" --body "$receiver"
gh variable set SPIRALCORE_PERSISTENT_PURPOSE --repo "$repo" --body diagnostic
gh variable set SPIRALCORE_PERSISTENT_URL_0 --repo "$repo" --body "$url"
gh secret set SPIRALCORE_PERSISTENT_TOKEN --repo "$repo" < "$root/capability.txt"
jq -n --arg source "$source_sha" --arg run "$campaign" --arg receiver "$receiver" --arg url "$url" \
 '{schema:"spiralcore.persistent-session.launch.v1",purpose:"diagnostic",source:$source,run:$run,receiver_executable_sha256:$receiver,url:$url,source_attempt:1,maximum_operations:66,preflight:"one unauthenticated GET rejected400 before diagnostic"}' > "$evidence/launch.json"
git push -u origin codex/spiralcore-persistent-session
date -u +%FT%TZ > "$evidence/pushed-utc.txt"
