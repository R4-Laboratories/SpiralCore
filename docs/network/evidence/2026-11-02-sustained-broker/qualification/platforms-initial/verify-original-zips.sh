#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
for archive in original-zips/*.zip; do
  name=$(basename "$archive" .zip)
  expected=$(jq -r --arg name "$name" '.artifacts[] | select(.name==$name) | .digest | sub("^sha256:"; "")' artifact-api.json)
  actual=$(shasum -a 256 "$archive" | awk '{print $1}')
  [[ "$expected" == "$actual" ]]
  files=0
  while IFS= read -r member; do
    [[ "$member" =~ ^[a-zA-Z0-9_.-]+$ ]]
    files=$((files+1))
  done < <(unzip -Z1 "$archive")
  entries=0
  # Normalize only the manifest parser; preserve every original ZIP byte.
  while read -r hash member; do
    [[ "$hash" =~ ^[a-fA-F0-9]{64}$ && "$member" =~ ^[a-zA-Z0-9_.-]+$ ]]
    actual=$(unzip -p "$archive" "$member" | shasum -a 256 | awk '{print $1}')
    [[ "$hash" == "$actual" ]]
    entries=$((entries+1))
  done < <(unzip -p "$archive" SHA256SUMS | LC_ALL=C sed $'s/\r$//;1s/^\xef\xbb\xbf//')
  [[ "$entries" -eq $((files-1)) ]]
  identity=$(unzip -p "$archive" identity.json | jq -c '{row,source_commit,run_id,attempt,runner_os,runner_arch,image_os,image_version,os_description,process_architecture,os_architecture,expected_target,classification,manifest_sha256,lock_sha256}')
  [[ $(jq -r '.source_commit' <<< "$identity") == 18e7a6c42c73ed1ec7ad71731240710f5ed0621a ]]
  [[ $(jq -r '.run_id' <<< "$identity") == 41577442243 ]]
  rustc_host=$(unzip -p "$archive" rustc.txt | LC_ALL=C sed $'s/\r$//' | sed -n 's/^host: //p')
  rustc_release=$(unzip -p "$archive" rustc.txt | LC_ALL=C sed $'s/\r$//' | sed -n 's/^release: //p')
  [[ "$rustc_host" == $(jq -r '.expected_target' <<< "$identity") && "$rustc_release" == 1.97.1 ]]
  [[ $(jq -r '.runner_arch' <<< "$identity") == $(jq -r '.process_architecture' <<< "$identity") ]]
  [[ $(jq -r '.runner_arch' <<< "$identity") == $(jq -r '.os_architecture' <<< "$identity") ]]
  identity=$(jq -c --arg host "$rustc_host" --arg release "$rustc_release" '.+{rustc_host:$host,rustc_release:$release}' <<< "$identity")
  unzip -p "$archive" mvm-source.json | jq -e 'length==2 and all(.[]; .source=="git+https://github.com/Casey-allard/mvm?rev=ae12e9deb41cc9a3264196ce22ebc72d4042883d#ae12e9deb41cc9a3264196ce22ebc72d4042883d")' > /dev/null
  counts=$(unzip -p "$archive" test-counts.json | jq -c 'if all(.[]; .listed == .passed and .failed==0 and .ignored==0 and .filtered==0) then . else error("failed native test gate") end')
  storage='[]'
  if unzip -Z1 "$archive" | rg -q '^unix-storage-counts.json$'; then
    storage=$(unzip -p "$archive" unix-storage-counts.json | jq -c 'if type=="array" then . else [.] end | if all(.[]; .listed == .passed and .failed==0 and .ignored==0 and .filtered==0) then . else error("failed storage test gate") end')
  fi
  jq -nc --arg archive "$archive" --arg sha256 "$expected" --argjson files "$files" --argjson entries "$entries" --argjson identity "$identity" --argjson counts "$counts" --argjson storage "$storage" '{archive:$archive,sha256:$sha256,original_files:$files,verified_inner_checksums:$entries,identity:$identity,portable_counts:$counts,unix_storage_counts:$storage}'
done
