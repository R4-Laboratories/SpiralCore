# Readback and ACK for a Reply-only credential

The selected durable object profile grants B the Reply operation on A’s mailbox. The earlier journal path reopened every retained message under a Send operation. On Linux the original destination accepted and retained a valid Reply, then A’s journal reader returned `Mailbox(OperationNotAuthorized)` on first lease creation. [Earlier guest and B journal checkpoint](README.md)

The typed journal methods now select the original retained credential from the checked pair. For B’s Call they require the original Send credential. For A’s Reply they require the original Reply credential and the pending Call under which B was authorized to reply. Before leasing or recording an ACK the journal checks both original boots, time window, policy epochs, current revocations, the exact credential stored with the row, the independent Receive grant and the guest’s active signed operation. The generic guest ACK refuses an explicitly selected paired grant. [Qualified journal source](https://github.com/Casey-allard/mvm/blob/754219216f3e119b4220587f995a7f2d9def99dd/crates/mvm-core/src/mailbox/journal.rs) · [Regression](https://github.com/Casey-allard/mvm/blob/754219216f3e119b4220587f995a7f2d9def99dd/crates/mvm-core/src/mailbox/object_pair/tests/leased_projection.rs)

## Executed boundary

The test seals the original Call and its correlated Reply under distinct directional keys. A real destination journal and file anchor accept the Reply. A obtains the four exact bytes `6f 62 6a 00` with a signed lease and recovers the identical lease, header and payload after close/reopen. Attempts to ACK with an unselected guest method, a differently correlated Call, a missing original Call, or revoked Reply credential must leave the journal tip unchanged. The matching signed ACK advances it once. Reopening A’s journal and replaying that exact ACK must return the same tip.

This is an in-process Linux journal test on actual storage. No socket traffic is generated. It establishes neither guest transfer speed nor compression efficiency. Wiring this method into the installed server, exposing only the selected guest operations, and running the cold and warm application protocols across both original guests remain on [#202](https://github.com/R4-Laboratories/SpiralCore/issues/202).

## Reproduction

On Linux x86-64, Rust 1.97.1, run the following from the pinned MVM workspace. The first command isolates the newly reproduced failure on the test-only predecessor. On the fix the broader second selector passed 61/61 in 0.01 s, including B’s existing Call and A’s Reply. This is the test harness’s rounded wall time for the suite. No isolated transfer CPU measurement was taken. [Recorded revisions, executable and logs](reverse-journal.json)

```sh
cargo test --locked -p mvm-core --lib reply_only_lease_reopens_under_original_call_and_requires_signed_ack
cargo test --locked -p mvm-core --lib mailbox::
```
