#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
for file in run.json ../portable/run.json; do
 jq -e '.head_sha=="b27fbba9bdc636575266d6da04ec8f1ad5c7c2ea" and .status=="completed" and .conclusion=="success" and .run_attempt==1 and .event=="pull_request"' "$file" >/dev/null
done
jq -e '(.jobs|length)==8 and all(.jobs[]; .conclusion=="success" and all(.steps[]; .conclusion=="success" or .conclusion=="skipped"))' ../portable/jobs.json >/dev/null
jq -e '(.jobs|length)==1 and all(.jobs[]; .conclusion=="success" and all(.steps[]; .conclusion=="success"))' jobs.json >/dev/null
awk -f derive-linux.awk run.log > test-counts.tsv
awk -F '\t' 'BEGIN {OFS="\t"; print "scope","passed","result_summaries","zero_test_summaries"}
 NR>1 {n[$1]++;p[$1]+=$3;z[$1]+=($3==0); for(i=4;i<=7;i++)if($i!=0)bad=1}
 END {for(s in n)print s,p[s],n[s],z[s]+0; if(p["release-live-all-targets"]!=111 || p["shared-object-v1"]!=8 || p["shared-adapter-all-targets"]!=130)bad=1; exit bad}' test-counts.tsv > summary.tsv
rg -q '7c10e0f063d1b513881b3ee3d4c9975504d90e11$' run.log
rg -q 'features linux-host --all-targets -- -D warnings' run.log
rg -q 'cargo \+1.97.1 check --locked' run.log
rg -q 'PASS: valid synthetic legacy/v1 fixture; rejects altered frame bytes, identities and duplicate cells' run.log
rg -q 'PASS: HVF and Linux synthetic sessions, known paired ratios, exact readback and rejecting mutations' run.log
awk -F '\t' 'NR>1&&$2=="service_tcp" {n++;if($3!=14||$4!=14)bad=1} END {exit(bad||n!=7)}' ../portable/target-counts.tsv
awk -F '\t' 'NR>1&&$2=="call_send" {n++;if($3!=6||$4!=6)bad=1} END {exit(bad||n!=7)}' ../portable/target-counts.tsv
printf 'PASS: exact-head run/job conclusions, checkout, measured scope/count/zero summaries, TCP14/call_send6 and retained build/analyzer logs\n'
