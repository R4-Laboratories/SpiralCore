# Exact transfer efficiency: implementation and real WAN measurements

The direct canonical builder, immutable prepared graphs, bounded receiver graph retention and
versioned sparse COPY/LITERAL transfer are implemented. On the tested warm 1 MiB workloads, the
sparse path reduced application-protocol bytes by 99.80–99.98% and reduced measured completion time
by 1.83–2.58 times against full raw resend. The completion timer includes preparation, encoding,
network exchanges, receiver verification and durable publication. These are conditional reuse
results on public synthetic data, not arbitrary-file compression or an energy-saving claim.

## Source and experiment identity

- Implementation and both WAN campaigns: `b5520171ab1cfbc988e16b2b79f41ab831d3668a`.
- Previous measured implementation: `354b616257eda62cd6bdcc2fb37902fc4cb4a5e8`.
- Final paired builder diagnostic: `f016b26e6a8ad83be459814c296be42b8aed2ea6`.
- [Warm campaign, run 40884300401](https://github.com/R4-Laboratories/SpiralCore/actions/runs/40884300401): 900 successful operations, 30 paired trials per arm, 943,718,400 bytes independently compared after reopening the receiver cache.
- [Prefix campaign, run 40886610208](https://github.com/R4-Laboratories/SpiralCore/actions/runs/40886610208): 240 successful operations, 30 paired trials per arm, 251,658,360 bytes independently compared after reopening the receiver cache.

Both campaigns ran from a GitHub Linux runner over the public Internet through Cloudflare to the
Mac receiver. They used separate fresh caches and temporary capabilities. TLS terminates at the
relay; this is the bounded synthetic-data experiment in [the WAN instructions](wan.md), not the
future MVM encrypted-envelope design. No paid VM was provisioned. The owned receivers and tunnels
were stopped, the temporary Actions secret/variables removed and the local capability files deleted.

The two campaigns have different run IDs, schedules and cache populations. Compare paired arms
within a workload; do not interpret differences between campaigns as properties of the mutation
alone. Complete receipts, plan, source/binary hashes, capture counters and verification results are
retained in [warm evidence](evidence/2026-10-29-efficiency/warm/) and
[prefix evidence](evidence/2026-10-29-efficiency/prefix/).

## What changed computationally

1. The SHA-256 ternary graph is built directly. The old path built the legacy graph, reconstructed
   it, decoded its records, then converted and hashed the active representation. Canonical v1
   manifests, identities, node encodings and padding remain byte-compatible.
2. A bounded, fold-local memo reuses exact repeated nodes. A slot hit requires complete Node
   equality; the slot function is not an identity. The immutable canonical SHA-256 remains the
   identity authority. This also repairs the repetitive-data regression in the first direct builder.
3. Prepared senders retain immutable verified state and compute changed leaves/ancestors.
   Receiver graphs are scoped by the complete authorization namespace, bounded within the existing
   resident budget and evictable when unpinned. Disk integrity and authorization are rechecked.
4. Exact repeats use the ordinary object-identity hit path without rebuilding a graph.
5. Sparse mode selects a bounded sequence of COPY ranges from a verified base and literal bytes.
   It compares aligned-run and prefix/suffix recipes, with raw fallback when a recipe is not smaller.
   A one-byte prefix insertion can reuse the entire old object despite shifted chunk boundaries.

The [sparse contract](sparse-v1.md) explicitly versions mode 3. Offsets, lengths, operation counts,
frame sizes, reconstructed length and whole-object SHA-256 are checked before durable publication.
There is no executable recipe, weakened identity or silent switch of a live transfer's descriptor.
Missing bases are explicit failures that require a fresh attempt. The CLI supports `--mode sparse
--base BASE`; Rust callers also have prepared and sparse batched APIs.

Whole-object comparison/hash, materialization and durable output remain linear work. Prepared state
does not make arbitrary unknown edits O(changes). Caller-owned preparation allocations are outside
the endpoint's resident limit, and retained byte charges are not peak process-memory measurements.

## Actual elapsed time and CPU

All medians use the lower-middle nearest-rank convention of the retained summary. Speedup is the
geometric mean of 30 matched raw/candidate elapsed ratios, not the ratio of the displayed medians.
Every trial is retained; no statistical population confidence interval is claimed.

| Warm workload | Full raw median ms | Sparse median ms | Raw / sparse paired speedup | Sparse p95 ms | Sender CPU median ms | Receiver handler CPU median ms |
|---|---:|---:|---:|---:|---:|---:|
| Exact repeat | 327.478 | 152.675 | 2.3819× | 297.179 | 2.505 | 6.943 |
| Localized 32-byte overwrite | 315.017 | 180.530 | 1.8347× | 281.390 | 4.164 | 32.733 |
| Scattered 32×32-byte overwrites | 382.994 | 213.299 | 2.1381× | 261.248 | 3.761 | 48.032 |
| One-byte prefix insertion | 546.781 | 205.751 | 2.5809× | 300.928 | 3.013 | 16.035 |

CPU is already included in elapsed time. Adding CPU seconds to wall-clock seconds double-counts
work. Sender CPU covers the measured operation, HTTP client setup/cleanup and prepared-state
cleanup. Receiver CPU covers batch decoding, endpoint handling and reply serialization; socket
parsing/authentication before the handler, socket writes, subsequent buffer destruction, Cloudflare
and the tunnel process are outside that per-operation CPU counter. Synthetic fixture generation is
outside transfer timing and recorded separately. The independent reopened-cache byte comparison is
also outside timing; the receiver's reconstruction, length/digest verification and durable publication
are inside. Whole-process snapshots and tunnel counters are retained separately. There is no
measured joule total.

For localized changes, the median of each trial's sender-plus-receiver CPU sum fell from **87.856 ms
to 36.724 ms**, with a paired CPU ratio of **2.4979×**. For scattered changes it fell from **105.516 ms
to 51.665 ms**, with a paired ratio of **2.0441×**. These sums are computed before taking a median;
they are not sums of independently selected marginal medians.

Prefix insertion's CPU sum fell from **67.936 ms to 18.863 ms**, a paired **3.9493×** reduction. Sparse
won measured combined CPU in all 30 pairs of each mutation case; it won elapsed time in 29/30
localized, 29/30 scattered and 30/30 prefix pairs. Prefix sparse also beat the faster zstd full-copy
control by 2.259× paired elapsed time. These counters combine two different machines and omit some
surrounding transport work; they measure CPU work, not whole-system energy.

The present raw/zstd publication path synchronously constructs and optionally retains a DAG;
sparse publication skips optional DAG warming. Both verify the complete object and publish durably,
but they have different subsequent DAG readiness. Thus the comparison is against SpiralCore's
current raw implementation, not a lower-bound implementation of raw byte copy. Some of the CPU
advantage is deferred optional graph work; future DAG use may incur that work. Sparse-to-sparse
reuse does not require a DAG.

The localized workload misses the proposed 2× elapsed target despite its large byte reduction.
Exact-repeat performance is shared with conventional content-addressed caching: the raw-CAS arm
used the same 238 body bytes and had a 157.501 ms median. Do not attribute that saving to a novel
geometric mechanism.

## Bytes and the meaning of send time

These counts include request and response HTTP bodies, protocol framing, metadata and receipts.
They exclude HTTP headers, TLS, TCP/IP, retransmissions and the relay's second leg.

| Warm workload | Full raw body bytes, median | Sparse body bytes, median | Median body reduction | Median body serialization at an assumed 10 Mbps |
|---|---:|---:|---:|---:|
| Exact repeat | 1,051,338 | 238 | 99.9774% | 0.1904 ms |
| Localized overwrite | 1,051,338 | 600 | 99.9429% | 0.4800 ms |
| Scattered overwrites | 1,051,338 | 2,137 | 99.7967% | 1.7096 ms |
| Prefix insertion | 1,051,339 | 556 | 99.9471% | 0.4448 ms |

Prefix sparse used 556 bytes in 29 trials and 569 bytes in trial 07; the table reports medians.

The serialization column is the illustrative calculation `8 × body_bytes / 10,000,000`, treating
both directions as one aggregate byte budget. Full raw's corresponding budget is about 841 ms.
This is not the measured bandwidth or an emulation of this WAN path. A duplex link, packet overhead,
latency, pacing and congestion alter actual delivery time. At 1 Gbps those serialization budgets are
100 times smaller; round trips and endpoint work can then dominate much more strongly.

The harness does not separately identify a pure network-only duration with all CPU excluded.
Subtracting CPU from wall time cannot establish one: remote work, I/O waiting, scheduling and
network exchanges do not form a simple additive CPU/wire decomposition. Packet timestamps also
span receiver processing. The measured quantity suitable for a user-visible speed claim is elapsed
time through durable commit; byte counts establish the bandwidth saving independently.

## Alternative paths and negative results

| Workload | Encoding | Median body bytes | Median elapsed ms | Sender CPU median ms | Receiver CPU median ms |
|---|---|---:|---:|---:|---:|
| Localized overwrite | Full-fold DAG | 2,342 | 292.204 | 63.476 | 72.439 |
| Localized overwrite | Prepared DAG | 2,342 | 238.365 | 25.762 | 69.800 |
| Localized overwrite | Sparse | 600 | 180.530 | 4.164 | 32.733 |
| Scattered overwrites | Full-fold DAG | 30,648 | 312.400 | 63.844 | 94.549 |
| Scattered overwrites | Prepared DAG | 30,648 | 284.043 | 28.614 | 89.303 |
| Scattered overwrites | Sparse | 2,137 | 213.299 | 3.761 | 48.032 |
| Prefix insertion | Full-fold DAG | 4,306,671 | 791.680 | 65.533 | 123.056 |
| Prefix insertion | Sparse | 556 | 205.751 | 3.013 | 16.035 |

Prepared state cuts sender CPU without increasing DAG mutation bytes, but its total measured
endpoint CPU still exceeds full raw on the localized/scattered workloads. Fixed-leaf DAG prefix
insertion remains an expansion and time regression. Sparse mode solves this tested alignment case;
its simple matcher is not a general optimal delta algorithm. Random synthetic data is incompressible
under the chunked zstd control, which fell back to the same body bytes as raw. This does not assess
zstd's advantage on compressible real files.

## Conventional delta control

A separate Rust helper ran the installed `/usr/bin/rsync` (openrsync protocol 29, rsync 2.6.9
compatible) locally on the exact WAN fixtures. It explicitly selected delta versus whole-file
operation and disabled timestamp-based skipping. Each arm received its own cold seed, followed by
the update. Thirty paired trials per case alternated first arm, 15 each way. All 360 campaign
seed/update receipts passed exact byte comparison, full ObjectId comparison and file/directory
synchronization; every ObjectId matched the corresponding WAN raw_full receipt. Four pilot rows
are retained separately and excluded. The helper's nine tests passed.

| Update workload | rsync delta reported bytes | rsync whole-file reported bytes | Delta wall median ms | Whole-file wall median ms | Delta aggregate CPU median ms | Whole-file aggregate CPU median ms |
|---|---:|---:|---:|---:|---:|---:|
| Localized overwrite | 11,396 | 1,048,836 | 27.279 | 21.891 | 26.911 | 13.730 |
| Scattered overwrites | 43,140 | 1,048,836 | 27.753 | 22.105 | 27.224 | 13.857 |
| Prefix insertion | 10,377 | 1,048,841 | 28.299 | 22.307 | 26.965 | 13.825 |

Conventional delta saved 98.913%, 95.887% and 99.011% of reported protocol bytes, respectively,
while taking more elapsed time and CPU in every paired update on this local transport. Including
seeding, byte savings were 49.457%, 47.943% and 49.505%. The timer includes process execution,
statistics validation, exact comparison/hash, file/directory sync and buffer teardown. CPU sums
helper SELF and waited CHILDREN user/system time; it has no endpoint split. Source fixture creation
is outside update timing and warms source pages. OS caches were not flushed.

These are rsync-reported bidirectional process-transport counters, not HTTPS-body or captured-IP
counts. This establishes a conventional algorithm reference on identical data, not a matched WAN
head-to-head or a claim of SpiralCore novelty/superiority. The conventional control also uses the
usual midpoint median for even samples, unlike the WAN summary's lower-middle convention. Preserve
that distinction when reproducing tables. [Source, build identities, raw receipts and summary](evidence/2026-10-29-efficiency/rsync/).

## Seeding and amortization

For mutations, every arm separately transfers the base, then its update, so seed-plus-update totals
compare equal lifecycle work. Prepared-base construction occurs inside the seed timer. A fresh
sparse base seed costs 1,051,338 body bytes; it is not free shared state.

| Workload | Raw seed + update bytes, median | Sparse seed + update bytes, median | Raw cumulative median ms | Sparse cumulative median ms | Paired cumulative elapsed speedup |
|---|---:|---:|---:|---:|---:|
| Localized overwrite | 2,102,676 | 1,051,938 | 674.648 | 588.326 | 1.2034× |
| Scattered overwrites | 2,102,676 | 1,053,475 | 938.157 | 576.998 | 1.4333× |
| Prefix insertion | 2,102,677 | 1,051,894 | 973.787 | 611.686 | 1.5048× |

This first seed-plus-update saves about 50% of body bytes, not 99.9%. Continued warm reuse can
amortize the seed further, but this campaign measures one update per retained base, not a long
successor chain. Prepared successor promotion is implemented/tested but deliberately not exercised
or timed here. Optional graph preparation after a sparse update is not hidden in these totals; it
was not performed.

For exact repeats, raw_full has no seed phase: its task is one full resend. The summary's automatic
seed-plus-repeat ratios therefore compare one raw send with a candidate seed plus a repeat. They
are unequal event counts and must not be used as a lifecycle speedup comparison. Report the warm
repeat and its initial seed separately.

## Paired builder diagnostic

Ten balanced old/new release process pairs (five AB, five BA) ran on the Mac. Each process reports
the median of seven internal samples; this is ten independent pairs, not seventy independent trials.
Five fixture descriptions matched in every pair and the probe checks exact reconstruction before
timing. Background load and OS caches were uncontrolled, so all observations and noisy controls are
retained in [the builder report](evidence/2026-10-29-efficiency/builder-final-paired.json).

| Full graph construction | Old CPU median ms | New CPU median ms | Paired CPU speedup | Pairs favoring new |
|---|---:|---:|---:|---:|
| Random 1 MiB | 99.260 | 41.397 | 2.3898× | 10/10 |
| Localized-change 1 MiB | 98.721 | 41.557 | 2.4230× | 10/10 |
| Periodic 64-byte pattern, 1 MiB | 17.638 | 3.833 | 4.6262× | 10/10 |

The first direct builder regressed periodic-data CPU from 17.467 to 28.933 ms; the bounded exact-node
memo corrected that mechanism before the final run. The initial pair 0 overlapped review tests and
is preserved but excluded; initial pairs 1–10 were used. All final pairs 0–9 are included. Unchanged
whole-object SHA control had a 0.9796× CPU ratio with only 4/10 pairs favoring new. Other unchanged
operations were noisy or slower; this is not a claim that every library operation became faster.
Hardware-SHA feature flags were unchanged. No GPU or parallel compute was added.

## Packet accounting and remaining qualification

The warm campaign observed 663,651,781 IP bytes on the runner-to-relay connection. It is incomplete:
26 ambiguous/reused TCP connections quarantine 36,474,756 IP bytes and some operation rows lack
unambiguous packet ownership. Captured/received counts match and kernel drops are zero, but those
facts do not resolve attribution. There is no complete per-transfer IP comparison for warm cases.

The prefix campaign passed the analyzer's current completeness checks: 246,122 captured, received
and PCAP-record counts agree; zero reported kernel drops; zero ambiguous connections; zero
unattributed bytes. It observed 332,836,545 IP bytes across both directions. This supports packet
accounting only for the captured runner-to-Cloudflare leg. The relay-to-Mac leg and Cloudflare
internal work remain outside the measurement. Header snapshot length (96 bytes) is not packet size.

Independent parsing reproduced every one of the 240 prefix rows' upload/download byte and packet
counts. For the update phase, raw's median captured IP traffic was **1,099,936 bytes**, versus
**5,859 bytes** for sparse, including both directions, captured headers, TLS, ACKs and retransmissions
on that leg. The median paired reduction was **99.4672%**, and the paired geometric mean byte ratio
was **187.0713×**. With matching seed-plus-update, the captured-byte ratio was **1.9892×**. This is a
stronger measured traffic claim for prefix insertion than body counts alone, still bounded to one
leg. [Independent paired calculations](evidence/2026-10-29-efficiency/independent-efficiency-wan-summary.json).

The mandatory fidelity checks passed for all 1,140 operations. Full validation before these campaigns
passed 124 transfer tests, one compile-fail doctest and 71 object-store tests; the exact-byte example,
fmt, clippy with warnings denied and release builds passed. Transfer line coverage is 5,167/5,721
(90.316%), with test files and the imported crate excluded from that denominator. Branch coverage
was not instrumented; zero branch counters are not a coverage result.

Qualification remains open for a matched conventional-delta WAN baseline, complete traffic on both
relay legs, direct peers, controlled 80 ms/10 Mbps shaping, loss/jitter matrices, larger objects and
small messages, long update chains, peak resident memory, power/joules, three-peer recovery and MVM.
Cold/no-reuse sparse fallback has correctness tests but no new 30-trial cold performance campaign.
No CUI/FIPS or production security qualification follows from these experimental results.

Next profiling candidates are persistent authenticated connections, avoiding synchronous optional
DAG warming in the raw baseline, the whole-cache quota scan at admission/publication, and redundant
full-object hashing in DAG Finish. Their exact shares have not been isolated, so they are hypotheses
for the next controlled experiment, not promised speedups. Keep final length/digest, authorization,
shared budgets and durable-commit checks as correctness gates while reducing that work.
