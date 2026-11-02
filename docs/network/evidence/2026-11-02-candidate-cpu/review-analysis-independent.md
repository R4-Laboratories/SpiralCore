# Independent candidate CPU campaign arithmetic

Reviewer: `mvm_tests`. Date: 2026-11-02. Verdict: **PASS; no arithmetic correction required.**

Candidate `343248065628176f76841e98608a4a4badf747dd`, baseline
`4b8cb79fe5fca9e9527a2923924c887d9e7ddbf8`, run `41399314329`.
Root separately verified source, binaries, workflow and artifact custody. This
review recomputed results from raw `client.json`, `server.json` and `capture.json`
under the retained local `candidate-evidence/campaign` directory, using independent
jq extraction and JavaScript arithmetic rather than invoking the Rust analyzer.
No builds, tests, new campaign or production edits were performed.

## Reconciliation

There were **zero mismatches** against `analysis.json` across:

- 18 session IP/logical-byte, summed-process-CPU and client-wall rows and their
  three-trial medians/paired ratios.
- All 264 operations joined to receiver batches by session and transfer ID:
  request/response deltas, frame count attribution, sender CPU, receiver-handler
  CPU, their sum, wall time and batch counts. No orphan receiver batches.
- All 252 paid-group cumulative rows and 252 connection-prefix rows, including
  both auxiliary publications, group preparation, namespace setup and earlier
  groups where applicable.
- Six before/after pairs of 16 operations each. Identity/length, mode, publication
  path, frame/data bytes, requests, sent/received batches and actual logical
  deltas are identical. Every Combined session uses **15,950,640 logical bytes**.

The raw dose is 216 deliveries, 1,717,047,288 requested bytes, 216 sends,
24 checkpoint and 24 schema publications. All 18 receiver receipts assert exact
comparison of eight expected files after reopening. Artifact authentication and
the actual readback implementation are covered separately, not inferred from
arithmetic alone.

## Complete-session arithmetic

Independent medians; CPU and wall are seconds. Process CPU is summed within a
trial before the median. Ratios below are also paired before taking the median.

| Link | Arm | Captured IP bytes | Process CPU | Client wall |
|---|---|---:|---:|---:|
| Shaped | Before | 17,237,338 | 1.130903 | 16.769147 |
| Shaped | After | 17,234,683 | 1.004654 | 16.632538 |
| Shaped | Whole-zstd | 41,294,515 | 1.659298 | 35.461769 |
| Unshaped | Before | 16,720,035 | 0.865372 | 1.006237 |
| Unshaped | After | 16,720,003 | 0.737390 | 0.838375 |
| Unshaped | Whole-zstd | 40,266,096 | 0.996329 | 0.975360 |

Shaped After versus Before reduces CPU **12.2066%** and wall **0.8146%**.
Against Whole, it reduces captured IP bytes **58.2649%**, CPU **39.7113%**,
and wall **53.0973%**. Unshaped reductions versus Before are **14.7892% CPU**
and **17.7394% wall**; versus Whole, **25.9893% CPU** and **14.0446% wall**.
Small captured-IP differences between the Combined versions are not attributed
to new codec savings because their complete logical traffic is identical.

## Operation gains, paid setup and losses

For shaped 14,851,384-byte objects, independent paired sender-CPU reductions
versus Before are **64.1759% generated**, **56.5022% residual edit**, and
**53.0074% opaque edit**. Their wall reductions are 14.1622%, 12.8725%, and
16.5395%. Selected logical bytes remain 771, 1,374 and 1,374 respectively.
The report's per-operation medians and near-1 MB generated example also match.

Both setup publications remain paid. Shaped After setup-only medians are:

| Tier | Logical bytes | Endpoint-stage CPU | Wall |
|---|---:|---:|---:|
| 1,047,202 bytes | 4,416 | 3.003 ms | 167.768 ms |
| 14,851,384 bytes | 4,140 | 22.628 ms | 186.457 ms |

Setup-plus-first costs 5,187 bytes / 336.504 ms for the small tier and
4,911 bytes / 461.988 ms for the large tier. The small tier still consumes
**15.8587% more endpoint-stage CPU than Whole** despite 20.2716% lower wall.
These intervals exclude connection setup and readback; complete-session costs
include them. The second tier's reduced setup bytes reflect actual repeated
auxiliary handling, not omitted publication.

Losses remain visible:

- Large shaped residual-repeat sender CPU increases **19.0886%**, endpoint-stage
  CPU **8.8210%**, and wall **1.7484%** versus Before. The path and 230 logical
  bytes are unchanged; no causal optimization claim is assigned to that variation.
- Large unrelated input sends the same 14,883,962 logical bytes in every arm.
  After still uses **25.1978% more sender CPU** and **0.2958% more wall** than
  Whole. Small unrelated sender CPU and wall remain 30.3048% and 16.6575% higher.
- Unshaped small unrelated After loses 7.1687% sender CPU and 5.1093% wall versus
  Before by paired ratios. No observation is removed to improve the conclusion.

## Report and limits

The reviewed result tables, paired percentages, stated losses and CPU/time
accounting in `docs/network/candidate-cpu-results-2026-11-02.md` agree with this
recomputation. Identities at review:

- `analysis.json`: `69b6efd388c8acfd418e528c05a52f079ca1aab26ef8b5631c22a00a933efef2`.
- Results report: `38119d4564f5bfb2c47cf1d3c7771362f679044faba1a6062821fd3ec481f858`.

Limits remain three descriptive matched trials on a calibrated local shaped
network, not Internet WAN or physical energy. Nested handler/operation CPU is
not added to process CPU or elapsed time. Per-operation logical bytes are not
per-operation captured IP bytes. The earlier native stage diagnostic remains a
separate measurement and is not pooled with this campaign.
