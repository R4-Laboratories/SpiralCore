# Original object Call/Reply plans and Linux workload images

The operator prepared two Linux x86_64 workload images for the installed object pilot. Reopening each ext4 image recovered the exact static ELF and confirmed that each guest is configured with one serial warm worker. The hashes below identify the two artifacts to select before either plan is signed. [Full digests, verity sidecars, runtime and executable identities](object-role-images.json)

| Original role | Workload | ext4 SHA-256 prefix | Worker SHA-256 prefix | Worker bytes | Calls per worker |
| --- | --- | --- | --- | ---: | ---: |
| A, caller | `spiralcore-durable-source` | `e1c9538a9dd52be5` | `e8403ef044ee0d88` | 2,825,488 | 16 |
| B, callee | `spiralcore-durable-destination` | `a0cfa1a1099e227d` | `037cad09cf84e0b5` | 2,825,488 | 8 |

Each ext4 file is 3,559,424 bytes. Both use the same canonical guest runtime archive, `187758c4b76fe311…`. The root filesystem images were sealed with the existing image builder. The guest workers were built with Rust 1.97.1, `cargo zigbuild --release --locked --offline --target x86_64-unknown-linux-musl --features large-object-fixture,object-reader-v1 --bins`. The series-a and series-b image profiles select the original role entrypoints and limit each worker to one queued request. [Workload source](https://github.com/R4-Laboratories/SpiralCore/tree/584f7f29c3b67406525dba0002a3601264c1fbc6/tools/ari-own-mailbox-witness/paired-guest) · [MVM guest runtime source](https://github.com/Casey-allard/mvm/commit/c9d9fe87b27cd2e5fc64d2b1d9c3a31b522b2097)

### What the operator checked

In a new private qualification home with a 30-second retention ceiling, the operator issued four signed capabilities. B's mailbox authority signed A's Call and B's Receive. A's authority signed B's Reply and A's Receive. Each direction permits one outstanding Call, with maximum application bodies of 4,152 bytes for a Call and 88 bytes for a Reply. The operator used MVM's original signing keys and retained the same two nonces for plan selection. A focused Linux test verified all four signatures and rejected swapped Receive capabilities and swapped original plan nonces. One invocation of the separate preflight executable took 0.04 s wall, 0.02 s user CPU and 0.00 s system CPU at 0.01 s resolution. Peak RSS was 33,840 KiB. [Operator source](../../../../../tools/ari-own-mailbox-witness/bin/paired/object_enrollment.rs)

The follow-up MVM admission code checks the selected name, image digest, mailbox, nonce and current authority against the prepared original pair. The plan test used both original signatures, refused an altered image, and rejected a later revocation of B's Receive credential before A could reuse the plan. The original pair tests passed 4/4 and the canonical admission suite passed 101/101 on Linux. Both the self-signed and externally signed admission paths retain the sealed native posture requirement. This revision still refuses service activation. [Original-plan source](https://github.com/Casey-allard/mvm/commit/27debc8a0451e10256b48e20332691c052d1f345)

### Installed acceptance

The booting process must hold the original in-memory preparation through plan signing and original worker registration. Running the standalone preflight a second time creates a fresh pair. These checks launched zero guests. The open acceptance run measures the cold 4 KiB readback, the warm repeat on the same B worker, actual traffic on port 5257, CPU, transfer time and teardown on the admitted pair. [Installed object acceptance #202](https://github.com/R4-Laboratories/SpiralCore/issues/202)
