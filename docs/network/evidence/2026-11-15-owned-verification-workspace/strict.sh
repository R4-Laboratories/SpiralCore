#!/bin/bash
set -euo pipefail
umask 0022
q=/var/tmp/sc-direct-flowmux.iptmjZre
export PATH=/home/callard/.cargo/bin:/usr/bin:/bin RUSTUP_TOOLCHAIN=1.97.1 CARGO_HOME="$q/source/.mvm-test/cargo" CARGO_TARGET_DIR="$q/source/.mvm-test/target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_DEBUG=0 TMPDIR="$q/verification-tmp"
cd "$q/verification-work"
/usr/bin/time -v cargo clippy --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --all-targets -- -D warnings > "$q/verification-strict-complete-export.log" 2>&1
