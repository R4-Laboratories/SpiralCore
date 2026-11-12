# Concurrent SDK launches and lifecycle publication

The installed pair’s destination launch failed while opening the tenant lifecycle gate. Its SDK reported an unresolved publication intent. When that launch withdrew from pair registration, the source saw an unavailable Cast lane. A regression reproduced the destination error while the original publisher was still alive. This checkpoint tests the lifecycle fault independently of a guest, Cast grant or transport. [Component results](concurrent-publication.json)

## Why the second launch stopped

The tenant coordinator serializes launch reservations using its original file lock. While holding that lock it creates a private intent file, writes the new state, and removes the intent. A crash can leave the file behind. The previous reader checked for it before acquiring the lock. That made an active writer and an interrupted writer indistinguishable to the competing reader. [Previous implementation](https://github.com/Casey-allard/mvm/blob/a571b4957e2e72064ef8dd59f972757da31feddb/crates/mvm-core/src/tenant_lifecycle.rs)

A caller can now obtain a handle to the existing gate while a writer holds the lock. Operations on that handle still require the original lock inode, a private namespace, and the caller’s deadline. Once the reader acquires the lock it inspects the intent. A surviving marker still prevents admission. The gate leaves crash evidence in place and preserves the generation and each VM’s distinct launch reservation. [Tested implementation](https://github.com/Casey-allard/mvm/commit/0f174e37facb0ee1e3a69dce3a08cab451b71f21) · [Reproducible patch](concurrent-publication.patch)

## Tests

| Platform and source | Result |
|---|---|
| macOS arm64, original host with the new regression | 0/1. A legitimate live publication produced the same unresolved-intent error. |
| macOS arm64, corrected host | 10/10 internal lifecycle tests and 15/15 public coordinator integration tests passed. |
| Linux x86_64, corrected host with the host-state feature | 10/10 internal lifecycle tests passed. |

The fixture holds the real lock after durable intent creation. A second caller is allowed to select the gate. Its short admission deadline expires while the writer is held. When the first publication finishes, the second caller can reserve under the same tenant generation. Each VM has a distinct reservation ID. Settling the second preserves the first. If publication fails instead, both a fresh caller and the already-selected second handle refuse admission, and the marker survives. Additional tests cover replaced lock inodes, stale generations, failed publication and incomplete drains. These are component results. No guest traffic or WAN timing was measured. [Original regression source](https://github.com/Casey-allard/mvm/commit/bda135521e8612307be9f2f3d6145d4b3e502ca4)

```sh
cargo test --locked --offline -p mvm-core --lib tenant_lifecycle::tests:: -- --test-threads=1
cargo test --locked --offline -p mvm-core --test tenant_lifecycle_coordinator -- --test-threads=1
# Linux component configuration
cargo test --locked --offline -p mvm-core --features host-state --lib tenant_lifecycle::tests:: -- --test-threads=1
```
