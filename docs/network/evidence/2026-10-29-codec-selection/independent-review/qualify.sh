#!/bin/bash
# Run from the repository root. Negative inputs are transformations of the retained original.
set -u
set -o pipefail
review=docs/network/evidence/2026-10-29-codec-selection/independent-review
data=docs/network/evidence/2026-10-29-codec-selection/measurements.jsonl
verifier="$review/verify.jq"
receipt="$review/negative-tests.jsonl"
scratch="$review/negative-current.stderr"
: > "$receipt"
reject() {
  label=$1
  filter=$2
  expected=$3
  jq -s -c "$filter" "$data" | jq -s -f "$verifier" > /dev/null 2> "$scratch"
  result=$?
  message=$(cat "$scratch")
  if test "$result" -eq 0; then
    jq -n -c --arg test "$label" '{test:$test,status:"FAIL: invalid input accepted"}' >> "$receipt"
    exit 1
  fi
  case "$message" in
    *"$expected"*) ;;
    *) jq -n -c --arg test "$label" --arg message "$message" '{test:$test,status:"FAIL: wrong rejection",message:$message}' >> "$receipt"; exit 1 ;;
  esac
  jq -n -c --arg test "$label" --arg filter "$filter" --arg expected "$expected" --arg message "$message" --argjson exit_code "$result" \
    '{test:$test,status:"PASS_REJECTED",mutation:$filter,expected_error:$expected,actual_error:$message,exit_code:$exit_code}' >> "$receipt"
}
reject missing_completion 'map(select(.event!="completion"))|.[]' 'row cardinality'
reject source_mismatch '.[0].claimed_source_revision=("0"*40)|.[]' 'source identity'
reject duplicate_operation '(map(select(.event=="operation"))[0]) as $first | (.[]|select(.event=="operation" and .arm=="zstd" and .phase=="transfer" and .trial==0 and .case=="cold_random" and .size==1048576))=$first |.[]' 'duplicate operation identity'
reject damaged_body_count '(.[]|select(.event=="operation" and .arm=="adaptive" and .phase=="transfer" and .trial==0 and .case=="localized_32" and .size==1048576)|.http_body_equivalent_bytes)+=1|.[]' 'body arithmetic or batches'
reject wrong_base '(.[]|select(.event=="operation" and .arm=="adaptive" and .phase=="transfer" and .trial==0 and .case=="localized_32" and .size==1048576)|.base)=("0"*64)|.[]' 'paired identity, seed, or selector minimum mismatch'
reject wrong_predicted_bytes '(.[]|select(.event=="operation" and .arm=="adaptive" and .phase=="transfer" and .trial==0 and .case=="localized_32" and .size==1048576)|.selection.expected_http_body_bytes)+=1|.[]' 'paired identity, seed, or selector minimum mismatch'
reject wrong_cleanup_path '(.[]|select(.event=="cleanup" and .label=="1048576-cold_random-00-raw")|.cache)="/not-the-owned-cache"|.[]' 'cleanup path binding'
reject wrong_summary_median '(.[]|select(.event=="summary" and .arm=="adaptive" and .case=="localized_32" and .size==1048576)|.operation_cpu_micros.median)+=1|.[]' 'summary mismatch'
reject wrong_lifecycle_p95 '(.[]|select(.event=="summary" and .arm=="adaptive" and .case=="unrelated_structured" and .size==16777216)|.seed_plus_transfer_operation_wall_micros.p95)+=1|.[]' 'summary mismatch'
rm "$scratch"
