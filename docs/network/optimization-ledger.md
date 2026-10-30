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
| SC-WAN-003 | Registered broad v4 campaign completed and independently reviewed | Matched byte-publication readiness, four arms, 30 pairs per case across twelve cases. All ten warm cases pass HTTP-body/time gates; sparse median body savings are 93.75–99.997% and raw-to-sparse elapsed ratios 5.59–8.77×. All 2,640 operations exact. Both cold cases pass byte/p95 gates, but sparse cold fallback loses to ordinary compression on structured data. Chunked zstd also uses less CPU on the warm structured case. Fifty-nine of sixty captures complete; middle shard 1 retains tuple-reuse ambiguity and no whole-case IP statistic. | Measured source `9dce4c5`; [final report](broad-wan-results-2026-10-29.md), [raw evidence and custody correction](evidence/2026-10-29-broad-wan/README.md), [registration](../plans/2026-10-29-broader-wan-and-scaling.md). |
| SC-OPT-006 | Implemented opt-in; local byte-selection gate passed; extra CPU retained | Pre-Begin selection among Raw/chunked zstd/sparse/prefix matches the smallest measured fixed encoding in all 240 paired inputs. All 1,800 operations exact at 1/16 MiB. Cold structured bodies fall about 93% versus Raw; warm localized updates remain 600 bytes. At 16 MiB, exhaustive selection raises median CPU from 51.455 ms forced sparse to 86.402 ms; no general CPU improvement or new WAN claim. | Runtime `850272f`; registration `791ec79`. [Full local result](codec-selection-results-2026-10-29.md), [evidence](evidence/2026-10-29-codec-selection/README.md), [decision](codec-selection-decision.md), [measurement contract](codec-selection-measurement.md). |
| SC-OPT-007 | Exactness/memory passed; CPU gates failed; candidate reverted | 70 old/new pairs preserve encoded output at 16 MiB. Every primary diagnostic allocation pair saves at least 99.85% frame-buffer peak, but middle insertion CPU ratio 1.092043 misses 1.10, and repeat/periodic paired CPU regress 6.816%/6.401%, above 5%. Prefix and unrelated CPU gates pass. All negatives retained; active encoder restored to `cc0199f`. | Candidate `317c7f7`; registration `5bd7c51`. [Full result](sparse-pruning-results-2026-10-30.md), [raw evidence and archived candidate](evidence/2026-10-30-sparse-pruning/README.md), [measurement contract](sparse-pruning-measurement.md). Local encoder/diagnostic only, no new WAN or energy claim. |
| SC-OPT-008 | Coupled runtime implemented; conditional byte win; CPU penalty retained | Actual ternary folding, Kuramoto/memory coupling and six CA rows now drive verified checkpoint replay plus exact residuals. 480 local and 72 real WAN operations reopen exactly. The 14,851,384-byte generated/edited workloads use 747/766 HTTP-body bytes, or 4,630/4,649 with seeding. Three-trial WAN paired median elapsed gains over raw are 1.349×/1.325×, but zstd is faster and runtime CPU is higher; unrelated content loses. | Runtime/local `b6627de`; corrected WAN launcher `35114da`, with identical Rust source. [Full result](shared-runtime-results-2026-10-30.md), [reference and measurements](evidence/2026-10-30-shared-runtime/README.md), [registered contract](evidence/2026-10-30-shared-runtime/measurement-contract.md). ARM64/macOS and x86-64/Linux fixtures agree. No energy, complete-wire-byte, MVM or full 30-trial WAN acceptance claim. |
| SC-OPT-010 | Implemented; local exact-frame and CPU comparison complete | Runtime-first selection uses proved lower bounds; bulk residual scans and immutable hash reuse preserve winners and frame bytes. All 420 targets and 420 seeds reopen exactly. Large prepared generated CPU 111.437→35.039 ms, elapsed 127.094→49.678 ms, sender CPU 81.552→1.640 ms; body remains 747 bytes. Edited body remains 766 bytes. Unrelated prepared CPU median regresses 63.122→67.137 ms; cold and small-case limits retained. | Candidate `d13db98`; baseline `11fb809`. [Full result](sender-cpu-results-2026-10-30.md), [raw observations](evidence/2026-10-30-sender-cpu/paired.jsonl), [contract](evidence/2026-10-30-sender-cpu/measurement-contract.md). No new WAN, energy or MVM claim. |

## Measured object sizes

SC-WAN-004 validates the compressed-schema sender optimization over real WAN
transfers at source `2fcfb0dae9524c073f6aab8982736fb4b9242173`. All 1,800 targets
and 120 seeds reopen byte-exactly; all 660 old/new body/mode pairs agree. Thirty
paired matching ~1 MiB trial blocks show 31.67% less sender CPU, 9.98% less combined
measured CPU and 11.19% less elapsed than the old encoder. Against the existing
chunked-zstd profile, matching schema sends 2.28% fewer body bytes but consumes
48.37% more combined CPU; single-transfer elapsed medians are nearly equal.
Paid ten-target mixed sessions remain slower than zstd. Small and drift losses,
the cancelled pretransfer launch, scope limits and all raw rows are retained in
the [WAN result](schema-wan-results-2026-10-30.md) and
[evidence](evidence/2026-10-30-schema-wan/README.md). This is a protocol-body,
CPU and durable-time comparison, not complete-path byte or energy qualification.

SC-OPT-013 reduces repeated layout parsing, allocation and hashing with an exact
first-template match, preserving the complete SCVAL stream and zstd frames.
Candidate `983bf181adda8dc922d0187e53652717d61d38b6` versus baseline harness
`4dacb487cd2b3b132aa6bbf015eb0e5f893e69e0` passed the registered sender CPU goal:
the geometric mean of ten paired normal-batch session ratios was 0.679367,
or 32.06% less sender CPU. All 800 targets and 40 seeds matched immediately and
after reopening, with identical baseline/candidate protocol transcripts, modes
and body costs. Matching ~1 MiB medians: sender CPU 7.570→5.172 ms, total CPU
11.8645→9.5525 ms, durable elapsed 19.841→16.989 ms, body unchanged at 127,557.5
bytes. Candidate zstd used 130,450.5 bytes, 4.045 ms CPU and 11.976 ms elapsed.
Zstd control timing variation and drift/fallback costs remain in the
[full result](schema-sender-results-2026-10-30.md) and
[recorded evidence](evidence/2026-10-30-schema-sender/README.md).
This is a paired local persistent-receiver result, not new WAN or energy evidence.

SC-OPT-012 compresses the exact schema value/residual stream and adds prepared
sender contexts plus an opt-in, bounded receiver codec cache. The corrected
campaign at `fa0b0abdde0200c9faf62701daa28f9727cc62b4` completed 1,600 target
transfers and 120 seeds in 160 persistent sessions with ten distinct targets per
receiver. All 1,720
operations matched immediately and again after reopening. Matching ~1 MiB
batches shrink from 796,723 median body bytes with schema V1 to 127,557.5 with
compressed schema, versus 130,450.5 ordinary zstd. Prepared compressed schema
uses 11.789 ms median CPU and 20.1545 ms durable elapsed versus zstd's 3.846 ms
and 12.2465 ms. Preparation preserves body costs but provides no substantial
combined CPU gain in this campaign. Ten-send totals charge actual seeding and
preparation; small-message setup losses, drift and unrelated controls remain
visible. One failed launch before any transfer and a stale build-path pin caught
before execution are preserved with the [full result](schema-reuse-results-2026-10-30.md)
and [evidence index](evidence/2026-10-30-schema-reuse/README.md).
This is local serialized durable transfer, not new WAN, packet or energy evidence.

SC-OPT-011 adds frozen flat-JSON literal templates and genuinely trained-zstd
dictionaries at source `fe98bf0a40cad50f513652690548f7f0bb4f948f`. One held-out,
ten-trial comparison completed 400 targets and 240 seeds with exact reopened
bytes. Matching ~1 MiB batches use 796,771 median body bytes with schema fitting,
against 1,051,446.5 raw and 130,432 ordinary zstd; schema CPU is 15.677 ms against
4.009 ms raw and 5.978 ms zstd. Small normal records use 590 median body bytes
with the trained dictionary versus 667.5 zstd, before the 8,705-byte destination
seed. The template prototype is functional but is not a demonstrated improvement
over conventional compression. Fit, drift, fallback and actual sender reuse plus
all destination seeds are included in the [complete result](schema-fitting-results-2026-10-30.md)
and [raw measurements](evidence/2026-10-30-schema-fitting/paired.jsonl).
These are local serialized durable transfers, not a new WAN or energy result.

SC-OPT-009 implements packed CA evolution and immutable prepared sender predictions
at source `384ad3af2a41e87643bd0678120b854e5fe7244b`, compared with `d4532b0`.
The single ten-trial paired campaign completed 360 targets and 360 seeds with exact
reopened bytes and unchanged runtime body costs. Large standalone replay CPU fell
from 228.305 to 12.184 ms; generated transfer CPU fell from 559.601 to 139.071 ms
with packed replay and to 117.050 ms with prepared reuse. Preparation costs
19.228 ms CPU and retains 14,854,835 bytes, actually reused across three sends.
Raw/zstd/adaptive still use less CPU locally. See the
[full result](replay-cpu-results-2026-10-30.md) and
[raw observations](evidence/2026-10-30-replay-cpu/paired.jsonl).
The separate 36-operation baseline phase profile is retained; no extra WAN or
energy claim is made. Targeted validation logs are retained as formal evidence.

| Size | Executed evidence | Qualification boundary |
|---|---|---|
| 142,738 bytes and 14,851,384 bytes | SC-OPT-008: actual coupled-runtime trace sizes; ten paired local trials per case and three paired real-WAN trials per large case. All 552 seed/target operations exact after reopen. | [Runtime result](shared-runtime-results-2026-10-30.md). Conditional model-generated workload savings; local CPU/wall losses and WAN losses to zstd retained. |
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
  the evidence hashes. The SC-OPT-001 route/resolver files and SC-OPT-008 original
  test/formatting receipts retain these raw-output whitespace exceptions.
- Before advancing: independently review the code and evidence; preserve regressions and control
  failures; commit compact artifacts with hashes. Keep large local captures identified by hash.
- Never infer energy from CPU alone, network-only time by subtracting CPU from elapsed, or total
  path traffic from a single captured relay leg. CPU and wall time overlap.
- A green workflow is an execution status. Byte equality, durable commit, performance, packet
  completeness, security qualification and production readiness remain separate claims.

The original Python runtime and saved sessions are preserved in the primary checkout. New active
implementation and diagnostic helpers use Rust. Research modules remain references unless an
experiment explicitly integrates and measures them.
