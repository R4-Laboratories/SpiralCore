# Installed native recovery after a withheld application reply

A canonically enrolled cooperating peer read and verified all 11,600 request bytes from the first installed Firecracker guest, then closed the mapped write direction with zero application reply bytes. The guest parsed the successful proxy CONNECT response, found the recovered body empty, and rejected it. The witness required the exact refusal marker on stderr, zero stdout, no backpressure, and exit status 1. This prevents an unrelated guest error or a silently accepted empty reply from satisfying the fault test.

The original native and endpoint owners retired the first guest and returned the ledger to zero live guests and zero staging, with the first Runtime generation still charged. The same resident and host helper then admitted a separately signed second guest. Its different authenticated scope and payload completed a byte-exact round trip of 11,600 bytes each way. Both post-stop scopes were inactive, original endpoint PIDs and custody markers were absent, and the fixture temporary directory was empty. The ledger and two telemetry rows passed their exact final checks.

| Measurement | Withheld reply | Subsequent exact transfer |
| --- | ---: | ---: |
| Application bytes at peer, received and sent | 11,600 and 0 B | 11,600 and 11,600 B |
| TLS bytes at peer, received and sent | 3,677 and 3,172 B | 4,321 and 3,869 B |
| Peer elapsed after accept | 47.15 ms | 57.61 ms |
| Peer application-thread CPU | 0.79 ms | 0.86 ms |
| Guest lifecycle through retirement | 1.595 s | 1.579 s |

Full witness wall time was 3.259 s with 2.022 s of host test-process CPU. The protected build passed 30 focused checks, all 5,409 source hashes matched, and the four native helpers linked against that graph. An earlier fixture variant compiled but was held from guest execution because its refusal oracle could accept unexpected stdout. The protected successor requires the exact causal marker.

The fault is an orderly authenticated half-close before an application reply in the installed same-host FlowMux path. Process crash, persisted replay, direct/relay WAN, mailbox final acknowledgments, and concurrent guest behavior require separate qualification.

[Result](result.json) · [Post-stop readback](post-stop.txt) · [Exact source and receipts](source-and-receipts.tar.xz)
