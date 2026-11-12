# Installed object transfer between two Firecracker guests

Two original MVMs on one x86_64 Linux host moved a 4,096-byte object through their signed Call, Reply and Receive grants. Both used the canonical guest mailbox on port 5257. The two host helpers exchanged sealed envelopes and signed receipts on their original connected Unix streams. Neither guest had a NIC. B read back the complete fixture twice, and the driver compared all 4,096 bytes each time. Both cold and warm requests reached the same B boot and worker instance. [Exact result and source identities](first.json) · [Sealed role images](../object-role-images.md)

| One observed transfer | Cold | Warm repeat |
| --- | ---: | ---: |
| Calls / literal bytes | 4 / 4,096 | 1 / 0 |
| Object request / reply bytes | 4,376 / 282 | 102 / 88 |
| Sealed A→B request / returned receipt bytes | 32,393 / 4,790 | 6,326 / 1,192 |
| Sealed B→A reply / returned receipt bytes | 21,800 / 4,814 | 5,494 / 1,211 |
| Unique sealed bytes across both private channels | 63,797 | 14,223 |
| A / B workload CPU | 8.328 / 30.250 ms | 2.281 / 8.593 ms |
| Coordinator process CPU | 29.947 ms | 11.305 ms |
| Elapsed through both guest results | 8.002 s | 2.200 s |

The warm repeat saved 4,468 Object-v1 application bytes (95.9%) and 49,574 sealed bytes (77.7%). Each sealed count was checked against the opposite original helper's accepted byte count. The per-phase totals subtract successive complete snapshots and count each physical byte once. Elapsed time includes the original MVM application Calls, signed receipts, verified replies and both guest results. These single observations do not establish sustained capacity or WAN performance.

**The complete guest-port meter failed.** Both 5257 observers exhausted their 64-exchange limit during the cold round and recorded overflow. The full guest-port totals for both rounds are unknown. Both original workers retired on their first SDK stop with unassisted exit code zero. The original audit verified 17 entries, two sessions and two seals. The native driver exited 1 after recording the independent data and teardown outcomes. [Acceptance tracker](https://github.com/R4-Laboratories/SpiralCore/issues/202)

On the same host, Zstandard 1.5.7 at levels 1 and 3 emitted 4,106 bytes for the same fixture, which was verified after every independent decode. For 8,192 encode/decode iterations the level-1 loop took 16.134 ms and the level-3 loop took 21.642 ms. This codec-only measurement excludes the MVM application Calls, envelopes, receipts and network, so it cannot establish an end-to-end speed comparison. [Reference measurement source](zstd_reference.rs)

To reproduce the installed pilot, build the witness with `--features installed-object-pair` and the original MVM workers with `durable-guest-io-witness`. In a fresh qualification home with the selected guest runtime, retention ceiling and native backend, run `paired-journal installed-object-repeat A.ext4 A_SHA256 B.ext4 B_SHA256`. The driver verifies each image digest, creates two persistent 1-vCPU/256-MiB definitions, reserves their original workers, waits for both native SDK starts, runs the object transfer and retires both original owners. [Driver revision](https://github.com/R4-Laboratories/SpiralCore/commit/424c36ba5c47dab768ae0e4ebfa98d1af6fb6620)
