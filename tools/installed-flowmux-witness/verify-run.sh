#!/usr/bin/env bash
# Structural readback only; canonical audit signature verification remains separate.
set -euo pipefail
if [[ $# != 1 ]]; then printf 'expected one private result directory\n' >&2; exit 1; fi
run=$1
for file in results controller.log peer.log drained/flowmux-operator.json drained/flowmux-guest.json drained/.archived-audit-0 drained/local.fm-a.workload.jsonl; do
  if [[ ! -f $run/$file || -L $run/$file || ! -s $run/$file ]]; then
    printf 'missing original result or audit artifact\n' >&2
    exit 1
  fi
done
if [[ $(cat "$run/results") != $'sdk=0\npeer=0\nnamespace_exit=0' ]]; then
  printf 'original process statuses differ from success\n' >&2
  exit 1
fi
if [[ $(grep -Ec '^test result: ok\. 1 passed; 0 failed; 0 ignored;' "$run/controller.log") != 1 ]]; then
  printf 'expected exactly one completed original workload test\n' >&2
  exit 1
fi
if [[ $(grep -Ec '^test installed_original_flowmux_workload \.\.\. ' "$run/controller.log") != 1 ]]; then
  printf 'expected the original named workload invocation\n' >&2
  exit 1
fi
jq -Rse '
  def one($name):
    [.[] | select(.phase == $name)] |
    if length == 1 then .[0] else error("missing or duplicate controller phase") end;
  split("\n") |
  map(ltrimstr("test installed_original_flowmux_workload ... ") | fromjson?) as $rows |
  ($rows | one("original_entrypoint")) as $entry |
  ($rows | one("sdk_retirement")) as $retired |
  ($rows | one("installed_resources")) as $resources |
  ($rows | one("original_retained_reservations")) as $retained |
  $entry.original_workload_exit == 0 and $entry.stderr_bytes == 0 and
  $retired.retired == true and $resources.tenant_stopped == true and
  $resources.charged.guests == 0 and $resources.charged.workers == 0 and
  $resources.all_resources_released == false and $retained.verified_four == true
' "$run/controller.log" >/dev/null
jq -Rse '
  def positive_integer:
    if type == "number" then . > 0 and . == floor else false end;
  [split("\n")[] | fromjson? | select(has("echo_received"))] |
  length == 1 and .[0].stream_id == 3 and
  .[0].echo_received == 5804 and .[0].echo_written == 5804 and
  (.[0].tls_rx | positive_integer) and (.[0].tls_tx | positive_integer) and
  (.[0].accept_to_half_close_ns | positive_integer)
' "$run/peer.log" >/dev/null
printf 'original result structure and required artifacts present; verify signed audit independently\n'
