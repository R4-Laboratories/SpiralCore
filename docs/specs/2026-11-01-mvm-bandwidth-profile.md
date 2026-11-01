# Bounded mailbox bandwidth profile

This approved implementation contract adds one explicit experimental throughput
profile to the existing local MVM patch at
`3104a12c5b071a6851323fa8115d2afaf3f0bb8b`. It follows the user's request to
optimize MVM/vsock overhead while retaining exact reconstruction, measured bytes
and computation, and fair comparison with whole-object zstd. Root review approved
the interface before implementation. The serialized value is defined by
[the profile schema](../contracts/mvm-bandwidth-profile-v1.schema.json).

The default remains `standard`: 4 MiB/s with an 8 MiB burst. The optional
`experimental64` profile selects 64 MiB/s with the same 8 MiB burst. Both retain
128 stream slots, the existing idle timeout and bounded backpressure. This changes
only port 5257's independent mailbox budget. Egress, broker, display, GPU and
trusted-builder policy stay as before. No unlimited variant or arbitrary numeric
rate is accepted.

`mvm_contract::protocol::mailbox::MailboxBandwidthProfile` has `Standard` and
`Experimental64MiB` variants, defaulting to `Standard`, with serialized names
`standard` and `experimental64`. Its rate and burst accessors expose exact byte
counts. `HostChannels.mailbox_bandwidth_profile` propagates through `RunInputs`
and `VsockHostBindings`. The new explicit endpoint/profile setter configures only
the mailbox handler before a session begins. The existing endpoint setter remains
available and selects `Standard`. Rebinding replaces that mailbox handler and
drops its old queued state; a profile is host authority, not snapshot or guest
negotiation. A cleared host binding resets to its default profile.

SpiralCore's `Plan.bandwidth_profile` uses a serde default so prior bootstrap JSON
continues selecting `standard`. The optional final session CLI argument is
`standard|experimental64`; omission preserves the old invocation. The worker
receives the selected value explicitly. Unknown profile strings are errors before
boot or output-directory creation. The plan, session and worker receipts expose
the selected profile and effective rate/burst. A verifier boot remains socket-free.

The same profile must be used for every compared arm in a measurement cell.
Changing the cap is a separately labeled transport configuration, not a codec
speedup. The existing canonical envelope, queue/capability checks, encryption,
per-message authentication, one-call window and durable commit are unchanged.
This milestone makes no new WAN, production-policy or energy claim.

Implementation sequence:

1. Preserve the prior host/guest executables and exact source identities before
   changing the local patch. Parent ownership of this step is confirmed.
2. Add a focused failing typed-profile/token-refill regression and a CLI contract
   regression for omitted, explicit and rejected values; retain the RED output.
3. Add the two fixed profiles, thread the host-owned choice into only the mailbox
   handler, and add serde-defaulted Plan/CLI/receipt propagation. Verify the token
   budget with a controlled clock, without wall-clock sleeps or a performance run.
4. Run only the relevant contract/VMM/live tests and scoped Clippy/build checks
   with two Cargo jobs. No broad coverage rebuild is required for this addition.
5. Regenerate the retained binary/full-index patch, exact base blobs, patched-file
   hashes and bundle hashes, including its existing MVM license. Verify clean
   application to the exact base and document the optional CLI.
6. Parent registers and runs the paired comparison after both baseline and
   candidate executables are ready. Retain all actual transfer, CPU and wall-time
   records and exact-byte checks. Compare both arms at each same profile; keep
   default and experimental results separate.
