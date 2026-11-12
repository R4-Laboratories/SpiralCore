# SpiralCore guest mailbox binding

This Rust crate moves one existing SpiralCore `SCMOBJ01` frame as one application message through the Linux workload's **own MVM mailbox**. It targets MVM draft #5377 at `543efc7f94aa756c09785b38bdd024b6ba8199c7`. The host must admit the mailbox and its trusted-host local-durable retention policy. Guest operations use MVM's workload outbox and local reader service. [MVM writer example](https://github.com/tinylabscom/mvm/blob/543efc7f94aa756c09785b38bdd024b6ba8199c7/crates/mvm-agentd/examples/mailbox_writer_client.rs) · [MVM reader example](https://github.com/tinylabscom/mvm/blob/543efc7f94aa756c09785b38bdd024b6ba8199c7/crates/mvm-agentd/examples/mailbox_reader_client.rs)

| Operation | Application contract |
|---|---|
| `PendingFrame::new(&frame)` | Encodes a SpiralCore frame and assigns one MVM message ID. Persist the ID and exact encoded bytes together before first submission if the application must recover after restart. `restore` accepts those two values. |
| `pending.stage()` | Calls MVM's writer at `/run/mvm/mailbox/outbox-local`. Success confirms staging. MVM's agent verifies the journal's signed V2 local-durable receipt before clearing the pending item. Keep the same frame and ID on retry. |
| `receive()` | Requests one delivery from the boot-bound application reader at `/run/mvm/mailbox/receive.sock`. An empty mailbox returns `Ok(None)`. A successful delivery contains the decoded frame and its original lease. |
| `delivery.acknowledge()` | Explicitly submit the lease after the application commits its work. Keep the delivery after an ambiguous I/O result. The agent owns signing and correlation. Application processing should be idempotent on redelivery. |

The receiver should submit the frame to its normal authenticated SpiralCore endpoint using the correct transfer context. Frame decoding validates the wire representation. Object length, identity, capability checks and durable reconstruction remain the responsibility of that endpoint. The binding performs no automatic ACK. Each reader operation uses one fresh AF_UNIX connection and a 12-second monotonic deadline. The workload uses its admitted Unix stream and ordinary readiness polling. Cryptographic keys, socket admission and the signed receipt stay in MVM.

Cross-guest routing requires a separate directed grant and the named destination's receipt authority. A successful own-mailbox delivery cannot qualify a route from guest A to guest B. The Linux component result and remaining delivery gates are recorded in [the paired-mailbox tracker](https://github.com/R4-Laboratories/SpiralCore/issues/168).

## Trust and file ownership

The binding assumes the VM's trusted agent provisioned and controls the reader socket path and that its same-workload outbox namespace stays intact during a call. On the durable path MVM uses one immutable request file. Existing incompatible content is refused, and retrying the staged ID with identical bytes succeeds. The public upstream writer also supports directory spools and selects its operation by inspecting the path. The binding checks the file type before calling that writer. Those observations cannot authenticate a namespace concurrently replaced by code with the same guest UID. The receive side relies on the original local agent for signed lease/ACK validation. Independent verification of a remote destination receipt requires its separately enrolled authority.

## Reproduce the guest component test

Run on Linux x86_64 with Rust 1.97.1. The test installs MVM's own workload seccomp program in the application thread. It uses disposable filesystem paths and a synthetic reader peer. The fixture needs no VM, guest NIC or root.

```sh
cargo test --locked --manifest-path crates/spiralcore-mvm-guest/Cargo.toml --lib -- --nocapture --test-threads=1
```

[Recorded Linux bytes, timings and qualification scope](../../docs/network/evidence/2026-11-12-paired-mailbox/durable-guest-binding/README.md)

## Single-guest warm workload

Install the static Linux executable at `/usr/local/bin/spiralcore-durable-own-mailbox` in an MVM workload image. Admit the trusted-host local-durable mailbox policy, its guest reader, one warm worker, and the normal private guest tmpfs. The caller supplies one canonical encoded SpiralCore frame in `WorkerCallRequest.stdin`, with an empty environment override list and a 45-second call budget. This fixture accepts an encoded frame of at most 16 KiB so the base64 payload and signed lease fit the upstream reader's 64 KiB JSON response limit.

The workload reserves `/tmp/spiralcore-durable-own-mailbox` for one invocation per fresh guest boot and stages the frame to its own mailbox. It compares the received message ID and full encoded frame with the staged original. Before acknowledging the delivery it writes the received body to a private file, syncs the file and its directory, reopens it and compares every byte with the original body. The file remains in guest tmpfs until teardown. After a failed operation a second invocation in that boot refuses to restage. An interrupted ACK can leave the upstream outcome uncertain.

A successful worker response contains compact JSON with the message ID, SHA-256 of the body, exact body and frame lengths, completed application socket bytes in both directions for Receive and ACK, empty-poll count, per-phase elapsed times, workload CPU time, and wall time. These clocks start inside the workload. Guest boot, host journal CPU, vsock traffic and any WAN carrier require separate measurements.

```sh
cargo test --locked --manifest-path crates/spiralcore-mvm-guest/Cargo.toml --all-targets
cargo zigbuild --locked --release --target x86_64-unknown-linux-musl \
  --manifest-path crates/spiralcore-mvm-guest/Cargo.toml --bin spiralcore-durable-own-mailbox
```

[Static binary and component measurements](../../docs/network/evidence/2026-11-12-paired-mailbox/durable-guest-binding/own-workload.json)
