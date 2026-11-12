# The destination mailbox helper

The development adapter binds B’s existing VM mailbox socket to one addressed Cast ingress. It checks the connecting coordinator’s kernel peer PID and the inherited control channel, validates the original signed plans and capabilities, then unwraps the directional key delegated privately to B. It announces readiness for that direction only after the socket is bound. A frame can enter through this socket only from the original coordinator. [Helper implementation](https://github.com/Casey-allard/mvm/blob/5e854749594f90d94ab2ca873f7818f9aa32b3e2/crates/mvm-hostd/src/mailbox_process/destination_ingress.rs)

The helper caps the complete peer body at 8,192 bytes and its encoded envelope at 4,096 bytes. It rechecks authorization after the guest’s greeting. An accepted guest-role acknowledgment records the replay identity in the current helper. A repeated identical frame receives the original positive verdict while the authorization and route remain live. An incomplete guest exchange stops the helper without a terminal verdict. The coordinator then holds workspace credit until it collects that original helper. This development profile fixes policy within one generation. Changing its policy requires retiring the original helper. [Key and policy enrollment](https://github.com/Casey-allard/mvm/blob/5e854749594f90d94ab2ca873f7818f9aa32b3e2/crates/mvm-hostd/src/mailbox_launch/cast_pair/development/destination.rs) · [Coordinator custody](../helper-handoff.md)

## Measured transfer

One Rust test on Apple Silicon macOS read back **`00 41 ff 42 00`** after sending the actual signed credential and ciphertext on a Unix stream socketpair. A second socketpair carried the canonical guest-role Deliver exchange. The counters use successful read and write lengths. The timer starts after preparing the signed frame and sockets and ends after receiving the peer verdict and joining the exact readback. The guest-role thread ran in the test process. [Recorded output](development.txt) · [Machine-readable result](results.json)

| Measurement | Observed |
|---|---:|
| Original application bytes recovered | 5 |
| Peer frame to B / terminal verdict back | 4,962 / 11 B |
| Local Deliver to guest role / acknowledgment back | 283 / 98 B |
| Peer thread CPU | 0.249125 ms |
| Destination helper thread CPU | 4.123833 ms |
| Guest-role thread CPU | 0.026416 ms |
| Elapsed through joined exact readback | 4.525208 ms |

The small payload exercises fidelity and authorization. Credential encoding, framing and acknowledgment costs dominate it. The timing is one unoptimized component observation. Provisioning, original source sealing, guest startup and durable journal commit fall outside its boundary. Two booted guests exchanging this Cast, two-host WAN transfer and comparison against zstd are still open. The canonical installed ReliableV1 admission gate remains in place. [Listener identity checks](socket.txt) · [Original tenant/domain refusal](scope.txt)

## Reproduce

Use the contribution fork at `5e854749594f90d94ab2ca873f7818f9aa32b3e2`. The archived patch applies to its parent `8ff62f25aa549967fb76aad879927537ee5609d8`. The result file identifies the source, patch and macOS test executable. The commands below use the pinned lockfile. [Exact source revision](https://github.com/Casey-allard/mvm/commit/5e854749594f90d94ab2ca873f7818f9aa32b3e2) · [Source patch](source.patch.gz)

```sh
export CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2
cargo test --locked -p mvm-hostd --lib development -- --test-threads=1 --nocapture
cargo test --locked -p mvm-hostd --lib existing_socket_and_unrelated_process_identity_cannot_be_adopted
cargo test --locked -p mvm-hostd --lib original_scope_refuses_validly_signed_foreign_tenant_or_domain
```
