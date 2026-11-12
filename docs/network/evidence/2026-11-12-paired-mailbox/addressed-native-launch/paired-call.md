# Two admitted guests and the first addressed workload call

Both original Linux/Firecracker guests booted, and both SDK launch calls returned admitted machines. Each guest had a separate signed mailbox, 1 vCPU, 256 MiB RAM and no NIC. The source Call supplied a 257-byte fixture. The destination Call supplied a separate 32-byte challenge. Neither workload returned verified output. [Original report fields, stream meters and termination observations](paired-call.json)

| Measurement on Wick | Result |
|---|---|
| Host pacing before guest clocks were seeded | 94.729 ms |
| Coordinator setup through both SDK launch returns | 5.581 s |
| A’s original mailbox stream on vsock 5257 | One dial. The host accepted 52 bytes for writing to the guest and read 0 bytes back. |
| B’s original mailbox stream on vsock 5257 | Zero dials. Both byte counters were 0. |
| Driver CPU through final coordinator exit | 8.379 s |
| Full lifecycle wall time, including intervention and retirement | 292.932 s |

The 52 bytes account for traffic on A’s mailbox stream only. The counters exclude the separate workload Call channels. There is no payload delivery, send-only latency or zstd comparison established by this run. [Original mailbox meter scope](../firecracker.md)

Both guest consoles recorded the OCI wrapper failing with EPERM after the warm worker spawned. Both signed plans specified the Standard seccomp tier. The host requested a 15-second Call deadline. The driver remained blocked in the Call path well beyond that deadline. The operators observed two workload-exit listeners and two blocked receives on sockets connected to the original VMMs. Determining the denied guest operation and bounding the host receive are separate work items.

After about 193 seconds the operator stopped the two original, independently identified systemd scopes. Both became inactive and empty. The driver then received a streaming response EOF, completed canonical stop and cleanup for both roles and collected the original coordinator with exit 42. The tenant reached Drained with zero pending operations. This was assisted teardown. The application readback and unassisted 15-second cancellation gates remain open. [MVM source used for the pilot](https://github.com/Casey-allard/mvm/tree/4f64492d44538e919f2e566ea8a325212b4c824b)
