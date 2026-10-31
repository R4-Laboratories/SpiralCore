# SpiralCore canonical MVM mailbox adapter

This standalone Rust crate carries existing SpiralCore object-transfer frames
through the real mailbox cryptography and transport authorization in MVM revision
`3104a12c5b071a6851323fa8115d2afaf3f0bb8b`. The transfer engine, codecs, warm
selection and on-disk object format are unchanged. It is a host conformance
integration; it does not yet connect running MVM guests over vsock.

`Direction::send` seals and signs a bounded frame, checks current capability and
signed route authorization, and queues it through MVM's `PeerTransport`.
`Direction::open` authenticates/decrypts an envelope, rejects replay, validates the
application frame, and returns a private `VerifiedFrame`. Only this successful
open installs a `MailboxAuthority` lease. `VerifiedFrame::dispatch` checks the
policy generation again before invoking the existing durable object endpoint.
Receive-only directions need the sender's public verification key, never its
private signing key. `Policy::replace` invalidates earlier leases and proofs.

The profile permits one at-most-64-KiB application frame per at-most-256-KiB
canonical envelope, independent call/reply capabilities, and at most 4,096
process-local replay identities per direction. Replay capacity fails closed;
entries are not evicted to admit new messages. Applications must not recreate a
direction to bypass this bound. A production lifecycle needs durable replay and
epoch/boot coordination, current policy/time refresh, key provisioning and mailbox
rate/admission enforcement.

The opt-in `conformance` feature supplies synthetic identities, ephemeral keys
and MVM's actual bounded `InMemoryPeerTransport`. It is excluded from the default
library. The fixture's time and epochs are fixed for reproducibility; it does not
model clock-driven expiry, a guest service or durable mailbox admission.

## Run

Use Rust 1.97.1 and the retained standalone lockfile:

```sh
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml \
  --all-targets --features conformance,hardware-sha256
cargo +1.97.1 build --release --locked \
  --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml \
  --example mailbox_transfer --features conformance,hardware-sha256
SPIRALCORE_SOURCE="$(git rev-parse HEAD)" \
  crates/spiralcore-mvm-adapter/target/release/examples/mailbox_transfer \
  /tmp/spiralcore-mvm-new-store /tmp/spiralcore-mvm-new-results.jsonl
```

Both example paths must be new. If `CARGO_TARGET_DIR` is set, use the executable
under that target directory. The example retains any failed session, verifies
every original byte after reopening each successful store, then removes only its
own freshly created stores. It compares persistent warm and prepared whole-object
zstd at 1 MiB and 16 MiB, records actual canonical-envelope bytes in both directions
and combined host CPU/elapsed time, and charges setup/seeding separately. There
is no network-send or WAN-time estimate. See the
[registered measurement contract](../../docs/network/evidence/2026-10-31-mvm-readiness/measurement-contract.md).

## Remaining guest integration

MVM's [dedicated mailbox vsock service (#4629)](https://github.com/tinylabscom/mvm/issues/4629)
and [approved Iroh transport (#4633)](https://github.com/tinylabscom/mvm/issues/4633)
were open at the pinned integration check. The existing FlowMux port 5253 is not
a mailbox port. Wire this adapter into the canonical plaintext endpoint once the
guest service contract exists, then demonstrate one guest/host, two guests under
a small orchestrator, two hosts, and three-peer recovery.

The pinned `open_envelope` returns authenticated payload only. A general call
broker needs an upstream verified header/reply handle, durable inbox/replay and
admission semantics. The conformance harness controls both directions and checks
application transfer/sequence IDs; this is not a verified reply-routing API.
Object publication, mailbox admission and application processing are separate
acknowledgments. Application retries can use a fresh envelope with the same
transfer/sequence; this provides idempotent publication, not exactly-once execution.

Policy changes invalidate pending proofs, but the existing object-store authority
hook does not provide a concurrent revocation/publication transaction. Before a
live deployment, define and enforce that boundary with the mailbox service. No
production security, CUI compliance or FIPS validation is claimed by this crate.
