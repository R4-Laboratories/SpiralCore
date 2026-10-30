#!/bin/bash
set -euo pipefail

# Custody only: reads retained archives and files, writes a fresh result directory.
[[ $# == 5 ]] || { printf 'expected archive remote_dir member_list count new_output_dir\n' >&2; exit 1; }
archive=$1
remote=$2
members=$3
expected=$4
out=$5
[[ -f "$archive" && ! -L "$archive" && -d "$remote" && ! -L "$remote" && -f "$members" && ! -L "$members" ]] || exit 1
[[ "$expected" =~ ^[0-9]+$ && "$expected" -ge 1 && "$expected" -le 64 ]] || exit 1
[[ $(wc -c < "$archive") -le 268435456 && $(wc -c < "$members") -le 16384 ]] || exit 1
[[ ! -e "$out" ]] || exit 1
mkdir "$out"
unzip -tq "$archive" > "$out/zip-integrity.txt"
unzip -Z1 "$archive" | LC_ALL=C sort > "$out/archive-members.txt"
LC_ALL=C sort "$members" > "$out/supplied-members.txt"
cmp "$out/archive-members.txt" "$out/supplied-members.txt"
[[ $(wc -l < "$out/supplied-members.txt") -eq "$expected" ]] || exit 1
[[ -z $(uniq -d "$out/supplied-members.txt") ]] || exit 1
[[ -z $(find "$remote" -type l -print) ]] || exit 1
(cd "$remote" && find . -type f | sed 's|^./||' | LC_ALL=C sort) > "$out/remote-members.txt"
cmp "$out/archive-members.txt" "$out/remote-members.txt"
visited=0
total_bytes=0
while IFS= read -r member; do
  [[ "$member" =~ ^[a-zA-Z0-9_./-]+$ ]] || exit 1
  case "$member" in /*|../*|*/../*|*/..|./*|*/./*|*/.|*//* ) exit 1;; esac
  [[ -f "$remote/$member" && ! -L "$remote/$member" ]] || exit 1
  bytes=$(wc -c < "$remote/$member")
  [[ "$bytes" -le 268435456 ]] || exit 1
  total_bytes=$((total_bytes + bytes))
  [[ "$total_bytes" -le 1073741824 ]] || exit 1
  member_sha=$(unzip -p "$archive" "$member" </dev/null | shasum -a 256 | awk '{print $1}')
  file_sha=$(shasum -a 256 "$remote/$member" | awk '{print $1}')
  [[ "$member_sha" == "$file_sha" ]] || exit 1
  printf '%s  %s\n' "$member_sha" "$member"
  visited=$((visited + 1))
done < "$out/supplied-members.txt" > "$out/remote-files.sha256"
[[ "$visited" -eq "$expected" ]] || exit 1
[[ -s "$out/remote-files.sha256" && $(wc -l < "$out/remote-files.sha256") -eq "$expected" ]] || exit 1
archive_sha=$(shasum -a 256 "$archive" | awk '{print $1}')
archive_bytes=$(wc -c < "$archive" | tr -d ' ')
manifest_sha=$(shasum -a 256 "$out/remote-files.sha256" | awk '{print $1}')
jq -n --arg archive "$archive" --arg remote "$remote" --arg archive_sha "$archive_sha" --arg manifest_sha "$manifest_sha" --argjson archive_bytes "$archive_bytes" --argjson expected "$expected" --argjson visited "$visited" --argjson total_bytes "$total_bytes" '{schema:"spiralcore.artifact-member-recheck.v1",status:"passed",archive_path:$archive,remote_directory:$remote,archive_sha256:$archive_sha,archive_bytes:$archive_bytes,expected_members:$expected,visited_members:$visited,manifest_lines:$visited,manifest_sha256:$manifest_sha,uncompressed_bytes_compared:$total_bytes,comparison:"Each actual ZIP member SHA-256 equals the retained remote-file SHA-256; exact member set and count also match",raw_inputs_modified:false}' > "$out/recheck.json"
