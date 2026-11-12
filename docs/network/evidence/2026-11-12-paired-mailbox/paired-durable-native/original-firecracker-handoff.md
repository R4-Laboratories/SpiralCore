# Original Firecracker owner for a selected object pair

The Rust SDK needs to distinguish the VM it actually configured from a later process under the same machine name. For a cold Linux launch of the selected Call/Reply profile, the Firecracker driver takes a Linux lifetime descriptor using `SO_PEERPIDFD` on the connected Unix stream used for the boot configuration and `InstanceStart`. The kernel peer PID must also match the launch in progress. The original reader binding comes from the identity-drive writer that issued it. After successful launch the admitted plan retains both observations, and the selected host API checks that the peer is still alive and that the boot matches the signed admission. Firecracker may claim an idle process for this boot. [Contributor source](https://github.com/Casey-allard/mvm/commit/cea41a6844c328cc05a3d39b2a995372abf0c060)

| Linux component | Observed result |
| --- | --- |
| Config peer lifetime | A child process served the original socket. Replacing its pathname produced a distinct peer. The original descriptor tracked the child's exit while the replacement stayed live. |
| Boot identity | The bytes serialized from the original public reader binding appeared in the generated guest drive and matched the protected host record. A second issue attempt preserved both originals. |
| Installed admission | The existing gate continued to reject a selected plan when the paired service was unavailable. |

These checks used Linux processes, a Unix socket and the real drive-building code. No Firecracker guest was started for this result. The two-guest warm transfer, its wire bytes, CPU and completion time remain open. The host needs kernel support for `SO_PEERPIDFD` on the selected path. [Tests, original logs and measurement boundaries](original-firecracker-handoff.json) · [Open delivery gate](https://github.com/R4-Laboratories/SpiralCore/issues/202).

## Reproduce

Use the pinned MVM checkout on Linux with Rust 1.97.1. The recorded profile disabled incremental compilation and dev/test debug information and used two Cargo jobs. Use a fresh target or a target reserved for this checkout. A first host-test compile in a reused multi-checkout target saw a reader-response variant absent from the source under test. Refreshing the workspace dependency builds restored the expected admission result.

```sh
cargo +1.97.1 test --locked --offline -p mvm-backends --lib driver::fc::original::
cargo +1.97.1 test --locked --offline -p mvm-runtime --lib workload_runner::runner::spawner::tests::
cargo +1.97.1 test --locked --offline -p mvm-hostd --lib selected_object_pair_requires_installed_endpoint_support
cargo +1.97.1 check --locked --offline -p mvm-client
```
