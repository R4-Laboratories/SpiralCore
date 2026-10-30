#!/bin/bash
# Bounded adaptation of the previously reviewed SC-OPT-003 driver.
# Parent authorization is still required before invoking this script.
set -euo pipefail
probe_root=$(cd -- "$(dirname -- "$0")" && pwd)
cd "$probe_root"
bash verify-inputs.sh
shasum -a 256 -c build-diagnostic/executables.sha256
shasum -a 256 -c build-diagnostic/helper-files.sha256
shasum -a 256 -c qualification/qualified-inputs.sha256
[[ "$(cat qualification/final.exit)" == 0 ]]
[[ -z "$(ls -A experiment/working)" ]]
mkdir measured
bash template-manifest.sh > measured/templates-before.sha256
cmp templates-established.sha256 measured/templates-before.sha256
for block in 0 1; do
  label="block${block}-diagnostic"
  date -u '+%Y-%m-%dT%H:%M:%SZ' > "measured/${label}-host-before.txt"
  uptime >> "measured/${label}-host-before.txt"
  printf '{"event":"start","block":%s,"utc":"%s"}\n' "$block" "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" >> measured/execution.jsonl
  printf 'Starting %s\n' "$label"
  set +e
  build-diagnostic/receiver-probe run --root "$probe_root/experiment" --out "$probe_root/measured/${label}.jsonl" --block "$block" --variant diagnostic > "measured/${label}.stdout" 2> "measured/${label}.stderr"
  code=$?
  set -e
  printf '{"event":"end","block":%s,"exit_code":%s,"utc":"%s"}\n' "$block" "$code" "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" >> measured/execution.jsonl
  printf 'Completed %s exit=%s\n' "$label" "$code"
  if [[ "$code" != 0 ]]; then exit "$code"; fi
done
bash template-manifest.sh > measured/templates-after.sha256
cmp measured/templates-before.sha256 measured/templates-after.sha256
shasum -a 256 measured/block*.jsonl > measured/observations.sha256
build-diagnostic/profile-analysis "$probe_root/measured" "$probe_root/profile-analysis.json"
