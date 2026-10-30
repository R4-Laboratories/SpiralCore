# SC-OPT-012 evidence

The completed campaign is **`paired-corrected.jsonl`**, summarized by **`summary-corrected.json`**. It contains 1,600 distinct-target transfers, 120 seeds, 40 sender preparations, 1,720 immediate and reopened full-byte comparisons, and 160 successful persistent-session cleanups. [The results report](../../schema-reuse-results-2026-10-30.md) distinguishes byte savings, CPU, durable local elapsed, setup, context retention and negative controls.

| Record | Purpose |
|---|---|
| [measurement-contract.md](measurement-contract.md) | Registered sequence, accounting rules and transparent pre-transfer correction |
| [plan-corrected.json](plan-corrected.json) | Frozen corrected source and all 200 paired target identities |
| [paired-corrected.jsonl](paired-corrected.jsonl) | Every completed campaign observation; no warm-up/outlier exclusion |
| [summary-corrected.json](summary-corrected.json) | Existing Rust summarizer; 64 family cells, 160 position cells, actual ten-send session totals |
| [run-corrected.json](run-corrected.json) | Commands, UTC bounds, exits, counts, scope and attempt custody |
| [source-pins-corrected.json](source-pins-corrected.json) | Source/executable/plan/raw/summary and immutable artifact identities |
| `artifacts-corrected/` | Exact training corpus and serialized schema/dictionary artifacts |
| [attempt1-failure.json](attempt1-failure.json) | Startup permission defect; zero seeds/targets; original fit observations retained |
| [plan.json](plan.json), [paired.jsonl](paired.jsonl), `artifacts/`, `run-*` without `corrected` | Original failed attempt, unchanged |
| [plan-stale-target.json](plan-stale-target.json) | Stale explicit-target executable plan caught before corrected timing; never executed |
| `helper-red.txt`, `helper-green.txt` | Scheduling helper test-first evidence |
| `harness-tests.txt` | Cargo PATH collision before compilation; no installation performed |
| `harness-tests-rustup.txt`, `harness-build.txt` | Initial six-test qualification and release build |
| [session-permissions-red.txt](session-permissions-red.txt), [harness-corrected-tests.txt](harness-corrected-tests.txt) | Real session reproduces failure, then seven tests pass after the narrow fix |
| [harness-corrected-build.txt](harness-corrected-build.txt) | Corrected implicit-native build; final execution uses the independently pinned explicit-target binary |

Each command receipt has its corresponding `.exit` file; parent qualification/review receipts are retained alongside these records. All data is synthetic. Metrics are local serialized HTTP-body-equivalent bytes and durable transfer CPU/elapsed, not WAN packet totals or energy. Prepared arms enable both sender and receiver state reuse; baseline arms retain the endpoint but disable its fitted decoder cache.

Reproduce from source `fa0b0abdde0200c9faf62701daa28f9727cc62b4` with explicit Rust Cargo and the explicit target triple, using new paths for all outputs:

```sh
CARGO_INCREMENTAL=0 /Users/casey.allard/.cargo/bin/cargo +1.97.1 build --release --locked --target aarch64-apple-darwin --features hardware-sha256 --example schema_fitting --manifest-path crates/spiralcore-transfer/Cargo.toml
crates/spiralcore-transfer/target/aarch64-apple-darwin/release/examples/schema_fitting reuse-plan NEW_PLAN.json
crates/spiralcore-transfer/target/aarch64-apple-darwin/release/examples/schema_fitting reuse-run NEW_PLAN.json fa0b0abdde0200c9faf62701daa28f9727cc62b4 NEW_RESULTS.jsonl NEW_CACHE_DIR NEW_ARTIFACT_DIR
crates/spiralcore-transfer/target/aarch64-apple-darwin/release/examples/schema_fitting reuse-summarize NEW_RESULTS.jsonl NEW_SUMMARY.json
```
