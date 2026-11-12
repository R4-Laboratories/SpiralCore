# Lost admission receipt for a 4 KiB object between two Firecracker guests

One Linux x86-64 host ran two separately booted Firecracker guests. A staged one 4,152-byte SpiralCore frame. The test relay read B’s genuine signed V2 admission receipt and withheld the first copy. A’s original guest agent retried. B returned the same serialized receipt for the identical sealed envelope. B’s workload synced and reopened the object, acknowledged its lease and performed a second authenticated read that found its queue empty. The driver compared B’s returned bytes with the original 4,096-byte fixture, including its trailing zero. A’s outbox retired without a rejected or uncertain archive. The host verified the V2 receipt. Delivery of a warm-cache application Reply to A remains open in [#202](https://github.com/R4-Laboratories/SpiralCore/issues/202).

| Measured relay operation | First attempt | Retry |
|---|---:|---:|
| Sealed request toward B | 12,685 B | 12,685 B |
| Signed receipt read from B | 1,219 B | 1,219 B |
| Signed receipt written to A’s helper | 0 B | 1,219 B |
| From A’s first Call to verified receipt at the host | 133.386 ms | 173.946 ms |

The retry sent the full sealed request again. Across both attempts B received 25,370 B and returned 2,438 B on the destination-facing relay connection. A received one 1,219-byte receipt on the source-facing connection. The separate port-5257 meters counted 8,440 B read and 1,238 B written on A, and 2,986 B read and 16,894 B written on B. Those streams also carried the reader, ACK or settlement exchanges. Adding the relay and guest counters would double-count parts of delivery. Both meters have a complete footer and a final zero-byte exchange with a write error at teardown. [A meter](a.counts) · [B meter](b.counts) · [Structured result](result.json).

| Elapsed time or process cost | One observed run |
|---|---:|
| SDK setup through both admissions | 5.646 s |
| A’s first Call through host-verified readback, ACK and empty probe | 774.847 ms |
| A workload wall / process CPU | 157.794 ms / 0.647 ms |
| B workload wall / process CPU | 492.232 ms / 0.603 ms |
| Orchestrator wall / process CPU including setup and retirement | 6.721 s / 6.112 s |
| Outer command wall / user CPU / system CPU / peak RSS | 6.79 s / 6.19 s / 0.09 s / 77,784 KiB |

Both original helpers exited with code zero on their first stop without assistance. The audit chain verified 17 entries, two sessions and two seals. No Firecracker process, mailbox helper or active MVM scope remained. This single-host reliability trial measured neither WAN traffic nor a compression advantage over zstd. The workload and orchestrator CPU figures cover distinct processes. [Clean 4 KiB transfers with control traffic](../size-control/README.md).

## Reproduce the trial

Use the [paired operator guide](../../../../../../tools/ari-own-mailbox-witness/paired-native.md) and the [tested SpiralCore driver and workloads](https://github.com/R4-Laboratories/SpiralCore/tree/cfeb388fdb30f57d0ee1cf39d5de243fb03ca4ea/tools/ari-own-mailbox-witness). Apply the [pinned test overlay](driver-only.patch) to the [MVM host source](https://github.com/Casey-allard/mvm/tree/8238000662f59893b6ec703ab4b37d7a86c08340) solely when building the test orchestrator. The five installed MVM service and CLI binaries match the clean 4 KiB run byte-for-byte. Use the [guest runtime source](https://github.com/Casey-allard/mvm/tree/93b3b65ae72a4746a54a21c7c158d5453cceb8f5).

1. Build the host driver and both guest workloads with both features enabled, `receipt-loss-qualification,large-object-fixture`. The host driver tests passed 10/10 and guest library tests passed 3/3.
2. Seal distinct `journal-a` and `journal-b` images. Each has one worker, at most two calls per worker, one vCPU and 256 MiB of guest memory. Use a fresh private MVM home and the original 30-second retention ceiling.
3. Run `paired-journal --backend firecracker A_EXT4 A_SHA256 B_EXT4 B_SHA256 fixtures/workload-4k-frame.bin` once. Check both meter footers, byte equality, independent empty read, original owner exit records and audit verification. The [digest list](artifact-sha256.txt) identifies the executable and fixture inputs, original result and counters. The published JSON omits plan and message identifiers. The driver removes the readback bytes after comparing them.

The first setup attempt stopped before guest boot because copying a prepared runtime cache under a private umask changed an SDK file mode from 0644 to 0600. The runtime verifier rejected that cache. Neither guest started and no transfer occurred. The successful run used a fresh private home and copied the verified runtime cache with its recorded modes preserved.
