# SC-OPT-017: fitting and compressible fallback costs

**The WAN run preserves exact recovery, 600-byte warm random-data updates and
the 570-byte compressible Whole result.** Actual fitting/setup costs fall, but
the new selector does not improve every transfer metric: the 1 MiB random
update has higher paired CPU subtotal and completion time than the old selector.
The zero-filled update has no byte advantage over tuned Whole zstd. A misleading
sampling control also increases preparation CPU by 57.35% in the paired median.
This is a bounded setup/fallback optimization, not a general compression gain.

## Change and matched comparison

Fitting now recognizes an exact duplicate of the first learned layout without
rebuilding its parser/hash structures. It preserves input admission, comparison
work limits, first-seen ordering and every artifact byte. Separately, an eligible
Whole probe can be skipped when eight sampled windows match a short-period
pattern; the unchanged full encoder then runs. The screen only schedules work.
It cannot prune a candidate, supply a compression bound or change selected bytes.
Native codec settings, the source-qualified first-block certificate, receiver,
authorization, wire formats and durable verification are unchanged.

The native measurement source is `980d5cc08b01833bb4a86f4bb3f7962b42de5055`.
WAN source is `aa22837a2bbfc0ab9c4cee7d380a768cab768e16`, with all 92 frozen
source files unchanged.
The baseline uses the common final harness with only `schema.rs` and
`schema_selection.rs` overlaid from
`4bb79f976e0939f3ef8b48c3830b07086b98f280`. Both builds retain the enabled native
probe and identical Whole implementation. The third arm uses equally tuned
prepared whole-object zstd with Raw fallback. The
[contract](evidence/2026-10-31-selector-setup/measurement-contract.md) fixes the
source bindings, gates, fixtures and all timing exclusions.

Three cells use the same 32-byte midpoint XOR edit: pseudorandom 1 MiB,
zero-filled 1 MiB, and pseudorandom 16 MiB. Every arm pays its own actual Raw base
seed in an authorized namespace; the target is absent. Exact-repeat probes and
DAG warming are disabled. The receiver persists within each shard, but each
sender session makes one update. Each selector session actually generates the
512 training records, fits and checks the same 169-byte artifact, then constructs
its contexts. No shared fitting or sender-reuse amortization is claimed.

CPU is sender-process CPU plus disjoint receiver-handler CPU, counted once.
Completion starts before selection/encoding/hashing and ends at the durable ACK.
Exchange elapsed is nested and includes waiting and receiver work; it is not pure
wire time. Bytes cover both HTTP bodies, including reply metrics, but exclude
HTTP/TLS/IP headers, retransmissions and extra relay hops. Receiver socket/logging
and relay CPU are unmeasured. Input fixture generation, process startup, receipt
hashing and independent final readback are outside operation timers. Training
generation remains inside setup. Energy is unmeasured.

## Real-WAN results

First-attempt [GitHub run 41156972123](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41156972123)
succeeded. The Linux sender reached six persistent Mac stores through temporary
Cloudflare tunnels. Thirty paired trials per case use all six arm permutations
five times in the registered order. The existing verifier accepted all **540
seed/update objects**, **270 sessions**, **180 actual fits** and **zero discarded
observations** after stopping the receivers and reopening every object exactly.
All ninety old/new winning-frame pairs agree. No transfer retry or favorable
rerun was used. The 59 artifact members and 92 source hashes are bound in the
[custody receipt](evidence/2026-10-31-selector-setup/wan/artifact-custody.json).

Values are cohort medians in milliseconds, except body bytes and the p95 column.
Independently calculated medians need not add to the median of their sum.

| Cell / method | Update body B | Sender CPU ms | Receiver CPU ms | CPU subtotal ms | Exchange ms | Durable completion ms | Completion p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| Random 1 MiB / old selector | 600 | 3.1625 | 6.9280 | 9.9505 | 141.4875 | 143.9990 | 190.751 |
| Random 1 MiB / new selector | 600 | 3.1540 | 7.6665 | 10.5890 | 150.5020 | 153.0445 | 275.611 |
| Random 1 MiB / Whole→Raw | 1,051,338 | 4.9850 | 6.9035 | 11.8835 | 350.2020 | 353.2995 | 627.373 |
| Zero 1 MiB / old selector | 570 | 3.2005 | 6.3040 | 9.3135 | 148.7760 | 151.3500 | 281.597 |
| Zero 1 MiB / new selector | 570 | 3.0855 | 6.9540 | 9.9875 | 147.6625 | 150.0960 | 294.011 |
| Zero 1 MiB / Whole | 570 | 3.2330 | 6.3235 | 9.5430 | 145.0940 | 147.7060 | 222.353 |
| Random 16 MiB / old selector | 600 | 29.3570 | 38.4045 | 68.1150 | 194.9515 | 223.5655 | 328.812 |
| Random 16 MiB / new selector | 600 | 29.3630 | 34.7920 | 64.4450 | 184.0690 | 212.7585 | 346.903 |
| Random 16 MiB / Whole→Raw | 16,814,026 | 54.9240 | 42.9650 | 98.6910 | 1,856.4080 | 1,897.5690 | 2,830.866 |

The existing warm-base mechanism saves **99.9429%** and **99.9964%** of update
body bytes versus Whole on the random 1/16 MiB cells. This round adds no byte
reduction over the old selector. On the zero-filled cell all arms send 570 bytes;
there is no measured codec-byte advantage over Whole.

Paired ratios divide each new-selector observation by its own matched control.
Below one favors the new selector; parentheses retain the number of wins out of
thirty. Lifecycle means actual setup + Raw seed + update.

| Cell / phase | Control | Sender CPU ratio (wins) | CPU subtotal ratio (wins) | Completion ratio (wins) |
|---|---|---:|---:|---:|
| Random 1 MiB / update | Old selector | 1.002696 (13) | 1.076171 (9) | 1.094249 (7) |
| Random 1 MiB / update | Whole | 0.637656 (30) | 0.906931 (22) | 0.542758 (29) |
| Zero 1 MiB / update | Old selector | 0.965597 (23) | 1.022281 (13) | 1.002466 (15) |
| Zero 1 MiB / update | Whole | 0.953645 (26) | 1.008138 (14) | 0.996962 (16) |
| Random 16 MiB / update | Old selector | 1.003653 (12) | 0.953340 (19) | 0.964335 (20) |
| Random 16 MiB / update | Whole | 0.537549 (30) | 0.667275 (30) | 0.116279 (30) |
| Random 1 MiB / lifecycle | Old selector | 0.975062 (23) | 1.026838 (12) | 0.939507 (16) |
| Random 1 MiB / lifecycle | Whole | 0.943216 (27) | 1.007475 (14) | 0.699780 (22) |
| Zero 1 MiB / lifecycle | Old selector | 0.949755 (28) | 1.002058 (15) | 0.981045 (16) |
| Zero 1 MiB / lifecycle | Whole | 1.132308 (0) | 1.075402 (6) | 0.992752 (15) |
| Random 16 MiB / lifecycle | Old selector | 1.004029 (11) | 0.982092 (18) | 0.988251 (16) |
| Random 16 MiB / lifecycle | Whole | 0.759835 (30) | 0.805689 (29) | 0.549958 (30) |

The random 1 MiB update is effectively unchanged in sender CPU versus the old
selector, while its paired CPU subtotal rises 7.62% and completion rises 9.42%.
The zero-filled case lowers paired sender CPU 3.44% but has no clear completion
improvement over the old selector or Whole. Its paid sender lifecycle costs
13.23% more CPU than Whole, with no sender-CPU wins. All three new-selector update
p95 values exceed the old selector's. The 16 MiB sender path is unchanged because
static pruning already skips Whole; lower receiver/elapsed medians do not establish
a new large-object compute gain. Receiver and route variation remain possible
contributors, not measured causal explanations.

Actual WAN setup CPU medians and paid lifecycles make those costs explicit:

| Cell | Old setup CPU ms | New setup CPU ms | Whole setup CPU ms |
|---|---:|---:|---:|
| Random 1 MiB | 1.6370 | 1.2985 | 0.0100 |
| Zero 1 MiB | 1.6385 | 1.2925 | 0.0100 |
| Random 16 MiB | 1.6520 | 1.3055 | 0.0110 |

| Cell / method | Lifecycle body B | CPU subtotal ms | Completion ms |
|---|---:|---:|---:|
| Random 1 MiB / old selector | 1,051,938 | 23.4140 | 674.6845 |
| Random 1 MiB / new selector | 1,051,938 | 24.8425 | 581.5555 |
| Random 1 MiB / Whole | 2,102,676 | 23.9275 | 826.3090 |
| Zero 1 MiB / old selector | 1,051,908 | 22.7690 | 612.1685 |
| Zero 1 MiB / new selector | 1,051,908 | 23.3670 | 629.7435 |
| Zero 1 MiB / Whole | 1,051,908 | 22.5120 | 583.9240 |
| Random 16 MiB / old selector | 16,814,626 | 162.7220 | 2,307.9315 |
| Random 16 MiB / new selector | 16,814,626 | 159.5405 | 2,320.2170 |
| Random 16 MiB / Whole | 33,628,052 | 193.6585 | 3,800.6935 |

Whole is seeded equally for this prior-state experiment although its codec does
not require the base. These are measured paid warm-object lifecycles, not optimal
cold-start comparisons. Setup-inclusive random-cell body savings are approximately
50%, rather than the greater-than-99.9% update-only savings. The complete
[WAN summary](evidence/2026-10-31-selector-setup/wan/summary.json) retains all
ratios, setup distributions and individual totals. A rejected preflight returned
HTTP 400 before measurement; receiver ordinals therefore begin at two for the
admitted campaign rows. That preflight is retained separately and is not counted
as a successful transfer or hidden retry.

## Local transfer results

All **180 seed/update objects** reopened byte-for-byte. All thirty old/new target
pairs preserve complete normalized winning request frames. Random updates remain
**600 bytes**; zero-filled updates use the same **570-byte Whole encoding** as the
Whole control. The ten paired trials per cell retain every observation.

These are loopback HTTP transfers with a separate receiver process. All
preregistered local gates pass:

| Gate: candidate / baseline | Median paired CPU ratio | Threshold | CPU wins |
|---|---:|---:|---:|
| Random 1 MiB actual setup | 0.772607 | ≤ 0.80 | 9/10 |
| Zero 1 MiB preparation-only control | 0.930573 | ≤ 0.95 | 7/10 |
| Random 1 MiB update sender | 0.959906 | ≤ 1.05 | 6/10 |

Lower CPU does not establish lower completion time here. Random 1 MiB update
completion has median paired ratio **1.064183** and only 3/10 wins; the zero-cell
ratio is **1.129580**, also 3/10 wins. The zero-cell candidate completion p95 is
96.026 ms versus 37.997 ms for the old selector. The large update path is unchanged
by the new probe screen because static pruning already applies; its different
timing medians do not establish a new large-object compute mechanism.

The complete [local summary](evidence/2026-10-31-selector-setup/local/summary.json)
retains all nine target and nine paid-lifecycle cells, all paired ratios, setup
distributions and p95 values. Lifecycle totals charge all sixty actual fits;
they do not replace paid setup with hypothetical reuse.

## Setup attribution and negative controls

The separate twenty-iteration stage profiles include their first observations.
They were collected sequentially and are descriptive attribution, not the paired
acceptance gate. CPU medians in microseconds are:

| Actual setup stage | Baseline | Candidate |
|---|---:|---:|
| Training generation and references | 1,152.5 | 1,153.0 |
| Fit and serialization | 441.5 | 186.5 |
| Artifact check | 0.5 | 0.0 |
| Context construction | 7.0 | 6.0 |
| Temporary cleanup | 9.0 | 8.0 |
| Entire setup interval | 1,627.0 | 1,365.5 |

Zero-microsecond entries reflect timer resolution. Independently calculated
medians are not additive. Training generation remains the largest measured
setup component; it was not changed or omitted. Production
`PreparedSelector::new` accepts an artifact and does not perform fitting. The
paired transfer campaign's random-1-MiB setup medians are 2.1320/1.7010 ms; these
are a different cohort from the stage profile.

The three preparation-only controls each retain ten old/new pairs and forty
exactly reopened base/target objects:

| 1 MiB control | Old prepare CPU ms | New prepare CPU ms | Median paired ratio | Interpretation |
|---|---:|---:|---:|---|
| Zero-filled | 2.2475 | 2.1225 | 0.930573 | Same 570-byte Whole result; 3/10 CPU losses retained |
| Repeated 64-byte pattern | 1.5740 | 1.5655 | 0.984311 | Short-period screen does not match this pattern |
| Repeated sampled windows, random gaps | 1.7070 | 2.6880 | 1.573497 | Misleading screen skips a useful probe; all 10 pairs use more CPU |

The last case is the scheduler's explicit limitation: sampled repetition is not
proof of compressibility. It can cause the full losing encoder to run, while
still preserving the exact winning bytes. These controls time `prepare_warm`
only; actual setup/seed and subsequent serialization/reopen are recorded
separately. See [all control results](evidence/2026-10-31-selector-setup/controls/summary.json).

## Evidence and reproduction

The [packet index](evidence/2026-10-31-selector-setup/README.md) links the source
pins, preserved RED/failed-build/GREEN logs, stage profiles, native receipts and
accepted WAN packet. Qualification records 373 passing all-target tests, two
historical ignored tests, no failures, 69 focused production/coverage tests, and
passing Clippy. Coverage is bounded changed-path evidence, not proof of every
input or a substitute for the exact transfer checks. No new timing experiment
or favorable rerun is implied by this report.
