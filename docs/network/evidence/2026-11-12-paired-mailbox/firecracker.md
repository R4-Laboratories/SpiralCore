# Two installed Firecracker guests on one host

Two Linux x86-64 guests, each with 1 vCPU, 256 MiB and no NIC, were simultaneously serving under separate signed mailbox grants on one coordinator. Each producer addressed its own mailbox. The host verified A’s original payload, stopped A and collected its original helper, confirmed B was still serving, then read B’s distinct payload. Both 1,024-byte readbacks matched their independent originals in content, length and SHA-256.

| Measurement | Guest A | Guest B |
|---|---:|---:|
| Application bytes verified by host | 1,024 | 1,024 |
| Original helper bytes read from guest on vsock 5257 | 1,060 | 1,060 |
| Original helper bytes accepted for writing back | 11 | 11 |
| Per-guest setup | 741.314 ms | 320.475 ms |
| Host mailbox receive through verified readback | 3.201 ms | 3.013 ms |
| Stop and collection of original helper receipt | 143.461 ms | 124.184 ms |

The host measured **2,142 bidirectional mailbox stream bytes** across both guests for 2,048 application bytes. Host receive time starts at the host read and may exclude time the message already spent queued. Both original helper exits were 0. The original coordinator exited with 42, its expected idle-retirement status. The tenant reached Drained with zero pending operations and both VM definitions and runtime states were absent.

The shared lifecycle took **104.402 s** including retention and idle shutdown. Host orchestrator CPU for that interval was **472.205 ms**. Guest-only CPU, send-only latency and NIC/WAN bytes were unmeasured. This test uses the plain mailbox and has no zstd or SpiralCore codec comparison.

The test used the [combined MVM source](https://github.com/Casey-allard/mvm/commit/11c6f04bd1f6fa261b6543d678acf8226e699f33) and the [executed two-guest Rust witness](https://github.com/R4-Laboratories/SpiralCore/tree/d7bca81e515e3c975e73dbdf2364fe3688f964af/tools/installed-mailbox-witness). An initial host build picked up an older contract artifact. Qualification rebuilt the in-tree packages and retained the original diagnostic. The corrected witness executable matched the first build byte-for-byte. Each guest’s retained runtime image and embedded agent were verified independently of the synthetic workload producer.

An authorized transfer from guest A to a workload in guest B still needs the installed route, the authenticated workload handoff and direct destination readback. A two-host run must then exercise that same path over the WAN. [Mailbox delivery tracker](https://github.com/R4-Laboratories/SpiralCore/issues/43).

[Recorded measurements, source and binary hashes](firecracker.json).
