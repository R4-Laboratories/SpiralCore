# Current ae12 native MVM object-transfer qualification

**All 16 local native sessions passed** on the current public MVM contribution
fork `ae12e9deb41cc9a3264196ce22ebc72d4042883d`, at SpiralCore source
`0cf256f2e691849e5c9c019ac2fda7eb97287938`. The runs exercise one Linux guest
connected to a host endpoint and two Linux guests connected through an opaque
host relay, on macOS/ARM64 HVF and Linux/x86-64 Firecracker. Each session uses
one persistent receiver, exact object publication/reopen, canonical Call/Reply
admission, and independently encrypted destination receipts in both directions.

This qualifies the **specified local native object-transfer pilot boundary**.
It is not real-WAN qualification, a fleet/capacity envelope, final production MVM
service adoption, FlowMux forwarding, owner-access delivery, certification or a
physical-energy result. There is one trial per backend/tier/topology/arm; the
numbers are observations, not distributions or confidence intervals. Earlier
[macOS](../mac-native-intermediate/README.md) and
[Linux](../linux-native-intermediate/README.md) intermediate successes and failed
gates remain unchanged and are not pooled into these results.

## Workload and effective transport profiles

The **1 MiB tier** transfers eight common primary payloads of **1,047,202 bytes
each**. The **16 MiB tier** transfers eight common primary payloads of
**14,851,384 bytes each**. Requested tier names are not actual object lengths.
The checkpoint is **3,382 bytes** and schema **48 bytes** in both tiers, as
observed in the publication/readback records. Combined pays for these two extra
setup publications; whole-mixed does not need them.

Whole-mixed uses prepared whole-object zstd, raw fallback, and the same verified
warm-object probe as Combined. Both arms receive the same eight primary inputs:
generated trace, residual, repeat, unrelated opaque data, opaque edit, opaque-edit
repeat, schema-fit, and schema-drift. The primary object IDs and lengths match
across **all arms, topologies and both architectures** for each tier. This input
identity check does not substitute for a cross-host/WAN interoperability run.

The profiles are deliberately kept separate:

- **macOS/HVF:** recorded `experimental64`, enforced 67,108,864 B/s (**64 MiB/s**)
  with an 8,388,608-byte (**8 MiB**) burst. The 1 MiB-tier session totals fit
  inside that burst. This is a local mailbox rate setting, not WAN delay/loss/
  jitter shaping or proof of sustained 64 MiB/s goodput.
- **Linux/Firecracker:** unshaped mailbox stream. Although the plan retains the
  requested profile name, the actual records report `mailbox_rate_enforced:
  false` and null effective rate/burst. Do not treat those nulls as a measured
  zero or pool the Linux timings with rate-profiled macOS timings.

macOS run-time OS records are preserved under each tier. Linux had no original
OS text in these run folders; [os-at-archive.txt](linux/os-at-archive.txt) explicitly
records its **archive-time** Ubuntu 24.04.3 / Linux 7.0.0-42-generic x86-64 and
Rust 1.97.1 metadata. It is not relabeled as an original run-time OS capture.

## Actual bytes in both directions

`Caller → receiver` includes normal Calls and Reply-admission receipts;
`receiver → caller` includes Call-admission receipts and application Replies.
These are actual canonical-envelope and framing counters, not selected-codec
cost estimates. Totals include four-byte frame prefixes and readiness traffic.
They exclude virtio control/credit headers, boot assets and the separate exit
service. They are not NIC/IP wire measurements.

One-guest sessions use one vsock link, so physical stream accounting equals the
logical both-direction total. Two-guest relay sessions forward each byte across
two links; physical accounting is twice the logical total. The original
per-hop counters remain in each result. Candidate fields such as planned
`target_http_body_bytes` are not transmitted-byte measurements.

### macOS / HVF — local experimental64 profile

| Tier | Topology | Arm / original result | Caller → receiver B | Receiver → caller B | Both directions B | Physical stream B | Calls |
|---|---|---|---:|---:|---:|---:|---:|
| 1 MiB | one | [whole-mixed](macos/1m/one-whole-mixed-1048576/result.json) | 6,644,439 | 708,157 | 7,352,596 | 7,352,596 | 73 |
| 1 MiB | one | [combined-cpu](macos/1m/one-combined-cpu-1048576/result.json) | 4,336,818 | 593,154 | 4,929,972 | 4,929,972 | 61 |
| 1 MiB | two | [whole-mixed](macos/1m/two-whole-mixed-1048576/result.json) | 6,638,349 | 706,854 | 7,345,203 | 14,690,406 | 73 |
| 1 MiB | two | [combined-cpu](macos/1m/two-combined-cpu-1048576/result.json) | 4,337,162 | 592,945 | 4,930,107 | 9,860,214 | 61 |
| 16 MiB | one | [whole-mixed](macos/16m/one-whole-mixed-16777216/result.json) | 99,069,749 | 8,174,310 | 107,244,059 | 107,244,059 | 843 |
| 16 MiB | one | [combined-cpu](macos/16m/one-combined-cpu-16777216/result.json) | 57,626,421 | 4,915,916 | 62,542,337 | 62,542,337 | 505 |
| 16 MiB | two | [whole-mixed](macos/16m/two-whole-mixed-16777216/result.json) | 99,087,887 | 8,164,291 | 107,252,178 | 214,504,356 | 843 |
| 16 MiB | two | [combined-cpu](macos/16m/two-combined-cpu-16777216/result.json) | 57,601,605 | 4,907,719 | 62,509,324 | 125,018,648 | 505 |

### Linux / Firecracker — unshaped local stream

| Tier | Topology | Arm / original result | Caller → receiver B | Receiver → caller B | Both directions B | Physical stream B | Calls |
|---|---|---|---:|---:|---:|---:|---:|
| 1 MiB | one | [whole-mixed](linux/1m/one-whole-mixed-1048576/result.json) | 6,646,535 | 711,621 | 7,358,156 | 7,358,156 | 73 |
| 1 MiB | one | [combined-cpu](linux/1m/one-combined-cpu-1048576/result.json) | 4,337,288 | 595,266 | 4,932,554 | 4,932,554 | 61 |
| 1 MiB | two | [whole-mixed](linux/1m/two-whole-mixed-1048576/result.json) | 6,643,817 | 711,940 | 7,355,757 | 14,711,514 | 73 |
| 1 MiB | two | [combined-cpu](linux/1m/two-combined-cpu-1048576/result.json) | 4,332,151 | 591,028 | 4,923,179 | 9,846,358 | 61 |
| 16 MiB | one | [whole-mixed](linux/16m/one-whole-mixed-16777216/result.json) | 99,105,185 | 8,195,208 | 107,300,393 | 107,300,393 | 843 |
| 16 MiB | one | [combined-cpu](linux/16m/one-combined-cpu-16777216/result.json) | 57,598,946 | 4,911,613 | 62,510,559 | 62,510,559 | 505 |
| 16 MiB | two | [whole-mixed](linux/16m/two-whole-mixed-16777216/result.json) | 99,156,757 | 8,218,435 | 107,375,192 | 214,750,384 | 843 |
| 16 MiB | two | [combined-cpu](linux/16m/two-combined-cpu-16777216/result.json) | 57,593,418 | 4,889,896 | 62,483,314 | 124,966,628 | 505 |

## Compute, elapsed and lifecycle time

All values below are original integer **microseconds**. Sender elapsed includes
encoding/selection, setup and verified protocol completion, so computation is
already included: **do not add CPU to elapsed**. Receiver processing elapsed
excludes receive/idle waits and independent readback. Lifecycle includes boot,
relay/orchestration, shutdown and the recovery boot where present. It is distinct
from sender transfer completion. Worker/VMM CPU remains in the original records;
do not sum nested/overlapping CPU domains as total host cost or physical energy.
No send-only time is measured or inferred here.

### macOS / HVF times

| Tier | Topology | Arm | Sender setup + operations CPU | Receiver setup + processing CPU | Sender elapsed | Receiver processing elapsed | Lifecycle elapsed |
|---|---|---|---:|---:|---:|---:|---:|
| 1 MiB | one | whole-mixed | 663,569 | 399,593 | 1,293,669 | 734,810 | 1,500,383 |
| 1 MiB | one | combined-cpu | 524,592 | 332,614 | 1,248,368 | 819,669 | 1,540,996 |
| 1 MiB | two | whole-mixed | 660,621 | 541,607 | 1,133,293 | 576,393 | 1,667,556 |
| 1 MiB | two | combined-cpu | 499,423 | 396,348 | 832,727 | 422,835 | 1,350,099 |
| 16 MiB | one | whole-mixed | 9,541,430 | 5,381,739 | 15,876,227 | 6,863,566 | 16,546,326 |
| 16 MiB | one | combined-cpu | 7,512,178 | 4,098,022 | 14,798,590 | 6,240,730 | 15,383,064 |
| 16 MiB | two | whole-mixed | 9,483,556 | 7,274,020 | 17,681,195 | 7,482,859 | 19,281,857 |
| 16 MiB | two | combined-cpu | 5,348,154 | 3,899,168 | 8,769,003 | 4,072,313 | 11,071,735 |

### Linux / Firecracker times

| Tier | Topology | Arm | Sender setup + operations CPU | Receiver setup + processing CPU | Sender elapsed | Receiver processing elapsed | Lifecycle elapsed |
|---|---|---|---:|---:|---:|---:|---:|
| 1 MiB | one | whole-mixed | 337,992 | 247,436 | 564,048 | 299,365 | 847,119 |
| 1 MiB | one | combined-cpu | 282,321 | 223,477 | 499,594 | 283,873 | 795,792 |
| 1 MiB | two | whole-mixed | 336,083 | 270,843 | 568,708 | 275,140 | 1,151,471 |
| 1 MiB | two | combined-cpu | 272,144 | 238,289 | 484,213 | 243,221 | 1,163,094 |
| 16 MiB | one | whole-mixed | 4,194,147 | 2,797,494 | 6,379,284 | 3,147,478 | 6,853,997 |
| 16 MiB | one | combined-cpu | 2,790,960 | 1,772,476 | 4,227,691 | 2,080,006 | 4,741,094 |
| 16 MiB | two | whole-mixed | 4,202,974 | 3,480,369 | 6,966,439 | 3,508,783 | 8,286,312 |
| 16 MiB | two | combined-cpu | 2,835,365 | 2,381,566 | 4,651,732 | 2,404,076 | 6,046,880 |

## Paired observations and retained losses

Each row compares Combined CPU-conscious selection with whole-mixed **within
the same backend, tier and topology**. Negative changes mean fewer bytes or
less CPU/elapsed; positive changes are losses. Setup/checkpoint/schema costs
are included in the complete session figures.

| Backend | Tier | Topology | Δ logical bytes | Δ sender CPU | Δ receiver CPU | Δ sender elapsed | Δ receiver processing elapsed | Δ lifecycle |
|---|---|---|---:|---:|---:|---:|---:|---:|
| macos | 1 MiB | one | -32.95% | -20.94% | -16.76% | -3.50% | +11.55% | +2.71% |
| macos | 1 MiB | two | -32.88% | -24.40% | -26.82% | -26.52% | -26.64% | -19.04% |
| macos | 16 MiB | one | -41.68% | -21.27% | -23.85% | -6.79% | -9.07% | -7.03% |
| macos | 16 MiB | two | -41.72% | -43.61% | -46.40% | -50.40% | -45.58% | -42.58% |
| linux | 1 MiB | one | -32.96% | -16.47% | -9.68% | -11.43% | -5.17% | -6.06% |
| linux | 1 MiB | two | -33.07% | -19.02% | -12.02% | -14.86% | -11.60% | +1.01% |
| linux | 16 MiB | one | -41.74% | -33.46% | -36.64% | -33.73% | -33.92% | -30.83% |
| linux | 16 MiB | two | -41.81% | -32.54% | -31.57% | -33.23% | -31.48% | -27.03% |

Combined had lower complete-session bytes, sender CPU, receiver processing CPU
and sender elapsed in all eight comparisons. The observed session-level losses
remain explicit: macOS 1 MiB/one-guest receiver processing elapsed rose from
734,810 to 819,669 us, and lifecycle rose from 1,500,383 to 1,540,996 us. Linux
1 MiB/two-guest lifecycle rose from 1,151,471 to 1,163,094 us despite lower
sender completion time and endpoint CPU.

The [operation ledger](operations.json) preserves every setup and object row,
not just favorable aggregate totals. Of 64 common-object paired comparisons,
28 have at least one higher wire-byte, CPU or elapsed counter for Combined:
12 wire-byte, 17 CPU and 16 elapsed losses, with overlap. These are individual
counter observations, not failed transfers or statistically established
regressions. Cold opaque-object expansion and variable per-operation costs are
visible. Neither these local results nor the codec label establishes a
universal gain for arbitrary input or opaque Internet traffic.

## Exact readbacks, receipts and actual boots

The [derived summary](summary.json) verifies all 16 original sessions report
success, canonical service profile, matching sender/receiver byte and Call
counters, and two sent/two received encrypted envelopes per Call at each endpoint.
That is the four-envelope Call → Call receipt → Reply → Reply receipt lifecycle.
The pinned consumer verifies canonical admission receipts and separately admits
application Replies before completion. Raw signed receipt bodies are not
serialized into these records, so archived JSON alone does not permit offline
signature re-verification.

Every session's first exact reopen events were matched to its published object
ID, namespace, kind and exact length. Whole-mixed has **8 logical publications / 6
distinct object IDs**; Combined has **10 / 8**, because repeats deliberately reuse
objects and Combined adds two setup artifacts. Each two-guest session then has a
separate successful Verify-role guest, a read-only disk mount, and **8 or 10**
exact read-only recovery events matched to those same publications.

The records contain **32 successful guest boots**, including **8 recovery
boots**, with **72 exact post-reboot recovery readbacks**. One-guest sessions
have one guest boot and a host receiver; two-guest sessions have sender,
receiver and recovery boots. A two-guest test is not a fleet-scale result.
These counts accumulate separate runs, not 32 concurrent MVMs. Aggregate
capacity, long-lived state, concurrency, fairness, saturation and tail latency
remain the [scale release gate #49](https://github.com/R4-Laboratories/SpiralCore/issues/49).
The Verify role uses the source-pinned keyless/no-mailbox bootstrap path; those
private bootstraps are not copied here. Linux workers record workload exit code
zero and observed process exit. HVF workers report successful guest completion
and clean power-off under their recorded contract; their null workload exit-code
field is not replaced with an invented zero.

## Mechanisms actually selected

Across both sizes and backends, Combined records select `RuntimeV1` for the
generated trace, `SparseV1` for residual and opaque edits, `SchemaZstdV1` for
schema-fit/drift, and verified warm probes for exact repeats. The checkpoint-based
ternary/phase-memory/Kuramoto/CA runtime remains active in this shared endpoint;
there is no second codec implementation in the MVM adapter. The checkpoint and
schema publications are paid setup, not free preexisting receiver state.

A repeat with mode label `Raw` and `exact_repeat_probe: true` does not imply that
a raw object body was sent. Whole-mixed also uses the common verified warm probe.
The recorded byte counters are the authority. Deterministic replay/readback of
these fixtures does not prove identical physical clocks across machines or
arbitrary-data compression without a residual.

## Provenance and archive integrity

Both platforms use project source
`0cf256f2e691849e5c9c019ac2fda7eb97287938` and the public MVM fork
`ae12e9deb41cc9a3264196ce22ebc72d4042883d`. This record does not relabel the earlier
`333f8e8`/`5766754` pilots. Original binary manifests are retained separately
under each platform/tier; no executables or kernels are vendored:

| Backend | Host executable SHA-256 | Guest executable SHA-256 |
|---|---|---|
| macOS/HVF | `2ab27493bd3d87f530350ecc25c2e248bbb370a6d8cd36fc0555a0712d0da09f` | `f137af79664af1e85cbab4b7b84750d8e6e9f55cc9033016d3bcab6fbfbe7431` |
| Linux/Firecracker | `468d9180f332564b2e3348e4594ac71ed8e56c84d8168762c3db3578d66b65e5` | `ae4ae2e8830f6d188ee208a57835c0569843ac909fea3e2e532a87a10b4e9f9f` |

The manifests also pin each kernel and the Linux Firecracker executable. The
macOS host is the signed executable actually used. One- and 16 MiB-tier binary
identities match within each backend.

Original `result.json` and `host-events.jsonl` files are copied verbatim.
[linux/original-records.sha256](linux/original-records.sha256) was captured at
the remote origin through the existing SSH control connection and all copied
Linux files matched it. macOS copies were compared byte for byte against their
original local records. [source-files.sha256](source-files.sha256) inventories the
pinned Git objects for the live, adapter, transfer, gateway and object-store
crates. [source-archive.sha256](source-archive.sha256) pins this reproducible,
uncompressed source stream:

```sh
git archive --format=tar 0cf256f2e691849e5c9c019ac2fda7eb97287938 \
  crates/spiralcore-mvm-live crates/spiralcore-mvm-adapter \
  crates/spiralcore-transfer crates/spiralcore-gateway-core \
  crates/uor-r4-object-store | shasum -a 256
```

Run `shasum -a 256 -c SHA256SUMS` from this directory to verify archived records
and metadata. Cleanup is limited to verified, successful current macOS run
payloads and recorded in [cleanup.json](cleanup.json). Disks, caches, private
MVM homes, bootstraps, failed-run state, executable binaries and kernels are not
part of the Git evidence archive. No Linux payload cleanup is performed here.

Authorized cleanup completed after archive/checksum/bitwise verification: **8**
successful current macOS payload paths removed, **1264533228
apparent bytes** and **407109632 allocated regular-file block bytes**.
The allocated sum is not a global free-space delta on APFS. Per-path sizes and
verified absence are in `cleanup.json`; all result/event/metadata files and old
intermediate/failed runs remain retained.
