#!/bin/bash
set -euo pipefail
variant=diagnostic
probe_root=$(cd -- "$(dirname -- "$0")" && pwd)
repo="$probe_root/snapshot"
candidate=f0966ebedcb8994f0985b54e9a2b961eeb8be914
cargo=/Users/casey.allard/.cargo/bin/cargo
rustc=/Users/casey.allard/.cargo/bin/rustc
cd "$probe_root"
bash verify-inputs.sh
# Reject uncontrolled flags/wrappers/profiles. CC/AR are pinned below.
while IFS='=' read -r key value; do
  case "$key" in
    RUSTFLAGS|CARGO_ENCODED_RUSTFLAGS|CARGO_BUILD_RUSTFLAGS|CARGO_TARGET_*_RUSTFLAGS|CARGO_TARGET_*_LINKER|RUSTC|RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER|RUSTC_BOOTSTRAP|CFLAGS*|CXXFLAGS*|CPPFLAGS*|LDFLAGS*|TARGET_CFLAGS|CRATE_CC_NO_DEFAULTS|CARGO_PROFILE_*|CC|CXX|AR|CC_*|CXX_*|AR_*|TARGET_CC|TARGET_CXX|TARGET_AR|HOST_CC|HOST_CXX|HOST_AR)
      [[ -z "$value" ]] || { printf 'Uncontrolled build setting: %s\n' "$key" >&2; exit 3; } ;;
  esac
done < <(env)
# Cargo configuration files can inject flags or environment. Do not silently inherit them.
scan="$probe_root"
while :; do
  [[ ! -s "$scan/.cargo/config" && ! -s "$scan/.cargo/config.toml" ]] || { printf 'Unreviewed Cargo config under %s\n' "$scan" >&2; exit 3; }
  [[ "$scan" == / ]] && break
  scan=$(dirname "$scan")
done
cargo_home=${CARGO_HOME:-/Users/casey.allard/.cargo}
[[ ! -s "$cargo_home/config" && ! -s "$cargo_home/config.toml" ]] || { printf 'Unreviewed Cargo home config\n' >&2; exit 3; }
export CC=/usr/bin/clang CXX=/usr/bin/clang++ AR=/usr/bin/ar RUSTC="$rustc"
record="$probe_root/build-$variant"
mkdir "$record"
"$rustc" -vV > "$record/compiler.txt"
native_target=$(sed -n 's/^host: //p' "$record/compiler.txt")
[[ "$native_target" == aarch64-apple-darwin ]] || { printf 'Unqualified native target\n' >&2; exit 3; }
"$CC" --version > "$record/c-compiler.txt"
shasum -a 256 "$CC" "$CXX" "$AR" > "$record/build-tools.sha256"
for tool in clang clang++ ar; do
  selected=$(/usr/bin/xcrun --find "$tool")
  printf '%s=%s\n' "$tool" "$selected" >> "$record/tool-selection.txt"
  shasum -a 256 "$selected" >> "$record/build-tools.sha256"
done
/usr/sbin/pkgutil --pkg-info=com.apple.pkg.CLTools_Executables > "$record/apple-tools-package.txt"
env | LC_ALL=C sort | awk '/^(CC|CXX|AR|RUST|CARGO|CFLAGS|CXXFLAGS|CPPFLAGS|LDFLAGS|SDKROOT|MACOSX_DEPLOYMENT_TARGET|DEVELOPER_DIR)/ && !/TOKEN|SECRET|PASSWORD|KEY/ { print }' > "$record/build-environment.txt"
fingerprint() {
  (cd "$repo"; { rg --files crates/spiralcore-transfer/src crates/uor-r4-object-store/src; printf '%s\n' crates/spiralcore-transfer/Cargo.toml crates/uor-r4-object-store/Cargo.toml; } | LC_ALL=C sort | while IFS= read -r path; do shasum -a 256 "$path"; done)
  shasum -a 256 "$probe_root/Cargo.lock"
}
fingerprint > "$record/production-source-files.sha256"
source_sha=$(shasum -a 256 "$record/production-source-files.sha256" | cut -d ' ' -f 1)
shasum -a 256 src/lib.rs src/main.rs src/bin/profile-analysis.rs Cargo.toml Cargo.lock schedule.json verify-inputs.sh build.sh run-profile.sh template-manifest.sh README.md > "$record/helper-files.sha256"
feature_args=(--features hardware-sha256)
"$cargo" tree --locked --offline --target "$native_target" -e features "${feature_args[@]}" > "$record/features.txt"
"$cargo" metadata --locked --offline --filter-platform "$native_target" --format-version 1 "${feature_args[@]}" > "$record/metadata.json"
expected='["asm","default","sha2-asm","std"]'
jq -e --argjson expected "$expected" '[.resolve.nodes[] | select(.id|contains("#sha2@0.10.9")) | .features | sort] == [$expected]' "$record/metadata.json" > "$record/feature-check.txt"
cmd=("$cargo" build --locked --offline --release --target-dir "$probe_root/target-$variant" --target "$native_target" --manifest-path "$probe_root/Cargo.toml" --bins "${feature_args[@]}")
printf '%q ' "${cmd[@]}" > "$record/command.txt"
printf '\n' >> "$record/command.txt"
SPIRAL_PROBE_LIBRARY_REVISION="$candidate" SPIRAL_PROBE_LIBRARY_SHA256="$source_sha" "${cmd[@]}"
fingerprint > "$record/production-source-files.after.sha256"
cmp "$record/production-source-files.sha256" "$record/production-source-files.after.sha256"
shasum -a 256 -c "$record/helper-files.sha256"
bash verify-inputs.sh
cp "$probe_root/target-$variant/$native_target/release/spiralcore-receiver-probe" "$record/receiver-probe"
cp "$probe_root/target-$variant/$native_target/release/profile-analysis" "$record/profile-analysis"
shasum -a 256 "$record/receiver-probe" "$record/profile-analysis" > "$record/executables.sha256"
mkdir "$record/helper-source"
cp src/lib.rs src/main.rs src/bin/profile-analysis.rs Cargo.toml Cargo.lock schedule.json verify-inputs.sh build.sh run-profile.sh template-manifest.sh README.md "$record/helper-source/"
jq -nc --arg variant "$variant" --arg revision "$candidate" --arg fingerprint "$source_sha" --arg target "$native_target" '{schema:"spiralcore.cache-profile-build.v1",variant:$variant,source_revision:$revision,diagnostic_source_fingerprint_sha256:$fingerprint,native_target:$target,feature:"spiralcore-transfer/hardware-sha256",status:"compiled_not_measured"}' > "$record/build.json"
