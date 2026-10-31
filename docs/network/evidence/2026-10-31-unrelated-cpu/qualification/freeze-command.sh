#!/bin/bash
set -euo pipefail
cd /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
owned=/Users/casey.allard/spiralcore/local/unrelated-cpu-20261031
evidence=docs/network/evidence/2026-10-31-unrelated-cpu/qualification/frozen
crate=crates/spiralcore-transfer
[[ $(df -Pk "$owned" | awk 'NR==2 {print $4}') -gt 4194304 ]]
git diff --exit-code
source_sha=$(git rev-parse HEAD)
mkdir "$owned/frozen" "$evidence"
printf '%s\n' "$source_sha" > "$evidence/source-revision.txt"
git ls-files -z -- "$crate" crates/uor-r4-object-store/src .github/workflows/spiralcore-unrelated-cpu.yml | sort -z | xargs -0 shasum -a 256 > "$evidence/source-files.sha256"
export CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target
export CARGO_INCREMENTAL=0
cargo_args=(+1.97.1 build --release --locked --manifest-path "$crate/Cargo.toml" --target aarch64-apple-darwin --features hardware-sha256)
/Users/casey.allard/.cargo/bin/cargo "${cargo_args[@]}" --example schema_fitting --bin spiralcore-wan > "$evidence/candidate-build.txt" 2>&1
cp "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/examples/schema_fitting" "$owned/frozen/candidate"
cp "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/spiralcore-wan" "$owned/frozen/spiralcore-wan"
restore_candidate() {
  for file in src/delta.rs src/schema.rs Cargo.toml Cargo.lock; do git show "$source_sha:$crate/$file" > "$crate/$file"; done
}
trap restore_candidate EXIT
for file in src/delta.rs src/schema.rs Cargo.toml Cargo.lock; do git show "56e15db5c77fe2ecdc03476d8e09664cd4213fd1:$crate/$file" > "$crate/$file"; done
shasum -a 256 "$crate/src/delta.rs" "$crate/src/schema.rs" "$crate/Cargo.toml" "$crate/Cargo.lock" > "$evidence/baseline-overlay.sha256"
/Users/casey.allard/.cargo/bin/cargo "${cargo_args[@]}" --example schema_fitting > "$evidence/baseline-build.txt" 2>&1
cp "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/examples/schema_fitting" "$owned/frozen/baseline"
restore_candidate
trap - EXIT
"$owned/frozen/candidate" unrelated-identity > "$evidence/candidate-identity.json"
"$owned/frozen/baseline" unrelated-identity > "$evidence/baseline-identity.json"
jq -e --slurpfile baseline "$evidence/baseline-identity.json" '(.sources | del(.delta,.schema,.cargo_manifest,.lockfile)) == ($baseline[0].sources | del(.delta,.schema,.cargo_manifest,.lockfile)) and .first_block_probe_available == true and $baseline[0].first_block_probe_available == true and .hardware_sha256 == $baseline[0].hardware_sha256 and .platform == $baseline[0].platform' "$evidence/candidate-identity.json" > "$evidence/identity-parity.txt"
shasum -a 256 -c "$evidence/source-files.sha256" > "$evidence/source-after-build.txt"
git diff --exit-code
shasum -a 256 "$owned/frozen/baseline" "$owned/frozen/candidate" "$owned/frozen/spiralcore-wan" > "$evidence/binaries.sha256"
