# Remove a discarded Windows metadata preflight

This is one measured follow-up under SpiralCore #44 and #49. Product baseline is `bf7107e62e7fa35547f86795b56d7f8d7b3793fc`, whose Rust source matches `661c74e7`. Native timing run 42927983955, job 130219221283, is the retained performance RED: 3,065 exact blocks per direction before the unchanged 240-second overall gate, with 103.046875 process CPU seconds at the 242.002462-second failure snapshot. Receiver Commit means were 38.966/41.512 ms, while Session/TX waits stayed below 1 ms. A prior same-feature pass remains separate evidence; these executions do not isolate host/storage variability or observer overhead.

## Change and invariants

Only Windows `endpoint::read_meta` skips `private_file(path)`, a checked open whose handle is immediately discarded. The following `storage::open_read` retains native path, ancestor, current-SID, owner, DACL, reparse, hardlink and sharing checks on the actual consumed handle. Preserve missing-file mapping and metadata header/length validation. Unix behavior is unchanged. No storage-provider, quota, authorization, format, hash, stage-sync, rename/durability, cache/resource-budget or FlowMux-deadline changes belong to this slice.

## Tests and bounded native comparison

Focused equivalence guards were authored and executed before the production edit. The two portable tests passed against the old production path for header/length, full body/digest, missing-file and hardlink behavior, using an exact test-only copy of that path as control. Directory refusal was added during source review. The retained native performance failure supplies the RED for this optimization; passing equivalence guards establish unchanged semantics rather than a fabricated semantic regression.

One Windows test compares old and candidate paths in the same executable and process: 64 distinct 16 KiB payloads, six paired trials alternating old/new order. The single owned private store contains 1,051,712 serialized object bytes, below 2 MiB, plus bounded directory/junction metadata. Record each pair's wall, aggregate process CPU and calling-thread CPU separately. The CPU domains overlap and must not be summed. Retain sub-resolution zero CPU samples without manufacturing ratios.

Record creation/sync setup and common metadata/body priming as separate measured setup phases. Compare every original byte and digest before the pairs, after each pair, and after negative fixtures, outside the timed metadata windows. This is a commonly primed metadata comparison; it makes no cold OS-cache or cold-transfer claim. Record every pair without asserting a speed ratio or hiding a loss.

Exercise bad DACL, reparse ancestor, hardlink, conflicting writer sharing, directory and missing-file refusal on both paths. ACL and junction fixtures use fixed local PowerShell scripts confined to the owned synthetic fixture, with no received code or product validation bypass. DACL restoration selects the Access section only. Restore permissions, remove owned aliases/junctions, then verify every original byte before cleanup. No new dependency or unsafe Rust is needed; `cpu-time` is already a direct dependency.

Native command after source/security review:

```text
cargo +1.97.1-x86_64-pc-windows-msvc test --release --locked --manifest-path crates/spiralcore-transfer/Cargo.toml --lib endpoint::metadata_tests::windows::single_checked_open_matches_guards_and_records_paired_cost -- --exact --show-output --test-threads=1
```

Portable tests and strict Clippy run on Wick using a fresh immutable 198-file closure from `bf7107e6`, recorded source hashes and separately identified candidate overlays, the existing Cargo target, two build jobs and nice level 10. No Mac build, copied target or large payload archive is needed. Native measurements remain pending until executed; source review or portable success is not a Windows performance result. After narrow independent security review, execute the original 4,096-block/240-second qualification with two-second operation deadlines and retain both passes and failures. Canonical MVM implementation continues independently.
