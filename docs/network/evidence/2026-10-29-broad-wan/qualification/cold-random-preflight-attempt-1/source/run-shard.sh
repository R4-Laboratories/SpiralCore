#!/bin/bash
# Sequential, owned synthetic campaign orchestration. Never retries a measured shard.
set -euo pipefail
umask 077
root=/Users/casey.allard/spiralcore/local/broad-wan-20261029
repo=R4-Laboratories/SpiralCore
case_name=${1:?registered case required}
shard=${2:?shard required}
case "$case_name" in
  localized_32|scattered_32x32|prefix_4096|middle_4096|delete_4096|append_4096|truncate_4096|replace_1m|structured_localized_32|compressed_localized_32|cold_random|cold_structured) ;;
  *) exit 2 ;;
esac
[[ "$shard" =~ ^[0-4]$ ]] || exit 1
source_rev=$(cat "$root/source-revision.txt")
campaign=$(cat "$root/run-id.txt")
[[ "$source_rev" =~ ^[0-9a-f]{40}$ && "$campaign" =~ ^[0-9a-f]{64}$ ]] || exit 1
# BEGIN producer preflight
qualified="$root/qualification/linux-qualified.json"
frozen_manifest="$root/qualification/producer-build-manifest.json"
for evidence in "$qualified" "$frozen_manifest"; do
  [[ -f "$evidence" && ! -L "$evidence" && $(wc -c < "$evidence") -le 131072 ]] || exit 1
done
jq -e --arg source "$source_rev" '.status == "passed" and .source == $source' "$qualified" >/dev/null
producer_run=$(jq -er '.producer.run_id | if type == "string" then . elif type == "number" and . >= 1 and . <= 9007199254740991 and . == floor then tostring else error("invalid producer run") end' "$qualified")
producer_artifact=$(jq -er '.producer.artifact_id | if type == "string" then . elif type == "number" and . >= 1 and . <= 9007199254740991 and . == floor then tostring else error("invalid producer artifact") end' "$qualified")
producer_digest=$(jq -er '.producer.artifact_digest | select(type == "string")' "$qualified")
[[ "$producer_run" =~ ^[1-9][0-9]{0,17}$ && "$producer_artifact" =~ ^[1-9][0-9]{0,17}$ ]] || exit 1
[[ "$producer_digest" =~ ^sha256:[0-9a-f]{64}$ ]] || exit 1
jq -e --arg source "$source_rev" --arg run "$producer_run" '
  .schema == "spiralcore.wan-sender-build.v1" and .identity.source == $source and
  (.identity.run_id | tostring) == $run and .identity.run_attempt == 1 and
  .qualification.source_tests_passed == true and .qualification.clippy_passed == true and .qualification.capture_passed == true
' "$frozen_manifest" >/dev/null
[[ $(gh variable get SPIRALCORE_WAN_BUILD_RUN --repo "$repo") == "$producer_run" ]] || exit 1
[[ $(gh variable get SPIRALCORE_WAN_BUILD_ARTIFACT --repo "$repo") == "$producer_artifact" ]] || exit 1
qualified_sha=$(shasum -a 256 "$qualified" | awk '{print $1}')
producer_manifest_sha=$(shasum -a 256 "$frozen_manifest" | awk '{print $1}')
# END producer preflight
[[ $(gh variable list --repo "$repo" --json name --jq '[.[] | select(.name == "SPIRALCORE_WAN_QUALIFY_ONLY")] | length') == 0 ]] || exit 1
[[ $(gh variable get SPIRALCORE_WAN_SOURCE --repo "$repo") == "$source_rev" ]] || exit 1
[[ $(gh api "repos/$repo/git/ref/heads/codex/spiralcore-efficient" --jq .object.sha) == "$source_rev" ]] || exit 1
[[ $(gh run list --repo "$repo" --workflow spiralcore-wan.yml --json status --jq '[.[] | select(.status != "completed")] | length') == 0 ]] || exit 1
shasum -a 256 -c "$root/receiver-executables.sha256" >/dev/null
[[ $(df -Pk "$root" | awk 'NR==2 {print $4}') -ge 2097152 ]] || exit 1
mkdir "$root/campaign.lock"
dir="$root/campaign/$case_name/shard-$shard"
origin_pid=
tunnel_pid=
configured=0
owned_dir=0
trap 'status=$?; trap - EXIT; set +e
cleanup_failed=0
if [[ "$configured" == 1 ]]; then
  gh secret delete SPIRALCORE_WAN_TOKEN --repo "$repo"
  for name in URL RUN TRIALS SUITE SHARD; do gh variable delete "SPIRALCORE_WAN_$name" --repo "$repo"; done
  gh secret list --repo "$repo" --json name > "$dir/secrets-after.json"
  [[ $? == 0 ]] && jq -e "all(.[]; .name != \"SPIRALCORE_WAN_TOKEN\")" "$dir/secrets-after.json" >/dev/null || cleanup_failed=1
  gh variable list --repo "$repo" --json name > "$dir/variables-after.json"
  [[ $? == 0 ]] && jq -e "all(.[]; (.name | test(\"^SPIRALCORE_WAN_(URL|RUN|TRIALS|SUITE|SHARD)$\")) | not)" "$dir/variables-after.json" >/dev/null || cleanup_failed=1
fi
for kind in tunnel origin; do
  if [[ "$kind" == tunnel ]]; then pid="$tunnel_pid"; else pid="$origin_pid"; fi
  if [[ -n "$pid" ]]; then
    observed=$(ps -p "$pid" -o lstart=)
    if [[ -n "$observed" && "$observed" == "$(cat "$dir/$kind.start")" ]] && jobs -pr | grep -qx "$pid"; then kill "$pid"; fi
    for attempt in {1..50}; do jobs -pr | grep -qx "$pid" || break; sleep 0.1; done
    if jobs -pr | grep -qx "$pid"; then
      if [[ "$(ps -p "$pid" -o lstart=)" == "$(cat "$dir/$kind.start")" ]]; then kill -KILL "$pid"; fi
      for attempt in {1..20}; do jobs -pr | grep -qx "$pid" || break; sleep 0.1; done
    fi
    if jobs -pr | grep -qx "$pid"; then outcome=unreaped; cleanup_failed=1; else wait "$pid"; outcome=$?; fi
    printf "%s\n" "$outcome" > "$dir/$kind.exit"
  fi
done
if [[ "$owned_dir" == 1 ]]; then
  rm -f "$dir/capability.secret" || cleanup_failed=1
  printf "{\"cleanup_failed\":%s,\"operation_exit\":%s}\n" "$cleanup_failed" "$status" > "$dir/cleanup.json"
fi
rmdir "$root/campaign.lock" || cleanup_failed=1
if [[ "$cleanup_failed" != 0 ]]; then status=1; fi
exit "$status"' EXIT
[[ $(gh variable list --repo "$repo" --json name --jq '[.[] | select(.name | test("^SPIRALCORE_WAN_(URL|RUN|TRIALS|SUITE|SHARD)$"))] | length') == 0 ]] || exit 1
[[ $(gh secret list --repo "$repo" --json name --jq '[.[] | select(.name == "SPIRALCORE_WAN_TOKEN")] | length') == 0 ]] || exit 1
mkdir -p "$root/campaign/$case_name"
mkdir "$dir"
owned_dir=1
# BEGIN producer checkpoint
cp "$qualified" "$dir/linux-qualified.json"
cp "$frozen_manifest" "$dir/frozen-producer-build-manifest.json"
[[ $(shasum -a 256 "$dir/linux-qualified.json" | awk '{print $1}') == "$qualified_sha" ]] || exit 1
[[ $(shasum -a 256 "$dir/frozen-producer-build-manifest.json" | awk '{print $1}') == "$producer_manifest_sha" ]] || exit 1
# END producer checkpoint
printf '%s\n' "$campaign" > "$dir/campaign-owner.txt"
"$root/bin/spiralcore-bench" plan --run "$campaign" --trials 30 --suite "broad16-$case_name" --shard "$shard" --out "$dir/plan.json"
openssl rand -hex 32 > "$dir/capability.secret"
SPIRALCORE_WAN_TOKEN=$(cat "$dir/capability.secret") "$root/bin/spiralcore-wan" serve --cache "$dir/cache" --plan "$dir/plan.json" --seconds 3600 > "$dir/origin-startup.txt" 2> "$dir/origin-diagnostic.txt" &
origin_pid=$!
printf '%s\n' "$origin_pid" > "$dir/origin.pid"
ps -p "$origin_pid" -o lstart= > "$dir/origin.start"
for attempt in {1..50}; do
  kill -0 "$origin_pid"
  if [[ -s "$dir/origin-startup.txt" ]]; then break; fi
  sleep 0.1
done
address=$(sed -n 's/^READY addr=//p' "$dir/origin-startup.txt")
[[ "$address" =~ ^127\.0\.0\.1:[0-9]+$ ]] || exit 1
grep -qx 'POLICY cold_graph_policy=Disabled' "$dir/origin-diagnostic.txt"
ps -p "$origin_pid" -o lstart=,comm= > "$dir/origin.identity"
cp "$dir/origin-diagnostic.txt" "$dir/receiver-policy.txt"
/Users/casey.allard/spiralcore/local/wan-tools/cloudflared tunnel --no-autoupdate --protocol quic --url "http://$address" > "$dir/tunnel-diagnostic.txt" 2>&1 &
tunnel_pid=$!
printf '%s\n' "$tunnel_pid" > "$dir/tunnel.pid"
ps -p "$tunnel_pid" -o lstart= > "$dir/tunnel.start"
/Users/casey.allard/spiralcore/local/wan-tools/cloudflared --version > "$dir/cloudflared-version.txt"
shasum -a 256 /Users/casey.allard/spiralcore/local/wan-tools/cloudflared > "$dir/cloudflared.sha256"
url=
for attempt in {1..60}; do
  kill -0 "$origin_pid"
  kill -0 "$tunnel_pid"
  url=$(sed -nE 's/.*(https:\/\/[a-z0-9-]+\.trycloudflare\.com).*/\1/p' "$dir/tunnel-diagnostic.txt" | head -1)
  if [[ -n "$url" ]] && grep -q 'Registered tunnel connection' "$dir/tunnel-diagnostic.txt"; then break; fi
  sleep 1
done
[[ "$url" =~ ^https://[a-z0-9-]+\.trycloudflare\.com$ ]] || exit 1
grep -q 'Registered tunnel connection' "$dir/tunnel-diagnostic.txt"
ps -p "$tunnel_pid" -o lstart=,comm= > "$dir/tunnel.identity"
printf '%s\n' "$url" > "$dir/url.txt"
grep -E 'Settings:|Initial protocol|Registered tunnel connection' "$dir/tunnel-diagnostic.txt" > "$dir/tunnel-metadata.txt"
configured=1
gh secret set SPIRALCORE_WAN_TOKEN --repo "$repo" < "$dir/capability.secret"
gh variable set SPIRALCORE_WAN_URL --body "$url" --repo "$repo"
gh variable set SPIRALCORE_WAN_RUN --body "$campaign" --repo "$repo"
gh variable set SPIRALCORE_WAN_TRIALS --body 30 --repo "$repo"
gh variable set SPIRALCORE_WAN_SUITE --body "broad16-$case_name" --repo "$repo"
gh variable set SPIRALCORE_WAN_SHARD --body "$shard" --repo "$repo"
date -u +%Y-%m-%dT%H:%M:%SZ > "$dir/dispatch-time.txt"
ps -axo pid,etime,pcpu,comm > "$dir/processes-before.txt"
sysctl vm.loadavg hw.memsize > "$dir/host-before.txt"
gh workflow run spiralcore-wan.yml --ref codex/spiralcore-efficient --repo "$repo"
run_id=
for attempt in {1..12}; do
  gh run list --repo "$repo" --workflow spiralcore-wan.yml --event workflow_dispatch --limit 20 --json databaseId,headSha,createdAt > "$dir/dispatch-candidates.json"
  run_id=$(jq -r --arg rev "$source_rev" --arg since "$(cat "$dir/dispatch-time.txt")" '[.[] | select(.headSha == $rev and .createdAt >= $since)] | if length == 1 then .[0].databaseId else empty end' "$dir/dispatch-candidates.json")
  if [[ -n "$run_id" ]]; then break; fi
  sleep 5
done
[[ "$run_id" =~ ^[0-9]+$ ]] || exit 1
printf '%s\n' "$run_id" > "$dir/github-run-id.txt"
printf 'START %s shard=%s github=%s\n' "$case_name" "$shard" "$run_id"
set +e
gh run watch "$run_id" --repo "$repo" --interval 30 --exit-status > "$dir/github-watch-diagnostic.txt" 2>&1
watch_status=$?
set -e
gh run view "$run_id" --repo "$repo" --json status,conclusion,headSha,jobs,url > "$dir/github-status.json"
jq -e --arg source "$source_rev" '.status == "completed" and .headSha == $source' "$dir/github-status.json" >/dev/null
gh run download "$run_id" --repo "$repo" --name "spiralcore-wan-$run_id-1" --dir "$dir/remote"
# BEGIN producer download validation
for evidence in build-reuse-validation.json producer-proof-validation.json sender-archive-validation.json producer-build-manifest.json; do
  [[ -f "$dir/remote/$evidence" && ! -L "$dir/remote/$evidence" && $(wc -c < "$dir/remote/$evidence") -le 131072 ]] || exit 1
done
jq -e --arg source "$source_rev" --arg run "$producer_run" --arg artifact "$producer_artifact" --arg digest "$producer_digest" \
  --slurpfile pinned "$dir/frozen-producer-build-manifest.json" '
  .schema == "spiralcore.wan-build-reuse-validation.v1" and .status == "passed" and .manifest == $pinned[0] and
  (.producer_run.id | tostring) == $run and .producer_run.head_sha == $source and .producer_run.run_attempt == 1 and
  .producer_run.status == "completed" and .producer_run.conclusion == "success" and
  (.artifact.id | tostring) == $artifact and (.artifact.workflow_run_id | tostring) == $run and
  .artifact.head_sha == $source and .artifact.digest == $digest
' "$dir/remote/build-reuse-validation.json" >/dev/null
jq -e --slurpfile reuse "$dir/remote/build-reuse-validation.json" '
  .schema == "spiralcore.wan-build-proof-validation.v1" and .status == "passed" and
  .producer_run == $reuse[0].producer_run and .artifact == $reuse[0].artifact
' "$dir/remote/producer-proof-validation.json" >/dev/null
jq -e --arg artifact "$producer_artifact" --arg sha "${producer_digest#sha256:}" --slurpfile reuse "$dir/remote/build-reuse-validation.json" '
  .schema == "spiralcore.sender-archive-validation.v1" and .status == "passed" and
  (.artifact_id | tostring) == $artifact and .archive_sha256 == $sha and .archive_bytes == $reuse[0].artifact.size_in_bytes
' "$dir/remote/sender-archive-validation.json" >/dev/null
cmp "$dir/frozen-producer-build-manifest.json" "$dir/remote/producer-build-manifest.json"
jq -n --arg source "$source_rev" --arg run "$producer_run" --arg artifact "$producer_artifact" --arg digest "$producer_digest" --arg manifest_sha "$producer_manifest_sha" \
  '{schema:"spiralcore.local-producer-reuse-validation.v1",status:"passed",source:$source,producer:{run_id:$run,artifact_id:$artifact,artifact_digest:$digest,manifest_sha256:$manifest_sha},scope:"downloaded reuse evidence and manifest match the frozen source-qualified producer"}' > "$dir/local-producer-reuse-validation.json"
# END producer download validation
[[ "$(ps -p "$tunnel_pid" -o lstart=,comm=)" == "$(cat "$dir/tunnel.identity")" ]] || exit 1
kill "$tunnel_pid"
for attempt in {1..50}; do jobs -pr | grep -qx "$tunnel_pid" || break; sleep 0.1; done
if jobs -pr | grep -qx "$tunnel_pid"; then exit 1; fi
set +e
wait "$tunnel_pid"
printf '%s\n' "$?" > "$dir/tunnel.exit"
set -e
tunnel_pid=
[[ "$(ps -p "$origin_pid" -o lstart=,comm=)" == "$(cat "$dir/origin.identity")" ]] || exit 1
kill "$origin_pid"
for attempt in {1..50}; do jobs -pr | grep -qx "$origin_pid" || break; sleep 0.1; done
if jobs -pr | grep -qx "$origin_pid"; then exit 1; fi
set +e
wait "$origin_pid"
printf '%s\n' "$?" > "$dir/origin.exit"
set -e
origin_pid=
ps -axo pid,etime,pcpu,comm > "$dir/processes-after.txt"
sysctl vm.loadavg hw.memsize > "$dir/host-after.txt"
[[ $(cat "$dir/remote/source-revision.txt") == "$source_rev" ]] || exit 1
cmp "$dir/plan.json" "$dir/remote/plan.json"
"$root/bin/spiralcore-bench" verify --plan "$dir/plan.json" --rows "$dir/remote/rows.jsonl" --cache "$dir/cache" > "$dir/verification.json"
jq -e '.status == "exact"' "$dir/verification.json" >/dev/null
find "$dir/cache" -type f -print0 > "$dir/publication-paths.nul"
file_count=0
while IFS= read -r -d '' file; do
  shasum -a 256 "$file" >> "$dir/publication-files.sha256"
  file_count=$((file_count + 1))
done < "$dir/publication-paths.nul"
[[ "$file_count" -gt 0 && $(wc -l < "$dir/publication-files.sha256") -eq "$file_count" ]] || exit 1
shasum -a 256 -c "$dir/publication-files.sha256" > "$dir/publication-hash-verification.txt"
[[ -s "$dir/publication-files.sha256" && $(cat "$dir/campaign-owner.txt") == "$campaign" ]] || exit 1
# Removal is restricted to this exclusive, verified synthetic shard cache.
rm -rf "$dir/cache"
printf '{"status":"removed_after_exact_reopen_and_hashing","scope":"this_new_synthetic_shard_cache"}\n' > "$dir/cache-cleanup.json"
printf 'COMPLETE %s shard=%s watch_status=%s\n' "$case_name" "$shard" "$watch_status"
[[ "$watch_status" == 0 ]] || exit 1
