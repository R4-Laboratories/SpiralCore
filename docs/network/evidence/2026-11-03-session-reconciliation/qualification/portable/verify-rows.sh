#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
: > row-observations.jsonl
printf 'row\ttarget\tlisted\tpassed\tfailed\tignored\tmeasured\tfiltered\n' > target-counts.tsv
for dir in extracted/*/; do
  row=$(jq -r '.row' "$dir/identity.json")
  [[ "$row" =~ ^[a-z0-9-]+$ ]]
  jq -e --slurpfile manifest tested-manifest.json '
    . as $i | ($manifest[0].rows[] | select(.id==$i.row)) as $r |
    .source_commit=="7c10e0f063d1b513881b3ee3d4c9975504d90e11" and
    .github_sha==.source_commit and .run_id=="41677859307" and .attempt=="1" and
    .runner_os==$r.os and .runner_arch==$r.arch and .expected_target==$r.target and
    (.process_architecture|ascii_downcase)==($r.arch|ascii_downcase) and
    (.os_architecture|ascii_downcase)==($r.arch|ascii_downcase) and
    .classification=="native-process / TCP-loopback / ephemeral" and
    ((.runner_os=="Windows" and
      (.manifest_sha256|ascii_downcase)=="78a3aab13ed6d2e2551665b0d45184dbc7afbe57f096da93fae724d67f201da8" and
      (.lock_sha256|ascii_downcase)=="617c3e54c5217ee896ecca8d26fd4fc93abc06ede61fedd1554e8fd264223a7d") or
     (.runner_os!="Windows" and
      (.manifest_sha256|ascii_downcase)=="ad75a86870f243c62fce46724c5717e2b8f8d86df0760f768c8fc399da3b3708" and
      (.lock_sha256|ascii_downcase)=="b611a2dcabad117ddd0a410d71eae8299b0f4c4e1c8f490fbc6beaf076ee1622"))
  ' "$dir/identity.json" >/dev/null
  host=$(awk '/^host: / {sub(/\r$/,""); print $2}' "$dir/rustc.txt")
  [[ "$host" == "$(jq -r '.expected_target' "$dir/identity.json")" ]]
  rg -q '^rustc 1\.97\.1 ' "$dir/rustc.txt"
  jq -e --slurpfile manifest tested-manifest.json '
    length==2 and ([.[].name]|sort)==["mvm-contract","mvm-core"] and
    all(.[]; .source==$manifest[0].mvm_source)
  ' "$dir/mvm-source.json" >/dev/null
  jq -e --slurpfile manifest tested-manifest.json '
    . as $counts | ($manifest[0].tests) as $required |
    length==13 and ([.[].target]|unique|length)==13 and
    ([.[].passed]|add)>=([$required[].minimum_count]|add) and
    all(.[]; .passed==.listed and .passed>0 and .failed==0 and .ignored==0 and .filtered==0) and
    all($required[]; . as $r | any($counts[]; .target==$r.name and .passed>=$r.minimum_count)) and
    any(.[]; .target=="receipt" and .passed==21)
  ' "$dir/test-counts.json" >/dev/null
  storage=0
  storage_file="$dir/unix-storage-counts.json"
  if [[ -f "$storage_file" ]]; then
    storage=$(jq 'if type=="array" then . else [.] end | if length==1 and .[0].target=="canonical_storage" and .[0].passed==2 and .[0].listed==2 and .[0].failed==0 and .[0].ignored==0 and .[0].filtered==0 then 2 else error("storage counts") end' "$storage_file")
    [[ "$(jq -r '.runner_os' "$dir/identity.json")" != Windows ]]
  else
    [[ "$(jq -r '.runner_os' "$dir/identity.json")" == Windows ]]
  fi
  jq -r '.[]|[.target,.listed,.passed]|@tsv' "$dir/test-counts.json" > "$dir/../$row-count-input.tsv"
  if [[ "$storage" == 2 ]]; then
    printf 'canonical_storage\t2\t2\n' >> "$dir/../$row-count-input.tsv"
  fi
  while IFS=$'\t' read -r target listed passed; do
    [[ "$target" =~ ^[a-z_]+$ ]]
    actual_listed=$(awk '/: test\r?$/ {n++} END {print n+0}' "$dir/$target-list.txt")
    [[ "$actual_listed" == "$listed" ]]
    summary=$(awk '/test result: ok\./ {s=substr($0,index($0,"test result: ok.")); split(s,a," "); print a[4],a[6],a[8],a[10],a[12]; n++} END {if(n!=1) exit 1}' "$dir/$target.txt")
    [[ "$summary" == "$passed 0 0 0 0" ]]
    printf '%s\t%s\t%s\t%s\t0\t0\t0\t0\n' "$row" "$target" "$listed" "$passed" >> target-counts.tsv
  done < "$dir/../$row-count-input.tsv"
  portable=$(jq '[.[].passed]|add' "$dir/test-counts.json")
  jq --arg host "$host" --argjson storage "$storage" --argjson portable "$portable" '
    {row,source_commit,github_sha,run_id,attempt,runner_os,runner_arch,
     process_architecture,os_architecture,expected_target,os_description,
     image_os,image_version,classification,manifest_sha256,lock_sha256,
     rustc_host:$host,portable_passed:$portable,receipt_passed:21,unix_storage_passed:$storage,
     failed:0,ignored:0,measured:0,filtered:0}
  ' "$dir/identity.json" >> row-observations.jsonl
done
jq -s 'sort_by(.row)' row-observations.jsonl > rows.json
jq -e 'length==7 and ([.[].row]|unique|length)==7 and ([.[].portable_passed]|add)==665 and ([.[].unix_storage_passed]|add)==8' rows.json >/dev/null
