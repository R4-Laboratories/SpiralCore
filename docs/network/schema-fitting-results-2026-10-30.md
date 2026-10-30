# Schema fitting versus trained zstd dictionaries: local results

**The Rust schema prototype recovered every byte, but did not beat conventional compression on this registered workload.** Its normal ~1 MiB batch used 24.2% fewer body bytes than raw transfer, yet about 6.1 times as many as ordinary zstd and 2.6 times its CPU. A genuinely trained zstd dictionary helped small normal records after seeding; it also lost to ordinary zstd on normal batches and on this campaign's total bytes and CPU including actual setup. Keep schema fitting opt-in research rather than treating this result as a reason to replace the existing selector.

One campaign ran on Apple M1/macOS 27.0 using Rust 1.97.1, release mode and hardware SHA-256. Source was `fe98bf0a40cad50f513652690548f7f0bb4f948f`. Ten paired trials covered four held-out families, two sizes and five arms: **400 targets plus 240 artifact/base seeds, all verified by complete byte comparison after close/reopen, with 400 successful store cleanups**. No observations or warm-ups were discarded and there was no performance rerun. The [registration](evidence/2026-10-30-schema-fitting/measurement-contract.md), [frozen fixtures](evidence/2026-10-30-schema-fitting/plan.json), [raw observations](evidence/2026-10-30-schema-fitting/paired.jsonl), and [all 40 cells, including p95 and nested timing distributions](evidence/2026-10-30-schema-fitting/summary.json) are retained.

These are **local serialized durable transfers**, including sender preparation, actual endpoint decoding, length/digest verification and durable commit. CPU counts sender and receiver once; sender CPU below excludes the nested exchange interval. Body bytes count both serialized directions and reply metrics. They exclude actual HTTP/TLS/IP/TCP overhead and retransmissions. Elapsed time is encoding-start through durable acknowledgment, not network-only send time. Exact reopened verification is separately timed outside that interval.

Normal small records were 210–219 bytes; normal batches were 1,048,580–1,048,788 bytes, assembled from whole records without padding. All values below are medians of ten trials, after any separately measured seed.

| Normal target | Arm | Body bytes | Total CPU, ms | Sender CPU, ms | Durable elapsed, ms |
|---|---|---:|---:|---:|---:|
| Small | Raw | 715 | 1.141 | 0.014 | 13.449 |
| Small | Ordinary zstd | 667.5 | 0.916 | 0.036 | 14.960 |
| Small | Adaptive | 600 | 1.372 | 0.349 | 8.828 |
| Small | Schema | 715 | 0.913 | 0.023 | 11.614 |
| Small | Trained dictionary | 590 | 1.072 | 0.087 | 8.538 |
| ~1 MiB | Raw | 1,051,446.5 | 4.009 | 0.936 | 19.905 |
| ~1 MiB | Ordinary zstd | 130,432 | 5.978 | 2.738 | 21.937 |
| ~1 MiB | Adaptive | 127,990.5 | 13.621 | 10.252 | 21.678 |
| ~1 MiB | Schema | 796,771 | 15.677 | 8.445 | 23.843 |
| ~1 MiB | Trained dictionary | 135,721.5 | 10.632 | 7.622 | 19.198 |

Schema chose raw for all ten small normal targets. Dictionary's small-record body saving versus ordinary zstd was 11.6%; its CPU was higher. Its lower elapsed medians, including 19.198 ms for the batch, are observations from this local durable run. Wall times varied substantially: dictionary batch 14.308–26.152 ms, raw 11.336–22.999 ms. Nested receiver intervals include storage work; this experiment does not isolate the cause of that variability or demonstrate a network-only speed improvement. It also does not establish dictionary CPU savings.

Both fitted codecs trained on the same frozen 512 records, totaling 107,730 bytes; no held-out record was used for fitting. Adaptive used that exact training corpus as its conventional prefix/delta base. Common categorical values recur, while complete normal records and sequence/node ranges are held out. Schema preserves lexical slots and literal fragments; variants or drift can require literal fallback.

| Retained state | Serialized bytes | Fit/train plus serialization CPU / elapsed | Raw seed body bytes per destination |
|---|---:|---:|---:|
| Adaptive training corpus | 107,730 | No model fitting | 108,364 |
| Schema artifact | 169 | 0.464 / 0.464 ms | 670 |
| Trained dictionary artifact | 8,204 | 6.394 / 6.395 ms | 8,705 |

These fit measurements are single observations, not distributions. Dictionary bytes include the 8,192-byte trained dictionary and 12-byte typed header. Retained byte counts describe owned artifacts, not peak memory or RSS. Per-call parsing/context preparation remains inside transfer CPU.

For an initially unfitted sender and a new destination, normal small dictionary cost was **9,295 body bytes, 8.510 ms CPU and 29.959 ms elapsed**, including its seed and one fitting cost; ordinary zstd needed 667.5 bytes, 0.916 ms and 14.960 ms. For normal batches, dictionary cold cost was 144,426.5 bytes, 17.913 ms CPU and 39.400 ms elapsed. An already-fitted sender does not retrain for each new destination. These compound medians were calculated from each trial's summed costs, not by summing independent medians. Actual receiver reuse across multiple targets was not measured.

The other batch families preserve the negative controls. Their median body bytes / CPU milliseconds were:

| Held-out batch | Raw | Ordinary zstd | Adaptive | Schema | Trained dictionary |
|---|---:|---:|---:|---:|---:|
| Lexical/field variants | 1,051,422 / 3.448 | 143,300 / 5.193 | 131,908 / 12.120 | 1,008,009 / 9.517 | 141,211.5 / 9.172 |
| Field/value drift | 1,051,455 / 3.472 | 91,149.5 / 4.473 | 91,149.5 / 9.035 | 1,051,455 / 8.376 | 102,004 / 7.734 |
| Unrelated binary | 1,051,446.5 / 3.360 | 1,051,446.5 / 3.716 | 1,051,446.5 / 5.856 | 1,051,446.5 / 5.122 | 1,051,446.5 / 5.313 |

Schema fell back to raw on every small target and every drift/unrelated batch. Dictionary fell back on every unrelated target. Attempted encoding CPU and artifact seeding remain charged even when raw wins. Small drift also favored ordinary zstd over dictionary on bytes (638.5 versus 688). The variants batch is a modest dictionary byte win over ordinary zstd, with higher CPU; adaptive produced fewer bytes than either.

Each model/base was actually reused by the sender for 80 targets, but each target had a fresh receiver and therefore its own seed. The following totals include all 80 actual destination seeds for dependent arms and one fit/training cost. They describe this equally weighted mixture, not an assumed production workload:

| Arm, 80 target uses | Total body bytes | Total CPU, ms | Summed durable elapsed, ms |
|---|---:|---:|---:|
| Raw | 42,087,287 | 203.404 | 1,354.581 |
| Ordinary zstd | 14,191,123 | 241.644 | 1,479.965 |
| Adaptive | 22,720,732 | 584.864 | 2,455.161 |
| Schema | 39,159,636 | 505.730 | 2,402.187 |
| Trained dictionary | 15,026,101 | 467.671 | 2,365.133 |

Ordinary zstd uses existing chunked frames; dictionary uses a whole frame with the prefix path's level/window/long-distance-matching/checksum settings. This is a comparison of implemented complete paths, not an isolated causal estimate of dictionary training. Adaptive provides a conventional whole-frame prefix control. No 16 MiB, WAN, energy, MVM or government-qualification claim follows from this experiment.

The run was `2026-10-30T17:09:01Z`–`17:09:18Z`, exit 0; summary exit 0. Exact source-file and artifact pins are in [source-pins.json](evidence/2026-10-30-schema-fitting/source-pins.json), and execution details are in [run.json](evidence/2026-10-30-schema-fitting/run.json). Principal SHA-256 pins:

| Artifact | SHA-256 |
|---|---|
| Frozen executable | `50e6fb5216a1ae0170543c82932988361f0c646be64bcf5082c8b520d68a45e8` |
| Registered plan | `cceecc5d54b9ddf1853810af969c9abcdeaa6b97f30e81571bdf5dfa6b28aa49` |
| Raw JSONL | `f14e06536cfe8034b6f1f603db1a26bced30224bf105c01b1c4280ba3d7bdffb` |
| Full summary | `90ddae1cdf295b606e5c129d9b5bf6fb6263ee9c891e27a1b1088168622ea98c` |

To reproduce from the pinned source, build the release example with locked dependencies and `--features hardware-sha256`, then use new output/cache/artifact paths:

```sh
schema_fitting plan NEW_PLAN.json
schema_fitting run NEW_PLAN.json fe98bf0a40cad50f513652690548f7f0bb4f948f NEW_RESULTS.jsonl NEW_CACHE_DIR NEW_ARTIFACT_DIR
schema_fitting summarize NEW_RESULTS.jsonl NEW_SUMMARY.json
```
