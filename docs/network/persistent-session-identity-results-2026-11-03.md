# Persistent session identity and registered-send authority

Product source: [`91baf28`](https://github.com/R4-Laboratories/SpiralCore/commit/91baf28efa56da94336d413ca8e912266079f654),
merged through [PR #56](https://github.com/R4-Laboratories/SpiralCore/pull/56)
at `0e02dd7`. [Contract](../specs/2026-11-03-persistent-session-state.md) ·
[evidence](evidence/2026-11-03-persistent-session/) ·
[continuing plan](../plans/2026-11-03-persistent-mvm-service.md).

## Implemented behavior

The real canonical caller now owns one pending request across partial admission
failure. It retains the original payload, options, FIFO position, prepared
envelope, encoded bytes and each successfully registered Call/Send handle.
`Full` before Call registration or after Call registration cannot silently
allocate a replacement identity. A different submitted frame is `Busy`.
An invoked send retains uncertainty and blocks implicit retransmission.

Pending state records the original monotonic deadline as well as the canonical
Unix expiry. A refused request cannot restart its lifetime after a wall-clock
rollback. Failed/fenced/uncertain retry paths expire owned service/tracker state
without forgetting the unresolved FIFO position. This maintenance occurs when
those paths are invoked; autonomous idle maintenance remains unfinished.

`LocalMailboxService::send_call_prepared` sends outside shared broker dispatch
without advancing accounting clocks. It checks exact service/binding/handle
ownership, Pending lifecycle, local time floor and both registered deadlines,
then compares the original generation and semantic request commitment under
the existing binding policy lock. It reuses the existing canonical verification
path, avoiding another cryptographic verification. Successful transmission is
separate from destination acceptance. The borrow prevents local cancellation or
shutdown mutation during that send; it does not prove that time remains valid
after blocking I/O or make delivery atomic with the peer.

The completion path now requires an application Reply before incrementing the
completed Call count or clearing pending identity. Canonical envelopes, object
codec/identity, cryptographic algorithms, resource ceilings, production retention,
the 2,048-Call witness cap and pipeline one remain unchanged.

## Reproduced failures and correctness checks

Three focused RED controls preceded their fixes:

1. The new registered-send tests initially failed because the API did not exist.
2. A real caller received two authenticated positive admission receipts for its
   current Call, with no application Reply. The old completion order returned
   `missing application Reply` after incrementing `calls` and dropping pending
   state. The regression observed `(1, false)` instead of `(0, true)`; it now
   passes with the original identity retained.
3. During implementation, a retained request refused for capacity could register
   after its original monotonic lifetime when the Unix clock had not advanced. The original fixed
   deadline now rejects that registration and releases expired active credits.

Additional checks cover capacity recovery with the same Call handle, envelope
and header; generation replacement; fenced-state expiry; unknown-send identity;
wrong prepared request; foreign service; cancellation; shutdown; clock rollback;
and both deadlines, with no transmitted bytes on refusal. The actual caller
path checks one sequence, one active Call and no prefix/payload before complete
local registration. Nine persistent real-stream object campaign tests pass.

One initial positive API test registered the reverse fixture route instead of
the forward Call route, yielding `Unavailable`; the fixture now derives its
route from the canonical binding. An early time-field compile error and one
strict Clippy style failure were corrected before qualification. The route/lint
failure records remain separate from product/performance results; the original
time-field compiler output was observed but not separately archived. RED logs are
observed source variants, not all independently archived source trees; the
missing-API test source and its hash are retained explicitly.

Independent Rust/code, security and runner/CI reviews approved the implementation.
Exact final-source CI passed **74 checks on each of seven native client rows
(518)** plus **eight Unix storage checks**. Linux passed **66 release checks
and 116 shared conformance checks**, strict Clippy and default guest/analyzers.
Scopes overlap and are not pooled as unique tests. CI executed merge `065abeb6`,
whose tree `0d414f9d56929506413da64dce73f4016ecd3c18` matches the reviewed head.
Seven original CI archives and all **236 inner-file checks** passed verification.
These client/component rows do not establish a Windows MVM backend or execution
of the private live consumer on macOS; this checkpoint's actual native runs are
Linux/Firecracker.

Four isolated instrumented binaries passed **46 tests**, with no LLVM export
warnings. Changed executable production coverage is **251/275 (91.27%)**;
the new pending module is **137/152 (90.13%)**. The broader three-file denominator
is **2,243/2,669 (84.04%)**, not whole-crate or branch coverage. All uncovered
changed lines and reproducible derivation remain in the [coverage packet](evidence/2026-11-03-persistent-session/coverage/README.md).
Instrumented timing is excluded from every performance figure below.

## Registered before/after host transfers

The existing Rust `broker_objects` probe ran **22 cells**: three paired trials
per arm at requested 1 MiB, plus one paired trial per zstd/Combined arm at
requested 16 MiB. Trials 0/2 ran old then new; trial 1 ran new then old. The
registration predates performance execution. Wick remains shared with DeepSeek
and other work; fixed order and small samples do not establish statistical
noninferiority or a causal timing attribution.

Each arm transfers the same eight target positions. Actual target lengths are
**1,047,202** and **14,851,384 bytes**. Combined additionally publishes its
checkpoint and schema. All 22 cells passed direct exact stopped-store readback
and directional counter agreement, then removed owned payload stores. Peak RSS
across the cells was **397,819,904 bytes**. Each fresh process includes both
actors; CPU is measured once for the aggregate, including fixture/codec setup,
and verified time includes both computation and waits. Setup, endpoint thread
CPU, independent readback and cleanup fields remain separate in the raw records.

The small rows are per-column medians of three trials; large rows have one
trial. Byte columns count actual canonical envelopes and framing in both
directions, excluding TCP/IP headers. Small changes in fresh canonical identity
and numeric ciphertext serialization are retained. No codec was changed.

| Actual bytes per target | Arm | Before wire bytes | After wire bytes | Before CPU | After CPU | Before verified time | After verified time |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1,047,202 | Raw with exact-object probe | 13,651,874 | 13,660,975 | 0.944261 s | 0.952936 s | 0.889565 s | 0.940073 s |
| 1,047,202 | Whole zstd | 7,349,431 | 7,351,668 | 0.608467 s | 0.587952 s | 0.587799 s | 0.586519 s |
| 1,047,202 | Combined | 4,931,451 | 4,929,203 | 0.491027 s | 0.532722 s | 0.504990 s | 0.536767 s |
| 14,851,384 | Whole zstd | 107,341,077 | 107,310,730 | 7.316435 s | 7.378298 s | 6.624879 s | 6.923090 s |
| 14,851,384 | Combined | 62,502,561 | 62,523,408 | 4.629288 s | 4.754053 s | 4.263772 s | 4.506860 s |

The small Combined median CPU increased **8.49%** and verified time increased
**6.29%**. Small raw CPU/time increased **0.92%/5.68%**; small zstd CPU/time
decreased **3.37%/0.22%**. The large single-trial CPU/time increases were
**0.85%/4.50%** for zstd and **2.70%/5.70%** for Combined. All remain inside the
registered 10% arm-median exploratory flag threshold, which is not a statistical
acceptance claim. One individual small Combined paired CPU increase was 10.02%;
its original record is retained. Both role ThreadCPU medians rose: sender
254,341 → 275,883 microseconds and receiver 236,671 → 255,718 microseconds,
while aggregate provisioning CPU decreased. The extra post-registration poll
does add capability-signature checking and expiry work; the new send gate itself
reuses its existing verifier. The measurements do not isolate those operations
from shared-host load as a cause. This safety change establishes no new throughput improvement. The
existing byte advantage over zstd remains, with the measured costs disclosed.

## Actual Linux/Firecracker execution

Three registered native cells passed **26 publication operations** and
**18 additional exact keyless reboot readbacks**, with zero mismatches. At most
two guests ran simultaneously, one vCPU and 1,024 MiB each, without a guest NIC.
The original pinned kernel/Firecracker were reused. The static GNU guest had no
interpreter. Source hashes for all **276 tracked crate files** matched Wick
before execution and again after coverage; normal binary hashes remained fixed.

| Native topology and arm | Actual bytes per target | Logical bytes, both directions | Timed sender CPU | Timed receiver CPU | Lifecycle wall |
|---|---:|---:|---:|---:|---:|
| Guest → host, whole zstd | 1,047,202 | 7,349,681 | 0.349582 s | 0.256487 s | 0.902807 s |
| Two guests, whole zstd | 14,851,384 | 107,319,201 | 4.448006 s | 3.700698 s | 8.843999 s |
| Two guests, Combined | 14,851,384 | 62,540,443 | 3.081656 s | 2.533253 s | 6.548710 s |

The current large native Combined arm used **41.72% fewer logical bytes,
30.72% less timed sender CPU and 25.95% less lifecycle time** than current zstd.
Each native arm has one trial; this does not isolate the safety change against
old native source. Across two vsock hops physical accounted stream bytes are
twice the listed logical totals. Readiness, prefixes, envelopes and encrypted
receipts are included; virtio credit/control headers and workload-exit service
traffic are excluded. Linux requested `experimental64` but still reports rate
enforcement absent. The runs are local native evidence, not WAN.

Native endpoint CPU sums timed campaign setup and operation intervals, including
fixture generation, service setup and codec preparation performed within them.
It excludes untimed gaps such as per-object cloning/identity bookkeeping between
intervals, record output and final independent verification outside the timers.
It is not total lifetime CPU. The existing lifecycle wall timer
starts in `session_inner` after initial journal/link admission and covers boot,
transfer, keyless verification and teardown. Nested worker/VMM CPU is retained
separately rather than added to guest CPU. Each two-guest transport owner peaked
at two links/one frame/262,144 reserved serialized bytes and returned to zero;
Stage One explicitly reports no transport-budget coverage. Those snapshot fields
were checked after execution; the runner does not contain the older 180-second
posthoc assertion or detailed budget assertion.

All 34 original public-receipt archive members verified after transfer. No VM
disk or payload was copied to the Mac. Three verified successful owned Wick
stores were removed after custody checks: **543,154,453 apparent file bytes**,
not a measured count of freed filesystem blocks. Other labs and prior failed
records remain untouched.

## Continuing release work

C7 now retains pre-send identity and enforces the registered send boundary.
The existing fixed two-frame caller/receiver choreography remains: arbitrary
duplicates, older events, lost observations and acknowledgment uncertainty
still need **both-peer C8 handling and cached Reply replay** before retransmission
is enabled. Reply payload/ack recovery state is not generally retained yet.
Original request identity is retained, and an invoked send blocks new work.

Autonomous idle expiry, same-owner sessions beyond 2,048 Calls with production
retention, host installation exclusion/recovery and aggregate plaintext/cache
partitions remain open. Pipeline stays one. Bounded window negotiation,
concurrent control latency, canonical direct/relay WAN, remaining native backend
and Windows durable-gateway qualification and final upstream delivery remain
active in the [plan](../plans/2026-11-03-persistent-mvm-service.md) and issues
[#43](https://github.com/R4-Laboratories/SpiralCore/issues/43),
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49) and
[#42](https://github.com/R4-Laboratories/SpiralCore/issues/42).
