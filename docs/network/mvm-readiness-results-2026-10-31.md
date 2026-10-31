# Canonical MVM mailbox host integration

The Rust adapter now carries unchanged SpiralCore frames through MVM's actual
canonical signing, encryption, outbound authorization, bounded channel and
authenticated opening. All **78 measured objects reopened byte-for-byte exactly**
across twelve persistent sessions. This completes a host conformance milestone,
not a live guest/vsock, WAN, durable mailbox inbox or security qualification.

Codec policy is frozen at SpiralCore `b2dacc6`; the measured adapter source is
`3553af21ea8a2bd2cde70ab7a6c1317e0dbf0dad`, pinned to MVM
`3104a12c5b071a6851323fa8115d2afaf3f0bb8b`. Neither existing core crate changed.
The [adapter README](../../crates/spiralcore-mvm-adapter/README.md) provides build
commands, the concrete Rust seam and guest integration requirements.

## What was exercised

One pre-registered campaign used three paired trials each at 1 MiB and 16 MiB,
alternating warm/Whole arm order. Each arm retained a receiver and prepared sender
for an initial pseudorandom Raw object, a 32-byte midpoint edit, an independent
replacement, another 32-byte edit, a 31-byte truncation and a 17-byte append.
Warm also fitted/published its schema artifact once. Whole used the equally
prepared whole-object zstd encoder, whose bounded fallback chose Raw for these
incompressible inputs. These fixtures do not test schema compression benefits.

Every request and response used MVM's real canonical envelope and upstream
`InMemoryPeerTransport`. Counts below are the actual encoded envelopes sent once
in each direction, including requests and publication replies; received counts
match. They exclude network framing because there was no network transport.
CPU is combined sequential host process time. Elapsed runs from transfer start
through verified durable object publication/reply, including codec selection,
hashing and mailbox cryptography. There is no separately measurable send time.

The host was ARM64 MacBookPro17,1, eight logical CPUs, 16 GiB RAM, macOS 27.0,
Rust 1.97.1 release with hardware SHA-256. No build overlapped timing. Synthetic
fixture generation, input ownership copies, expected digest calculation, final
independent readback and evidence logging are outside operation timing. Setup
and fixture generation have separate raw records. Lifecycle sums include setup,
the actual namespace probe, base, schema where applicable and all five targets;
they exclude between-operation reporting/verification/cleanup. No energy or
separate sender/receiver CPU result is inferred.

## Observed operation medians

Each row is an independent median of three observations. Times are milliseconds.
Whole means prepared whole-object zstd with its Raw fallback.

| Object/workload | Warm envelope bytes | Whole envelope bytes | Warm / Whole CPU ms | Warm / Whole elapsed ms |
|---|---:|---:|---:|---:|
| 1 MiB, first localized edit | 22,581 | 2,012,859 | 20.243 / 82.541 | 34.634 / 116.551 |
| 1 MiB, unrelated replacement | 2,012,943 | 2,012,823 | 66.398 / 71.568 | 95.162 / 136.101 |
| 16 MiB, first localized edit | 22,570 | 31,880,804 | 122.570 / 936.245 | 199.636 / 1,570.057 |
| 16 MiB, unrelated replacement | 31,879,281 | 31,880,477 | 920.994 / 941.391 | 1,492.520 / 1,527.468 |

The paired median localized-edit envelope reductions are **98.878% / 99.929%**
at 1/16 MiB. Paired CPU ratios (Warm/Whole) are 0.230939 / 0.130917, and elapsed
ratios 0.283384 / 0.119210. Append/truncate and post-replacement edit rows are
retained in the [complete summary](evidence/2026-10-31-mvm-readiness/summary.json).

**The unrelated-object negative remains:** the 1 MiB paired CPU ratio is
**1.129162**, a 12.916% penalty, despite the separately computed medians in the
table ordering differently. At 16 MiB the paired CPU ratio is 0.986445. Unrelated
transfers use the same application frame count/bytes; small envelope-byte
differences come from per-session/per-message metadata, not a compression gain.
Three trials and visible seed/time variation do not establish a general speed
claim or resolve the earlier unrelated-object optimization gate failures.

The existing localized application exchange remains **536 frame bytes**, which
is **600 bytes under the original HTTP body accounting**, for all twelve warm
localized updates. MVM envelope overhead increases that to about 22.6 KB here.
Cold/full 16 MiB objects use about 31.9 MB of canonical envelopes in both arms.
This exposes a real integration cost. The unchanged selector minimizes its HTTP
cost model, not a proven minimum under canonical MVM framing. A later transport
cost model/batching change must retain authorization and measure its benefit.

## Complete paid sessions

These sums charge all initial setup and transfers, including the extra schema
publication in Warm. Each cell is a median of three complete session sums.

| Initial object size | Warm / Whole envelope bytes | Warm / Whole CPU ms | Warm / Whole elapsed ms |
|---|---:|---:|---:|
| 1 MiB | 4,142,295 / 12,081,097 | 197.420 / 398.003 | 329.088 / 650.239 |
| 16 MiB | 63,875,025 / 191,289,482 | 2,310.288 / 5,541.874 | 3,882.398 / 9,477.987 |

Paired session envelope reductions are **65.710% / 66.608%**; CPU ratios are
0.478559 / 0.415463 and elapsed ratios 0.458565 / 0.412120. These are conditional
host measurements on this mixed sequence, not an extrapolation to WAN or to
arbitrary data. No performance acceptance threshold was registered for this
first adapter demonstration.

## Qualification and handoff

Fifteen final tests passed with no failures or ignored tests. They cover public
verification-key-only receivers, rejected recipient/epoch/revocation/tamper,
bounded frames/queues/replay, policy changes between open and dispatch, fresh
envelope retries, lost publication replies and exact reopened objects. Existing
ternary/Kuramoto/CA checkpoint, generated replay and residual transfers also
crossed canonical envelopes exactly in the tests; this campaign did not benchmark
that runtime or attribute the measured gains to it.

Default build check, all-feature Clippy and release example build passed. Source
file line coverage was 580/623 (93.10%), including module-test lines; this is not
branch, MC/DC or whole-repository coverage. Independent code/Rust/security review
resolved one evidence durability issue: verification records are now synced
before owned-store deletion, with an observed failing-then-passing sync-failure
test. Earlier failed lint checks and all test receipts are retained.

All twelve successful stores were removed only after exact readback and synced
records. The compact JSONL, identities, review receipts and results remain in
Git; no plaintext objects or signing/encryption keys are retained in evidence.

The next integration is the canonical guest mailbox service, then one guest/host,
two guests under a small orchestrator, two real hosts and three-peer recovery.
MVM [#4629](https://github.com/tinylabscom/mvm/issues/4629) and
[#4633](https://github.com/tinylabscom/mvm/issues/4633) were open at the pinned
check. No port was invented and FlowMux 5253 was not repurposed. A verified reply
handle, durable inbox/replay, live time/policy refresh, rate/admission controls,
guest memory qualification and concurrent revocation/publication semantics are
explicit remaining boundaries. Object commit is not mailbox admission or
exactly-once application execution. Linux guest execution, new WAN measurements
and FIPS/CUI qualification were not run in this milestone.

Raw records, reproducible aggregation and source/binary hashes are in the
[evidence index](evidence/2026-10-31-mvm-readiness/README.md).
