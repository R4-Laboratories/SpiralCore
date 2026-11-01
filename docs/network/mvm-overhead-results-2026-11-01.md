# MVM transfer overhead — 2026-11-01

The exact-repeat optimization works in actual MVM guests, and the explicit higher
mailbox bandwidth profile substantially reduces large-transfer elapsed time.
**All 36 registered sessions passed: 324 object operations were exact after
publication/reopen and again after a separate keyless, read-only receiver boot.**
There were no failed sessions or retries. This is a local two-guest HVF/vsock
experiment on one physical Mac, not a WAN result or production qualification.

The change avoids preparing the full warm encoder when the owned previous object
is byte-identical. It still probes the receiver and requires its current authenticated
publication response; remote eviction takes the existing repair path. The default
mailbox rate remains 4 MiB/s. A separate, explicitly selected experimental profile
allows 64 MiB/s, preserving the 8 MiB burst, frame/queue limits and other controls.

The before binaries come from the prior final implementation at
`be5fac2991f5bea43ff3b38b0bfc4cbc7adddd21`; the after source is
`a078847d2d1af1bc09dd484ef799693b69bc6355`.
[Before identities](evidence/2026-11-01-mvm-overhead/before-binaries.sha256),
[after identities](evidence/2026-11-01-mvm-overhead/after-binaries.sha256) and
[source identities](evidence/2026-11-01-mvm-overhead/after-source.sha256) are retained.

There are three matched trials per size/arm/condition. The three conditions were
interleaved within each trial, with their order rotated. All fixture object IDs and
lengths match across the conditions. Whole uses prepared whole-object zstd with raw
fallback for incompressible data; it already deduplicates identical cached objects.
The common workload is seed, exact repeat, localized overwrite, unrelated object,
another localized overwrite, truncation and append. A nominal 16 MiB session has
117,440,467 common reconstructed bytes, nearly 112 MiB, rather than one 16 MiB file.
Warm's four extra schema/runtime objects are excluded from the common-operation
table and included in full lifecycle figures.

The table gives separate sample medians. Bytes are **logical encrypted stream
bytes in both directions**, including framing. Every logical byte crosses two
vsock hops, so physical stream bytes are twice these values; virtio credit/control
headers are excluded. Operation time includes preparation, authentication,
receiver work, durable publication and the authenticated acknowledgment.

| Updated code / profile | Object size | Arm | Common logical bytes | Common operation time | Sender guest CPU | Receiver processing CPU |
|---|---:|---|---:|---:|---:|---:|
| Standard 4 MiB/s | 1 MiB | Whole | 12,202,791 | 0.903 s | 0.643 s | 0.168 s |
| Standard 4 MiB/s | 1 MiB | Warm | 4,177,114 | 0.220 s | 0.110 s | 0.075 s |
| Experimental 64 MiB/s | 1 MiB | Whole | 12,204,409 | 0.456 s | 0.226 s | 0.155 s |
| Experimental 64 MiB/s | 1 MiB | Warm | 4,176,270 | 0.208 s | 0.103 s | 0.071 s |
| Standard 4 MiB/s | 16 MiB | Whole | 192,950,098 | 43.984 s | 37.463 s | 4.547 s |
| Standard 4 MiB/s | 16 MiB | Warm | 64,386,017 | 13.812 s | 11.232 s | 1.675 s |
| Experimental 64 MiB/s | 16 MiB | Whole | 192,909,606 | 6.777 s | 3.225 s | 2.271 s |
| Experimental 64 MiB/s | 16 MiB | Warm | 64,412,643 | 3.291 s | 1.456 s | 1.028 s |

At 16 MiB under the experimental profile, the median **trial-paired** Warm/Whole
ratio is 0.333901 for common bytes and 0.467363 for common operation time: about
66.61% fewer bytes and 53.26% less time. A localized 16 MiB overwrite has medians
25,557 logical bytes and 109.736 ms with Warm, versus 32,150,043 bytes and
1,009.587 ms with Whole. The paired byte reduction is 99.9205%.

The next table isolates exact-repeat operations at the **same standard 4 MiB/s
profile**. Exchange time is nested inside total time and includes framing,
authentication, receiver processing and waiting; it is not socket-send-only time.
Sender CPU is the Linux guest's process accounting, not physical energy.

| Size | Implementation / arm | Logical bytes | Sender CPU | Authenticated exchange | Encode through publication |
|---|---|---:|---:|---:|---:|
| 1 MiB | Before Whole | 8,492 | 4.269 ms | 2.850 ms | 6.012 ms |
| 1 MiB | Before Warm | 8,605 | 5.354 ms | 2.933 ms | 7.233 ms |
| 1 MiB | After Whole | 8,518 | 3.583 ms | 2.854 ms | 5.391 ms |
| 1 MiB | After Warm | 8,540 | 1.503 ms | 2.556 ms | 3.058 ms |
| 16 MiB | Before Whole | 8,546 | 37.516 ms | 13.890 ms | 49.222 ms |
| 16 MiB | Before Warm | 8,567 | 74.229 ms | 21.627 ms | 94.548 ms |
| 16 MiB | After Whole | 8,559 | 40.662 ms | 13.277 ms | 53.384 ms |
| 16 MiB | After Warm | 8,483 | 10.790 ms | 24.794 ms | 31.938 ms |

Warm's trial-paired after/before repeat CPU ratios are 0.280725 at 1 MiB and
0.110294 at 16 MiB; elapsed ratios are 0.438794 and 0.337797. Every paired repeat
trial improved both measures. Against updated Whole at the same rate, Warm's
repeat CPU ratios are 0.419481 and 0.265358, and elapsed ratios are 0.567242 and
0.644717. The removed work is principally before the exchange: median preparation
CPU fell from 4.211 to 0.503 ms at 1 MiB and from 72.738 to 9.619 ms at 16 MiB.
The existing small repeat exchange remains small; byte differences of tens of
bytes include canonical-envelope metadata variation.

**Non-repeat losses remain.** The equality comparison can read through a long
shared prefix before finding a changed byte. The following are median trial-paired
after/before ratios for Warm at the unchanged standard profile; values above one
are worse. They are descriptive observations under shared-host load, not a causal
allocation of every regression to the equality check.

| Size / operation | Sender CPU ratio | Operation-time ratio |
|---|---:|---:|
| 1 MiB localized overwrite | 1.132 | 1.116 |
| 1 MiB second localized overwrite | 1.298 | 1.238 |
| 1 MiB unrelated object | 1.065 | 1.127 |
| 16 MiB localized overwrite | 1.419 | 1.401 |
| 16 MiB second localized overwrite | 1.189 | 1.185 |
| 16 MiB append | 1.277 | 1.326 |
| 16 MiB truncation | 1.352 | 1.259 |

The 16 MiB session's extra runtime checkpoint/generated/residual observations also
regressed: paired sender CPU ratios 1.639/2.291/3.550 and elapsed ratios
2.305/2.402/3.393. Their after medians are 13.773/20.256/29.189 ms elapsed. These
small extra objects are retained in the results rather than hidden by the common
seven-object comparison. The complete per-kind results include favorable and
unfavorable observations for both arms.

Full lifecycle accounting includes boot, prepared state, all objects, shutdown,
checked disk synchronization and the separate keyless verifier. Native OS CPU is
the macOS orchestrator plus all three worker processes, each counted once. Do not
add the guest CPU values above to this enclosing native accounting.

| 16 MiB condition | Arm | Native OS process CPU | Full lifecycle |
|---|---|---:|---:|
| Before / standard | Whole | 15.920 s | 45.497 s |
| Before / standard | Warm | 6.772 s | 15.307 s |
| After / standard | Whole | 15.853 s | 45.343 s |
| After / standard | Warm | 7.019 s | 15.750 s |
| After / experimental64 | Whole | 8.851 s | 8.722 s |
| After / experimental64 | Warm | 5.240 s | 4.890 s |

The sender shortcut is **not an overall same-profile speedup** in this sample.
For Warm, standard-profile after/before paired common-operation elapsed ratios are
1.073592 at 1 MiB and 1.001239 at 16 MiB; full native CPU ratios are 1.049280 and
1.126816. Full lifecycle ratios are 1.048589 and 1.024611. A ratio of separately
sorted medians can differ from the median of trial-paired ratios; the pairing is
the primary comparison, including where separate medians look more favorable.

The explicit rate-profile effect is separate. Comparing the **same updated code**
at 64 versus 4 MiB/s, paired common-operation elapsed ratios are 0.238260 for Warm
and 0.154108 for Whole at 16 MiB: approximately 4.20× and 6.49× faster. Native OS
CPU ratios are 0.757888 and 0.558308. This does not turn a rate-policy change into
a codec improvement. Byte reduction still comes from warm reuse; the increased
allowance exposes remaining encoding, serialization, authentication and receiver
costs after much of the rate wait is removed.

Compilers were observed at the beginning or end of 13 of 36 sessions: four before,
five after-standard and four after-experimental64. The sampled one-minute host
load ranged from 3.80 to 21.86. An empty compiler snapshot means only “no compiler
observed at endpoints,” not proof of no mid-session contention. Guest virtual CPU
accounting may inflate under host descheduling; the native totals have their own
OS accounting boundary and neither establishes energy use. All timings are n=3
descriptive samples, with no p95, confidence interval or clean-host qualification.
The isolated native debug/test-ack diagnostic is retained separately and is not
used as release-guest performance evidence.

The 36 disposable receiver disks were removed after successful receipt custody
checks; compact public logs and JSON remain in Git. The retained packet is about
13 MiB, and approximately 22 GiB was free after this campaign. No other lab's work
was stopped or altered. No WAN, Firecracker, production mailbox activation,
concurrent policy-revocation, FIPS or CUI qualification is implied.

Reproduction and review: [registration](evidence/2026-11-01-mvm-overhead/registration.json),
[pre-run amendment and exact order](evidence/2026-11-01-mvm-overhead/amendment-01.json),
[36 session summaries](evidence/2026-11-01-mvm-overhead/sessions.jsonl),
[324 operation rows](evidence/2026-11-01-mvm-overhead/operations.jsonl),
[all medians and paired ratios](evidence/2026-11-01-mvm-overhead/summary.json),
[exactness/completion](evidence/2026-11-01-mvm-overhead/completion.json),
[fixture identity check](evidence/2026-11-01-mvm-overhead/fixture-pairing.txt),
[runner and accounting notes](evidence/2026-11-01-mvm-overhead/README.md),
[extraction](evidence/2026-11-01-mvm-overhead/extract.jq), and
[analysis](evidence/2026-11-01-mvm-overhead/analyze.jq). Each summary row links its
full raw result containing guest events, directional counters and worker receipts.
