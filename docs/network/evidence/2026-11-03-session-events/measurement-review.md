# Independent C8 measurement arithmetic review

Reviewed existing Rust-generated records only; no benchmark, build, product edit
or remote action. Machine-readable values and every per-pair metric delta are in
[summary.json](summary.json). Raw records and all failed functional gates remain.

All **22 host cells / 11 before-after pairs** passed exact verification and owned
store cleanup: **192 publications and 192 independent readbacks**. Every pair
has identical eight target kinds, lengths and object identities. Actual target
lengths are 1,047,202 and 14,851,384 bytes; Combined additionally publishes its
3,382-byte checkpoint and 48-byte schema. Directional counters and recorded wire
totals agree. Peak host process RSS was **397,512,704 bytes**, below the registered
1 GiB sampled ceiling. This is measured process high water, not a total-memory
enforcement result.

The four native cells passed **36 publications and 36 additional exact keyless
reboot readbacks** with matching lengths, identities, namespaces and endpoint/
relay counters. Both versions retain two links/two slots/524,288 reserved bytes,
peak at that ceiling, and finish with zero usage/refusals/fencing. Physical
accounted stream bytes equal twice logical mailbox totals.

## Small host pairs: primary statistic and individual losses

Percent change is `100 * (after / before - 1)`. The primary small statistic is
the **median of three matched percentage changes**, not the percentage change
between independently selected column medians. Positive values are increases.

| Arm, requested 1 MiB | Aggregate process CPU | Verified wall | Sender ThreadCPU | Receiver ThreadCPU |
|---|---:|---:|---:|---:|
| Raw with exact-object warm probe | +2.59% | +4.02% | +4.73% | -0.54% |
| Whole zstd | -2.64% | +0.56% | -4.20% | +0.53% |
| Combined | -4.06% | -4.19% | -4.60% | -2.66% |

Every individual primary CPU/time pair is retained here, including losses hidden
by an arm median. The two thread columns are independent domains, not additive
attributions of the process-clock difference.

| Arm | Trial | Process CPU | Verified wall | Sender ThreadCPU | Receiver ThreadCPU |
|---|---:|---:|---:|---:|---:|
| Raw | 0 | -1.69% | -1.06% | -1.72% | -1.62% |
| Raw | 1 | +6.43% | +7.27% | +4.73% | +8.51% |
| Raw | 2 | +2.59% | +4.02% | +5.19% | -0.54% |
| Whole zstd | 0 | -4.18% | -3.00% | -8.16% | +0.53% |
| Whole zstd | 1 | -0.34% | +0.56% | -4.20% | +4.31% |
| Whole zstd | 2 | -2.64% | +1.12% | -2.63% | -2.66% |
| Combined | 0 | -4.06% | -4.19% | -4.60% | -3.50% |
| Combined | 1 | -1.71% | +0.34% | -4.03% | +0.80% |
| Combined | 2 | -6.87% | -5.03% | -10.46% | -2.66% |

Small RSS increased in Whole trials 0/2 (+0.87%/+1.38%) and Combined trial 2
(+1.38%). Whole trial 1 wire bytes increased 0.01445%. The JSON retains all
positive/negative byte, RSS, setup, offline-readback, cleanup and lifecycle deltas.
For example, Whole's ratio of wall column medians is a 0.41% decrease, while its
median matched wall change is the **0.56% increase** above; do not substitute them.

## Large host cells: requested 16 MiB, one pair per arm

| Arm | Before wire bytes | After wire bytes | Before process CPU | After process CPU | Before verified wall | After verified wall |
|---|---:|---:|---:|---:|---:|---:|
| Whole zstd | 107,355,277 | 107,290,442 | 7.073526 s | 7.102022 s | 6.696404 s | 6.868626 s |
| Combined | 62,505,645 | 62,508,810 | 4.693671 s | 4.580914 s | 4.488073 s | 4.427442 s |

Whole process CPU/wall increased **0.40%/2.57%**; Combined decreased
**2.40%/1.35%**. Receiver ThreadCPU increased for both: **1.72% Whole** and
**1.57% Combined**, despite sender ThreadCPU decreasing 0.69%/5.65%. Combined
wire bytes increased **0.00506%** and RSS **0.106%**. Its offline readback CPU
increased **2.04%** and cleanup CPU **16.16%** (2,580 to 2,997 microseconds).
Whole provisioning CPU increased **32.15%** (818 to 1,081 microseconds).
These separately timed costs remain in the JSON rather than being pooled.

## Native local two-guest cells, one trial each

| Version | Arm | Logical bytes | Sender timed CPU | Receiver timed CPU | Relay process CPU | Lifecycle wall |
|---|---|---:|---:|---:|---:|---:|
| Before | Whole zstd | 107,320,914 | 4.515531 s | 3.755624 s | 0.117692 s | 8.896883 s |
| Before | Combined | 62,510,789 | 3.028168 s | 2.485406 s | 0.084667 s | 6.428218 s |
| After | Whole zstd | 107,299,202 | 4.384472 s | 3.639661 s | 0.122823 s | 8.652877 s |
| After | Combined | 62,547,228 | 3.005620 s | 2.502463 s | 0.077445 s | 6.385582 s |

Before/after Whole sender CPU/lifecycle decreased **2.90%/2.74%**, while relay
CPU increased **4.36%**. Combined sender CPU/lifecycle decreased **0.74%/0.66%**,
while receiver CPU increased **0.69%** and logical/physical bytes **0.05829%**;
its relay CPU decreased 8.53%. Current Combined versus current zstd used
**41.71% fewer logical bytes, 31.45% less timed sender CPU and 26.20% less
lifecycle time** across this particular mixed sequence, including setup.

Aggregate advantage is not a universal per-object improvement. Current native
per-workload losses versus current Whole are explicit:

| Workload | Whole sender CPU | Combined sender CPU | CPU increase | Whole operation wall | Combined operation wall | Wall increase | Combined byte change |
|---|---:|---:|---:|---:|---:|---:|---:|
| Schema fit | 621,303 us | 672,490 us | +8.24% | 1,070,270 us | 1,091,538 us | +1.99% | -1.00% |
| Schema drift | 614,791 us | 730,684 us | +18.85% | 1,043,368 us | 1,170,848 us | +12.22% | -0.64% |
| Unrelated cold target | 1,120,166 us | 1,130,681 us | +0.94% | 1,905,978 us | 1,907,297 us | +0.07% | +0.067% |

Repeat and opaque-edited-repeat wire totals also increased 0.310%/0.236%,
despite lower measured CPU/wall. All eight current workload comparisons are in
the JSON. The unrelated 14,851,384-byte cold target used roughly **31.3 MB of
logical canonical traffic** in either arm. This is a canonical encrypted MVM
baseline, not a comparison with generic secured raw/TLS or a low-overhead cold
transport claim. Generated/residual/repeat gains depend on the registered shared
fixture/setup/cache sequence; eight positions are not eight unrelated cold files.

## Identity, timer domains and interpretation

After source is `5c41d6ac2d9b40e6eead62e201d5e9b020f96019`, bound by the
284-entry qualified crate manifest (SHA-256
`cb583c9335baecfe0cef7579e09d7e545a322b97b4b6478f13c5ed4387f961fc`).
Host before is the pinned C7 `91baf28` endpoint binary `879111c9...`; native before
is protected duplex `62cb6042`. Relay changes are absent from direct broker_objects
execution; native pairs use duplex on both sides. Exact before/after host, guest
and probe binary hashes are retained in the JSON and original binary manifests.

Host aggregate process CPU includes both actors and setup; per-role ThreadCPU
is separate. Overlapping operation ProcessTime fields cannot be summed as endpoint
CPU. Verified host wall includes compute/waits and receiver readback; offline
readback/cleanup and lifecycle are separately labeled. Native endpoint CPU sums
timed setup/operation intervals, including fixture/service/codec work within them,
and excludes untimed gaps, JSON and final independent verification. Native
lifecycle starts after journal/link admission and includes boot, transfer,
keyless verification and teardown. Relay CPU and nested worker/VMM CPU stay
separate. Planned application bytes never substitute for actual wire bytes.

The parent verified compact archive custody and all **73 members** before removing
only four successful owned native disks (**1,073,741,824 apparent file bytes**).
This arithmetic review checked the cleanup record, not custody hashes again.
Host successful cleanup is separately reported by the unchanged Rust engine.
Failure records were preserved; no measurements were dropped or rerun here.

The frozen source changes reliability, not codec policy or a promised CPU
optimization. The fixed exploratory order, shared Wick and small sample prevent
causal attribution, statistical speedup or noninferiority claims. These are local
Linux native/direct-TCP observations, not WAN or fleet evidence. No energy,
send-only time, automatic retry/resume, retention beyond 2,048 Calls or pipeline
above one is established by these measurements.
