# Native delivery with unassisted shutdown

Guest A sent a 313-byte SpiralCore frame through the signed MVM mailbox direction to guest B. B decoded and recovered all 257 bytes, synced and reopened its guest file, acknowledged the lease and returned the exact bytes. The host compared B’s readback with A’s original fixture. Both original VMMs stopped and both original mailbox helpers then exited unassisted with status 0 on the first attempt. Both port-5257 recorders closed normally. The driver exited 0 and the SDK audit verified 17 entries, two sessions and two seals.

The first two-guest pilot delivered the same object and verified B’s signed V2 receipt, but its helpers stayed alive until terminated by the SDK. This second run used a fresh installation, new signed original VM identities, the same measured guest runtime and the same two sealed role images. The host correction gives each paired helper a kernel exit observer for its own original VMM. The helper can finish after the VMM exits while the owning orchestrator and the protected state directory remain alive. [Initial pilot](README.md) · [Qualified source change](https://github.com/Casey-allard/mvm/compare/7a1aa5bc5f92e7530e3939d0b3917ab9491cd207...27038e3f322e4336df180073e12cae59cc4dba54) · [Original-helper control](../persistent-sdk/original-lifetime.md).

| One observed transfer per version | Initial pilot | Graceful rerun |
|---|---:|---:|
| Original helper relay, sealed A → B | 5,810 B | 5,856 B |
| Original helper relay, signed B → A | 1,195 B | 1,207 B |
| Setup before first A Call | 4.678693 s | 4.612147 s |
| First A Call through verified destination V2 | 117.819 ms | 118.420 ms |
| Same start through B’s ACK and exact readback | 1.103199 s | 850.304 ms |
| Guest A stage, wall / process CPU | 0.614303 / 0.211465 ms | 0.580549 / 0.187363 ms |
| Guest B receive through readback and ACK, wall / process CPU | 935.310775 / 0.601859 ms | 674.501934 / 0.554139 ms |
| Orchestrator wall / process CPU, including setup and retirement | 6.567857 / 5.169023 s | 5.753559 / 5.134208 s |
| External invocation wall / user / system | 6.63 / 5.20 / 0.13 s | 5.82 / 5.19 / 0.10 s |
| External peak RSS | 81,936 KiB | 82,280 KiB |
| Original helpers | Both assisted, driver exit 1 | Both unassisted, driver exit 0 |

The times are single same-host observations on one Linux x86-64 machine. Each Firecracker guest had one vCPU and 256 MiB RAM, with no guest NIC. Verification of V2 measures the host’s observation of B’s durable admission. Guest B’s application ACK and exact readback are later events. No direct application-level receipt observation in A, matched zstd transfer, remote route, send-only time, per-helper CPU or energy measurement was taken.

## Closed original port-5257 streams

| Rerun meter at original helper | Guest → host | Host → guest |
|---|---:|---:|
| A submission | 381 B | 1,207 B |
| B reader lease, challenge and possession | 355 B | 5,950 B |
| B reader ACK | 2,288 B | 2,292 B |
| B stream total | 2,643 B | 8,242 B |

The final zero-byte turn in each recorder encountered a write error as the guest closed. Each recorder then wrote its matching end-of-session footer. These are full counts for the observed application streams at the original helpers. The 7,063-byte helper-to-helper relay and the workload/control sockets have separate measurement boundaries. The elapsed and CPU intervals overlap. [Original A recorder](graceful-a.counts) · [Original B recorder](graceful-b.counts) · [Measurements, executable hashes and raw record digests](graceful.json).

To reproduce, follow the [native operator instructions](../../../../../tools/ari-own-mailbox-witness/paired-native.md) with host revision [27038e3f](https://github.com/Casey-allard/mvm/tree/27038e3f322e4336df180073e12cae59cc4dba54), guest revision [93b3b65a](https://github.com/Casey-allard/mvm/tree/93b3b65ae72a4746a54a21c7c158d5453cceb8f5) and the same workload digests. A deliberately lost V2 response and byte-identical retry remain to be demonstrated in these two installed guests. Earlier tests exercised that case with two real helpers and synthetic guest sockets. [Helper retry](../two-helper-journal/README.md) · [Remaining acceptance](https://github.com/R4-Laboratories/SpiralCore/issues/168).
