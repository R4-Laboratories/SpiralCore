# SC-MVM-004: Combined selection through canonical mailbox streams

The user approved integrating the current Combined runtime/warm-cache selector
into the existing one-guest and two-guest MVM witness, then measuring a persistent
mixed session. This implements that approved scope without changing codec,
mailbox, cryptographic or transport policy.

## Decision

Add explicit `combined`, `combined-cpu` and `whole-mixed` arms. The old `warm`,
`whole` and `raw` fixtures and behavior remain reproducible. Both Combined arms
use `ObjectSender::with_combined_policy`; they publish the checkpoint and schema
explicitly before sending eight objects. Whole-mixed uses the same prepared
whole-object zstd sender, identical payloads and warm-probe setting, without
unnecessary checkpoint/schema publications. CpuConscious stays opt-in.

Use the public fixture construction from the object-gateway campaign: width 4096,
289 or 4096 replay steps, 16 edits of 32 bytes, deterministic domain-separated
opaque bytes, and the existing trained schema plus held-out fit/drift controls.
Requested size 1 MiB maps to 1,047,202 actual bytes; requested 16 MiB maps to
14,851,384 bytes. Trial is bounded to 0..2. Fixture generation is charged in
setup and remains separate from transfer operation timing. The CA replay and
canonical encodings are reused directly, with no alternate runtime implementation.

Send generated, residual, repeat, unrelated, opaque-edited, opaque-edited-repeat,
schema-fit and schema-drift in that order through one persistent receiver. Record
selection policy, actual skips separately from proven pruning, candidate costs
as HTTP-body model values, and actual canonical envelope/framing counters as the
mailbox transport result. Sender and receiver CPU domains remain separate.

The authorized plaintext endpoint encodes before canonical mailbox encryption and
decodes only after canonical authentication/decryption. The host relay remains
keyless. Experimental vsock port 5257, production-disabled supervisor binding,
FlowMux port 5253, quota and epoch/capability requirements are unchanged.

## Acceptance and limitations

Observe focused RED/GREEN evidence, then exact reconstruction/reopen across all
three arms. Exercise a fresh authenticated socket and reopened endpoint against
durable objects, and a deliberately interrupted stream that cannot publish its
unfinished object. Actual HVF one/two guest measurements and keyless reboot
readback are separate from socket-level regression tests. Compare the same
bandwidth profile across arms, retain setup and failed runs, and report bytes,
CPU and wall time without inferring physical energy or Internet WAN performance.

No transparent TUN stream compression, MVM production mailbox activation, new WAN
deployment, or FIPS qualification is included in this bounded milestone.
