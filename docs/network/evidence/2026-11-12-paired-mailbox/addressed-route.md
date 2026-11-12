# Addressed Cast on Linux and macOS

A sender and receiver can share a host and tenant name while belonging to different signed authorization domains. The addressed route checks the source’s domain membership, the destination authority’s Send grant for that sender, and its separate Receive grant for the recipient. The destination verifies those rights before recording replay state. An earlier implementation consumed the replay identity before checking Receive and then refused the corrected retry. The regression now passes on both platforms. [MVM route source](https://github.com/Casey-allard/mvm/blob/50bc377cae41aa3cec10eb547d2d8ebcb6954d14/crates/mvm-core/src/mailbox/frame/cast/route.rs)

| Destination open through local inbox acknowledgment | Linux x86-64 | macOS arm64 |
|---|---:|---:|
| Application bytes verified | 5 | 5 |
| Deliver-role bytes sent / received over the Unix socketpair | 276 / 98 | 276 / 98 |
| Locally encoded envelope bytes | 2,975 | 2,967 |
| Elapsed time | 460,247 ns | 867,834 ns |
| Destination host-role thread CPU | 453,108 ns | 855,916 ns |
| Guest-role thread CPU | 7,455 ns | 14,167 ns |

The socketpair connected two threads in a test process. The encoded envelope stayed local. The timing covers one open and dispatch through acknowledgment per platform. A separate test handed 257 bytes to a workload process, checked the bytes in its callback and measured 474 bytes toward the workload and 46 back. These two fixtures measure different boundaries and their byte counts cannot be added into an end-to-end transfer. [Fixture and timing boundaries](https://github.com/Casey-allard/mvm/blob/50bc377cae41aa3cec10eb547d2d8ebcb6954d14/crates/mvm-core/tests/mailbox_cast_endpoint.rs) · [Measurements, test counts and executable hashes](addressed-route.json)

The host tests verified the shared two-worker reservation and independent default mailbox domains. The initial route limits are 4,096 bytes per encoded envelope, 32 KiB of reserved protocol capacity and one fresh admission per coordinator per minute. These limits bound the component. Customer capacity still requires measurements under sustained load. The two booted Firecracker guests already have separate own-mailbox readbacks. Cross-guest acceptance requires a signed source Submit, provisioned directional keys and verified readback inside the destination guest before the two-host WAN run. [Installed two-guest measurements](firecracker.md) · [Mailbox tracker](https://github.com/R4-Laboratories/SpiralCore/issues/43)

## Upstream timeout fix

Ari’s mailbox hunk from MVM PR #5370 rejects a duration that cannot fit the socket timeout field before accessing a descriptor. Its Linux regression failed on the prior implementation and passed with the original patch. The integration commit includes only the mailbox change and that regression. [Upstream change](https://github.com/tinylabscom/mvm/pull/5370) · [Contribution branch commit](https://github.com/Casey-allard/mvm/commit/87cc624e8bb7bb8053d5794633ced35f4d510baf)

## Reproduce the component tests

At MVM revision `50bc377cae41aa3cec10eb547d2d8ebcb6954d14`, run the following from the repository root. Append `-- --test-threads=1 --nocapture` to display each timing row. The Linux guest module includes an additional host-CID/socket-boundary check.

```sh
cargo test --locked -p mvm-core --test mailbox_cast_endpoint
cargo test --locked -p mvm-agentd --lib mailbox_cast::tests::
cargo test --locked -p mvm-hostd --lib addressed_cast_shares_the_two_workers_and_preserves_control_headroom
cargo test --locked -p mvm-hostd --lib installed_default_mailbox_domains_differ_between_named_guests
```

At revision `87cc624e8bb7bb8053d5794633ced35f4d510baf`, run the upstream regression with `cargo test --locked -p mvm-agentd --lib mailbox_outbox::tests::a_timeout_outside_timeval_range_is_refused_before_touching_the_socket -- --exact`.
