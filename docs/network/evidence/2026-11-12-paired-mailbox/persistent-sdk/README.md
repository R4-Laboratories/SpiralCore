# Starting a paired journal with the persistent MVM SDK

An orchestrator can now start a stopped persistent machine with one explicit mailbox role using `LocalBackend::start_persistent_with_journal`. The SDK carries the original authority into signed-plan admission and retains the helper it started in the caller’s slot. The application continues to use MVM’s existing mailbox. [Contributor implementation](https://github.com/Casey-allard/mvm/commit/10b8b6b4406b8aba31a2ce0233669739b0a6e86c) · [Rust patch](persistent-sdk.patch)

| Step | What the orchestrator and SDK do |
| --- | --- |
| Define | Create B’s and A’s persistent machine definitions through MVM. Each journal grant applies to one boot. |
| Start B | Pass B’s mailbox grant, its enrolled incoming policy, and an empty destination-owner slot. The SDK verifies the signed production plan and starts B’s original journal worker. |
| Start A | Supply the live original B owner and three scoped credentials for A’s source, the send into B, and B’s receive. Submit the paired source authority with A’s grant and a separate empty source-owner slot. |
| Serve | The source owner drives the relay. B signs its own destination-journal receipt. The application’s Receive and ACK are separate exchanges. |
| Retire | Stop the admitted VM and collect the original helper. Loss of a slot or a process ID cannot establish retirement. The durable ownership marker survives uncertain exit. |

The SDK rejects mismatched roles before boot. Ordinary and paired SDK starts acquire a common lock before changing the machine definition or runtime. A 12-bit hash selects one of 4,096 nonblocking locks. Different names that select the same busy lock can retry after a conflict. Fresh transient names leave no per-name mailbox key directory. Paired starts also verify original helper custody and claim any separate short socket directory by exclusive creation. A failed resource-grant or host-budget check requests rollback from the backend that started the VM. If rollback fails, the SDK retains the volume leases and records the teardown failure with the audit session open.

## Recorded checks

On Apple Silicon macOS with Rust 1.97.1, 217 launch tests passed in 13.49 s and 11 original-helper tests passed in 0.20 s. Both selectors skipped one test. One needs the pinned Linux kernel and one needs the separately built Linux helper executable. Tests covered refusal of an ordinary or transient start under a held reservation, bounded lock paths across 128 transient names, independent retention of runtime state, and failed grants/budget accounting with confirmed and uncertain abort. An intermediate build returned the wrong SDK error class for a concurrent ordinary create. The tested revision returns the existing conflict class. [Recorded checks and exclusions](results.json)

These times measure test suites. This SDK test did not transfer a payload. The real Linux measurements for a guest reading its own mailbox and for two helper processes speaking to fixture guests have their own records. The two-guest installed test must carry A’s payload through B’s own reader and verify B’s acknowledgment and original-owner cleanup. [Own-mailbox native result](../own-mailbox-native/verified-own-mailbox.md) · [Paired helper measurements](../two-helper-journal/README.md) · [Installed delivery tracker](https://github.com/R4-Laboratories/SpiralCore/issues/168)

## Reproduce

Use the contributor commit linked above, or apply the patch to its parent `f17ddafd3749bf0dcc24e68b3386d07133aa86aa`. The resulting Git tree is `cdc278074e0adfe3b55024f1ac9aa6082a5156bb`. Run with the locked dependencies available locally.

```sh
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0
cargo +1.97.1 test --locked --offline -j2 -p mvm-client --features test-support --lib launch:: -- --test-threads=1
cargo +1.97.1 test --locked --offline -j2 -p mvm-hostd --lib mailbox_process::original_journal -- --test-threads=1
```
