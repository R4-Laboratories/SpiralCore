#!/bin/bash
# Source/build plumbing reused from SC-OPT-003; all new analysis is Rust.
set -euo pipefail
probe_root=$(cd -- "$(dirname -- "$0")" && pwd)
cd "$probe_root"
shasum -a 256 -c snapshot-established.sha256 > /dev/null
cmp Cargo.lock /Users/casey.allard/spiralcore/local/sha-efficiency-20261029/Cargo.lock
bash template-manifest.sh > receipts/templates-current.sha256
cmp templates-established.sha256 receipts/templates-current.sha256
