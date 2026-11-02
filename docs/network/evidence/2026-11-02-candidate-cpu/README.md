# Candidate CPU evidence

[Report](../../candidate-cpu-results-2026-11-02.md).
Measured candidate `343248065628176f76841e98608a4a4badf747dd`, baseline
`4b8cb79fe5fca9e9527a2923924c887d9e7ddbf8`.
[Paired run41399314329](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41399314329)
completed18sessions,264operations and216 exact reopened deliveries. Separate
[TUN regression41399314515](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41399314515)
completed12sessions and48 exact flows. No retry or failed network campaign.

- Before execution: [registration](network-registration.json),
  [analysis contract](analysis-method.md), [design](../../../specs/2026-11-02-candidate-cpu-design.md)
  and [implementation plan](../../../plans/2026-11-02-candidate-cpu.md).
- Local diagnosis: [profile](profile-before.json), [metadata](profile-metadata.json),
  [fixture parity](profile-fixture-parity.json), exact compiled helper/fixture
  snapshots in `profile-source/`. These independent stage measurements are not
  additive transfer cost or network speedup evidence.
- Correctness: frozen selector and independent original schema encoder comparisons,
  RED/GREEN and corrected regression receipts, [core review](review-core-independent.md),
  [infra review](review-infra-independent.md), [implementation gate](implementation-gate.md).
  One preexisting ignored diagnostic and the earlier invalid test-target invocation
  are explicitly retained. Mac final gateway15core/42gateway tests passed; Linux
  runs15core/44gateway tests because two additional Linux-only checks execute.
- Results: [full analysis](analysis.json), [raw compact run](linux-run/),
  [independent arithmetic review](review-analysis-independent.md),
  [delivery verification](delivery-verification.md). All source, operation,
  receiver, capture-summary and calibration receipts remain in the compact run.
- Full original captures/binaries: [main archive receipt](candidate-run-41399314329-archive.json),
  [TUN archive receipt](quick-regression-41399314515-archive.json),
  [local profiler archive](local-profile-archive.json). These point to immutable
  revisions in the existing private Hugging Face dataset. Repository access is
  required to retrieve them. Compact source/test/results stay in Git.

Root verified904main artifact entries,165candidate source entries,151pinned
baseline source entries and4frozen executables against original Git blobs.
The quick regression verifies862artifact entries,44source entries and2binaries.
Both executed workflow blobs match measured source. Original ZIPs match the
GitHub API digests; independent fresh forced downloads match size, SHA-256 and
bytes. `verified-*.txt` and archive receipts distinguish these checks.

Reproduce the full analysis from compact retained records:

```sh
CARGO_TARGET_DIR=/path/to/shared-target cargo +1.97.1 run --locked --release \
  --manifest-path tools/gateway-network/runtime-analysis/Cargo.toml \
  --bin analyze_runtime_objects -- \
  "$PWD/docs/network/evidence/2026-11-02-candidate-cpu/linux-run/campaign" > /tmp/candidate-analysis.json
cmp /tmp/candidate-analysis.json docs/network/evidence/2026-11-02-candidate-cpu/analysis.json
```

Both full-original and compact-input analysis exited0 and produced SHA-256
`69b6efd388c8acfd418e528c05a52f079ca1aab26ef8b5631c22a00a933efef2`.
Raw output whitespace is preserved intentionally. Full manifests reference
archived PCAPs/executables omitted from Git; use the original ZIP for those checks.

Three paired trials, no outlier filtering, confidence interval, p95 or energy
claim. Real QUIC transfers over calibrated local/shaped Linux links are not a
new Internet-WAN test, two physical machines, transparent TUN runtime compression
or MVM qualification. Logical, IP and Ethernet bytes are separate layers; CPU
is not added to elapsed or subtracted to invent isolated send-only time.
