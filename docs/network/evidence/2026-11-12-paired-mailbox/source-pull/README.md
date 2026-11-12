# Pulling an encrypted submission from its original mailbox helper

The host coordinator can request one submission on guest A’s private per-VM mailbox socket. Before sending the one-byte trigger, it checks both original workers, their signed direction, current permissions, the kernel PID of A’s socket, and the shared memory budget. A has a 32 KiB reservation for its retained encrypted submission. Reconnecting to A reuses that reservation. The coordinator holds a second 32 KiB reservation for the forwarding turn and keeps control capacity available. Collecting an original helper releases its own share. [Rust coordinator and wire contract](https://github.com/Casey-allard/mvm/blob/43e99217dc7ff94d4f4f24aa7596dba18281bd7c/crates/mvm-hostd/src/mailbox_launch/cast_pair/source_pull.rs)

A returns a four-byte length and the usual MVM frame. The coordinator compares the signed Send credential and canonical encrypted envelope byte for byte before recording admission. It samples authorization again before forwarding to B. It relays B’s bounded ingress verdict to A, then half-closes the original socket. An ambiguous outcome leaves A responsible for its original submission. The inner message ID stays encrypted. The [destination helper](../destination-ingress/README.md) owns delivery to B’s guest.

## Evidence

The ownership regression reproduced an early 32,768-byte refund. Its successor ran two real helper processes. It refused the wrong PID or direction before charging, reused A’s allocation across two connections, maintained B’s allocation on reply loss, preserved control capacity, and returned each allocation on collection of its corresponding child. A separate socketpair test exercised the one-byte pull, length boundary, truncated input and exact binary readback. The final revision also rejects alternative JSON encodings of the enrolled credential.

The Linux run used `da4004eb` before the canonical credential comparison was tightened. The macOS run used `43e99217` after the change. The recorded elapsed time and CPU refer specifically to the 257-byte handoff into B that ended at ambiguous EOF. The five-byte pull framing test and the full public routing entry point have no end-to-end guest timing in this packet. [Exact results, revisions, CPU and byte counts](results.json)

## Reproduce

Apply the [compressed source patch](source.patch.gz) to MVM contribution revision `8ff62f25aa549967fb76aad879927537ee5609d8`. This recreates the final source tree. Run these selectors from the resulting checkout.

```sh
cargo test --locked -p mvm-hostd --lib lost_peer_reply_keeps_cast_workspace -- --nocapture
cargo test --locked -p mvm-hostd --lib original_pull_
cargo test --locked -p mvm-hostd --lib cast_credit::tests
cargo test --locked -p mvm-hostd --lib mailbox_process::owned_wire::tests
```
