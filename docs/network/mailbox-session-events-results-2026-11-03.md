# Bounded canonical mailbox events and exact cached Reply replay

SpiralCore now handles duplicate, earlier and Late canonical mailbox events
without confusing them with completion of the current object request. The
receiver can replay its exact cached Reply when the original Call is repeated
while it awaits a receipt. It executes no additional object work and allocates
no new Reply identity or sequence. This is the next persistent-service slice,
delivered through [PR #60](https://github.com/R4-Laboratories/SpiralCore/pull/60);
automatic timed retries and reconnect/resume remain disabled.

The existing conditional byte advantage survives this change. Four local
Linux/Firecracker cells passed **36 publication operations and 36 additional
keyless reboot readbacks**, with zero mismatches. In the current large mixed
sequence, Combined used **41.71% fewer logical mailbox bytes, 31.45% less timed
sender CPU and 26.20% less full lifecycle time** than equally tuned whole-object
zstd/raw with the same exact-object warm probe. These are exploratory local
observations, not WAN or fleet qualification or a causal speedup claim.

[Complete compact evidence](evidence/2026-11-03-session-events/README.md) ·
[Independent arithmetic and all losses](evidence/2026-11-03-session-events/measurement-review.md) ·
[Machine-readable summary](evidence/2026-11-03-session-events/summary.json) ·
[Registration and bounds](evidence/2026-11-03-session-events/registration.json) ·
[Source and executable pins](evidence/2026-11-03-session-events/execution-pins.json).

## Behavior and ownership

- One original Call retains its exact request wire, partial service/tracker
  handles, lifetime, receipt disposition and Reply acknowledgment progress.
  A receipt-send failure leaves the charged application payload in the canonical
  service; successful completion consumes it only after fresh deadline/FIFO checks.
- A caller sends actual canonical Late and Duplicate receipts. Unknown negative
  events cannot advance current progress. The additive optional registered-Reply
  observer preserves the existing strict API and known identity, authorization,
  expiry, commitment and verification errors.
- Before admitting a candidate repeat, the receiver authenticates it and checks
  that its cached Reply is exactly the current Reply. A different Call gains no
  admission while the receiver awaits acknowledgment. Replay uses the same
  cached envelope and tracker handle, with no resealing or object execution.
- Each receiver exchange permits at most **16 incoming frames**, including its
  initial Call, and **eight Reply sends**, including the original. An incomplete
  exchange fences a subsequent invocation before another socket read. An eligible
  ACK at the send ceiling still completes; a ninth send is refused.
- One original Instant/Unix/owner-monotonic lifetime guards initial receipt,
  application work, post-work Reply preparation, sending, replay and cache release.
  Independent security review found the initial post-work deadline gap in the
  first patch; its finding, correction and boundary tests remain public.

Canonical MVM still owns encryption, signatures, capabilities and service
dispositions. `AcceptedEphemeral` is distinct from durable object publication.
There is no new envelope, ACK, retry timer, raised quota or pipeline window.

## Current native comparison

Each target is **14,851,384 actual bytes**, at the requested 16 MiB tier. There
are eight target positions; Combined additionally pays for a 3,382-byte runtime
checkpoint and 48-byte schema. The sequence mixes generated data, residuals,
exact repeats, unrelated bytes, sparse opaque edits and schema fitting/drift.
It is not eight unrelated cold objects.

| Version | Arm | Logical traffic, both directions | Timed sender CPU | Timed receiver CPU | Full lifecycle |
|---|---|---:|---:|---:|---:|
| Before | Whole zstd/raw | 107,320,914 B | 4.515531 s | 3.755624 s | 8.896883 s |
| Before | Combined | 62,510,789 B | 3.028168 s | 2.485406 s | 6.428218 s |
| Current | Whole zstd/raw | 107,299,202 B | 4.384472 s | 3.639661 s | 8.652877 s |
| Current | Combined | 62,547,228 B | 3.005620 s | 2.502463 s | 6.385582 s |

Physical accounted stream traffic crosses two vsock hops: **214,598,404 B** for
current Whole and **125,094,456 B** for current Combined. Before/after Combined
receiver CPU increased **0.69%**, and bytes increased **0.0583%**. Whole relay CPU
increased **4.36%**. All observations remain; reliability changes do not establish
a CPU optimization.

Current schema-fit used 1.00% fewer bytes than Whole, but sender CPU and operation
wall were **8.24% and 1.99% higher**. Schema-drift used 0.64% fewer bytes while CPU
and wall were **18.85% and 12.22% higher**. The byte-first selection policy is
unchanged; these remain optimization targets. The unrelated cold target used
roughly **31.3 MB of canonical logical traffic** in either arm for 14.85 MB of
content. This compares two canonical MVM paths, not generic secured raw/TLS.
Exact-repeat traffic is about **20.4 kB including canonical envelope/receipt
costs** here; smaller object-core wire figures do not describe this outer mapping.

Native cells used at most two simultaneous guests, each one vCPU and 1 GiB RAM,
without guest NICs. Both versions peaked at two relay links/two frame slots/
524,288 reserved serialized bytes, returned to zero and recorded no refusal or
fencing. These are orchestrator transport credits; aggregate guest plaintext/
cache ownership and actual native process RSS were not qualified by this run.
Linux's requested `experimental64` rate remains explicitly unenforced.

## Host paired costs and verification

The unchanged Rust direct-TCP harness completed **22 cells / 11 matched pairs**,
with **192 publications and 192 independent exact readbacks**. It uses one
persistent receiver per cell and includes setup/seeding. Small targets are
1,047,202 actual bytes at the requested 1 MiB tier. Large targets have the native
length above. Every matched pair has identical target kinds, lengths and IDs.

| Requested 1 MiB arm | Median paired process CPU change | Median paired verified wall change |
|---|---:|---:|
| Raw with the same exact-object cache | +2.59% | +4.02% |
| Whole zstd | -2.64% | +0.56% |
| Combined | -4.06% | -4.19% |

These are medians of three paired percentage changes. Individual losses remain
in the summary; Raw trial 1 increased CPU/wall **6.43%/7.27%**. At the large tier,
Whole CPU/wall increased **0.40%/2.57%**, while Combined decreased **2.40%/1.35%**.
Peak measured host RSS was **397,512,704 B**. The registered sampled ceiling is
1 GiB; this is not proof of enforced total-memory admission.

The reviewed source is `5c41d6ac2d9b40e6eead62e201d5e9b020f96019`, with a verified
284-entry crate manifest and protected normal executables outside Cargo's target.
The exact-source local release run passed **92 checks**, strict Clippy, **116
adapter conformance checks and eight object-v1 checks**. Counts have overlapping
scope and are not a count of independent scientific experiments. Original REDs,
fixture failures, deadline review finding and GREEN records are retained.
Hosted qualification, source identities and original artifact custody are in the
[qualification packet](evidence/2026-11-03-session-events/qualification/).

The hosted source passed **81 checks on each of seven native client rows**
(567 repeated portable executions), plus eight separate Unix storage checks.
Windows Server 2022/2025 x64, Windows 11 ARM64, macOS 26 ARM64/Intel and Ubuntu
24.04 ARM64/x64 are client/component/TCP-loopback evidence, not Windows MVM,
durable Windows storage or WAN evidence. All seven original ZIP digests and
236 internal checksums verified. The tested PR merge tree equals the reviewed
head tree. Linux's workflow separately confirms the 92/124 scopes above.

One selective, isolated [coverage run](evidence/2026-11-03-session-events/coverage/)
passed 91 overlapping tests: **287/309 changed executable lines (92.88%)** and
**1,495/1,582 production lines across these three files (94.50%)**. All 22
uncovered changed lines are retained; branch mapping is unavailable. This is
line coverage of the scoped source, not whole-project or semantic-path coverage.
Executed instrumented binaries and raw profiles remain protected on Wick;
normal benchmark binaries and all 284 source hashes were unchanged. No
instrumented timing is included in the performance comparison.

## Measurement and delivery limits

Host aggregate process CPU includes both actors and setup; per-role ThreadCPU
is separate. Do not sum overlapping host operation ProcessTime intervals.
Native endpoint CPU covers timed setup/operation intervals, excluding untimed
gaps, JSON output and final independent verification. Native lifecycle starts
after journal/link admission and includes boot, transfer, keyless verification
and teardown. Nested worker/VMM CPU and relay CPU remain separate. Actual stream
counters include encrypted envelopes, framing and receipts in both directions;
they exclude lower-level virtio/TCP/IP control traffic. No send-only or energy
measurement is inferred.

Service-profile per-Call counts describe the nominal successful four-frame
exchange. They are not transmitted-event counters after replay: use the actual
stream counters, which include every additional duplicate receipt and cached
Reply transmission. The focused replay tests assert those larger exact counts.

Both archives and all **73 host/native members** verified locally before removal
of four successful owned native disks totaling **1,073,741,824 apparent file
bytes**. Apparent length is not measured freed allocation. The Rust host engine
cleans its successful store after exact verification and record emission, before
archive collection. No payload or VM disk was copied to the Mac.

Full C8 retry/resume, production-retention sessions beyond 2,048 Calls, installed
host exclusion/orphan recovery, shared plaintext/cache partitions, negotiated
pipelining, remaining native roles and canonical cross-host WAN acceptance remain
open in [#43](https://github.com/R4-Laboratories/SpiralCore/issues/43),
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49) and parent
[#42](https://github.com/R4-Laboratories/SpiralCore/issues/42). This slice is not the
final upstream MVM contribution or a certification claim.
