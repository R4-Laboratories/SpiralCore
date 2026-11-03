# Required native storage gate

This is a prepared gate, **NOT_RUN** here. Add it to the existing native client
matrix, preserving the canonical MVM source/artifact pins and platform roles.
The initial Windows profile is native Server 2022/2025 x64 and Windows 11 ARM64
on local fixed NTFS. Linux and macOS run the same portable Endpoint tests.
Do not infer native Windows MVM/backend or WAN support from these tests.

Use the normal Rust 1.97.1 toolchain, locked transfer manifest and two build
jobs. No new features or alternate protocol are required. Native Windows has
the existing supported C compiler/librarian for the shared codec dependencies.

```sh
cargo test --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --lib endpoint::storage_tests -- --list
cargo test --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --lib endpoint::storage_tests -- --test-threads=1
cargo test --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --test endpoint -- --test-threads=1
```

The new selected library scope currently contains three portable tests plus
three `cfg(windows)` tests: expect three on Unix, six on Windows. Enforce a
positive listed count and equality between listed/passed tests, zero failures
and zero ignored tests. Record the unrelated library filtered count as an
excluded scope; do not claim that selected library tests ran the entire
library. The existing endpoint target ran 21 tests on Linux; its one Unix-only
privacy test is omitted on Windows, so require and report the actual native
count rather than copying a Linux receipt. Windows ACL and junction tests
have no skip-on-setup-failure path. Windows must execute both positive
publication/restart/readback and the refusal cases.

Retain full logs, exit codes, Rust/runner identities, exact source commit and
tested merge/tree identity, and original native artifacts with their API and
inner checksum verification. A build failure, ACL/junction setup failure,
zero test scope or unsupported filesystem is a failed/unavailable gate, not
an accepted native result. Keep the failed local full MSVC cross-check in this
packet distinct from the eventual native toolchain result.

The executed Linux command shapes were the first two selected test commands
above (without the list for runtime) and `cargo clippy --locked --offline
--manifest-path crates/spiralcore-transfer/Cargo.toml --lib --tests -- -D warnings`.
The compiler-only adapter command was `cargo clippy --locked --offline --target
x86_64-pc-windows-msvc --lib -- -D warnings` in the retained adapter directory.
These commands describe the scopes in the raw receipts; this document itself
is not an additional test execution.
