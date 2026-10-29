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
| SC-OPT-004 | Direct attribution completed; no production scan change selected | 32/32 exact instrumented operations. At N768, sparse quota scans used a midpoint median 25.4735 ms CPU within a 30.174 ms operation; median per-observation share 84.3912%. At N0, 0.1215 ms and 2.7083%. This locates scan cost but does not separate filesystem work from formatting/allocation or establish a safe optimization. | Baseline snapshot `f0966eb`; registration `867ad33`. [Full result](cache-profile-results-2026-10-29.md), [raw evidence and independent Rust review](evidence/2026-10-29-cache-profile/README.md), [options and preserved contract](cache-scan-decision.md). |
| SC-OPT-005 | Finer diagnostic completed; primary attribution screen failed; no candidate adopted | 72/72 exact. Counter-only sparse CPU medians 4.775/7.903/14.047/28.230 ms across empty/compact64/compact768/spread768 layouts. Primary fine-clock inflation 1.533316× exceeds 1.25× ceiling; all six primary screens fail. All 24 cells and 48 screens independently reproduced. | Snapshot `d03fe16`; registration `9c9650d`. [Full result](fine-cache-profile-results-2026-10-29.md), [raw evidence, independent Rust method and cleanup custody](evidence/2026-10-29-fine-cache-profile/README.md). |
| SC-WAN-002 | 16 MiB body/time gates passed; all 180 exact; full-campaign IP qualification incomplete | Thirty localized-edit pairs: 16,814,026 to 600 HTTP-body bytes, paired raw/sparse elapsed 8.734144× and combined CPU 3.165958×; 30/30 wins on both. Seed-plus-update elapsed 1.811394× and about 50% fewer body bytes. Third shard retains an unresolved three-packet received/captured discrepancy, so no complete thirty-pair IP reduction is claimed. | Measured source `f4e5e9a`; [full result](large-wan-results-2026-10-29.md), [raw evidence and Rust reviews](evidence/2026-10-29-large-wan/README.md), [registered plan](../plans/2026-10-29-cache-profile-and-large-wan.md). |

| SC-WAN-003 | Broad v4 campaign in progress; 4/12 transfer cases independently reviewed | Matched byte-publication readiness, four arms, 30 pairs per case. Localized, scattered, prefix and middle insertion passed HTTP-body/time gates; sparse uses 600–4,664 median body bytes with 6.73–8.01× raw-to-sparse median elapsed ratios. All 960 operations exact. Nineteen captures complete; middle shard 1 retains ambiguous connection attribution and no complete whole-case IP ratio. | Measured source `9dce4c5`; [current report](broad-wan-results-2026-10-29.md), [raw evidence](evidence/2026-10-29-broad-wan/README.md), [registration](../plans/2026-10-29-broader-wan-and-scaling.md). |

## Measured object sizes

| Size | Executed evidence | Qualification boundary |
|---|---|---|
| 1 MiB, plus one byte for prefix insertion | Real WAN campaigns with 30 paired trials per workload/arm; SC-OPT-001 sparse results. SC-OPT-002/003 separately use 1 MiB local fixtures. | [Current sparse WAN measurements](efficiency-results-2026-10-29.md); [new local SHA measurements](hardware-sha-results-2026-10-29.md). |
| 16 MiB (16,777,216 bytes) | SC-WAN-002: thirty real-WAN localized-edit pairs, raw/zstd/sparse, 180 exact reopened seed/update operations; native hardware SHA on the Mac. Earlier cold/repeat smoke evidence is retained separately. | [Repeated larger WAN results](large-wan-results-2026-10-29.md): raw median 2,030.458 ms versus sparse 219.650 ms. One-update lifecycle only; third capture incomplete. [Historical cold/repeat smoke](wan-results-2026-10-29.md#cache-seeding-and-larger-objects). |
| 64 MiB | Core protocol ceiling only in this evidence set; the batched client currently caps objects at 16 MiB. | Requires a separate client/resource qualification; a declared core limit is not a working or measured large-object WAN path. |

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
