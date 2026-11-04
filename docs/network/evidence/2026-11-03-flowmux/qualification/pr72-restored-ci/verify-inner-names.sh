#!/bin/bash
set -u -o pipefail
out=${1:-/tmp/spiralcore-pr72-gate-20261104/restored-ci}
if ! printf '%s\n' clippy.txt counts.json identity.json rustc.txt test-list.txt tests.txt | sort > "$out/expected-inner-files.txt"; then exit 1; fi
for dir in "$out"/derived/flowmux-component-*; do
    if ! awk '{print $NF}' "$dir/normalized-checksums.txt" | sed 's/^\*//' | sort > "$dir/inner-files.txt"; then exit 1; fi
    if ! cmp "$out/expected-inner-files.txt" "$dir/inner-files.txt"; then exit 1; fi
    printf '%s:6 distinct expected files\n' "${dir##*/}"
done
