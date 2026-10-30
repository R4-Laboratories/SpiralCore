# Packed replay and prepared predictions — 2026-10-30

The Rust optimization substantially reduces computation while preserving the existing transfer bytes and exact output. For the **14,851,384-byte** generated trace, standalone replay median CPU fell from **228.305 ms to 12.184 ms**, an **18.74× ratio of medians**. Full local transfer CPU fell from **559.601 ms to 139.071 ms** with packed replay, then to **117.050 ms** when using an already prepared sender prediction. These are measured local improvements over the previous runtime. Raw and zstd still cost less CPU and elapsed time for this large workload; this run does not establish a new WAN or energy result.

Source is frozen at [`384ad3af2a41e87643bd0678120b854e5fe7244b`](https://github.com/R4-Laboratories/SpiralCore/commit/384ad3af2a41e87643bd0678120b854e5fe7244b), compared with preserved baseline `d4532b0fade8e41b4beb17e6b00572d436cfe444`. The [Rust harness](../../crates/spiralcore-transfer/examples/replay_cpu.rs) is identical in both builds; the candidate enables its prepared-API compile branch. Both are Rust 1.97.1 release builds with `hardware-sha256`, on Apple M1/macOS. Executable and source identities are in [source-pins.json](evidence/2026-10-30-replay-cpu/source-pins.json).

The changes are private packed CA rows with reusable scratch and direct canonical trace encoding, plus an explicit immutable `PreparedPrediction` reused by the sender. The receiver still reconstructs the prediction, applies exact residuals, checks the final object and publishes durably. Standalone replay isolates the engine change; the packed transfer arm includes the accompanying small one-shot setup-helper/vector refactor. We do not attribute every saved transfer cycle exclusively to the CA loop.

The [registered local experiment](evidence/2026-10-30-replay-cpu/measurement-contract.md) ran ten paired trials for two actual trace sizes, three cases and six arms, with randomized arm order. **All 360 targets and 360 separately measured checkpoint seeds passed complete byte comparison after endpoint close/reopen.** All 120 owned worker caches were removed after success. Paired checkpoint, prediction and target identities matched; baseline, packed and prepared transfer-body costs matched exactly. A separately labelled three-trial baseline phase profile retained another 18 targets and 18 seeds. No warm-up, failed observation or outlier was discarded; there was one completed paired campaign and no performance rerun.

All times below are milliseconds. CPU covers the sender plus receiver in one process, counted once. Elapsed time includes encoding, serialized batch exchange, handler work and durable acknowledgment. These are in-process local transfers using the real wire serializers and endpoint, not TCP/HTTP/WAN measurements. Body counts include the equivalent HTTP reply metrics but exclude HTTP headers, TLS, IP/TCP and retransmission overhead. Exact reopen verification is outside the timed transfer. The raw rows retain its separate cost.

For the 14,851,384-byte generated trace, target-only results were:

| Arm | Body bytes | Median CPU | Median elapsed | p95 CPU | p95 elapsed |
|---|---:|---:|---:|---:|---:|
| Previous runtime | 747 | 559.601 | 578.524 | 633.951 | 749.171 |
| Packed runtime | 747 | 139.071 | 158.894 | 258.371 | 586.076 |
| Prepared runtime | 747 | 117.050 | 134.899 | 149.206 | 177.679 |
| Raw | 14,883,938 | 37.596 | 51.247 | 44.201 | 76.483 |
| zstd | 3,513,668 | 38.775 | 53.213 | 42.740 | 78.123 |
| Conventional adaptive | 3,201,258 | 95.861 | 111.221 | 118.173 | 162.326 |

The generated-trace median CPU reductions are **75.15%** for packed replay and **79.08%** for the prepared send, compared with the previous runtime. The corresponding paired geometric-mean CPU speedups are 3.84× and 4.74×. The previous runtime already sent 747 bytes: this optimization preserves that byte benefit while reducing compute.

Other cases show both the improvement and its limits. Each cell below is **median CPU / median elapsed**; prepared target times exclude its separately charged construction.

| Actual target size | Case | Previous runtime | Packed runtime | Prepared runtime |
|---|---|---:|---:|---:|
| 14,851,384 B | One-bit edit | 566.940 / 602.545 | 138.861 / 163.562 | 118.829 / 134.258 |
| 14,851,384 B | Unrelated | 293.296 / 322.541 | 79.658 / 104.309 | 66.262 / 86.473 |
| 142,738 B | Generated | 3.441 / 13.469 | 3.088 / 11.075 | 2.739 / 10.090 |
| 142,738 B | One-bit edit | 4.900 / 13.464 | 4.406 / 12.993 | 2.735 / 12.175 |
| 142,738 B | Unrelated | 2.873 / 11.020 | 3.159 / 11.583 | 1.411 / 10.073 |

All ten large-case CPU pairs improved for both candidate arms. Small-case results are less consistent: packed replay improved CPU in 9/10 generated pairs and 6/10 edited/unrelated pairs; prepared improved 8/10 pairs in each small case. In particular, **small unrelated packed median CPU and elapsed time increased**. Cold small edited elapsed medians also increased: previous 26.072 ms, packed 26.845 ms, prepared 26.440 ms. These observations remain in the record. With ten trials, the reported nearest-rank p95 is the maximum observation; it is not a precise population tail estimate.

The large one-bit edit still costs 766 body bytes. Unrelated content selects raw and costs 14,883,938 bytes for every arm; prepared CPU of 66.262 ms remains above raw's 33.653 ms and conventional adaptive's 59.922 ms. On the small generated case, prepared CPU of 2.739 ms remains above raw's 1.092 ms and zstd's 1.395 ms. Byte-first selection is not a promise of minimum CPU or latency.

Preparation and cache setup are explicit costs:

| Trace size | Median preparation CPU / elapsed | Retained prediction/checkpoint/setup buffers | Actual uses |
|---|---:|---:|---:|
| 142,738 B | 0.360 / 0.361 ms | 143,153 B | 3 |
| 14,851,384 B | 19.228 / 19.344 ms | 14,854,835 B | 3 |

Each prepared object was actually reused for generated, edited and unrelated targets into separate fresh receiver stores. The retained-byte figure excludes fixture buffers, endpoint memory, struct/allocator overhead and send scratch; it is not RSS or peak memory. Averaging the measured preparation plus those three target transfers gives median per-use CPU/elapsed of **2.547 / 10.931 ms** for the small workload mixture and **107.332 / 126.288 ms** for the large mixture. This amortization is specific to those three actual sends.

For a cold large generated transfer, charge the checkpoint seed, preparation and target: **4,630 body bytes, 138.355 ms CPU, 168.734 ms elapsed**. The edited counterpart is **4,649 bytes, 138.800 ms CPU, 167.114 ms elapsed**. These are medians of per-trial sums, not sums of separately reported medians. The practical cold raw/zstd comparison uses their target-only cost because they do not require the experimental checkpoint seed. The raw seed rows remain available for every arm. Application trace generation is separately measured and is not silently added to ordinary file-transfer cost or credited as eliminated work.

The phase measurements explain the remaining compute cost. In the large generated case, receiver-handler median CPU fell from **240.799 ms to 31.544 ms** in the packed arm. Prepared target sends still spend a median **84.863 ms outside the exchange**, versus 32.180 ms in the receiver handler. The former includes sender selection, residual/compression candidates, hashing and encoding; this profile does not isolate those individually. These phase intervals are nested within total CPU and must not be added again. Future sender selection work has a measured motivation, but no further optimization or result is claimed here.

The [raw paired observations](evidence/2026-10-30-replay-cpu/paired.jsonl), [baseline phase profile](evidence/2026-10-30-replay-cpu/baseline-profile.jsonl), [full Rust-produced summary](evidence/2026-10-30-replay-cpu/summary.json) and [run receipt](evidence/2026-10-30-replay-cpu/run.json) retain all trials, exactness checks, hashes, preparation, seeds, nested timing and cleanup. Owned builds/tests were idle during timing; unrelated machine activity was uncontrolled. No new WAN run, energy measurement, MVM integration or FIPS qualification occurred.
