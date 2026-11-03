# Strict host-transfer records: independent measurement review

**PASS for fidelity, arithmetic and the scoped observations below.** Recomputed
from the twelve retained Rust JSONL records using jq/report arithmetic only.
All preregistered trials remain; no run, build, remote action or product edit was
performed. Full-precision observations, all pair/target deltas and explicit
positive-change lists are retained in [summary.json](summary.json).

This is **strict host TCP transfer**, not measurement of the opt-in controller,
channel, Idle polling or recovery CPU. There is no native MVM/WAN, energy,
send-only, tail-latency, causal significance or thirty-trial acceptance claim.

## Fidelity, identities and timing order

All **12 cells / six matched pairs** have exact verification, directional byte/
frame agreement and successful owned-store cleanup. They contain **108 publication
operations and 108 independent exact readbacks**, including repeated object IDs.
Published byte-array IDs/namespaces normalize to the recorded offline hex values;
all lengths/kinds agree. Before/after and current arm comparisons match the same
eight target IDs/lengths per trial. Every target is **1,047,202 actual bytes** at
requested 1 MiB; Combined additionally publishes a 3,382-byte checkpoint and
48-byte schema. Before/after semantic application request/response sizes match.

Peak process RSS is **36,560,896 bytes**; maximum owned store is **6,287,069 bytes**,
below registered 1 GiB/512 MiB ceilings. No fleet/guest capacity follows from this.
The retained host-build-order record places measurement completion at
11:31:20.227492447 and the next build start at 11:31:44.526589692, a **24.299 s**
gap. This excludes that subsequent build from these intervals, not all shared-host
activity. Recorded Wick load remained approximately 2.4–2.7 over the short batch.

## Strict before/after: paired changes and retained losses

Primary statistic is the median of the three matched
`100 * (after / before - 1)` changes, not the ratio of separately computed column
medians. Positive means an increase. CPU domains remain separate.

| Arm | Aggregate process CPU | Sender ThreadCPU | Receiver ThreadCPU | Verified wall | Lifecycle wall |
| --- | ---: | ---: | ---: | ---: | ---: |
| Whole zstd | +0.765% | +2.521% | -3.858% | -0.124% | -0.115% |
| Combined | +0.156% | +0.242% | +0.067% | +0.026% | +0.003% |

Every individual main-domain pair is retained here; no trial was discarded:

| Arm/trial | Process CPU | Sender ThreadCPU | Receiver ThreadCPU | Verified wall | Wire | RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Whole 0 | -4.313% | -2.025% | -6.739% | -3.689% | -0.02315% | -3.009% |
| Whole 1 | +0.765% | +4.597% | -3.858% | +0.0277% | +0.06322% | +2.280% |
| Whole 2 | +1.772% | +2.521% | +0.878% | -0.1239% | +0.02215% | +1.905% |
| Combined 0 | +5.278% | +6.384% | +3.995% | +4.539% | -0.02433% | +0.427% |
| Combined 1 | -2.594% | -4.519% | -0.148% | -1.100% | +0.11226% | -1.867% |
| Combined 2 | +0.156% | +0.242% | +0.067% | +0.0263% | +0.05443% | +0.360% |

Other paired-median domains also retain losses: Whole offline readback CPU/wall
**+2.239%/+2.239%**, cleanup **+1.154%/+1.154%**, lifecycle CPU **+0.811%**;
Combined cleanup **+2.414%/+2.062%**, lifecycle CPU **+0.128%**. Harness setup
CPU/wall changes are Whole **-1.460%/-1.340%**, Combined **-0.487%/-0.487%**.
Per-role setup process-clock intervals, every individual auxiliary-domain loss
and absolute values remain in JSON; they are not discarded or added to role CPU.

Per-target before/after paired medians expose variation hidden by the aggregate.
These CPU values are sender-operation **process-clock intervals**, not isolated
sender-thread CPU. Individual trial values remain in JSON, including increases
when a listed median is negative.

| Target | Whole CPU | Whole wall | Combined CPU | Combined wall |
| --- | ---: | ---: | ---: | ---: |
| Generated | -12.121% | -11.102% | +0.536% | +0.818% |
| Residual | -2.966% | -13.575% | +2.434% | +9.091% |
| Repeat | +5.179% | -4.288% | +12.093% | +9.440% |
| Unrelated | -0.142% | -0.404% | +5.800% | +5.159% |
| Opaque edited | +0.918% | -0.669% | +1.459% | +0.267% |
| Opaque edited repeat | +1.346% | +0.655% | +0.918% | +0.244% |
| Schema fit | +2.224% | +1.256% | +0.837% | +0.256% |
| Schema drift | -1.727% | -0.871% | -4.973% | -4.341% |

Actual encrypted/framed wire changes remain unnormalized. Common semantic bytes
match, while randomized canonical identities/numeric encodings can vary envelope
lengths; this review does not attribute each changed byte to a specific field.

## Current Combined versus current Whole

Column values are medians; the final column is the separately computed median
paired relative change. Wall ratios of the displayed medians differ from it.

| Domain | Whole median | Combined median | Median paired change |
| --- | ---: | ---: | ---: |
| Both-direction canonical bytes | 7,351,454 B | 4,930,219 B | -32.935% |
| Aggregate process CPU | 591,385 us | 498,841 us | -15.649% |
| Sender ThreadCPU | 324,952 us | 276,180 us | -15.009% |
| Receiver ThreadCPU | 265,464 us | 221,713 us | -15.106% |
| Verified transfer wall | 595,057 us | 518,749 us | -13.192% |
| Lifecycle wall | 608,634 us | 532,919 us | -12.839% |

Across the three current cells per arm, Whole/Combined totals are
**22,054,037 / 14,788,229 bytes**, **1,774,478 / 1,499,173 us aggregate CPU**,
**1,797,456 / 1,561,628 us verified wall**, and **1,838,434 / 1,603,745 us lifecycle
wall**. Ratios of these totals are respectively **-32.945%, -15.515%, -13.120%
and -12.766%**. Totals are sums of separate cells, not a single-transfer timing.

Aggregate savings do not establish a universal per-workload win:

| Target | CPU paired median | Wall paired median | Wire paired median |
| --- | ---: | ---: | ---: |
| Generated | -5.645% | -15.155% | -65.715% |
| Residual | -31.046% | -21.892% | -73.856% |
| Repeat | -19.585% | -23.564% | +0.1473% |
| Unrelated | +1.150% | +0.821% | +0.01906% |
| Opaque edited | -86.487% | -83.793% | -97.249% |
| Opaque edited repeat | -11.395% | -12.993% | +0.01470% |
| Schema fit | +6.909% | +6.612% | -0.7641% |
| Schema drift | +0.590% | +0.724% | -0.4129% |

Schema-fit loses CPU/wall in all three trials: **+12.788/+12.495%,
+6.909/+6.612%, +4.294/+5.565%**. Schema-drift loses in trials 0/2:
**+10.245/+10.669%, +0.590/+0.724%**. Unrelated loses in trials 0/2:
**+1.150/+0.821%, +3.557/+3.082%**. Repeat and opaque-edited-repeat wire losses
also remain in individual trials. Combined's paired-median RSS increase versus
Whole is **18.366%**; sender setup process-interval CPU/wall **+8.669%/+7.509%**,
receiver setup **+0.954%/+1.881%**, cleanup CPU/wall **+10.584%/+10.584%**.
The workload includes warm repeats, generated/residual and schema cases, pays
Combined setup, and compares the same canonical mailbox paths rather than
generic raw/TLS or eight independent cold objects.

## Pins and interpretation

Baseline source **960e73c5**, protected normal binary
`04a57c32154acad6dbc3754775039aad80cab632e77f9d50dce9965d52ad8d35`;
current production source **07e1bf83**, normal binary
`fe7937f2fb348ff0a3c517cf856f2ad132883346a8aa05020e6dbd0e1d62b6d7`.
The broker_objects Rust engine is unchanged. Later fixture-only source commits
are not relabelled as the executed binary's commit.

Aggregate ProcessTime is counted once; endpoint ThreadTime includes work on that
thread. Operation/setup ProcessTime intervals overlap other threads and cannot
be summed as exclusive role CPU. Verified wall includes compute, durable
publication and receiver readback. Offline verification, cleanup and lifecycle
are separately retained. Actual wire includes both canonical directions, framing,
initial Status and setup; it excludes TCP/IP headers and TCP retransmission.

These are three fixed exploratory paired trials on a shared host. They show
strict-path fidelity and observed costs, without a causal performance, statistical
noninferiority, opt-in wake/recovery, native/WAN or energy conclusion. Functional
loss/retry fixtures have separate source, traffic and processing evidence. Only
this review and its derived JSON were written; original records/failures remain.
