# Real two-machine WAN results — October 29, 2026

SpiralCore transferred and durably reconstructed the synthetic test objects across a real Internet path. All **906 seed/transfer operations succeeded**, and an independent reopening of the receiver cache compared **1,044,381,786 bytes exactly**, including their lengths. There were 575 tasks, 331 seed operations and 845 persisted scope/object entries, representing 151 distinct content identities. No fidelity loss was observed.

Warm DAG updates materially reduced protocol-body bytes, but the current computation made localized and scattered changes slower than full resends on this connection. Ordinary exact-object cache hits reduced both bytes and time. Cold random data and prefix insertion expanded substantially under the DAG encoding. Complete per-transfer IP accounting did not qualify; the measured protocol-body results and observed campaign IP total have different scopes.

## Experiment and identity

- Measured source: `55ded7630f4a53476118bb00a9355fcd26952793`.
- [Successful hosted run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/40817487041).
- Path: GitHub Actions Ubuntu 24.04 x86-64 sender → verified HTTPS → Cloudflare Quick Tunnel → Apple M1 Mac receiver. These were two different machines communicating over the Internet, without artificial latency sleeps or network shaping.
- All arms used the same HTTPS adapter and durable publication contract. The origin accepted only the generated plan's scopes and an ephemeral bearer capability. Cloudflare terminated TLS and could inspect this public synthetic data.
- Six 1 MiB workloads used 30 paired trials per arm, with seeded counterbalanced arm order and identical inputs within each pair. A separate 16 MiB check used one trial per arm.
- End-to-end transfer timing begins before required base preparation/encoding, connection setup and HTTPS work; it ends after the verified durable publication reply. Synthetic input generation is recorded separately. Reopening the cache and comparing expected bytes is an independent verification outside these timings.
- Standard GitHub runners are free for public repositories; the hosted artifact was retained for one day. No paid VM was created. Artifact-storage quotas still apply. [GitHub billing](https://docs.github.com/en/billing/concepts/product-billing/github-actions), [Cloudflare Quick Tunnels](https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/).

## All 1 MiB measurements

Bytes below are **HTTP request bodies plus response bodies**, including binary protocol framing, requests and acknowledgments. They exclude HTTP headers, TLS, TCP/IP and relay overhead. Each row has 30 successful transfers and no failures. Quantiles use nearest rank: median is the 15th and p95 the 29th sorted value out of 30. These p95 estimates have limited tail precision.

Sender CPU includes base/codec and HTTP/TLS processing. Receiver CPU covers the protocol handler, decoding/reconstruction, durability and reply encoding, but excludes the separate tunnel process and socket wait. ARM and x86 process CPU seconds are not interchangeable energy or instruction counts. **Do not add CPU time to elapsed time; elapsed time already includes the computation.**

| Workload | Encoding/control | Body bytes, median | Elapsed median ms | Elapsed p95 ms | Sender CPU median ms | Receiver CPU median ms |
|---|---|---:|---:|---:|---:|---:|
| cold_random | dag | 4,306,412 | 993.238 | 1163.785 | 130.344 | 227.559 |
| cold_random | raw_full | 1,051,338 | 476.746 | 588.897 | 4.878 | 141.388 |
| cold_random | zstd_full | 1,051,338 | 481.505 | 709.248 | 5.059 | 143.336 |
| cold_structured | dag | 178,872 | 321.791 | 434.637 | 24.483 | 56.392 |
| cold_structured | raw_full | 1,051,338 | 386.237 | 505.766 | 4.870 | 46.267 |
| cold_structured | zstd_full | 77,765 | 238.643 | 277.123 | 3.024 | 40.747 |
| localized_32 | dag | 2,342 | 605.083 | 768.295 | 213.366 | 237.202 |
| localized_32 | raw_full | 1,051,338 | 463.200 | 601.045 | 4.872 | 154.206 |
| localized_32 | zstd_full | 1,051,338 | 470.016 | 598.629 | 5.222 | 156.482 |
| prefix_1 | dag | 4,306,671 | 1177.034 | 1540.298 | 241.013 | 355.490 |
| prefix_1 | raw_full | 1,051,339 | 482.616 | 567.638 | 5.068 | 163.740 |
| prefix_1 | zstd_full | 1,051,339 | 474.128 | 552.831 | 5.092 | 163.594 |
| repeat | dag | 841 | 352.028 | 455.851 | 207.002 | 8.765 |
| repeat | raw_cas | 238 | 145.156 | 167.709 | 2.538 | 7.421 |
| repeat | raw_full | 1,051,338 | 482.599 | 615.409 | 4.810 | 150.229 |
| repeat | zstd_full | 1,051,338 | 481.682 | 578.111 | 4.797 | 150.736 |
| scattered_32x32 | dag | 30,648 | 625.014 | 879.759 | 223.033 | 240.840 |
| scattered_32x32 | raw_full | 1,051,338 | 462.061 | 562.239 | 4.943 | 157.112 |
| scattered_32x32 | zstd_full | 1,051,338 | 466.663 | 686.773 | 5.257 | 159.543 |

`raw_full` and `zstd_full` deliberately transmit the complete target. `raw_cas` is an ordinary content-addressed cache lookup, included for exact repeats. `dag` uses the current ordered 32-byte ternary DAG. `localized_32` changes one 32-byte region; `scattered_32x32` changes 32 separated 32-byte regions; `prefix_1` inserts one byte at the front.

`zstd_full` uses this profile's independent 60 KiB, level-1 zstd chunks with raw fallback. It is not an optimized continuous zstd stream or a trained dictionary. Localized and scattered edits also have different edit budgets: 32 versus 1,024 changed bytes.

The practical results are:

- **Localized edit:** 2,342 versus 1,051,338 body bytes, a **99.7772% reduction**. Median elapsed time was 605.083 versus 463.200 ms. The byte advantage is real for this warm fixture; the latency advantage is absent on this path.
- **Scattered edits:** 30,648 versus 1,051,338 body bytes, a **97.0849% reduction**. Median elapsed time was 625.014 versus 462.061 ms.
- **Exact repeat:** ordinary CAS used 238 body bytes and 145.156 ms; DAG used 841 bytes and 352.028 ms; full raw resend used 1,051,338 bytes and 482.599 ms. Generic cache identity explains this gain. It is not evidence of a DAG-specific mathematical advantage.
- **Structured cold data:** zstd's 77,765 bytes and 238.643 ms beat DAG's 178,872 bytes and 321.791 ms. Both beat raw bytes here, but zstd is the stronger measured conventional control.
- **Random cold data:** DAG transmitted about 4.10 times as many body bytes as raw and took longer. Raw and bounded zstd had identical body sizes because incompressible chunks fell back to raw.
- **One-byte prefix insertion:** DAG again transmitted about 4.10 times as many body bytes as raw, with median elapsed time 1,177.034 versus 482.616 ms. Fixed 32-byte alignment destroys reuse in this fixture.

These are observations of the named arms, not an automatic codec selector. The implementation does not yet choose the best measured encoding for each workload. The raw and zstd controls do not establish superiority over conventional delta systems; rsync/VCDIFF remain unmeasured.

Independent paired calculations confirm that this is not just a ratio-of-medians artifact: the DAG/raw elapsed-time geometric means were 1.3341 for localized edits, 1.3703 for scattered edits, 2.0579 for cold random data and 2.4811 for prefix insertion, with DAG slower in all 30 pairs for each of those cases. Ordinary repeat CAS was 3.5441 times faster than full raw resend on the paired geometric mean. These are point estimates from this campaign, without a population confidence-interval claim.

## Cache seeding and larger objects

Seeding is retained as a separate operation in the receipts and included in `seed_plus_transfer` summaries. Mutation arms all receive the same initial raw seed. For localized edits, seed plus target uses 1,053,680 body bytes with DAG versus 2,102,676 with two raw objects, saving 49.8886% over this two-version sequence. Scattered edits use 1,081,986 versus 2,102,676 bytes, saving 48.5424%. A prefix insertion instead costs 5,358,009 versus 2,102,677 bytes. No seeding cost is hidden.

The repeat full-copy controls intentionally use fresh unseeded scopes so they cannot silently succeed on Begin. If the candidate alone is charged a new 1,051,338-byte seed, both 1 MiB cache-hit arms need two subsequent repeats to beat an otherwise unseeded full-copy baseline in total body bytes. This is a constant-savings extrapolation, not a separately measured long sequence. If an initial version had to be delivered anyway, that common initial delivery should instead be charged to both systems.

The 16 MiB observations below are **one-trial smoke results**, not 30-trial performance estimates. The current DAG node budget does not qualify arbitrary random 16 MiB DAGs; these larger checks cover raw, zstd and whole-object reuse.

| Workload | Encoding/control | Body bytes | Elapsed ms | Sender CPU ms | Receiver CPU ms |
|---|---|---:|---:|---:|---:|
| cold_random | raw_full | 16,814,026 | 1861.489 | 63.967 | 324.873 |
| cold_random | zstd_full | 16,814,026 | 2044.774 | 72.006 | 281.461 |
| repeat | raw_cas | 238 | 212.462 | 12.579 | 54.113 |
| repeat | raw_full | 16,814,026 | 1758.386 | 61.869 | 292.406 |
| repeat | zstd_full | 16,814,026 | 1719.019 | 64.291 | 287.737 |

## Campaign costs and packet-accounting limit

The measured operation sequence lasted 473.640 seconds, including all seeds and controls. HTTP bodies totaled 1,045,067,178 bytes. Sender process CPU summed to 34.996 seconds; receiver handler CPU summed to 138.904 seconds. Over the broader before/after observation interval, the Mac origin consumed 144.82 process CPU seconds and `cloudflared` consumed 53.91 seconds. Those whole-process deltas include setup/idle work and are not additional per-transfer latency.

Five-second Mac samples observed a maximum resident size of 123,120 KiB for the origin and 49,200 KiB for the tunnel. These are sampled maxima, not proven peaks; sender peak RSS was not measured. The 845 object files occupied 964,758,295 bytes, containing 964,690,010 bytes of unique retained payload within this experiment's isolated scopes.

The runner captured **1,094,144,794 IP bytes**, including both directions, captured TLS/HTTP traffic, TCP acknowledgments and retransmissions on the runner-to-Cloudflare leg. This is an observed total, not a complete whole-path traffic claim. It excludes the Cloudflare-to-Mac leg and Cloudflare internals. Mac `nettop` process counters are retained separately because their accounting layer differs and must not be added to the captured IP count as if the units were identical.

**Per-transfer IP qualification failed conservatively:**

1. Linux reused 24 TCP four-tuples with different initial sequence numbers. The analyzer quarantined 47,663 packets / 62,277,586 captured IP bytes because it cannot yet prove distinct TCP epochs. Read-only inspection found sequential closes before reuse, but the current analyzer does not turn that observation into an ownership proof.
2. tcpdump reported 840,479 captured packets, 840,480 received by filter and zero kernel drops. The one-count discrepancy is unresolved. It does not prove a lost transfer packet, and zero drops does not prove complete capture. Libpcap documents platform-dependent filtering and unread-buffer semantics. [Statistics documentation](https://www.tcpdump.org/manpages/pcap_stats.3pcap.html).

The original remote capture report already marked the paired campaign incomplete because of tuple ambiguity. A later analyzer repair at `32a486797c8008f9b01766dfd7dec04f2d97efba` also rejects received/captured counter discrepancies and missing statistics. Both original and rechecked reports are retained. The earlier one-trial smoke's original `capture_complete: true` is superseded by this counter reassessment: its 87,199 captured versus 87,204 received counts are also unresolved. Object fidelity and protocol-body measurements are unaffected.

Do not present the body-byte savings as fully qualified all-wire savings or drop ambiguous rows silently to claim a complete 30-pair IP comparison. A future capture qualification needs verified TCP-epoch attribution and capture-boundary accounting. Direct-peer and controlled 80 ms / 10 Mbps impairment experiments, loss/jitter matrices, three-peer recovery and MVM remain unrun.

## Evidence and reproducibility

- [Qualification receipt](evidence/2026-10-29-wan/qualification.json): source, exact verification, counts, process totals, analyzer identity, cleanup and limits.
- [Raw operation receipts](evidence/2026-10-29-wan/rows.jsonl), [canonical plan](evidence/2026-10-29-wan/plan.json), [Rust summary](evidence/2026-10-29-wan/summary.json), [exact cache verification](evidence/2026-10-29-wan/verification.json).
- [Independent analysis](evidence/2026-10-29-wan/independent-analysis.json): source/plan/receipt joins, independently parsed PCAP totals, paired comparisons and seed accounting.
- [Original capture report](evidence/2026-10-29-wan/capture.json), [rechecked capture report](evidence/2026-10-29-wan/capture-rechecked.json), [raw tcpdump counters](evidence/2026-10-29-wan/tcpdump-stats.txt), [artifact hashes](evidence/2026-10-29-wan/original-artifact-manifest.json).
- The full 88 MiB header-only PCAP is preserved locally at `/Users/casey.allard/spiralcore/local/wan-20261029-paired/remote/headers.pcap`; SHA-256 is `e7aef8c70dd57c991ea9611656f30d2f5672c2b906941206f78d8a3d64e474fb`. A local gzip copy is also retained. The hosted artifact expires after one day; compact receipts/counters are committed here.
- [WAN instructions](wan.md) explain fresh-plan/cache requirements, temporary credentials, timing boundaries and cleanup. The endpoint and tunnel were stopped, the temporary Actions secret and four variables were deleted, and private capability files were removed.

The useful next implementation work follows the measured costs: keep cold transfer on raw/zstd; use direct whole-object lookup before rebuilding a DAG; persist/reuse prepared base graphs; and investigate an insertion-resilient delta/chunking alternative against rsync or VCDIFF. These are subsequent optimizations, not results claimed by this campaign. MVM integration remains deferred by user priority.
