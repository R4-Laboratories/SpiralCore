# Persistent MVM service: dispatch isolation and bounded relay ownership

Product source: [`7531985`](https://github.com/R4-Laboratories/SpiralCore/commit/753198531d4f3ef87e8a79c63d44f4f081ce4077).
Delivery: [PR #54](https://github.com/R4-Laboratories/SpiralCore/pull/54),
[#43](https://github.com/R4-Laboratories/SpiralCore/issues/43),
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49).
[Design](../specs/2026-11-03-persistent-mvm-service.md) ·
[contract](../api/persistent-host-budget-v1.md) ·
[continuing plan](../plans/2026-11-03-persistent-mvm-service.md) ·
[compact original evidence](evidence/2026-11-03-persistent-mvm-service/).

## Delivered behavior

The canonical receiver releases the shared `BrokerContext` dispatch gate before
object reconstruction, codec replay, hashing and durable I/O. It retains the
existing private admitted proof and policy/live/owner guards, then reacquires
dispatch with fresh time before Reply preparation and tracker registration.
No mailbox envelope, receipt, object-v1 encoding, quota or retention was changed.

A new `HostTransportBudget` charges host/tenant link counts, frame slots and
serialized ciphertext-buffer capacity atomically. One supervisor injects its
owner into all relay links, including links to separate guest processes. The
two-guest supervisor reserves both links before child activation. Each relay
frame reserves the unchanged maximum 262,144-byte buffer capacity before the
allocating read and retains it through route validation and forwarding. The
buffer and decoded envelope drop before the credit; a live frame pins its link.
Partial startup, input and forwarding errors release only their owned resources.

This is **orchestrator-scoped, two-guest keyless-relay transport accounting**.
It does not measure decoded-envelope copies, kernel buffers, allocator overhead,
guest heap, plaintext service state, cache or codec workspace. Independent
orchestrators still have independent owners. Stage One is rejected by the
budgeted API; its ordinary guest/host endpoint remains available with a null
budget and explicit unaccounted transport coverage. There is no hidden fallback.

## Correctness and focused verification

- The dispatch regression first failed with `application work retained global
  dispatch` when its blocked application closure ran under `with_time`. It
  passed after the helper sampled time under dispatch and ran application work
  outside it. Both workers are always released/joined before asserting. This
  proves that boundary plus actual independent `LocalMailboxService::expire`
  progress; it does not measure authenticated control latency during real replay.
- Post-work regressions perform successful `ObjectMailbox.handle`, then replace
  the incoming policy or advance to the exact Call deadline. Reply preparation
  refuses with `Unauthorized`/`Expired` without adding queued Reply state.
- Budget tests cover shared aggregate and tenant limits, foreign scopes,
  overflow, frame-slot ceilings, link pinning, reuse, concurrent reservations,
  malformed reports, underflow fencing and poisoned state. Relay tests cover
  pre-read refusal, truncated/malformed input, incorrect routes, cancellation,
  failed writes, partial startup rollback and all four canonical envelopes.
- Locked release Linux live all-target checks passed **59 tests**; strict release
  Clippy passed. Focused authority/receipt checks passed **nine tests**, overlapping
  the broader adapter scope. These are executed host tests, not booted guests.
- Independent Rust/code, security and runner infrastructure reviews approved
  the exact product/runner identities. The one-guest reporting gap found in
  review was fixed before native execution. [Review record](evidence/2026-11-03-persistent-mvm-service/reviews.md).

Raw validation records and source manifests are retained beside this report.
Source hashes match all **272 tracked crate files** exported to Wick. The initial
RED tool output was observed in the execution session; its pre-change source
variant and raw output were not separately archived, so it is not presented as
an independently source-pinned performance control.

Focused isolated LLVM coverage ran 40 tests across three binaries, with no
export warnings. New budget production coverage is **179/198 lines (90.40%)**;
changed executable production across the three reviewed files is **315/362
(87.02%)**. The broader production denominator is **706/1,092 (64.65%)**,
including only 38.04% of all orchestrator lines: normal native runs do not supply
instrumented hits. These are explicit new-module/changed-line gates, not whole
live-crate or branch coverage. [Derivation and uncovered lines](evidence/2026-11-03-persistent-mvm-service/coverage/README.md).

Final CI passed [seven native-client rows](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41590624501)
and [Linux contracts](https://github.com/R4-Laboratories/SpiralCore/actions/runs/41590624517).
The former records 72 portable checks per row (**504 total**) and eight additional
Unix storage checks. The latter records 59 release-host checks and 114 shared
conformance checks, strict Clippy and default-guest/analyzer success; scopes
overlap and are not pooled into unique tests. CI executed merge `a0d02f5`, whose
tree `19f1cfe94ce45b92a086028acf91f1a2f54ee129` exactly matches reviewed source
`7531985`. The seven original archive digests and 222 inner member checks passed.
The native-client matrix does not exercise the new host budget on Windows or
qualify a Windows durable gateway, native MVM backend or WAN path.

## Matched before/after host-process transfers

The existing Rust `broker_objects` probe ran three trials per arm before and
after the change on Wick. Each fresh process transferred the same eight target
positions of **1,047,202 bytes each**, with the same exact-repeat probe,
canonical encryption and durable readback. Combined additionally publishes its
checkpoint and schema, giving ten publication operations versus eight. Trial
IDs are paired; fixed order and three trials make this an exploratory comparison,
not a statistical acceptance study. Registration was written after baseline
collection, before the native campaign; that distinction is retained.
Wick remains a shared host with DeepSeek active. CPU isolation and randomized
arm order were not established, so small CPU/time changes are descriptive and
cannot be attributed uniquely to this scheduling edit.

Values are the median of each column, so they need not describe one trial.
Bytes are actual canonical envelopes plus framing, both directions. CPU is one
aggregate process clock including both actors' fixture/codec preparation and
verified transfer; setup and teardown fields are separately retained. Times
include CPU and waits, rather than an inferred send-only duration.

| Arm | Before bytes | After bytes | Before CPU | After CPU | Before verified time | After verified time |
|---|---:|---:|---:|---:|---:|---:|
| Raw codec with warm-object probe | 13,659,891 | 13,669,097 | 0.921233 s | 0.927851 s | 0.870003 s | 0.888283 s |
| Whole-object zstd | 7,352,057 | 7,346,798 | 0.568484 s | 0.567378 s | 0.553080 s | 0.556116 s |
| Combined | 4,928,135 | 4,931,451 | 0.478036 s | 0.482453 s | 0.487197 s | 0.485234 s |

All **18 cells** passed exact stopped-store verification and directional counter
agreement, then removed their owned payload stores. No new serial throughput
improvement is established: Combined median CPU increased **0.92%**, while its
verified time decreased **0.40%**. Raw time increased **2.10%** and zstd time
increased **0.55%**. Small envelope-byte variation is retained; fresh canonical
identities and serialized fields differ between runs. No codec was changed.
The phase split targets dispatch contention, which these single active-pair
cells do not measure. Maximum observed process high-water RSS across these
cells was 36,225,024 bytes; it is not a fleet-memory claim.

## Actual Linux/Firecracker guest boundaries

The registered runner executed five sequential native cells using the pinned
MVM fork `ae12e9d`, Firecracker and kernel already used by the previous pilot.
At most two guests ran simultaneously: one vCPU and 1,024 MiB each, no guest NIC.
The two guests stopped before a separate keyless read-only verifier guest ran.
The preflight refused preexisting Firecracker processes; the temporary executable
link was removed only after its target, inode and owner matched. DeepSeek was
not changed. All five cells passed: **44 publication operations**, including
setup/repeated positions, and **36 additional keyless reboot readbacks**, with
zero fidelity mismatches. No VMM or temporary executable link remained afterward.

Each native arm has one trial. The table compares the same eight target byte
strings within each size. The requested 1 MiB fixture has actual target length
1,047,202 bytes; the requested 16 MiB fixture has **14,851,384 bytes**. Combined
includes two additional setup publications. These are local vsock results,
not WAN or secured raw-TCP qualification. Linux rate enforcement remains absent
and is explicitly reported despite the `experimental64` requested profile.

| Two-guest arm | Actual bytes per target | Logical bytes, both ways | Timed sender CPU | Timed receiver CPU | Timed sender elapsed | Full lifecycle |
|---|---:|---:|---:|---:|---:|---:|
| Whole zstd | 1,047,202 | 7,350,340 | 0.345244 s | 0.296406 s | 0.600230 s | 1.171030 s |
| Combined | 1,047,202 | 4,921,535 | 0.283301 s | 0.248039 s | 0.500345 s | 1.193854 s |
| Whole zstd | 14,851,384 | 107,252,374 | 4.437105 s | 3.767473 s | 7.460040 s | 8.807415 s |
| Combined | 14,851,384 | 62,536,088 | 3.011795 s | 2.573990 s | 5.005895 s | 6.470517 s |

For the larger session Combined used **41.69% fewer logical bytes**, **32.12%
less timed sender CPU** and **26.53% less full lifecycle time** than zstd. In the
smaller session its byte and timed CPU advantages remained, but full lifecycle
was **1.95% slower**. That loss is retained. These compare current arms; they do
not isolate the new scheduling change against old native source.

Logical bytes count both directions at one endpoint. **Across two vsock hops,
the accounted stream total is twice each logical figure.** Counters include
readiness, frame prefixes, application envelopes and encrypted receipt traffic;
they exclude virtio credit/control headers and the independent workload-exit
service. Timed endpoint CPU/elapsed are the existing campaign interval sums:
they omit fixture cloning/identity bookkeeping outside those intervals, JSON
outside timers and final independent readback. They are not total process
lifetime CPU. Full lifecycle wall includes the supervisor's boot, transfers,
verification and teardown phases; its existing timer starts in `session_inner`
after initial journal/link reservation setup. Orchestrator and worker/VMM CPU
are retained separately and must not be added to nested guest CPU as if disjoint.

All four two-guest records show the shared transport owner at peak **two links,
one frame slot and 262,144 reserved serialized-buffer bytes**, within a configured
two-slot/524,288-byte ceiling. Usage returned to zero after joined teardown;
there were no transport-budget refusals in these successful cells. The one-guest
zstd scope regression also passed (0.844047-second lifecycle) and reported a
null transport budget as required. This is actual separate-guest link ownership,
not aggregation of their independent plaintext mailbox ledgers.

## Remaining delivery gates

The [continuing plan](../plans/2026-11-03-persistent-mvm-service.md) remains active.
Next qualify production-retention session lifetime and exact retry identity
beyond the unchanged 2,048-Call cap before enabling any bounded window. Installation
owner exclusion/recovery and aggregate plaintext/cache/workspace partitioning
remain unfinished. Existing object proof locks can still block a same-owner
operation; an accounting dispatch waiting on that owner can indirectly delay
unrelated work. Snapshot-at-work-start time does not provide strict expiry
preemption during a disk operation. A later Reply refusal does not roll back an
already durable object, nor prove gap-free resumable Reply ordering.

Authenticated control-contention measurements, full production-retention/churn,
negotiated pipelining, canonical direct/relay WAN, Windows durable gateway and
the final upstream contribution remain open. Prior Mac deadline failures,
large shared-budget raw refusal, Combined memory costs and prior CPU/timing
limitations remain in the [sustained report](sustained-broker-results-2026-11-02.md).
No energy, certification, fleet-capacity or final MVM-delivery claim follows.
Only compact evidence is stored locally; no guest disk or raw payload was copied
to the Mac.
After all 56 native public-receipt member hashes were verified locally, five
successful owned payload stores/disks were removed on Wick: **1,080,025,365
apparent file bytes**. This is not a measurement of freed filesystem blocks.
The [cleanup record](evidence/2026-11-03-persistent-mvm-service/native-cleanup.txt)
does not remove public evidence, old failed cells, other labs or model data.
