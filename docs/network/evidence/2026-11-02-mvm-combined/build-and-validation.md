# SC-MVM-004 build and validation

The campaign uses the existing isolated checkout and shared Cargo target at
`/Users/casey.allard/spiralcore/target`. No additional VM image or build-cache tree
was created. All fourteen patched MVM files matched the retained
`crates/spiralcore-mvm-live/mvm-patch/patched-files.sha256`; the isolated MVM
checkout remains at `3104a12c5b071a6851323fa8115d2afaf3f0bb8b` with only those
fourteen tracked modifications. The retained kernel matched SHA-256
`0f97f109154a173352c07db223ecff48d44b38514b500f39cd9897701dbac0b6`.

Commands run from the SpiralCore checkout, using the existing prepared patch:

```sh
export CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target
export CARGO_BUILD_JOBS=2
export CARGO_INCREMENTAL=0
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --config /Users/casey.allard/spiralcore/local/mvm-vsock-20261031/patch.toml --all-targets
cargo +1.97.1 clippy --locked --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --config /Users/casey.allard/spiralcore/local/mvm-vsock-20261031/patch.toml --all-targets -- -D warnings
cargo +1.97.1 build --release --locked --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --config /Users/casey.allard/spiralcore/local/mvm-vsock-20261031/patch.toml
```

The guest build additionally sources the retained
`local/mvm-vsock-20261031/tools/env.sh`, supplying Zig 0.13.0, cargo-zigbuild
0.23.0 and the retained Rust 1.97.1 Linux-musl sysroot, then runs:

```sh
cargo +1.97.1 zigbuild --release --locked --target aarch64-unknown-linux-musl \
  --manifest-path crates/spiralcore-mvm-live/Cargo.toml \
  --config /Users/casey.allard/spiralcore/local/mvm-vsock-20261031/patch.toml
```

Only the copied campaign host executable is ad-hoc signed using the repository's
`hvf-entitlements.plist`; `codesign --verify --strict` succeeds. Signed host and
static ARM64 Linux guest SHA-256 hashes are registered before the first VM run.

Validation: 33 tests passed, zero failed, including the canonical live adapter,
all five persistent campaign tests, interruption/reconnect and private boot/disk
checks. Strict Clippy passed. All six changed/new Rust files pass scoped
rustfmt. Full-crate rustfmt detects an unchanged `tests/host_pack.rs` wrapping
difference, independently reproduced from the baseline; it was not normalized
as part of this change. The guest linker emits its existing deprecated
optimization-setting warning; the release build exits successfully.

The measurement harness has observed RED/GREEN evidence, independently checked
known paired ratios, and six rejecting mutations. Independent Rust/security and
measurement/infrastructure reviews are retained beside these receipts. No
`/quality-gate` executable is exposed by the installed skill package; the two
recorded independent reviews perform its code/security review function. This
does not claim execution of an unavailable command.

Raw test and failure receipts are intentionally retained at the user's request.
This overrides the generic skill's diagnostic-log pruning guideline. Successful
campaign payload stores are disposable only after the runner's exact/read-only
verification gates; no unrelated user work or build cache is removed.
