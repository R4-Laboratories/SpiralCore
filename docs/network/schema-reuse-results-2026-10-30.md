# Compressed schema and persistent codec reuse: local results

**Compressed schema now approaches conventional compression on bytes, but remains substantially more expensive in CPU. Prepared sender/receiver state works without changing bytes; this experiment does not establish a substantial total CPU benefit.** On normal ~1 MiB targets, schema+zstd reduces the earlier schema representation from 796,723 to 127,557.5 body bytes. That is 2.2% below ordinary zstd's 130,450.5 bytes, at roughly three times its CPU. Across the actual mixed sessions including setup, compressed schema saves only 0.43% of ordinary zstd's total bytes and consumes about 2.45 times its CPU. Both mechanisms remain opt-in research capabilities.

The completed campaign used source `fa0b0abdde0200c9faf62701daa28f9727cc62b4`, Rust 1.97.1 release, hardware SHA-256 and Apple M1/macOS 27.0. It ran **1,600 targets, 120 seeds and 40 sender preparations across 160 sessions**. Every session kept one Endpoint open for ten distinct targets: four normal, two lexical/field variants, two drift and two unrelated binary objects. Every target ID differed within its session, and all arms received identical paired inputs. **All 1,720 immediate and 1,720 final reopened byte comparisons passed**, and all 160 owned stores were cleaned. No transfer observation or warm-up was discarded.

Small objects were 210–331 bytes; batches were 1,048,578–1,048,869 bytes of whole records, without padding. The same frozen 512 training records and 8,192-byte dictionary budget were used throughout, with no held-out fitting. See the [contract](evidence/2026-10-30-schema-reuse/measurement-contract.md), [exact fixtures](evidence/2026-10-30-schema-reuse/plan-corrected.json), [raw observations](evidence/2026-10-30-schema-reuse/paired-corrected.jsonl) and [full summary](evidence/2026-10-30-schema-reuse/summary-corrected.json), including 64 family cells, 160 position cells, p95, directional bytes and nested timings.

The following are normal-batch medians, pooling four targets in each of ten paired sessions. CPU includes sender and receiver once; sender/receiver are nested parts, whose medians should not be added. Elapsed time spans encoding through verified durable acknowledgment. Seed and explicit preparation costs appear separately below.

| Arm | Body bytes | Total CPU, ms | Sender CPU, ms | Receiver CPU, ms | Durable elapsed, ms |
|---|---:|---:|---:|---:|---:|
| Raw | 1,051,408 | 2.856 | 0.665 | 2.091 | 11.190 |
| Ordinary zstd | 130,450.5 | 3.846 | 1.816 | 2.030 | 12.247 |
| Adaptive | 127,990.5 | 8.296 | 6.262 | 2.001 | 16.459 |
| Schema V1 | 796,723 | 9.654 | 5.317 | 4.196 | 17.648 |
| Schema+zstd | 127,557.5 | 11.879 | 7.545 | 4.254 | 20.176 |
| Prepared schema+zstd | 127,557.5 | 11.789 | 7.536 | 4.234 | 20.155 |
| Trained dictionary | 135,850.5 | 6.660 | 4.575 | 2.087 | 14.868 |
| Prepared dictionary | 135,850.5 | 6.611 | 4.597 | 2.016 | 14.885 |

Prepared and corresponding unprepared paths had identical selected modes and body-byte costs for all paired targets. Receiver codec retention was disabled in baseline arms and enabled in prepared arms, so this comparison changes **both sender and receiver reuse**. It does not isolate a sender-only speedup. Batch CPU median changes were under 1% for both prepared codecs. Across all observed targets and setup, prepared schema CPU was slightly higher (1,107.200 versus 1,103.450 ms), as was prepared dictionary CPU (741.487 versus 729.192 ms). These are retained outcomes, not evidence of a large optimization gain.

Normal small dictionary targets used 589 body bytes versus ordinary zstd's 668. Prepared dictionary sender CPU fell from 72 to 29.5 microseconds, but total CPU increased from 1.062 to 1.227 ms; receiver medians were 0.977 and 1.171 ms. Durable elapsed medians were 8.986 and 8.526 ms. Schema+zstd chose raw for all 40 small normal targets, using 716 body bytes, including framing. Local storage and scheduling variability remain in these elapsed observations; the data does not isolate their cause or imply network-only speed gains.

The whole ten-target session is the practical reuse unit. These batch-session medians include the real one-time destination seed and sender preparation, where required. Fitting is excluded here because it occurs once across sessions, and is included in the campaign totals later.

| Ten mixed batch targets | Total body bytes | Total CPU, ms | Summed durable elapsed, ms |
|---|---:|---:|---:|
| Raw | 10,514,401.5 | 29.118 | 119.434 |
| Ordinary zstd | 3,093,800.5 | 37.069 | 123.369 |
| Adaptive | 3,169,212.5 | 74.044 | 167.549 |
| Schema V1 | 9,409,434.5 | 74.752 | 165.504 |
| Schema+zstd | 3,079,318 | 97.928 | 193.948 |
| Prepared schema+zstd | 3,079,318 | 98.243 | 193.635 |
| Trained dictionary | 3,141,315.5 | 60.206 | 154.363 |
| Prepared dictionary | 3,141,315.5 | 59.999 | 157.941 |

For first-use versus subsequent-use accounting, batch session medians were:

| Arm | Seed + preparation + first target CPU / elapsed, ms | Subsequent nine targets CPU / elapsed, ms |
|---|---:|---:|
| Ordinary zstd | 3.746 / 15.919 | 33.282 / 107.881 |
| Schema+zstd | 12.561 / 31.189 | 85.377 / 159.314 |
| Prepared schema+zstd | 12.633 / 32.410 | 85.556 / 160.048 |
| Trained dictionary | 7.405 / 26.727 | 52.801 / 125.788 |
| Prepared dictionary | 7.423 / 28.658 | 52.711 / 130.181 |

The later group contains different families, so it is not a pure cache warm-up comparison. Values are medians of per-session sums, not sums of independent medians. The prepared dictionary batch session with 371.526 ms elapsed remains included; no cause is inferred or outlier removed.

Schema fit plus serialization cost 0.440 ms CPU / 0.441 ms elapsed and produced 169 bytes; dictionary training plus serialization cost 6.165 / 6.166 ms and produced 8,204 bytes. These are one observation each. Destination seeds cost 670 body bytes for schema, 8,705 for dictionary and 108,364 for adaptive's 107,730-byte corpus. Prepared schema construction cost median 12 microseconds CPU for small sessions and 8 for batch sessions; dictionary construction cost 17 and 9. Lazy workspace growth during encoding stays inside target timing. For ten small targets, dictionary still used 15,116.5 total body bytes versus zstd's 6,732.5: ten sends did not amortize its seed enough to win.

Decoder reuse is directly observed: prepared schema recorded **79 hits, 20 misses/preparations**, and prepared dictionary **140 hits, 20 misses/preparations**, with no evictions. Raw fallbacks did not require fitted decoding. Sender retained payload/context bytes grew from 5,544 to at most 1,460,512 for schema and from 21,668 to at most 1,098,332 for dictionary. These counts exclude Rust/allocator metadata. Conservative sender admission charges were 134,742,016 and 134,283,264 bytes; receiver `resident_bytes` was a **5,242,880-byte quota charge**, not measured allocation or RSS. No energy or peak-memory claim follows.

Negatives persist under drift and unrelated content. Drift batches used 91,095.5 bytes / 3.425 ms CPU with ordinary zstd versus 100,896 / 8.158 with prepared schema+zstd and 101,793.5 / 5.437 with prepared dictionary. All unrelated batch paths used the same 1,051,445.5 body bytes; prepared schema/dictionary CPU was 6.274/4.443 ms versus raw 2.948 and ordinary zstd 3.124. Their attempted encoding costs remain charged. Variants favored compressed schema on bytes (131,894.5 versus zstd 143,390.5), with higher CPU (10.698 versus 4.199 ms for prepared schema).

Actual alternative-arm totals below include 200 target uses, 20 real destination seeds where needed, 20 sender preparations for prepared arms, and one fit of the required artifact. The experiment physically shared two fitted artifacts; do not sum these alternative totals as though each duplicated fit actually ran.

| Arm | Total body bytes | Total CPU, ms | Summed durable elapsed, ms |
|---|---:|---:|---:|
| Raw | 105,217,402 | 375.575 | 2,175.927 |
| Ordinary zstd | 31,006,426 | 451.858 | 2,232.089 |
| Adaptive | 32,840,158 | 915.692 | 2,859.787 |
| Schema V1 | 94,174,281 | 835.471 | 2,718.974 |
| Schema+zstd | 30,873,630 | 1,103.450 | 3,053.607 |
| Prepared schema+zstd | 30,873,630 | 1,107.200 | 3,039.477 |
| Trained dictionary | 31,561,950 | 729.192 | 2,601.629 |
| Prepared dictionary | 31,561,950 | 741.487 | 2,872.385 |

This is local serialized HTTP-body-equivalent accounting in both directions, including acknowledgments and reply metrics. It excludes HTTP/TLS/IP/TCP overhead, retransmissions and real network send time. Immediate and final reopened comparisons are separately timed outside transfer measurements. Ordinary zstd is chunked while the new compressed paths use whole-stream settings, so this does not isolate dictionary/schema training from framing differences. No larger-than-~1 MiB, WAN, MVM or energy result is claimed.

The first launch failed before any transfer because the harness pre-created a nonprivate receiver directory. [That attempt, including both fit observations, is preserved](evidence/2026-10-30-schema-reuse/attempt1-failure.json). A real-session regression reproduced it, then all seven example tests passed after removing pre-creation. A stale explicit-target plan was also caught before execution and retained. The corrected campaign used new frozen pins and separate paths, ran once from `17:57:28Z` to `17:58:06Z` on 2026-10-30, and its run and summary exited 0. [Run receipt](evidence/2026-10-30-schema-reuse/run-corrected.json), [source/artifact pins](evidence/2026-10-30-schema-reuse/source-pins-corrected.json), and [reproduction commands/evidence index](evidence/2026-10-30-schema-reuse/README.md) preserve the complete distinction.

| Completed-campaign artifact | SHA-256 |
|---|---|
| Frozen executable | `f43aed1bc353b6f2d47b0937a2e1e933fc7f313aae027683c52c8fc8482ec8aa` |
| Corrected plan | `daf2d9e9411759fb8636fad4a19e426ca6087b3d7d5906a50dc552a8403c3a19` |
| Raw JSONL | `e41ce987a666d632d3744bdbd57f9cdcbfe992953702f9f750d95ceaf09d4d18` |
| Summary | `8f65552d2ff45d3700a5ae6c42317a077e034ffe5ee099050418b4c31ecb414e` |
