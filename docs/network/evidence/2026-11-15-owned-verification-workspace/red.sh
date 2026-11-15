#!/bin/bash
set -euo pipefail
umask 0022
q=/var/tmp/sc-direct-flowmux.iptmjZre
export PATH=/home/callard/.cargo/bin:/usr/bin:/bin RUSTUP_TOOLCHAIN=1.97.1 CARGO_HOME="$q/source/.mvm-test/cargo" CARGO_TARGET_DIR="$q/source/.mvm-test/target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_DEBUG=0 TMPDIR="$q/verification-tmp"
cd "$q/verification-work"
rustc -vV > "$q/verification-red-rustc.txt"
set +e
/usr/bin/time -v cargo test --release --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --test owned_verification_workspace -- --test-threads=1 --show-output > "$q/verification-red.log" 2>&1
result=$?
set -e
printf '%s\n' "$result" > "$q/verification-red-exit.txt"
tail -45 "$q/verification-red.log"
test "$result" -eq 101
grep -F 'filesystem verification allocated a publication body:' "$q/verification-red.log"
