# Installed two-guest mailbox reservation on Linux

Two signed plans reached admission on each of two fresh Linux installations on Wick. The installed coordinator returned its expected original-child exit status of 42 on both runs. Neither launch pair reached the workloads. The 257-byte fixture has no destination readback, vsock transfer counts or transfer timing from these runs. [Recorded results, meter scopes and report digests](reservations.json)

| Installed run | Guest A original owner record | Guest B original owner record | Coordinator setup and both SDK launch attempts, wall | Driver process CPU over lifecycle | Full lifecycle, wall |
|---|---|---|---:|---:|---:|
| 4 | PendingV1 | Owned | 4.645 s | 7.119 s | 104.984 s |
| 5 | Owned | PendingV1 | 4.663 s | 7.162 s | 105.006 s |

Each guest requested 1 vCPU, 256 MiB and no NIC. Both runs used the same verified kernel and separately [sealed source and destination images](../operator-images/README.md). A prior preparation attempt omitted resource-owner provisioning in the fresh home. The coordinator refused startup. Runs 4 and 5 used the canonical SDK provisioner before installing the guest runtime cache.

In run 5 the destination SDK refused an unresolved lifecycle publication intent while the source held an original worker reservation. Cancellation retired their shared Cast rendezvous. The source then refused that retired rendezvous. A lifecycle writer temporarily publishes an intent marker while holding the tenant gate. A second reader can see that marker before acquiring the gate. The repair must let the reader wait for the active writer and still refuse an intent left behind after a failed publication. Run 4 retained only the generic backend error for each role. [Lifecycle gate at the tested source](https://github.com/Casey-allard/mvm/blob/a571b4957e2e72064ef8dd59f972757da31feddb/crates/mvm-core/src/tenant_lifecycle.rs)

On both runs the coordinator reached Drained with no pending tenant operations. The driver kept each VM definition and original owner record because it had no launch outcome authorizing removal. The first failed installation used an older coordinator and retains its own unresolved retirement record. [First installed launch](README.md) · [Corrected watcher](watcher-retirement.md)
