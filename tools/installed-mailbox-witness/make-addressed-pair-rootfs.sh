#!/bin/bash
set -euo pipefail
umask 077
[[ $# -eq 4 ]] || { echo "usage: make-addressed-pair-rootfs.sh FRESH_DIRECTORY WRAPPER SUBMIT_WORKLOAD RECEIVE_WORKLOAD" >&2; exit 2; }
dest=$1
wrapper=$2
submit=$3
receive=$4
for program in "$wrapper" "$submit" "$receive"; do
  [[ -f "$program" && -x "$program" ]] || { echo "missing executable fixture" >&2; exit 1; }
done
if cmp -s "$submit" "$receive"; then echo "addressed source and destination workloads are identical" >&2; exit 1; fi
dir="$(cd "$(dirname "$0")" && pwd)"
mkdir -m700 "$dest"
bash "$dir/make-guest-rootfs.sh" "$dest/a" "$wrapper" "$submit"
bash "$dir/make-guest-rootfs.sh" --cast-destination "$dest/b" "$wrapper" "$receive"
(
  cd "$dest"
  sha256sum a/usr/lib/mvm/wrappers/oci-entrypoint a/usr/local/bin/spiralcore-mailbox-producer \
            b/usr/lib/mvm/wrappers/oci-entrypoint b/usr/local/bin/spiralcore-cast-recipient > guest-inputs.sha256
  sha256sum -c guest-inputs.sha256
)
