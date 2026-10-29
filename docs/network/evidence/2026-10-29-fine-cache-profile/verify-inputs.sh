#!/bin/bash
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")" && pwd)
cd "$root"
shasum -a 256 -c snapshot-established.sha256 > /dev/null
cmp Cargo.lock /Users/casey.allard/spiralcore/local/cache-profile-20261029/Cargo.lock
