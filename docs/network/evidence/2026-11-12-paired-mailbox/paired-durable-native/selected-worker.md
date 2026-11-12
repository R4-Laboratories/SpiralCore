# Installed Linux mailbox worker for Object-v1

The selected Firecracker profile now has a runnable host helper. The executable accepts both original VMM process descriptors and two anonymous Unix stream sockets from its owning parent over a private control socket when invoked as `mvm-mailbox --original-object-endpoint-v1`. Launching the helper from the installed SDK is a separate integration step. One stream carries this helper’s outbound Call or Reply and its signed destination verdict. The other carries the opposite endpoint’s request and this helper’s verdict. The guest application opens its in-guest reader Unix socket. The host helper dials the original guest agent over vsock port 5257. The operator’s selected pair, current signed plans and enrolled roles determine each endpoint. [MVM contributor source](https://github.com/Casey-allard/mvm/commit/1ecf1e761812e89aaae79bdb3deb1ac59ab9fea4).

Before sending, the helper persists the exact sealed canonical envelope and charges the attempt. B’s application Reply requires its original live Call lease. A’s journal verifies that the incoming Reply corresponds to A’s original Call. Each sender independently checks the returned V2 local-durability receipt before reporting acceptance to its guest. A guest reader has separate possession and ACK operations. Disk operations on the journal release its mutex before either worker waits for a remote receipt. A partial or unreadable frame closes that peer stream. The original source record remains available for reconciliation on a newly owned connection.

## Linux x86-64 checks

| Scope | Result |
|---|---|
| Selected helper | 8/8, including an exec child detecting original controller disconnect, rejection of swapped/stale VMM identities, both anonymous data sockets, frame boundaries and directional byte accounting |
| Signed pair and retained sender | 4/4, including a five-byte Call submitted twice after a lost response, identical ciphertext and delivery ID, refusal of the wrong boot before transport, and retryable local contention |
| Original source intent | 3/3 |
| Guest stream recorder | 4/4 |
| Executable entry point | Refused ordinary file stdin, exited unsuccessfully and produced zero stdout |
| Build | Linux default profile checked. Witness-enabled debug executable SHA-256 `b4804334262f883346a3dff3329317f6fc1f43e84cd33ab53642fc08c8416e56` |

The deadline regression initially returned an untyped error for a current grant whose I/O deadline had elapsed. The fixed path returns `Unavailable`, preserving the retained seal for retry. Expiry of its actual authorization window returns `Expired`.

The meter check used a six-byte request and seven-byte response on real connected sockets. Including each four-byte length prefix, the source wrote 10 bytes and read 11. The destination read 10 bytes and wrote 11. Summing both endpoint copies would double-count traffic. This control exercises the byte counters with synthetic records. It yields no compression speed measurement.

The guest port counter remains in `mailbox-guest-stream-v1.counts`. A witness build also emits `object-sealed-out-v1.counts` and `object-sealed-in-v1.counts` under each original VM state directory. For each complete exchange, `end` records the sequence, bytes read, bytes accepted by writes, read errors, write errors and overflow flag. Compare crossed source/peer counts, require the closing `finish` row for full totals, and report any incomplete capture. These are same-host socket bytes. Wire overhead and WAN transport require separate counters.

```sh
cargo test --locked -p mvm-hostd --features durable-guest-io-witness --lib mailbox_process::object_worker
cargo test --locked -p mvm-hostd --features durable-guest-io-witness --lib mailbox_launch::object_pair
cargo test --locked -p mvm-hostd --features durable-guest-io-witness --lib object_intent
cargo test --locked -p mvm-hostd --features durable-guest-io-witness --lib witness_io
cargo build --locked -p mvm-hostd --features durable-guest-io-witness --bin mvm-mailbox
cargo check --locked -p mvm-hostd --bin mvm-mailbox
```

This run launched zero guests. The acceptance run must boot the two original guests, match their selected image identities, persist and reopen the cold object, verify its exact repeat on B’s same worker, and collect both original helpers. Only that run can supply installed bytes, CPU and elapsed time for this profile. [Acceptance tracker](https://github.com/R4-Laboratories/SpiralCore/issues/202).
