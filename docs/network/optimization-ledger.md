# SpiralCore optimization ledger

This is the review entry point for ongoing optimization work. Every experiment records its baseline,
candidate, fixture and executable identities, measured scope, failures and unrun qualifications.
Raw receipts and analysis are committed before beginning the next implementation round. Findings
remain tied to their measured source; later results do not overwrite earlier negatives.

| ID | State | Question and result | Source and evidence |
|---|---|---|---|
| SC-OPT-001 | Implemented, measured, independently reviewed | Can direct graph construction, retained verified state and sparse recipes reduce computation while preserving exact bytes? Yes on the measured fixtures: direct-builder CPU 2.39–4.63× better; sparse warm WAN elapsed 1.83–2.58× better and 99.80–99.98% fewer HTTP-body bytes. Localized elapsed misses 2×. Seed-plus-update savings are smaller. | Baseline `354b616257eda62cd6bdcc2fb37902fc4cb4a5e8`; measured WAN source `b5520171ab1cfbc988e16b2b79f41ab831d3668a`; evidence commit `fedf38a8e308f3064b6ca3006aaf721a418c0d80`. [Report](efficiency-results-2026-10-29.md), [raw evidence and checksums](evidence/2026-10-29-efficiency/README.md). |
| SC-OPT-002 | Baseline characterized; narrow candidate under regression testing | Sixteen observations passed exact reconstruction. At 768 background objects, sparse local-operation CPU was 37–40 ms, versus about 17 ms with only the active base; single-cell samples are noisy. Selected candidate hoists invariant namespace formatting/comparison while preserving every filesystem/quota check. Paired effect estimate pending. | Production baseline `fedf38a8e308f3064b6ca3006aaf721a418c0d80`; probe build revision `74ee59d`. [Plan and acceptance](../plans/2026-10-29-receiver-efficiency.md), [baseline receipts and method](evidence/2026-10-29-receiver-cost/README.md), [architecture decision](receiver-quota-scan-decision.md). |

## Recording rules

- Before implementation: pin the baseline, hypothesis, expected affected work, correctness gates,
  comparison policies and resource limits. Mark pending or unrun results explicitly.
- Before measurement: record source and executable hashes, command/configuration, input generator
  and seed, trial ordering, cache state, units, timing boundaries and exclusions. Do not run builds
  or another benchmark concurrently with a timed campaign.
- After each campaign: retain every success and failure receipt, exact-byte verification, both
  directions' byte counts at a stated layer, elapsed and endpoint CPU, seed and cleanup costs,
  distributions and paired comparisons. Distinguish measured values from projections.
- Preserve raw command output byte-for-byte, including route/resolver output's trailing spaces.
  Source/document whitespace checks exclude such original artifacts; normalizing them would change
  the evidence hashes. The SC-OPT-001 raw route/resolver files are this explicit exception.
- Before advancing: independently review the code and evidence; preserve regressions and control
  failures; commit compact artifacts with hashes. Keep large local captures identified by hash.
- Never infer energy from CPU alone, network-only time by subtracting CPU from elapsed, or total
  path traffic from a single captured relay leg. CPU and wall time overlap.
- A green workflow is an execution status. Byte equality, durable commit, performance, packet
  completeness, security qualification and production readiness remain separate claims.

The original Python runtime and saved sessions are preserved in the primary checkout. New active
implementation and diagnostic helpers use Rust. Research modules remain references unless an
experiment explicitly integrates and measures them.
