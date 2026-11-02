# Unrelated-object CPU optimization in the shared gateway

The exact aligned-run scanner now skips isolated byte matches that cannot become
a COPY. Large opaque residual-search CPU fell about **74%** in the retained native
stage profile. In the same-run shaped-link gateway campaign, large unrelated
sender CPU improved **2.35%** by matched median, while its completion time was
**0.20% worse**. The complete mixed session used **1.32% less process CPU** by
matched median and was **0.35% slower**. This is a targeted CPU improvement,
**not a demonstrated shaped-WAN completion-time improvement**.

All **216 delivered objects reopened exactly**, and all before/after operation
parity checks passed. Both Combined versions use exactly **15,950,640 logical
channel bytes per mixed session**. The existing runtime/cache advantage over
prepared zstd remains: optimized Combined uses **58.26% fewer captured IP bytes,
41.93% less process CPU and 53.17% less elapsed time** on the shaped mixed workload.
Unrelated objects alone still lose to zstd; those results are retained below.

Measured candidate `c7ee11fdde26c922da9ae6d9574a91f88b2a3573`, baseline
`4a8ec79c9b185c85407e9767653dbcc4af97bf7e`.
[Implementation PR33](https://github.com/R4-Laboratories/SpiralCore/pull/33),
[run 41446807449](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41446807449),
[evidence and reproduction](evidence/2026-11-02-unrelated-gateway-cpu/README.md).

## Actual unrelated-object transfers

Median measured operation values; logical bytes count both channel directions.
Per-operation elapsed includes sender work, network exchange and durable commit;
checkpoint/schema setup and reopened readback are paid in the full-session table.

| Link | Object bytes | Arm | Logical bytes | Sender CPU | Completion elapsed |
|---|---:|---|---:|---:|---:|
| 80 ms /10 Mbps | 14,851,384 | Before | 14,883,962 | 362.910 ms | 12.966031 s |
| 80 ms /10 Mbps | 14,851,384 | After | 14,883,962 | 340.230 ms | 12.991936 s |
| 80 ms /10 Mbps | 14,851,384 | Whole-zstd | 14,883,962 | 273.597 ms | 12.916361 s |
| 80 ms /10 Mbps | 1,047,202 | Before | 1,049,988 | 29.375 ms | 1.249960 s |
| 80 ms /10 Mbps | 1,047,202 | After | 1,049,988 | 28.768 ms | 1.249906 s |
| 80 ms /10 Mbps | 1,047,202 | Whole-zstd | 1,049,988 | 25.881 ms | 1.070871 s |
| Unshaped local | 14,851,384 | Before | 14,883,962 | 197.611 ms | 267.337 ms |
| Unshaped local | 14,851,384 | After | 14,883,962 | 191.573 ms | 260.313 ms |
| Unshaped local | 14,851,384 | Whole-zstd | 14,883,962 | 147.807 ms | 216.729 ms |

Percentages use the median of same-trial ratios; they are not computed by dividing
the separate medians in the table. Large shaped sender CPU improved in all three
pairs, by 7.72%,2.35%,1.48%; elapsed improved in one and regressed in two. Small
shaped sender CPU improves 2.33% by paired median, with 0.11% worse elapsed. Large
unshaped unrelated CPU improves 3.06%,elapsed 1.12%; the smaller unshaped case
improves 11.04% CPU and 5.50% elapsed. These are descriptive three-trial outcomes.

Against zstd, large shaped unrelated data still costs **27.39% more sender CPU**
and **0.33% more elapsed** at identical bytes. The small shaped unrelated case
costs 14.81% more CPU and 16.71% more elapsed. Large unshaped unrelated data costs
29.29% more CPU and 19.90% more elapsed. Avoiding failed copy searches does not
remove the remaining full schema-compression candidate cost.

## Complete mixed persistent session

| Link | Arm | Captured IP bytes, both directions | Summed endpoint process CPU | Client elapsed |
|---|---|---:|---:|---:|
| 80 ms /10 Mbps | Before | 17,235,006 | 1.288825 s | 16.740083 s |
| 80 ms /10 Mbps | After | 17,235,137 | 1.290544 s | 16.777292 s |
| 80 ms /10 Mbps | Whole-zstd | 41,295,873 | 2.212616 s | 35.824993 s |
| Unshaped local | Before | 16,715,127 | 0.828217 s | 0.976193 s |
| Unshaped local | After | 16,714,951 | 0.808005 s | 0.923386 s |
| Unshaped local | Whole-zstd | 40,263,132 | 1.182657 s | 1.117460 s |

Shaped before/after process-CPU ratios are0.986832,1.007352,0.973503: their median
is1.32% lower even though the after arm's separate CPU median above is slightly
higher. That distinction is retained rather than choosing the more favorable
statistic. Shaped elapsed ratios are0.999136,1.004453,1.003539, giving0.35% worse
elapsed. Unshaped full-session CPU improves 2.44%,elapsed 5.46% versus Before.
Against zstd, unshaped After improves CPU 31.63%,elapsed 16.08% andIPbytes58.48%.

The codec sent no new savings relative to Before: all logical operation bytes
are identical. Minor captured-IP differences reflect transport behavior. Matching
large generated objects still use771logical bytes; residual and opaque edits use
1374 bytes; exact repeats use230 bytes under this channel's accounting. Their byte
advantages are preserved, not newly attributed to the scanner. A large opaque
edit improves sender CPU 6.77% and elapsed 3.99% against Before, while the small
opaque edit has 1.37% worse elapsed despite 3.37% less sender CPU. All trials remain.

## Native stage diagnosis

The unchanged Rust profiling helper ran18rows before and 18after on identical
fixtures, with exact reconstruction and output-length agreement. These are
sequential native diagnostics with prepared contexts, not additive transfer costs
or a substitute for the separate Linux campaign.

| Opaque fixture | Stage | CPU before → after | Paired CPU reduction |
|---|---|---:|---:|
| 1,047,202 bytes | Runtime residual | 183→33 µs | 80.37% |
| 1,047,202 bytes | Sparse residual | 175→26 µs | 83.85% |
| 14,851,384 bytes | Runtime residual | 2289→602 µs | 73.50% |
| 14,851,384 bytes | Sparse residual | 2273→591 µs | 74.00% |

The before profile has 39110 µs large-opaque schema-zstd CPU and 18098 µs Whole CPU;
full schema remains the larger avoidable-work research target. There is no sound
cheap general exclusion for that candidate in this implementation. The local
profile also retains large-generated Sparse 567→652 µs, large residual
Runtime 1093→1145 µs and Sparse 1235→1247 µs. Unchanged Whole opaque paired CPU rose 4.8%,
showing run-to-run spread. No selective rerun or observation removal was used.

## Change and exactness

The aligned delta encoder emits COPY only for equal runs of at least 32 bytes.
Previously it visited isolated equal bytes that could never become a COPY.
The new search probes an anchor every 32positions; every eligible equal run must
contain one. Equal anchors expand backward by at most31 bytes and forward to the
same maximal run. A short run restarts after its known mismatch. This is exact
coverage, not probabilistic sampling or an entropy heuristic.

Only `delta.rs` production code changed. Prefix/suffix candidates, literal
boundaries, strict cost ties, operation caps and packed bytes are preserved.
Whole-zstd, schema evaluation, runtime replay, hashes, authorization, wire modes
and receiver publication/ACK behavior are unchanged. Full schema compression
remains necessary when no sound exclusion can establish that it loses.

An independent scalar run oracle and frozen4a8 encoder compare exact frames,
operation counts and reconstruction across anchors,31/32/33-byte boundaries,
unequal lengths, append/truncate, long controls and the operation ceiling.
Focused 2/2 tests passed; the complete scoped run passed 72 tests, including those
focused tests, with one preexisting diagnostic ignored. Final local gateway-core
15 tests and gateway 42 tests, plus strict Clippy, passed.
The separately recorded Linux checks run two additional platform-specific tests.

## Workload and accounting

Candidate `c7ee11fdde26c922da9ae6d9574a91f88b2a3573` is compared with pinned
`4a8ec79c9b185c85407e9767653dbcc4af97bf7e` and current unchanged Whole-zstd.
Both service versions are built on the same runner with Rust 1.97.1 release settings;
all compilation finishes before timing. Encoder/probe/dictionary sources and
lockfiles are identical for the Whole control. Three complete arm-order rotations
use identical public fixtures and authenticated reliable QUIC.

Each persistent receiver gets six objects at each of two sizes:1,047,202 bytes
and 14,851,384 bytes. Sequence: generated runtime output,512-byte residual edit,
repeat,unrelated opaque data,localized opaque edit,repeat. Both Combined arms pay
checkpoint/schema publication and preparation; all arms charge input reads,
hashing,durable publication and reopened receiver readback. Whole uses the same
verified-cache probe. Complete dose:18sessions,264operations,216deliveries and
1,717,047,288requested bytes.

Logical channel bytes count both directions including batch framing but exclude
QUIC/IP overhead. Captured IP totals include both directions,ACKs,retransmissions,
and connection setup/close. They are measured per session, not invented per
object. CPU is recorded independently of elapsed and overlaps it; no subtraction
creates an isolated send-only time. Nested handler/operation CPU is not added to
whole-process CPU. Per-object times exclude setup charged separately in complete
sessions. Matched-ratio medians differ from ratios of median times.

This is three descriptive trials through calibrated Linux network namespaces
on one hosted runner, not a new two-host Internet-WAN test. No p95, confidence
interval, physical-energy, transparent TUN runtime compression, or new Combined
MVM qualification is claimed. The existing TUN path was not modified or rerun in
this round; gateway integration tests execute on both local and Linux builds.

## Execution and custody

Retained directional calibration means are80.219–80.294 ms RTT and 9.609–9.795 Mbps.
Capture packet counts reconcile with zero capture drops; finite shaped queues
reported 1199drops, whose repair/retransmission cost stays in the results.
Host routes and frozen binaries remained unchanged. One registered campaign ran
without retry. The analysis initially rejected the new schema because root
invoked a stale release binary directly; rebuilding through pinned-source Cargo
resolved it. Both exit receipts remain. No traffic or source was changed to fix
that invocation error.

The original 27,080,829-byte ZIP, full captures and executables are in the existing
private HF dataset at a verified immutable revision. Git retains readable analysis,
source/binary hashes, reviews and one 280 KiB compact receipt bundle with 887 original
files. A fresh extraction verifies every file and reproduces byte-identical
analysis. Historical results remain unchanged.

The measured result supports keeping the exact search optimization, with its
modest CPU benefit and preserved bytes. It does not justify claiming that
unrelated transfer now beats zstd or that every workload becomes faster. Further
large gains require reducing full schema-candidate cost without a heuristic that
can discard a smaller valid encoding, or changing the explicit selection contract.
