#!/usr/bin/env bash
# Existing Rust engine; strict-transfer confirmation, not Idle retry timing.
set -euo pipefail
umask 077
root=/var/tmp/spiralcore-session-idle-20261103
before=/var/tmp/spiralcore-session-events-20261103/bin/broker_objects
after="$root/bin/broker_objects"
results="$root/host-pairs"
[[ ! -e "$results" && ! -L "$results" ]]
[[ $(sha256sum "$before" | awk '{print $1}') == 4109a318f859e9b7ddc71f32c3e3519d59d966f987c428ec1b2c0dd91f6dc1f3 ]]
(cd "$root/source"; sha256sum --status -c ../qualified-source-sha256.txt)
(cd "$root/bin"; sha256sum --status -c SHA256SUMS)
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
