# Independent unrelated-object campaign arithmetic

Reviewer: `mvm_tests`. Date: 2026-11-02. Verdict: **PASS; no arithmetic correction required.**

Run `41446807449`; measured candidate
`c7ee11fdde26c922da9ae6d9574a91f88b2a3573`; baseline
`4a8ec79c9b185c85407e9767653dbcc4af97bf7e`. Root separately verified original
artifact/source/binary/workflow custody. This bounded review independently
extracted raw client, server and capture JSON from the retained campaign and
recomputed arithmetic with jq and JavaScript. No build, test, new campaign,
production edit or expanded audit was performed.

## Reconciliation

**Zero mismatches** against `analysis.json` across all 18 complete-session rows,
264 transfer-ID-matched operation/receiver joins, 252 paid-group cumulative rows,
and 252 connection-prefix rows. Channel deltas, requests/batches, sender CPU,
receiver-handler CPU, their sum, and wall time reconcile. No orphan receiver
batches were found. Both auxiliary publications, group preparation, namespace
setup and preceding groups are charged in the appropriate cumulative rows.

All six before/after pairs preserve their 16 operations' target identity/length,
mode, publication path, frame/data/count receipts and logical deltas. Every
Combined session uses **15,950,640 logical channel bytes**. The recomputed dose
is 216 deliveries, 1,717,047,288 requested bytes, 216 sends, 24 checkpoints and
24 schema publications. Every receiver receipt reports eight expected files
compared exactly after reopen; authentication of those receipts is a separate
custody responsibility.

## Complete mixed sessions

Independent three-trial medians; CPU and wall are seconds.

| Link | Arm | Captured IP bytes | Summed process CPU | Client wall |
|---|---|---:|---:|---:|
| Shaped | Before | 17,235,006 | 1.288825 | 16.740083 |
| Shaped | After | 17,235,137 | 1.290544 | 16.777292 |
| Shaped | Whole-zstd | 41,295,873 | 2.212616 | 35.824993 |
| Unshaped | Before | 16,715,127 | 0.828217 | 0.976193 |
| Unshaped | After | 16,714,951 | 0.808005 | 0.923386 |
| Unshaped | Whole-zstd | 40,263,132 | 1.182657 | 1.117460 |

Shaped After versus Before has **1.3168% lower paired process CPU**, but
**0.3539% higher wall time**. This is not contradicted by After's slightly larger
CPU median: paired ratios are calculated within each trial before their median,
not by dividing the table's independent medians. Shaped versus Whole reductions
are **58.2643% captured IP bytes, 41.9271% CPU, and 53.1688% wall**. These retain
the existing runtime/cache advantages, rather than attributing them all to this
new scan optimization. Unshaped After versus Before reduces CPU **2.4404%** and
wall **5.4610%**. Captured-IP variation is not a new codec saving.

## Unrelated objects and retained losses

Shaped unrelated-transfer medians:

| Object size | Arm | Logical bytes | Sender CPU ms | Client wall ms |
|---|---|---:|---:|---:|
| 1,047,202 | Before | 1,049,988 | 29.375 | 1,249.960 |
| 1,047,202 | After | 1,049,988 | 28.768 | 1,249.906 |
| 1,047,202 | Whole | 1,049,988 | 25.881 | 1,070.871 |
| 14,851,384 | Before | 14,883,962 | 362.910 | 12,966.031 |
| 14,851,384 | After | 14,883,962 | 340.230 | 12,991.936 |
| 14,851,384 | Whole | 14,883,962 | 273.597 | 12,916.361 |

Large unrelated After has **2.3532% lower paired sender CPU** than Before and
**0.1998% higher wall time**. It still costs **27.3940% more sender CPU** and
**0.3329% more wall** than Whole at identical logical bytes. Small unrelated
After reduces sender CPU 2.3319% versus Before but increases wall 0.1125%; against
Whole, sender CPU and wall remain 14.8148% and 16.7053% higher. The unrelated-
object overhead is therefore not resolved, and this campaign does not establish
an incremental shaped-link speedup.

Setup remains paid: After's shaped zero-delivery rows contain 4,416 logical bytes
for the small tier and 4,140 for the large tier, at median wall times 168.122 ms
and 192.422 ms. Through-six rows remain 1,058,383 and 14,892,081 bytes. The smaller
second setup reflects actual repeated auxiliary handling; neither checkpoint nor
schema is silently free.

## Evidence and limits

Reviewed `analysis.json` SHA-256:
`a97182d496dac0173815ee2d5d67f00ebe282489a468e2f0830012c86fde9f02`.
The first retained analysis invocation exited 1 because an older executable did
not recognize the new schema. The explicit fresh-source release invocation exited
0. This was an analysis-tool invocation correction, not a changed campaign or
removed observation; both receipts remain retained.

Three descriptive matched trials do not establish tails, confidence, physical
energy, or Internet-WAN behavior. Process CPU is not added to nested handler CPU
or elapsed time. Logical operation bytes are not per-operation captured IP bytes.
Native scan timings remain a separate diagnostic rather than substituted network
cost. All trials and losing cases remain in the analysis.

## Final report check

Reviewed `docs/network/unrelated-gateway-cpu-results-2026-11-02.md`, SHA-256
`179682f210846c5d5cf53547e36f9a61de93b917b1fd820ed066d359338c85ea`.
Its network tables, matched-ratio percentages, object/session counts, preserved
logical-byte claims and opaque-edit comparisons agree with the independent
recomputation. No numeric correction is required. The report explicitly retains
the shaped elapsed regressions and unrelated-object losses to Whole, distinguishes
paired ratios from ratios of medians, and limits this dose to three descriptive
trials on a shaped single-runner network. It does not claim an incremental WAN
completion-time win, physical-energy qualification or a new MVM/TUN measurement.
Native-stage diagnostics and artifact-custody claims remain separately verified
by their owners; this addendum does not broaden the arithmetic review.
