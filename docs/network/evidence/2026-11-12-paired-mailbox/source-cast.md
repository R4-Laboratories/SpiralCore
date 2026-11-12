# Addressed Cast from source preparation to exact local readback

The sender borrows the original mailbox worker’s signer and sequence. It verifies the signed permissions for A to send and B to receive, then seals the application body from A’s Submit request in MVM’s existing envelope. While the original worker retains the request, a reconnect can reuse the same ciphertext. A changed request, replaced signed plan, tighter frame budget, expired credential or revoked Receive grant cannot release that ciphertext. A source-role test also checks that a valid issuer signature cannot move a legacy worker into another tenant or authorization domain. [Sender](https://github.com/Casey-allard/mvm/blob/32af78af211da2c331553014d00134808e9a009b/crates/mvm-core/src/mailbox/frame/cast/source.rs) · [Original worker adapter](https://github.com/Casey-allard/mvm/blob/32af78af211da2c331553014d00134808e9a009b/crates/mvm-hostd/src/mailbox_service/cast_source.rs)

## Observed transfer

One macOS arm64 test sent the canonical Send credential and sealed envelope across a Unix socketpair. The receiver verified and decrypted the envelope, delivered it over a second socketpair, and returned the peer acknowledgment after the guest-role thread checked all five bytes, `00 41 ff 42 00`. The five-byte input exercises binary fidelity. This run used no compression or warm-cache baseline. [Fixture](https://github.com/Casey-allard/mvm/blob/32af78af211da2c331553014d00134808e9a009b/crates/mvm-core/tests/mailbox_cast_source/check.rs) · [Nanosecond measurements, source and binary identities](source-cast.json)

| Measurement | macOS arm64, one debug-profile transfer |
|---|---:|
| Source → receiver / receiver → source | 4,450 / 11 bytes |
| Receiver → local inbox / inbox → receiver | 276 / 98 bytes |
| Initial authorization and sealing | 1.474 ms |
| From initial preparation through peer acknowledgment | 6.358 ms |
| Source / receiving worker / guest-role thread CPU | 1.638 / 4.392 / 0.038 ms |

The two socketpairs represent separate hops. The peer counts include the full credential, length fields and reply. The local role counts include greeting and acknowledgment. Preparation is part of elapsed time. Socket setup, boot, cross-host transport and durable journal acceptance fall outside this measurement. The retry test retains the first envelope across a new source borrow and a failed local disposition callback. Worker restart and two-guest delivery need installed-service tests. [Earlier destination-only measurement](addressed-route.md) · [Installed two-guest own-mailbox measurement](firecracker.md)

## Reproduce

At MVM contribution revision `32af78af211da2c331553014d00134808e9a009b`, run the following on macOS with `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_INCREMENTAL=0`. The fixture prints both socket counters and thread CPU times. Envelope length can vary with randomized, canonically encoded cryptographic fields.

```sh
cargo test --locked -j2 -p mvm-core --no-default-features --features test-support \
  --test mailbox_cast_endpoint --test mailbox_local_frames --test mailbox_submit_exchange -- --nocapture
cargo test --locked -j2 -p mvm-hostd --lib mailbox_service::
```

This contribution contains upstream MVM `8fd62a0d`, including its rustix dependency update. It also carries the exact epoch-7 envelope refusal under an epoch-8 receiver from the proposed rebuild handoff. A sender intent, a destination-verified envelope and a destination-signed durable receipt have distinct owners. Directional key installation in both original workers, destination workload readback across booted MVMs and the canonical WAN run remain open. [Upstream dependency update](https://github.com/tinylabscom/mvm/pull/5371) · [Rebuild handoff](https://github.com/tinylabscom/mvm/pull/5304) · [Ari’s coordination thread](https://github.com/tinylabscom/mvm/pull/5328#issuecomment-6815760571)
