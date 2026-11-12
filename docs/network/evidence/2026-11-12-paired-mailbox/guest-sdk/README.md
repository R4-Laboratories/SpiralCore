# Two signed guest roles at launch

Two SDK launches now rendezvous on their original, separately acquired VM leases. Each uses its own signed boot record and private mailbox role. Before either launch proceeds, both workers must agree on the same transfer direction and stage their boot records within the shared deadline. [SDK source](https://github.com/Casey-allard/mvm/blob/60965c80db91ca98f51eb647dc32e7dcf16495ee/crates/mvm-client/src/launch/cast_rendezvous.rs)

A delayed observer used to be able to undo one side after its peer had already observed joint publication. The regression reproduced that split decision. Once both sides publish on time, their decision now survives a delayed wake or cancellation. Starting the VM still requires current authorization. Nine focused tests passed on Linux at `60965c80`, including cancellation, the delayed observer, sidecar rollback, capability expiry and refusal to release source bytes after a revoked greeting. [Measurements, test names and executable hashes](results.json)

## Guest workload measurements

These runs exercised two different edges of the mailbox path on Linux x86-64, using actual Rust workload processes in isolated PID and mount namespaces. Each used synthetic 257-byte input. Each row is a single observation at the source revision shown.

| Edge exercised | Actual bytes at that edge | Elapsed | Agent thread CPU | Workload CPU |
|---|---|---:|---:|---:|
| Source workload submits through its guest agent, [a0cc1f3c](https://github.com/Casey-allard/mvm/commit/a0cc1f3ce34796d82caeb2da18334742b8ee7d5f) | 477 toward the host-side test peer, 194 back. Application received a 32-byte result. | 50.228 ms | 0.088 ms | 0.099 ms |
| Destination agent hands bytes to its workload, [e670d5fc](https://github.com/Casey-allard/mvm/commit/e670d5fc7267b1023f662f79649ae09ea5f6c2ef) | 474 toward the workload, 46 back, plus a separate 250-byte setup datagram. Workload verified all 257 bytes. | 51.254 ms | 0.832 ms | 0.088 ms |

The source timer covers dispatch through the local application result. The destination timer covers its workload exercise. The rows use separate processes, runs, source revisions and measurement boundaries, so adding the two timings would not give an A-to-B latency. The next measurement requires A and B booted together, one submission through the installed helpers, and an independent read inside B. [Mailbox integration tracker](https://github.com/R4-Laboratories/SpiralCore/issues/43)
