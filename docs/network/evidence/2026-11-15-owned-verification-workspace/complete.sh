#!/bin/bash
set -euo pipefail
umask 0022
q=/var/tmp/sc-direct-flowmux.iptmjZre
export PATH=/home/callard/.cargo/bin:/usr/bin:/bin RUSTUP_TOOLCHAIN=1.97.1 CARGO_HOME="$q/source/.mvm-test/cargo" CARGO_TARGET_DIR="$q/source/.mvm-test/target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_DEBUG=0 TMPDIR="$q/verification-tmp"
cd "$q/verification-work"
systemctl --user show sc-owned-verification-complete -p ActiveState -p MainPID -p MemoryMax -p MemorySwapMax -p CPUQuotaPerSecUSec > "$q/verification-complete-limit-receipt.txt"
sha256sum -c ../complete-inputs.sha256 > "$q/verification-complete-inputs-before.log"
/usr/bin/time -v cargo test --release --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --all-targets -- --show-output > "$q/verification-complete-transfer.log" 2>&1
/usr/bin/time -v cargo test --release --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --doc > "$q/verification-complete-transfer-doc.log" 2>&1
/usr/bin/time -v cargo clippy --release --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --all-targets -- -D warnings > "$q/verification-complete-clippy.log" 2>&1
/usr/bin/time -v cargo test --release --locked --offline --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --all-targets -- --show-output > "$q/verification-complete-flowmux.log" 2>&1
sha256sum -c ../complete-inputs.sha256 > "$q/verification-complete-inputs-after.log"
sha256sum -c "$q/verification-final-executable-before.sha256" > "$q/verification-complete-executable-after.log"
printf 'COMPLETE_QUALIFICATION_COMPLETE\n'
