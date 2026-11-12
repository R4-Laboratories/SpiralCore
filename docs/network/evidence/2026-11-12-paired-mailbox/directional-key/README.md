# Enrolling one key for an addressed Cast

SpiralCore’s development integration can now prepare a directional AES-256-GCM key for a specific pair of MVM mailbox workers. A Rust component test sealed five bytes at the source, sent the encrypted envelope through a Unix socket, opened it at the authorized destination and read the original five bytes from the guest-role inbox. The input began and ended with zero and contained `ff`. [Exact measurements and executable hashes](results.json)

## Where the key lives

1. **Before guest boot**, the development launcher checks the two original Reserved owners, signed plans, and mailbox authorities. It verifies A’s signed membership in its own scope, B’s Send grant for A, and B’s Receive grant for its recipient. The public direction commitment binds the ordered owners, credentials, epochs, role generations, route, expiry and key ID. B can bind its Deliver greeting to that commitment in its boot record.
2. **At private configuration**, the launcher wraps the same fresh data key separately for each role using MVM’s existing AES-GCM implementation. Each original worker has its own wrapping key, conveyed on its once-delegated private config pipe. The guest can use the public commitment and greeting. The opaque relay handles encrypted envelopes and 60-byte wrapped capsules.
3. **While serving**, A seals the Submit with its original signer and retained sequence. B rechecks the authorized direction and source boot before it updates replay state. A reconnect can reuse the first sealed envelope. A replayed or swapped setup, a revoked capability or a changed original owner fails its corresponding check. The later macOS revision also pins the prepared commitment inside each private config and tests that assigning the two issuer slots is atomic. [Source and Linux checkpoint](https://github.com/Casey-allard/mvm/tree/edb2136c02fe2b9574d3d3991d4c072a0359a27a/crates/mvm-hostd/src/mailbox_launch/cast_pair) · [Successor and macOS checkpoint](https://github.com/Casey-allard/mvm/tree/ca26ff453a0012e5894f91484471b3f170853769/crates/mvm-hostd/src/mailbox_launch/cast_pair)

## Observed socket traffic and CPU

| One exact transfer of `00 41 ff 42 00` | Linux x86-64 | macOS arm64 |
|---|---:|---:|
| MVM source revision | `edb2136c` | `ca26ff45` |
| Passing hostd selectors | 4 | 5 |
| Ciphertext socket, A → B / B → A | 3,449 / 0 B | 3,443 / 0 B |
| Deliver socket, host → guest role / guest role → host | 283 / 98 B | 283 / 98 B |
| Source preparation thread CPU | 578,925 ns | 1,106,917 ns |
| Destination open thread CPU | 474,821 ns | 1,547,292 ns |
| Deliver dispatch thread CPU | 388,929 ns | 483,833 ns |
| Guest-role receive and callback thread CPU | 44,714 ns | 34,250 ns |
| First preparation through verified readback | 1,647,599 ns | 3,654,583 ns |

Each column records one local observation on its named source revision. The envelope socket carried canonical encrypted JSON. The test supplied the signed credentials independently, so its byte counts omit credential transport and outer peer framing. The second socket carried the canonical local Deliver exchange and acknowledgment. The timer excluded key provisioning, VM startup and durable journal admission. These runs supply no WAN, compression-speed or FIPS qualification. [Linux raw output](linux-x86_64.txt) · [macOS raw output](macos-arm64.txt) · [Earlier complete peer-frame measurement](../source-cast.md)

## Capacity and next installed gate

The proposed development profile admits a 4 KiB encoded envelope inside an 8 KiB worker frame. Two queued frames per worker leave room for the retained 32 KiB source allocation and a separate 32 KiB routing allocation inside the 1 MiB aggregate. A host-library test charged both allocations and retained control headroom. The normal paired worker profile keeps its four-entry queue. These are protocol budget checks. RSS, throughput and multi-tenant fairness still require measurement. [Capacity implementation](https://github.com/Casey-allard/mvm/blob/ca26ff453a0012e5894f91484471b3f170853769/crates/mvm-hostd/src/mailbox_process/installed_profile.rs)

The stock installed service still refuses to start the complete ReliableV1 role set. The component fixture uses synthetic worker identities and local sockets. The next acceptance run needs the original acquired helpers, their protected interprocess handoff, an admitted source Submit and exact readback inside the destination VM. The existing two-guest Firecracker run established each guest’s own mailbox path. [Installed native measurements](../firecracker.md) · [Mailbox tracker](https://github.com/R4-Laboratories/SpiralCore/issues/43)

## Reproduce

At either listed revision, run these selectors from the MVM repository root. The fifth selector was added in the macOS revision. The first selector prints the measured row.

```sh
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
cargo test --locked -p mvm-hostd --lib independent_original_development_keys_carry_exact_bytes_and_refuse_swaps -- --nocapture --test-threads=1
cargo test --locked -p mvm-hostd --lib development_source_and_routed_turn_share_the_unchanged_host_budget
cargo test --locked -p mvm-hostd --lib addressed_cast_shares_the_two_workers_and_preserves_control_headroom
cargo test --locked -p mvm-hostd --lib original_scope_refuses_validly_signed_foreign_tenant_or_domain
cargo test --locked -p mvm-hostd --lib original_development_pair_binding_is_atomic_and_cannot_rotate
```
