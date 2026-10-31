# Actual MVM/vsock integration — 2026-10-31

SpiralCore now transfers lossless objects through actual Linux guests using MVM's
native ARM64 HVF backend. The accepted pilot contains **100 object transfers in
12 persistent sessions**, all independently compared byte-for-byte and by length.
All **50 objects received by a second guest** were also checked after a separate
read-only verifier boot. That verifier carried no mailbox keys.

Source: [live witness](../../crates/spiralcore-mvm-live/README.md),
[canonical adapter](../../crates/spiralcore-mvm-adapter),
[retained MVM source patch](../../crates/spiralcore-mvm-live/mvm-patch/README.md).
MVM base: `3104a12c5b071a6851323fa8115d2afaf3f0bb8b`.
The experimental port is 5257; FlowMux 5253 is unchanged.

The one-guest stage sends from Linux to a host endpoint. The two-guest stage uses
two separate HVF workers and a host relay that forwards bounded encrypted MVM
envelopes without decrypting them. The trusted launcher provisions ephemeral
keys through private pipes/in-memory initramfs: this is not a claim of secrecy
against a compromised launcher/host. No credential-bearing bootstrap is retained.

## Matched seven-object workload

Each session transfers seed, identical repeat, localized 32-byte overwrite,
unrelated object, another localized edit, truncation by 31 bytes and append of
17 bytes. Thus a “16 MiB session” reconstructs **117,440,467 bytes (almost 112 MiB)**,
not a single 16 MiB file. Warm has four additional schema/runtime objects; they
are excluded from this matched table and retained separately in full-session costs.

All modes use the same canonical encrypted transport, persistent receiver,
publication/durability rules and fixture bytes. Whole uses prepared whole-object
zstd after the shared raw seed. **All baselines already deduplicate exact repeats**;
this is a stronger baseline than blindly resending the identical object.

Logical stream bytes below count encrypted envelopes, prefixes and both directions.
For two guests, each logical byte crosses two vsock hops: physical stream bytes
are twice the shown byte count. Virtio control/credit headers are not counted.
Elapsed is the sum of seven encode-through-authenticated-publication operations;
it includes encoding, receiver work, transport waits and acknowledgment.

| Stage | Nominal object size | Raw bytes | Whole zstd bytes | Warm bytes | Raw elapsed | Whole zstd elapsed | Warm elapsed |
|---|---:|---:|---:|---:|---:|---:|---:|
| Guest → host | 1 MiB | 12,202,452 | 12,203,715 | 4,178,250 | 1.795 s | 1.521 s | 0.634 s |
| Guest → host | 16 MiB | 192,966,986 | 192,941,202 | 64,396,580 | 43.956 s | 44.608 s | 14.708 s |
| Guest → guest | 1 MiB | 12,201,740 | 12,203,664 | 4,176,315 | 2.960 s | 4.284 s | 1.609 s |
| Guest → guest | 16 MiB | 192,961,621 | 192,932,537 | 64,408,198 | 42.280 s | 43.854 s | 16.033 s |

These are individual observations from the registered **n=1 functional pilot**,
not medians, p95s, randomized statistical qualification or WAN results.
[Exact common-workload summary](evidence/2026-10-31-mvm-vsock/pilot/common-workload-summary.json)
and [all 100 operation rows](evidence/2026-10-31-mvm-vsock/pilot/operations.jsonl).

## Where the advantage is — and where it is not

| 16 MiB operation | Whole zstd logical bytes | Warm logical bytes | Whole zstd elapsed | Warm elapsed |
|---|---:|---:|---:|---:|
| Localized edit, guest → host | 32,154,709 | 25,461 | 7.563 s | 0.191 s |
| Localized edit, guest → guest | 32,153,825 | 25,497 | 6.921 s | 0.234 s |
| Exact repeat, guest → host | 8,536 | 8,569 | 0.0646 s | 0.1592 s |
| Exact repeat, guest → guest | 8,506 | 8,570 | 0.0589 s | 0.1285 s |

The localized changes save about 99.92% of logical stream bytes and finish much
sooner. Exact repeats are a current loss: the conventional object-cache baseline
already sends only a small authenticated exchange, while Warm does extra sender
work. Do not describe warm reuse as an unconditional speed win.

The shared-runtime checkpoint, generated trace and residual also passed actual
MVM transfer and reboot verification. Their 47,158-byte traces and separate schema
artifact are explicit rows, not evidence that arbitrary data can be reconstructed
from a clock or that physical time is identical across nodes.

## Why the absolute transfer times are slow

The inherited mailbox stream allowance is **4 MiB/s sustained with an 8 MiB burst**.
The one-guest 16 MiB Whole session sends 192,949,618 total bytes including setup;
its ideal bucket floor is 44.003 seconds, close to 44.608 seconds for the seven
operations. This policy limit dominates the observed bulk elapsed time.

A cold 16 MiB object becomes about 32.15 MB on this canonical binding and requires
276 serial authenticated Call/Reply exchanges. MVM's inner payload and outer
ciphertext both use base64 JSON, causing approximately 16/9 data expansion before
metadata and replies. The adapter allows one outstanding call. The rate limit is
preserved, not bypassed; token exhaustion uses sleeping/backpressure.

This is a working canonical MVM integration baseline, not the codec's maximum
throughput. The next performance work should profile serialization/authentication,
receiver materialization and budget waits separately, then qualify an explicit
mailbox bandwidth profile and bounded pipelining. No rate controls were silently
raised for these results. [Source-linked diagnosis](evidence/2026-10-31-mvm-vsock/qualification/performance-diagnosis.json).

## CPU and complete lifecycle costs

CPU is recorded in three distinct accounting domains:

- Sender process CPU as reported by its Linux guest.
- Receiver processing CPU: authentication, reconstruction, publication and reply;
  framed receive, read-buffer allocation/copy and idle receive are excluded.
- Native macOS worker/orchestrator process CPU and MVM's native vCPU-thread CPU.

The guest and host values differ materially. For example, the original one-guest
16 MiB Whole session reports 26.174 s sender guest CPU, 9.455 s for its enclosing
native worker, and 8.185 s for MVM's vCPU-thread accounting. The latter uses Mach
thread user/system time. Neither the guest numbers nor the native figures have
been qualified as a measurement of all physical HVF execution or energy. Never
add guest CPU to its enclosing worker CPU. Earlier commentary's “combined endpoint
CPU” described a partial mixed-domain sum and must not be read as physical compute.

The following **native OS process CPU** sums the orchestrator and every owned
worker once, including the separate verification worker for two guests. Full
lifecycle wall time includes boot, setup, all objects (including Warm's four extras),
shutdown, disk synchronization and read-only recovery. This is distinct from the
matched operation-time table above.

| Stage/size | Raw host OS CPU | Whole host OS CPU | Warm host OS CPU | Raw lifecycle | Whole lifecycle | Warm lifecycle |
|---|---:|---:|---:|---:|---:|---:|
| Guest → host, 1 MiB | 1.074 s | 1.154 s | 0.734 s | 2.118 s | 1.846 s | 1.056 s |
| Guest → host, 16 MiB | 12.116 s | 13.593 s | 6.261 s | 47.279 s | 46.579 s | 16.909 s |
| Guest → guest, 1 MiB | 1.669 s | 1.937 s | 1.262 s | 3.916 s | 6.083 s | 3.160 s |
| Guest → guest, 16 MiB | 13.329 s | 13.431 s | 9.202 s | 52.364 s | 60.065 s | 33.551 s |

The OS-reported CPU and lifecycle totals are lower for Warm in this sample, but
this is not a physical-energy claim. Fixture generation and prepared codec work
are included in setup. Per-operation expected-digest checks and fixture ownership
copies are excluded from operation timing; full lifecycle costs retain that work.
[Complete session summary](evidence/2026-10-31-mvm-vsock/pilot/session-summary.json).

## Retained failures and qualification limits

Two initial 4 KiB attempts transferred and read back all 11 objects but the new
runner rejected MVM's established native Linux two-system-register-trap baseline.
The corrected gate permits exactly two class 0x18 traps only for the pinned kernel
SHA, while retaining clean power-off, no watchdog/reset, clock and exactness gates.
The individual trapped registers remain unexamined; arbitrary hypervisor behavior
is not qualified. Hashing and booting use the same open kernel descriptor.

Pilot01 then stopped during the two-guest 16 MiB Warm verification boot because an
EXT4 printk appeared before a queued JSON newline. Every object was read back
exactly, but the strict receipt parser correctly refused acceptance. Checked tty
drains now precede mount and unmount. Only the affected matched two-guest 16 MiB
group was repeated with the corrected guest; the previous Whole result is retained
as superseded and is not pooled. Nine accepted groups use the earlier binary and
three matched groups use the corrected binary. Transfer and codec code are the
same. [Registration](evidence/2026-10-31-mvm-vsock/registration.json),
[amendment](evidence/2026-10-31-mvm-vsock/pilot/amendment-01.json),
[accepted raw receipts](evidence/2026-10-31-mvm-vsock/pilot/accepted),
[retained failed reboot](evidence/2026-10-31-mvm-vsock/pilot/failed-two-16MiB-warm).

The boundary is real Linux/virtio-vsock/native MVM HVF, with 1 GiB RAM and one vCPU
per guest, a 128 MiB cache quota, 512 MiB endpoint budget and 256 MiB sparse receiver
disk. Both guests share one physical Mac. This does not qualify WAN, Firecracker,
full mvmctl fleet activation, a general TCP/IP router, production mailbox service,
Iroh, concurrent control-plane revocation, new-epoch cache migration, arbitrary
power-loss recovery, FIPS validation or CUI accreditation. The production MVM
supervisor leaves the experimental mailbox binding disabled.

Compact source, hashes, JSON and verification receipts belong in Git. Generated
public fixture stores and VM disks are reproducible data and can be removed after
custody checks. There was no cloud charge, global tool install, or publication to
Ari's upstream repository.

## Final source qualification

Review found one narrower-grant enforcement gap: incoming envelopes used the
global size cap but did not also apply a smaller signed queue contribution.
The final adapter rejects those envelopes before decoding or installing authority.
The focused regression failed before the repair and passes afterward. This fix
postdates the performance executables; every measured grant used the same 262,144
byte cap as the existing global bound, so the measured campaign is unaffected.
Final build and smoke receipts remain separate from the pilot measurements.

The final native live suite passes 29 tests; the original pinned adapter's
conformance suite passes 15 tests. Strict Clippy passes. Selective LLVM coverage
over the native compiled live/adapter source is 2,656/2,869 lines (92.58%), including
source-local tests. This excludes Linux-only syscall code and upstream MVM
dependencies; actual guest execution provides the separate Linux boundary check.
The original full-dependency coverage attempt was stopped for memory pressure,
then replaced with the bounded selective run. Review and measurement receipts
are retained in [qualification](evidence/2026-10-31-mvm-vsock/qualification).
