#!/usr/bin/env bash
# Metadata/artifact verification only; never builds, tests, pushes or checks out.
set -euo pipefail
root=/tmp/spiralcore-firecracker-host-lease-20261103/release
repo=/Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
head=731b5071df98b1ba6ee2f56489796158b7692cc8
base=e71f40800524c3effa130afb88a6c2d730be2cde
merge=556670cd9b98536d99648e6a3a4438a56bcf3ff3
tree=95881a0c3f922b208c75823fa67e4b320b19b28b
fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
cd "$repo" || fail 'repo directory'
for run in owner linux; do
  if ! jq -e --arg h "$head" --arg b "$base" '.head_sha==$h and .status=="completed" and .conclusion=="success" and .event=="pull_request" and .run_attempt==1 and .pull_requests[0].head.sha==$h and .pull_requests[0].base.sha==$b' "$root/$run-run.json"; then fail "$run run identity/status"; fi
done
if ! jq -e '.jobs|length==4 and all(.[];.conclusion=="success" and .status=="completed")' "$root/owner-jobs.json"; then fail 'four owner jobs'; fi
if ! jq -e '.jobs|length==1 and all(.[];.conclusion=="success" and .status=="completed")' "$root/linux-jobs.json"; then fail 'Linux job'; fi
if ! jq -e '.total_count==4 and (.artifacts|length)==4 and all(.artifacts[];.expired==false)' "$root/owner-artifacts.json"; then fail 'four live original artifacts'; fi
if ! jq -e --arg h "$head" --arg t "$tree" '.sha==$h and .tree.sha==$t' "$root/reviewed-head.json"; then fail 'reviewed API tree'; fi
if ! jq -e --arg m "$merge" --arg h "$head" --arg b "$base" --arg t "$tree" '.sha==$m and .tree.sha==$t and [.parents[].sha]==[$b,$h]' "$root/tested-merge.json"; then fail 'tested merge parents/tree'; fi
local_tree=$(git rev-parse "$head^{tree}") || fail 'frozen local tree'
if [[ "$local_tree" != "$tree" ]]; then fail 'local/API tree mismatch'; fi
: > "$root/expected-source.sha256"
for source in crates/spiralcore-mvm-live/src/installed_owner.rs crates/spiralcore-mvm-live/src/installed_owner/tests.rs crates/spiralcore-mvm-live/tests/installed-owner-harness/Cargo.toml crates/spiralcore-mvm-live/tests/installed-owner-harness/Cargo.lock crates/spiralcore-mvm-live/tests/installed-owner-harness/tests/lifecycle.rs; do
  digest=$(git cat-file blob "$head:$source" | shasum -a 256 | cut -d' ' -f1) || fail "frozen blob $source"
  printf '%s  %s\n' "$digest" "$source" >> "$root/expected-source.sha256"
done
printf 'source-commit.txt\nsource.sha256\nrustc.txt\nplatform.txt\nunit.txt\nlifecycle.txt\nclippy.txt\n' | sort > "$root/expected-members.txt"
rows=$(jq -r '.artifacts[]|[.id,.name,.digest]|@tsv' "$root/owner-artifacts.json") || fail 'artifact row parsing'
printf 'artifact\ttarget\tapi_digest\tarchive_bytes\tunit\tlifecycle\tcompiled_pins\tmember_hashes\n' > "$root/custody.tsv"
: > "$root/archive-members.sha256"
while IFS=$'\t' read -r id name digest; do
  if ! [[ "$id" =~ ^[0-9]+$ ]]; then fail 'artifact ID'; fi
  case "$name" in
    installed-owner-x86_64-unknown-linux-gnu-1) target=x86_64-unknown-linux-gnu; life=11; os=Linux; arch=x86_64;;
    installed-owner-aarch64-unknown-linux-gnu-1) target=aarch64-unknown-linux-gnu; life=11; os=Linux; arch=aarch64;;
    installed-owner-x86_64-apple-darwin-1) target=x86_64-apple-darwin; life=12; os=Darwin; arch=x86_64;;
    installed-owner-aarch64-apple-darwin-1) target=aarch64-apple-darwin; life=12; os=Darwin; arch=arm64;;
    *) fail "unexpected artifact $name";;
  esac
  archive="$root/original-archives/$name.zip"
  actual=$(shasum -a 256 "$archive" | cut -d' ' -f1) || fail "hash $name"
  if [[ "sha256:$actual" != "$digest" ]]; then fail "API digest $name"; fi
  unzip -Z1 "$archive" | sort > "$root/actual-members.txt" || fail "member listing $name"
  if ! cmp "$root/expected-members.txt" "$root/actual-members.txt"; then fail "unexpected/missing ZIP member $name"; fi
  mkdir -p "$root/rows/$name" || fail 'owned row directory'
  while IFS= read -r member; do
    unzip -p "$archive" "$member" > "$root/rows/$name/$member" || fail "member read $name/$member"
    member_hash=$(shasum -a 256 "$root/rows/$name/$member" | cut -d' ' -f1) || fail 'member hash'
    printf '%s  rows/%s/%s\n' "$member_hash" "$name" "$member" >> "$root/archive-members.sha256"
  done < "$root/expected-members.txt"
  actual_commit=$(cat "$root/rows/$name/source-commit.txt") || fail 'artifact source commit'
  if [[ "$actual_commit" != "$merge" ]]; then fail "tested checkout $name"; fi
  if ! cmp "$root/expected-source.sha256" "$root/rows/$name/source.sha256"; then fail "frozen compiled pins $name"; fi
  if ! grep -Fx "host: $target" "$root/rows/$name/rustc.txt"; then fail "Rust host $name"; fi
  if ! grep -Fx 'release: 1.97.1' "$root/rows/$name/rustc.txt"; then fail "Rust version $name"; fi
  if ! grep -E "^$os .* $arch( |$)" "$root/rows/$name/platform.txt"; then fail "actual native OS/arch $name"; fi
  if ! grep -E '^test result: ok\. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9.]+s$' "$root/rows/$name/unit.txt"; then fail "unit counts $name"; fi
  if ! grep -E "^test result: ok\. $life passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9.]+s$" "$root/rows/$name/lifecycle.txt"; then fail "lifecycle counts $name"; fi
  if ! grep -q 'Finished `dev` profile' "$root/rows/$name/clippy.txt"; then fail "Clippy completion $name"; fi
  bytes=$(wc -c < "$archive" | tr -d ' ') || fail 'archive size'
  printf '%s\t%s\t%s\t%s\t4\t%s\t5\t7\n' "$name" "$target" "$digest" "$bytes" "$life" >> "$root/custody.tsv"
done <<< "$rows"
printf 'PASS: frozen731 head; tested556670 merge and equal95881 tree; five successful jobs; four original ZIP/API digests;20 frozen compiled pins;28 member hashes;62 native entries including4 helpers; strict focused Clippy.\n'
