#!/bin/bash
set -eu
set -o pipefail
review=docs/network/evidence/2026-10-30-sparse-pruning/independent-review
source_pin=1111111111111111111111111111111111111111
binary_pin=2222222222222222222222222222222222222222222222222222222222222222
candidate_pin=3333333333333333333333333333333333333333333333333333333333333333
legacy_pin=4444444444444444444444444444444444444444444444444444444444444444
jq -c -f "$review/synthetic-allocations.jq" "$review/synthetic-fixture.jsonl" > "$review/synthetic-allocations.jsonl"
verify() { jq -s --arg expected_source "$source_pin" --arg expected_executable "$binary_pin" --arg expected_candidate "$candidate_pin" --arg expected_legacy "$legacy_pin" --slurpfile timing "$review/synthetic-fixture.jsonl" -f "$review/verify-allocations.jq"; }
verify < "$review/synthetic-allocations.jsonl" > "$review/synthetic-allocation-result.json"
jq -e '.memory_acceptance_pass and all(.cells[]|select(.primary);.memory_gate_pass)' "$review/synthetic-allocation-result.json" > /dev/null
: > "$review/allocation-checker-tests.jsonl"
reject() {
  label=$1; mutation=$2; expected=$3
  set +e
  jq -sc "$mutation" "$review/synthetic-allocations.jsonl" | verify > /dev/null 2> "$review/allocation-checker-current.stderr"
  code=$?
  set -e
  message=$(cat "$review/allocation-checker-current.stderr")
  test "$code" -eq 5
  case "$message" in *"$expected"*) ;; *) exit 1;; esac
  jq -nc --arg test "$label" --arg mutation "$mutation" --arg expected "$expected" --arg actual "$message" --argjson code "$code" '{test:$test,status:"PASS_REJECTED",mutation:$mutation,expected_error:$expected,actual_error:$actual,exit_code:$code}' >> "$review/allocation-checker-tests.jsonl"
}
reject missing_sample 'del(.[2])|.[]' 'allocation row cardinality'
reject wrong_pair '.[2].pair=9|.[]' 'allocation pair identity or schedule'
reject wrong_candidate_source '.[2].candidate_source_sha256=("0"*64)|.[]' 'allocation sample identity'
reject wrong_output_join '.[2].target_object=("0"*64)|.[]' 'normal and diagnostic output join'
reject leaked_buffer '.[2].after_drop.live_bytes=1|.[]' 'allocation lifetime arithmetic'
reject registry_failure '.[2].at_return.registry_failed=true|.[]' 'allocation lifetime arithmetic'
reject wrong_retained_capacity '.[2].at_return.live_bytes=18|.[]' 'allocation lifetime arithmetic'
reject wrong_completion '.[-1].allocation_diagnostic=false|.[]' 'allocation completion disagrees'
jq -sc '(.[]|select(.event=="allocation_sample" and .case=="prefix_insert" and .arm=="new" and .pair==0)) |= (.at_return.peak_bytes=600|.after_drop.peak_bytes=600|.at_return.cumulative_allocated_bytes=1200|.after_drop.cumulative_allocated_bytes=1200)|.[]' "$review/synthetic-allocations.jsonl" | verify > "$review/synthetic-negative-allocation-result.json"
jq -e '.status=="VALID_RECORDS" and (.memory_acceptance_pass|not) and any(.cells[];.case=="prefix_insert" and (.memory_gate_pass|not))' "$review/synthetic-negative-allocation-result.json" > /dev/null
jq -nc '{test:"retain_one_pair_memory_gate_failure",status:"PASS_NEGATIVE_RESULT_RETAINED"}' >> "$review/allocation-checker-tests.jsonl"
jq -sc '(.[]|select(.event=="allocation_sample" and .case=="prefix_insert" and .arm=="new" and .pair==0)) |= (.at_return.header_allocations+=1|.after_drop.header_allocations+=1|.at_return.frees+=1|.after_drop.frees+=1)|.[]' "$review/synthetic-allocations.jsonl" | verify > "$review/synthetic-materialization-failure-result.json"
jq -e '.status=="VALID_RECORDS" and (.memory_acceptance_pass|not) and any(.cells[];.case=="prefix_insert" and any(.paired_peaks[]; .pruned_candidate_zero_headers|not))' "$review/synthetic-materialization-failure-result.json" > /dev/null
jq -nc '{test:"retain_rejected_candidate_materialization_failure",status:"PASS_NEGATIVE_RESULT_RETAINED"}' >> "$review/allocation-checker-tests.jsonl"
rm "$review/allocation-checker-current.stderr"
