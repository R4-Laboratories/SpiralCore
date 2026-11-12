#!/bin/bash
set -euo pipefail
if [[ $# -ne 6 ]]; then echo "Usage: $0 IMAGE MOUNTPOINT ROLE WORKLOAD_SHA256 WRAPPER_SHA256 PROBE" >&2; exit 2; fi
# The caller owns a fresh region beneath a 0700 test directory. The mount only exists
# inside this process's private namespace. No privileged Python runs.
sudo -n unshare --mount --propagation private /bin/sh -euc '
    mount -t ext4 -o ro,noload,loop,nosuid,nodev "$1" "$2"
    exec setpriv --reuid=1000 --regid=1000 --clear-groups --bounding-set=-all --inh-caps=-all --ambient-caps=-all --no-new-privs -- /usr/bin/python3 -I "$6" "$2" "$3" "$4" "$5"
' check-service "$@"
