#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
merge=7c10e0f063d1b513881b3ee3d4c9975504d90e11
head=b27fbba9bdc636575266d6da04ec8f1ad5c7c2ea
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
