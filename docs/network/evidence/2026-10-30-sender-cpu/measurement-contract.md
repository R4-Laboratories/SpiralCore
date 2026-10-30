# Sender CPU comparison

SC-OPT-010 reuses `examples/replay_cpu.rs` and its existing serializer, endpoint, fixture, source-pin, exact-reopen and summary machinery. Both benchmark binaries enable `RUSTFLAGS='--cfg sender_cpu --cfg replay_cpu_prepared'`; the candidate additionally enables `--cfg sender_candidate` to record its new `Selection.pruned` receipts. This compile branch only reads the added receipt field and does not select production behavior. The previous six-arm experiment remains available without `sender_cpu`; its historical evidence is unchanged.

The baseline is `11fb80931a97b06f3c1b40621b7e8c40710274e5`. Its transfer crate and object-store dependency were archived before production edits, SHA-256 `c47e3f5d10d02400dc86564ef25a5fccfb99b780553fce3c238d6a35466673c0`. The same final benchmark source is built against that preserved source and the candidate, with Rust1.97.1, release optimization, locked dependencies and `hardware-sha256`. Each worker records executable and algorithm source hashes; the frozen candidate commit is recorded before acceptance timing.

The baseline phase profile uses `examples/sender_profile.rs`: three trials × two sizes × three cases × five independent API phases. It measures `object_id`, prepared residual encoding, delta encoding against the prediction, delta encoding against the checkpoint and whole-file zstd-prefix encoding. Six preparation costs are separate. The 90 phase rows retain returned bytes or explicit raw fallback, with reconstruction verification outside timing and returned buffers dropped after timing. These independent phase durations must not be summed as if they partition one transfer.

Six conventional-adaptive baseline workers additionally retain 18 actual target transfers and 18 seeds, including total CPU and the nested exchange duration. They use the earlier frozen prepared-capable executable `4ea4437b52b4b84111250e0fb7c2a0a031135705d83477db1d27a34b6218ac21`, whose production source hashes match this baseline. Its source/executable identity is recorded in each worker. This is a bounded phase profile, separate from acceptance, with no discarded warm-up.

The one acceptance campaign uses the same unpadded 142,738-byte and 14,851,384-byte canonical traces (width48/steps243 and width4096/steps4096), generated/one-bit-edited/unrelated cases and fixture seeds as SC-OPT-009. It runs ten randomized paired trials with seven arms:

| Recorded arm | Meaning | Binary |
|---|---|---|
| `baseline` | Previous one-shot runtime transfer | Baseline |
| `baseline-prepared` | Previous prepared runtime transfer | Baseline |
| `packed` | Optimized one-shot runtime transfer; label retained for harness compatibility | Candidate |
| `prepared` | Optimized prepared runtime transfer | Candidate |
| `raw` | Raw control | Candidate |
| `zstd` | Chunked zstd control | Candidate |
| `adaptive` | Conventional adaptive control | Candidate |

Each worker seeds a checkpoint by separately timed raw transfer into a new receiver store for each case. A prepared worker constructs one prediction and actually reuses it for the three different targets. There are **140 workers, 40 preparations, 420 target transfers and 420 seeds**, with exact close/reopen comparison after every operation. All 140 owned worker stores must be cleaned after success; failures retain their outputs and stores. No target cache is prepopulated, DAG warming is disabled and no exact-repeat probe is used. CPU and wall timers retain the previous scope: sender work through verified durable acknowledgment, including receiver CPU once. Receiver and exchange timings are nested subsets, not additive costs.

Sender mode retains the already serialized request and reply vectors until the timed transfer finishes. It then hashes normalized frame kind, sequence and body, excluding differing transfer IDs and nondeterministic timing metadata. Batch boundaries are bound. These hashes, selected modes, target identity, exact output and total body bytes must match across previous/optimized one-shot and prepared runtime arms. No payload clone or transcript hash is added inside the timed exchange; a few vector moves retain bounded serialized buffers. This matched harness memory overhead is recorded as retained transcript bytes and is not charged as production prepared-prediction memory.

The summary reports total CPU/wall, sender CPU outside exchange, receiver/exchange subsets, bytes, per-trial paired old/new ratios, preparation, seeds and actual-three-send amortization. Practical cold comparisons charge checkpoint seed plus construction plus target for prepared paths, and target only for raw/zstd. Compound medians are medians of per-trial sums. Prepared retention records owned buffers and struct size separately, excluding allocator overhead and other process memory.

All observations, including slower pairs and regressions, are retained. There is no performance rerun, outlier removal, extra analyzer project or acceptance-workload adjustment based on results. No timing overlaps owned builds/tests/other experiments. The comparison is local serialized durable transfer, not HTTP/TCP/WAN latency or energy. Body-equivalent bytes exclude HTTP headers, TLS/IP/TCP and retransmissions. The baseline profile measures the original full-prefix cost. The candidate may skip prefix only when the encoded-format block-count lower bound proves it cannot beat the incumbent; otherwise it retains full encoding. Pruned receipts are lower bounds, not measured exact candidate costs. No universal CPU win is assumed.

```text
sender_profile NEW_PHASE_JSONL
replay_cpu run BASELINE_EXE CANDIDATE_EXE NEW_PAIRED_JSONL NEW_CACHE
replay_cpu summarize PAIRED_JSONL NEW_SUMMARY_JSON
```
