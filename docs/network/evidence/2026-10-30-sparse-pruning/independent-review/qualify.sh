#!/bin/bash
# Synthetic checker qualification; never substitute these values for observations.
set -eu
set -o pipefail
review=docs/network/evidence/2026-10-30-sparse-pruning/independent-review
source_pin=1111111111111111111111111111111111111111
binary_pin=2222222222222222222222222222222222222222222222222222222222222222
jq -nc -f "$review/synthetic-fixture.jq" > "$review/synthetic-fixture.jsonl"
verify() { jq -s --arg expected_source "$source_pin" --arg expected_executable "$binary_pin" -f "$review/verify.jq"; }
verify < "$review/synthetic-fixture.jsonl" > "$review/synthetic-result.json"
jq -e '.cpu_acceptance_pass and all(.cells[]; if .primary then (.legacy_over_candidate_cpu_geometric_mean-1.25|fabs)<0.000000000001 and .candidate_cpu_wins==10 else (.candidate_over_legacy_cpu_geometric_mean-1.02|fabs)<0.000000000001 end)' "$review/synthetic-result.json" > /dev/null
: > "$review/checker-tests.jsonl"
reject() {
  label=$1; mutation=$2; expected=$3
  set +e
  jq -sc "$mutation" "$review/synthetic-fixture.jsonl" | verify > /dev/null 2> "$review/checker-current.stderr"
  code=$?
  set -e
  message=$(cat "$review/checker-current.stderr")
  test "$code" -eq 5
  case "$message" in *"$expected"*) ;; *) exit 1;; esac
  jq -nc --arg test "$label" --arg mutation "$mutation" --arg expected "$expected" --arg actual "$message" --argjson code "$code" '{test:$test,status:"PASS_REJECTED",mutation:$mutation,expected_error:$expected,actual_error:$actual,exit_code:$code}' >> "$review/checker-tests.jsonl"
}
reject missing_sample 'del(.[2])|.[]' 'row cardinality'
reject wrong_pair '.[2].pair=9|.[]' 'pair identity or AB/BA schedule'
reject wrong_source '.[2].source_revision=("0"*40)|.[]' 'sample source or executable pin'
reject wrong_binary '.[2].executable_sha256=("0"*64)|.[]' 'sample source or executable pin'
reject wrong_output '.[2].recipe.frame_sha256=("0"*64)|.[]' 'paired fixture or output mismatch'
reject wrong_base '.[2].base_object=("0"*64)|.[]' 'paired fixture or output mismatch'
reject wrong_arithmetic '.[2].encode_plus_drop_cpu_micros+=1|.[]' 'timing arithmetic or resolution label'
reject forged_completion '.[-1].samples=139|.[]' 'producer completion disagrees'
# A valid but unfavorable control observation must remain valid data and fail
# acceptance, not be rejected as malformed or silently dropped.
jq -sc '(.[]|select(.event=="sample" and .case=="repeat" and .arm=="new")) |= (.encode_cpu_micros=1060|.encode_plus_drop_cpu_micros=1070)|.[]' "$review/synthetic-fixture.jsonl" | verify > "$review/synthetic-negative-result.json"
jq -e '.status=="VALID_RECORDS" and (.cpu_acceptance_pass|not) and any(.cells[];.case=="repeat" and (.cpu_gate_pass|not))' "$review/synthetic-negative-result.json" > /dev/null
jq -nc '{test:"retain_valid_control_regression",status:"PASS_NEGATIVE_RESULT_RETAINED"}' >> "$review/checker-tests.jsonl"
rm "$review/checker-current.stderr"
