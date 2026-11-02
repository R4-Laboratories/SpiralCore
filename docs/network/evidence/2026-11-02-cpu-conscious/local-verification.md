# Local implementation verification

Baseline: 9f5912698f6ab46ddc771fe15fd0c0a7bcec8c5b. The user approved the
opt-in policy tradeoff; Exhaustive remains default. Existing isolated worktree
and shared Cargo target were reused, with Rust 1.97.1, --locked,
CARGO_BUILD_JOBS=2 and CARGO_INCREMENTAL=0. No production Python or dependency
change was introduced.

- Gateway baseline: 2 existing object_combined tests passed.
- Gateway RED: new combined-cpu configuration failed at validation, exit 101.
- Gateway GREEN: 4 object_combined tests passed, including both actual pinned
  authenticated sessions and exact reopened objects.
- Gateway-core: 15 all-target tests passed.
- Final gateway: 34 lib/bin/integration tests passed; fixture tests are separately
  recorded by the measurement owner. Strict all-target Clippy passed.
- Transfer: see core-checks.md for the 73-test scoped regression, strict Clippy,
  formatting, frozen encoder parity and expected/corrected failures.
- Measurement: 6 fixture tests and 12 analyzer tests passed; shell/workflow
  validation and focused Clippy passed. Four historical complete analyses match
  byte for byte. See infra-readiness.json and the historical receipts.

Root commands, with the environment above:

```
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-gateway/Cargo.toml --test object_combined
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-gateway-core/Cargo.toml --all-targets
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-gateway/Cargo.toml --lib --bins --tests
cargo +1.97.1 clippy --locked --manifest-path crates/spiralcore-gateway/Cargo.toml --all-targets -- -D warnings
```

The focused 4-test gateway run is contained in the later 34-test run; do not add
those counts. Test commands use real transfer/receiver paths, not measurements
of claimed speed. Timing claims require the separate registered campaign.

Cross-author code/security and infrastructure/measurement review passed in the
two review-*-independent.md receipts. No executable /quality-gate command is
installed; these actual reviews supply the gate, not a claimed command run.
Raw validation and failure logs are retained as requested experimental evidence.
No unrelated coverage audit or new test framework was added.
