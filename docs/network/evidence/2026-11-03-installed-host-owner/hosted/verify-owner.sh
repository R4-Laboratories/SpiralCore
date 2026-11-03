#!/usr/bin/env bash
# Run from the qualified SpiralCore checkout; retain original compact ZIPs.
set -euo pipefail
root=docs/network/evidence/2026-11-03-installed-host-owner/hosted/41697837824
expected=64d93fed72fdac6784d99e6e1a258052bba605f6
tested_merge=dd8362d0f7265849653f58c53c6d5bfd185f7a46
expected_base=7c220cc841fa441df176a58cb24aeecd4d4d5fad
expected_tree=3d478a93c3366997913bc0d44f2c5c8e3440d98b
fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
# Bash 3.2 does not apply errexit to a failed [[...]] command. Evidence
# comparisons therefore use explicit failure branches.
if ! jq -e --arg expected "$expected" --arg base "$expected_base" '.head_sha==$expected and .event=="pull_request" and .status=="completed" and .conclusion=="success" and (.pull_requests|length)==1 and .pull_requests[0].head.sha==$expected and .pull_requests[0].base.sha==$base' "$root/run.json"; then fail 'run/head/base identity'; fi
if ! jq -e '.jobs|length==4 and all(.[];.status=="completed" and .conclusion=="success")' "$root/jobs.json"; then fail 'four successful jobs'; fi
if ! jq -e '.total_count==4 and (.artifacts|length)==4 and all(.artifacts[];.expired==false)' "$root/artifacts.json"; then fail 'four live artifacts'; fi
if ! jq -e --arg head "$expected" --arg tree "$expected_tree" '.sha==$head and .tree.sha==$tree' "${root%/*}/reviewed-head-commit.json"; then fail 'reviewed-head API tree'; fi
if ! jq -e --arg merge "$tested_merge" --arg head "$expected" --arg base "$expected_base" --arg tree "$expected_tree" '.sha==$merge and .tree.sha==$tree and [.parents[].sha]==[$base,$head]' "${root%/*}/tested-merge-commit.json"; then fail 'tested-merge API parents/tree'; fi
head_tree=$(git rev-parse "$expected^{tree}") || fail 'local reviewed-head tree lookup'
if [[ "$head_tree" != "$expected_tree" ]]; then fail 'local reviewed-head tree differs from tested merge'; fi
shasum -a 256 crates/spiralcore-mvm-live/src/installed_owner.rs \
  crates/spiralcore-mvm-live/src/installed_owner/tests.rs \
  crates/spiralcore-mvm-live/tests/installed-owner-harness/Cargo.toml \
  crates/spiralcore-mvm-live/tests/installed-owner-harness/Cargo.lock \
  crates/spiralcore-mvm-live/tests/installed-owner-harness/tests/lifecycle.rs > "$root/expected-source.sha256" || fail 'local compiled-source hashes'
artifact_rows=$(jq -r '.artifacts[]|[.id,.name,.digest]|@tsv' "$root/artifacts.json") || fail 'artifact listing'
while IFS=$'\t' read -r id name digest; do
  if ! [[ "$id" =~ ^[0-9]+$ ]]; then fail 'invalid artifact ID'; fi
  case "$name" in
    installed-owner-x86_64-unknown-linux-gnu-1|installed-owner-aarch64-unknown-linux-gnu-1) count=9 ;;
    installed-owner-x86_64-apple-darwin-1|installed-owner-aarch64-apple-darwin-1) count=10 ;;
    *) fail "unexpected artifact name: $name" ;;
  esac
  archive="$root/$name.zip"
  if [[ ! -e "$archive" ]]; then
    gh api "repos/R4-Laboratories/SpiralCore/actions/artifacts/$id/zip" > "$archive" || fail "artifact download: $name"
  fi
  archive_digest=$(shasum -a 256 "$archive" | cut -d' ' -f1) || fail "archive digest: $name"
  if [[ "sha256:$archive_digest" != "$digest" ]]; then fail "archive/API digest mismatch: $name"; fi
  entries=$(unzip -Z1 "$archive") || fail "archive listing: $name"
  if printf '%s\n' "$entries" | grep -E '(^/|(^|/)\.\.(/|$))'; then fail "unsafe archive path: $name"; fi
  mkdir -p "$root/$name" || fail "artifact directory: $name"
  unzip -oq "$archive" -d "$root/$name" || fail "artifact extraction: $name"
  actual_commit=$(cat "$root/$name/source-commit.txt") || fail "artifact commit read: $name"
  if [[ "$actual_commit" != "$tested_merge" ]]; then fail "artifact commit differs from tested merge: $name"; fi
  if ! cmp "$root/expected-source.sha256" "$root/$name/source.sha256"; then fail "compiled-source pins: $name"; fi
  if ! grep -E '^test result: ok\. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9.]+s$' "$root/$name/unit.txt"; then fail "unit count: $name"; fi
  if ! grep -E "^test result: ok\. $count passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9.]+s$" "$root/$name/lifecycle.txt"; then fail "lifecycle count: $name"; fi
  if ! grep -q 'Finished `dev` profile' "$root/$name/clippy.txt"; then fail "strict Clippy completion: $name"; fi
  printf 'VERIFIED artifact=%s tested_merge=%s reviewed_head=%s tree=%s\n' "$name" "$actual_commit" "$expected" "$expected_tree"
done <<< "$artifact_rows"
printf 'PASS: four original ZIP/API digests, four tested-merge commits with reviewed-head tree equality, twenty compiled source pins, 54 native test entries including four helpers, and strict Clippy.\n'
