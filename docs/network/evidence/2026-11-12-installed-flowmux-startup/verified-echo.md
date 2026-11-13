# Installed FlowMux echo on Linux Firecracker

Trial 9 completed one cold, signed entrypoint invocation in an x86_64 Linux Firecracker guest under the Dev posture. The application received HTTP 403 for its denied destination. Its second CONNECT received HTTP 200, sent 5,804 bytes, read the full 5,804-byte echo, checked equality and reached EOF. The independent, mutually authenticated cooperating peer checked the complete original payload before echoing it. The guest exited zero with no stderr. The unchanged workload package and fresh signed two-verb plan used the same finite resources as [trial 8](entrypoint-grant.md). [Exact results and artifact digests](verified-echo.json)

| Observation | Sent | Received | Wall time | Process CPU |
|---|---:|---:|---:|---:|
| Guest denied CONNECT, request and response | 79 B | 64 B | 6.961262 ms | 1.150120 ms |
| Guest permitted CONNECT and verified echo | 69 B CONNECT + 5,804 B application | 39 B CONNECT + 5,804 B application | 179.468664 ms | 2.770237 ms |
| Cooperating peer, accepted TLS session | 3,234 B TLS | 3,661 B TLS | 166.194533 ms from accept through half-close and resource drain | Phase CPU unmeasured |

The 5,804 bytes are application stream data. The fixture repeats a 29-byte JSON line 200 times, then appends four binary bytes. Both applications compare all bytes. This single repetitive input does not establish general compression performance.

The TLS counters cover the original peer socket, including the handshake and object mapping. The guest counts cover the application and its local proxy. Those counts overlap across layers. The peer ran on the same physical host. TCP/IP headers, physical interface bytes and WAN latency were unmeasured.

Canonical startup preparation took 6.188641063 s. The SDK returned the running guest at 32.235299808 s from the start of preparation. The complete authenticated entrypoint invocation took 232.034574 ms. Original SDK retirement finished at 32.647609664 s from the same start. The SDK command and collected children used 29.99 s user and 1.63 s system CPU over 32.83 s wall time. Its peak RSS was 274,548 KiB and the controller reported a virtual peak of 571,940 KiB. The separate peer process used 0.01 s user and 0.00 s system at GNU time resolution over 32.58 s, including its wait for the guest. The separate installer used 4.81 s user and 0.28 s system over 5.46 s.

The installed owner retained the original audit file through deregistration. A separate macOS content check verified two signed entries in the 2,336-byte copy against the original signer, tenant, workload and session. The entries recorded one `host.flow.allowed` for the admitted peer and one `host.flow.denied` for the denied destination, both on port 38443. The content checker used the canonical verifier from a previously built dependency set. Its digest, the chain digest and the witness receipt are recorded in the JSON.

The original SDK, peer and namespace exited zero. No Firecracker process, mount or loop attachment remained. The tenant stopped. The terminal ledger showed zero guests, guest RAM and workers and verified all four retained reservation identities. The retained balances were 806,371,328 host metadata bytes, 9,494,528 cache bytes and 536,875,008 staging bytes. Full reclamation, additional guest lifetimes, two simultaneous funded endpoints and cross-platform operation remain open.

The controller was built from the measured SpiralCore source with its MVM dependencies resolved against the same local copy of the contributor revision. All 4,631 MVM files and modes matched that revision. The controller’s two output tests and strict Clippy passed. Fourteen focused MVM controls passed, including two real registered helper tests, and seven packages passed strict Clippy. The published source links identify the measured components. Final integration of the package manifests and lockfiles remains a separate gate. [SpiralCore witness source](https://github.com/R4-Laboratories/SpiralCore/commit/8f0a1e363704a3348d9fde51658a10bffa0347cd) · [MVM SDK and helper source](https://github.com/Casey-allard/mvm/commit/fc09150b5c08bc037c5c5f66daa461648f1fb911)

After the run, a broader workspace build at the measured MVM revision found a missing field in a test-only mock initializer. The owner supplied the field in a fixture-only successor. The component and native results above apply to the measured revision. Whole-workspace acceptance is recorded separately.
