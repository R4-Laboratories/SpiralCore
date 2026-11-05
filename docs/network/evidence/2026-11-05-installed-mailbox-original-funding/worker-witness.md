# Installed mailbox worker and guest witness

Use the funded original mailbox attachment from the opaque selected Hostd
admission. Move it to the original Runtime slot before any listener or thread
effect. Keep native selection closed until all gates below qualify.

## Host Unix socket witness

Use the adapter's executable example
`core-1025/crates/spiralcore-mvm-adapter/tests/service_tcp.rs` test
`canonical_calls_and_encrypted_receipts_cross_tcp_without_recursive_traffic`.
Use its canonical sequence over the actual installed owner's private
filesystem socket. Do not pull campaign recovery or object transfer logic
into the worker.

- Use `live::StreamTransport<UnixStream>` for SCMVS001 framing and the
  existing bounded length-prefixed canonical envelopes. The handshake is a
  discriminator. `MailboxBinding` and signed route/capability admission
  authenticate each message.
- Share one resident `BrokerContext` and its HostBudget. Keep socket I/O
  outside `with_time` dispatch. Reuse `service_sender::prepare`,
  `LocalMailboxService::register_call` and `receive`,
  `ReceiptReturnConfig::admit`, `reply_prepared`,
  `MailboxBinding::send_prepared` and `ReceiptTracker`. Prove one exact
  request/Reply and encrypted receipt Casts, then one separately granted
  application Cast. Foreign and revoked grants refuse before delivery.
- Each worker owns an independent accept wake and an original duplicate of
  the accepted socket before the handshake. A blocking StreamTransport I/O
  holds its state mutex, so calling its close method cannot be the only
  wakeup. Stop with partial prefix and partial ciphertext outstanding.
  Require positive join and exact socket identity cleanup before refund.
- Limit actual read and write bytes with a shared Standard rate. Reuse
  canonical profile values and partial-I/O refund discipline. Bound queue
  slots and bytes, explicit thread stack, socket/parser/ciphertext buffers,
  and any plaintext/object workspace in the same host and tenant ledger.
  Waits must wake on cancellation without relying on wall-clock sleep.

## Installed signed guest witness

Preserve canonical MVM stage0 and agentd. Add a static workload client inside
the authenticated rootfs. It dials HOST_CID/MAILBOX_PORT as the existing
development `guest.rs` client does and uses the same canonical adapter
exchange. Start with synthetic one-use credentials in a private,
authenticated fixture artifact whose attachment the consuming launch
actually verifies. Never replace the verified initrd or use the raw
plan_binding=None development boot as installed evidence.

The current installed endpoint spawner refuses requested secrets, so an
actual supported bootstrap carrier is a prerequisite. Bind the canonical
tenant, domain, principals, route, boot identity, policy and capability
epochs and trust keys to the selected original admission. `Pair::provision`
is a test authority that invents its own domain and development issuer.
Do not use it as production authority. Before native launch, also prove
host worker readiness and cancellation, unchanged independent FlowMux
authorization, actual bidirectional rate, and original grant retirement.
