# SC-OPT-003: hardware SHA-256 result

**All three registered CPU gates passed.** All 256 observations recovered the exact target and base after reopening. Hardware was faster in all 128 paired operation-CPU comparisons, covering 16 configurations with eight pairs each.

Both binaries use candidate `80449939bd92348593cc88555cf7fde0847ceffd`, the same expanded lock, measured Rust, compiler and environment. The sole active configuration difference is the default-off hardware-sha256 feature. Plan `f6a0144` and qualification packet `be2dfc3` were committed before measurement. Complete graph/manifest encodings matched across 36 qualification fixtures.

Ratios are geometric means of matched software/hardware observations. CPU covers the local operation from frame/recipe preparation through validated Published and request-buffer destruction. Handler CPU is a subset. Marginal wall medians are separate from paired ratios.

| Required cell | Software CPU median | Hardware CPU median | Paired CPU ratio | Required ratio | CPU wins | Software wall median | Hardware wall median | Paired wall ratio |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| N0 raw / nodes 262144 | 51.964 ms | 25.303 ms | 2.104× | 1.10× | 8/8 | 60.494667 ms | 33.378479 ms | 1.880× |
| N0 sparse / nodes 262144 | 16.360 ms | 4.402 ms | 3.669× | 1.50× | 8/8 | 24.649729 ms | 12.738000 ms | 1.866× |
| N768 sparse / nodes 262144 | 38.283 ms | 27.571 ms | 1.413× | 1.20× | 8/8 | 46.535708 ms | 36.285812 ms | 1.369× |

These paired CPU ratios correspond descriptively to 52.47%, 72.74% and 29.21% less CPU, respectively. They describe this local operation on this host, not WAN time or energy.

| Background objects | Arm / max nodes | Software CPU median ms | Hardware CPU median ms | Paired CPU ratio | CPU wins | Paired wall ratio | Wall wins |
|---:|---|---:|---:|---:|---:|---:|---:|
| 0 | raw / 262144 | 51.964 | 25.303 | 2.104× | 8/8 | 1.880× | 8/8 |
| 0 | sparse / 262144 | 16.360 | 4.402 | 3.669× | 8/8 | 1.866× | 8/8 |
| 0 | raw / 1 | 11.991 | 2.950 | 4.055× | 8/8 | 1.772× | 8/8 |
| 0 | sparse / 1 | 16.416 | 4.787 | 3.511× | 8/8 | 1.994× | 8/8 |
| 64 | raw / 262144 | 81.399 | 46.881 | 1.722× | 8/8 | 1.581× | 8/8 |
| 64 | sparse / 262144 | 35.107 | 13.560 | 2.507× | 8/8 | 1.939× | 8/8 |
| 64 | raw / 1 | 27.081 | 10.649 | 3.059× | 8/8 | 2.157× | 8/8 |
| 64 | sparse / 1 | 35.380 | 13.779 | 2.597× | 8/8 | 1.948× | 8/8 |
| 256 | raw / 262144 | 74.124 | 44.245 | 1.723× | 8/8 | 1.530× | 7/8 |
| 256 | sparse / 262144 | 35.831 | 19.044 | 1.886× | 8/8 | 1.692× | 8/8 |
| 256 | raw / 1 | 29.623 | 17.023 | 1.765× | 8/8 | 1.332× | 7/8 |
| 256 | sparse / 1 | 35.276 | 19.271 | 2.012× | 8/8 | 1.711× | 8/8 |
| 768 | raw / 262144 | 75.377 | 45.867 | 1.688× | 8/8 | 1.577× | 8/8 |
| 768 | sparse / 262144 | 38.283 | 27.571 | 1.413× | 8/8 | 1.369× | 8/8 |
| 768 | raw / 1 | 34.116 | 26.142 | 1.258× | 8/8 | 1.217× | 8/8 |
| 768 | sparse / 1 | 38.039 | 27.017 | 1.405× | 8/8 | 1.310× | 8/8 |

Wall time improved in 126/128 observations. Two slower observations remain: N256 raw normal block 4 (ratio 0.980723×) and N256 raw one-node block 2 (0.715277×). Every configuration's wall geometric mean favors hardware. Small components are mixed: N0 raw normal Begin CPU ratio is 0.937253×; N0 sparse normal Data CPU is 0.972932×. All stage and setup regressions remain in the machine report.

The stage measurements support the intended mechanism. For N0 sparse normal, median frame-preparation CPU falls from 8.4395 to 2.2755 ms, Begin from 3.8815 to 0.9255 ms, and Finish from 3.9595 to 1.208 ms. These paths contain full-object identity/verification hashes. The cryptographic function, integrity checks, identities and publication semantics remain unchanged.

Large-cache overhead remains: hardware sparse-normal CPU median is 4.4015 ms at N0 and 27.571 ms at N768. At N768, median Begin/Finish costs are still 13.077/12.033 ms. Source retains full quota scans; stages also include hashing, reads and durable publication, so this is not a per-function scan profile.

Modeled bidirectional application-frame bytes are unchanged: **1,051,138 bytes per raw observation and 536 per sparse observation**, totaling 67,307,136 per backend across the campaign. These are protocol-frame sizes in direct Endpoint calls, not captured traffic.

The 16 processes ran from 16:16:09Z to 16:25:05Z on 2026-10-29 (536 seconds including cloning/verification). All exited zero. All 2,944 frame samples, identities, inventories, readiness, stage sums and provenance pins passed validation. The 1,100-entry template manifest is unchanged. No retries, filtering, extra trials or post-hoc normalization occurred.

`paired-analysis.json` retains all 21 metric summaries per cell, eight paired values, midpoint medians and nearest-rank p95. With eight samples, p95 equals the observed maximum. Host one-minute loads ranged from 4.54 to 8.62; scheduling, OS caches and thermal/power state were uncontrolled.

Qualification scope is native Apple ARM; the feature remains default-off. This experiment measures no WAN throughput/latency, packets, power or joules. Do not multiply these ratios by earlier WAN ratios. Remote x86 hosts may already use hardware SHA by default. Unsupported targets and cryptographic-module validation remain outside this result.
