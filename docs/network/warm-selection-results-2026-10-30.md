# SC-OPT-015: warm-object selection preserves the 600-byte path

**Automatic selection now includes verified-base sparse transfer and preserves
its byte savings.** In thirty paired real-WAN trials at each of 1 and 16 MiB,
all sixty selector updates chose Sparse and transferred 600 HTTP-body bytes.
All 180 actual base seeds and 180 updates were independently regenerated and
compared byte-for-byte after stopping and reopening the receivers. No transfer
failed, was retried or was discarded.

At 16 MiB, selector update medians were **51.178 ms measured endpoint CPU and
179.8485 ms completion**, versus **85.8375 ms and 1,840.930 ms** for equally
tuned whole-object zstd/Raw. At 1 MiB the selector also reduced median CPU and
completion versus that control, but it still spends more CPU than directly
forcing Sparse. Paying the initial seed and setup reduces byte savings to
about 50%; the small paid lifecycle uses more CPU than Whole. These limits
remain part of the result.

The [design](../specs/2026-10-30-warm-selection-design.md),
[measurement contract](evidence/2026-10-30-warm-selection/measurement-contract.md),
[raw packet](evidence/2026-10-30-warm-selection/README.md) and
[complete summary](evidence/2026-10-30-warm-selection/wan/summary.json)
retain source identities, every trial, distributions and selection decisions.

## Implementation and avoided work

`PreparedSelector::prepare_warm(target, base, schema_ready, warm_probe)` is an
additive API; the original Raw/Whole/Schema `prepare` remains available. The new
method compares Raw, Whole, Sparse and Schema in that tie order, evaluating the
existing sparse recipe early. It skips another encoder only when a proven
format-size lower bound, including any required schema seed, cannot beat the
incumbent. Evaluated exact costs and pruned lower bounds occupy separate fields.

The prepared plan owns the winning payload and offer. The client consumes that
plan without encoding or hashing it again. The target is hashed once; the base
is hashed only if Sparse wins. Receiver authorization, resource limits, wire
formats, reconstruction checks and durable publication remain unchanged.
Missing or revoked receiver state fails explicitly rather than causing a hidden
fallback or retry.

In the measured selector decisions, all sixty cold-schema candidates were
pruned: the target bound 547 plus actual artifact-seed cost 670 exceeds 600
bytes. All thirty 16 MiB Whole candidates were pruned because their structural
bound is 896 bytes. Whole was still evaluated for all thirty 1 MiB updates,
where its 536-byte bound cannot rule out a smaller result. This is the remaining
small-object selection cost. No unsafe inference from zstd destination-buffer
exhaustion is used, and a bound is never reported as an observed encoded size.

The forced Sparse arm is the unchanged direct path measured alongside the
selector, not an earlier run on a different route. The integration preserves
that path's bytes; it does not establish a new compression advantage over
Sparse itself or attribute these savings to the separate CA/ternary mechanisms.

## Real-WAN measurements

WAN source is `8bf2d85f6ece94013e66f7df9b6e1d6cddc8c43c`, with compiled sources
and workflow identical to local qualification commit
`389accd2be2f8913ff6995f0c79c8867577ec6c1`. First-attempt
[GitHub run 41144943338](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41144943338)
transferred from 23:33:21 to 23:38:47 UTC on October 30, 2026. One frozen Linux
sender connected to six bounded Mac receiver stores through Cloudflare Quick
Tunnels. All six permutations of the three arms occur five times per size,
in a registered seeded shuffle. Each shard holds five trials per size and sixty
transfers. No owned tests, builds, profiling or uploads overlapped measurement.

The fixtures are exactly 1,048,576 and 16,777,216 pseudorandom bytes; targets XOR
32 midpoint bytes. Each arm pays the same actual Raw base in a separate fresh
authorized namespace. Targets are absent and exact-repeat probing is disabled.
DAG warming is disabled in every arm. Whole uses prepared level-3/window-25,
long-distance-matching, single-thread zstd with checksum and content size, then
selects Raw when smaller. All sixty Whole updates selected Raw; the attempted
compression remains timed. Thus this is a valid incompressible-base localized
edit comparison, not evidence that zstd compresses every data family poorly.

Every selector session actually fits the frozen 512-record corpus, checks the
169-byte artifact and constructs contexts inside its setup timer. All sixty
fits are charged once to their actual paid lifecycle. Setup median CPU/wall is
1.0385/1.0405 ms at 1 MiB and 1.0465/1.0495 ms at 16 MiB. No fitted artifact or
base hash is treated as free, and no longer hypothetical reuse sequence is
substituted for this measured seed-plus-update session.

The table gives medians except the final p95 column. Body bytes include request
and response bodies. CPU subtotal adds sender-process CPU and disjoint
receiver-handler CPU once. Completion starts before preparation, candidate
computation and hashing, and ends after the verified durable acknowledgment.
Exchange elapsed is a nested wall interval, not pure wire time. Medians in
different columns need not add to the median of their sum.

| Object and method | Update body bytes | Sender CPU ms | Receiver-handler CPU ms | CPU subtotal ms | Exchange elapsed ms | Completion ms | Completion p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1 MiB forced Sparse | 600 | 1.6475 | 6.5345 | 8.1695 | 119.8795 | 120.9640 | 138.484 |
| 1 MiB automatic selector | 600 | 2.3960 | 5.9855 | 8.3280 | 116.2025 | 118.0010 | 199.533 |
| 1 MiB Whole/Raw | 1,051,338 | 3.0860 | 5.7310 | 8.8010 | 223.5980 | 225.2855 | 499.761 |
| 16 MiB forced Sparse | 600 | 17.8700 | 33.2415 | 51.2345 | 166.1340 | 183.3010 | 289.720 |
| 16 MiB automatic selector | 600 | 17.4835 | 33.4940 | 51.1780 | 162.9600 | 179.8485 | 259.004 |
| 16 MiB Whole/Raw | 16,814,026 | 36.5350 | 49.5885 | 85.8375 | 1,814.4080 | 1,840.9300 | 3,160.597 |

Selector update body savings versus Whole are **99.9429% and 99.9964%**.
Ratios of completion medians are **1.91x and 10.24x** in favor of the selector.
Those are cohort-median ratios, not medians of paired speedups. The small-object
ratio remains below the original proposed 2x time target.

Actual paired ratios below divide each selector result by its own matched
control; values below one favor the selector. Thirty pairs contribute to each
row, with no warmup deletion.

| Size, phase and comparison | Median paired sender CPU ratio | Median paired CPU subtotal ratio | CPU wins | Median paired completion ratio | Completion wins |
|---|---:|---:|---:|---:|---:|
| 1 MiB update / Whole | 0.771235 | 0.920124 | 21/30 | 0.509063 | 29/30 |
| 16 MiB update / Whole | 0.474451 | 0.590554 | 30/30 | 0.100558 | 30/30 |
| 1 MiB update / Sparse | 1.432347 | 1.061508 | 11/30 | 0.985265 | 16/30 |
| 16 MiB update / Sparse | 0.982773 | 0.998273 | 15/30 | 1.002941 | 15/30 |
| 1 MiB paid lifecycle / Whole | 1.032202 | 1.056483 | 10/30 | 0.720662 | 25/30 |
| 16 MiB paid lifecycle / Whole | 0.747101 | 0.785118 | 29/30 | 0.533143 | 30/30 |

The 1 MiB selector uses more sender CPU than forced Sparse in all thirty pairs:
its median paired ratio is 1.432347. At 16 MiB, where both competing encoders
are pruned, selector/Sparse CPU and completion are approximately equal; small
median differences do not prove that adding selection makes Sparse faster.
Shared-host and relay variation remain present.

One small update loses in completion: trial 22 took **279.669 ms selector versus
224.304 ms Whole**, despite the 600-byte update. Its exchange was 277.959 ms and
receiver-handler elapsed was 54.732 ms. These intervals do not identify the
cause of waiting. It remains included in every aggregate.

## Paid state and local qualification

The following lifecycles sum actual context setup/fitting, one Raw seed and one
update. No phase is replaced with a common modeled seed or omitted cost.

| Object and method | Lifecycle body bytes | CPU subtotal ms | Completion ms |
|---|---:|---:|---:|
| 1 MiB forced Sparse | 1,051,938 | 18.1085 | 541.5660 |
| 1 MiB automatic selector | 1,051,938 | 19.5425 | 514.8495 |
| 1 MiB Whole/Raw | 2,102,676 | 18.8375 | 662.1195 |
| 16 MiB forced Sparse | 16,814,626 | 136.2395 | 2,179.8380 |
| 16 MiB automatic selector | 16,814,626 | 134.0430 | 2,042.2540 |
| 16 MiB Whole/Raw | 33,628,052 | 168.3855 | 3,877.3570 |

Paid lifecycle byte savings are **49.9715% and 49.9982%**. The small selector
lifecycle is faster in median elapsed but uses more CPU than Whole. This is not
a universal win on both resources. The large lifecycle improves both medians;
all per-trial and setup-inclusive ratios remain in the summary.

Local qualification used three pairs per size through a separate receiver
process, with all 36 seeds/updates exact after reopening. Small target CPU/wall
medians were Sparse 3.962/24.660 ms, selector 5.290/20.490 ms and Whole
5.329/20.070 ms. Large values were 40.864/62.528, 43.072/70.737 and
64.095/85.027 ms respectively. Local timing has only three pairs and is not the
WAN evidence. It also preserves the selector overhead over forced Sparse.

## Verification, custody and limits

Mac and hosted Linux all-target release qualifications each passed **341 tests,
zero failures and two preexisting ignored tests across 43 targets**. Clippy and
release builds passed. Actual missing-API and workflow RED cases are retained.
Focused current-object coverage passes eighteen tests: selector module
192/195 lines, 20/20 functions and 327/358 regions; the new client path has
25/25 deduplicated regions. This is not branch/MC/DC or whole-repository coverage.
Procedural review precedes independent code/Rust and split-author security/infra
review. Final review checks the retained raw arithmetic without rerunning a
measurement campaign.

All **51 downloaded artifact members, 86 source hashes and six cross-platform
plans** match. The Linux sender SHA-256 is
`e95bca8d524a0926063fa1a09be92a820cd3a7700b50f63c7c0c9654301ee687`;
the Mac receiver is
`d5f8b407eb8d06e4ba1d8d60f163e29a7747ec04d2adefda4c067fc188968e70`.
[Custody](evidence/2026-10-30-warm-selection/wan/custody.json) binds executable,
artifact, observation, readback and summary identities. The receiver uses the
explicit hardware-SHA feature; the Linux sender uses default features. Every
arm uses the same build on each endpoint.

HTTP-body accounting excludes headers, TLS, TCP/IP, retransmissions and extra
relay hops. CPU excludes receiver socket/logging and relay work. Fixtures,
process startup, receipt hashes and independent final readback are outside
operation timers; actual schema fitting is explicitly inside setup timing.
CPU cannot be subtracted from exchange wall time to invent pure wire time.
This unshaped relay route and localized-edit corpus do not establish arbitrary
data compression, conventional-delta superiority, energy savings, the full WAN
acceptance matrix, MVM integration or CUI/FIPS deployment qualification.

The delivered API now lets callers select the existing warm-object mechanism
without losing its byte advantage. The remaining implementation opportunity is
to reduce small-object evaluation/setup costs while preserving exact minimum
body-cost selection; this result does not justify skipping a potentially
smaller candidate merely because Sparse is usually favorable.
