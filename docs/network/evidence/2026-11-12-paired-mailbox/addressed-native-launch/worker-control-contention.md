# Concurrent installed mailbox admissions

Two guest launches can reach the same tenant coordinator while it handles an earlier request. We exercised two distinct admission points on the original signed Linux Unix socket. Each refused a different request before the fix.

| Active condition | Original behavior | Corrected behavior |
|---|---|---|
| Another operation holds the daemon’s mutation semaphore | A signed initial Acquire read EOF before its deadline. | Acquire waited, verified its authority again and returned the original reservation. A Query for the first owner completed while the semaphore was held. An expired waiting request created no slot. |
| Two workers are reserved and the only affordable control workspace is occupied | A signed Query for an original owner received a connection reset (Linux errno 104). | Query waited for the same workspace to be refunded and returned the original owner. A separate occupied-control case admitted the second worker without raising the byte limit. |

In the installed Cast profile, the fixed actor consumes **137,216 bytes** of accounting credit, each worker reserves **245,760 bytes**, and one control workspace reserves **327,680 bytes**. Two workers plus one workspace total **956,416 bytes** of the shared **1,048,576-byte** limit. These are protocol reservations. The ingress loop can hold one accepted socket per listener while waiting for a refund. Further connections remain in the kernel backlog. The same five-second ingress deadline covers waiting and reading. A verified mutation also has its own signed deadline and admission expiry. [Host implementation](https://github.com/Casey-allard/mvm/tree/2ae85317df9337705b52d01662c3ba0d335563b3/crates/mvm-hostd/src/broker)

Both targeted regressions passed on Linux. The entire worker-control test binary passed 16 of 17 cases. A separate test expected Query to observe collection after Stop timed out. That assertion failed on both the original product source and this change. The two contention cases establish possible causes for native control failures. Attribution of a particular guest run requires matching its own control evidence. The tests used one tenant and booted zero VMs. Fleet fairness, guest readback and cross-host measurements remain separate gates. [Exact selectors, failure classes and artifact hashes](worker-control-contention.json)

To reproduce, check out the specified MVM source and run `cargo test --locked -p mvm-hostd --test mailbox_worker_owner -- --test-threads=1`. The JSON identifies the two earlier test revisions that reproduce EOF and connection reset independently.
