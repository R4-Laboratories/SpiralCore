# SC-OPT-018: acknowledged persistent transfer sessions

The local qualification completed **660 exact transfers**. Reusing acknowledged
state reduced median paired setup-plus-ten-update sender CPU by **23.15% at
1 MiB and 31.02% at 16 MiB**, relative to constructing the selector for each
update. All localized selector updates retain the **600-byte** bidirectional
HTTP-body representation. A subsequent six-pair WAN pilot completed **396 exact
transfers**, with lower CPU and elapsed time than prepared Whole zstd on the
measured warm workload. The first pilot failed during transport; its cause
remains unresolved and that failure is retained separately.

## Implementation and measurement contract

`WarmSession` owns the last acknowledged input and its verified identity, retaining
one prepared selector. It promotes a target only after the existing client checks
the durable acknowledgment. Failures retain the last acknowledged bytes and
invalidate receiver readiness; explicit recovery checks receiver state. Context
and epoch changes invalidate scoped state. The separate `RuntimeSession` retains
an immutable prepared CA prediction and checkpoint; it does not change the
runtime mathematics or unify the runtime and schema selectors.

The optional HTTP profile reuses connections with bounded request counts and
idle time; default-close behavior remains available. Every request still checks
authorization and scope. Owned-state admission is bounded; reported charges are
not measured RSS. No decoder, digest, durability or exactness check is skipped.

The [registered contract](evidence/2026-10-31-persistent-session/measurement-contract.md)
fixes three arms: fresh selector, persistent selector and prepared whole-object
zstd with Raw fallback. Each selector fits the same artifact once per session;
the fresh arm constructs its contexts ten times, the persistent arm once. All
arms use one HttpClient and pay an actual Raw seed. Whole does not need that seed;
the paid lifecycle is an equal-prior-state comparison, not optimal cold Whole.

Each sequence contains a seed and ten distinct updates: four 32-byte localized
edits, one independent replacement, then five localized edits of that replacement.
The 1 and 16 MiB fixtures are deterministic pseudorandom bytes. Fresh/persistent
winning modes, costs and complete normalized frames agree at every position.
The replacement tests fallback and resumption of warm transfers after a new ACK.

## Completed local run

Source `58990cec4d190f815fccd51df9cadf94029ab78f`, Rust 1.97.1 release,
`hardware-sha256`, aarch64 macOS. The frozen executable and all source files are
bound in the [qualification manifests](evidence/2026-10-31-persistent-session/qualification/binaries.sha256).
The run occurred at 04:00:36–04:01:53 UTC on 2026-10-31. Ten paired trials per
size produced 60 sessions, 60 seeds, 600 targets and 40 actual fits. Every object
was regenerated and read back byte-for-byte after stopping its receiver. Each
local session used one observed connection for eleven requests. All ten stores
were removed only after verification; compact raw records remain.

Values below are medians per seed-plus-ten-update session. Times are milliseconds.
The CPU gate includes setup and all ten updates, excluding the actual seed and
explicit target generation, which are included in the separately reported outer
clock. CPU subtotal is sender process plus disjoint receiver handler CPU.

| Size | Arm | Setup + updates sender CPU | Outer CPU subtotal | Outer completion | Paid body bytes |
|---|---|---:|---:|---:|---:|
| 1 MiB | Fresh selector | 23.7800 | 54.4060 | 161.1735 | 2,108,076 |
| 1 MiB | Persistent selector | 18.4005 | 49.1745 | 151.4160 | 2,108,076 |
| 1 MiB | Whole → Raw | 26.5110 | 57.2280 | 163.5260 | 11,564,718 |
| 16 MiB | Fresh selector | 281.8735 | 602.4670 | 958.3710 | 33,633,452 |
| 16 MiB | Persistent selector | 198.7985 | 529.5600 | 853.8335 | 33,633,452 |
| 16 MiB | Whole → Raw | 441.3740 | 794.5710 | 1,204.1710 | 184,954,286 |

The paired persistent/fresh sender-CPU medians are 0.7684541486044776 and
0.6897717335086377; both pass the preregistered ≤1.05 gate. A ratio of cohort
medians need not equal the median paired ratio. The large-object CPU comparison
contains one losing pair; elapsed losses also remain in the raw distributions.
This is evidence for retained state under this workload, not universal superiority.
The independent replacement is still an unfavorable case: both arms send the
same Raw bytes, but persistent-selector versus Whole median sender CPU is
7.218 versus 2.691 ms at 1 MiB, and 101.129 versus 42.444 ms at 16 MiB. Their
corresponding local completion times are 17.5985 versus 13.6970 ms and 159.4955
versus 99.1010 ms. Nine warm updates outweigh that penalty in these measured
sequences; a workload dominated by unrelated replacements need not benefit.
See the [full local summary](evidence/2026-10-31-persistent-session/local/summary.json)
and [independent recomputation](evidence/2026-10-31-persistent-session/local-review.json).

Bytes count both HTTP bodies, not HTTP/TLS/IP headers, retransmissions or extra
relay hops. Operation completion includes encoding through durable ACK; exchange
time is nested and includes receiver work and waiting, so it is not pure send
time. Outer completion also includes input generation and loop overhead. Receiver
socket/logging and relay CPU are outside the measured CPU subtotal. Energy is
unmeasured.

## Retained first WAN failure

[GitHub run 41167011759](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41167011759)
used the same source with a Linux sender and temporary Cloudflare relays to the
Mac origins. Linux qualification passed. The first three 1 MiB sessions completed,
followed by the 16 MiB fresh-selector seed and updates 1–3: **37 acknowledged
operations**. Update 4 failed before an admitted receiver observation. The
sender attempted a 342-byte request, recorded zero response body bytes, and
returned `Storage` after 500.452 ms of exchange time. The planned pilot was
396 transfers; it stopped on this failure without a retry.

The current client erased the underlying ureq status/I/O error, and the original
receiver telemetry recorded only admitted requests. Matching the 500 ms idle
window is not sufficient to establish the cause. The preflight GET consumed
request ordinal 1; admitted ordinals 2–38 correspond to the 37 successful
operations. The failed target has no publication filename. This filename check
is not independent exact-byte readback of the partial store.

All six receivers and tunnels were stopped; temporary repository configuration
and the capability secret were removed. The failed store is retained separately.
The 31-file artifact manifest and downloaded ZIP digest were verified; compact
records are preserved under [wan/remote](evidence/2026-10-31-persistent-session/wan/remote/custody.json).
The [diagnosis receipt](evidence/2026-10-31-persistent-session/qualification/wan-attempt1-diagnosis.json)
records the unresolved boundary. A separate diagnostic adds sanitized error and
connection-close categories without changing the timeout, codec or retry policy.

The separately registered [diagnostic run 41168897547](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41168897547)
at `dbdbc709f0311a06efe5ffac43e5b334ca8a1a87` completed all 66 transfers.
Independent stopped-store readback verified all 66 exactly. Its connection log
contains the preflight rejection, two normal 32-request cap closures and one
empty idle timeout; no transfer rejection or transport failure occurred. The
failed pilot was **not reproduced**, and its cause remains unresolved. The
diagnostic is not pooled with performance trials. Its store was removed after
exact verification, and all temporary services and credentials were retired.
See [diagnostic evidence](evidence/2026-10-31-persistent-session/wan-diagnostic-01/remote/custody.json).

## Completed subsequent WAN pilot

[GitHub run 41169424060](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41169424060)
completed the one subsequently registered pilot at
`9a6d7f758fb724b848ca1f24c1c27f5088da671d`. All 99 source-manifest
entries match qualified `dbdbc709`; only documentation/evidence changed. The
Linux sender used Rust 1.97.1 release/default features; the Mac receiver retained
its qualified hardware-SHA executable. All three arms share those endpoints,
transport and durability settings. Six pairs per size use all six arm orders.
This is a pilot, not thirty-pair acceptance or a direct-host network qualification.

All **396 objects**, **36 sessions**, **24 actual fits**, and **120 paired winning
frame comparisons** passed the existing verifier after receiver shutdown. All
216 localized selector updates cost 600 body bytes. Every session and unfavorable
sample is retained. The 53-member artifact manifest and ZIP digest were verified.
The [full summary](evidence/2026-10-31-persistent-session/wan-pilot-02/summary.json)
and [raw artifact custody](evidence/2026-10-31-persistent-session/wan-pilot-02/remote/custody.json)
bind the measurement. The earlier failed pilot and 66-operation diagnostic are
not pooled with these timing results.

Localized-update cohort medians follow, in milliseconds except bytes. Each
arm/size has 54 localized updates; those repeated updates are not 54 independent
session trials. Completion includes encoding, hashing and the durable ACK;
exchange is its nested transport/receiver interval. Pure wire time is unmeasured.

| Size | Arm | Body B | Sender CPU ms | Receiver CPU ms | CPU subtotal ms | Exchange ms | Completion ms | Completion p95 ms |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 1 MiB | fresh-selector | 600 | 2.3405 | 4.9145 | 7.2610 | 54.6355 | 56.4620 | 89.5960 |
| 1 MiB | persistent-selector | 600 | 1.6415 | 5.0240 | 6.7495 | 54.9915 | 56.0905 | 70.8060 |
| 1 MiB | whole | 1,051,338 | 3.8145 | 4.5815 | 8.4665 | 130.1030 | 132.8555 | 179.8110 |
| 16 MiB | fresh-selector | 600 | 23.5335 | 26.0360 | 49.7555 | 83.4815 | 106.2685 | 148.1050 |
| 16 MiB | persistent-selector | 600 | 12.9395 | 32.4505 | 45.5365 | 93.6155 | 106.6970 | 180.2930 |
| 16 MiB | whole | 16,814,026 | 53.2025 | 39.7960 | 90.8980 | 1,122.0270 | 1,159.6070 | 1,471.0870 |

Each following row is the median of six complete seed-plus-ten-update sessions.
Whole pays the same actual seed, although it does not require that prior state.
The outer interval includes generation and setup; independently computed medians
need not add or have the same ratio as paired medians.

| Size | Arm | Setup + updates sender CPU ms | Outer CPU subtotal ms | Outer completion ms | Paid body B |
|---|---|---:|---:|---:|---:|
| 1 MiB | fresh-selector | 32.4865 | 92.5390 | 917.7620 | 2,108,076 |
| 1 MiB | persistent-selector | 26.1330 | 84.5315 | 836.6065 | 2,108,076 |
| 1 MiB | whole | 39.9040 | 95.7385 | 1,643.2110 | 11,564,718 |
| 16 MiB | fresh-selector | 355.4025 | 730.9600 | 3,562.6935 | 33,633,452 |
| 16 MiB | persistent-selector | 260.1590 | 705.8775 | 3,737.6900 | 33,633,452 |
| 16 MiB | whole | 514.1880 | 1,042.4225 | 13,383.2270 | 184,954,286 |

Against fresh selection, median paired setup-plus-update sender CPU falls **19.05%**
and **26.98%** at 1/16 MiB, with six of six wins at each size. Outer CPU subtotal
ratios are 0.946371 and 0.932264 (five of six wins each). Outer elapsed ratios are
0.970712 (five wins) and **1.008108 (three wins)**: the large persistent arm is
slightly slower in the paired median. It also has a worse localized-update p95
than fresh at 16 MiB. These results do not establish an incremental WAN time win
over fresh selection at both sizes.

Against Whole, complete paid sequences use **81.77% and 81.82% fewer body bytes**.
Median paired outer elapsed ratios are **0.580578 and 0.270910**; CPU subtotal
ratios are **0.911386 and 0.674791**, all six pairs favoring persistent selection.
These are observed warm-sequence gains, including actual seeding, a full
replacement and compute; they are not universal compression or energy claims.

The unrelated replacement remains unfavorable against Whole: 1/16 MiB median
sender CPU is 9.4845/140.4305 ms for persistent selection versus 3.7970/47.2230 ms
for Whole. Completion is 132.2885/1,276.6885 ms versus 120.9745/1,209.5020 ms,
with identical Raw body bytes. The initial failed pilot still prevents any claim
that the transport defect was fixed or its reliability qualified. Successful
connection logs show only preflight rejection, request-cap closes and empty idle
closes; they do not explain the original failure.

## Separate receiver-stage diagnostic

An external Begin/Data/Finish clock experiment recovered all 44 objects exactly.
The 1 MiB median instrumentation ratio, 0.874499, passed the ≤1.10 screen;
the 16 MiB ratio, **1.102040816**, failed. At the qualified small size, warm
median CPU was 0.812 ms in Begin, 0.020 ms in Data and 1.125 ms in Finish,
within 1.965 ms outer CPU. These independently computed medians need not add.
The large stage attribution remains unqualified and no receiver production
optimization was made from it. [Method](receiver-stages-method-2026-10-31.md),
[raw results](evidence/2026-10-31-persistent-session/receiver-stages.jsonl),
[screen results](evidence/2026-10-31-persistent-session/receiver-stages-summary.json).
