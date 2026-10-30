#!/bin/bash
set -euo pipefail
probe_root=$(cd -- "$(dirname -- "$0")" && pwd)
template_run=/Users/casey.allard/spiralcore/local/receiver-efficiency-20261029/experiment
cd "$probe_root"
bash verify-inputs.sh
jq -e --arg identity_receipt "$(shasum -a 256 identity-qualification.json | cut -d ' ' -f 1)" --arg identity_execution "$(shasum -a 256 identity-execution.jsonl | cut -d ' ' -f 1)" '.status == "passed" and .source_revision == "80449939bd92348593cc88555cf7fde0847ceffd" and .identity_outputs_equal == true and .probe_tests_both == true and .identity_qualification_sha256 == $identity_receipt and .identity_execution_sha256 == $identity_execution' qualification.json > /dev/null
for variant in software hardware; do
  jq -e --arg variant "$variant" '.source_revision == "80449939bd92348593cc88555cf7fde0847ceffd" and .variant == $variant' "build-$variant/build.json" > /dev/null
  shasum -a 256 -c "build-$variant/executables.sha256"
  shasum -a 256 -c "build-$variant/helper-files.sha256"
  jq -e --arg variant "$variant" --arg probe "$(shasum -a 256 "build-$variant/receiver-probe" | cut -d ' ' -f 1)" --arg identity "$(shasum -a 256 "build-$variant/identity" | cut -d ' ' -f 1)" --arg helper "$(shasum -a 256 "build-$variant/helper-files.sha256" | cut -d ' ' -f 1)" --arg tests "$(shasum -a 256 "logs/probe-$variant-tests.log" | cut -d ' ' -f 1)" '.configurations[$variant].receiver_probe_sha256 == $probe and .configurations[$variant].identity_executable_sha256 == $identity and .configurations[$variant].helper_manifest_sha256 == $helper and .configurations[$variant].test_log_sha256 == $tests and .configurations[$variant].tests_passed == 7 and .configurations[$variant].tests_failed == 0' qualification.json > /dev/null
done
for file in helper-files.sha256 production-source-files.sha256 compiler.txt c-compiler.txt build-tools.sha256 tool-selection.txt apple-tools-package.txt build-environment.txt; do
  cmp "build-software/$file" "build-hardware/$file"
done
[[ -z "$(ls -A "$template_run/working")" ]]
mkdir paired
bash template-manifest.sh > paired/templates-before.sha256
cmp templates-established.sha256 paired/templates-before.sha256
jq -r '.pairs[] | .block as $b | .order[] | "\($b) \(.)"' schedule.json > paired/execution-order.txt
[[ "$(wc -l < paired/execution-order.txt | tr -d ' ')" == 16 ]]
shasum -a 256 run-paired.sh schedule.json paired/execution-order.txt qualification.json > paired/driver-inputs.sha256
while read -r block configuration; do
  [[ "$block" =~ ^[0-7]$ && ( "$configuration" == software || "$configuration" == hardware ) ]]
  variant=old
  [[ "$configuration" != hardware ]] || variant=new
  label="block${block}-${configuration}"
  date -u '+%Y-%m-%dT%H:%M:%SZ' > "paired/${label}-host-before.txt"
  uptime >> "paired/${label}-host-before.txt"
  jq -nc --arg block "$block" --arg configuration "$configuration" --arg variant "$variant" --arg time "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" '{event:"start",block:($block|tonumber),configuration:$configuration,receipt_variant:$variant,utc:$time}' >> paired/execution.jsonl
  printf 'Starting %s (%s)\n' "$label" "$variant"
  set +e
  "$probe_root/build-${configuration}/receiver-probe" run --root "$template_run" --out "$probe_root/paired/${label}.jsonl" --block "$block" --variant "$variant" > "paired/${label}.stdout" 2> "paired/${label}.stderr"
  code=$?
  set -e
  jq -nc --arg block "$block" --arg configuration "$configuration" --arg variant "$variant" --arg code "$code" --arg time "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" '{event:"end",block:($block|tonumber),configuration:$configuration,receipt_variant:$variant,exit_code:($code|tonumber),utc:$time}' >> paired/execution.jsonl
  printf 'Completed %s exit=%s\n' "$label" "$code"
  if [[ "$code" != 0 ]]; then exit "$code"; fi
done < paired/execution-order.txt
bash template-manifest.sh > paired/templates-after.sha256
cmp paired/templates-before.sha256 paired/templates-after.sha256
shasum -a 256 paired/block*.jsonl > paired/observations.sha256
