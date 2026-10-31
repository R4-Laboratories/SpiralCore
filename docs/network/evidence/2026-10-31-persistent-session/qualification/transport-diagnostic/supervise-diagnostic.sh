#!/bin/bash
set -euo pipefail
cd /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
run_root=/Users/casey.allard/spiralcore/local/persistent-session-20261031/diagnostic-01
frozen=/Users/casey.allard/spiralcore/local/persistent-session-20261031/diagnostic-transport-frozen
evidence=docs/network/evidence/2026-10-31-persistent-session/wan-diagnostic-01
[[ $(df -Pk "$run_root" | awk 'NR==2 {print $4}') -gt 4194304 ]] || exit 1
git diff --exit-code
mkdir "$evidence"
owned=()
trap 'for child in "${owned[@]}"; do if [[ $(ps -o ppid= -p "$child" 2>/dev/null | tr -d " ") == "$$" ]]; then kill "$child" 2>/dev/null || true; fi; done; wait || true' EXIT
trap 'exit 130' INT TERM
export SPIRALCORE_PERSISTENT_SOURCE
SPIRALCORE_PERSISTENT_SOURCE=$(git rev-parse HEAD)
export SPIRALCORE_WAN_TOKEN
SPIRALCORE_WAN_TOKEN=$(cat "$run_root/capability.txt")
campaign=$(cat "$run_root/run-id.txt")
"$frozen/candidate" persistent-identity > "$evidence/receiver-harness-identity.json"
shasum -a 256 "$frozen/candidate" "$frozen/spiralcore-wan" > "$evidence/receiver-binaries.sha256"
date -u +%FT%TZ > "$evidence/receivers-start-utc.txt"
sysctl vm.loadavg > "$evidence/macos-load-before.txt"
df -k "$run_root" > "$evidence/disk-before.txt"
for shard in 0; do
  "$frozen/candidate" persistent-plan "$campaign" "$shard" "$evidence/plan-$shard.json"
  "$frozen/spiralcore-wan" serve --cache "$run_root/cache-$shard" --plan "$evidence/plan-$shard.json" --seconds 1800 --connection-log "$evidence/connections-$shard.jsonl" > "$evidence/receiver-$shard.stdout" 2> "$evidence/receiver-$shard.jsonl" < /dev/null &
  owned+=("$!")
  printf '%s\n' "$!" > "$run_root/receiver-$shard.pid"
  for attempt in {1..100}; do
    if rg -q '^READY addr=' "$evidence/receiver-$shard.stdout"; then break; fi
    sleep 0.1
  done
  address=$(sed -n 's/^READY addr=//p' "$evidence/receiver-$shard.stdout")
  [[ "$address" == 127.0.0.1:* ]] || exit 1
  /Users/casey.allard/spiralcore/local/wan-tools/cloudflared tunnel --config /dev/null --no-autoupdate --url "http://$address" > "$run_root/tunnel-$shard.log" 2>&1 < /dev/null &
  owned+=("$!")
  printf '%s\n' "$!" > "$run_root/tunnel-$shard.pid"
  printf 'Receiver %s ready at %s; persistent supervisor owns children\n' "$shard" "$address"
done
printf '%s\n' 'Supervisor remains alive until the campaign owner stops its children.'
wait
