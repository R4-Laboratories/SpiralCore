#!/usr/bin/env bash
# Existing unchanged Rust engine; strict compatibility, not opt-in retry performance.
set -euo pipefail
umask 077
root=/var/tmp/spiralcore-reconciliation-20261103
before=/var/tmp/spiralcore-session-idle-20261103/bin/broker_objects
after="$root/bin/broker_objects"
results="$root/host-pairs"
[[ ! -e "$results" && ! -L "$results" ]]
[[ $(sha256sum "$before" | awk '{print $1}') == 04a57c32154acad6dbc3754775039aad80cab632e77f9d50dce9965d52ad8d35 ]]
[[ $(sha256sum "$after" | awk '{print $1}') == fe7937f2fb348ff0a3c517cf856f2ad132883346a8aa05020e6dbd0e1d62b6d7 ]]

mkdir -p "$results/before" "$results/after"
uname -a > "$results/host.txt"
uptime >> "$results/host.txt"
free -b >> "$results/host.txt"
sha256sum "$before" "$after" > "$results/binaries.sha256"
while read -r arm size trial; do
  order=(before after)
  if [[ "$trial" == 1 ]]; then order=(after before); fi
  for phase in "${order[@]}"; do
    binary="$before"
    if [[ "$phase" == after ]]; then binary="$after"; fi
    name="$arm-$size-$trial"
    timeout 120 "$binary" "$results/$phase/$name-store" \
      "$results/$phase/$name.jsonl" "$arm" "$size" "$trial"
    [[ ! -e "$results/$phase/$name-store" ]]
  done
done <<'CELLS'
whole-mixed 1048576 0
whole-mixed 1048576 1
whole-mixed 1048576 2
combined-cpu 1048576 0
combined-cpu 1048576 1
combined-cpu 1048576 2
CELLS
uptime >> "$results/host.txt"
printf 'PASS: twelve exact strict-transfer cells, successful owned stores removed.\n'
