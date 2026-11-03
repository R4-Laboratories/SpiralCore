#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
for file in run.json ../linux-run.json; do
  jq -e '.head_sha=="d14271ddec5424326603572693c15aeb1a2f2e48" and .status=="completed" and .conclusion=="success" and .run_attempt==1 and .event=="pull_request"' "$file" >/dev/null
done
jq -e '(.jobs|length)==8 and all(.jobs[]; .conclusion=="success" and all(.steps[]; .conclusion=="success" or .conclusion=="skipped"))' jobs.json >/dev/null
jq -e '(.jobs|length)==1 and all(.jobs[]; .conclusion=="success" and all(.steps[]; .conclusion=="success"))' ../linux-jobs.json >/dev/null
awk -f derive-linux.awk ../linux-run.log > linux-test-counts.tsv
awk -F '\t' 'NR>1 {n[$1]++;p[$1]+=$3;z[$1]+=($3==0);for(i=4;i<=7;i++)if($i!=0)bad=1}
END {if(n["release-live-all-targets"]!=10||p["release-live-all-targets"]!=92||z["release-live-all-targets"]!=2||n["shared-object-v1"]!=1||p["shared-object-v1"]!=8||n["shared-adapter-all-targets"]!=22||p["shared-adapter-all-targets"]!=127||z["shared-adapter-all-targets"]!=2)bad=1;exit bad}' linux-test-counts.tsv
rg -q '1746e57c1adaf57a32269ee2d7bde213d4fed5ce$' ../linux-run.log
rg -q 'features linux-host --all-targets -- -D warnings' ../linux-run.log
rg -q 'cargo \+1.97.1 check --locked' ../linux-run.log
rg -q 'PASS: valid synthetic legacy/v1 fixture; rejects altered frame bytes, identities and duplicate cells' ../linux-run.log
rg -q 'PASS: HVF and Linux synthetic sessions, known paired ratios, exact readback and rejecting mutations' ../linux-run.log
# Explicit target-count assertion, independently of aggregate/minimum checks.
awk -F '\t' 'NR>1&&$2=="service_tcp" {n++;if($3!=11||$4!=11)bad=1} END {exit(bad||n!=7)}' target-counts.tsv
printf 'PASS: run/job conclusions, checkout, exact scopes/counts/zeros, TCP11 and retained build/analyzer records\n'
awk -F '\t' 'NR>1&&$2=="call_send" {n++;if($3!=6||$4!=6)bad=1} END {exit(bad||n!=7)}' target-counts.tsv
