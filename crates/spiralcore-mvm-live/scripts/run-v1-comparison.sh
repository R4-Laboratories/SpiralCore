#!/usr/bin/env bash
# 24 interleaved legacy/v1 sessions; identical Combined-CPU sequence and paid setup.
set -euo pipefail
umask 077
[[ $# == 7 ]] || { printf "usage: run-v1-comparison.sh LEGACY_HOST LEGACY_GUEST V1_HOST V1_GUEST KERNEL NEW_OUTPUT_DIR REGISTRATION_JSON\n" >&2; exit 2; }
legacy_host=$1
legacy_guest=$2
v1_host=$3
v1_guest=$4
kernel=$5
out=$6
registration=$7
here=$(cd -- "$(dirname -- "$0")" && pwd)
[[ $(uname -s) == Linux ]] || exit 2
backend=firecracker
[[ ! -e "$out" && ! -L "$out" ]] || { printf "output must be new\n" >&2; exit 2; }
[[ -x "$legacy_host" && -x "$v1_host" && -f "$legacy_guest" && -f "$v1_guest" && -f "$kernel" && -f "$registration" ]] || exit 2
for input in "$legacy_host" "$legacy_guest" "$v1_host" "$v1_guest" "$kernel" "$registration"; do
    [[ ! -L "$input" ]] || { printf "inputs must be regular nonsymlink files\n" >&2; exit 2; }
done
jq -e '
  .format=="spiralcore-v1-endpoint-registration-v1" and .trials==3 and
  .stages==["one","two"] and .sizes==[1048576,16777216] and
  .implementations==["legacy","v1"] and .arm=="combined-cpu" and
  .bandwidth_profile=="experimental64" and .backend=="firecracker" and
  all([.legacy.source_commit,.v1.source_commit][]; test("^[0-9a-f]{40}$")) and
  all([.legacy.host_sha256,.legacy.guest_sha256,.v1.host_sha256,.v1.guest_sha256,.kernel_sha256][]; test("^[0-9a-f]{64}$"))
' "$registration" >/dev/null
sha() { shasum -a 256 "$1" | awk '{print $1}'; }
host_snapshot() {
    date -u '+%Y-%m-%dT%H:%M:%SZ'
    uptime
    if [[ "$backend" == firecracker ]]; then cat /proc/loadavg; else sysctl vm.loadavg; fi
    # Counts only, no arguments, paths, identities or environment from other work.
    ps -Ao comm= | awk '
      /(^|\/)(cargo|rustc|clang|clang\+\+|cc|c\+\+|zig)$/ {compilers++}
      /(^|\/)spiralcore-mvm-live$/ {witnesses++}
      END {printf "compiler_process_count=%d\nspiralcore_witness_process_count=%d\n", compilers, witnesses}'
}
for implementation in legacy v1; do
    if [[ "$implementation" == legacy ]]; then host=$legacy_host; guest=$legacy_guest; else host=$v1_host; guest=$v1_guest; fi
    [[ $(sha "$host") == "$(jq -r --arg i "$implementation" '.[ $i ].host_sha256' "$registration")" && $(sha "$guest") == "$(jq -r --arg i "$implementation" '.[ $i ].guest_sha256' "$registration")" ]] || { printf "executable hash mismatch\n" >&2; exit 1; }
done
[[ $(sha "$kernel") == "$(jq -r .kernel_sha256 "$registration")" ]] || exit 1
mkdir -- "$out"
out=$(cd -- "$out" && pwd -P)
cp -- "$registration" "$out/registration.json"
jq -n --slurpfile registration "$registration" '
  $registration[0] as $r |
  [$r.stages[] as $stage | $r.sizes[] as $size | range(0;3) as $trial |
   range(0;2) as $slot | $r.implementations[(($slot+$trial)%2)] as $implementation |
   {name:($stage+"-"+($size|tostring)+"-t"+($trial|tostring)+"-"+$implementation),implementation:$implementation,stage:$stage,size:$size,trial:$trial,arm:$r.arm,bandwidth_profile:$r.bandwidth_profile}]
' > "$out/schedule.json"
shasum -a 256 "$legacy_host" "$legacy_guest" "$v1_host" "$v1_guest" "$kernel" "$registration" "$here/run-v1-comparison.sh" "$here/combined-session.jq" "$here/v1-comparison.jq" > "$out/identities.sha256"
{
    date -u '+%Y-%m-%dT%H:%M:%SZ'
    uname -a
    uptime
    df -Pk "$out"
    if [[ "$backend" == firecracker ]]; then
        getconf _NPROCESSORS_ONLN
        awk '/^MemTotal:/ {print}' /proc/meminfo
    else
        sysctl hw.memsize hw.ncpu machdep.cpu.brand_string
    fi
} > "$out/host-before.txt"
while IFS= read -r item; do
    name=$(jq -r .name <<< "$item")
    stage=$(jq -r .stage <<< "$item")
    size=$(jq -r .size <<< "$item")
    trial=$(jq -r .trial <<< "$item")
    arm=$(jq -r .arm <<< "$item")
    implementation=$(jq -r .implementation <<< "$item")
    if [[ "$implementation" == legacy ]]; then runner=$legacy_host; guest=$legacy_guest; else runner=$v1_host; guest=$v1_guest; fi
    available_kib=$(df -Pk "$out" | awk 'END {print $4}')
    [[ "$available_kib" -ge 4194304 ]] || { printf '4 GiB reserve reached; retained all receipts\n' >&2; exit 1; }
    [[ $(sha "$runner") == "$(jq -r --arg i "$implementation" '.[ $i ].host_sha256' "$registration")" && $(sha "$guest") == "$(jq -r --arg i "$implementation" '.[ $i ].guest_sha256' "$registration")" ]] || { printf 'executable changed during campaign\n' >&2; exit 1; }
    printf 'starting %s\n' "$name"
    host_snapshot > "$out/$name.host-before.txt"
    if "$runner" session "$out/$name" "$kernel" "$guest" "$stage" "$arm" "$size" "$trial" experimental64 > "$out/$name.stdout" 2> "$out/$name.stderr"; then rc=0; else rc=$?; fi
    printf '%s\n' "$rc" > "$out/$name.exit"
    host_snapshot > "$out/$name.host-after.txt"
    [[ "$rc" == 0 ]] || { printf 'failed %s; owned data and receipts retained\n' "$name" >&2; exit "$rc"; }
    session="$out/$name"
    [[ -d "$session" && ! -L "$session" ]] || { printf 'unsafe session directory\n' >&2; exit 1; }
    jq -n --argjson expected "$(jq 'del(.implementation)' <<< "$item")" --arg source "$name/result.json" \
       --slurpfile r "$session/result.json" --slurpfile host "$session/host-events.jsonl" \
       -f "$here/combined-session.jq" |
       jq --arg implementation "$implementation" --slurpfile raw "$session/result.json" '
         .session.implementation=$implementation |
         .session.actual_frames_sent=$raw[0].sender.counters.frames_sent |
         .session.actual_frames_received=$raw[0].sender.counters.frames_received |
         .operation_rows |= map(. as $op |
           [$raw[0].sender_worker.guest_events[]|select(.event=="sender_operation" and .kind==$op.kind)] as $sent |
           if ($sent|length)!=1 then error("missing actual frame counters") else
             .implementation=$implementation |
             .actual_frames_sent=$sent[0].counters.frames_sent |
             .actual_frames_received=$sent[0].counters.frames_received
           end)' > "$session/verified-summary.json"
    # The session owns these reproducible payload stores. Public receipts remain.
    # Never remove data from failed sessions or from an unverified readback.
    if [[ "$stage" == one ]]; then
        [[ -d "$session/cache" && ! -L "$session/cache" ]] || { printf 'unsafe cache cleanup target\n' >&2; exit 1; }
        disk_kib=$(du -sk "$session/cache" | awk '{print $1}')
        rm -rf -- "$session/cache"
        removed=cache
    else
        [[ -f "$session/receiver.ext4" && ! -L "$session/receiver.ext4" ]] || { printf 'unsafe disk cleanup target\n' >&2; exit 1; }
        disk_kib=$(du -sk "$session/receiver.ext4" | awk '{print $1}')
        rm -- "$session/receiver.ext4"
        removed=receiver.ext4
    fi
    jq -n --arg removed "$removed" --argjson allocated_kib "$disk_kib" \
      '{removed_owned_payload_store:$removed,allocated_kib_before:$allocated_kib,after_exact_readback:true,public_receipts_retained:true}' > "$session/cleanup.json"
    (cd -- "$session" && find . -maxdepth 1 -type f ! -name receipts.sha256 -exec shasum -a 256 {} +) > "$session/receipts.sha256"
    printf 'passed %s; retained receipts and removed verified disposable store\n' "$name"
done < <(jq -c '.[]' "$out/schedule.json")
jq -s -f "$here/v1-comparison.jq" "$out"/*/verified-summary.json > "$out/analysis.json"
date -u '+%Y-%m-%dT%H:%M:%SZ' > "$out/completed-utc.txt"
printf 'completed 24 sessions; analysis: %s/analysis.json\n' "$out"
