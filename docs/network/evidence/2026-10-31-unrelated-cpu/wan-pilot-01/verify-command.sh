#!/bin/bash
set -euo pipefail
cd /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
root=/Users/casey.allard/spiralcore/local/unrelated-cpu-20261031/run-01
frozen=/Users/casey.allard/spiralcore/local/unrelated-cpu-20261031/frozen-phase2
evidence=docs/network/evidence/2026-10-31-unrelated-cpu/wan-pilot-01
for file in "$root"/*.pid; do
  pid=$(cat "$file")
  if kill -0 "$pid" 2>/dev/null; then echo "Owned PID still live: $pid" >&2; exit 1; fi
done
date -u +%FT%TZ > "$evidence/receivers-stopped-utc.txt"
test "$(shasum -a 256 "$root/hosted-artifact.zip" | awk '{print $1}')" = 616bf7589ee61a3550570db1318af1c7525cfd37ac3ba7f22dd4be7bf77671ed
(cd "$root/hosted" && shasum -a 256 -c unrelated-cpu/artifact-sha256.txt) > "$evidence/artifact-verification.txt"
mkdir "$evidence/hosted"
for file in "$root/hosted/unrelated-cpu"/*; do
  if [[ -f "$file" ]]; then cp "$file" "$evidence/hosted/"; fi
done
export SPIRALCORE_UNRELATED_RECEIVER_SHA256
SPIRALCORE_UNRELATED_RECEIVER_SHA256=$(shasum -a 256 "$frozen/spiralcore-wan" | awk '{print $1}')
args=()
for shard in 0 1 2 3 4 5; do
  cmp "$evidence/plan-$shard.json" "$evidence/hosted/plan-$shard.json"
  export SPIRALCORE_UNRELATED_OBSERVATIONS="$evidence/receiver-$shard.jsonl"
  "$frozen/candidate" unrelated-verify "$evidence/plan-$shard.json" "$root/cache-$shard" "$evidence/verification-$shard.jsonl"
  args+=("$evidence/plan-$shard.json" "$evidence/hosted/observations-$shard.jsonl" "$evidence/verification-$shard.jsonl")
done
"$frozen/candidate" unrelated-summary "${args[@]}" "$evidence/summary.json"
date -u +%FT%TZ > "$evidence/verification-completed-utc.txt"
