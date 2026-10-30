# Compressed-schema sender CPU: measured before and after

**The exact-layout fast path passed the registered goal: 32.06% less sender CPU on matching ~1 MiB batches, with identical protocol bytes and exact reconstruction.** The primary candidate/baseline ratio was **0.679367**, below the predeclared 0.90 threshold. All ten paired ratios were between 0.646 and 0.719. Normal-batch total CPU also decreased from 11.865 to 9.553 ms, and median local durable elapsed from 19.841 to 16.989 ms. Ordinary zstd remains substantially faster; the schema path sends about 2.2% fewer body bytes. This is an improvement to the existing schema path, not a general compression win.

The change verifies fixed fragments from the first validated schema template directly and parses primitive value tokens into bounded borrowed storage. A mismatch uses the original general parser. It avoids constructing and hashing the same matching layout again, while retaining token spelling, UTF-8 checks, work limits, literal fallback and the exact serialized stream. No codec settings, receiver, authorization or commit behavior changed.

The baseline was measured before production edits. Twenty fixtures in each size/family cell produced 640 independent stage observations plus eight separately timed setup observations. Normal-batch CPU medians were 4.310 ms for prepared schema encoding, 2.211 ms for compression of its precomputed intermediate, 6.651 ms for combined prepared encoding and 0.512 ms for object identity. These separate calls are **not additive pipeline components**. The [baseline profile](evidence/2026-10-30-schema-sender/baseline-profile.jsonl) retains first use, setup, exact compressed-frame/decode checks and all negatives.

The paired campaign ran on Apple M1/macOS 27.0 with Rust 1.97.1 release and hardware SHA-256. Baseline source was `4dacb487cd2b3b132aa6bbf015eb0e5f893e69e0`; candidate was `983bf181adda8dc922d0187e53652717d61d38b6`. Only production schema source differed between their pinned source sets. Ten paired trials at two sizes alternated baseline/candidate prepared-schema and ordinary-zstd workers in ABBA/BAAB source order. Each worker kept one receiver open across four normal, two variant, two drift and two unrelated **distinct** targets.

**All 800 targets and 40 seeds passed immediate and reopened full-byte comparisons.** All 420 paired protocol transcripts, selected modes and body costs matched. Forty sender preparations and 80 successful session cleanups are recorded. No observation, warm-up or outlier was discarded; the campaign ran once. The [contract](evidence/2026-10-30-schema-sender/measurement-contract.md), [raw worker observations](evidence/2026-10-30-schema-sender/paired.jsonl), and [full summary](evidence/2026-10-30-schema-sender/summary.json) retain every case and the ten primary ratios.

Normal-batch medians below pool four targets from each of ten sessions. CPU includes sender and receiver once; the subset medians should not be added to reproduce the total. The primary gate instead uses each trial's **sum of four sender CPU values**, then the geometric mean of ten paired ratios.

| Path | Body bytes | Sender CPU, ms | Receiver CPU, ms | Total CPU, ms | Durable elapsed, ms |
|---|---:|---:|---:|---:|---:|
| Baseline prepared schema | 127,557.5 | 7.570 | 4.305 | 11.865 | 19.841 |
| Candidate prepared schema | 127,557.5 | 5.172 | 4.270 | 9.553 | 16.989 |
| Baseline ordinary zstd | 130,450.5 | 1.823 | 2.245 | 4.091 | 12.822 |
| Candidate ordinary zstd | 130,450.5 | 1.835 | 2.234 | 4.045 | 11.976 |

Thus the candidate still uses about **2.36 times ordinary zstd's CPU** on this matching batch. The unchanged zstd control's elapsed median also moved from 12.822 to 11.976 ms, while sender CPU moved slightly upward. Preserve that control variation when interpreting wall-time changes: this run does not isolate storage/scheduling effects, establish confidence bounds, or measure WAN latency.

The actual ten-target mixed session includes its seed and sender preparation. Medians are computed from per-session sums, not by adding independent medians:

| Size / path | Session body bytes | Session CPU, ms | Summed durable elapsed, ms |
|---|---:|---:|---:|
| Small, baseline schema | 7,907 | 8.983 | 107.786 |
| Small, candidate schema | 7,907 | 8.767 | 108.452 |
| Small, baseline zstd | 6,732.5 | 8.306 | 92.645 |
| Small, candidate zstd | 6,732.5 | 8.067 | 92.850 |
| Batch, baseline schema | 3,079,318 | 99.794 | 191.941 |
| Batch, candidate schema | 3,079,318 | 89.565 | 179.965 |
| Batch, baseline zstd | 3,093,800.5 | 38.874 | 123.516 |
| Batch, candidate zstd | 3,093,800.5 | 39.940 | 127.649 |

Across each schema version's actual 200 targets and 20 sessions, setup-inclusive CPU totals were **1,096.626 → 990.806 ms**, and summed elapsed **3,009.516 → 2,896.277 ms**, with exactly 30,873,630 body bytes in either version. This is the registered mixture, not a predicted production workload. Small sessions did not uniformly become faster.

The fixed 169-byte SC-OPT-012 artifact was reused without refitting. Every schema session paid its actual **670-byte seed**. Batch seed CPU/elapsed medians were 0.662/11.942 ms for baseline and 0.545/11.452 ms for candidate; sender preparation CPU was 15/14 microseconds. These separately observed setup costs are included above. Lazy context allocation during encoding remains inside target timing. Both sources recorded 79 decoder hits, 20 misses/preparations and zero evictions. Maximum retained sender payload/context bytes were unchanged at 1,460,512. The 134,742,016-byte sender admission allowance and 5,242,880-byte receiver quota charge are conservative resource accounting, not RSS or peak-memory measurements.

Nonmatching controls retain their less favorable results:

| Schema batch family | Unchanged body bytes | Sender CPU before → after, ms | Total CPU before → after, ms | Elapsed before → after, ms |
|---|---:|---:|---:|---:|
| Variants | 131,894.5 | 7.954 → 7.939 | 10.726 → 10.658 | 18.857 → 19.025 |
| Drift | 100,896 | 6.143 → 6.262 | 8.229 → 8.328 | 16.115 → 16.093 |
| Unrelated binary | 1,051,445.5 | 4.094 → 4.112 | 6.404 → 6.393 | 14.497 → 13.861 |

Drift sender CPU increased by about 1.9%; the optimization does not improve every layout. Small unrelated schema total CPU also increased from 0.805 to 0.855 ms, and elapsed from 8.699 to 9.583 ms. Raw fallback and attempted encoding costs remain charged. All 32 family/source/arm/size distributions, including source-specific zstd controls, are available in the summary.

Measurements use actual serialized protocol batches, exact receiver checks and durable publication locally. Both directions count framing/acknowledgments and equivalent reply metrics, excluding HTTP/TLS/IP/TCP overhead and retransmissions. Transcript hashes and exact verification are computed outside transfer timers. Transcript retention changes buffer lifetimes relative to older campaigns, identically for these two binaries; the frozen matched baseline is the valid comparison. No new WAN, energy, larger-than-~1 MiB or MVM result is claimed.

The run completed `2026-10-30T19:45:37Z`–`19:45:58Z`, exit 0; the existing Rust summarizer exited 0. Qualification included 44 focused tests, 288 all-target passes with two historical ignores, one doctest, Clippy, and independent source/security reviews. A mixed-object coverage report was corrected using the same collected profile and exactly the tested binaries, with the initial report preserved; it was not a performance rerun. [Execution receipt](evidence/2026-10-30-schema-sender/run.json), [source/executable pins](evidence/2026-10-30-schema-sender/comparison-pins.json), and [evidence index/reproduction commands](evidence/2026-10-30-schema-sender/README.md) provide custody and practical limits.

| Artifact | SHA-256 |
|---|---|
| Baseline executable | `c76f08e41dc5fc52fa83477700a09d0af403399cd8c923dc18a547647bd28479` |
| Candidate executable | `5dc69edc6208011e89d5f6c98a7a1f19ea1a98c28f70f91074683a818c4c03b5` |
| Paired JSONL | `239fe8c22fbf23dddc26807d86b72663a1a39286dee0e69021fb2d9a0d60bf07` |
| Summary | `a50c1ba4d1905f27b1192131d360d4d27b905b673558a70c25b387b0612fa877` |
