#!/usr/bin/env bash
# Verify the seven original gh-api ZIP downloads without rewriting their bytes.
set -euo pipefail
cd "$(dirname "$0")"
jq -e '.total_count==7 and (.artifacts|length)==7 and all(.artifacts[]; .expired==false and .workflow_run.id==41649886300 and .workflow_run.head_sha=="d14271ddec5424326603572693c15aeb1a2f2e48")' artifacts.json >/dev/null
jq -r '.artifacts[] | [.id,.name,.digest,.size_in_bytes] | @tsv' artifacts.json > artifact-api.tsv
printf 'artifact_id\tname\tapi_sha256\tactual_sha256\tarchive_bytes\tinner_checksums\n' > custody.tsv
while IFS=$'\t' read -r id name digest expected_bytes; do
  [[ "$id" =~ ^[0-9]+$ && "$name" =~ ^mvm-mailbox-portable-[a-z0-9-]+-1$ ]]
  archive="zips/$name.zip"
  # Original command used once for each id, before this verifier:
  # gh api "repos/R4-Laboratories/SpiralCore/actions/artifacts/$id/zip" > "$archive"
  actual=$(shasum -a 256 "$archive" | awk '{print $1}')
  expected=${digest#sha256:}
  [[ "$actual" == "$expected" ]]
  bytes=$(wc -c < "$archive" | tr -d ' ')
  [[ "$bytes" == "$expected_bytes" ]]
  # This evidence format consists solely of flat regular files.
  unzip -Z1 "$archive" | awk '!/^[A-Za-z0-9_.-]+$/ {bad=1} END {exit bad}'
  unzip -Z -l "$archive" | awk '/^[ldcbps]/ {bad=1} END {exit bad}'
  dest="extracted/$name"
  [[ ! -e "$dest" ]]
  mkdir "$dest"
  unzip -q "$archive" -d "$dest"
  # Only CRLF in checksum-manifest parsing is adapted; stored files are intact.
  (cd "$dest"; tr -d '\r' < SHA256SUMS | shasum -a 256 -c -) > "extracted/$name.checksums.txt"
  count=$(wc -l < "$dest/SHA256SUMS" | tr -d ' ')
  entries=$(unzip -Z1 "$archive" | wc -l | tr -d ' ')
  [[ "$entries" == "$((count+1))" ]]
  [[ $(rg -c ': OK$' "extracted/$name.checksums.txt") == "$count" ]]
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$id" "$name" "$expected" "$actual" "$bytes" "$count" >> custody.tsv
done < artifact-api.tsv
