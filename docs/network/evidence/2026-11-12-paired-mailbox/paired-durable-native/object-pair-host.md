# Provisioning the two original object mailboxes

Each endpoint supplies a separately signed credential to receive its own mailbox. B also authorizes the original Call from A, and A authorizes a correlated Reply from B. The host verifies both signed plans, boot identities, grants and validity windows before deriving the two direction-specific keys and signing the guest reader descriptions. An immutable retained intent reopens with its original ciphertext, delivery ID, application IDs, sequence and exact body. B can seal a Reply only for a matching active signed lease in B’s journal. [Reviewed contributor source](https://github.com/Casey-allard/mvm/commit/9fa12ca962c9f7f5d5b42c61a9e7f265ea8602a8) · [Measurements and hashes](object-pair-host.json)

The first receipt test accepted a genuine V2 for a different B boot using the same grant and execution epoch. On the corrected source the host checks the direction key ID, derived with both original boots, and the time at which the destination signed its acceptance. Two of three tests passed before this correction. All three passed after it. Six retained-custody tests, sixty core mailbox tests and the Linux SDK compilation also passed.

The test used real signatures, authenticated encryption and a local journal with an anchor. It sealed and reopened a 5-byte Call and a 7-byte Reply including trailing zero bytes. A separate single-test invocation took 0.01 s wall, 0.01 s user and 0.00 s system time at GNU time’s 0.01 s reporting precision, with 35,412 KiB peak RSS. These figures cover fixture construction, storage, verification and refusal cases in one unoptimized test process.

No guest was booted. The measured component had no relay sockets and produced no network byte count or transfer-speed result. At this revision only the original macOS HVF launch exposes the native owner required by the new provisioning API. The installed selected listener, Linux cold-launch handoff and actual two-guest cold/repeat transfer remain open. The earlier installed 4 KiB Cast measurement used full transmission. [Installed 4 KiB results](size-control/README.md) · [Delivery tracker](https://github.com/R4-Laboratories/SpiralCore/issues/202)

## Reproduce on Linux

Use the pinned MVM source, Rust 1.97.1 and its Cargo.lock. The reported run used two Cargo jobs, incremental compilation disabled and debug information disabled for dev and test.

```sh
cargo +1.97.1 test --locked --offline -p mvm-hostd --lib mailbox_launch::object_pair
cargo +1.97.1 test --locked --offline -p mvm-hostd --lib mailbox_custody::
cargo +1.97.1 test --locked --offline -p mvm-core --features test-support --lib mailbox::
cargo +1.97.1 check --locked --offline -p mvm-client
```
