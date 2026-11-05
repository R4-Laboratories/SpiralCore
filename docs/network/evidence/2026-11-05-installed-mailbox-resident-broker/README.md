# Installed resident mailbox broker custody

A funded job previously owned the only strong broker reference. After the
last job retired, a later job received a new broker, which forgot accepted
identities and rate history and started a new monotonic clock. The regression
replayed the same signed envelope through two issued jobs and reproduced a
second AcceptedEphemeral result.

The original resident now retains the exact funded broker before an
entered job can mutate its service. Staged work keeps a weak reference in
the resident slot and retains its existing rollback on unused or failed
provisioning. One MiB of already-reserved broker metadata stays charged
between jobs. The original worker resources drain on retirement, and the
supervisor finishes after the resident provider releases the broker.

The successor returned Duplicate for the same message identity in the
second job. It passed 34 installed-parent checks, with three opt-in native
tests ignored in that command. The two-guest native development recovery
witness then ran separately and exited zero. Both grants retired with zero
guest and staging charges. Firecracker used port 5257 with no configured
guest NIC. The native rerun used fresh enrollment and credentials for the
second guest. It verifies compatibility with custody and retirement;
the cross-worker replay assertion comes from the socket-pair component.

| Native measurement | First guest, deliberate close | Second guest, full exchange |
| --- | ---: | ---: |
| Guest to host vsock stream payload | 120,195 B | 129,743 B |
| Host to guest vsock stream payload | 5,860 B | 130,970 B |
| Guest / host exchange CPU | 6.159 / 2.687 ms | 8.991 / 5.719 ms |
| Guest / host exchange elapsed | 8.590 / 8.604 ms | 13.840 / 13.417 ms |

The resident completed in 5.345 s through final retirement. Each direction's
Firecracker data-packet payload matched its endpoint's envelope and framing
counts. This measures one development rerun with workload-installed seccomp.
Worker-local pending Calls, FIFO state and Reply caches are not made portable
by broker retention. Autonomous expiry with no active receive, safe owner
epoch turnover, strict guest-agent enforcement, WAN and sustained capacity
remain separate acceptance gates.
