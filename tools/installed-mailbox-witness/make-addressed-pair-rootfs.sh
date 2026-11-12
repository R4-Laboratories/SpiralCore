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
for role in a b; do
  case "$role" in
    a) program=/usr/local/bin/spiralcore-mailbox-producer ;;
    b) program=/usr/local/bin/spiralcore-cast-recipient ;;
  esac
  config="$dest/$role/etc/mvm"
  printf '%s\n' '/usr/lib/mvm/wrappers/oci-entrypoint' > "$config/entrypoint"
  printf '{"argv":["%s"],"env":[],"working_dir":"/"}\n' "$program" > "$config/image-runtime.json"
  printf '{"language":"rust","module":"cast","function":"main","format":"binary","source_path":"%s","concurrency":{"kind":"warm_process","max_calls_per_worker":1,"max_rss_mb":64,"pool_size":1,"in_process":"serial","max_queue_depth":1}}\n' "$program" > "$config/runtime.json"
  chmod 0644 "$config/entrypoint" "$config/image-runtime.json" "$config/runtime.json"
done
(
  cd "$dest"
  sha256sum a/usr/lib/mvm/wrappers/oci-entrypoint a/usr/local/bin/spiralcore-mailbox-producer \
            b/usr/lib/mvm/wrappers/oci-entrypoint b/usr/local/bin/spiralcore-cast-recipient \
            a/etc/mvm/{entrypoint,image-runtime.json,runtime.json} b/etc/mvm/{entrypoint,image-runtime.json,runtime.json} > guest-inputs.sha256
  sha256sum -c guest-inputs.sha256
)
