# Linux Firecracker on the Darwin-corrected mailbox revision

Two original Linux x86-64 Firecracker guests passed clean delivery and recovery from a deliberately lost first signed receipt on the same MVM host revision as [the Apple Silicon/HVF runs](arm64-hvf.md). The driver observed a Firecracker marker for each guest in both trials. Each had one vCPU, 256 MiB and no guest NIC. The original helpers and guest agent used MVM’s canonical mailbox services. [Machine-readable results, hashes and full meter records](linux-823.json).

In each fresh installation, A staged one 313-byte SpiralCore frame. B used its own enrolled reader, recovered all 257 payload bytes including the trailing zero, synced and reopened its file, and acknowledged the original lease. The host independently compared B’s returned bytes with A’s input and verified the signed destination receipt. In the recovery trial the orchestrator withheld the first receipt from A. The automatic retry preserved the envelope and returned B’s byte-identical signed receipt. A’s workload observed the directed pending slot retire without a rejected or uncertain archive. B’s additional authenticated read, before the retention deadline, returned empty.

| One native trial per column | Clean delivery | First receipt withheld |
|---|---:|---:|
| Guest admission setup | 4.711756 s | 4.696212 s |
| A’s first Call to B’s first verified durable receipt | 105.627 ms | 121.335 ms |
| Same start to verification of B’s replayed receipt | — | 161.334 ms |
| Same start through B’s ACK and verified readback | 0.440133 s | 1.361047 s |
| Sealed envelope and Send credential forwarded toward B | 5,808 B | 5,836 B twice |
| Signed receipt read from B | 1,188 B | 1,193 B twice |
| Signed receipt written to A’s original helper | 1,188 B | 0 B, then 1,193 B |
| A workload wall / process CPU | 0.606 ms / 0.202 ms | 148.058 ms / 0.689 ms |
| B workload wall / process CPU | 282.525 ms / 0.615 ms | 1,140.773 ms / 0.510 ms |
| Orchestrator elapsed / process CPU (setup through retirement) | 5.440258 s / 5.114537 s | 6.327957 s / 5.165027 s |
| Outer invocation wall / user / system | 5.51 / 5.17 / 0.09 s | 6.38 / 5.21 / 0.10 s |
| Peak RSS, outer invocation | 83,320 KiB | 80,668 KiB |
| A port 5257 closed stream, guest → host / host → guest | 381 / 1,188 B | 762 / 1,212 B |
| B port 5257 closed stream, guest → host / host → guest | 2,585 / 8,140 B | 2,976 / 11,715 B |

The receipt timing starts at A’s first SDK Call and ends at the host’s verification. The application readback interval also includes the confirmation returned over the SDK channel; the recovery timing additionally includes B’s authenticated empty read. Each original helper exited with code 0 on the first stop attempt without assistance. All four per-role port-5257 sessions recorded a final footer. Both audit chains verified 17 entries, two sessions and two seals. No Firecracker or mailbox helper process remained after either trial. Application CPU, orchestrator CPU, guest-reader I/O, relay bytes and vsock bytes describe different scopes and cannot be added to infer a wire rate. These single-host tests provide no WAN, zstd comparison, isolated send-only time or per-helper CPU measurement.

## Reproduction

Use [the paired operator guide](../../../../../tools/ari-own-mailbox-witness/paired-native.md) with host driver [f096d110](https://github.com/R4-Laboratories/SpiralCore/tree/f096d110e5daa2045cf9ad645eeaa67139e47ee9), MVM host [82380006](https://github.com/Casey-allard/mvm/tree/8238000662f59893b6ec703ab4b37d7a86c08340) and the verified guest runtime [93b3b65a](https://github.com/Casey-allard/mvm/tree/93b3b65ae72a4746a54a21c7c158d5453cceb8f5). Use separate private homes, the original 30-second retention ceiling and `paired-journal --backend firecracker`. The clean trial reused the sealed x86-64 role images from [the initial clean-shutdown result](graceful.md). The recovery trial reused the distinct workloads from [the original retry result](receipt-loss.md). Production helper binaries were identical across trials. The rebuilt recovery orchestrator alone used the [one-receipt loss overlay](../../../../../tools/ari-own-mailbox-witness/receipt-loss/driver-only.patch). Keep Linux arm64 native, Windows client, WAN and capacity qualification separate. [Platform tracker](https://github.com/R4-Laboratories/SpiralCore/issues/46).
