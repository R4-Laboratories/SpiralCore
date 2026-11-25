# Watcher-owned FlowMux shutdown with signed audit custody

MVM development commit `9bc1447b99df847321b5914cbc5a23117a190b3a`, tree
`031ea75a5cd9f565b001742a9daf1b1856889851`, changes the local composition
fixture from `2b2bd663`. The canonical watcher delivers the refusal to a
fixture callback that owns the original carrier handle. [Development source](https://github.com/Casey-allard/mvm/commit/9bc1447b99df847321b5914cbc5a23117a190b3a).
[Prior ledger composition](../2026-11-25-ledger-flow-closure/README.md).

Each case admits an authenticated FlowMux TCP flow and holds an earlier
append on the same plan-bound recorder before committing the ledger change.
After replacement or quarantine, the watcher callback shuts down the
retained carrier before entering revocation recording. The test driver
observes both carrier and upstream TCP EOFs while the earlier append is
still held and the audit file retains its original bytes. One absolute
one-second deadline begins before the durable ledger commit.

The test first releases the earlier append and keeps the subsequent
revocation append held. It verifies the earlier record and original chain
history before releasing the revocation. After joining the watcher and
session worker, it verifies one new Ed25519 chain record for the precise
refusal, bound to the original plan, tenant, workload and epoch. Both stages
preserve the exact previous file prefix.

## Executed qualification on Wick

Rust 1.97.1 on Linux x86-64 passed 36 distinct tests across the composition,
FlowMux audit and epoch watcher scopes. Both transition cases passed 16
additional repetitions on the same executable. Formatting and strict
all-target `mvm-hostd` Clippy passed. The fixture polls every 5 ms and uses
the canonical unreadable-ledger grace. The production polling interval is
one second.

| Observation over 16 repetitions per transition | Replacement min / median / max | Quarantine min / median / max |
|---|---:|---:|
| Setup through accepted flow | 2,074 / 2,167 / 2,708 µs | 2,082 / 2,296 / 3,459 µs |
| Bidirectional transfer through exact readback | 211 / 215.5 / 290 µs | 209 / 219 / 630 µs |
| Ledger commit start through both EOFs | 5,120 / 5,125.5 / 5,212 µs | 5,112 / 5,130 / 5,172 µs |
| Elapsed through joins and chain verification | 8,363 / 8,463 / 9,242 µs | 8,347 / 8,714.5 / 10,078 µs |
| Signed bytes in the earlier append | 1,149 B per run | 1,149 B per run |
| Signed bytes in the revocation append | 1,401 B per run | 1,413 B per run |

Each run verifies 13 payload bytes guest to upstream and 7 bytes in reverse,
including trailing zeros. The measured encrypted carrier interval uses
193 bytes guest to host and 369 bytes host to guest, including the credit
frame. The carrier counters cover transfer. The final clock runs through
joins and chain verification, before local fixture descriptors and the
temporary directory are dropped.

| Linux process | Wall | User CPU | System CPU | Peak RSS |
|---|---:|---:|---:|---:|
| Two composition tests | 0.02 s | 0.00 s | 0.00 s | 51,440 KiB |
| Thirteen FlowMux audit tests | 0.44 s | 0.02 s | 0.00 s | 45,084 KiB |
| Twenty-one epoch watcher tests | 4.17 s | 0.01 s | 0.00 s | 49,160 KiB |
| Test build | 22.29 s | 27.14 s | 2.00 s | 2,656,672 KiB |
| Strict all-target Clippy | 14.80 s | 14.25 s | 0.55 s | 1,756,680 KiB |

CPU time is reported to hundredths of a second. Per-endpoint transfer CPU,
installed readiness and WAN measurements remain pending.

The 4,878-file source manifest hashes to
`f4ddb93fbfc48a78d65b7978334efb7750b3d46223f042f888c623a6f73c897e`.
The tested executable hashes to
`c232b642702eb92625efbe4f6a62f951c00855a44675af028d44285d2c9ee869`.
The verified 34,961,111-byte archive hashes to
`167f5d210d1f0e6ba61fe99f21f1092107bad78f172ef277fc62e66b2dada508`.
The executable, source manifest and raw measurements are retained on Wick.

## Executable and installed qualification

The fixture qualifies stop ordering for one authenticated carrier and TCP
relay. The executable callback still waits on audit before process exit.
Production qualification requires an independent stop for all serving
transports and a canonical owner for the pending signed record. Ari's
revocation and installed audit replies, the compatible publisher-signed
guest runtime, the original readiness test and both #258 gates remain
pending. [Revocation discussion](https://github.com/tinylabscom/mvm/issues/5848#issuecomment-7171352474).
[Installed audit interface](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6935221842).
[#258](https://github.com/R4-Laboratories/SpiralCore/issues/258).
[#42](https://github.com/R4-Laboratories/SpiralCore/issues/42).
