# First installed Linux/Firecracker transfer and retirement

On Wick, a canonically signed synthetic workload booted one installed guest through the original guardian. The live Firecracker API showed one vCPU, 1024 MiB RAM, vsock, and zero guest NICs. The authenticated peer and guest each verified the exact bytes. The original resident, native owner and endpoint owner completed shutdown and endpoint-store retirement. The witness verified that only the two expected telemetry reservations remained in the closed managed ledger. The protected build passed 30 focused checks, all 5,409 source readbacks matched, and the four native helpers linked against that graph.

| Observed measurement | Result |
| --- | ---: |
| Plaintext sent and returned | 11,600 bytes each way |
| TLS bytes at the peer, received and sent | 4,321 and 3,869 bytes |
| Peer elapsed after TCP accept | 60.60 ms |
| Peer application-thread CPU | 1.17 ms |
| TLS setup | 7.95 ms |
| Admission elapsed | 224.87 ms |
| Host test-process CPU | 1.263 s |
| Witness wall time including retirement | 2.043 s |

TLS bytes count the authenticated byte stream at the peer. This is a same-host fixture with a synthetic repeated payload. There is no comparison against a secured raw or zstd baseline, and the measured timing does not qualify WAN or multi-guest behavior.

The original Firecracker scope was inactive and dead after retirement. The original endpoint PID, endpoint-disk inventory, and native custody marker were absent. The telemetry catalog sealed an empty records file in this short run. The separate portable Runtime evidence generation remains on disk after successful stop. It occupies 104,759,296 allocated bytes and needs an explicit bounded retention or retirement policy before sustained qualification.

Two preceding executions already booted a guest. The first rejected a fresh V1 telemetry catalog because its witness incorrectly required a V2 handle. The second explicitly confirmed exact byte readback and original retirement, then failed an assertion that treated expected admission closure as a fault. The final witness checks catalog mode consistency and expects closed admission while retaining the exact reservation count, resource claim, and original retirement assertions. Product admission and retirement policy were unchanged by these witness corrections.

The [source and receipts](source-and-receipts.tar.xz) include both failed executions and the protected successful output. [Post-stop observations](post-stop.txt) record the separate retained disk footprint.
