#!/usr/bin/env bash
# Orchestration only; all measurement and exact recovery use the Rust engine.
set -euo pipefail
umask 077
root=/var/tmp/spiralcore-session-events-20261103
before=/var/tmp/spiralcore-session-20261103/bin/broker_objects
after="$root/bin/broker_objects"
results="$root/host-pairs"
[[ ! -e "$results" && ! -L "$results" ]]
[[ $(sha256sum "$before" | awk '{print $1}') == 879111c94d8ed8753526758997e497ee4391b8893040b6ef1067068fe96309e5 ]]
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
uptime >> "$results/host.txt"
printf 'PASS: 22 exact host cells; Rust engine removed only successful owned stores.\n'
