# Durable object Call/Reply enrollment

A verified warm-object offer requires an application reply from the receiver. The installed paired Cast profile gives A authority to send a complete fixture to B. B’s signed journal receipt establishes durable acceptance of that envelope. This contributor change defines the separately selected authority needed for the receiver to answer an object offer. Ordinary signed plans and boot records omit the selection and retain their existing serialization. [Installed 4 KiB trial](size-control/README.md) · [Earlier authorized warm component](object-reply-boundary.md) · [SpiralCore #202](https://github.com/R4-Laboratories/SpiralCore/issues/202)

## Operator inputs and message flow

1. Reserve both original plan nonces. B’s mailbox authority issues A an Ed25519-signed canonical Send capability restricted to Call and B’s address. A’s mailbox authority independently issues B a Reply capability restricted to Reply and A’s address. Both are limited to `application/spiralcore-object` version 1. The workload sender principals on this profile are their verifying-key bytes and must be the corresponding peer’s named recipients. Operators supply both credentials, same tenant/domain, distinct mailbox identities, validity, limits, revocation IDs and exact epochs. Existing Cast templates have different semantics and cannot stand in for either credential.
2. Hash the profile domain, the fixed-length lowercase hexadecimal representation of the two nonces in caller/callee order, and the SHA-256 digest of each canonical signed credential in forward/return order. Select this commitment in both signed plans. After admission, check both signatures with the respective installation’s trusted plan keys, content-addressed plans, finite retention and original boot projections. The original owner supplies each live boot identity, execution epoch, policy epoch and current revocations again on every turn.
3. A’s authenticated Call names its precise return mailbox and Reply capability. A retained, fixed-size correlation record binds A’s message ID, correlation ID, lifetime, profile and both original boots. Validate B’s actual signed, encrypted Reply under its own grant, including its message kind, recipient, original boot, correlation, causation and expiry. Check replay after correlation so a mismatched reply cannot consume the valid reply’s replay entry. The check exposes no plaintext and issues no key or receipt.
4. An installed service still needs separately admitted directional keys, the two destination reader/Receive enrollments, verified header-to-lease binding, durable outstanding-call accounting and the existing SpiralCore sender/receiver at each authorized plaintext endpoint. The contributing implementation explicitly refuses this selector at ordinary host admission, key derivation and guest listener startup. Enabling the service requires positive installed execution and lifecycle evidence. [MVM implementation and tests](https://github.com/Casey-allard/mvm/commit/b3868edc9d307f450dde9a4de0e17b7f43969b0e)

## Qualification

The original installed-source release is `8238000662f59893b6ec703ab4b37d7a86c08340`. The regression on that source failed as expected when the previous parser encountered the new signed selector. The Linux positive uses two signed synthetic plans and distinct boot projections, independently signed forward/return credentials, AES-GCM ciphertexts, Ed25519 sender signatures and the canonical verifier. The check also exercises revoked or expired authority, a replaced boot in the same epoch, stale policy, wrong original nonce, signature, return route, correlation, causation and replay. [Typed contract](https://github.com/Casey-allard/mvm/blob/b3868edc9d307f450dde9a4de0e17b7f43969b0e/crates/mvm-core/src/mailbox/object_pair.rs)

This is an in-process contract and installed-startup refusal check. Its test inputs were generated for the test. It provides no VM, wire-byte, CPU savings or WAN performance measurement. Installed cold/warm and loss/retry trials remain tracked under [#202](https://github.com/R4-Laboratories/SpiralCore/issues/202).

The retained Linux x86-64 run used Rust 1.97.1, unoptimized test builds, debug symbols disabled, incremental off and two Cargo jobs. The endpoint fixture sealed a four-byte synthetic body including a trailing zero in each direction. The test asserts credentials, return-route matching and replay decisions. It does not attempt an application readback or a transfer measurement.

| Executed check | Result |
|---|---|
| Original parser with the new selector | 0/1, unknown field |
| Stable grant serialization and retention | 4/4 |
| Core mailbox suite, including two new pair tests | 59/59 |
| Host admission and key-derivation refusal | 2/2 |
| Ordinary guest mailbox and reader controls | 12/12 |
| SDK crate with the preserved selection | `cargo check` |

To reproduce the qualified slice, check out the contributor commit linked above and run the following commands on Linux. The retained result records the exact log hashes. [Qualification record](object-pair-enrollment.json)

```sh
export CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2
export CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
cargo +1.97.1 test --offline --locked -p mvm-contract --features protocol --test mailbox_retention
cargo +1.97.1 test --offline --locked -p mvm-core --features test-support --lib mailbox::
cargo +1.97.1 test --offline --locked -p mvm-hostd --lib selected_object_pair
cargo +1.97.1 test --offline --locked -p mvm-agentd --bin mvm-guest-agent mailbox
cargo +1.97.1 check --offline --locked -p mvm-client
```
