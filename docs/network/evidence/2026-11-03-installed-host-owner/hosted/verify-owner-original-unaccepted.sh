#!/usr/bin/env bash
# Run from the qualified SpiralCore checkout; retain original compact ZIPs.
set -euo pipefail
root=docs/network/evidence/2026-11-03-installed-host-owner/hosted/41697837824
expected=64d93fed72fdac6784d99e6e1a258052bba605f6
jq -e --arg expected "$expected" '.head_sha==$expected and .status=="completed" and .conclusion=="success"' "$root/run.json"
jq -e '.jobs|length==4 and all(.[];.status=="completed" and .conclusion=="success")' "$root/jobs.json"
jq -e '.total_count==4 and (.artifacts|length)==4 and all(.artifacts[];.expired==false)' "$root/artifacts.json"
shasum -a 256 crates/spiralcore-mvm-live/src/installed_owner.rs \
  crates/spiralcore-mvm-live/src/installed_owner/tests.rs \
  crates/spiralcore-mvm-live/tests/installed-owner-harness/Cargo.toml \
  crates/spiralcore-mvm-live/tests/installed-owner-harness/Cargo.lock \
  crates/spiralcore-mvm-live/tests/installed-owner-harness/tests/lifecycle.rs > "$root/expected-source.sha256"
while IFS=$'\t' read -r id name digest; do
  [[ "$id" =~ ^[0-9]+$ ]]
  case "$name" in
    installed-owner-x86_64-unknown-linux-gnu-1|installed-owner-aarch64-unknown-linux-gnu-1) count=9 ;;
    installed-owner-x86_64-apple-darwin-1|installed-owner-aarch64-apple-darwin-1) count=10 ;;
    *) exit 1 ;;
  esac
  archive="$root/$name.zip"
  if [[ ! -e "$archive" ]]; then
    gh api "repos/R4-Laboratories/SpiralCore/actions/artifacts/$id/zip" > "$archive"
  fi
  [[ "sha256:$(shasum -a 256 "$archive" | cut -d' ' -f1)" == "$digest" ]]
  if unzip -Z1 "$archive" | grep -E '(^/|(^|/)\.\.(/|$))'; then exit 1; fi
  mkdir -p "$root/$name"
  unzip -oq "$archive" -d "$root/$name"
  [[ "$(cat "$root/$name/source-commit.txt")" == "$expected" ]]
  cmp "$root/expected-source.sha256" "$root/$name/source.sha256"
  grep -E '^test result: ok\. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9.]+s$' "$root/$name/unit.txt"
  grep -E "^test result: ok\. $count passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9.]+s$" "$root/$name/lifecycle.txt"
  grep -q 'Finished `dev` profile' "$root/$name/clippy.txt"
done < <(jq -r '.artifacts[]|[.id,.name,.digest]|@tsv' "$root/artifacts.json")
printf 'PASS: four original ZIP/API digests, four exact commits, twenty compiled source pins, 54 native test entries including four helpers, and strict Clippy.\n'
