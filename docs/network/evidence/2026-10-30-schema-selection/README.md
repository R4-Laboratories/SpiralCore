# SC-OPT-014 evidence packet

Measured source: `58d71cf10a45fb9f1ad19ae2557135d53a1edc1d`.
[Result and interpretation](../../schema-selection-results-2026-10-30.md).
[Registered contract](measurement-contract.md).
[First-attempt successful WAN run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41135473115).

This packet compares retained codec contexts on distinct new objects. It does
not exercise prior-object sparse/prefix/DAG or shared-runtime reuse. The selector
matches the tuned Whole/Raw control's body bytes on all 600 targets and 60 paid
sessions, but adds CPU. It chooses Whole 480 times, Raw 120 times and Schema zero
times. All 1,800 targets and 60 forced-schema seeds reopen exactly; the 60 unseeded
selector artifacts are confirmed absent. The historical warm-object results
remain separately qualified and are not invalidated by this comparison.

| Evidence | Purpose |
|---|---|
| [qualification/local-final.json](qualification/local-final.json) | Actual final 315-pass release suite, two preexisting ignored tests, Clippy and release build commands/results |
| [qualification/production.json](qualification/production.json) | Behavioral RED, retained build corrections, focused exact-byte/seed tests and bounded coverage |
| [qualification/harness-tdd.json](qualification/harness-tdd.json) | Seed-cost RED/GREEN, 31 real local HTTP transfers with exact readback, summary corruption checks |
| [qualification/workflow.json](qualification/workflow.json) | Actual workflow source/attempt/URL/auth gate RED/GREEN |
| [premeasurement-review.json](premeasurement-review.json) | Sequential procedural and independent code/Rust review |
| [security-production-review.json](security-production-review.json), [security-harness-review.json](security-harness-review.json) | Cross-author bounded security/infra reviews |
| [wan/custody.json](wan/custody.json) | 36 downloaded artifact members, 81 source hashes and three byte-identical cross-host plans |
| [wan/hosted/](wan/hosted/) | Full hosted qualification, source/compiler/features, plans, all nested raw session observations and timing boundaries |
| [wan/summary.json](wan/summary.json) | Complete paired target/session distributions, actual setup totals, cache telemetry joins and exact-readback prerequisites |
| [wan/candidate-cost-analysis.json](wan/candidate-cost-analysis.json) | Arithmetic comparison without schema seed charge; zero target-only schema wins, not another transfer |
| [final-review.json](final-review.json) | Independent raw recomputation and report approval, including the warm-object scope distinction |
| [wan/cleanup.json](wan/cleanup.json) | Owned processes, temporary credentials and synthetic receiver-store disposition |

The three `wan/verification-*.jsonl` files contain the independent stopped-store
readbacks and optional artifact presence checks. `wan/receiver-*.jsonl` retains
every admitted request and cache observation. One preflight ordinal gap per
receiver remains explicit. No transfer was retried or discarded. Original raw
logs retain their whitespace and hashes; they are not normalized to pass source
formatting checks.

The frozen Mac verifier SHA-256 is
`148738fda699d4e177b543b2c55ff53588f7e5af0d2405e62f4c3f728a34445f`;
receiver `572667bda7eba40c269e92c58e7bc6747e96a026c5cc92d06532328e4d26620c`;
Linux sender `85dee51c4e283a5132b958b408b5493dcf4226bcf3cd78e665e7c4d1cd0afd55`.
Mac builds use Rust 1.97.1 with `hardware-sha256`; Linux uses the same compiler
and default features. All compressed candidates share the fixed zstd settings.

Frozen binaries are retained locally and in GitHub Actions artifact
`13016707357` (30-day retention), rather than committed as binary blobs. All
nonbinary artifact members are copied here; the original artifact manifest
also names the binary, whose verified hash and download directory remain in
the custody receipt. Source, lockfile, fixtures and raw measurements remain in
Git for reproduction beyond artifact expiry.

To reproduce, build the pinned source and use the `schema_fitting` commands
documented in the measurement contract. Generate fresh bounded plans/scopes
and capabilities, run the foreground supervisor while the hosted sender runs,
then stop receivers before `selection-verify`. Run `selection-summary` on all
three plan/observation/verification triples. Never reuse this campaign's retired
URLs or capability. HTTP-body byte counts and measured CPU scopes are not
complete-path packet or energy measurements.
