# Send an admitted Cast to the local MVM journal

`send_cast` maps an existing SpiralCore v1 source binding onto the trusted-host local durability tier in [MVM #5377](https://github.com/tinylabscom/mvm/pull/5377). This host component runs on Unix. The destination is the mailbox in the original signed Send capability. The signed direct route selects an absolute Unix socket with a `unix:` locator. This mapping requires distinct sender and recipient principals, a Cast, and the admitted guest-message v1 content type.

1. At A’s authorized plaintext endpoint, encode a SpiralCore `Frame`, issue the original `MessageOptions` and call the existing `service_sender::prepare` once. Retain the returned `PreparedMailboxEnvelope` while retrying. A process restart needs separate original-envelope custody.
2. Pass the binding, original envelope, trusted clock sample and an absolute deadline to `send_cast`. The source verifies the signed Send grant, authenticated header, current policy and transport route. It retains its policy generation through this exchange. The effective deadline is the earliest of the supplied deadline, five seconds and the signed authorization expiry.
3. The adapter decodes the already sealed envelope with Ari’s pinned Rust types, checks byte-for-byte agreement of its encoding and the signed Send credential, and calls MVM’s `send_local_receipted_frame`. The returned `Exchange` carries the original signed V2 receipt and counts bytes actually read and written on that socket. Ambiguous failures leave A’s original envelope available for retry. This API does not retire a guest outbox.
4. The original B journal opens and admits the envelope and issues its own receipt. Its authority public key comes from the trusted source binding. In this tier B signs as the admitted mailbox authority. A reader needs separate Receive authority. A journal receipt establishes local durable admission. Readback, an explicit reader ACK, and application processing have separate lifecycles.

The original SpiralCore adapter remains pinned to MVM `ef1067e3`. The V2 sender and journal use `543efc7f`. No second encryption, message ID, receipt or journal is introduced at their boundary. The source endpoint sees its original authenticated header. The carrier receives the existing sealed envelope plus signed Send grant. The test’s two endpoint policies use synthetic keys in one trusted process. Production key provisioning, broker admission and independent guest lifecycle remain integration responsibilities.

Run the native regression from the SpiralCore checkout.

```sh
cargo test --locked --manifest-path crates/spiralcore-mvm-journal-host/Cargo.toml \
  --test addressed -- --nocapture
```

The test starts B’s canonical Unix journal service, fences a stale destination policy, verifies A’s original signed receipt, drops B, observes an unavailable endpoint, reopens the same journal and sends the same envelope. B returns the byte-identical receipt. A separately authorized host reader verifies the original frame, acknowledges it and finds the inbox empty. The checks also cover a foreign reader, a mismatched source recipient and a revoked route. [Recorded socket measurements](../../docs/network/evidence/2026-11-12-paired-mailbox/durable-host-binding/README.md) · [Installed delivery tracker](https://github.com/R4-Laboratories/SpiralCore/issues/168)
