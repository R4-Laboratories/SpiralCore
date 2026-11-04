#!/bin/bash
set -u -o pipefail
out=${1:-/tmp/spiralcore-pr72-gate-20261104/restored-ci}
head=1e69a99d27691d1db9921568d834a98814defc81
merge=86d598fbd218ecdbf2d0d03b56e56ca5f67eb689
tree=94426f89602dc2714842198f9bc4dd7fa4fa8d8b
fail() { printf 'FAIL:%s\n' "$*" >&2; exit 1; }
if ! jq -e --arg head "$head" '.id==41784774137 and .head_sha==$head and .status=="completed" and .conclusion=="success" and .run_attempt==1 and .event=="pull_request"' "$out/run.json" >/dev/null; then fail run; fi
if ! jq -e --arg head "$head" --arg tree "$tree" '.sha==$head and .tree.sha==$tree' "$out/head-commit.json" >/dev/null; then fail head; fi
if ! jq -e --arg merge "$merge" --arg head "$head" --arg tree "$tree" '.sha==$merge and .tree.sha==$tree and ([.parents[].sha]==["e66caf5a30fcbcba525932bfa0b4a64d9112059b",$head])' "$out/tested-merge.json" >/dev/null; then fail tested-merge; fi
if ! jq -e --arg head "$head" '.jobs|length==8 and all(.[];.head_sha==$head and .status=="completed" and .conclusion=="success")' "$out/jobs.json" >/dev/null; then fail jobs; fi
if ! jq -e --arg head "$head" '.artifacts|length==7 and all(.[];.expired==false and (.digest|startswith("sha256:")) and .workflow_run.id==41784774137 and .workflow_run.head_sha==$head)' "$out/artifacts.json" >/dev/null; then fail artifacts; fi
if ! printf '%s\n' SHA256SUMS clippy.txt counts.json identity.json rustc.txt test-list.txt tests.txt | sort > "$out/expected-entries.txt"; then fail expected-entries; fi
if ! lf_lock=$(shasum -a 256 /tmp/spiralcore-pr72-gate-20261104/source-1e69/crates/spiralcore-mvm-flowmux/Cargo.lock | awk '{print $1}'); then fail source-lock; fi
if ! crlf_lock=$(awk '{printf "%s\r\n",$0}' /tmp/spiralcore-pr72-gate-20261104/source-1e69/crates/spiralcore-mvm-flowmux/Cargo.lock | shasum -a 256 | awk '{print $1}'); then fail windows-lock; fi
if ! jq -r '.artifacts[]|[.name,.digest,.size_in_bytes]|@tsv' "$out/artifacts.json" > "$out/artifact-rows.tsv"; then fail artifact-rows; fi
printf 'artifact\tapi_sha256\tactual_sha256\tzip_bytes\tinner_checks\ttested_source\tlisted\tpassed\tnonzero_test_scopes\tlock_bytes\n' > "$out/custody.tsv" || fail custody-header
while IFS=$'\t' read -r name digest bytes; do
    case "$name" in
        flowmux-component-ubuntu24-x64-1) os=Linux; arch=X64; target=x86_64-unknown-linux-gnu;;
        flowmux-component-ubuntu24-arm64-1) os=Linux; arch=ARM64; target=aarch64-unknown-linux-gnu;;
        flowmux-component-macos26-intel-1) os=macOS; arch=X64; target=x86_64-apple-darwin;;
        flowmux-component-macos26-arm64-1) os=macOS; arch=ARM64; target=aarch64-apple-darwin;;
        flowmux-component-windows2022-x64-1|flowmux-component-windows2025-x64-1) os=Windows; arch=X64; target=x86_64-pc-windows-msvc;;
        flowmux-component-windows11-arm64-1) os=Windows; arch=ARM64; target=aarch64-pc-windows-msvc;;
        *) fail unknown-row;;
    esac
    zip="$out/archives/$name.zip"
    if ! actual=$(shasum -a 256 "$zip" | awk '{print $1}'); then fail zip-hash; fi
    if [ "$actual" != "${digest#sha256:}" ]; then fail zip-digest; fi
    if ! actual_bytes=$(wc -c < "$zip" | tr -d ' '); then fail zip-size; fi
    if [ "$actual_bytes" != "$bytes" ]; then fail zip-size-mismatch; fi
    if ! unzip -Z1 "$zip" | sort > "$out/zip-entries.txt"; then fail zip-entries; fi
    if ! cmp "$out/expected-entries.txt" "$out/zip-entries.txt"; then fail unexpected-archive-content; fi
    dir="$out/derived/$name"
    if ! mkdir "$dir"; then fail derived-already-exists; fi
    if ! unzip -q "$zip" -d "$dir"; then fail unzip; fi
    if ! tr -d '\r' < "$dir/SHA256SUMS" > "$dir/normalized-checksums.txt"; then fail normalize-checksums; fi
    if [ "$(wc -l < "$dir/normalized-checksums.txt" | tr -d ' ')" != 6 ]; then fail inner-count; fi
    if ! (cd "$dir" && shasum -a 256 -c normalized-checksums.txt) > "$dir/checksums-verified.txt"; then fail inner-digest; fi
    lock=$lf_lock; lock_bytes=LF
    if [ "$os" = Windows ]; then lock=$crlf_lock; lock_bytes=CRLF; fi
    if ! jq -e --arg merge "$merge" --arg os "$os" --arg arch "$arch" --arg lock "$lock" '.source==$merge and .run=="41784774137" and .attempt=="1" and .os==$os and .arch==$arch and (.lock_sha256|ascii_downcase)==$lock and .scope=="native process; paired TCP/TLS/shared-object component; no MVM backend or WAN"' "$dir/identity.json" >/dev/null; then fail identity; fi
    if ! jq -e '.listed==34 and .passed==34 and .failed==0 and .ignored==0' "$dir/counts.json" >/dev/null; then fail counts; fi
    if ! counts=$(awk '/^test result: ok\./ {scopes++;sum+=$4;if($4==0||$6!=0||$8!=0||$10!=0||$12!=0)bad=1} END {if(bad||scopes!=9||sum!=34)exit 1;print sum "\t" scopes}' "$dir/tests.txt"); then fail test-results; fi
    if ! listed=$(awk '/: test\r?$/ {n++} END{print n+0}' "$dir/test-list.txt"); then fail test-list; fi
    if [ "$listed" != 34 ]; then fail listed-count; fi
    if ! grep -F "host: $target" "$dir/rustc.txt" >/dev/null; then fail rust-target; fi
    if ! grep -F 'rustc 1.97.1' "$dir/rustc.txt" >/dev/null; then fail rust-version; fi
    if ! grep -E 'Finished .*release' "$dir/clippy.txt" >/dev/null; then fail clippy; fi
    printf '%s\t%s\t%s\t%s\t6\t%s\t34\t%s\t%s\n' "$name" "${digest#sha256:}" "$actual" "$actual_bytes" "$merge" "$counts" "$lock_bytes" >> "$out/custody.tsv" || fail custody-row
done < "$out/artifact-rows.tsv"
printf 'PASS:7 original ZIP API digests and sizes;42 inner SHA256 checks;34 listed/passed per native role across9 nonzero scopes;8 jobs successful;tested-merge parents and fulltree match exact reviewed head.\n'
