#!/bin/bash
set -u
root=/var/tmp/spiralcore-flowmux-reader-20261104
receipt="$root/validation/api-red"
if test -e "$root/source"; then exit 2; fi
mkdir -p "$root/source" "$receipt" || exit 1
cp -a /var/tmp/spiralcore-installed-factory-20261104/dependency-ca5/crates "$root/source/" || exit 1
chmod -R u+w "$root/source" || exit 1
cp /var/tmp/spiralcore-installed-factory-20261104/reader-completion-api-red.rs "$root/source/crates/spiralcore-mvm-flowmux/tests/reader_completion.rs" || exit 1
cd "$root/source" || exit 1
find . -type f -print0 | sort -z | xargs -0 sha256sum > "$receipt/source-before.sha256" || exit 1
export CARGO_HOME=/home/callard/.cargo
export CARGO_TARGET_DIR=/var/tmp/spiralcore-flowmux-20261103/target
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
unset RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER RUSTFLAGS CARGO_ENCODED_RUSTFLAGS LLVM_PROFILE_FILE
manifest="$root/source/crates/spiralcore-mvm-flowmux/Cargo.toml"
/home/callard/.cargo/bin/cargo +1.97.1 check --manifest-path "$manifest" --offline --locked --release --test reader_completion > "$receipt/check.txt" 2>&1
result=$?
printf '%s\n' "$result" > "$receipt/check.exit"
cat "$receipt/check.txt"
sha256sum -c --quiet "$receipt/source-before.sha256" > "$receipt/source-after.txt" || exit 1
exit "$result"
