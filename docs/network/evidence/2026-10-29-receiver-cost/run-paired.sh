#!/bin/bash
set -euo pipefail
probe_root=/Users/casey.allard/spiralcore/local/receiver-efficiency-20261029
cd "$probe_root"
[[ "$(shasum -a 256 schedule.json | cut -d ' ' -f 1)" == a4a37c517d557377fb5a053bbb6ac8cd25b0650baeb3111b6ecafd3b5d429a29 ]]
[[ "$(jq -r .source_revision build-new/build.json)" == b4a18602c04863bac2ba404450c401109dbee3ea ]]
cmp build-old/helper-files.sha256 build-new/helper-files.sha256
cmp build-old/compiler.txt build-new/compiler.txt
cmp build-old/compiler-environment.txt build-new/compiler-environment.txt
shasum -a 256 -c build-old/executable.sha256
shasum -a 256 -c build-new/executable.sha256
shasum -a 256 -c build-old/helper-files.sha256
mkdir "$probe_root/paired"
jq -r '.pairs[] | .block as $b | .order[] | "\($b) \(.)"' schedule.json > paired/execution-order.txt
[[ "$(wc -l < paired/execution-order.txt | tr -d ' ')" == 16 ]]
shasum -a 256 run-paired.sh schedule.json paired/execution-order.txt > paired/driver-inputs.sha256
while read -r block variant; do
  [[ "$block" =~ ^[0-7]$ && ( "$variant" == old || "$variant" == new ) ]]
  label="block${block}-${variant}"
  date -u '+%Y-%m-%dT%H:%M:%SZ' > "paired/${label}-host-before.txt"
  uptime >> "paired/${label}-host-before.txt"
  jq -nc --arg block "$block" --arg variant "$variant" --arg time "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" '{event:"start",block:($block|tonumber),variant:$variant,utc:$time}' >> paired/execution.jsonl
  printf 'Starting %s\n' "$label"
  set +e
  "$probe_root/build-${variant}/receiver-probe" run --root "$probe_root/experiment" --out "$probe_root/paired/${label}.jsonl" --block "$block" --variant "$variant" > "paired/${label}.stdout" 2> "paired/${label}.stderr"
  code=$?
  set -e
  jq -nc --arg block "$block" --arg variant "$variant" --arg code "$code" --arg time "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" '{event:"end",block:($block|tonumber),variant:$variant,exit_code:($code|tonumber),utc:$time}' >> paired/execution.jsonl
  printf 'Completed %s exit=%s\n' "$label" "$code"
  if [[ "$code" != 0 ]]; then exit "$code"; fi
done < paired/execution-order.txt
shasum -a 256 paired/block*.jsonl > paired/observations.sha256
