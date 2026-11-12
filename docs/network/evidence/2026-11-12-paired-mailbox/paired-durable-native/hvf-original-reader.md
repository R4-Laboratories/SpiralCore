# Original mailbox reader for an HVF cold launch

The selected object-pair SDK now borrows the mailbox reader identity from the original cold launch. Its production identity writer captures the public boot record while creating the guest identity drive. The runtime retains that record alongside the original native owner. Before the SDK uses it, the existing HVF lifecycle check requires the activated child, its control instance and its authenticated guest bridge. The compatibility entry point also requires exact equality with the reader supplied by its caller. [Contributor source](https://github.com/Casey-allard/mvm/commit/b5e9d0b8370e442a52c2e9bb44f5945e719f0cdc)

Two cold starts may share a signed plan, mailbox, epoch and plan nonce. Their independently minted boot IDs and ACK keys distinguish which guest can acknowledge a delivery. A plan-only comparison accepted the sibling binding. The regression now rejects a different boot and a different ACK key, and the original remains usable. The component checks ran on Linux x86-64 with Rust 1.97.1, a locked debug build, two build jobs and debug symbols disabled. All 4,526 source blobs matched contributor tree `b406198f78190430874c85c91148caaea3f97cd9`. [Baseline regression](https://github.com/Casey-allard/mvm/commit/e8f38dfcb4af00b8317ad5d35bb5dfb69bf0c791)

| Cargo package and test filter | Result | Raw log SHA-256 |
| --- | --- | --- |
| `mvm-hostd --lib original_hvf_reader_rejects` on the plan-only comparison | 0/1, accepted a sibling cold boot | `ea1e8e2ac89cb025807042514131e2192bc4c9f596269965b5c04909dd70216b` |
| `mvm-hostd --lib mailbox_launch::object_pair::tests::` | 8/8, including the original boot and ACK key checks | `e058c57ce38d28e9a9fe27022bcfa60c32b2a1df216d713c409822a71382b91f` |
| `mvm-runtime --lib original_selected_reader_is_the_boot_written_to_its_single_cold_drive` | 1/1, independent cold drive bytes under the same plan and refused reuse of the first mint | `043c5dd1ac18f7b0c69d2be0c55f840aa0edf0baf3e68e8c6b67795d0abbd455` |
| `mvm-runtime --lib cold_hvf_channel::tests::` | 12/12 lower lifecycle controls | `81d206acb3dcaf5d6a008be0c9ed65d40bd3947d652dadeba6bc433f8b63f1a7` |
| `cargo check --locked -p mvm-client --lib` | SDK compiled | `91d7d6df001783cfcba3f3f15cac78f66b6ebacadd6257ca71335c2063a761c8` |

The final hostd test executable SHA-256 was `5eaf0289838cb375e04575e9ab3c917e698d250395320137f59589e15470d940`. The runtime test executable SHA-256 was `603e9d3ecffb74d0841c1570f73aeaef723801c5af4088b57bbe3d1cf999cdb2`.

The next Apple integration needs a worker lifetime channel tied to the two original HVF processes and a connection to each guest’s original mailbox bridge. The existing selected worker accepts Linux Firecracker pidfds. This run started zero guests and produced no transport bytes, CPU or transfer-time measurement. Apple Silicon execution of the selected object pair remains open. Earlier Apple results exercised the durable Cast route. [Apple Cast evidence](arm64-hvf.md) · [Object-pair delivery tracker](https://github.com/R4-Laboratories/SpiralCore/issues/202)
