#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
merge=1da75b1aa4a6aacda5f47706caeeeaa213f0e99a
head=960e73c59d7eab62f15a16667f95e594e1cabc8b
{
  git show -s --format='tested_merge %H%nmerge_parents %P%ntested_tree %T' "$merge"
  git show -s --format='reviewed_head %H%nhead_tree %T' "$head"
  git diff --exit-code "$merge" "$head"
} > source-tree.txt
[[ "$(git rev-parse "$merge^{tree}")" == "$(git rev-parse "$head^{tree}")" ]]
git show "$merge:docs/platforms/mvm-mailbox-portable.json" > tested-manifest.json
printf 'path\tlf_sha256\tcrlf_sha256\n' > source-projections.tsv
for path in docs/platforms/mvm-mailbox-portable.json crates/spiralcore-mvm-adapter/Cargo.lock; do
  lf=$(git show "$merge:$path" | shasum -a 256 | awk '{print $1}')
  crlf=$(git show "$merge:$path" | awk '{printf "%s\r\n", $0}' | shasum -a 256 | awk '{print $1}')
  printf '%s\t%s\t%s\n' "$path" "$lf" "$crlf" >> source-projections.tsv
done
