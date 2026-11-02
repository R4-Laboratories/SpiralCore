Both diagnostics used Rust 1.97.1, `--locked`, `CARGO_BUILD_JOBS=2`,
`CARGO_INCREMENTAL=0`, and the existing
`CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target`.

The measured production source was
`0f15e3d3f8b5ae3521386905e4a4d43e838ed6fb`. The diagnostic example was new,
with its exact measured source and executable SHA-256 recorded before each run.
There were no production edits. The second diagnostic changed only the example;
the first executable and exact example source were preserved before that edit.

Build commands (with the environment above):

```sh
/Users/casey.allard/.cargo/bin/cargo +1.97.1 build --locked --release --manifest-path crates/spiralcore-transfer/Cargo.toml --example schema_context_profile
/Users/casey.allard/.cargo/bin/cargo +1.97.1 build --locked --release --manifest-path crates/spiralcore-gateway/Cargo.toml --example object_fixtures
```

The first command was run once for each example version. Build exits and output
are retained as `profile-build.*`, `fixture-build.*`, and `allocation-build.*`.
The resulting executables were copied before timing to the paths below.

Canonical fixtures were generated outside timing with the unchanged generator:

```sh
for trial in 0 1 2; do
  /Users/casey.allard/spiralcore/local/schema-context-cpu-20261102/object-fixtures /Users/casey.allard/spiralcore/local/schema-context-cpu-20261102/fixtures-$trial "$trial" combined
done
```

Every generated manifest is retained as `fixture-{0,1,2}.json` and hashed in
`fixture-identities.sha256`. The additional fitting control is deterministic
Rust code in each frozen example; each target and intermediate identity appears
in an `input` record before its compression measurements.

Measured commands, each executed once:

```sh
/Users/casey.allard/spiralcore/local/schema-context-cpu-20261102/schema-context-profile /Users/casey.allard/spiralcore/local/schema-context-cpu-20261102 /Users/casey.allard/spiralcore/local/schema-context-cpu-20261102/context-profile.jsonl 0f15e3d3f8b5ae3521386905e4a4d43e838ed6fb
/Users/casey.allard/spiralcore/local/schema-context-cpu-20261102/schema-allocation-profile /Users/casey.allard/spiralcore/local/schema-context-cpu-20261102 /Users/casey.allard/spiralcore/local/schema-context-cpu-20261102/allocation-profile.jsonl 0f15e3d3f8b5ae3521386905e4a4d43e838ed6fb
```

Each run contains 144 paired observations (288 arm observations), six paired
repetitions per input, balanced arm order, and no discarded warmup. Both produced
exact compressed bytes and exact decoding throughout. First-use and repeated
summaries remain separate. CPU and wall intervals are separate; pair intervals
contain the stage intervals and must never be added to them.

Exact measured source snapshots remain under the same local directory:
`source/schema_context_profile.rs` for context reuse and
`source/allocation/schema_context_profile.rs` for allocation reuse.
`profile-identities.sha256` and `allocation-identities.sha256` bind them to their
executables and unchanged codec sources. All raw rows and summary arithmetic
are retained. These are local codec diagnostics, not network or energy results.
