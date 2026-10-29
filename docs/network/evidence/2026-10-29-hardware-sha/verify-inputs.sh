#!/bin/bash
set -euo pipefail
probe_root=$(cd -- "$(dirname -- "$0")" && pwd)
repo=/Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
candidate=80449939bd92348593cc88555cf7fde0847ceffd
cd "$probe_root"
[[ "$(shasum -a 256 src/lib.rs | cut -d ' ' -f 1)" == 4b63af43eeb6ba11d3cd28015f49650b936cf85ef6b6a0de0880bfb8d0dafbbb ]]
[[ "$(shasum -a 256 src/main.rs | cut -d ' ' -f 1)" == 33da423338e9b17248f085368d9a282698862d251e177382c4f4ef95c1a256c2 ]]
[[ "$(shasum -a 256 Cargo.toml | cut -d ' ' -f 1)" == 9bed2eb57db6d46e65c6e656bc13602f63a596f97da126a2a848bba33b00f5cd ]]
[[ "$(shasum -a 256 baseline-probe.toml | cut -d ' ' -f 1)" == 229b68e61f5eabd5e35676ab9e2bc59af13bdfc5456d75ea6f6875958b464c1f ]]
cmp baseline-probe.toml <(sed '/^\[features\]$/,+2d' Cargo.toml)
[[ "$(shasum -a 256 baseline-probe.lock | cut -d ' ' -f 1)" == 059f60f06070b6629cdfe36812b57db7198833af0a4b0c49332d310e0af07bba ]]
[[ "$(shasum -a 256 schedule.json | cut -d ' ' -f 1)" == 8f099a90b7659f72d24446f0105a06e53f5f78bda00561e671cf193912b04832 ]]
# The only permitted lock delta is this exact optional package and its sha2 edge.
awk 'BEGIN { RS=""; ORS="\n\n" } /name = "sha2-asm"/ {
  if ($0 != "[[package]]\nname = \"sha2-asm\"\nversion = \"0.6.4\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"b845214d6175804686b2bd482bcffe96651bb2d1200742b712003504a2dac1ab\"\ndependencies = [\n \"cc\",\n]") exit 3;
  found++; next
} { if (/name = "sha2"/) { edge += gsub(/\n "sha2-asm",/, ""); } print } END { if (found != 1 || edge != 1) exit 4 }' Cargo.lock | cmp - <(awk 'BEGIN { RS=""; ORS="\n\n" } { print }' baseline-probe.lock)
cd "$repo"
git diff --quiet "$candidate" -- crates/spiralcore-transfer/src crates/uor-r4-object-store/src crates/spiralcore-transfer/Cargo.toml crates/uor-r4-object-store/Cargo.toml crates/spiralcore-transfer/Cargo.lock
[[ -z "$(git status --porcelain --untracked-files=all -- crates/spiralcore-transfer/src crates/uor-r4-object-store/src crates/spiralcore-transfer/Cargo.toml crates/uor-r4-object-store/Cargo.toml crates/spiralcore-transfer/Cargo.lock)" ]]
# The rejected quota change and all other Rust changes must remain absent.
git diff --quiet 74ee59df7b7a55476fa573b17f5d947fc28fb030 -- crates/spiralcore-transfer/src crates/uor-r4-object-store/src
