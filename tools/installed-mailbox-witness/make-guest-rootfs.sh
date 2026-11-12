#!/bin/bash
set -euo pipefail
# Synthetic guest contents. These are the entire fixture, before SDK image assembly.
role=source
if [[ "${1-}" == "--cast-destination" ]]; then role=destination; shift; fi
[[ $# -eq 3 ]] || { echo "usage: make-guest-rootfs.sh [--cast-destination] ROOTFS WRAPPER WORKLOAD" >&2; exit 2; }
rootfs=$1
wrapper=$2
workload=$3
mkdir -m700 "$rootfs"
for entry in dev dev/pts data etc etc/mvm home mnt mvm mvm/runtime mvm/sdk proc run sys usr usr/lib usr/lib/mvm usr/lib/mvm/wrappers usr/local usr/local/bin work; do
  mkdir -m755 "$rootfs/$entry"
done
mkdir -m1777 "$rootfs/dev/shm" "$rootfs/tmp"
printf 'mvm-worker:x:901:901:mvm workload:/home/mvm-worker:/bin/sh\n' >"$rootfs/etc/passwd"
printf 'mvm-worker:x:901:\n' >"$rootfs/etc/group"
printf 'oci\n' >"$rootfs/etc/mvm/name"
printf 'dev\n' >"$rootfs/etc/mvm/variant"
chmod 644 "$rootfs/etc/passwd" "$rootfs/etc/group" "$rootfs/etc/mvm/name" "$rootfs/etc/mvm/variant"
install -m555 "$wrapper" "$rootfs/usr/lib/mvm/wrappers/oci-entrypoint"
if [[ "$role" == destination ]]; then
  target="$rootfs/usr/local/bin/spiralcore-cast-recipient"
else
  target="$rootfs/usr/local/bin/spiralcore-mailbox-producer"
fi
install -m555 "$workload" "$target"
cmp "$workload" "$target"
