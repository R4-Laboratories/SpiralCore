# Recovering a lost destination receipt between two Firecracker guests

Two Linux x86-64 guests on the same host completed one addressed transfer after the relay withheld B’s first signed admission receipt. A staged the 313-byte SpiralCore frame once. The same original guest agent retried its pending operation. B returned the same complete envelope commitment, delivery and message identity, and its signed receipt bytes compared equal across both turns. B’s workload read, synced and reopened all 257 payload bytes, including the final zero. It acknowledged the lease once and returned the readback for independent byte-for-byte comparison by the host. One additional authenticated read from B returned empty within the original 30-second retention window.

| Original relay I/O | First turn | Automatic retry |
|---|---:|---:|
| Sealed frame and Send credential written toward B | 5,814 B | 5,814 B |
| Signed receipt read back from B | 1,203 B | 1,203 B |
| Signed receipt written to A’s original helper | 0 B | 1,203 B |
| Elapsed from A’s first Call through host receipt verification | 116.159314 ms | 156.536676 ms |

The discarded reply was injected in a separately built test orchestrator. After reading the full first receipt from B, it returned from the original socket handler without writing receipt bytes to A’s helper. On retry, the orchestrator verified the canonical grant and compared the complete decoded envelope and the original serialized receipt. It wrote the full second receipt to A’s helper. The source guest observed its single outbox slot disappear and checked that neither its rejected nor its uncertain directed archive existed. This is evidence of agent-side settlement. The A application did not receive the signed receipt directly. [Test-only relay overlay](../../../../../tools/ari-own-mailbox-witness/receipt-loss/driver-only.patch) · [Rust qualification driver and workloads](https://github.com/R4-Laboratories/SpiralCore/tree/06df80d07c27f94115881bfa2f8d74bb5d542978/tools/ari-own-mailbox-witness).

| Timing or resource measurement | Single run |
|---|---:|
| SDK setup through both guest admissions | 4.987216 s |
| A’s first Call through verified B readback, ACK and empty probe | 1.159652 s |
| A workload wall / process CPU | 143.207324 ms / 0.603556 ms |
| B workload wall / process CPU | 917.186606 ms / 0.607280 ms |
| Orchestrator process CPU / wall, including setup and clean retirement | 5.476909 s / 6.424491 s |
| Outer timed command wall / user CPU / system CPU / peak RSS | 6.49 s / 5.54 s / 0.08 s / 82,820 KiB |

Both original port-5257 meters closed normally. A’s helper read **762 B** and accepted **1,222 B** for writing. Its first guest exchange returned a 19-byte response after the lost signed receipt. B’s helper read **2,990 B** and accepted **11,724 B** for writing across the lease, ACK, empty probe and final idle turn. The relay, guest-reader, SDK Call and post-mux scopes overlap. The empty probe adds a signed reader exchange to the measured workload. These counters describe a same-host reliability trial. WAN speed, maximum throughput, savings against zstd, separate send time and energy remain unmeasured. [A meter](receipt-loss-a.counts) · [B meter](receipt-loss-b.counts) · [Timing, counts and artifact hashes](receipt-loss.json).

Both VMMs stopped and both original helpers exited with code 0 on the first stop attempt without assistance. The audit chain verified 17 entries, two sessions and two seals. No original Firecracker or mailbox process remained. The two-guest result qualifies one retained operation, one deliberate loss and one retry. It does not establish exactly-once application execution across crashes. [MVM host and unmodified helpers](https://github.com/Casey-allard/mvm/tree/27038e3f322e4336df180073e12cae59cc4dba54) · [Guest runtime](https://github.com/Casey-allard/mvm/tree/93b3b65ae72a4746a54a21c7c158d5453cceb8f5) · [Reproduction](../../../../../tools/ari-own-mailbox-witness/paired-native.md#qualifying-a-lost-first-receipt) · [Issue #168](https://github.com/R4-Laboratories/SpiralCore/issues/168).
