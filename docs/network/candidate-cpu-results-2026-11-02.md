# Candidate work reduction in the Combined gateway

The Combined selector now avoids provably losing schema compression and reuses
identical residual work. All selected logical bytes and all 216 exact reopened
object deliveries were preserved in the paired campaign. On the calibrated
80 ms RTT / 10 Mbps link, the complete mixed session used **12.21% less endpoint
process CPU and 0.81% less elapsed time than the pinned previous selector**.
Compared with prepared whole-object zstd, it used **58.26% fewer captured IP
bytes, 39.71% less CPU and 53.10% less elapsed time**.

This is three descriptive paired trials on one Linux runner using real QUIC
traffic through isolated, calibrated network namespaces. It is not a new
Internet-WAN measurement, a physical-energy measurement, or transparent runtime
compression of arbitrary TUN traffic. The existing ternary/Kuramoto/CA prediction
and rolling-cache paths remain the selected mechanisms for matching content.

Implementation [PR #31](https://github.com/R4-Laboratories/SpiralCore/pull/31),
measured source `343248065628176f76841e98608a4a4badf747dd`, pinned baseline
`4b8cb79fe5fca9e9527a2923924c887d9e7ddbf8`,
[run 41399314329](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41399314329).
The [evidence index](evidence/2026-11-02-candidate-cpu/README.md) links source,
raw observations, full analysis, independent reviews and immutable archives.

## What changed

The previous selector could fully compress a schema candidate even after finding
a tiny Runtime or Sparse recipe. The new path builds the exact canonical schema
intermediate once and applies the existing bounded zstd certificate to that input.
A proven lower bound can discard a candidate that cannot win. Inconclusive or
out-of-domain cases retain full encoding, using the prepared intermediate.
Tie order, codec settings and resource errors are preserved. The existing Whole
probe context is reused; no third codec context or new decoder was introduced.

When the acknowledged rolling base is byte-for-byte identical to the immutable
runtime prediction, one residual encoding supplies both candidates' checked costs.
This requires full equality, not a hash hint. Sparse dominates the identical
residual plus Runtime setup, so no duplicate Runtime payload is materialized.
Canonical Offer/Data comparison against a frozen selector and an independent
original schema encoder checks exact output preservation separately from the
network receipts. Receiver authorization, identity, publication and ACK rules
are unchanged.

A retained local baseline diagnostic identified schema compression as expensive:
about 88 ms CPU for the large generated fixture, versus about 0.73 ms for its
Runtime residual. Those independently timed stages are not summed into a transfer
or mixed with this Linux before/after campaign.

## Complete persistent sessions

Each session sends six targets at each of two sizes: **1,047,202 bytes** and
**14,851,384 bytes**. The sequence is generated runtime data, a 512-byte residual
change, its exact repeat, unrelated opaque data, a localized opaque edit, and its
repeat. Both Combined arms pay checkpoint and schema publication, preparation and
replay. Whole-zstd uses the same verified target-cache probe. All arms charge input
reads, hashing, durable publication and receiver readback.

The campaign completed **18 sessions, 264 operations and 216 deliveries**, totaling
**1,717,047,288 requested object bytes**. Three complete arm-order rotations used
the same trial fixtures. Baseline and candidate were built on the same runner with
Rust 1.97.1 and identical release settings; Whole encoder/probe/dictionary sources
and lockfiles were verified identical. Both builds finished before timing.

Median full-session values:

| Link | Arm | Captured IP bytes, both directions | Summed endpoint process CPU | Client elapsed |
|---|---|---:|---:|---:|
| 80 ms / 10 Mbps | Combined before | 17,237,338 | 1.130903 s | 16.769147 s |
| 80 ms / 10 Mbps | Combined after | 17,234,683 | 1.004654 s | 16.632538 s |
| 80 ms / 10 Mbps | Whole-zstd | 41,294,515 | 1.659298 s | 35.461769 s |
| Unshaped local link | Combined before | 16,720,035 | 0.865372 s | 1.006237 s |
| Unshaped local link | Combined after | 16,720,003 | 0.737390 s | 0.838375 s |
| Unshaped local link | Whole-zstd | 40,266,096 | 0.996329 s | 0.975360 s |

Percentages are medians of matched trial ratios, not ratios of the medians above.
Unshaped Combined improves CPU by **14.79%** and elapsed by **17.74%** against its
previous version; against zstd, it improves CPU by **25.99%** and elapsed by
**14.04%**. This removes the previous mixed-session local elapsed disadvantage in
this campaign, without claiming all individual cases now win.

Before and after use exactly **15,950,640 logical channel bytes per session**.
The small captured-IP differences are transport scheduling/ACK/retransmission
outcomes, not new codec savings. All six matched session pairs preserve operation
identities, selected modes, publication paths, frame/data counts and bidirectional
logical deltas. The unchanged repeat path uses **230 logical channel bytes** here;
that accounting is different from historical 600-byte HTTP-body experiments.

## Individual transfers on the shaped link

These medians describe each delivered **14,851,384-byte** target. Logical bytes
include requests and responses with channel framing; they exclude QUIC/IP overhead.
Setup is separately paid in the full-session table, not hidden or allocated into
every target row. Sender CPU includes input reads, hashing and selection; elapsed
is measured completion, including transfer and publication.

| Target | Arm | Logical bytes | Sender CPU | Completion elapsed |
|---|---|---:|---:|---:|
| Generated | Before | 771 | 66.017 ms | 320.896 ms |
| Generated | After | 771 | 23.659 ms | 275.450 ms |
| Generated | Whole-zstd | 3,194,293 | 100.632 ms | 2,994.110 ms |
| Residual edit | Before | 1,374 | 78.604 ms | 317.443 ms |
| Residual edit | After | 1,374 | 33.743 ms | 277.236 ms |
| Residual edit | Whole-zstd | 3,194,762 | 94.663 ms | 2,983.618 ms |
| Opaque edit | Before | 1,374 | 56.594 ms | 315.917 ms |
| Opaque edit | After | 1,374 | 26.595 ms | 262.611 ms |
| Opaque edit | Whole-zstd | 14,883,962 | 187.200 ms | 12,884.436 ms |
| Unrelated opaque | Before | 14,883,962 | 243.858 ms | 12,940.852 ms |
| Unrelated opaque | After | 14,883,962 | 241.683 ms | 12,936.397 ms |
| Unrelated opaque | Whole-zstd | 14,883,962 | 193.298 ms | 12,898.249 ms |

Large generated sender CPU falls **64.18%**, residual CPU **56.50%**, and opaque-edit
CPU **53.01%** by matched medians. Corresponding elapsed reductions are **14.16%**,
**12.87%**, and **16.54%**. Bytes are identical to before. The large unrelated
transfer still uses **25.20% more sender CPU than zstd** and has **0.30% worse
elapsed**, with identical logical bytes. Its selection overhead remains real.

The near-1 MB generated target falls from 5.711 to 3.442 ms sender CPU and from
171.095 to 168.756 ms elapsed; zstd uses 6.564 ms and 421.983 ms. Its unchanged
771 logical bytes compare with 88,481 for zstd. The smaller unrelated target
still takes 1,245.836 ms versus zstd's 1,067.153 ms at equal 1,049,988 logical bytes.
All trial-level results, including setup and repeat timing variation, remain in
the full analysis. For example, the large first repeat is **1.75% slower than
before** by paired median despite unchanged code path and bytes; it is not removed.

## Qualification and interpretation

Directional retained calibration means were **80.200–80.300 ms RTT** and
**9.786–9.797 Mbps**. Captures have zero capture drops and complete packet counts.
The finite shaped queues dropped **1,197 packets** across sessions; resulting
repair/retransmission costs remain in measured bytes and time. Host routes and
frozen executable identities remained unchanged.

Summed endpoint process CPU is work consumed by both processes. It overlaps
elapsed time and must not be added to it. Sender operation CPU and receiver
handler CPU are narrower stage measurements; receiver readback/finish is charged
in full-session cost. No isolated send-only stopwatch was measured, and subtracting
CPU from elapsed would not produce one. No p95, confidence interval, energy-saving
or general Internet-WAN claim follows from three trials.

The changed selector now spends substantially less CPU on known losing work while
preserving the runtime/cache byte advantages. The next implementation target is
remaining unrelated-object selection overhead, followed by current-binary
Internet-WAN validation and the shared MVM adapter integration. Predictive runtime
savings require matching shared state or sparse residuals; arbitrary encrypted
traffic does not acquire those properties merely by traversing the gateway.
