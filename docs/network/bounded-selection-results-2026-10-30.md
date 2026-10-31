# SC-OPT-016: stop encoding a proven losing candidate

**The optimized selector preserves the 600-byte warm update and cuts 1 MiB WAN
sender CPU by 28.0% in the median paired comparison with the old selector.**
All 360 WAN objects reopen byte-for-byte; no transfer failed, was retried or was
discarded. Small-update paired completion improves 9.0%, with twenty of thirty
wins. The compressible fallback control costs more CPU, and small paid-lifecycle
completion does not improve versus the old selector; those negatives remain below.

This round reduces the remaining small-object warm-selector CPU without changing
the winning transfer. It adds a source-qualified first-block rejection bound for
Whole zstd, after the existing static pruning. The full encoder remains the
fallback whenever the bound is inconclusive. The receiver, formats, authorization,
exact reconstruction and durable publication are unchanged.

The [design](../specs/2026-10-30-bounded-selection-design.md),
[measurement contract](evidence/2026-10-30-bounded-selection/measurement-contract.md)
and [evidence packet](evidence/2026-10-30-bounded-selection/README.md) retain the
registered gate, exact implementation identities and all observations.

## Why the optimization is safe

On eligible objects, the sender gives zstd the complete input but collects only
the frame and first-block headers. The pinned implementation computes that first
block with adequate internal output capacity. The header supplies its actual
payload length; remaining mandatory block headers and checksum supply a lower
bound for the complete frame. Existing chunk/framing costs convert it into a
complete protocol-body bound. Whole is skipped only when this bound cannot beat
the current winner under the existing tie order.

Probe output is never transmitted, and rejection bounds are not labelled exact
encoded sizes. Unexpected header shape is inconclusive; resource, codec and
reset errors remain errors. An exact bundled-source hash guard and runtime version
check prevent applying this source-specific argument to a foreign implementation.
Unqualified builds retain the original full encode. A preliminary maximum-bound
check avoids a useless probe when even the strongest possible certificate cannot
exclude Whole. All candidate contexts retain their original allocation limits.

The registered random 1 MiB target yields a 131,879-byte body lower bound, already
greater than the 600-byte Sparse winner. The old selector fully compressed it.
At 16 MiB the existing static 896-byte bound already excludes Whole, so that size
is an unchanged-path control. No new compression gain is attributed to the probe;
the existing verified-base sparse mechanism supplies the byte savings.

## Matched experiment and timing scope

Qualified native source is `eccb104d8d5dbdf56ad2fdb37a920b0424edbceb`. WAN source
is `8e4b87329ea2a72ad5a408e39869bb36285befe8`, with all 91 frozen source files
unchanged. Baseline is the common current harness with exactly `zstd_whole.rs`
and `schema_selection.rs` overlaid from
`4234591e25b4ca87db6a4670bdf407f48a581575` and the probe disabled. It is explicitly
a production overlay, not a claim that the historical checkout contains the new
measurement commands. The candidate and baseline use the same compiler, manifest,
lockfile, target, features and harness on each endpoint.

The fixtures are 1,048,576 and 16,777,216 pseudorandom bytes, with 32 midpoint
bytes XORed by 0xa5. Every arm pays an actual Raw base seed in its own fresh scoped
store; the target is absent. Exact-repeat probing and DAG warming are disabled.
The third arm is equally tuned prepared whole-object zstd with Raw fallback.
Both selector arms actually fit the same 512-record training corpus and construct
their contexts inside setup timing; no free fitting or base state is assumed.

CPU adds sender-process CPU and disjoint receiver-handler CPU once. Completion
starts before selection, encoding and hashing and ends after the verified durable
ACK. Exchange elapsed is a nested interval containing receiver work and waiting,
not pure wire time. HTTP-body bytes count both directions but exclude HTTP/TLS/IP,
retransmissions and extra relay hops. Receiver socket/logging and relay CPU are not
measured. Fixture creation, process startup, receipt hashing and independent final
readback are outside operation timers. Paid lifecycles sum actual setup, seed and
update once; no hypothetical reuse sequence replaces the observed session.

## Real-WAN results

First-attempt [GitHub run 41153084110](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41153084110)
transferred from 01:02:53 to 01:07:20 UTC on October 31, 2026. The frozen Linux
sender reached six Mac stores through temporary Cloudflare Quick Tunnels. Thirty
paired trials per size use all six arm permutations five times each, in the
registered seeded order. No owned builds, tests, profiling or uploads overlapped
the timed campaign. The local and hosted binaries remain separately identified.

All 180 actual base seeds and 180 absent-target updates reopen exactly after
stopping the receivers. All sixty old/new winner-frame pairs agree, and every
one of the 120 selector updates costs 600 body bytes. The Whole control attempts
the matched level-3/window-25/LDM/checksum/content-size encoding, then selects Raw
on all sixty pseudorandom targets; attempted compression remains timed. Both
selectors pay their actual fitting/context preparation: 120 fits total.

The following are cohort medians, except the final p95 column. Independently
computed medians need not add to the median of their sum.

| Object and method | Update body bytes | Sender CPU ms | Receiver-handler CPU ms | CPU subtotal ms | Exchange elapsed ms | Completion ms | Completion p95 ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1 MiB old selector | 600 | 3.2840 | 7.0800 | 10.2425 | 71.2760 | 73.9480 | 86.942 |
| 1 MiB new selector | 600 | 2.3100 | 6.1895 | 8.7145 | 66.2630 | 67.9955 | 88.292 |
| 1 MiB Whole/Raw | 1,051,338 | 4.3475 | 5.6955 | 9.9405 | 151.1070 | 153.5535 | 262.005 |
| 16 MiB old selector | 600 | 23.5370 | 34.2985 | 57.8020 | 123.2495 | 146.0940 | 178.074 |
| 16 MiB new selector | 600 | 23.5650 | 33.8510 | 57.4050 | 118.6785 | 141.5105 | 215.374 |
| 16 MiB Whole/Raw | 16,814,026 | 51.9335 | 43.0955 | 94.4450 | 1,600.4675 | 1,638.8490 | 2,465.994 |

New-selector update body savings versus Whole are **99.9429% at 1 MiB** and
**99.9964% at 16 MiB**. Ratios of completion medians are **2.26x and 11.58x**.
These are conditional warm-state gains over the whole-object control, not new
byte savings over the old selector or a general data-compression result.

Paired ratios divide each candidate by its own matched control. Values below
one favor the new selector; counts retain every loss.

| Size, phase and comparison | Median paired sender CPU ratio | Sender wins | Median paired CPU subtotal ratio | CPU wins | Median paired completion ratio | Completion wins |
|---|---:|---:|---:|---:|---:|---:|
| 1 MiB update / old selector | 0.720097 | 30/30 | 0.873056 | 26/30 | 0.910351 | 20/30 |
| 16 MiB update / old selector | 1.001397 | 9/30 | 0.985551 | 17/30 | 0.983132 | 17/30 |
| 1 MiB update / Whole | 0.536652 | 30/30 | 0.864855 | 27/30 | 0.451879 | 30/30 |
| 16 MiB update / Whole | 0.454589 | 30/30 | 0.610078 | 30/30 | 0.084477 | 30/30 |
| 1 MiB paid lifecycle / old selector | 0.907731 | 30/30 | 0.983996 | 16/30 | 1.017745 | 13/30 |
| 16 MiB paid lifecycle / old selector | 1.006482 | 6/30 | 0.995078 | 15/30 | 0.973444 | 17/30 |
| 1 MiB paid lifecycle / Whole | 0.968876 | 25/30 | 1.002077 | 14/30 | 0.739467 | 28/30 |
| 16 MiB paid lifecycle / Whole | 0.733569 | 30/30 | 0.810951 | 29/30 | 0.541208 | 30/30 |

The small target's sender CPU reduction is consistent across all thirty pairs;
its median paired CPU subtotal improves 12.7%. The unchanged 16 MiB path has
approximately equal sender CPU; small cohort differences do not establish a new
large-object compute gain. Receiver-handler CPU and route waiting vary despite
identical receiver code and winning frames. Completion improvements cannot be
attributed entirely to the sender CPU reduction. Both candidate selector p95
update times exceed the old selector's in this run, and ten small-update elapsed
losses remain included. No confidence interval or isolated causal wall-time claim
is asserted.

Actual setup-plus-Raw-seed-plus-update totals are:

| Object and method | Lifecycle body bytes | CPU subtotal ms | Completion ms |
|---|---:|---:|---:|
| 1 MiB old selector | 1,051,938 | 22.7205 | 259.7855 |
| 1 MiB new selector | 1,051,938 | 21.5105 | 255.9215 |
| 1 MiB Whole/Raw | 2,102,676 | 21.7395 | 343.4415 |
| 16 MiB old selector | 16,814,626 | 151.3680 | 1,878.2325 |
| 16 MiB new selector | 16,814,626 | 147.5605 | 1,852.6010 |
| 16 MiB Whole/Raw | 33,628,052 | 180.6890 | 3,395.3190 |

Seeding reduces the measured lifecycle byte savings to **49.9715% and 49.9982%**.
Although the small paid lifecycle's cohort completion median is lower than the
old selector's, its median paired completion ratio is **1.017745**, with only
13/30 wins. Small lifecycle CPU versus Whole is also approximately tied and
slightly worse by the paired median (**1.002077**, 14/30 wins). These results do
not support a blanket improvement in every seed-inclusive resource metric.
The full [summary](evidence/2026-10-30-bounded-selection/wan/summary.json) retains
every ratio, setup/seed/update distribution and actual lifecycle.

## Local gate and fallback cost

All 120 registered local transfers reopened byte-for-byte. All twenty paired
selector updates preserve Sparse, 600 body bytes and complete normalized winning
request-frame identity. The median of ten paired 1 MiB candidate/baseline sender
CPU ratios is **0.600062**, a **40.0% reduction**, passing the preregistered 0.80
gate. Every ratio is below the threshold; none was discarded or rerun.

| Object and method | Update body bytes | Sender CPU ms | CPU subtotal ms | Completion ms |
|---|---:|---:|---:|---:|
| 1 MiB old selector | 600 | 2.9275 | 6.6470 | 22.3680 |
| 1 MiB new selector | 600 | 1.8080 | 5.4410 | 22.4365 |
| 1 MiB Whole/Raw | 1,051,338 | 2.8275 | 6.7645 | 22.5035 |
| 16 MiB old selector | 600 | 18.6150 | 44.8485 | 67.6190 |
| 16 MiB new selector | 600 | 20.6135 | 46.8550 | 72.3780 |
| 16 MiB Whole/Raw | 16,814,026 | 47.8400 | 78.4990 | 105.9080 |

These are cohort medians. Lower small sender CPU did not improve local median
completion, and the unchanged large path has slower candidate medians in this
run. The shared host is not an isolated timing appliance.

The separate ten-pair compressible control uses a zero-filled 1 MiB base with the
same midpoint edit. All forty base/target objects reopen exactly; all ten pairs
produce identical completed Whole receipts and **570-byte** winning transfers.
The probe is inconclusive, so the candidate pays both the probe and the original
full encode. Preparation CPU medians are **2.1595 ms old versus 2.2930 ms new**;
the median paired ratio is **1.101489**, or **10.15% more CPU**, with eight of ten
pairs slower. Preparation wall medians are 2.1985/2.3400 ms. Setup and Raw seeding
are recorded separately; actual serialization, sending and exact reopened checks
occur after the preparation timer. This negative is an encoding-cost control,
not a WAN timing measurement.

## Qualification and limits

Native and hosted Linux all-target release qualifications each passed 360 tests, zero failed and two
preexisting ignored across 46 targets. Clippy passes after four mechanical
test/harness lint fixes, followed by 39 passing focused tests. Both enabled and
disabled guard builds are exercised. Focused current-object coverage is 253/259
selector lines, 281/298 Whole lines and 47/49 guard-helper lines; it includes
embedded unit tests and is not branch, MC/DC or whole-repository coverage.
Retained qualification failures are separate from measured transfers.

Sequential procedural review and independent production code/Rust/security plus
cross-author harness/workflow review passed. The independent local arithmetic
review checks every distribution, paid lifecycle and frame pair without rerunning
the experiment. The first preflight shell command encountered zsh's reserved
`status` variable after an unauthenticated GET; the corrected preflight is recorded.
No transfer worker had started and no measurement was discarded.

[Custody](evidence/2026-10-30-bounded-selection/wan/custody.json) verifies the
downloaded ZIP against GitHub's digest, all 59 artifact members, 91 frozen source
hashes, six cross-platform plans, separate executable identities and 360 stopped
readbacks. Compact raw receipts remain in Git; frozen executables remain outside
Git and in the hosted artifact through its recorded November 30 expiration.
Generated payload stores and temporary WAN capabilities are cleaned only after
verification; see the separate cleanup receipt.

This localized-edit corpus does not establish universal compression, superiority
over a conventional delta baseline, energy savings, a shaped-WAN latency/loss
matrix, direct-peer transport, MVM integration or CUI/FIPS qualification. The
compressible control demonstrates a real tradeoff of the added rejection probe.
