# Sustained broker recovery and native object-transfer pilots

Implementation is on [PR #52](https://github.com/R4-Laboratories/SpiralCore/pull/52).
The shared Rust broker now reports the actual local quota branch that refused
work. A real five-minute retention test recovered without replacing its owners
or connections. Seven Linux/Firecracker sessions verified 62 publications and
44 keyless reboot readbacks. Broader fleet, Mac resource-pressure, Windows
durable-gateway and canonical WAN gates remain open under
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49) and
[#42](https://github.com/R4-Laboratories/SpiralCore/issues/42).

**Campaign status:** all five registered Linux cells passed their stated gates.
The campaign began on November 2 and completed on November 3, 2026. The Mac
deadline failure below remains a failure; moving a test does not repair it.

## What changed

- Fixed-size, trusted-owner refusal diagnostics distinguish total and reserved
  data/control budgets, signed capability contributions, rate limits and the
  receipt tracker's local ceiling. Counts and limits contain no tenant/message
  identities or payloads. Canonical errors, envelopes and grants are unchanged.
- The tracker exposes aggregate retained-entry, attempt and next-expiry counts.
  Observing a snapshot does not expire or release replay/receipt state.
- A concurrent Rust probe runs one persistent TCP pair per worker through the
  same `BrokerContext`. Socket waits remain outside dispatch. It records each
  worker, exact readbacks, actual two-way bytes, CPU, RSS, FDs and tail latency.
- The object harness adds a raw-codec arm on the existing mixed fixtures and
  preserves partial failure evidence. It reuses the existing object-v1 engine
  and canonical mailbox service; it adds no codec or MVM envelope.

Diagnostic attribution requires a changed counter inside the failing dispatch.
Ordering prechecks and a direct single-envelope contribution precheck still have
unattributed refusal paths; the recorder must keep those causes unknown instead
of attaching a stale global last-refusal record.

The ternary/CA/oscillator runtime, warm cache, sparse residuals and schema codec
remain active. In the 1 MiB mixed pilot, generated data selected `RuntimeV1`,
residual and opaque edits selected `SparseV1`, and structured records selected
`SchemaZstdV1`. Exact repeats used the existing verified object probe. A reported
`Raw` mode on a repeat is not evidence that its whole payload crossed the wire.
The actual counters, rather than the mode label, measure transmission.

## Saturation and full retention recovery

The pinned Mac test completed **1,638 operations and 2,457 exact readbacks**,
then recorded `Sender / Control / Identities: 4096 + 1 > 4096` during receipt
observation. The next request had been admitted but had no payload readback;
it is retained as incomplete, not counted as delivered.

The same broker, services, trackers, ordering owners and TCP pair waited a real
**301.103 seconds** after that refusal. Retained tracker entries and request
indices fell **2,458 → 0**, while the three FIFO scope checkpoints remained.
Queues and active Calls were empty. A new Call and Cast then completed with
**three exact readbacks** and matching two-way counters. There was no reset,
quota increase, early tombstone eviction or skipped unknown FIFO position.

The recovery interval after pair provisioning took **313.634 seconds**, used **12.155 seconds of process CPU**
and reached **32.82 MB peak process RSS**. These include the recorder and wait
lifecycle; they are not codec speed or energy measurements. This recovery mode
did not separately record provisioning CPU/time, which must remain unavailable;
the concurrent and object modes record their setup separately. The explicit
synthetic authorization lifetime was 15 minutes, while the historical short
profile and production validity checks remain unchanged.

## Mac concurrency boundary retained

At two logical endpoints, the strict one-second/10-operations-per-second profile
completed **3,100 operations and 4,650 exact readbacks** over 310 seconds without
an incomplete admission, quota refusal or mismatch.

At eight endpoints, the same profile **failed**: **974 complete operations,
1,463 exact comparisons, four incomplete admissions, zero mismatches and zero
quota refusals**. Three workers observed `Expired` during payload consumption;
the fourth observed `Unauthorized` during Reply admission. Source checks show
strict deadline enforcement, but the latter error has several possible causes.
Aggregate host load and swap observations around/after failure do not uniquely
establish OS scheduling as the cause. See the
[focused diagnosis](evidence/2026-11-02-sustained-broker/qualification/steady-8-debug.md).

The remaining Mac cells and new HVF launches were deferred under current resource
pressure. Linux uses the **same one-second profile and quotas**, with separately
recorded source/binary/host identities. The tests do not silently lengthen TTLs.

## Qualified Linux logical-endpoint workload

Each steady cell ran about **310 seconds**, with **4,096-byte payloads**, ten
offered operations per second across the pair population and one outstanding
operation per pair. Calls and generic Casts alternate. The probe has a one-second
application TTL, one-second service-retention extension and native-sized identity
quotas; the tracker retains its full deadline-plus-300-second period. This is the
explicit probe profile. Production service-retention defaults are unchanged and
are not established as load-qualified by this shorter service profile.

| Logical endpoints / pair workers | Completed operations | Exact readbacks | Actual bytes both ways | Process CPU | Wall time | Peak RSS | Worst worker p95 |
|---|---:|---:|---:|---:|---:|---:|---:|
| 2 / 1 | 3,100 | 4,650 | 73,249,394 | 18.078 s | 310.004 s | 31.84 MB | 8.267 ms |
| 8 / 4 | 3,100 | 4,652 | 73,276,836 | 18.822 s | 310.005 s | 30.67 MB | 27.645 ms |
| 32 / 16 | 3,104 | 4,656 | 73,972,248 | 18.811 s | 310.405 s | 37.65 MB | 112.803 ms |

All steady cells had zero incomplete admissions, mismatches and quota refusals;
sender/receiver counters agreed. The last cell's per-worker pacing ceiling gives
194 operations × 16 workers and the extra 0.4-second remainder. Sixteen pair
operations were observed in flight. This is a paced supported workload, not a
maximum throughput estimate or a count of booted MVMs.

The 65-second interference cells also passed their declared gates:

| Logical endpoints | Completed operations | Exact readbacks | Explicit hot-peer incomplete admissions | Process CPU | Peak RSS | Worst paced-worker p95 |
|---:|---:|---:|---:|---:|---:|---:|
| 8 | 2,127 | 3,192 | 1 | 9.221 s | 32.67 MB | 62.531 ms |
| 32 | 2,253 | 3,387 | 1 | 10.131 s | 42.39 MB | 103.512 ms |

In each cell the hot worker reached the causal sender control-identity quota
after 1,638 completed operations. All three/fifteen paced peers completed their
scheduled 163/41 operations, respectively, without an error. The slow peer's
50 ms wait occurred after request transmission, outside dispatch and inside its
latency interval. Malformed local admissions were refused; an unsent Call and a
blocked terminal receive were cancelled. The hot burst ends at its first refusal,
malformed input is injected locally, and transport/application-stall tests do
not establish kernel send-buffer backpressure or remote processing cancellation.

Across the five Linux cells: **13,684 completed operations, 20,537 exact readbacks,
zero mismatches**, matching stream counters and two separately retained hot-peer
incomplete admissions. Actual framed-envelope traffic totaled **324,300,612 bytes**.
There were no dropped recorder checkpoints. All sampled resource gates passed;
the largest observed process RSS was **42.39 MB**, below the registered 512 MiB
ceiling, and sampled FDs reached 37, below 128.

CPU includes both roles, broker actions, repeated observer snapshots, checkpoints
and teardown, excluding separately reported setup. Percentiles are per worker;
the table uses the maximum worker p95, not a pooled percentile. Pre-drop worker
snapshots retained 3/12/48 FIFO scopes and 4,515/4,520/4,528 tracker entries across
the steady cells. The aggregate budget snapshot after worker joins includes
teardown releases and cannot represent live steady-state occupancy. One crossed
retention window and a fixed peer population do not establish indefinite churn,
the native consumer beyond its 2,048-Call cap, large-object concurrency or a
production fleet capacity envelope.

## Matched local object pilots on Wick

Each cell is a fresh process using one shared broker and two local TCP roles.
Aggregate process CPU is measured once; separate role thread CPU is retained.
The original campaign's overlapping process-clock intervals must not be summed.
Completion includes fixture generation, codec preparation, setup publications,
network exchange, durable publication and receiver readback. Additional offline
verification and cleanup are recorded separately.

| Actual bytes per application target | Arm | Actual bytes both ways | Aggregate process CPU | Verified completion | Peak RSS |
|---:|---|---:|---:|---:|---:|
| 1,047,202 | Raw codec + exact-object cache | 13,668,089 | 0.966 s | 0.914 s | 29.29 MB |
| 1,047,202 | Prepared whole-object zstd | 7,351,803 | 0.553 s | 0.536 s | 30.37 MB |
| 1,047,202 | CPU-conscious Combined | 4,932,948 | 0.469 s | 0.474 s | 35.72 MB |
| 14,851,384 | Raw codec + exact-object cache | **Incomplete: Full** | Original recorder unavailable | Unavailable | Unavailable |
| 14,851,384 | Prepared whole-object zstd | 107,307,160 | 7.094 s | 6.419 s | 346.41 MB |
| 14,851,384 | CPU-conscious Combined | 62,489,506 | 4.566 s | 4.193 s | 397.46 MB |

Every successful arm sends the same eight target byte strings. Combined also
pays two checkpoint/schema setup publications. Raw and zstd use the same
exact-object probe, so this raw baseline is cache-aware. Generated/reusable data
is deliberately part of the mix; these results do not imply gains on arbitrary
encrypted or unrelated traffic. Combined's higher RSS is a retained cost.

The original large raw failure has only `Error: Full` and an empty result file;
its cause and complete transfer counters remain unknown. One preregistered
unchanged-profile follow-up preserved partial records and observed a **global
post-join** `Tenant / Control / Identities: 8192 + 1 > 8192`, with two recorded
refusals. That is not per-dispatch attribution of the original failure. No failed
cell is credited as a complete matched raw comparison. Quotas were not raised.

## Actual Linux MVM boundaries

Seven source-pinned Firecracker sessions used at most two guests at once, each
with one vCPU and 1 GiB RAM, no guest NIC, and the existing typed mailbox/vsock
path. All **62 publications** reopened exactly; the five two-guest sessions also
performed **44 keyless reboot checks**. Both-direction stream counters matched.
The temporary owned Firecracker link was removed, and no VMM remained running.

The large two-guest batch has eight targets of **14,851,384 bytes each**. Combined
includes its two setup objects. The baseline exact-object probe is common.

| Arm | Logical mailbox bytes, both directions | Timed sender CPU | Timed receiver CPU | Timed sender wall intervals | Full boot/transfer/keyless-recovery lifecycle |
|---|---:|---:|---:|---:|---:|
| Raw codec + exact-object cache | 188,047,462 | 7.006 s | 5.875 s | 11.747 s | 13.086 s |
| Prepared whole-object zstd | 107,268,387 | 4.342 s | 3.632 s | 7.231 s | 8.580 s |
| CPU-conscious Combined | 62,506,959 | 2.946 s | 2.494 s | 4.885 s | 6.309 s |

These are **single fixed-order native pilots**, not confidence intervals. They
observe fewer bytes and lower compute-included elapsed time against zstd on this
workload. Across both mailbox hops, accounted vsock stream bytes total twice
the logical byte count; these counters exclude the separate workload-exit service
and virtio headers.
Linux's mailbox rate remains unenforced and explicitly recorded as such.

There is also a smaller-workload loss: the two-guest 1 MiB-tier Combined pilot
took **1.193 seconds for the complete lifecycle versus zstd's 1.163 seconds**,
despite lower timed sender CPU and sender wall intervals. This single observation does
not isolate boot/recovery variability from codec cost; it remains in the record.

The native roles use separate process-owned budgets, while the host-process
object pilot above places both roles in one accounting domain. Native success
does not erase the shared-budget refusal or establish aggregate enforcement
across guest processes. The native 2,048-Call witness cap remains unchanged:
the large raw/zstd/Combined sessions used **1,467 / 843 / 505 Calls**.

Guest CPU/session fields sum the recorded setup and operation/receiver-processing
intervals. Per-object fixture copies/hash bookkeeping, JSON output and final
independent readback outside those intervals are excluded. Full
boot/transfer/keyless-recovery wall time includes those activities. Worker, relay
and partial observed Firecracker CPU remain separate in the raw receipts; they
cannot be added indiscriminately as total host CPU. Allocated guest RAM is a
configured budget, not a measured guest RSS peak. Native MVM execution is not WAN evidence.

## Evidence, validation and remaining gates

[Compact evidence and source identities](evidence/2026-11-02-sustained-broker/README.md)
retain preregistrations, raw rows, failed gates, original CI artifacts, independent
reviews and cleanup records. Product adapter line coverage is **92.85%**; changed
adapter production lines measure **99.40%**. These are declared line scopes,
not branch or whole-repository coverage.

Canonical direct/relay WAN, a qualified Windows durable gateway, admitted FlowMux
integration, production mailbox supervision and cross-process aggregate host
accounting remain separate program gates. The older HTTP WAN results do not
qualify this newer canonical MVM path. No FIPS validation, certification,
exactly-once execution, energy saving or universal compression claim follows.
