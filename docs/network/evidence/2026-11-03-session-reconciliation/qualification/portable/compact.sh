#!/usr/bin/env bash
# Remove only this verifier's duplicate extracted members; preserve original ZIPs.
set -euo pipefail
cd "$(dirname "$0")"
while IFS=$'\t' read -r id name digest expected_bytes; do
  [[ "$name" =~ ^mvm-mailbox-portable-[a-z0-9-]+-1$ ]]
  [[ "$(shasum -a 256 "archives/$name.zip" | awk '{print $1}')" == "${digest#sha256:}" ]]
  [[ "$(rg -c ': OK$' "extracted/$name.checksums.txt")" == "$(wc -l < "extracted/$name/SHA256SUMS" | tr -d ' ')" ]]
  rm -r -- "extracted/$name"
done < artifact-api.tsv
