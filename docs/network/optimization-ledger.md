# SpiralCore optimization ledger

This is the review entry point for ongoing optimization work. Every experiment records its baseline,
candidate, fixture and executable identities, measured scope, failures and unrun qualifications.
Raw receipts and analysis are committed before beginning the next implementation round. Findings
remain tied to their measured source; later results do not overwrite earlier negatives.

| ID | State | Question and result | Source and evidence |
|---|---|---|---|
| SC-OPT-001 | Implemented, measured, independently reviewed | Can direct graph construction, retained verified state and sparse recipes reduce computation while preserving exact bytes? Yes on the measured fixtures: direct-builder CPU 2.39–4.63× better; sparse warm WAN elapsed 1.83–2.58× better and 99.80–99.98% fewer HTTP-body bytes. Localized elapsed misses 2×. Seed-plus-update savings are smaller. | Baseline `354b616257eda62cd6bdcc2fb37902fc4cb4a5e8`; measured WAN source `b5520171ab1cfbc988e16b2b79f41ab831d3668a`; evidence commit `fedf38a8e308f3064b6ca3006aaf721a418c0d80`. [Report](efficiency-results-2026-10-29.md), [raw evidence and checksums](evidence/2026-10-29-efficiency/README.md). |
| SC-OPT-002 | Correctness passed; performance target missed; candidate reverted | All 256 paired updates were exact. Registered N768 sparse CPU ratios were 1.079686× (6/8 wins) and 1.037647× (4/8), below the joint 1.10×/6-win gate. Controls included regressions and substantial host variation. Candidate b4a1860 was reverted by cb376b8; no new WAN or energy gain is claimed. | Baseline probe `74ee59d`; candidate `b4a1860`. [Complete result and disposition](receiver-cost-results-2026-10-29.md), [raw evidence](evidence/2026-10-29-receiver-cost/README.md), [registered plan](../plans/2026-10-29-receiver-efficiency.md), [architecture decision](receiver-quota-scan-decision.md). |
| SC-OPT-003 | Implemented opt-in; native local qualification and all registered CPU gates passed | Same-source software/hardware builds completed 256 exact local updates. Registered CPU ratios 2.104109×, 3.668717× and 1.412714×, each 8/8 wins. All 16 cells favor hardware CPU; 126/128 paired elapsed wins, with negatives retained. Application bytes unchanged. Default-off feature; no new WAN or energy claim. | Source `8044993`; preregistration `f6a0144`; qualification checkpoint `be2dfc3`. [Full result](hardware-sha-results-2026-10-29.md), [raw evidence and reviews](evidence/2026-10-29-hardware-sha/README.md), [registered plan](../plans/2026-10-29-hardware-sha256.md), [architecture decision](hardware-sha256-decision.md). |

## Measured object sizes

| Size | Executed evidence | Qualification boundary |
|---|---|---|
| 1 MiB, plus one byte for prefix insertion | Real WAN campaigns with 30 paired trials per workload/arm; SC-OPT-001 sparse results. SC-OPT-002/003 separately use 1 MiB local fixtures. | [Current sparse WAN measurements](efficiency-results-2026-10-29.md); [new local SHA measurements](hardware-sha-results-2026-10-29.md). |
| 16 MiB (16,777,216 bytes) | Earlier real WAN raw/zstd cold transfer and whole-object repeat, one trial per arm; exact recovery. Separate local TCP/save-restart check also passed. | [16 MiB WAN results](wan-results-2026-10-29.md#cache-seeding-and-larger-objects). Full resend 1,758.386 ms versus whole-object hit 212.462 ms in that repeat trial. Not a repeated sparse-edit or new hardware-backend WAN qualification. |
| 64 MiB | Protocol ceiling only in this evidence set. | A declared limit is not a completed large-object performance qualification. |

Older UOR records include a 52,535,088-byte WAN object. They are historical results
on the imported predecessor, not current SpiralCore transfer qualification; see the
[audit's historical evidence](audit.md#historical-performance-evidence).

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
