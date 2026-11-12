# Starting a paired journal with the persistent MVM SDK

An orchestrator can now start a stopped persistent machine with one explicit mailbox role using `LocalBackend::start_persistent_with_journal`. The SDK carries the original authority into signed-plan admission and retains the helper it started in the caller’s slot. The application continues to use MVM’s existing mailbox. [Contributor implementation](https://github.com/Casey-allard/mvm/commit/ce3d0f0e9da5d8d2ea6bf013170f25f4ba50c690) · [Rust patch](persistent-sdk.patch)

| Step | What the orchestrator and SDK do |
| --- | --- |
| Define | Create B’s and A’s persistent machine definitions through MVM. Each journal grant applies to one boot. |
| Start B | Pass B’s mailbox grant, its enrolled incoming policy, and an empty destination-owner slot. The SDK verifies the signed production plan and starts B’s original journal worker. |
| Start A | Supply the live original B owner and three scoped credentials for A’s source, the send into B, and B’s receive. Submit the paired source authority with A’s grant and a separate empty source-owner slot. |
| Serve | The source owner drives the relay. B signs its own destination-journal receipt. The application’s Receive and ACK are separate exchanges. |
| Retire | Stop the admitted VM and collect the original helper. Loss of a slot or a process ID cannot establish retirement. The durable ownership marker survives uncertain exit. |

The SDK rejects mismatched roles before boot. During a paired start it reserves the machine name among paired launches, checks original helper custody, and uses MVM’s lifecycle-aware cleanup before preparing the runtime. A failed resource-grant or host-budget check requests rollback from the backend that started the VM. If rollback fails, the SDK retains the volume leases and records the teardown failure with the audit session open.

## Recorded checks

On Apple Silicon macOS with Rust 1.97.1, 38 persistent SDK tests passed in 2.34 s and one mailbox ownership/cleanup test passed in 0.02 s. The latter exercised an unresolved original marker, a held VM lifecycle lock, successful cleanup after release, and a competing paired start. The SDK tests covered the three legal role combinations, ordinary boot refusal for a directed grant, and failing grants and budget accounting with both confirmed and uncertain abort. [Named tests and source identities](results.json)

These times measure test suites. This SDK test did not transfer a payload. The real Linux measurements for a guest reading its own mailbox and for two helper processes speaking to fixture guests have their own records. The two-guest installed test must carry A’s payload through B’s own reader and verify B’s acknowledgment and original-owner cleanup. [Own-mailbox native result](../own-mailbox-native/verified-own-mailbox.md) · [Paired helper measurements](../two-helper-journal/README.md) · [Installed delivery tracker](https://github.com/R4-Laboratories/SpiralCore/issues/168)

## Reproduce

Use the contributor commit linked above, or apply the patch to its parent `f17ddafd3749bf0dcc24e68b3386d07133aa86aa`. The resulting Git tree is `f7c9e749fe85f499bc4515f8e902e3e9d4507c76`. Run with the locked dependencies available locally.

```sh
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0
cargo +1.97.1 test --locked --offline -j2 -p mvm-client --features test-support --lib persistent -- --test-threads=1
cargo +1.97.1 test --locked --offline -j2 -p mvm-hostd --lib paired_sdk_reap -- --test-threads=1
```
