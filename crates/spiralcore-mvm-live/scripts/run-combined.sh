#!/usr/bin/env bash
# 36 registered native guest sessions, sequential and bounded in local disk use.
set -euo pipefail
umask 077
[[ $# == 5 ]] || { printf 'usage: run-combined.sh HOST_RUNNER KERNEL GUEST_ELF NEW_OUTPUT_DIR REGISTRATION_JSON\n' >&2; exit 2; }
runner=$1
kernel=$2
guest=$3
out=$4
registration=$5
here=$(cd -- "$(dirname -- "$0")" && pwd)
repo=$(git -C "$here" rev-parse --show-toplevel)
case $(uname -s) in
    Linux) backend=firecracker ;;
    Darwin) backend=hvf ;;
    *) printf 'unsupported host platform\n' >&2; exit 2 ;;
esac
[[ ! -e "$out" && ! -L "$out" ]] || { printf 'output must be new\n' >&2; exit 2; }
[[ -x "$runner" && -f "$kernel" && -f "$guest" && -f "$registration" ]] || exit 2
for input in "$runner" "$kernel" "$guest" "$registration"; do
    [[ ! -L "$input" ]] || { printf 'inputs must be regular nonsymlink files\n' >&2; exit 2; }
done
jq -e '
  .format=="spiralcore-mvm-combined-registration-v1" and .trials==3 and
  .stages==["one","two"] and .sizes==[1048576,16777216] and
  .arms==["whole-mixed","combined","combined-cpu"] and
  .bandwidth_profile=="experimental64" and
  (.source_commit|test("^[0-9a-f]{40}$")) and
  all([.host_sha256,.guest_sha256,.kernel_sha256][]; test("^[0-9a-f]{64}$"))
' "$registration" >/dev/null
[[ $(jq -r '.backend // "hvf"' "$registration") == "$backend" ]] || { printf 'registered backend mismatch\n' >&2; exit 2; }
[[ $(git -C "$repo" rev-parse HEAD) == "$(jq -r .source_commit "$registration")" ]] || { printf 'source revision mismatch\n' >&2; exit 2; }
# Freeze tracked source and runner, while permitting unrelated local research files.
git -C "$repo" diff --quiet HEAD -- crates scripts .github || { printf 'commit source changes before registered run\n' >&2; exit 2; }
[[ -z $(git -C "$repo" ls-files --others --exclude-standard -- crates scripts .github) ]] || { printf 'untracked implementation or runner files\n' >&2; exit 2; }
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
[[ $(sha "$runner") == "$(jq -r .host_sha256 "$registration")" &&
   $(sha "$guest") == "$(jq -r .guest_sha256 "$registration")" &&
   $(sha "$kernel") == "$(jq -r .kernel_sha256 "$registration")" ]] || { printf 'registered executable/kernel hash mismatch\n' >&2; exit 2; }
mkdir -- "$out"
out=$(cd -- "$out" && pwd -P)
cp -- "$registration" "$out/registration.json"
jq -n --slurpfile registration "$registration" '
  $registration[0] as $r |
  [$r.stages[] as $stage | $r.sizes[] as $size | range(0;3) as $trial |
   range(0;3) as $slot | $r.arms[(($slot+$trial)%3)] as $arm |
   {name:($stage+"-"+($size|tostring)+"-t"+($trial|tostring)+"-"+$arm),stage:$stage,size:$size,trial:$trial,arm:$arm,bandwidth_profile:$r.bandwidth_profile}]
' > "$out/schedule.json"
shasum -a 256 "$runner" "$kernel" "$guest" "$registration" "$here/run-combined.sh" "$here/analyze-combined.sh" "$here/combined-session.jq" "$here/combined-analysis.jq" > "$out/identities.sha256"
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
    available_kib=$(df -Pk "$out" | awk 'END {print $4}')
    [[ "$available_kib" -ge 4194304 ]] || { printf '4 GiB reserve reached; retained all receipts\n' >&2; exit 1; }
    [[ $(sha "$runner") == "$(jq -r .host_sha256 "$registration")" && $(sha "$guest") == "$(jq -r .guest_sha256 "$registration")" ]] || { printf 'executable changed during campaign\n' >&2; exit 1; }
    printf 'starting %s\n' "$name"
    host_snapshot > "$out/$name.host-before.txt"
    if "$runner" session "$out/$name" "$kernel" "$guest" "$stage" "$arm" "$size" "$trial" experimental64 > "$out/$name.stdout" 2> "$out/$name.stderr"; then rc=0; else rc=$?; fi
    printf '%s\n' "$rc" > "$out/$name.exit"
    host_snapshot > "$out/$name.host-after.txt"
    [[ "$rc" == 0 ]] || { printf 'failed %s; owned data and receipts retained\n' "$name" >&2; exit "$rc"; }
    session="$out/$name"
    [[ -d "$session" && ! -L "$session" ]] || { printf 'unsafe session directory\n' >&2; exit 1; }
    jq -n --argjson expected "$item" --arg source "$name/result.json" \
       --slurpfile r "$session/result.json" --slurpfile host "$session/host-events.jsonl" \
       -f "$here/combined-session.jq" > "$session/verified-summary.json"
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
"$here/analyze-combined.sh" "$out" > "$out/analysis.json"
date -u '+%Y-%m-%dT%H:%M:%SZ' > "$out/completed-utc.txt"
printf 'completed 36 sessions; analysis: %s/analysis.json\n' "$out"
