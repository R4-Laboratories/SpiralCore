#!/usr/bin/env bash
set -euo pipefail
umask 077
root=/var/tmp/spiralcore-session-20261103
old=/var/tmp/spiralcore-persistent-20261103/after/bin/broker_objects
new="$root/bin/broker_objects"
[[ ! -e "$root/host-pairs" ]]
[[ $(sha256sum "$old" | awk '{print $1}') == fd741385ce73485dbe204376e3d5dfe1c90bac66ac0c562982e2609aeb3b525e ]]
[[ $(sha256sum "$new" | awk '{print $1}') == 879111c94d8ed8753526758997e497ee4391b8893040b6ef1067068fe96309e5 ]]
( cd "$root/source"; sha256sum --status -c ../source-sha256.txt )
mkdir -p "$root/host-pairs/before" "$root/host-pairs/after"
uname -a > "$root/host-pairs/host.txt"
uptime >> "$root/host-pairs/host.txt"
free -b >> "$root/host-pairs/host.txt"
sha256sum "$old" "$new" > "$root/host-pairs/binaries.sha256"
while read -r arm size trial; do
  if [[ "$trial" == 1 ]]; then order=(after before); else order=(before after); fi
  for phase in "${order[@]}"; do
    binary="$old"
    if [[ "$phase" == after ]]; then binary="$new"; fi
    name="$arm-$size-$trial"
    "$binary" "$root/host-pairs/$phase/$name-store" \
      "$root/host-pairs/$phase/$name.jsonl" "$arm" "$size" "$trial"
    [[ ! -e "$root/host-pairs/$phase/$name-store" ]]
  done
done <<'CELLS'
raw-mixed 1048576 0
raw-mixed 1048576 1
raw-mixed 1048576 2
whole-mixed 1048576 0
whole-mixed 1048576 1
whole-mixed 1048576 2
combined-cpu 1048576 0
combined-cpu 1048576 1
combined-cpu 1048576 2
whole-mixed 16777216 0
combined-cpu 16777216 0
CELLS
uptime >> "$root/host-pairs/host.txt"
printf 'PASS: 22 cells completed exact verification; owned payloads removed by Rust probe.\n'
