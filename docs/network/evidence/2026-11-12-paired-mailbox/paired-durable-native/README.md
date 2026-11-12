# Native delivery between two Firecracker guests

A later run of the same payload and guest images verified unassisted shutdown of both original helpers. This page retains the first pilot. [Graceful rerun and comparison](graceful.md).

On 2026-11-12, two separately admitted Linux x86-64 Firecracker guests exchanged one synthetic object through MVM’s trusted-host mailbox service. Each guest had one vCPU and 256 MiB of RAM. Only guest A received the original 313-byte SpiralCore frame. Guest B reopened a file containing the reconstructed 257 bytes, including the trailing zero, acknowledged its lease and returned its readback for an independent byte-for-byte comparison. **Delivery passed. Graceful helper shutdown failed for both guests.**

| Measurement | Single observed run |
|---|---:|
| Two-guest setup before sending | 4.678693 s |
| First A Call through host verification of B’s signed durable receipt | 117.819 ms |
| Same start through B’s application ACK and host readback verification | 1.103199 s |
| Original A helper → original B helper, envelope and Send credential | 5,810 bytes |
| Original B helper → original A helper, correlated signed V2 response | 1,195 bytes |
| A workload wall / process CPU | 0.614303 / 0.211465 ms |
| B workload wall / process CPU | 935.310775 / 0.601859 ms |
| Orchestrator process CPU / elapsed, including setup and cleanup | 5.169023 / 6.567857 s |
| Outer invocation wall, user CPU, system CPU and maximum RSS | 6.63 s, 5.20 s, 0.13 s, 81,936 KiB |

The 7,005 relay bytes include the cost of signed, addressed delivery of this short frame. The workload CPU intervals cover the code operating the local guest mailbox. No separate helper CPU, send-only time, physical network bytes, WAN link or matched zstd sample was measured. The debug host run establishes functional correctness. Its timing does not establish throughput or a compression advantage. CPU scopes overlap.

## Receipt and application boundaries

1. B booted with its signed mailbox plan and an independently enrolled Receive capability. The pair bound A’s own Send scope, the B-addressed Send capability and B’s actual Receive credential. The operator retention ceiling was 30 seconds.
2. A staged the original frame and returned its message ID. The host authenticated the V2 response for B’s full address and epoch, its lifetime, the original ciphertext commitment, delivery ID and staged message ID. The receipt outcome was `AcceptedLocalDurable`. Direct observation of that receipt inside A’s workload was unmeasured.
3. B’s application received only the correlated ID through its SDK Call. It used its own guest reader to lease the frame, decoded it, synced and reopened its guest file, checked that readback and then ACKed the lease. The host compared the returned 257 bytes with A’s original fixture. The SDK audit chain verified 17 entries, two sessions and two seals.

## Socket measurements and cleanup

| Post-mux original port-5257 helper stream | Guest → host | Host → guest |
|---|---:|---:|
| A submission | 381 | 1,195 |
| B lease, including challenge and possession | 358 | 5,921 |
| B application ACK | 2,264 | 2,268 |

Each helper recorded a subsequent zero-byte turn with a write error. Neither recorder produced its normal session footer. The table reports observed prefixes only. The separate relay, guest-reader and SDK control measurements apply to different sockets. The control channel includes B’s returned readback. [A recorder](source-helper.counts) · [B recorder](destination-helper.counts) · [Machine-readable result and identities](result.json).

Both original VMM stops were confirmed. Both original mailbox helpers remained alive past the grace interval and the SDK terminated and collected each original child. The harness exited 1. Both records report first-attempt assistance, unsuccessful process status and no numeric exit code. There were zero remaining original Firecracker or mailbox processes. The helper’s loop waited while the VM state directory existed and either its launcher or VM remained alive. Its owning launcher had to remain alive to collect it, and the SDK retained the VM state until collection. The next gate is an explicit original-helper shutdown followed by unaided collection and a complete recorder footer. [SDK and helper revision](https://github.com/Casey-allard/mvm/tree/7a1aa5bc5f92e7530e3939d0b3917ab9491cd207) · [Guest runtime revision](https://github.com/Casey-allard/mvm/tree/93b3b65ae72a4746a54a21c7c158d5453cceb8f5) · [Orchestrator and reproduction](../../../../../tools/ari-own-mailbox-witness/paired-native.md) · [Acceptance tracker](https://github.com/R4-Laboratories/SpiralCore/issues/168).
