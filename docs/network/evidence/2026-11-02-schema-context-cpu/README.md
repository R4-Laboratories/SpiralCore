# Exact schema-context CPU diagnostics

Production baseline: `0f15e3d3f8b5ae3521386905e4a4d43e838ed6fb`.
Decision: neither experimental variant is promoted. The remaining unrelated
object CPU gap is open. [Result and limitations](../../schema-context-cpu-results-2026-11-02.md).

Two Rust helper versions each ran once on the same native macOS host and the same
24 input cases: three fixture trials, two sizes, four classes. Each retained six
balanced paired repetitions, producing 144 pairs / 288 observations per dose.
All compressed frames and decoded bytes match exactly. First-use observations
are retained separately from repeated state; none was discarded.

## Readable evidence

- `context-profile.jsonl` and `.summary.json`: separate versus shared public
  Whole encoder contexts, including canonical schema-stream compression.
- `allocation-profile.jsonl` and `.summary.json`: joint context sharing and
  retained output-vector experiment, using raw CCtx calls and original-encoder
  frame comparisons outside both arms' compression timers.
- `context-source.rs.txt`: exact first measured helper. The second helper is
  [the runnable crate example](../../../../crates/spiralcore-transfer/examples/schema_context_profile.rs).
- `profile-identities.sha256`, `allocation-identities.sha256`,
  `fixture-identities.sha256`, and `fixture-{0,1,2}.json`: source, executable,
  fixture and lockfile identities frozen before timing. The production revision
  is not a claim that the then-new helper was already committed: its separate
  source hash and archived snapshot bind the measured addition.
- `run-commands.md`, `toolchain.txt`, `*-build.*`, `*-run.*`: invocation and
  completion receipts. Both run exits are zero.
- `profile-clippy.*`, `profile-format.*`: targeted final-example strict Clippy
  and formatting checks, both exit zero. No production suite or network campaign
  was repeated for unchanged production sources.
- `review-independent.md`: cross-author Rust/code/security review and independent
  arithmetic for every summary field, including the negative controls.
- `production-unchanged.diff`: empty successful comparison of transfer production
  sources/manifest/lockfile and gateway/core against the production baseline.

The independent review fulfills the narrow code/security gate. No executable
`/quality-gate` command is installed in this environment, and none is claimed.
Logs listed here are retained experimental evidence requested by the user.

## Immutable executable custody

The original measured executables, both helper snapshots, generator, JSONL,
summaries, fixture manifests and invocation receipts are in the existing private
Hugging Face dataset, not committed as binaries to Git:

- Dataset: `caseyallard/spiralcore-experiment-artifacts`
- Path: `2026-11-02/schema-context-cpu-local.zip`
- Revision: `c94730de0ea8d22ba2c2455730c486c084af0714`
- Size: 1,485,562 bytes
- SHA-256: `e992baec8a8ebcbd078947df896064f6464d886a51f8a51a25f9d867cc266c52`

`artifact-storage.json` records the upload and verification. A forced download
from that exact revision to a fresh directory, with Xet disabled, matched the
original ZIP byte for byte. All 30 files in its internal manifest verified after
fresh extraction. The manifest and local/fresh verification outputs are retained
here. Generated fixture payloads are reproducible and are excluded from the
archive; their generation source, commands, seeds and exact identities remain.

## Reproduction

Use Rust 1.97.1 and the locked crate dependencies. The detailed commands in
`run-commands.md` identify the original paths; use a new owned output directory
for a new run. Generate `fixtures-0`, `fixtures-1`, `fixtures-2` with the unchanged
gateway `object_fixtures` example and its `combined` profile. The helper also
constructs deterministic fitting controls and records their identities.

The current transfer `schema_context_profile` example reproduces the joint
context/output-vector variant. To reproduce the original context-only variant,
place `context-source.rs.txt` at that example path in a disposable checkout of
the baseline before building. The examples share no production changes. Both
require the fixture root, a new JSONL output path and the baseline revision as
arguments. Both create output exclusively and refuse to overwrite prior results.

Reproduction establishes a new measurement; it cannot recreate historical
timing. Compare payload/frame identities and ordered observations, and retain
new environmental and executable identities. CPU intervals are nested within
pair wall intervals; do not sum overlapping stages or infer network-send time
by subtracting CPU from elapsed. Neither dose is a WAN or energy measurement.
