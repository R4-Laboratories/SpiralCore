# Shared object-v1 endpoint and canonical MVM results — 2026-11-02

The shared Rust object-v1 endpoint and canonical MVM mapping passed a native
Linux compatibility comparison: **24 sessions, 240 exact publications and 120
additional credential-free guest-reboot readbacks**. All 120 matched operation
groups retained the same object identity, exact length, selected codec, planned
application-frame bytes and actual mailbox-frame counts. There is one transfer
implementation behind the gateway and MVM adapters. Existing SCMOBJ01 frames,
mode IDs and canonical MVM encryption/envelopes remain unchanged.

This milestone standardizes the protocol contract and endpoint API. It is not a
new codec optimization. The larger workload showed little measured CPU/time
change, while the smaller workload recorded sender CPU and elapsed regressions.
Those losses are retained below. This is a local Firecracker/vsock comparison,
not a new WAN or production MVM mailbox qualification.

## What is standardized

The [v1 specification](../specs/spiralcore-v1.md) and
[descriptor schema](../api/spiralcore-v1.schema.json) define the pinned
`spiralcore-object` 1.0 / `object-v1` profile. The shared receiver enforces
immutable codec/object limits before admission while preserving core authority
checks. The extracted shared sender checks the selected profile before handing
frames to a carrier. MVM dispatch consumes authenticated canonical mailbox
payloads through that same endpoint API; payload metadata cannot create authority.

The first canonical MVM adapter pins the full profile from trusted configuration
and content-type version 1. The capability API is pure validation/intersection,
not implemented dynamic wire discovery. Existing packet/TUN service semantics
are not claimed to be object-v1 until a packet profile is specified and qualified.

## Source and measurement contract

- Candidate measured source: `f45c6c27a5062fc7123af6dd3f620d872f170bac`.
- Frozen legacy baseline: `a9f038c49bcaf69ce7d6780970dadd412a31a573`.
- Both use the same pinned MVM base
  `3104a12c5b071a6851323fa8115d2afaf3f0bb8b`, retained mailbox patch,
  Rust 1.97.1 release builds, static x86-64 GNU guest, Firecracker 1.17.0 and kernel.
- Sequential sessions on the approved Wick Linux KVM host, with at most two
  1-vCPU/1-GiB guests, public synthetic fixtures, no guest NIC or other-lab changes.
- Stages: one guest → host, and guest → ciphertext relay → guest, followed by a
  credential-free read-only recovery guest. Each implementation/stage/tier has
  three matched trials with alternating implementation order.
- Every session uses the existing CPU-conscious Combined policy, eight targets
  plus paid runtime-checkpoint/schema setup. Each target's exact length is
  **1,047,202** or **14,851,384** bytes; the nominal tiers are 1 MiB and 16 MiB.
- Linux currently has no enforced mailbox rate limiter. Requested
  `experimental64` is intent; effective rate and burst remain null.

[Registration and executable identities](evidence/2026-11-02-v1-endpoint/registration.json),
[schedule](evidence/2026-11-02-v1-endpoint/schedule.json),
[measurement contract](evidence/2026-11-02-v1-endpoint/measurement-contract.md),
[execution and validation receipt](evidence/2026-11-02-v1-endpoint/execution-receipt.json),
[source-file hashes](evidence/2026-11-02-v1-endpoint/source-files.sha256).

## Measured bytes, CPU and elapsed

The rows are medians of three complete sender sessions, including preparation
and paid setup. Bytes count actual encrypted canonical mailbox envelopes and
stream framing in both directions across one logical delivery path. Two guests
carry twice that count across their two mailbox hops. Counts exclude the separate
workload-exit service and virtio credit/control traffic.

| Topology / exact target size | Implementation | Logical mailbox bytes | Sender CPU | Sender session elapsed | Full lifecycle |
|---|---|---:|---:|---:|---:|
| One guest / 1,047,202 B | Legacy | 4,207,425 | 90.361 ms | 191.564 ms | 493.099 ms |
| One guest / 1,047,202 B | Shared v1 | 4,208,549 | 102.932 ms | 216.173 ms | 487.485 ms |
| Two guests / 1,047,202 B | Legacy | 4,207,069 | 90.584 ms | 173.423 ms | 829.537 ms |
| Two guests / 1,047,202 B | Shared v1 | 4,207,838 | 101.646 ms | 181.173 ms | 822.781 ms |
| One guest / 14,851,384 B | Legacy | 56,568,216 | 1,171.032 ms | 1,681.790 ms | 2,189.313 ms |
| One guest / 14,851,384 B | Shared v1 | 56,557,278 | 1,180.098 ms | 1,684.890 ms | 2,207.157 ms |
| Two guests / 14,851,384 B | Legacy | 56,552,816 | 1,177.301 ms | 2,038.226 ms | 3,429.002 ms |
| Two guests / 14,851,384 B | Shared v1 | 56,549,245 | 1,176.205 ms | 2,045.014 ms | 3,441.460 ms |

Paired changes are the median of candidate/baseline trial ratios minus one;
positive CPU/time values mean regressions. These differ from ratios of the
separately reported medians above.

| Topology / tier | Mailbox-byte change | Sender CPU change | Sender elapsed change | Lifecycle change |
|---|---:|---:|---:|---:|
| One guest / 1 MiB | +0.0478% | **+13.53%** | **+11.43%** | −1.14% |
| Two guests / 1 MiB | +0.0183% | **+12.87%** | **+2.51%** | +1.35% |
| One guest / 16 MiB | −0.0113% | +0.76% | +1.21% | +1.78% |
| Two guests / 16 MiB | −0.0064% | −0.09% | +0.33% | +0.36% |

Every small session sent/received 61 actual mailbox frames; every larger session
sent/received 505, identically for both implementations. Canonical envelope
identities/encoding vary per session, so tiny encrypted-byte differences do not
establish a codec improvement. All matched *planned* application-byte totals
also agree. Those totals count entire batches before the carrier callback; early
Published responses can skip unsent frames. They are explicitly **not actual
plaintext transport-byte measurements**.

The operation-level records retain variability and losses. For example, the
one-guest small schema-drift operation recorded 18.086 → 30.733 ms median sender
CPU, and a small exact-repeat operation recorded a large elapsed outlier without
a corresponding CPU increase. Three trials on a shared host do not identify a
causal source of these regressions. They do not justify claiming zero API
overhead, a p95 result, or statistical significance.

[Full analysis and per-operation records](evidence/2026-11-02-v1-endpoint/analysis.json)
include receiver processing CPU, exchange CPU/wall time and enclosing worker
measurements. Endpoint CPU is a partial sum; keep guest CPU separate from
worker/VMM CPU. Exchange elapsed includes crypto, receiver work and waiting;
it is not send-only time. Full lifecycle includes VM start, independent readback
and two-guest recovery, outside sender session elapsed.

## Correctness and reproducibility

All publications passed exact length/byte reopen checks. Every two-guest object
also passed keyless read-only guest reboot verification. The analyzer validates
all 24 unique registration cells, all 120 matched operation groups and directional
carrier counters. It rejects modified planned bytes, identities, actual frame
counts and duplicate registration cells. Parser tests use explicitly synthetic
labels on old receipts; they are never counted as new transfers.

The six archived crate trees contain 210 source files; their hashes still matched
after build/testing. Candidate adapter tests needed an isolated Cargo lockfile
resolution for local MVM path overrides: the initial `--locked` refusal is
recorded, the generated test lock was hashed, and the frozen source lock was
restored. The measured live release used its unchanged lockfile throughout.

Linux verification passed **31 live contract tests, 36 gateway tests, 8 shared-v1
contract tests and 24 canonical-adapter tests**. The static guest has no ELF
interpreter. All 24 copied receipt sets verified locally, and local reanalysis
matches the remote result byte-for-byte. The temporary standard-path Firecracker
link was removed after exact target/inode/owner verification, and no Firecracker
processes remained. Verified ephemeral payload stores were removed after their
required readbacks; compact public receipts remain in Git.

This campaign compares v1 against the previous Combined implementation. The
[earlier Linux comparison against prepared whole-object zstd](linux-mvm-results-2026-11-02.md)
is separate evidence; it was not rerun or pooled into these paired ratios.
Production durable mailbox service, Iroh/WAN transport, FlowMux integration,
rate enforcement, broader fault/contention tests and separately admitted guest
access services remain qualification work. The [Linux build/run guide](linux-firecracker.md)
retains the deployment prerequisites and accounting boundaries.
