# Executed commands and source scope

Rust1.97.1, CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, existing owned Wick publication-target.

```sh
cargo +1.97.1 test --manifest-path crates/spiralcore-mvm-live/Cargo.toml --release --offline --locked --features live,linux-host,canonical-host --lib --test installed_parent --no-run
cargo +1.97.1 test --manifest-path crates/spiralcore-mvm-live/tests/parent-retirement-harness/Cargo.toml --release --offline --locked --test retirement --no-run
```

Emitted lib, parent and Mock qualification executables were copied and chmod500 before the following protected executions, with source and binary checks before/after:

```sh
./lib-tests installed_parent::retirement_policy_tests:: --test-threads=1 --nocapture
./lib-tests installed_resources::tests:: --test-threads=1 --nocapture
./parent-tests --test-threads=1 --nocapture
./mock-test --test-threads=1 --nocapture
```

`parent-retirement-v2-protected/` is the final executed scope. The earlier narrower result and initial fixture failure remain separate. Intentional caught thread panics in accounting-poison tests are visible. The metadata-only pending API probe uses the exact command/protected library hash in its JSON record; its exit1 is expectedE0599 before implementation. No VM/performance/all-target release qualification ran here.
