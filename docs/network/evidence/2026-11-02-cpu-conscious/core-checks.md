# Transfer implementation checks

Contract commit: `284b7c3`. Production scope is the four transfer files listed in
`core-source.sha256`; that manifest also binds the four changed/new test files.
No receiver, wire, crypto, dependency or default policy change.

All Cargo commands used `/Users/casey.allard/.cargo/bin/cargo +1.97.1`,
`CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target`, `CARGO_BUILD_JOBS=2`,
`CARGO_INCREMENTAL=0`, `--locked`, and manifest
`crates/spiralcore-transfer/Cargo.toml` from the repository root.

- RED: `test --lib --test combined_session cpu_conscious`; exit101 on missing
  policy, constructor, counted-encoding and test-only compression-counter APIs.
  An initial shell wrapper tried assigning zsh's reserved `status` variable after
  Cargo and failed to record its exit; the unchanged command was immediately
  rerun using `core_test_exit`. `core-red.*` is that recorded run.
- First GREEN attempt: same command; the new tiny-key JSON test wrongly expected
  no useful record. The valid JSON layout did save bytes. Its correction tests
  the defensive non-beneficial branch with an explicitly synthetic private
  template. No production code changed for that correction. Keep `green-01`.
- Focused GREEN: same command; 3 unit and 1 persistent endpoint tests passed.
- Preservation regression: `test --lib --test combined_session --test schema_codec
  --test schema_prepared --test schema_sender --test schema_reuse`; 73 passed,
  including a subsequent poison/oversize preparation test and existing immutable
  schema/selection/frame witnesses. No ignored or failed tests.
- Strict Clippy: the same target list with `clippy` and `-- -D warnings`; exit0.
- Rustfmt1.97.1 with `--edition 2021 --config skip_children=true --check` on the
  eight owned source/test files; final exit0. Initial format01 requested wrapping
  one new test assertion, which was applied. Frozen production hashes were
  unchanged by that final test-only formatting correction.

The test-only counter increments immediately before real `compress2`. The no-fit
policy test observes zero calls; a mixed stream with one emitted template observes
one, and explicit exhaustive observes full compression. This is functional work
omission evidence, not a CPU speedup or network result. Performance and any byte
regression remain for the separately registered gateway campaign.
