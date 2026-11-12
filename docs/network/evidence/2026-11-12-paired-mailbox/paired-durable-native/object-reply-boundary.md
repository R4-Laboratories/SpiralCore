# What the installed mailbox permits at the object edge

A can send an encrypted Cast to B. B can attest durable acceptance and later ACK its own reader lease. For a warm transfer, A also needs an authenticated **application reply** from B. The signed enrollment used by the installed Firecracker pair has no such route. The installed 257-byte and 4 KiB results establish exact delivery of full fixtures. [Native measurements](size-control/README.md)

## Installed authority

On Linux x86-64 the host regression invoked the same enrollment constructor used by both original guests, in a fresh private test home. It verified both issued Ed25519 signatures and inspected the public signed grants. A’s local scope and B’s remote Send each name A as the subject and permit only Send/Cast of `application/vnd.mvm.guest-message` version 1. The destination grant commits B’s installed incoming policy and has no outgoing journal direction. The focused enrollment control and ten other host controls passed. No guest was started in this check. [Test](../../../../../tools/ari-own-mailbox-witness/bin/paired/enrollment.rs) · [Installed incoming policy](https://github.com/Casey-allard/mvm/blob/8238000662f59893b6ec703ab4b37d7a86c08340/crates/mvm-hostd/src/mailbox_process/paired_incoming.rs#L104)

The original paired helper emits a Cast with no reply route. Its signed V2 receipt binds the destination and accepted ciphertext. It contains no application frame expressing Ready, missing records or Published. B’s reader ACK settles its individual lease. Neither fact allows A to promote an object into its verified sender cache. [Canonical paired header](https://github.com/Casey-allard/mvm/blob/8238000662f59893b6ec703ab4b37d7a86c08340/crates/mvm-hostd/src/mailbox_process/original_journal/authorization.rs#L148) · [Receipt validation](../../../../../tools/ari-own-mailbox-witness/bin/paired/main.rs)

## Existing object Call/Reply mapping

A separate macOS arm64 component test ran the shared Rust sender against a persistent object receiver through two local mailbox services. The older pinned binding provided independently issued forward Call and reverse Reply credentials for `application/spiralcore-object` version 1, each with a 65,536-byte plaintext cap. For each transfer B verified the exact 131,090 original bytes, including 17 appended zero bytes. A consumed B’s correlated Published response. The repeated transfer used a fresh transfer ID and sent only the Offer. An admitted Cast in the same test suite received `Unauthorized` from both the object handler and the reply builder. All six tests passed. [Component source](../../../../../crates/spiralcore-mvm-adapter/tests/service_object.rs) · [Pinned MVM source](https://github.com/Casey-allard/mvm/tree/ef1067e3bcce4f72bcd5cdacb6b914e76298ae20)

| One local transfer, in execution order | Cold raw | Exact repeat |
|---|---:|---:|
| Original bytes independently read back at B | 131,090 | 131,090 |
| Raw Data payload bytes | 131,090 | 0 |
| Object request / response frame bytes | 131,431 / 346 | 102 / 88 |
| Encoded MVM Call / Reply envelope bytes | 252,786 / 17,142 | 4,016 / 3,475 |
| CPU on the single test thread | 325.978 ms | 42.553 ms |
| Elapsed including B readback and A polling the reply | 337.404 ms | 42.737 ms |

This single ordered component sample used Cargo’s unoptimized test profile, debug information disabled, Rust 1.97.1. Measurements start immediately before the shared transfer and include sealing, admission, signature/scope checks, storage, exact comparison and correlation on one thread. Setup is outside this interval. The envelope counts are the lengths of the encoded bytes submitted to the in-process services. Receipts returned as local objects, SDK traffic, socket framing, CPU split by role, send-only time and WAN latency have no wire measurements here. Fresh signed message identities can change serialized lengths between runs.

## Required integration decision

1. Establish a canonical durable route for a correlated Reply under an original, independently issued B-to-A authorization. Bind the object content type and MVM version, both tenant/domain/principal addresses, original boot and epochs, expiry/revocation, per-message byte limits and outstanding work to the two signed plans. The existing destination-only profile explicitly requires B’s outbound direction to be absent. A combined durable profile needs owner approval and versioning; adding an outbound value to the existing B grant fails admission.
2. At the authorized recipient application, feed a verified A request to the existing object receiver and send its exact reply under that grant. A verifies the correlated response, transfer ID, sequence and Published object identity before committing its sender state. An expired return grant or dropped reply leaves the sender unconfirmed. Host relays continue to carry opaque envelopes.
3. Size each object frame to the original signed limits. For example, the 4 KiB native fixture grants 4,152 bytes. A single Raw Data frame containing all 4,096 bytes needs 4,157 bytes (56 frame + 5 Raw Data metadata + body). Test segmentation and truncation against the original grant independently of reciprocal authorization.
4. Repeat cold, exact-repeat, lost-reply, eviction and epoch/revocation trials on actual installed guests. Count both envelope directions, receipts, guest↔host traffic and CPU through verified application readback. Carry the accepted mapping to the two-host WAN matrix. [Delivery tracker](https://github.com/R4-Laboratories/SpiralCore/issues/43)

## Reproduction

Use SpiralCore `d2e21e3c830c4e4a2e0f4dd370c0cdcb9add1874`. The first command uses the adapter lockfile. For the second, place the native witness beside `../source` checked out at MVM `8238000662f59893b6ec703ab4b37d7a86c08340`, following the [native setup](size-control/README.md).

```sh
cargo test --locked --manifest-path crates/spiralcore-mvm-adapter/Cargo.toml   --features live --test service_object -- --test-threads=1 --nocapture
# From the staged Linux native witness directory
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked   --features paired-journal-qualification,large-object-fixture   --bin paired-journal -- --test-threads=1
```

| SHA-256 | macOS Call/Reply suite | Linux enrollment suite |
|---|---|---|
| Test executable | `bf5eb2c05aa520b91ce728de7e8e20aff7eb8eec47999c87b01f2706673cc786` | `a6ca25c2753627d034f4580b819fe5c24f31444b418bccf481397d12998b12a1` |
| Original Cargo log | `2cbff9cbdbf9b54d610c253a8490136104a7ce5abce84853b2fa9faf86fba762` | `debf49dd6096ef00d8b71290bcbb94353d1fed2f4659a78ce27dfeda1dbbc19e` |
