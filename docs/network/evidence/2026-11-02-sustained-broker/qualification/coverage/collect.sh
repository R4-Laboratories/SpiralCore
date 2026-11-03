#!/usr/bin/env bash
set -euo pipefail
source_root=${1:?source root}
target_root=${2:?shared owned Cargo target}
coverage_root=${3:?new owned coverage root}
export CARGO_TARGET_DIR="$target_root" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC_WORKSPACE_WRAPPER RUSTC_WRAPPER
cd "$source_root"
sha256sum -c "$coverage_root/source-files.sha256" > "$coverage_root/source-verification.txt"
/home/callard/.cargo/bin/rustc +1.97.1 -vV > "$coverage_root/rustc.txt"
/home/callard/.cargo/bin/rustup component add llvm-tools-preview --toolchain 1.97.1 > "$coverage_root/llvm-install.txt" 2>&1
llvm=$(/home/callard/.cargo/bin/rustc +1.97.1 --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin
"$llvm/llvm-cov" --version > "$coverage_root/llvm-version.txt"
/home/callard/.cargo/bin/cargo +1.97.1 clean --manifest-path crates/spiralcore-mvm-live/Cargo.toml --release -p spiralcore-mvm-live -p spiralcore-mvm-adapter > "$coverage_root/owned-clean.txt" 2>&1
export RUSTC_WRAPPER="$coverage_root/coverage-wrapper.sh"
mkdir "$coverage_root/profiles-initial"
export LLVM_PROFILE_FILE="$coverage_root/profiles-initial/%m-%p.profraw"
/home/callard/.cargo/bin/cargo +1.97.1 test --release --locked --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml --all-features --all-targets --no-run --message-format=json > "$coverage_root/adapter-build.jsonl" 2> "$coverage_root/adapter-build.stderr"
jq -s '[.[] | select(.reason=="compiler-artifact" and .executable!=null and .profile.test==true and (.manifest_path|endswith("/crates/spiralcore-mvm-adapter/Cargo.toml"))) | {target:.target.name,fresh,features,executable}]' "$coverage_root/adapter-build.jsonl" > "$coverage_root/adapter-artifacts.json"
jq -e 'length>0 and all(.[]; .fresh==false)' "$coverage_root/adapter-artifacts.json" > /dev/null
jq -r '.[].executable' "$coverage_root/adapter-artifacts.json" > "$coverage_root/adapter-objects.txt"
/home/callard/.cargo/bin/cargo +1.97.1 test --release --locked --manifest-path crates/spiralcore-mvm-live/Cargo.toml --test broker_objects --no-run --message-format=json > "$coverage_root/live-build.jsonl" 2> "$coverage_root/live-build.stderr"
jq -s '[.[] | select(.reason=="compiler-artifact" and .executable!=null and .profile.test==true and (.manifest_path|endswith("/crates/spiralcore-mvm-live/Cargo.toml"))) | {target:.target.name,fresh,features,executable}]' "$coverage_root/live-build.jsonl" > "$coverage_root/live-artifacts.json"
jq -e 'length==1 and all(.[]; .fresh==false)' "$coverage_root/live-artifacts.json" > /dev/null
jq -r '.[].executable' "$coverage_root/live-artifacts.json" > "$coverage_root/live-objects.txt"
bash "$coverage_root/run-per-binary.sh" "$coverage_root" "$llvm" > "$coverage_root/per-binary-tests.txt" 2> "$coverage_root/per-binary.stderr"
while IFS= read -r binary; do sha256sum "$binary"; done < "$coverage_root/per-binary/objects.txt" > "$coverage_root/test-binaries.sha256"
if [[ -s "$coverage_root/per-binary.stderr" ]] || find "$coverage_root/per-binary" -name export.stderr -size +0c | rg .; then
  printf 'Coverage export or execution emitted warnings/errors\n' >&2
  exit 1
fi
printf 'PASS: all emitted test binaries executed; isolated exports have no warnings\n'
