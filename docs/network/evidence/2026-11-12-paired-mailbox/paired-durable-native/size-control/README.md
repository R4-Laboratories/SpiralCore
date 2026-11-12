# 4 KiB and 257-byte transfers through the addressed MVM mailbox

Two fresh Linux x86-64 Firecracker guests stayed up for three 4,096-byte transfers, each with a distinct original message ID. The same driver then ran its original 257-byte profile on a second fresh pair. For all six messages, B's workload synced and reopened the received body, verified its bytes before acknowledging the lease, and returned the body for an independent byte comparison. The host verified B's signed local-durable acceptance receipt for each original envelope. Each pair ran one vCPU and 256 MiB per guest on a single host with no guest NIC. [4 KiB operation records](large.json) · [257-byte operation records](legacy.json)

| Executed interval or count | Original 257-byte body | 4,096-byte body |
|---|---:|---:|
| Application bytes verified across 3 operations | 771 | 12,288 |
| SpiralCore frame per operation | 313 B | 4,152 B |
| Sealed relay traffic toward B across 3 operations | 17,574 B | 37,972 B |
| Signed relay replies toward A across 3 operations | 3,610 B | 3,608 B |
| Median from A workload Call start through B's verified receipt | 221.052 ms | 226.014 ms |
| Median through B's ACK, returned readback and host comparison | 1,259.226 ms | 1,192.366 ms |
| Median interleaved control Call wall time | 81.266 ms | 81.039 ms |
| A data workload CPU across 3 operations | 0.539 ms | 0.548 ms |
| B data workload CPU across 3 operations | 1.472 ms | 1.601 ms |
| A control workload CPU across 3 probes | 0.150 ms | 0.173 ms |
| Pair setup / three-operation series wall time | 4.621 s / 3.755 s | 4.632 s / 3.593 s |
| Whole driver process CPU, including setup and teardown | 5.132 s | 5.182 s |
| GNU time for the full command, elapsed / user / system | 8.74 / 5.40 / 0.10 s | 8.59 / 5.48 / 0.11 s |
| GNU command maximum RSS | 83,324 KiB | 75,052 KiB |

**The measured relay used more bytes than the application body in both profiles.** Its counts include the current canonical Send credential, envelope framing and ciphertext, and the signed return receipt. The test used the ordinary SpiralCore object frame, the MVM authorization and local-durable mailbox service. Compression selection, warm-cache savings, a matched zstd transport, WAN goodput and physical energy were outside this run. Three sequential operations per size do not support a speedup or a tail-latency estimate.

**Control timing.** After each A workload staged its immutable send, the driver submitted a small, separately admitted direction-availability query to A. It waited for the original A agent's reply before forwarding the data to B. Thus each of six probes completed while an original transfer was pending. The schedule measures interleaving at the staging boundary. It leaves scheduling during a saturated payload stream open.

| Closed helper stream totals across 3 operations | 257 B, guest → host / host → guest | 4 KiB, guest → host / host → guest |
|---|---:|---:|
| A mailbox, port 5257 | 1,143 / 3,610 B | 12,660 / 3,608 B |
| B mailbox, port 5257 | 7,895 / 24,654 B | 7,835 / 39,898 B |

Guest-to-host counts are bytes read; host-to-guest counts are bytes accepted by the original helper's stream write. These streams, SDK Calls and the host relay occupy distinct measurement boundaries. Application confirmation came from B's signed lease ACK and the returned readback. Both pairs had closed meter records, both original helpers exited with code zero on the first stop without assistance, and no VMM, helper or run scope remained. Each audit verified 17 entries, two sessions and two seals. [4 KiB A stream](large-original-sc-journal-a.counts) · [4 KiB B stream](large-original-sc-journal-b.counts) · [257 B A stream](legacy-original-sc-journal-a.counts) · [257 B B stream](legacy-original-sc-journal-b.counts)

## Limits and reproduction

The pinned MVM contract allows up to 1 MiB of mailbox plaintext. Its original installed template grants at most 16 KiB; the host bounds a persisted sealed source frame at 64 KiB. The reader wire bounds encoded requests at 16 KiB and responses at 64 KiB. The canonical worker pipe has a 256 KiB cap and the SDK's one-shot Call accepts 63,488 input bytes. These limits apply to different representations. At 4 KiB, this fixture's hex-encoded SDK request was 8,478 bytes and its B response, including readback, was 8,471 bytes. A's guest submit consumed 4,220 measured bytes per transfer on port 5257. The A source grant, B incoming template and Send grant, and A signed outbound direction capped the 4 KiB trial at the 4,152-byte frame. The two guest workloads checked the exact fixture hash. [Host template](https://github.com/Casey-allard/mvm/blob/8238000662f59893b6ec703ab4b37d7a86c08340/crates/mvm-hostd/src/mailbox_launch.rs#L447) · [Reader wire](https://github.com/Casey-allard/mvm/blob/8238000662f59893b6ec703ab4b37d7a86c08340/crates/mvm-core/src/mailbox/reader_wire.rs) · [SDK frame limits](https://github.com/Casey-allard/mvm/blob/8238000662f59893b6ec703ab4b37d7a86c08340/crates/mvm-agentd/src/vsock/mod.rs#L204)

The driver and workloads were built from SpiralCore `cfeb388f`; both profiles used MVM host `82380006` and guest runtime `93b3b65a`. Each profile passed ten host harness unit tests, including bounded pipe backpressure and excess output. The large guest workload passed three component tests and the default workload passed two. A first preparatory invocation refused an already-present retention configuration before images were sealed or any VM started. The canonical preparer created the configuration in the fresh home and it matched the required operator configuration byte for byte before the measured launch. [Build and run instructions](../../../../../../tools/ari-own-mailbox-witness/paired-native.md) · [Frozen driver](https://github.com/R4-Laboratories/SpiralCore/commit/cfeb388fdb30f57d0ee1cf39d5de243fb03ca4ea) · [Capacity tracker](https://github.com/R4-Laboratories/SpiralCore/issues/49)
