#!/bin/bash
set -u
root=/tmp/spiralcore-pr72-gate-20261104
out=$root/qualification
head=57c09d9253f9fb509044b578dd278e777f744a13
merge=fcf0555e1d60f8ccb068a630615b207aa4f40d94
tree=38fc292eab1dcfc5df2138f8b48f23eb219e0dc2
if ! jq -e --arg head "$head" '.head_sha==$head and .id==41777249828 and .conclusion=="success" and .status=="completed" and .run_attempt==1' "$out/run.json" >/dev/null; then exit 81; fi
if ! jq -e --arg head "$head" --arg merge "$merge" --arg tree "$tree" '.sha==$merge and .tree==$tree and (.parents|index($head)!=null)' "$out/tested-merge.json" >/dev/null; then exit 82; fi
if ! jq -e 'length==8 and all(.[];.status=="completed" and .conclusion=="success")' "$out/job-statuses.json" >/dev/null; then exit 83; fi
if ! jq -e '.artifacts|length==7 and all(.[];.expired==false and (.digest|startswith("sha256:")))' "$out/artifacts.json" >/dev/null; then exit 84; fi
mkdir -p "$out/portable/derived"
printf '%s\n' SHA256SUMS clippy.txt counts.json identity.json rustc.txt test-list.txt tests.txt | sort > "$out/portable/expected-entries.txt"
lf_lock=$(shasum -a 256 "$root/source/crates/spiralcore-mvm-flowmux/Cargo.lock" | awk '{print $1}')
crlf_lock=$(awk '{printf "%s\r\n",$0}' "$root/source/crates/spiralcore-mvm-flowmux/Cargo.lock" | shasum -a 256 | awk '{print $1}')
printf 'artifact\tapi_sha256\tactual_sha256\tzip_bytes\tinner_checks\tsource\tlisted\tpassed\ttest_scopes\tlock_bytes\n' > "$out/portable/custody.tsv"
while IFS=$'\t' read -r name digest bytes; do
  zip="$out/portable/archives/$name.zip"
  case "$name" in
    flowmux-component-ubuntu24-x64-1) os=Linux; arch=X64; target=x86_64-unknown-linux-gnu;;
    flowmux-component-ubuntu24-arm64-1) os=Linux; arch=ARM64; target=aarch64-unknown-linux-gnu;;
    flowmux-component-macos26-intel-1) os=macOS; arch=X64; target=x86_64-apple-darwin;;
    flowmux-component-macos26-arm64-1) os=macOS; arch=ARM64; target=aarch64-apple-darwin;;
    flowmux-component-windows2022-x64-1|flowmux-component-windows2025-x64-1) os=Windows; arch=X64; target=x86_64-pc-windows-msvc;;
    flowmux-component-windows11-arm64-1) os=Windows; arch=ARM64; target=aarch64-pc-windows-msvc;;
    *) exit 85;;
  esac
  actual=$(shasum -a 256 "$zip" | awk '{print $1}')
  if [ "$actual" != "${digest#sha256:}" ]; then echo 'ZIP digest mismatch' >&2; exit 86; fi
  actual_bytes=$(wc -c < "$zip" | tr -d ' ')
  if [ "$actual_bytes" != "$bytes" ]; then exit 87; fi
  if ! unzip -Z1 "$zip" | sort > "$out/portable/zip-entries.txt"; then exit 88; fi
  if ! cmp "$out/portable/expected-entries.txt" "$out/portable/zip-entries.txt"; then exit 89; fi
  dir="$out/portable/derived/$name"
  mkdir "$dir"
  if ! unzip -q "$zip" -d "$dir"; then exit 90; fi
  tr -d '\r' < "$dir/SHA256SUMS" > "$dir/normalized-checksums.txt"
  if [ "$(wc -l < "$dir/normalized-checksums.txt" | tr -d ' ')" != 6 ]; then exit 91; fi
  if ! (cd "$dir" && shasum -a 256 -c normalized-checksums.txt) > "$dir/checksums-verified.txt"; then exit 92; fi
  lock=$lf_lock; lock_bytes=LF
  if [ "$os" = Windows ]; then lock=$crlf_lock; lock_bytes=CRLF; fi
  if ! jq -e --arg merge "$merge" --arg os "$os" --arg arch "$arch" --arg lock "$lock" '.source==$merge and .run=="41777249828" and .attempt=="1" and .os==$os and .arch==$arch and (.lock_sha256|ascii_downcase)==$lock' "$dir/identity.json" >/dev/null; then exit 93; fi
  if ! jq -e '.listed==33 and .passed==33 and .failed==0 and .ignored==0' "$dir/counts.json" >/dev/null; then exit 94; fi
  counts=$(awk '/^test result: ok\./ {scopes++;sum+=$4;if($6!=0||$8!=0||$10!=0||$12!=0)bad=1} END {if(bad||scopes!=9||sum!=33)exit 1;print sum "\t" scopes}' "$dir/tests.txt")
  if [ "$?" != 0 ]; then exit 95; fi
  listed=$(awk '/: test\r?$/ {n++} END{print n+0}' "$dir/test-list.txt")
  if [ "$listed" != 33 ]; then exit 96; fi
  if ! grep -F "host: $target" "$dir/rustc.txt" >/dev/null; then exit 97; fi
  if ! grep -F 'rustc 1.97.1' "$dir/rustc.txt" >/dev/null; then exit 98; fi
  if ! grep -E 'Finished .*release' "$dir/clippy.txt" >/dev/null; then exit 99; fi
  printf '%s\t%s\t%s\t%s\t6\t%s\t33\t%s\t%s\n' "$name" "${digest#sha256:}" "$actual" "$actual_bytes" "$merge" "$counts" "$lock_bytes" >> "$out/portable/custody.tsv"
done < <(jq -r '.artifacts[]|[.name,.digest,.size_in_bytes]|@tsv' "$out/artifacts.json")
printf 'PASS:7 originalZIP digests,42 innerchecks,33 executed tests per native row, testedmerge/head fulltree identity verified.\n' > "$out/verification.txt"
