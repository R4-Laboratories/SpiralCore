#!/bin/bash
set -euo pipefail
cd /Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
owned_root=/Users/casey.allard/spiralcore/local/persistent-session-20261031
evidence=docs/network/evidence/2026-10-31-persistent-session/qualification/transport-diagnostic
[[ $(df -Pk "$owned_root" | awk 'NR==2 {print $4}') -gt 4194304 ]]
git diff --exit-code
source_sha=$(git rev-parse HEAD)
crate=crates/spiralcore-transfer
mkdir "$owned_root/diagnostic-transport-frozen"
mkdir "$evidence"
printf '%s\n' "$source_sha" > "$evidence/source-revision.txt"
git ls-files -z -- "$crate" crates/uor-r4-object-store/src .github/workflows/spiralcore-persistent-session.yml | sort -z | xargs -0 shasum -a 256 > "$evidence/source-files.sha256"
export CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target
export CARGO_INCREMENTAL=0
/Users/casey.allard/.cargo/bin/cargo +1.97.1 build --release --locked --manifest-path "$crate/Cargo.toml" --target aarch64-apple-darwin --features hardware-sha256 --example schema_fitting --bin spiralcore-wan > "$evidence/build.txt" 2>&1
cp "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/examples/schema_fitting" "$owned_root/diagnostic-transport-frozen/candidate"
cp "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/spiralcore-wan" "$owned_root/diagnostic-transport-frozen/spiralcore-wan"
"$owned_root/diagnostic-transport-frozen/candidate" persistent-identity > "$evidence/candidate-identity.json"
jq -e '.first_block_probe_available == true' "$evidence/candidate-identity.json" > /dev/null
shasum -a 256 -c "$evidence/source-files.sha256" > "$evidence/source-after-build.txt"
git diff --exit-code
shasum -a 256 "$owned_root/diagnostic-transport-frozen/candidate" "$owned_root/diagnostic-transport-frozen/spiralcore-wan" > "$evidence/binaries.sha256"
printf '%s\n' 'Frozen common sender, diagnostic and receiver; source unchanged.'
