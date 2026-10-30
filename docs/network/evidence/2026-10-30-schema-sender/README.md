# SC-OPT-013 evidence

The completed paired campaign passed its registered sender-CPU goal with exact protocol/reconstruction parity. See [the results report](../../schema-sender-results-2026-10-30.md) for the improvement, conventional zstd comparison, setup costs and negative controls.

| Record | Purpose |
|---|---|
| [measurement-contract.md](measurement-contract.md) | Predeclared profile, worker order, exactness gate and primary ratio ≤0.90 |
| [comparison-pins.json](comparison-pins.json) | Frozen baseline/candidate source and executable identities; only schema source differs |
| [plan.json](plan.json), [plan-candidate.json](plan-candidate.json) | Identical target/artifact/harness identities; differing production schema hashes |
| [baseline-profile-run.json](baseline-profile-run.json) | Before-edit profile command, scope and custody |
| [baseline-profile.jsonl](baseline-profile.jsonl), [baseline-profile-summary.json](baseline-profile-summary.json) | All 640 independent stage observations and eight setup observations, retaining first use |
| [paired.jsonl](paired.jsonl) | Registration, 80 sequential worker results with all 1,040 nested observations, and completion |
| [summary.json](summary.json) | Exact parity/count validation, ten primary ratios, 32 family cells and 80 actual setup-inclusive session totals |
| [run.json](run.json) | Exact commands, UTC bounds, exits, counts and interpretation limits |
| [final-review.json](final-review.json), [security-review.json](security-review.json) | Procedural/code/Rust review and independent production-only security review |
| `helper-red.txt`, `helper-green.txt`, `harness-tests.txt`, `harness-final-tests.txt`, `harness-clippy.txt` | Actual helper/test/Clippy qualification outputs, preserved without whitespace normalization |
| `baseline-build.txt`, `candidate-build.txt` | Explicit shared-target/triple release builds |

Other parent-owned codec/qualification/coverage receipts remain alongside these files. No failed profile or campaign attempt was discarded. Each command's status is retained in its corresponding `.exit` file. No new Python, dependency, WAN resource or account is part of the experiment.

The artifact is the fixed SC-OPT-012 `artifacts-corrected/schema.artifact` (169 bytes, SHA-256 `79cddd88c69e34f09efaefc0184d8ca310010fd50defd8958dc20ca28ddadcd8`). It was not fitted again here. Every actual destination seed and encoder preparation is included in session totals. Both sources use the same optional transcript retention/hashing behavior; hashes are computed after timers and retained buffers are released before the next timed operation.

Build each source with the same command, preserving its executable before switching source:

```sh
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/Users/casey.allard/spiralcore/target /Users/casey.allard/.cargo/bin/cargo +1.97.1 build --release --locked --target aarch64-apple-darwin --features hardware-sha256 --example schema_fitting --manifest-path crates/spiralcore-transfer/Cargo.toml
```

From the repository root, using new output/cache paths:

```sh
BASE_EXE sender-plan NEW_PLAN.json
BASE_EXE sender-profile 4dacb487cd2b3b132aa6bbf015eb0e5f893e69e0 NEW_PROFILE.jsonl
CANDIDATE_EXE sender-paired NEW_PLAN.json BASE_EXE 4dacb487cd2b3b132aa6bbf015eb0e5f893e69e0 CANDIDATE_EXE 983bf181adda8dc922d0187e53652717d61d38b6 NEW_PAIRED.jsonl NEW_CACHE_DIR
CANDIDATE_EXE sender-summary NEW_PAIRED.jsonl NEW_SUMMARY.json
```

`BASE_EXE` and `CANDIDATE_EXE` stand for the preserved binaries, not commands installed globally. Run sequentially in a quiet measurement window. The primary comparison is the geometric mean of ten candidate/baseline ratios, each formed from the four normal batch targets' summed sender CPU in one paired session; pooled target medians are additional descriptions, not the gate formula.
