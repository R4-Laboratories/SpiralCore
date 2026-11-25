# Ledger-triggered FlowMux closure with signed audit custody

MVM development commit `2b2bd66392f649fadb0c82cbe19a2783d7cdf445`, tree
`265feae54e8636465ea8cefeba3f6570e48d7a38`, extends `df5941ba` with the
canonical epoch watcher module and a local composition test for replacement
and quarantine. The watcher module matches the frozen candidate `52ec86560`
byte for byte. Its SHA-256 is
`52a8392882af7e9d256db726cb28357c8530128f3cd3c0acdec710050a141cdb`.
[Development source](https://github.com/Casey-allard/mvm/commit/2b2bd66392f649fadb0c82cbe19a2783d7cdf445).
[Earlier normal-teardown witness](../2026-11-25-flowmux-audit-flush-closure/README.md).

Each case verifies a content-addressed signed test plan and opens an
authenticated TCP flow through FlowMux. The fixture commits the selected
transition through the coordinator's durable ledger implementation. The
watcher reports a stale epoch for replacement
and a quarantined workload for quarantine. The same plan-bound recorder
that recorded the allowed flow starts recording revocation. A test hook holds that
operation before the Ed25519 file signer appends its record.

The test driver shuts down the retained original carrier. The guest carrier
and upstream TCP socket both reach EOF within one absolute deadline starting
before the ledger commit. The audit file retains its exact prior bytes while
the append is held. After release, the verifier authenticates the entire
chain and checks exactly one new record against the original plan ID, tenant,
workload, execution epoch, and precise refusal. The session worker and the
watcher are joined before the final verification.

## Executed qualification on Wick

Rust 1.97.1 on Linux x86-64 passed 36 distinct tests across the composition,
FlowMux audit, and epoch watcher scopes. The two new cases passed 16 additional
repetitions each on the identical executable. Strict all-target `mvm-hostd`
Clippy and formatting passed. The 5 ms polling interval belongs to the fixture;
the canonical interval is one second. Each local run uses a separate one-second
observation deadline and retains the canonical unreadable-ledger grace.

| Observation over 16 repetitions per transition | Replacement min / median / max | Quarantine min / median / max |
|---|---:|---:|
| Setup through accepted flow | 2,075 / 2,143.5 / 3,796 µs | 2,073 / 2,205 / 3,085 µs |
| Transfer through exact readback | 202 / 215 / 276 µs | 190 / 214.5 / 226 µs |
| Ledger commit start through both EOFs | 5,228 / 5,245.5 / 5,381 µs | 5,229 / 5,242 / 5,318 µs |
| Elapsed through joins and chain verification | 8,101 / 8,219 / 10,208 µs | 8,095 / 8,326.5 / 9,207 µs |
| Appended signed audit record | 1,401 B in every run | 1,413 B in every run |

Every transfer verifies 13 bytes guest to upstream and 7 bytes upstream to
guest, including trailing zeros. The encrypted carrier moves 193 bytes guest
to host and 369 bytes host to guest during the measured transfer, including
the credit frame. Authentication, opening, and closure are outside those byte
counters. The final clock ends before the remaining fixture descriptors and
temporary directory are dropped.

| Linux process | Wall | User CPU | System CPU | Peak RSS |
|---|---:|---:|---:|---:|
| Two composition tests | 0.02 s | 0.00 s | 0.00 s | 52,224 KiB |
| Thirteen FlowMux audit tests | 0.44 s | 0.01 s | 0.00 s | 44,164 KiB |
| Twenty-one epoch watcher tests | 4.17 s | 0.01 s | 0.00 s | 48,720 KiB |
| Test build | 42.54 s | 27.36 s | 3.00 s | 2,671,360 KiB |
| Strict all-target Clippy | 19.73 s | 24.73 s | 2.41 s | 1,755,040 KiB |

The process clocks report CPU to hundredths of a second. Per-endpoint transfer
CPU and installed or WAN measurements remain pending.

The verified 4,878-file manifest has SHA-256
`70ad8cb75fd61784fe11a63ef759b4a9989f51b6d8d2e3657efc371e4d9f6e30`.
The executable has SHA-256 `157ba03c6bd95623bff9c9612a7bc228452abc86f19b3bde8aa93ca666d0962a`.
The verified 34,969,174-byte archive has SHA-256
`0184b605f61696c6aa63187434c2c8cd7b912467c7a23b72d080469690671a59`.
The tested executable, source manifest and run records are retained on Wick's HDD.
Each generated audit chain is verified inside its run's temporary directory.

## Remaining executable and installed gates

The fixture driver owns the stop call while the production callback waits for
its plan-bound append. The executable still needs an independent transport
stop path and a canonical owner for the pending signed record. The decision
on that custody, the installed audit interface, and the compatible
publisher-signed runtime remain pending. Original readiness, both #258 gates,
and the full delivery goal remain open. [Revocation ownership discussion](https://github.com/tinylabscom/mvm/issues/5848#issuecomment-7171352474).
[Installed audit interface](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6935221842).
[#258](https://github.com/R4-Laboratories/SpiralCore/issues/258).
[#42](https://github.com/R4-Laboratories/SpiralCore/issues/42).
