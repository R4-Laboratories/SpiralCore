# Addressed two-guest witness integration

The historical `--pair` witness exercises two original workers and two own-mailbox messages. The addressed driver stage uses the same installed SDK and teardown. It requires a signed A→B direction, an actual guest A Submit workload and B's warm OCI application. The native addressed run is pending.

## Supply separate guest images

From the integrated MVM revision, build the original OCI wrapper for the target Linux guest architecture and build the B receiver example with the same toolchain. For x86_64 musl the example target is:

```sh
cargo zigbuild --release --locked --target x86_64-unknown-linux-musl \
  -p mvm-agentd --example original_cast_workload
```

Build A’s `mvm-agentd --example original_submit_workload` for the same target. Provide its executable and the B receiver to the existing rootfs builder. The paired builder requires a new directory and distinct binaries, and records SHA-256 for both packaged workloads and wrappers. Record the final guest-agent binary and assembled images separately.

```sh
bash make-addressed-pair-rootfs.sh "$fresh_images" "$wrapper" "$submit_workload" "$receive_workload"
```

The A image contains the Submit producer. The B image contains the recipient alone. B's signed execution plan selects the warm OCI recipient at `/usr/local/bin/spiralcore-cast-recipient`. Configure the installed SDK provider for the named B destination, reserve both original worker contexts, and use the same signed direction, boot commitments and local grants when starting them. Pin the integrated MVM source and rebuilt guest assets before a native run. The previous native witness targets an earlier revision.

## Observe B independently

Call `native::cast_readback::verify_original_pair` with the original SDK launch results and retained worker-control clients, the expected synthetic bytes and the admitted A→B carrier operation that returns its message identifier. The driver hands the original bytes only to A’s ordinary workload call and verifies its 32-byte ephemeral admission result equals the routed identifier. The driver starts B’s ordinary call with a fresh 32-byte challenge and concurrently invokes the submission. The call’s stdin contains only the challenge. At most 1,024 source bytes are admitted for this fixture.

B must return exactly the entire source byte string followed by that challenge. The verifier also requires a successful call, complete output, no control records, guest application CPU and wall time, and both original workers still Serving after the result. Failed delivery, timeout, partial responses, echoed stdin alone and a stale challenge fail the test. The application response is observed separately from mailbox admission, journal persistence and signed receipts. Meter both actual role streams, encrypted carrier and reply, and both worker CPU domains when the native route is active.

## Host carrier required by the driver

The original A helper retains the sealed request and sequence, and retries with the same bytes while the signed policy remains current. The paired development limits admit a 4,096-byte sealed envelope. The complete signed Send plus envelope and framing can exceed that number and requires its own 8 KiB frame bound. Both the retained source reservation and the routing reservation draw 32 KiB each from the shared worker ledger. The latter must remain charged after publishing across private IPC until the original transfer is reconciled. A dropped descriptor, lost reply, deadline or new process cannot free it or mint another turn.

Before routing, the coordinator verifies the two live original workers, signed grants, admitted plans, tenant, direction, epochs, time and revocation state. B’s original helper independently checks its binding and current grants when opening the envelope and before publishing plaintext on its admitted Deliver stream. Keep private key material in each authorized endpoint. FD 3 remains the installed helper’s private Prepare/liveness channel. A coordinator-local Rust borrow must remain local. The process boundary needs an original-bound handoff plus terminal reconciliation. A guest-role Deliver acknowledgment alone cannot authorize durable acceptance or refund an ambiguous publication.

The paired workloads, original-bound carrier across the installed processes, and native SDK orchestration must be connected before the driver can produce a two-guest acceptance report. The own-mailbox CLI performs its original operation.
