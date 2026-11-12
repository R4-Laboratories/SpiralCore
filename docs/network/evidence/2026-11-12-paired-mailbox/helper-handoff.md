# Original helper custody when a reply is lost

The coordinator connects to the receiving mailbox helper’s provisioned Unix data socket. Immediately before writing, it checks the signed direction, current permissions, original worker leases, kernel peer PID and the direction selected by the helper on its private readiness connection. A legacy readiness message cannot authorize this operation. The canonical Send credential and the retained encrypted envelope use MVM’s existing length-prefixed frame. [Forwarding code](https://github.com/Casey-allard/mvm/blob/8ff62f25aa549967fb76aad879927537ee5609d8/crates/mvm-hostd/src/mailbox_launch/cast_pair.rs) · [Readiness binding](https://github.com/Casey-allard/mvm/blob/8ff62f25aa549967fb76aad879927537ee5609d8/crates/mvm-hostd/src/mailbox_process/owned_wire.rs)

The coordinator and the original child share one workspace reservation. A complete ingress verdict on the authenticated connection ends that operation. EOF without a verdict, an incomplete reply or a timeout leaves the reservation with the original child. Collection of that child can then release it. The destination’s journal separately owns durable acceptance. [Workspace ownership](https://github.com/Casey-allard/mvm/blob/8ff62f25aa549967fb76aad879927537ee5609d8/crates/mvm-hostd/src/broker/mailbox_workers/cast_credit.rs)

## Measured failure and recovery

Two macOS arm64 helper subprocesses exercised the coordinator’s real worker registry and shared budget. A fixture helper read every byte of a 257-byte binary body, including the final zero, and closed its data connection without returning a verdict. Its process remained alive. The coordinator’s original deadline, peer-PID check, budget and child collection paths ran. The fixture supplied its own child factory and readiness setup. [Executable fixture](https://github.com/Casey-allard/mvm/blob/8ff62f25aa549967fb76aad879927537ee5609d8/crates/mvm-hostd/src/broker/mailbox_workers/cast_handoff_tests.rs) · [Measurements and binary hashes](helper-handoff.json)

| Measurement | Observed result |
|---|---:|
| Successful bytes into the helper | 4-byte prefix + 257 body bytes |
| Reply bytes from the helper | 0 |
| Workspace retained after lost reply | 32,768 bytes |
| Exchange start through ambiguous EOF | 1.365 ms |
| Coordinator test-thread CPU | 0.113 ms |
| Receiving helper thread CPU | 0.063 ms |

Before the fix the coordinator refunded 32,768 bytes while B was still alive. After the fix another data reservation was refused, control traffic could still reserve space, and collecting B restored the original budget. The timing is one debug-profile observation. The receiver CPU scope covers the read and byte comparison. Setup, process launch, disposal and result-file publication are outside that CPU scope. The custody fixture uses an opaque test body. The separate [complete encrypted-frame transfer](source-cast.md) exercises the canonical codec.

## Reproduce

The [six-file source patch](original-helper-custody.patch) applies to MVM contribution revision `ca26ff453a0012e5894f91484471b3f170853769`. Applying it to that tree reproduces the tested tree exactly.

At revision `8ff62f25aa549967fb76aad879927537ee5609d8`, run the focused regression:

```sh
cargo test --locked -p mvm-hostd --lib lost_peer_reply_keeps_cast_workspace -- --nocapture
```

The default installed helper still needs the exclusive addressed handler, its per-turn policy check, and workload readback across the two guest boundaries. Reconciliation of a lost verdict while the same receiving helper remains live is a further recovery gate. [Mailbox tracker](https://github.com/R4-Laboratories/SpiralCore/issues/43)
