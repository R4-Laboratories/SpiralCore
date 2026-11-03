# Executed normal validation commands

Product source: commit `d4f2ed412ae961c5d2bd267caf0ec1a2628157c1`,
SHA-256 `b75172d56b284adf9b69417fac3d6835fd9cb2edf311b871cb518222bda1ae57`.
Subsequent README/CI changes do not change this module or its tests. Commands
below are the expanded task invocations; stdout and numeric exit receipts are
retained alongside this record. Commands do not establish additional tests.

## Mac ARM64 / macOS27.0 build26A5378n

From this SpiralCore worktree, Rust1.97.1 via `rustup run` (PATH cargo is a
different shim; its rejected invocation is retained separately):

```sh
CARGO_TARGET_DIR=/tmp/spiralcore-installed-owner-20261103/target rustup run 1.97.1 cargo test --manifest-path crates/spiralcore-mvm-live/tests/installed-owner-harness/Cargo.toml --locked --offline extended_acl_access_refuses_even_when_posix_modes_remain_private -- --exact
CARGO_TARGET_DIR=/tmp/spiralcore-installed-owner-20261103/target rustup run 1.97.1 cargo test --manifest-path crates/spiralcore-mvm-live/tests/installed-owner-harness/Cargo.toml --locked --offline
CARGO_TARGET_DIR=/tmp/spiralcore-installed-owner-20261103/target rustup run 1.97.1 cargo clippy --manifest-path crates/spiralcore-mvm-live/tests/installed-owner-harness/Cargo.toml --locked --offline --all-targets -- -D warnings
```

`acl-fixed`, `core-final-macos`, `clippy-final-macos` respectively. Mac full
harness was parallel and passed; designated reproducible process qualification
uses serial fixtures on both Unix systems as documented in the harness README.
This Mac run does not establish macOS26 or Intel execution; native CI records
are separate.

## Linux x86_64 / Wick

Source was copied to the new owned directory
`/var/tmp/spiralcore-installed-owner-20261103/source`. The exact five compiled
file hashes are `linux-source.sha256`. Pinned native Rust1.97.1 is selected
through `/home/callard/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin`.

```sh
CARGO_TARGET_DIR=/var/tmp/spiralcore-installed-owner-20261103/target CARGO_BUILD_JOBS=2 cargo test --manifest-path crates/spiralcore-mvm-live/tests/installed-owner-harness/Cargo.toml --locked --offline
```

This first parallel execution is **FAIL** (four unit passes, eight lifecycle
passes plus one safe Busy failure). It is `core-final-linux`, despite that
premature filename. The exact-test, serial lifecycle, trace and instrumented
diagnostic commands/results are recorded in `linux-first-diagnosis.md`; their
scope must not overwrite the original failure. Instrumented coverage has its
own `../coverage/collect.sh`, separate target, source pins and explicit serial
execution, and is not a CPU/performance run.

The full crate graph was independently copied from `git archive d4f2ed41 crates`
to `full-source` under the same owned root. From there:

```sh
PATH=/home/callard/.cargo/bin:$PATH CARGO_TARGET_DIR=/var/tmp/spiralcore-mvm-service-20261102/target CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 cargo +1.97.1 test --release --locked --offline --manifest-path crates/spiralcore-mvm-live/Cargo.toml --features linux-host --lib --tests
PATH=/home/callard/.cargo/bin:$PATH CARGO_TARGET_DIR=/var/tmp/spiralcore-mvm-service-20261102/target CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 cargo +1.97.1 clippy --locked --offline --manifest-path crates/spiralcore-mvm-live/Cargo.toml --features linux-host --all-targets -- -D warnings
```

`main-linux` records115 passes across the nine nonempty targets plus zero-test
binary target. `main-clippy-linux` records exit0. The four private component
unit tests overlap with the focused harness; do not add these counts as unique
scenarios. No guest was booted by any command in this packet. These tests do
not establish installed VM lease integration, network performance or WAN.
