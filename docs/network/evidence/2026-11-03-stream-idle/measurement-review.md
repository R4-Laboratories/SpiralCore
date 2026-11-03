# Strict-transfer confirmation — independent performance/content review

Verdict: **PASS for the registered records and scoped claims below**. This is
quality-gate Step 2 performance/content confirmation, reusing existing Rust
records and jq arithmetic only. No benchmark, build, product edit, remote action
or expanded audit was performed. Full precision, separate metric domains and
every per-pair delta remain in [summary.json](summary.json).

**These transfers use legacy strict receive. They do not call receive_or_idle
and do not measure Idle wake CPU, idle polling, retry timers or resume.**
They confirm strict-transfer correctness/costs after a shared framing refactor.
Dedicated functional tests and separate qualification establish the additive API.

All **12 host cells / six matched pairs** passed exact verification, directional
counter/total agreement and engine cleanup: **108 publications and 108 independent
readbacks**. Each pair has identical eight target kinds, lengths and object IDs.
Each target is **1,047,202 actual bytes**, at requested 1 MiB. Combined also
publishes its 3,382-byte checkpoint and 48-byte schema.

Peak host process RSS was **36,823,040 bytes**, within the registered 1 GiB
sampled ceiling. These are small host cells only; comparing that peak with a
previous batch containing large cells would not establish a memory improvement.

## Host paired costs and retained losses

Primary statistic: median of three paired `100*(after/before-1)` values.
Column medians are separately retained in JSON and are not substituted for that
statistic. CPU/thread domains remain separate; positive values are increases.

| Arm | Process CPU | Verified wall | Sender ThreadCPU | Receiver ThreadCPU |
|---|---:|---:|---:|---:|
| Whole zstd | +1.51% | +0.38% | +0.86% | +2.29% |
| Combined | -1.07% | -0.97% | -2.28% | -0.004% |

Every individual CPU/time pair is retained, including Combined's trial-zero loss:

| Arm | Trial | Process CPU | Verified wall | Sender ThreadCPU | Receiver ThreadCPU |
|---|---:|---:|---:|---:|---:|
| Whole | 0 | +3.80% | +4.84% | +2.87% | +4.98% |
| Whole | 1 | +1.51% | +0.38% | +0.86% | +2.29% |
| Whole | 2 | +0.81% | +0.32% | +0.86% | +0.76% |
| Combined | 0 | +4.79% | +4.62% | +9.09% | -0.004% |
| Combined | 1 | -5.47% | -4.57% | -5.21% | -5.87% |
| Combined | 2 | -1.07% | -0.97% | -2.28% | +0.34% |

All three Combined RSS observations increased: **2.03%, 5.34%, 1.30%**; Whole
trials 1/2 increased **0.69%/3.15%**. Positive wire-byte changes also remain:
Whole trials 0/2 **0.06185%/0.01110%**, Combined trials 1/2 **0.01261%/0.08264%**.
These small canonical identity/numeric-encoding variations are actual bytes,
not normalized away. JSON also retains every setup, offline-readback, cleanup
and lifecycle delta; the separate median cleanup CPU increases are 6.67% Whole
and 10.13% Combined, at sub-millisecond absolute scale. No metric was dropped.

## Current native cells: no fresh native before/after pair

Both current-source Linux/Firecracker cells passed **18 publication operations
and 18 additional keyless reboot readbacks**, with exact lengths, object IDs and
namespaces. Both exits are zero. Every endpoint/relay direction and logical/two-hop
total agrees. Limits and peak remain two links/two slots/524,288 reserved bytes,
with zero final usage/refusals/fencing. Native process RSS is not qualified here.

| Current arm | Logical bytes, both directions | Physical two-hop bytes | Sender timed CPU | Receiver timed CPU | Relay process CPU | Lifecycle wall |
|---|---:|---:|---:|---:|---:|---:|
| Whole zstd/raw | 107,356,654 | 214,713,308 | 4.368591 s | 3.619492 s | 0.119133 s | 8.644921 s |
| Combined | 62,537,586 | 125,075,172 | 2.954190 s | 2.509574 s | 0.070838 s | 6.364583 s |

Current Combined used **41.75% fewer logical bytes, 32.38% less timed sender CPU
and 26.38% less lifecycle time** than current Whole for this mixed sequence.
Only these two cells ran now; historical native records are not new paired
evidence about the refactor. Each target is **14,851,384 actual bytes** at requested
16 MiB. Eight positions include repeats/generated/residual/unrelated/edit/schema
cases; Combined pays checkpoint/schema setup. These are not eight cold files.

Aggregate gains coexist with current per-workload losses versus Whole:

| Workload | Whole sender CPU | Combined sender CPU | CPU change | Whole operation wall | Combined operation wall | Wall change | Combined wire change |
|---|---:|---:|---:|---:|---:|---:|---:|
| Schema fit | 598,559 us | 675,901 us | +12.92% | 1,046,512 us | 1,119,688 us | +6.99% | -1.07% |
| Schema drift | 628,226 us | 675,740 us | +7.56% | 1,052,465 us | 1,111,968 us | +5.65% | -0.71% |
| Unrelated cold target | 1,119,782 us | 1,126,660 us | +0.61% | 1,938,206 us | 1,898,921 us | -2.03% | -0.0013% |

Opaque-edited-repeat wire increased **0.06372%**, despite lower CPU/wall.
All eight per-workload comparisons remain in JSON. The unrelated target still
uses roughly **31.35 MB logical canonical traffic** for 14.85 MB of content.
This compares canonical MVM paths, not generic secured raw/TLS, and establishes
no universal per-object win or low-overhead cold-transfer claim.

## Pins, timer domains, custody and limits

Source `960e73c59d7eab62f15a16667f95e594e1cabc8b` is bound by the 285-entry
qualified manifest (SHA-256
`612057cc7577fe1e0c2f58b3c09d1c1535b53cd36fba93c5a18ac0bc1389bfde`).
The live product hash is `29010dde...`; host baseline is prior C8 `5c41d6ac`,
probe `4109a318...`; current normal probe/host/static-guest hashes are
`04a57c32...`, `feebe513...`, `125072d1...`. Exact values are in the JSON and
execution/binary manifests. Baseline applicability is strict-transfer behavior.

Host aggregate process CPU includes both actors/setup; role ThreadCPU is
independent. Overlapping operation ProcessTime intervals cannot be pooled as
endpoint CPU. Verified wall includes compute/waits and receiver readback; offline
re-verification, cleanup and lifecycle remain separate. Native CPU sums timed
setup/operations including fixture/service/codec work within those intervals,
excluding untimed gaps, JSON and independent final verification. Native lifecycle
starts after journal/link admission and covers boot through keyless verification
and teardown. Relay and nested worker/VMM CPU remain separate. Actual stream bytes
include envelopes/prefixes/receipts, excluding lower-level control traffic.

Parent verified both compact archives and **41 members** on Mac/remote before
removing two successful owned disks: **536,870,912 apparent file bytes**, confirmed
by the cleanup record, not an assertion of freed blocks. This review did not
repeat those custody hashes. Failed functional/fixture records remain preserved;
only the two derived review outputs were written. CI ZIP verification belongs
to the independent Rust confirmation and is not claimed as this arithmetic work.

These are shared-host, fixed exploratory trials. There is no causal/statistical
speedup or noninferiority, Idle-consumer cost, retry/resume, WAN/fleet, energy or
send-only claim. Allocation remains two NIC-less guests/two vCPUs/2 GiB; Linux
experimental64 is still unenforced. Full C8, long retention, larger windows and
remaining platform/upstream gates remain open.
