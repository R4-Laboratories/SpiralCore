#!/bin/bash
set -euo pipefail
phase=$1
size=$2
trial=$3
arm=$4
repo=$(cd "$(dirname "$0")/../../../.." && pwd)
evidence="$repo/docs/network/evidence/2026-11-01-mvm-overhead"
owned=/Users/casey.allard/spiralcore/local/mvm-overhead-20261101
kernel=/Users/casey.allard/spiralcore/local/mvm-vsock-20261031/assets/Image
case "$phase" in
  before-standard) binary=before; profile= ;;
  after-standard) binary=after; profile=standard ;;
  after-experimental64) binary=after; profile=experimental64 ;;
  *) exit 2 ;;
esac
case "$size:$trial:$arm" in
  1048576:[012]:whole|1048576:[012]:warm|16777216:[012]:whole|16777216:[012]:warm) ;;
  *) exit 2 ;;
esac
id="two-${size}-t${trial}-${arm}"
run="$owned/runs/$phase/$id"
receipt="$evidence/$phase/$id"
test ! -e "$run"
test ! -e "$receipt"
free_kib=$(df -k "$owned" | awk 'NR==2 {print $4}')
test "$free_kib" -ge 4194304
mkdir -p "$(dirname "$run")" "$receipt"
compiler_snapshot() {
  ps -axo comm | awk '{n=$0; sub(/^.*\//,"",n); if(n ~ /^(cargo|rustc|zig|cc1|clang)$/) counts[n]++} END {for(n in counts) print n,counts[n]}' | sort
}
compiler_snapshot > "$receipt/build-processes-before.txt"
sysctl -n vm.loadavg > "$receipt/load-before.txt"
date -u +%FT%TZ > "$receipt/started.utc"
args=(session "$run" "$kernel" "$owned/$binary/guest" two "$arm" "$size" "$trial")
if [ -n "$profile" ]; then args+=("$profile"); fi
printf '%q ' "$owned/$binary/host" "${args[@]}" > "$receipt/command.txt"
printf '\n' >> "$receipt/command.txt"
set +e
"$owned/$binary/host" "${args[@]}" > "$receipt/run.stdout" 2> "$receipt/run.stderr"
run_exit=$?
set -e
printf '%s\n' "$run_exit" > "$receipt/run.exit"
date -u +%FT%TZ > "$receipt/finished.utc"
compiler_snapshot > "$receipt/build-processes-after.txt"
sysctl -n vm.loadavg > "$receipt/load-after.txt"
jq -n --rawfile before "$receipt/build-processes-before.txt" --rawfile after "$receipt/build-processes-after.txt" \
  --rawfile load_before "$receipt/load-before.txt" --rawfile load_after "$receipt/load-after.txt" \
  '{compiler_names_counts_before:$before,compiler_names_counts_after:$after,host_load_before:$load_before,host_load_after:$load_after,
    compiler_observed_at_endpoints:($before!="" or $after!=""),
    scope:"Shared host; endpoint observations only, not continuous contention monitoring. Owned SpiralCore builds paused by coordination; unrelated lab builds permitted by amendment-01."}' > "$receipt/environment.json"
for file in "$run"/*.json "$run"/*.jsonl "$run"/*.stderr; do
  [ -f "$file" ] || continue
  cp -p "$file" "$receipt/"
done
test "$run_exit" -eq 0
jq -e --arg arm "$arm" --argjson size "$size" --argjson trial "$trial" '
  .ok == true and .event == "session_done" and
  .plan.stage == "two" and .plan.arm == $arm and .plan.size == $size and .plan.trial == $trial and
  .receiver.exact_reopened == true and .sender.operations == .receiver.operations and
  .receiver.operations == (if $arm == "warm" then 11 else 7 end) and
  .receiver_worker.ok == true and .sender_worker.ok == true and .recovery_worker.ok == true and
  .receiver_worker.backing_disk_synced == true and .recovery_worker.backing_disk_synced == true and
  ([.recovery_worker.guest_events[] | select(.event=="verified")] | length) == .receiver.operations and
  all(.recovery_worker.guest_events[] | select(.event=="verified"); .exact==true and .read_only==true) and
  ([.sender_worker.guest_events[] | select(.event=="sender_operation") | [.kind,.bytes,.object]] | sort) ==
  ([.recovery_worker.guest_events[] | select(.event=="verified") | [.kind,.bytes,.object]] | sort) and
  ([.recovery_worker.guest_events[] | select(.event=="guest_done" and .ok==true)] | length) == 1 and
  .physical_vsock_stream_bytes == (.logical_envelope_and_framing_bytes * 2)
' "$receipt/result.json" > "$receipt/acceptance.txt"
if [ -n "$profile" ]; then
  rate=4194304
  [ "$profile" != experimental64 ] || rate=67108864
  jq -e --arg profile "$profile" --argjson rate "$rate" '
    .plan.bandwidth_profile==$profile and .mailbox_rate_bytes_per_second==$rate and .mailbox_burst_bytes==8388608 and
    all(.sender_worker,.receiver_worker,.recovery_worker;
      .mailbox_bandwidth_profile==$profile and .mailbox_rate_bytes_per_second==$rate and .mailbox_burst_bytes==8388608)
  ' "$receipt/result.json" > "$receipt/profile-acceptance.txt"
fi
(
  cd "$receipt"
  shasum -a 256 result.json sender.json receiver.json recovery.json host-events.jsonl > custody.sha256
  shasum -a 256 -c custody.sha256 > custody-check.txt
)
# Only the fresh disk owned by this exact registered session is disposable.
test -f "$run/receiver.ext4"
test ! -L "$run/receiver.ext4"
rm "$run/receiver.ext4"
jq -n --arg disk "$run/receiver.ext4" '{exact_reopened:true,keyless_reboot_verified:true,receipt_hashes_verified:true,removed_owned_disk:$disk}' > "$receipt/cleanup.json"
printf '%s %s exact and reboot verified; owned disk removed\n' "$phase" "$id"
