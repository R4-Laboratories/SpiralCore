# Starting a paired journal with the persistent MVM SDK

An orchestrator can now start a stopped persistent machine with one explicit mailbox role using `LocalBackend::start_persistent_with_journal`. The SDK carries the original authority into signed-plan admission and retains the helper it started in the caller’s slot. The application continues to use MVM’s existing mailbox. [Contributor implementation](https://github.com/Casey-allard/mvm/commit/8b5e86b2d822cfecc03b2288d554a8bd953a0c78) · [Rust patch](persistent-sdk.patch)

| Step | What the orchestrator and SDK do |
| --- | --- |
| Define | Create B’s and A’s persistent machine definitions through MVM. Each journal grant applies to one boot. |
| Start B | Pass B’s mailbox grant, its enrolled incoming policy, and an empty destination-owner slot. The SDK verifies the signed production plan and starts B’s original journal worker. |
| Start A | Supply the live original B owner and three scoped credentials for A’s source, the send into B, and B’s receive. Submit the paired source authority with A’s grant and a separate empty source-owner slot. |
| Serve | The source owner drives the relay. B signs its own destination-journal receipt. The application’s Receive and ACK are separate exchanges. |
| Retire | Call `LocalBackend::stop_persistent_journal` with the original owner. Confirm the original VMM’s kernel exit, collect that helper, release VM resources and then clear the durable marker. The stop outcome records whether helper termination required assistance. |

The SDK rejects mismatched roles before boot. Ordinary and paired SDK starts acquire a common lock before changing the machine definition or runtime. A 12-bit hash selects one of 4,096 nonblocking locks. Different names that select the same busy lock can retry after a conflict. Fresh transient names leave no per-name mailbox key directory. Paired starts also verify original helper custody and claim any separate short socket directory by exclusive creation. A failed resource-grant or host-budget check requests rollback from the backend that started the VM. If rollback fails, the SDK retains the volume leases and records the teardown failure with the audit session open.

The installed helper captures the original VMM’s kernel identity and arms an exit observer before spawning. Polling an exited helper cannot clear its marker while the VM or cleanup remains unresolved. Stop verifies the original plan, nonce, state directory and short socket directory before removing runtime files. A failure to execute the helper releases a proved unused claim. If execution began, the caller retains the actual owner through subsequent failure or timeout.

## Recorded checks

| Platform and source | Executed checks | Result |
| --- | --- | --- |
| Apple Silicon macOS, `3b77224ed` | SDK launch suite | 217 passed, one Linux kernel case skipped, 13.40 s |
| Apple Silicon macOS, `3b77224ed` | Original journal helper suite | 14 passed, one Linux two-helper case skipped, 0.54 s |
| Linux x86-64, `8b5e86b2d` | Eight focused selectors, including two real helpers through fixture guest sockets | 24 passed, including the two-helper case in 0.61 s |

The Linux source adds one test to the Apple-tested source. The product code is identical. The suites exercised ordinary/paired launch contention, bounded state across 128 transient names, rollback when grants or budget accounting fail, and keeping ownership until the original process exits and cleanup succeeds. An intermediate build returned the wrong SDK error class for a concurrent create. The released code returns the existing conflict class. [Named checks, source identities and executable hashes](results.json)

These timings measure test suites. The installed two-Firecracker-guest trial used a separately pinned host and guest. It delivered 257 exact bytes from A to B and verified B’s acknowledgment. Both helpers required assisted termination. The issue tracks independent verification of graceful retirement. [Installed trial and delivery tracker](https://github.com/R4-Laboratories/SpiralCore/issues/168) · [Single-guest result](../own-mailbox-native/verified-own-mailbox.md) · [Two-helper measurements](../two-helper-journal/README.md)

## Reproduce

Use the contributor commit linked above, or apply the patch to its parent `f17ddafd3749bf0dcc24e68b3386d07133aa86aa`. The resulting Git tree is `bf07e16ada6c4470068e5cc6f0288b15069eaabd`. Run with the locked dependencies available locally.

```sh
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0
cargo +1.97.1 test --locked --offline -j2 -p mvm-client --features test-support --lib launch:: -- --test-threads=1
cargo +1.97.1 test --locked --offline -j2 -p mvm-hostd --lib mailbox_process::original_journal -- --test-threads=1
```
