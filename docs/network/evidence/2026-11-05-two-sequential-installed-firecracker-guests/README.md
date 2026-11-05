# Two sequential installed Firecracker guests under one service

One installed resident, original host helper, nonce ledger and managed resource pool started two independently signed guests on Wick. Each guest used a separate signed name, grant, authenticated peer scope and payload. Both completed an exact FlowMux round trip over vsock and retired through their original native and endpoint owners. Guest two started after guest one had returned the original ledger to zero live guests, zero guest memory and zero staging bytes. The first retained Runtime charge stayed in the same pool.

| Measurement | Guest 1 | Guest 2 |
| --- | ---: | ---: |
| Plaintext verified each direction | 11,600 B | 11,600 B |
| TLS bytes at peer, received and sent | 4,321 and 3,869 B | 4,321 and 3,869 B |
| Peer elapsed after accept | 56.88 ms | 55.73 ms |
| Peer application-thread CPU | 0.69 ms | 0.73 ms |
| Admission elapsed | 102.06 ms | 100.73 ms |
| Guest lifecycle through retirement | 1.590 s | 1.547 s |
| Retained Runtime allocated after stop | 104,759,296 B | 104,759,296 B |
| Cumulative cache charge after retirement | 125,870,080 B | 251,731,968 B |

Host test-process CPU for the full witness was 1.949 s. Full wall time was 3.231 s, including final shutdown and ledger checks. Across both guests the peers verified 46,400 plaintext bytes and counted 16,380 TLS bytes. This same-host synthetic witness has no matched raw or zstd control.

The post-stop record shows both original Firecracker scopes inactive, original endpoint PIDs absent, the endpoint-disk inventory gone, and the fixture temporary directory empty. Firecracker logs recorded machine configuration of one vCPU and 1024 MiB, vsock configuration and InstanceStart for both guests. Both guest consoles reported no eth0. The independent live API sampler did not capture a complete shape in this execution. The original protected witness verified both sealed telemetry rows, both separate retained disk charges, five final reservations and the exact final resource claim. Both records files were empty in this short run.

The protected build passed 30 focused checks with 5,409 source readbacks. Four native helpers linked from the same graph. This is sequential installed FlowMux evidence. Concurrent guests, the dedicated mailbox service, sustained operation, durable Runtime credit recovery, WAN and platform acceptance remain separate.

[Machine-readable result](result.json) · [Post-stop readback](post-stop.txt) · [Exact source and receipts](source-and-receipts.tar.xz)
