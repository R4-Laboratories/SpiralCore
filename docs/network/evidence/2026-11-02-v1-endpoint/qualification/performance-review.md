# Focused v1 compatibility performance assessment

Reviewed 2026-11-02 by the roster-eligible performance-profiler role. This is a
read-only assessment of the completed registered campaign, not a new experiment
or an optimization. No runtime source changed during this assessment.

The retained [registration](../registration.json), [analysis](../analysis.json)
and [measurement contract](../measurement-contract.md) identify the frozen
legacy `a9f038c49bcaf69ce7d6780970dadd412a31a573` and candidate
`f45c6c27a5062fc7123af6dd3f620d872f170bac` binaries. There are three matched trials
per cell, two stages and two fixture tiers: 24 sessions total. All three trials
remain included as registered; none was discarded as warmup or noise. These are
local unshaped Linux/Firecracker/vsock sessions, not WAN measurements.

## Observed session results

Times are medians in milliseconds. Percent changes are the median of each
matched trial's v1/legacy ratio, which can differ from the ratio of the two
displayed medians. Session CPU includes preparation and both paid setup objects.

| Stage / nominal tier | Sender CPU legacy → v1 | Paired CPU change | Session elapsed legacy → v1 | Paired elapsed change |
|---|---:|---:|---:|---:|
| Guest/host, 1 MiB | 90.361 → 102.932 | +13.53% | 191.564 → 216.173 | +11.43% |
| Two guests, 1 MiB | 90.584 → 101.646 | +12.87% | 173.423 → 181.173 | +2.51% |
| Guest/host, 16 MiB | 1,171.032 → 1,180.098 | +0.76% | 1,681.790 → 1,684.890 | +1.21% |
| Two guests, 16 MiB | 1,177.301 → 1,176.205 | −0.09% | 2,038.226 → 2,045.014 | +0.33% |

The small-tier sender CPU regression is real in these observations and must not
be described as overhead-free. Larger-tier session sender CPU is approximately
flat in this descriptive sample. Receiver-processing paired CPU changes are
+1.50%, +0.23%, +2.22% and −0.22%, respectively, in the table's row order.

## Localization available in the existing receipts

The receipts already divide each operation into sender CPU spent inside the
exchange callback and the remaining sender CPU. This provides phase localization,
not a sampled stack profile or a proven root cause.

- Guest/host 1 MiB trials 1 and 2 increased total sender CPU by 13.188 ms and
  12.406 ms. Summed operation nonexchange CPU increased by 11.841 ms and
  11.733 ms; exchange CPU increased only 0.638 ms and 0.603 ms. The schema-drift
  operation alone increased 12.832 ms and 13.093 ms. Trial 0 increased only
  2.335 ms overall.
- Two-guest 1 MiB trial 0 increased sender CPU by 14.230 ms. The residual
  operation increased 15.003 ms; its nonexchange portion changed from 2.013 ms
  to 17.025 ms while exchange CPU changed from 1.603 ms to 1.594 ms.
- Two-guest 1 MiB trial 2 increased sender CPU by 11.590 ms. Schema-fit increased
  10.858 ms, with nonexchange CPU changing from 12.109 ms to 21.971 ms. Trial 1
  increased only 0.275 ms overall.
- Small-tier setup CPU stays around 11–12 ms; it does not explain the dominant
  operation-specific increases above. Exact-repeat operations do not show a
  consistent increase across all matched cells.

The affected operations and trials differ. The data does not establish that
profile checks, sender extraction, compiler code generation, allocator behavior,
or host/guest execution effects caused the increases. Source inspection shows
the sender extraction and additional profile checks, without codec algorithm
changes; that is a possible investigation boundary, not causal evidence.
No flame graph, sampled stack profile, hardware-counter run or controlled
single-change ablation was collected. Calling the regression mere noise or
assigning it to code generation would exceed the evidence.

## Compatibility and release recommendation

All registered object identities, lengths, selected modes, planned application
frame totals and actual carrier-frame counts match across the paired runs.
Planned totals are not actual transferred frame-byte counts. Actual encrypted
stream-byte paired median changes range from −0.0113% to +0.0478% across the four
cells; per-session randomized envelope identities/encoding vary. Exact reopen
and the registered two-guest keyless reboot checks passed.

The registered milestone is protocol/API compatibility with recorded cost; it
does not contain a no-regression performance gate. This review therefore does
not block that compatibility release, provided the small-tier regression and
the absence of a new WAN/speedup claim are published prominently. It does not
approve the stronger claim that v1 has zero performance overhead.

The next performance investigation should instrument sender nonexchange
selection/encoding phases for the small schema-fit/schema-drift/residual
workloads on these exact binary/source identities, then isolate one change at a
time. Preserve the current full-profile byte counts and exact readback contract.
No broad campaign or speculative optimization is recommended from this sample.
