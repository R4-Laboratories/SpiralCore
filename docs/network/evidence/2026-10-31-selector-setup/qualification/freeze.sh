#!/bin/bash
set -euo pipefail
cd /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
owned_root=/Users/casey.allard/spiralcore/local/selector-setup-20261031
evidence=docs/network/evidence/2026-10-31-selector-setup/qualification
[[ $(df -Pk "$owned_root" | awk 'NR==2 {print $4}') -gt 4194304 ]]
git diff --exit-code
source_sha=$(git rev-parse HEAD)
baseline=4bb79f976e0939f3ef8b48c3830b07086b98f280
crate=crates/spiralcore-transfer
mkdir "$owned_root/frozen"
printf '%s\n' "$source_sha" > "$evidence/source-revision.txt"
git ls-files -z -- "$crate" crates/uor-r4-object-store/src .github/workflows/spiralcore-selector-setup.yml | sort -z | xargs -0 shasum -a 256 > "$evidence/source-files.sha256"
export CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target
export CARGO_INCREMENTAL=0
/Users/casey.allard/.cargo/bin/cargo +1.97.1 build --release --locked --manifest-path "$crate/Cargo.toml" --target aarch64-apple-darwin --features hardware-sha256 --example schema_fitting --bin spiralcore-wan > "$evidence/candidate-build.txt" 2>&1
cp "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/examples/schema_fitting" "$owned_root/frozen/candidate"
cp "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/spiralcore-wan" "$owned_root/frozen/spiralcore-wan"
"$owned_root/frozen/candidate" setup-identity > "$evidence/candidate-identity.json"
jq -e '.first_block_probe_available == true' "$evidence/candidate-identity.json" > /dev/null
restore() {
  for file in src/schema.rs src/schema_selection.rs; do
    git show "$source_sha:$crate/$file" > "$crate/$file"
  done
}
trap restore EXIT
for file in src/schema.rs src/schema_selection.rs; do
  git show "$baseline:$crate/$file" > "$crate/$file"
done
shasum -a 256 "$crate/src/schema.rs" "$crate/src/schema_selection.rs" > "$evidence/baseline-overlay.sha256"
/Users/casey.allard/.cargo/bin/cargo +1.97.1 build --release --locked --manifest-path "$crate/Cargo.toml" --target aarch64-apple-darwin --features hardware-sha256 --example schema_fitting > "$evidence/baseline-build.txt" 2>&1
cp "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/examples/schema_fitting" "$owned_root/frozen/baseline"
"$owned_root/frozen/baseline" setup-identity > "$evidence/baseline-identity.json"
jq -e '.first_block_probe_available == true' "$evidence/baseline-identity.json" > /dev/null
restore
trap - EXIT
shasum -a 256 -c "$evidence/source-files.sha256" > "$evidence/source-after-build.txt"
git diff --exit-code
shasum -a 256 "$owned_root/frozen/candidate" "$owned_root/frozen/baseline" "$owned_root/frozen/spiralcore-wan" > "$evidence/binaries.sha256"
printf '%s\n' 'Frozen candidate, baseline overlay and receiver; tracked source restored exactly.'
