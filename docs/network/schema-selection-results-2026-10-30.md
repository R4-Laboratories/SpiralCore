# SC-OPT-014: exact codec selection versus equally tuned whole-object zstd

**Implemented and measured over a real WAN. The selector preserves the smallest
eligible body representation and exact bytes, but does not outperform the tuned
whole-object zstd control on this corpus.** Across 600 selector targets it chose
whole-object zstd 480 times and Raw 120 times, never Schema. Its body bytes equal
the whole/Raw control in every paired target and every paid session. Evaluating
the unused schema adds sender CPU. This negative result supersedes any inference
that SC-WAN-004's small byte advantage over chunked level-1 zstd also holds
against equally configured whole-object zstd.

**This is codec-context reuse, not the earlier warm-object delta experiment.**
Every target is distinct, and this selector considers only Raw/Whole/Schema.
It does not offer an already verified object as a sparse or prefix base, reuse
its DAG, or supply a shared-runtime checkpoint/recipe. These results therefore
do not establish a regression of SpiralCore's warm-object transfer mechanism.
The retained [16 MiB warm-update WAN result](large-wan-results-2026-10-29.md)
measured 600 sparse body bytes and 219.650 ms median completion versus
16,814,026 bytes and 2,016.728 ms for a zstd full send; combined CPU medians
were 79.509 and 237.824 ms. Those are historical measurements with the explicit
baseline/readiness and capture limitations in that report, not a rerun with
the new whole-object zstd control. A successor comparison must enable verified
base reuse before drawing a conclusion about warm-cache performance.

The source is `58d71cf10a45fb9f1ad19ae2557135d53a1edc1d`. Public GitHub run
[41135473115](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41135473115)
completed its first attempt successfully. Actual transfers ran from
**22:06:22 through 22:12:17 UTC on 2026-10-30**, from a GitHub Ubuntu Linux sender
(four exposed AMD EPYC 7763 CPUs) through Cloudflare Quick Tunnels to the local
Apple M1 receiver. This is a real relayed WAN route, without controlled bandwidth,
delay or packet loss. The Mac is a shared desktop, not an isolated benchmark host.

The [registered contract](evidence/2026-10-30-schema-selection/measurement-contract.md),
[raw hosted observations](evidence/2026-10-30-schema-selection/wan/hosted/),
[validated summary](evidence/2026-10-30-schema-selection/wan/summary.json) and
[source/artifact custody](evidence/2026-10-30-schema-selection/wan/custody.json)
are retained. The three host-generated plans are byte-identical to the receiver
plans. All 36 downloaded artifact members and 81 source-file hashes verified.

**The comparison is now matched.** Both compressed schema and the new explicit
`ZstdWholeV1` mode use zstd level 3, window log 25, long-distance matching,
one compression thread, content size and checksum. Both reuse prepared sender
contexts and the same opt-in bounded receiver-decoder cache policy. The old
chunked level-1 `Mode::Zstd` behavior is unchanged and is not the control here.
All three arms use one Linux executable, source and transport.

The schema reference pays its 169-byte artifact seed before each session. The
selector encodes Raw/Whole/Schema candidates before Begin, charges the exact
670-byte protocol-body seed when the schema is absent, and would seed only for
a strict effective-byte win. It consumes the immutable winning payload without
re-encoding. All candidate CPU and hashing are timed. The decision is greedy
for this transfer, not an optimum over unknown future traffic.

Each size has **30 counterbalanced paired trials**, with ten distinct objects
sent to each persistent receiver scope: four matching records/batches, two
lexical/order variants, two schema-drift objects, and two unrelated binary
objects. Small objects are **210–331 bytes**; large objects are
**1,048,576–1,048,903 bytes**. The fixed SC012 schema was not retrained on the
held-out targets. There were **1,800 targets, 60 actual seeds, 180 context setups
and 180 sessions**. No selector seed was needed. No transfer was retried or
discarded. After stopping the receivers, **all 1,860 transferred objects matched
their regenerated originals exactly**; the 60 unseeded selector artifacts were
also confirmed absent, giving 1,920 independent presence/readback checks.

The following are medians of the thirty actual ten-object sessions per row.
All context construction and actual seeds are included once. Times are
milliseconds; each large session carries about 10 MiB of original data.

| Object size / method | Both-direction body bytes | Sender CPU ms | Measured CPU subtotal ms | Total completion ms |
|---|---:|---:|---:|---:|
| Small / forced schema | 7,909 | 11.0225 | 119.5325 | 1,542.560 |
| Small / whole zstd + Raw | 6,790 | 10.2005 | 121.0605 | 1,377.508 |
| Small / selector | 6,790 | 10.3460 | 110.3770 | 1,420.259 |
| ~1 MiB / forced schema | 3,079,510 | 91.9340 | 257.2210 | 2,302.9215 |
| ~1 MiB / whole zstd + Raw | 3,053,215.5 | 60.2335 | 204.4945 | 2,110.8950 |
| ~1 MiB / selector | 3,053,215.5 | 126.3195 | 251.9675 | 2,301.0540 |

Paired statistics use one ratio per trial, rather than treating the ten positions
as independent. For large sessions, selector/whole geometric mean ratios are
**1.0000 for bytes, 2.1028 for sender CPU, 1.3223 for the measured CPU subtotal,
and 1.0676 for total completion time**. The selector completed faster in only
12/30 pairs. Small-session ratios are 1.0000, 1.0201, 1.0187 and 1.0200,
respectively, with 9/30 completion-time wins. Thus the smaller unpaired small
CPU median in the table is not evidence of a paired CPU improvement.

Against forced schema, the selector reduces paid-session body bytes in all
thirty trials of each size: geometric mean reductions are **14.16% for small
objects and 0.857% for large objects**. Large-session sender CPU rises 37.44%;
total elapsed is effectively tied (ratio 0.99645, 14/30 wins). Whole/Raw obtains
the same byte reduction without the selector's extra encoding work.

For matching ~1 MiB objects alone, the following medians use 120 target operations
per arm. Context setup and the forced schema's upfront seed belong to the paid
session table above. There were no selector seeds in these measurements.

| Method | Body bytes | Sender CPU ms | Measured CPU subtotal ms | HTTP exchange ms | Encode-to-durable-ACK ms |
|---|---:|---:|---:|---:|---:|
| Forced schema | 127,548 | 7.8525 | 26.6875 | 174.4000 | 181.2705 |
| Whole-object zstd | 125,922 | 6.5890 | 20.8625 | 174.0700 | 179.7050 |
| Selector | 125,922 | 12.5515 | 24.7125 | 180.2480 | 191.7995 |

The large controls tell the same story. For variants, schema uses a median
131,951.5 body bytes versus 130,170 for whole/selector; drift uses 100,943.5
versus 93,166.5; unrelated binary data uses 1,051,430 in all three arms through
Raw fallback. Every paired selector/whole byte comparison is equal. The recorded
candidate receipts also show **zero schema target-only wins out of 600 even
after removing its seed charge arithmetically**. Its target-only excess is
56–120 bytes for small objects and 1,160–21,293 bytes for large objects. That
calculation is retained in
[candidate-cost-analysis.json](evidence/2026-10-30-schema-selection/wan/candidate-cost-analysis.json);
it is not an additional warm transfer experiment.

The complete large cohort, including setup, consumed:

| Method / 300 targets | Body bytes | Sender CPU seconds | Measured CPU subtotal seconds | Sum of completion seconds |
|---|---:|---:|---:|---:|
| Forced schema | 92,387,351 | 2.754690 | 7.965572 | 68.706966 |
| Whole zstd + Raw | 91,595,906 | 1.800569 | 5.975468 | 64.530287 |
| Selector | 91,595,906 | 3.785862 | 7.730612 | 68.839691 |

These are actual sums, not hypothetical amortizations. Each alternative arm's
thirty sessions owns its actual context setups and seed costs. Fitting the
already supplied immutable artifact is historical work, not repeated here.
The sender workload advantage of Whole is clear. Receiver CPU and WAN latency
vary with run order, the shared receiver and route; identical whole/selector
payloads do not justify attributing their receiver-CPU differences to a codec
change. Large-session p95 completion is 2,677.983 ms for schema, 2,945.163 ms for
whole and 2,876.669 ms for selector, showing the observed tail variability.

All **1,860 receiver request observations** agree with the registered operation
order and codec-cache progression: **180 preparations, 1,075 hits and 156
evictions** overall. Each receiver retains at most eight entries charged at
40 MiB total. The single ordinal gap per shard is its retained unauthenticated
preflight request; it is not a discarded measured operation. No implicit target
or artifact sharing occurs across authorized experiment scopes.

Measurement boundaries matter. Body counts include requests, framing, responses,
and actual seed traffic in both directions. They exclude HTTP/TLS/TCP/IP overhead,
retransmissions and extra relay-hop bytes. Exchange elapsed includes network
waiting and receiver work; it is not pure wire serialization time. Primary target
elapsed is the outer interval from encoding/selection start through durable ACK,
including any seed. Session elapsed is the sum of those intervals plus measured
context setup and any separate upfront seed; fixture generation, child-process
startup and report serialization are outside it. CPU is sender-process CPU plus
disjoint receiver-handler CPU, excluding receiver HTTP/socket/logging and relay
CPU. These results establish neither complete-system energy savings nor a
controlled 80 ms/10 Mbps acceptance result.

Local final qualification passed **315 tests, zero failures, two preexisting
ignored tests**, Clippy and release builds. Hosted qualification also passed.
Focused new-source coverage is 100% of selector lines/functions and 90.28% of
whole-codec lines with 100% of functions; branch/MC/DC coverage was not collected.
Positive lazy seeding, ready-schema reuse and missing-seed failure passed local
tests. This WAN corpus exercised no positive schema selection, so those local
tests must not be relabeled as WAN evidence. Premeasurement independent reviews
and all qualification failures/corrections are retained in the
[evidence packet](evidence/2026-10-30-schema-selection/README.md).

For this measured new-object workload, use the explicit whole-object zstd/Raw
path when compute and time matter. Retain schema selection as an opt-in capability
for workloads with a demonstrated advantage. The next comparison returns to
verified warm-object bases, sparse/prefix deltas and the applicable prepared
runtime, against the same tuned whole-object control. It must separate a change
in workload/available receiver knowledge from an actual encoder regression and
count both seed-plus-update and warmed updates. The historical ternary/CA/Kuramoto
mechanisms are unchanged; this campaign makes no new claim about them.
MVM/vsock remains a later integration.
