# Installed-owner component tests

This small standalone crate compiles the exact production
`../../src/installed_owner.rs` file with only libc and sha2. It avoids a second
implementation and the native MVM dependency graph during storage/lifecycle
tests. The product module is exported only for the admitted native Unix host
builds; this harness does not establish a Windows host backend.

Run from the repository root with the pinned toolchain:

```sh
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-mvm-live/tests/installed-owner-harness/Cargo.toml -- --test-threads=1
cargo +1.97.1 clippy --locked --manifest-path crates/spiralcore-mvm-live/tests/installed-owner-harness/Cargo.toml --all-targets -- -D warnings
```

Process-lifetime fixtures must run serially. On Linux, a concurrently spawned
test subprocess can inherit another test's open-file descriptions until exec,
even with CLOEXEC. The extra inherited holder correctly causes a nonblocking
Busy refusal. Serial execution removes that unrelated test interference; it
does not relax the product's refusal semantics or test assertions.

Four private fault-boundary unit tests and eleven lifecycle entries execute on
Linux; macOS adds the extended-ACL regression. One lifecycle entry is the
subprocess helper, not a separate acceptance scenario. The tests use real
processes and locks but boot no guests. Actual worker/VMM lease custody and
installed admission remain separate integration requirements.
