#!/usr/bin/env bash
# Registered n=1 functional pilot. Each session owns a new persistent receiver.
set -euo pipefail
umask 077
if [[ $# != 5 ]]; then
    printf 'usage: run-pilot.sh HOST_RUNNER KERNEL GUEST_ELF NEW_OUTPUT_DIR REGISTRATION_JSON\n' >&2
    exit 2
fi
runner=$1
kernel=$2
guest=$3
out=$4
registration=$5
[[ ! -e "$out" && ! -L "$out" ]] || { printf 'output must be new\n' >&2; exit 2; }
[[ -x "$runner" && -f "$kernel" && -f "$guest" && -f "$registration" ]] || exit 2
mkdir -- "$out"
cp -- "$registration" "$out/registration.json"
shasum -a 256 "$runner" "$kernel" "$guest" "$registration" > "$out/identities.sha256"
for stage in one two; do
    for size in 1048576 16777216; do
        for arm in whole warm raw; do
            available_kib=$(df -Pk "$out" | awk 'END {print $4}')
            [[ "$available_kib" -ge 4194304 ]] || { printf '4 GiB disk reserve reached\n' >&2; exit 1; }
            name="${stage}-${size}-${arm}"
            printf 'starting %s\n' "$name"
            if "$runner" session "$out/$name" "$kernel" "$guest" "$stage" "$arm" "$size" 0 > "$out/$name.stdout" 2> "$out/$name.stderr"; then
                rc=0
            else
                rc=$?
            fi
            printf '%s\n' "$rc" > "$out/$name.exit"
            if [[ "$rc" != 0 ]]; then
                printf 'failed %s; receipts and owned data retained\n' "$name" >&2
                exit "$rc"
            fi
            printf 'passed %s\n' "$name"
        done
    done
done
printf 'completed all 12 sessions; retain receipts before removing owned cache data\n'
