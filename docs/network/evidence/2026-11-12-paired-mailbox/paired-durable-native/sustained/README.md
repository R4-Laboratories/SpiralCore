# Eight durable operations between one persistent Firecracker pair

Two original Linux x86-64 guests remained up for eight addressed operations. Each carried the same 257-byte body (313-byte SpiralCore frame) under a new stable message ID. For every operation the host verified B's signed V2 receipt against the original sealed message, B synced and reopened its application file and acknowledged the message, and the host compared all returned bytes. B removed the verified scratch file after its ACK. The original helpers exited with code 0 on their first stop attempts. Both VMMs stopped. The audit verified 17 entries, two sessions and two seals. [Per-operation measurements, identities and hashes](measurements.json)

For each operation A staged the original message. While that operation awaited forwarding, another SDK Call asked A's guest agent to check the original outbound direction, recipient and admitted size. The relay forwarded the message after the control returned. All eight controls completed inside that interval. The measured control time includes the SDK Call. This schedule exercises progress while data is pending. Concurrent large transfers, priority scheduling and saturation require additional runs.

| Host-observed interval from A Call start | Median | Longest |
|---|---:|---:|
| Through independently verified B receipt | 220.763 ms | 229.981 ms |
| Through B's ACK, returned readback and independent comparison | 883.826 ms | 908.223 ms |
| Control Call only, from launch through reply | 83.785 ms | 86.688 ms |

The nearest-rank p95 equals the maximum in this eight-operation sample. The series took 7.0704 s after 5.4470 s of setup. Over all eight operations the relay carried **46,819 bytes toward B and 9,590 back**, for 2,056 original application bytes. The two closed port-5257 sessions recorded 3,048 guest-to-helper and 9,590 helper-to-guest bytes at A, and 21,052 and 65,805 respectively at B. These measurements describe different sockets. [A stream](original-sc-journal-a.counts) · [B stream](original-sc-journal-b.counts)

Across eight data operations the A workload consumed 1.186 ms CPU, B 3.622 ms CPU and the eight controls 0.402 ms CPU. The orchestrator process used 6.077 s CPU across setup, transfer and teardown. GNU time reported 12.89 s elapsed, 6.80 s user, 0.17 s system and 77,084 KiB maximum RSS for its command. Service-wide peak memory and VMM CPU require separate readings. No WAN, fleet, throughput advantage or zstd comparison was measured.

## Reproduction and capacity

Use the paired durable-native preparation with original host source 82380006 and original guest source 93b3b65a. Build the host driver with the paired-journal-qualification feature and the two guest workload executables for Linux musl. Seal the workloads with the series-a and series-b image modes. Verify both roles use one worker, with 16 A Calls and eight B Calls allowed before recycling. Use the original signed provisioning and 30 s retention authority, one vCPU and 256 MiB per guest, no guest NIC. Invoke the existing paired driver as follows. [Paired native instructions](../../../../../../tools/ari-own-mailbox-witness/paired-native.md) · [Tested driver source](https://github.com/R4-Laboratories/SpiralCore/commit/cd85d075d996173d3fc195b3083a066cc5244004) · [Original host](https://github.com/Casey-allard/mvm/commit/8238000662f59893b6ec703ab4b37d7a86c08340) · [Original guest](https://github.com/Casey-allard/mvm/commit/93b3b65ae72a4746a54a21c7c158d5453cceb8f5)

```sh
paired-journal --backend firecracker --objects 8 \
  "$source_image" "$source_digest" "$destination_image" "$destination_digest" \
  "$workload_frame"
```

The first preflight refused the group-writable cache directory before starting a VM. A subsequent fresh installation lacked the required operator retention ceiling and refused before launch. A fresh private cache directory and the original operator configuration passed the admission checks. Neither preflight moved payload bytes. Eight host component checks passed on the source used for the native run.

The runner currently accepts 2–8 operations. The original source sealer bounds retained intents at 64 and uses their count to recover its sequence. The destination journal bounds its cursor at 10,000. Preserving exact retry and sequence ownership beyond those limits remains open. The older 2,048-Call result belongs to a different profile. Expiry, multi-tenant fairness, large payloads alongside control and more than two native endpoints remain separate capacity gates. [Scale tracker](https://github.com/R4-Laboratories/SpiralCore/issues/49)
