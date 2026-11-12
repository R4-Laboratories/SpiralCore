# Linux guest workload binding to MVM's local-durable mailbox

The application staged a 257-byte binary SpiralCore frame body containing every byte value and a trailing zero. It read back the complete frame, checked its original MVM message ID and exact frame contents, then explicitly acknowledged that delivery lease. Its thread ran the actual MVM Standard seccomp program with the mailbox grant selected. The peer was a synthetic MVM reader on a real Unix stream. It used MVM's canonical message, lease and ACK encoding with fixture keys. [Binding and reproduction command](../../../../../crates/spiralcore-mvm-guest/README.md) · [MVM draft #5377](https://github.com/tinylabscom/mvm/pull/5377)

| Boundary in the recorded run | Bytes |
|---|---:|
| Synthetic application's frame body | 257 |
| Encoded SpiralCore frame, including its 56-byte header | 313 |
| Staged MVM file, including 32-byte recipient and 32-byte message ID | 377 |
| Application → reader, Receive request | 54 |
| Reader → application, Delivery response | 2,009 |
| Application → reader, explicit ACK request carrying the lease | 1,501 |
| Reader → application, ACK confirmation | 1,764 |

The socket totals were **1,555 bytes from the application** and **3,773 bytes to the application**. The Delivery response includes 420 bytes of base64 for the 313-byte frame. The remaining 1,589 bytes include framing, type and signed lease metadata. The two ACK messages account for 3,265 bytes. The 377-byte file stays a separate filesystem measurement. The journal's signed V2 destination receipt was outside this reader exchange.

One application-thread sample used **1.804484 ms CPU** and **3.926032 ms elapsed** from filter setup through frame preparation, staging, readback, comparison and ACK confirmation. The elapsed interval includes scheduling and fixture response time. CPU excludes the synthetic reader thread and the kernel/host work of a running VM. These numbers describe one local operation with debug-profile binaries and supply no compression or throughput comparison.

Four adapter tests passed, including ambiguity after a partial reader response, refusal/empty handling, exact same-ID staging and conflicting outbox protection. The provider's upstream guest writer, reader, wire and confinement checks were also run separately at the pinned revision. The production destination journal, two admitted guest VMs, inter-host transport, regular secure-zstd baseline and WAN measurements remain separate acceptance gates. [Machine-readable results](results.json) · [Paired delivery tracker](https://github.com/R4-Laboratories/SpiralCore/issues/168)
