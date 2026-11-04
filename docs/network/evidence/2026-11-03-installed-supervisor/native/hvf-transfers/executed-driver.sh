#!/bin/bash
set -euo pipefail
umask 077
base=/private/tmp/spiralcore-installed-mac-20261103
id=5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a
for size in 1048576 16777216; do
  for arm in whole-mixed combined-cpu; do
    cell="two-$arm-$size-halfclose"
    if [ -e "$base/native/$cell" ]; then exit 95; fi
    set +e
    /usr/bin/time -l "$base/bin/host-halfclose" session-installed "$base/native/installation" "$id" "$base/native/$cell" \
      /Users/casey.allard/spiralcore/local/mvm-vsock-20261031/assets/Image "$base/bin/guest" two "$arm" "$size" 0 \
      > "$base/validation/$cell.stdout" 2> "$base/validation/$cell.stderr"
    task_rc=$?
    set -e
    printf '%s\n' "$task_rc" > "$base/validation/$cell.exit"
    if [ "$task_rc" -ne 0 ]; then tail -20 "$base/validation/$cell.stderr"; exit "$task_rc"; fi
    cat "$base/validation/$cell.stdout"
    jq -e '.ok == true and .receiver.exact_reopened == true and .sender_worker.installed_host_lease == true and .sender_worker.backend_exit_confirmed == true and .receiver_worker.installed_host_lease == true and .receiver_worker.backend_exit_confirmed == true and .recovery_worker.installed_host_lease == true and .recovery_worker.backend_exit_confirmed == true' "$base/native/$cell/result.json" > "$base/validation/$cell.accepted"
  done
done
