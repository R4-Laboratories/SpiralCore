# Both Firecracker guests reached workload activation

A fresh installed MVM coordinator admitted the two original guest plans on each of two same-host runs. Both VMMs booted with 1 vCPU, 256 MiB and no NIC. The guest agent in each role responded to the activation request. The 257-byte application fixture never reached guest B. The installed host passed all 17 worker-ownership checks before launch. [Run data, source identities and report digests](guest-activation.json)

| Run | Both guest agents reported | Coordinator setup through both SDK returns | Host driver CPU through retirement deadline | Wall time through retirement deadline |
|---|---|---:|---:|---:|
| 7 | Permission denied reading the sealed image’s runtime configuration | 5.354 s | 7.931 s | 215.443 s |
| 8 | Cast outside the admitted local endpoint | 5.340 s | 7.891 s | 215.432 s |

In run 7 the configuration file had read permission, but the service account lacked traversal permission on its parent directories. Run 8 used images with traversable workload paths. [Service-account readback of the sealed images](../operator-image-service-access/README.md). That run reached the Cast authorization check on both guests. The public SDK reports contain sanitized refusal categories. Only the corresponding fixed error labels from the private guest diagnostics appear here.

For both runs the SDK reported that original backend termination was unconfirmed. The operator verified each original systemd scope, invocation identity, retained cgroup identity, VMM executable and private API endpoint before stopping that scope on Wick. All four scopes became inactive and empty. The operator confirmed zero remaining Firecracker processes after each pair and retained the machine definitions and mailbox owner records for reconciliation.

The retired VMM scopes establish administrative teardown. Neither driver obtained the original coordinator’s exit receipt within its deadline. Guest mailbox stream meters, exact application readback, send-only time and WAN bytes remain unmeasured here. [Previous two-guest guest-to-host measurement](../firecracker.md) · [Earlier reservation results](reservations.md)
