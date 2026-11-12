# Host deadline and unassisted retirement on two Firecracker guests

Both signed Standard guests booted and both SDK launches returned. Each had one vCPU, 256 MiB RAM, a separate mailbox and no NIC. The addressed driver started two workload Calls and the signed route under one 15-second host deadline. The route failed. Both Calls returned a control-exchange deadline error. The original SDK stopped and cleaned up A and B without operator intervention. The coordinator exited 42 and its persisted lifecycle showed Drained with zero pending and no unresolved work. A process count after exit found zero Firecracker processes on Wick. [Original driver JSON](paired-deadline-native.json)

| Same-host Linux measurement | Previous run (12) | Deadline run (13) |
|---|---:|---:|
| Setup through both SDK admissions | 5.581 s | 5.686 s |
| Input length supplied to A's workload Call | 257 B | 257 B |
| A mailbox stream, host write accepted / host read | 52 B / 0 B | 52 B / 0 B |
| B mailbox stream, host write accepted / host read | 0 B / 0 B | 0 B / 0 B |
| A / B original SDK retirement | 239 / 87 ms | 235 / 104 ms |
| Host driver process CPU | 8.379 s | 8.482 s |
| Full lifecycle wall time | 292.932 s | 121.586 s |
| Operator needed to stop the VMMs | Yes | No |
| Exact application readback in B | Missing | Missing |

Each run used a fresh challenge of the same length and the same sealed role images. The installed mailbox meter counts the original host streams on port 5257. It excludes both workload control channels. Both runs recorded one A dial and no B dial. The 15 seconds is the configured common deadline. The individual call starts and returns have no elapsed-time sample. The full lifecycle includes waiting for the tenant coordinator to retire. The driver reported failure on the addressed delivery gate. Guest CPU, codec CPU, send-only latency and WAN bytes remain unmeasured. [Previous native run](paired-call.md) · [Component deadline control](one-shot-call-deadline.md)

### Reproduction pins

- [Host, SDK and helper source](https://github.com/Casey-allard/mvm/commit/82bc1e2d1324cf9251a674166bed1c1e95640dc7)
- SpiralCore driver built at `73168d3396c6a68230a4984b9b1f19a3526a7929`. The [published witness subtree](https://github.com/R4-Laboratories/SpiralCore/tree/e6a1783be8ff2bc2aef7bcbdd762d491942f20a9/tools/installed-mailbox-witness) has identical Git tree contents. Installed SHA-256 `8045ed7d9c9cf91033d9eb9e8505e0b742c648935b87f8bdbbe0d79b79c49aaf`
- Guest runtime `9d7224cf1d08949aed0b08311e9ed94d1f904303`
- Original driver JSON SHA-256 `1d9a81e9189dc82a07c48053eae99b064ac37ec69e1a8013ecac5ca1768156f5`
