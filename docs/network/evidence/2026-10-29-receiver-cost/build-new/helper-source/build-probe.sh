#!/bin/bash
set -euo pipefail
variant=${1:?old or new required}
[[ "$variant" == old || "$variant" == new ]] || exit 2
probe_root=$(cd -- "$(dirname -- "$0")" && pwd)
repo=/Users/casey.allard/.codex/worktrees/spiralcore-object-transfer/spiralcore
build_record="$probe_root/build-$variant"
mkdir "$build_record"
cd "$repo"
[[ -z "$(git status --porcelain --untracked-files=all -- crates/spiralcore-transfer/src crates/spiralcore-transfer/Cargo.toml crates/uor-r4-object-store/src crates/uor-r4-object-store/Cargo.toml)" ]] || { printf 'Production inputs are dirty\n' >&2; exit 1; }
source_revision=$(git rev-parse HEAD)
fingerprint() {
  {
    rg --files crates/spiralcore-transfer/src crates/uor-r4-object-store/src
    printf '%s\n' crates/spiralcore-transfer/Cargo.toml crates/uor-r4-object-store/Cargo.toml
  } | LC_ALL=C sort | while IFS= read -r path; do shasum -a 256 "$path"; done
  shasum -a 256 "$probe_root/Cargo.lock"
}
fingerprint > "$build_record/production-source-files.sha256"
source_sha=$(shasum -a 256 "$build_record/production-source-files.sha256" | cut -d ' ' -f 1)
(
  cd "$probe_root"
  shasum -a 256 src/lib.rs src/main.rs Cargo.toml Cargo.lock schedule.json build-probe.sh > "$build_record/helper-files.sha256"
)
/Users/casey.allard/.cargo/bin/rustc -vV > "$build_record/compiler.txt"
native_target=$(sed -n 's/^host: //p' "$build_record/compiler.txt")
[[ -n "$native_target" && "$native_target" != */* ]] || { printf 'Invalid native target\n' >&2; exit 1; }
printf 'RUSTFLAGS=%s\nCARGO_ENCODED_RUSTFLAGS=%s\n' "${RUSTFLAGS-}" "${CARGO_ENCODED_RUSTFLAGS-}" > "$build_record/compiler-environment.txt"
SPIRAL_PROBE_LIBRARY_REVISION="$source_revision" SPIRAL_PROBE_LIBRARY_SHA256="$source_sha" \
 /Users/casey.allard/.cargo/bin/cargo build --locked --offline --release --target-dir "$probe_root/target" --target "$native_target" --manifest-path "$probe_root/Cargo.toml"
fingerprint > "$build_record/production-source-files.after.sha256"
cmp "$build_record/production-source-files.sha256" "$build_record/production-source-files.after.sha256"
(
  cd "$probe_root"
  shasum -a 256 -c "$build_record/helper-files.sha256"
)
cp "$probe_root/target/$native_target/release/spiralcore-receiver-probe" "$build_record/receiver-probe"
shasum -a 256 "$build_record/receiver-probe" > "$build_record/executable.sha256"
mkdir "$build_record/helper-source"
cp "$probe_root/src/lib.rs" "$probe_root/src/main.rs" "$probe_root/Cargo.toml" "$probe_root/Cargo.lock" "$probe_root/schedule.json" "$probe_root/build-probe.sh" "$build_record/helper-source/"
printf '{"source_revision":"%s","production_source_fingerprint_sha256":"%s","variant":"%s","profile":"release","native_target":"%s","status":"compiled_not_measured"}\n' "$source_revision" "$source_sha" "$variant" "$native_target" > "$build_record/build.json"
