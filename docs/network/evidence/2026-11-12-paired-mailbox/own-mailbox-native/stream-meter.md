# Installed durable helper stream measurements

The qualification build measures the original helper’s guest-facing mailbox connection after the vsock multiplexer has selected port 5257. It counts successful read and write return values, including short transfers on a later refusal. Phase records identify when the helper returned the Submit verdict, wrote the reader challenge, received a possession response, or produced a lease, ACK, empty response or refusal. The records contain sequence numbers, counters and fixed labels. [Metered helper revision](https://github.com/Casey-allard/mvm/commit/bd43ebb76b8239978151aed0a6950190141811ee) · [Host witness and reproduction](../../../../../tools/ari-own-mailbox-witness/README.md#measuring-the-original-durable-guest-stream)

The SDK driver retains the original report descriptor across VM retirement. A matching final record establishes recorder closure. A completely paired prefix without closure reports only the bytes observed in that prefix. An interrupted exchange, overflow, truncation or out-of-sequence line prevents aggregation. A workload readback, signed receipt and original process exit each have their own acceptance check.

| Linux x86-64 qualification | Result |
|---|---|
| Durable helper focused suite | 6 passed, including 4 new recorder controls |
| Original observer binary suite | 3 passed |
| Installed helper SHA-256 | `bf076aabafc0372bd5630c81bc10b65d1e2e2cf16d5620078427e5af37f925d1` |
| Host observer SHA-256 | `b1023e069beb503796259f7f4e025d672f5468b877f63a57d5983817da1c31d0` |

All 4,487 tracked MVM source paths, executable modes and contents matched the pinned fork before the final build on Wick. An early build using a shared target failed because it resolved an incompatible compiled mailbox grant. The final binaries used a fresh, isolated Cargo target. These component results establish the meter semantics. A native report must identify its own helper and observer binaries, whether it captured a closed session or a prefix, and the exact scopes of its byte counts. Firecracker packet headers, workload Calls, WAN traffic and data at rest have independent meters.
