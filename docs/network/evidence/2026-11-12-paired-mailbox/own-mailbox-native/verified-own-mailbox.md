# Exact readback and ACK in one Firecracker guest

One Linux x86-64 Firecracker guest submitted a **313-byte SpiralCore frame** to its own signed production mailbox, received it on the original boot-enrolled reader, and verified **all 257 original body bytes**, including a trailing zero. The workload checked the leased message identity and the re-encoded frame, synced its new guest file and directory, reopened the file, compared its bytes and SHA-256, then sent the application ACK. The original host journal reached **cursor 3**. [Machine-readable evidence](verified-own-mailbox.json) · [Original helper meter](guest-5257.counts)

| Measured boundary | Result |
|---|---:|
| SDK launch until admitted and ready | 3.836482 s |
| SDK application Call through verified response | 2.004564 s |
| Guest workload wall / process CPU | 1.944468 s / 0.622517 ms |
| Guest staging / Receive wall | 0.043211 ms / 1.873852 s |
| Exact comparison and file commit / explicit ACK wall | 0.048751 ms / 70.455021 ms |
| SDK launch through original stop and removal | 8.002941 s |
| Timed host command, wall / user CPU / system CPU | 8.04 s / 4.54 s / 0.07 s |
| Timed host command maximum RSS | 81,272 KiB |

This was one functional run with one vCPU and 256 MiB of guest RAM. The workload starts its clocks inside the guest. Host process accounting covers the timed command and waited-for descendants. The guest's private application file lived in tmpfs. The three host journal records persisted after SDK teardown and occupied 11,934, 7,105 and 4,059 bytes in order. Its 797-byte anchor identified cursor 3. The original VM and mailbox helper exited, and the audit chain verified with eleven entries, one session and one seal.

## Bytes at each boundary

| Original port-5257 host stream exchange | Guest → host, bytes actually read | Host → guest, bytes accepted for writing |
|---|---:|---:|
| Submit and verdict | 381 | 1,169 |
| Reader challenge, possession and lease | 344 | 5,853 |
| Signed application ACK and response | 2,246 | 2,250 |
| **Sum of the three completed exchanges** | **2,971** | **9,272** |

The observer retained the original private counter file descriptor across SDK teardown. All three exchanges have closing rows with zero I/O errors and zero overflow. The file has no normal process-close footer, so it establishes the measured exchanges only. The independent Firecracker DATA-header sums on port 5257 agree per exchange in both directions. On the separate application reader Unix socket, Receive wrote 54 bytes and read 2,474. ACK wrote 1,966 bytes and read 2,250. Adding those socket totals to the host stream would count different observations of the same transfer.

The 12,243 measured host stream bytes include the signed mailbox protocol around a 257-byte input. This data point demonstrates fidelity and application acknowledgment. It provides no measured speedup, physical link byte count, crash recovery, or result for transfer between separate guests. [Addressed A→B qualification](https://github.com/R4-Laboratories/SpiralCore/issues/168)

## How the earlier refusal was resolved

In trial 5 the guest started Receive 677 ms before the original signed enrollment became valid and finished with a refusal 606 ms before validity. The corrected guest can wait up to two seconds **shared by enrollment and lease on the original request**. It checks the signed credential and scope before spending time on a future boundary, discards that preliminary result, and performs verification again at the actual guest wall time before using it. Already-valid credentials take the immediate path. Invalid signatures, revoked capabilities, mismatched identities, elapsed deadlines and expired leases continue to refuse. Three Linux controls exercised the normal delay, a clock rollback, expiry, malformed authority and insufficient budget. An initial native compilation found a missing explicit module path, which was corrected before the qualified release build. [Original negative measurements](reader-clock.md) · [Qualified guest source](https://github.com/Casey-allard/mvm/commit/161c30cf81064d4c4dcf844cea88a64a46b0bad2)

## Reproduction

Use the [original Rust witness](../../../../../tools/ari-own-mailbox-witness/README.md), synthetic fixtures and fresh private MVM home. Keep the following inputs distinct.

1. Build the host SDK driver and ordinary host services at `1755a2dea880ae5b7fcdf0479ca7f9807face651`. Build the separately metered host mailbox helper from the [meter revision](https://github.com/Casey-allard/mvm/commit/bd43ebb76b8239978151aed0a6950190141811ee) with `cargo build --locked -p mvm-hostd --features durable-guest-io-witness --bin mvm-mailbox`. Use this binary as the original `MVM_MAILBOX_PATH`.
2. Point the canonical runtime resolver and `MVM_RUNTIME_OVERLAY_SOURCE_ROOT` to guest revision `161c30cf81064d4c4dcf844cea88a64a46b0bad2`. Rebuild the complete version-matched archive, initramfs and runtime overlay. The archived guest-agent ELF SHA-256 was `905db36efd9ffdd9e9d4881f32702dff13efdd1a01f6cf7d11c6af8199deb74b`. Use the binary and image hashes recorded in the JSON to identify differences in independent builds.
3. Use the [diagnostic workload](https://github.com/R4-Laboratories/SpiralCore/commit/e6be57323de5ffa99409a142d8c5e9f841faafba). In the test image builder, change only `max_calls_per_worker: 1` to `max_calls_per_worker: 2`. The driver issues exactly one Call. This avoids synchronous worker recycling before the first response. Inject the official wrapper and verify the sealed filesystem. The workload ext4 digest matched the one used in trial 5.
4. Retain the original helper's mode-0600 `mailbox-guest-stream-v1.counts` file descriptor from the fresh VM's original state directory before the Call. Read it after the SDK stops and removes the VM. Compare each completed exchange to the corresponding DATA header lengths for port 5257. Preserve missing footers, errors and incomplete exchanges. Verify the returned guest file digest and ACK, anchor cursor, original process exit and audit chain.
