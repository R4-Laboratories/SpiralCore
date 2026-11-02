# Unrelated gateway CPU evidence

[Full report](../../unrelated-gateway-cpu-results-2026-11-02.md).
Candidate `c7ee11fdde26c922da9ae6d9574a91f88b2a3573`, baseline
`4a8ec79c9b185c85407e9767653dbcc4af97bf7e`.
[Run 41446807449](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41446807449)
completed 18 sessions, 264 operations and 216 exact reopened deliveries, with
six successful before/after parity comparisons and complete captures.

- [Registered dose](network-registration.json), [analysis method](analysis-method.md),
  [design](../../../specs/2026-11-02-unrelated-gateway-cpu-design.md) and
  [implementation plan](../../../plans/2026-11-02-unrelated-gateway-cpu.md).
- [Native before profile](profile-before.json), [after profile](profile-after.json),
  [all paired stage comparisons](profile-comparison.json), [metadata](profile-metadata.json).
  These independent native stages are not summed into transfer cost. Both exact
  profiler binaries and source/fixture variants are preserved in the local-profile archive.
- Focused RED/GREEN, full scoped 72-pass run (including the new focused tests;
  one existing ignored diagnostic), final 15 core/42 gateway local tests and
  strict Clippy receipts. Linux runs two additional gateway platform tests.
- [Independent core review](review-core-independent.md),
  [infrastructure review](review-infra-independent.md),
  [implementation gate](implementation-gate.md). Infrastructure test originals
  are packed into `infra-check-receipts.tar.gz` with per-file manifest.
- [Readable complete analysis](analysis.json), [independent arithmetic review](review-analysis-independent.md),
  [delivery verification](delivery-verification.md).
- [Compact original receipts](compact-receipts.tar.gz): 887 files, excluding full
  PCAPs and executable binaries. [Per-file hashes](compact-receipts-files.sha256),
  [bundle hash](compact-receipts.sha256), and reopened verification are retained.
- [Full original campaign archive](unrelated-run-41446807449-archive.json) and
  [local profiler archive](local-profile-archive.json): immutable private Hugging
  Face revisions, freshly downloaded and verified by size, SHA-256 and bytes.
  Dataset access is required. Historical raw evidence trees remain unchanged.

Reproduce from a fresh owned directory:

```sh
receipt_dir=$(mktemp -d)
tar -xzf docs/network/evidence/2026-11-02-unrelated-gateway-cpu/compact-receipts.tar.gz -C "$receipt_dir"
# Run the manifest check from receipt_dir with an absolute path to the manifest.
# Build the analyzer from this source; an old shared-target binary may predate the schema.
cargo +1.97.1 run --locked --release \
  --manifest-path tools/gateway-network/runtime-analysis/Cargo.toml \
  --bin analyze_runtime_objects -- "$receipt_dir/candidate-evidence/campaign" > /tmp/unrelated-analysis.json
cmp /tmp/unrelated-analysis.json docs/network/evidence/2026-11-02-unrelated-gateway-cpu/analysis.json
```

Original and fresh compact extraction produce analysis SHA-256
`a97182d496dac0173815ee2d5d67f00ebe282489a468e2f0830012c86fde9f02`.
Root initially invoked a stale release analyzer and received `unknown campaign
registration schema`; `analysis-command.exit` retains exit1. The explicit
source-bound Cargo build/run in `analysis-command-02.*` exits0. No source or
campaign was changed to resolve that invocation mistake.

Root verified the original ZIP against its GitHub API digest, all 908 artifact
entries, 167 candidate source entries, 154 pinned baseline entries and four
executables. The executed workflow blob matches measured source. Full artifact
manifests reference archived captures/binaries omitted from this compact bundle;
use the original ZIP for their full verification. Raw whitespace remains exact.

Three descriptive trials on one calibrated Linux runner, not a new Internet-WAN,
energy, transparent TUN or current Combined MVM qualification. Selected logical
bytes remain identical; captured IP and CPU/time variation remain measured
outcomes. Per-operation logical bytes are not per-operation IP bytes. CPU overlaps
elapsed and is never added to it or subtracted to invent send-only time.
