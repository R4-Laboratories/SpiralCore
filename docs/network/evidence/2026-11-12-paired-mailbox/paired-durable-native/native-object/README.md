# Installed object transfer between two Firecracker guests

The matched Raw and whole-object zstd controls used two fresh installations with the same role images. Both returned all 4,096 original bytes. Fixed zstd carried 4,110 bytes of encoded data. [Matched controls, readbacks, CPU and six-stream measurements](matched-codecs.md)

### Complete measurement on a fresh pair

The second Linux trial equipped each original guest link with a bounded 512-exchange observer. Both original Firecracker guests returned the same 4,096-byte fixture on a four-Call cold transfer and a one-Call repeat. The receiver boot and worker instance matched across the two operations. All six independent observers produced closed final records, and the native qualification exited 0. [Exact per-direction counters, whole sessions, binary identities and timing](closed-streams.json) · [Qualified MVM host revision](https://github.com/Casey-allard/mvm/commit/30ca64feba564f8a3a3f62ee96c746c51176567c)

| Measured boundary | Cold | Warm |
|---|---:|---:|
| Object requests / replies | 4,376 / 282 B | 102 / 88 B |
| A port 5257, host read / write | 36,703 / 168,608 B | 8,487 / 46,467 B |
| B port 5257, host read / write | 73,370 / 203,308 B | 20,362 / 49,497 B |
| Both guest-port links, both directions | 481,989 B | 124,813 B |
| Sealed A→B request / receipt back | 32,293 / 4,822 B | 6,304 / 1,200 B |
| Sealed B→A reply / receipt back | 21,688 / 4,807 B | 5,463 / 1,203 B |
| Unique sealed bytes | 63,610 B | 14,170 B |
| A / B workload CPU | 5.144 / 23.898 ms | 1.819 / 7.404 ms |
| Coordinator process CPU | 13.216 ms | 4.869 ms |
| Time through both verified guest results | 3.793 s | 1.045 s |

The guest-port totals include reader polling, authorization, submission, lease and ACK exchanges within each phase. The sealed totals count canonical mailbox requests and signed receipts once, after matching every write to the accepting peer's reads. Cold snapshots covered 47 exchanges on A's guest port and 43 on B's. The warm counts were 13 and 10. Neither phase recorded a read or write error. Over the complete lifecycle, A's guest-port observer recorded one write error outside those phases. Both guest-port observers closed, at 62 and 54 exchanges. Each of the four sealed observers closed at five exchanges.

Setup through both verified SDK starts took 5.104 s. The complete command took 10.42 s wall, 9.88 s user CPU and 0.19 s system CPU, with peak RSS 84,856 KiB in GNU time's process accounting. Both original helpers exited 0 unassisted on their first SDK stop. No Firecracker process or active scope remained, and the audit verified 17 entries across two sessions and two seals. The two guests each used one vCPU and 256 MiB on one host with no guest NIC. HVF for this object route, WAN and sustained capacity remain separate acceptance gates. [Driver revision](https://github.com/R4-Laboratories/SpiralCore/commit/876e10156d4344b0865f99196b0cef7488e8ca1a) · [Open acceptance criteria](https://github.com/R4-Laboratories/SpiralCore/issues/202)

### First pilot with incomplete guest-port totals

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
