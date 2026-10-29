#!/bin/bash
# Fixed registered campaign. Invoke only after parent quiet-window authorization.
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")" && pwd)
cd "$root"
bash verify-inputs.sh
shasum -a 256 -c build-diagnostic/executables.sha256
shasum -a 256 -c build-diagnostic/helper-files.sha256
shasum -a 256 -c qualification/qualified-inputs.sha256
[[ "$(cat qualification/final-checks.exit)" == 0 ]]
for layout in empty compact64 compact768 spread768; do [[ -z "$(ls -A "experiment/$layout/working")" ]]; done
mkdir measured
bash template-manifest.sh > measured/templates-before.sha256
cmp templates-established.sha256 measured/templates-before.sha256
jq -r '.invocations[] | "\(.block) \(.variant)"' schedule.json > measured/execution-order.txt
[[ "$(wc -l < measured/execution-order.txt | tr -d ' ')" == 9 ]]
while read -r block variant; do
  [[ "$block" =~ ^[0-2]$ && ( "$variant" == counters || "$variant" == fine || "$variant" == empty ) ]]
  label="block${block}-${variant}"
  date -u '+%Y-%m-%dT%H:%M:%SZ' > "measured/$label-host-before.txt"
  uptime >> "measured/$label-host-before.txt"
  printf '{"event":"start","block":%s,"variant":"%s","utc":"%s"}\n' "$block" "$variant" "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" >> measured/execution.jsonl
  printf 'Starting %s\n' "$label"
  set +e
  build-diagnostic/receiver-probe run --root "$root/experiment" --out "$root/measured/$label.jsonl" --block "$block" --variant "$variant" > "measured/$label.stdout" 2> "measured/$label.stderr"
  code=$?
  set -e
  printf '{"event":"end","block":%s,"variant":"%s","exit_code":%s,"utc":"%s"}\n' "$block" "$variant" "$code" "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" >> measured/execution.jsonl
  printf 'Completed %s exit=%s\n' "$label" "$code"
  if [[ "$code" != 0 ]]; then exit "$code"; fi
done < measured/execution-order.txt
bash template-manifest.sh > measured/templates-after.sha256
cmp measured/templates-before.sha256 measured/templates-after.sha256
shasum -a 256 measured/block*.jsonl > measured/observations.sha256
build-diagnostic/profile-analysis "$root/measured" "$root/profile-analysis.json"
