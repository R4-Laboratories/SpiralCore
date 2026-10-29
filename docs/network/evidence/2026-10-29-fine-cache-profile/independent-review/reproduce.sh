#!/bin/bash
# Post-run evidence checks only. This never invokes the measured endpoint or probe.
set -euo pipefail
root=/Users/casey.allard/spiralcore/local/broad-wan-20261029/cache-profile
cd "$root"
rustc=/Users/casey.allard/.cargo/bin/rustc
deps=target-diagnostic/aarch64-apple-darwin/release/deps
args=(--edition 2021 --target aarch64-apple-darwin independent-review/main.rs -L "dependency=$deps" --extern "serde_json=$deps/libserde_json-1526d65a4d76f4d2.rlib" --extern "sha2=$deps/libsha2-edcd0ba55ef603fc.rlib")
"$rustc" -vV > independent-review/compiler.txt
"$rustc" "${args[@]}" --test -o independent-review/review-tests
independent-review/review-tests
"$rustc" "${args[@]}" -o independent-review/review-final
independent-review/review-final "$root" "${1:?absolute new output JSON path required}"
shasum -a 256 independent-review/main.rs independent-review/reproduce.sh independent-review/review-final independent-review/review-tests independent-review/compiler.txt "$deps/libserde_json-1526d65a4d76f4d2.rlib" "$deps/libsha2-edcd0ba55ef603fc.rlib" > independent-review/method-files.sha256
