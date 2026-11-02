# Linux/Firecracker canonical MVM results — 2026-11-02

The native Linux host implementation passed one-guest/host and two-guest tests,
then a registered 36-session comparison using the existing Combined transfer
core. All **336 publications reconstructed exactly**, and the two-guest stage
completed **168 additional credential-free, read-only reboot verifications**.
A forced receiver-worker death produced a failed session and removed its owned
Firecracker process. This establishes a bounded Linux backend witness alongside
the existing HVF witness; production mailbox integration remains open.

## Source, environment and workload

- SpiralCore measured source: `a9f038c49bcaf69ce7d6780970dadd412a31a573`.
- MVM base: `3104a12c5b071a6851323fa8115d2afaf3f0bb8b` plus the retained fourteen-file experimental patch.
- Rust 1.97.1 release host and static x86-64 GNU guest, Firecracker 1.17.0,
  pinned MVM image-set v0.2.4 kernel, on the approved Wick Linux KVM host.
- Dedicated private state; sequential sessions; at most two 1-vCPU/1-GiB guests.
  No guest NIC, FlowMux replacement, `mvmd` dependency or customer payloads.
- Stages `one` and `two`; requested tiers 1 MiB and 16 MiB; trials 0, 1, 2;
  rotating arm order. Eight mixed targets per session have exact lengths
  **1,047,202** or **14,851,384** bytes. These are fixture tiers, not arbitrary
  application corpora. Combined additionally pays checkpoint/schema publication.
- All arms use the same Linux path, which currently has **no enforced mailbox
  rate limiter**. Effective rate/burst are null. The requested `experimental64`
  profile is recorded as intent only; timings are not pooled with HVF's shaped
  results. Wick was also used by other work; host-load snapshots are retained.

[Registration and binary/kernel hashes](evidence/2026-11-02-linux-mvm/pilot36/registration.json),
[schedule](evidence/2026-11-02-linux-mvm/pilot36/schedule.json),
[measurement contract](evidence/2026-11-02-linux-mvm/measurement-contract.md),
[asset provenance](evidence/2026-11-02-linux-mvm/assets.json),
[build and run guide](linux-firecracker.md).

## Two MVMs: bytes, CPU and elapsed time

Each row is the median of three complete sender sessions, including fixture and
codec preparation, paid setup and transfers. Independent reboot/readback and
VM boot are included in lifecycle records, outside sender session elapsed.
The byte column counts encrypted mailbox envelope/framing bytes in both
directions across one logical delivery path. The two host-relay hops carry twice
that count. It excludes the separate workload-exit service and virtio
credit/control traffic; this is not full-device or physical-NIC accounting.

| Exact bytes per target | Arm | Logical mailbox bytes | Sender CPU | Sender session elapsed |
|---:|---|---:|---:|---:|
| 1,047,202 | Prepared whole-object zstd | 6,485,072 | 90.000 ms | 180.628 ms |
| 1,047,202 | Exhaustive Combined | 4,208,975 | 94.460 ms | 179.624 ms |
| 1,047,202 | CPU-conscious Combined | 4,207,268 | 93.389 ms | 175.452 ms |
| 14,851,384 | Prepared whole-object zstd | 97,347,823 | 1,339.786 ms | 2,424.832 ms |
| 14,851,384 | Exhaustive Combined | 56,558,976 | 1,201.477 ms | 2,084.206 ms |
| 14,851,384 | CPU-conscious Combined | 56,544,015 | 1,181.150 ms | 2,047.916 ms |

For the larger two-MVM batch, CPU-conscious Combined's median paired reductions
against prepared whole-object zstd are **41.92% mailbox bytes, 11.84% sender CPU
and 15.00% sender elapsed**. Its median physical mailbox-stream count is
113,088,030 bytes versus 194,695,646. Receiver-processing plus sender CPU falls
11.11% by paired medians, but that sum excludes receiver framed receive and is
not total host CPU.

Exhaustive Combined's corresponding larger-batch reductions are **41.91% bytes,
10.32% sender CPU and 14.05% elapsed**. CPU-conscious selection is optional and
can miss a smaller eligible encoding outside this workload. Small encrypted-byte
differences between arms also include per-session envelope identity/encoding
variation; no new codec improvement is inferred from those differences.

## One guest to host, and the small-workload losses

| Exact bytes per target | Arm | Logical mailbox bytes | Sender CPU | Sender session elapsed |
|---:|---|---:|---:|---:|
| 1,047,202 | Prepared whole-object zstd | 6,489,194 | 87.814 ms | 180.593 ms |
| 1,047,202 | Exhaustive Combined | 4,207,367 | 91.905 ms | 190.279 ms |
| 1,047,202 | CPU-conscious Combined | 4,208,382 | 90.402 ms | 190.268 ms |
| 14,851,384 | Prepared whole-object zstd | 97,329,491 | 1,327.454 ms | 1,995.787 ms |
| 14,851,384 | Exhaustive Combined | 56,554,847 | 1,198.054 ms | 1,712.650 ms |
| 14,851,384 | CPU-conscious Combined | 56,555,478 | 1,176.224 ms | 1,691.003 ms |

CPU-conscious Combined versus zstd, using medians of matched trial ratios:

| Topology / tier | Fewer mailbox bytes | Less sender CPU | Less elapsed |
|---|---:|---:|---:|
| One guest / 1 MiB tier | 35.15% | **−2.84%** | **−5.23%** |
| Two guests / 1 MiB tier | 35.12% | **−1.43%** | 3.73% |
| One guest / 16 MiB tier | 41.89% | 11.39% | 15.27% |
| Two guests / 16 MiB tier | 41.92% | 11.84% | 15.00% |

Negative reductions mean a regression. Smaller transfers save bytes but do not
uniformly save CPU or time on this fast local path. Percentages are medians of
paired trial ratios, not ratios of the separately reported median totals.

The larger batch also contains per-object losses. In the two-guest 16 MiB tier,
CPU-conscious schema-fit/drift operations save only 0.7–1.1% of mailbox bytes
while using about 50% more sender CPU and 34–35% more elapsed time than zstd.
The unrelated object uses essentially equal bytes but about 7.02% more sender
CPU and 1.45% more elapsed. These rows remain in the operation-level analysis;
the aggregate benefit depends on the workload mix and reusable state.

## Correctness, lifecycle and reproducibility

The [raw session receipts](evidence/2026-11-02-linux-mvm/pilot36/) retain every
operation's object identity, exact length, directional counters, measured CPU
and elapsed time, verified reopen, and keyless reboot results. The analyzer
checks 96 matched payload-identity groups, all 36 session identities, directional
counter agreement and expected verification counts. It rejects failed canonical
exit, failed reboot verification, payload mismatch and missing byte fields.

An [offline cross-backend comparison](evidence/2026-11-02-linux-mvm/cross-backend-payloads.json)
also matched all 96 deterministic payload-identity/length groups against the
earlier HVF campaign. That confirms these recorded payloads agree across the
backends; it does not substitute for a Mac-to-Linux transport test.

[Analysis](evidence/2026-11-02-linux-mvm/pilot36/analysis.json) was regenerated
locally from those raw receipts. Its original output is preserved as
`analysis-original.json`: the only correction is the obsolete HVF-only platform
label and an explicit backend field. All numerical values compare identically.
No VM measurement was rerun to obtain a different result.

MVM's canonical workload-exit channel reports successful application cleanup;
the host then terminates the owned VMM and confirms exit. Receipts explicitly
say `natural_poweroff_observed: false`. The forced-worker test covers early
receiver death after Firecracker exists and before peer boot, not every
mid-transfer/simultaneous-sibling fault window. Existing interrupted-transfer
contract tests separately reject partial publication and recover exact bytes.

The [execution receipt](evidence/2026-11-02-linux-mvm/execution-receipt.json)
retains the initial user-session-bus failure, the private-PATH/sudo failure,
observer permission failure and successful scoped fault test. No guest booted
in the two prerequisite failures. Final cleanup found no Firecracker processes;
the temporary executable-path link was removed by exact recorded identity.
Verified ephemeral stores were removed; public receipts and failed-run evidence
remain. There were no DeepSeek or other-lab process/configuration changes.

[Validation](evidence/2026-11-02-linux-mvm/validation.json): 37 macOS contract tests,
31 Linux contract tests, strict Clippy, independent Linux CI, and focused
[Rust/security review](evidence/2026-11-02-linux-mvm/review.md).

## Remaining qualification

This three-trial pilot demonstrates functional x86-64 Linux/Firecracker host
integration and conditional byte/CPU/time savings. It does not establish a WAN
result, physical energy savings, total-host CPU savings, ARM64 Linux execution,
Windows support or production mailbox acceptance. The Firecracker CPU field is
only the last observed process sample before termination and excludes stop work.

Upstream #4629's production service, durable replay and broader lifecycle,
#4633's Iroh direct/relay transport, rate enforcement, and #4626/#4635's full
contention/isolation matrices remain open. #4607's FlowMux/host-network adapter
is a separate decision. The requested development-shell and restricted
sealed-workload management modes likewise require separate admitted services.
