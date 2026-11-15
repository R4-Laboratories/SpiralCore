#!/bin/bash
set -euo pipefail
umask 0022
q=/var/tmp/sc-direct-flowmux.iptmjZre
export PATH=/home/callard/.cargo/bin:/usr/bin:/bin RUSTUP_TOOLCHAIN=1.97.1 CARGO_HOME="$q/source/.mvm-test/cargo" CARGO_TARGET_DIR="$q/source/.mvm-test/target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_DEBUG=0 TMPDIR="$q/verification-tmp"
cd "$q/verification-work"
systemctl --user show sc-owned-verification-green -p ActiveState -p MainPID -p MemoryMax -p MemorySwapMax -p CPUQuotaPerSecUSec > "$q/verification-green-limit-receipt.txt"
/usr/bin/time -v cargo test --release --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --test owned_verification_workspace -- --test-threads=1 --show-output > "$q/verification-green-workspace.log" 2>&1
/usr/bin/time -v cargo test --release --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --lib endpoint::metadata_tests -- --test-threads=1 --show-output > "$q/verification-green-metadata.log" 2>&1
/usr/bin/time -v cargo clippy --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --all-targets -- -D warnings > "$q/verification-green-clippy.log" 2>&1
