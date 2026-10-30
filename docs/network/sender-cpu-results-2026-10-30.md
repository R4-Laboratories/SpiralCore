# Sender selection and residual CPU — 2026-10-30

The new sender path preserves the exact object and transfer frames while avoiding provably losing codec work. For a **14,851,384-byte generated trace**, prepared sender CPU outside exchange fell from **81.552 ms to 1.640 ms**. Total sender-plus-receiver CPU fell from **111.437 ms to 35.039 ms**, and local elapsed time from **127.094 ms to 49.678 ms**, with the same **747 body bytes**. The edited trace retains its **766-byte** transfer. These results improve the existing packed/prepared runtime; they are not a new WAN or energy measurement.

The [registered comparison](evidence/2026-10-30-sender-cpu/measurement-contract.md) used baseline `11fb80931a97b06f3c1b40621b7e8c40710274e5` and frozen candidate [`d13db98d8b09bf9ce79a42f69a8ac60430668164`](https://github.com/R4-Laboratories/SpiralCore/commit/d13db98d8b09bf9ce79a42f69a8ac60430668164). Both ran identical Rust harness source, release/locked dependencies, `hardware-sha256`, Rust1.97.1, on Apple M1/macOS. There was one ten-trial campaign, randomized arm order, two unpadded trace sizes and three cases. **All 420 targets and 420 separately timed seeds passed full byte comparison after close/reopen.** All 140 owned worker stores were cleaned. Selected modes, body counts and normalized request/reply frame hashes matched across previous/optimized one-shot and prepared arms. No observation was discarded or performance campaign repeated.

The bounded baseline profile measured large generated/edited prefix compression at **59.388/67.554 ms CPU**, prepared residual encoding at **10.355/11.546 ms**, and target hashing at **7.622/7.356 ms**. These are independent three-trial API profiles, not additive phases of one operation. Actual conventional-adaptive sender time outside exchange was 77.560/74.297 ms. The profile and output checks are retained in [phases.jsonl](evidence/2026-10-30-sender-cpu/phases.jsonl) and [adaptive-profile.jsonl](evidence/2026-10-30-sender-cpu/adaptive-profile.jsonl).

The implementation changes are bulk equality/mismatch scans that preserve the residual recipe, cached immutable checkpoint/prediction identities used only after exact equality, and runtime-first selection with sound lower bounds. On the large favorable cases, the chosen runtime cost is 747/766 bytes, while lower bounds are **32,796 bytes for chunked zstd, 57,624 for sparse, and 886 for prefix**. Those candidates cannot win and are skipped. The prefix bound follows the frame/block format; it does not rely on a failed small output allocation. Where a bound cannot exclude a candidate, full encoding remains. Each pruned row records a lower bound separately from exact evaluated costs.

For the large generated target, medians were:

| Arm | Body bytes | Total CPU ms | Sender outside exchange ms | Elapsed ms |
|---|---:|---:|---:|---:|
| Previous one-shot | 747 | 130.825 | 101.153 | 146.683 |
| Optimized one-shot | 747 | 60.313 | 29.968 | 75.326 |
| Previous prepared | 747 | 111.437 | 81.552 | 127.094 |
| Optimized prepared | 747 | 35.039 | 1.640 | 49.678 |
| Raw | 14,883,938 | 36.599 | 10.976 | 53.814 |
| zstd | 3,513,668 | 39.595 | 20.675 | 53.861 |
| Conventional adaptive | 3,201,258 | 99.187 | 82.082 | 112.653 |

The optimized prepared path beat the previous prepared path in all ten large generated and edited CPU pairs. Its paired geometric-mean speedups are **3.51× total CPU / 52.34× sender CPU** for generated content and **3.27× / 10.70×** for the edit. Generated prepared transfers also beat raw CPU and elapsed time in **8/10 pairs** (paired geometric means 1.084× CPU, 1.094× elapsed), and beat zstd CPU in 10/10 and elapsed time in 9/10. This is a modest conditional local advantage over raw, not a universal speed claim. Prepared timing assumes the prediction already exists; its construction cost is below.

Median total CPU across the other cells shows the limits:

| Actual target size | Case | Previous → optimized one-shot, ms | Previous → optimized prepared, ms |
|---|---|---:|---:|
| 14,851,384 B | One-bit edit | 134.322 → 57.869 | 113.088 → 39.799 |
| 14,851,384 B | Unrelated | 78.981 → 77.910 | 63.122 → 67.137 |
| 142,738 B | Generated | 3.346 → 2.866 | 2.815 → 2.458 |
| 142,738 B | One-bit edit | 3.457 → 3.203 | 2.950 → 2.897 |
| 142,738 B | Unrelated | 1.967 → 1.874 | 1.519 → 1.588 |

For the large edit, prepared elapsed time improved from 127.959 to **55.629 ms**, but raw remained lower at **33.574 ms CPU / 50.935 ms elapsed**, and zstd at **37.479 / 51.711 ms**. Unrelated prepared transfers retained the full **14,883,938 bytes**, regressed in median CPU and elapsed time (**75.909 → 81.308 ms elapsed**), and improved CPU in only **3/10 pairs**. Their paired geometric-mean CPU ratio is still 1.011× because individual retained ratios affect that aggregate; it does not establish a consistent improvement. Neither unrelated prepared CPU nor elapsed time beat raw/zstd in any pair.

Small-case latency is also mixed. Prepared edited elapsed time increased **11.333 → 14.079 ms**, and prepared unrelated increased **10.260 → 12.596 ms**. Small prepared edited CPU improved in only 5/10 pairs. There are substantial tail observations in the retained data: large generated previous/optimized prepared p95 CPU is **272.174/63.644 ms**, and p95 elapsed **385.896/88.480 ms**. With ten observations, nearest-rank p95 is the maximum, not a precise population tail estimate. No outlier was removed and no cause is asserted for host variation.

Prepared setup and memory costs remain explicit:

| Trace size | Previous → optimized median preparation CPU | Optimized preparation elapsed | Owned buffers | Struct size, previous → new |
|---|---:|---:|---:|---:|
| 142,738 B | 0.383 → 0.457 ms | 0.460 ms | 143,153 B | 48 → 112 B |
| 14,851,384 B | 19.046 → 19.683 ms | 19.971 ms | 14,854,835 B | 48 → 112 B |

Each prepared object was actually reused for generated, edited and unrelated sends. The median per-use cost of preparation plus those three targets is **53.998 ms CPU / 68.506 ms elapsed** for the optimized large mixture, versus **101.912 / 116.763 ms** previously. This is specific to those three actual sends and excludes their separately retained receiver seed costs. Buffers plus struct size exclude allocator overhead, fixture memory, endpoint memory and other scratch; they are not RSS or peak memory.

For a practical **cold** large generated transfer, charge seed plus preparation plus target: **4,630 body bytes, 55.447 ms CPU, 84.816 ms elapsed**. The edited counterpart is **4,649 bytes, 60.183 ms CPU, 86.198 ms elapsed**. These are medians of per-trial sums, not sums of separately reported medians. Raw/zstd need no checkpoint seed, so their cold comparison uses target-only cost; they still win cold local CPU/time. Fixture generation is separate and is not credited as eliminated file-transfer work.

The harness uses real serialization and durable endpoint handling in one process. CPU includes both endpoints once; exchange and receiver intervals are nested subsets. Reported bytes are HTTP-body equivalents, not packet totals or WAN traffic. To compare frames, both builds retain already serialized buffers and hash them **after** transfer timing; maximum retained transcript length was 14,883,922 bytes. This is matched benchmark overhead, separate from production prediction retention. Candidate pruning receipts are also serialized outside the timer. Owned builds/tests were idle during the campaign; unrelated machine activity was uncontrolled.

The [raw observations](evidence/2026-10-30-sender-cpu/paired.jsonl), [full summary](evidence/2026-10-30-sender-cpu/summary.json), [source/executable pins](evidence/2026-10-30-sender-cpu/source-pins.json) and [run receipt](evidence/2026-10-30-sender-cpu/run.json) preserve the counts, controls, negatives and setup costs. The source snapshot and two executable pins were preserved; existing build targets were reused under a 4 GiB free-space guard. No new WAN run, energy measurement, MVM integration or remote artifact upload was performed by this experiment.

Retained qualification reports **242 all-target tests passed, two ignored; one doctest; 46 focused tests; 81 coverage tests; and final Clippy passed**. Moving only the identity test module resolved the final lint, followed by a passing identity test. File line coverage measured before that test-only relocation was client82.27%, delta96.875%, selection97.537%, shared-transfer96.875% and prefix95.755%; these are bounded coverage observations, not a branch or whole-repository assurance claim. The shared benchmark's four focused tests passed after the recorded frame-normalization RED. All campaign timing finished before subsequent offload/compression of older experiment directories.

```text
# Same harness against the pinned baseline and candidate; compile flags are in source-pins.json.
replay_cpu run BASELINE_EXE CANDIDATE_EXE NEW_PAIRED_JSONL NEW_CACHE
replay_cpu summarize PAIRED_JSONL NEW_SUMMARY_JSON
```
