#!/bin/bash
set -euo pipefail
umask 0022
q=/var/tmp/sc-direct-flowmux.iptmjZre
export PATH=/home/callard/.cargo/bin:/usr/bin:/bin RUSTUP_TOOLCHAIN=1.97.1 CARGO_HOME="$q/source/.mvm-test/cargo" CARGO_TARGET_DIR="$q/source/.mvm-test/target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_RELEASE_DEBUG=0 TMPDIR="$q/verification-tmp"
cd "$q/verification-work"
systemctl --user show sc-owned-verification-final -p ActiveState -p MainPID -p MemoryMax -p MemorySwapMax -p CPUQuotaPerSecUSec > "$q/verification-final-limit-receipt.txt"
rustc -vV > "$q/verification-final-toolchain.txt"
sha256sum -c ../final-inputs.sha256 > "$q/verification-final-inputs-before.log"
cargo test --release --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --test owned_verification_workspace --no-run --message-format=json > "$q/verification-final-build.jsonl" 2> "$q/verification-final-build.stderr"
mapfile -t artifacts < <(jq -r 'select(.reason == "compiler-artifact" and .profile.test == true and .target.name == "owned_verification_workspace" and .executable != null) | .executable' "$q/verification-final-build.jsonl")
test "${#artifacts[@]}" -eq 1
executable="${artifacts[0]}"
sha256sum "$executable" > "$q/verification-final-executable-before.sha256"
"$executable" --list --format terse > "$q/verification-final-list.log"
test "$(rg -c ': test$' "$q/verification-final-list.log")" -eq 1
/usr/bin/time -v "$executable" --exact maximal_store_retirement_does_not_allocate_a_publication_body --show-output --test-threads=1 --color never > "$q/verification-final-workspace.log" 2>&1
sha256sum -c "$q/verification-final-executable-before.sha256" > "$q/verification-final-executable-after.log"
/usr/bin/time -v cargo test --release --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --all-targets -- --show-output > "$q/verification-final-transfer.log" 2>&1
/usr/bin/time -v cargo test --release --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --doc > "$q/verification-final-transfer-doc.log" 2>&1
/usr/bin/time -v cargo clippy --release --locked --offline --manifest-path crates/spiralcore-transfer/Cargo.toml --all-targets -- -D warnings > "$q/verification-final-clippy.log" 2>&1
/usr/bin/time -v cargo test --release --locked --offline --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml --all-targets -- --show-output > "$q/verification-final-flowmux.log" 2>&1
sha256sum -c ../final-inputs.sha256 > "$q/verification-final-inputs-after.log"
printf 'FINAL_QUALIFICATION_COMPLETE\n'
