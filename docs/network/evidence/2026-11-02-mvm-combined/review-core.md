# Independent scoped Rust and security review

Reviewed SC-MVM-004 production changes in `campaign.rs`, `config.rs`,
`fixtures.rs` and `main.rs`, the mixed campaign regression tests, and the existing
`ObjectSender`, canonical live adapter and orchestrator boundaries they invoke.
Review took place before the frozen-source native campaign. No blocking finding
was identified in this scope.

- Both Combined arms call the existing `with_combined_policy` facade and pay
  explicit checkpoint/schema publications. The receiver uses the same bounded
  authenticated endpoint and durable publication implementation. No new wire
  format, crypto, capability interpretation, relay key access or production
  mailbox service is introduced.
- All three mixed arms pass `warm_probe=true` through `send_with_receipt`. Whole
  uses the prepared whole-object zstd implementation with its existing verified
  presence optimization. The legacy arms retain their original branch and
  fixtures. The explicit CPU-conscious policy does not silently replace the
  exhaustive default.
- The fixture construction matches the retained object gateway definitions:
  canonical CA state/replay, domain-separated opaque bytes, sparse edit offsets,
  and held-out schema-fit/drift construction. Nominal sizes and trial count are
  bounded before provisioning. The new source pins the near-1-MiB trial-zero
  fixture identities to the previous retained campaign.
- Common fixture generation, including CA replay, is included in setup for all
  arms. Combined additionally pays its prepared prediction replay. This is a
  synthetic workload-generation cost as well as codec setup; it must not be
  represented as pure application encoding CPU. Per-operation ownership copies
  and independent expected-object hashing remain outside the reported operation
  window, consistently with the existing witness.
- Selection candidate costs remain explicitly labeled as the existing HTTP-body
  model. Actual encrypted envelope/framing counters are the mailbox byte result;
  planned application frame counts can exceed transmitted frames after an early
  publication reply. Exchange wall time includes remote work and waiting and is
  not an isolated send-only measurement.
- Relay admission, canonical authentication/decryption before dispatch, final
  publication identity checks, complete trailing-frame rejection, exact disk
  reopen and keyless second-guest readback remain on the existing path.

## Focused interruption evidence

The reviewer added `tests/combined_recovery.rs`. A real authenticated Unix socket
stream is cut after 256 KiB of sender writes. The observed receiver receipts
contain exactly checkpoint, schema, generated, residual and repeat. The unrelated
object is interrupted before publication; the only `.object` files are the set
of acknowledged objects. Neither side emits successful campaign completion.
New actors and a new authenticated socket reopen the same durable store and
complete all ten publications, followed by exact readback.

The test passed in `recovery-negative-green.txt` (one test, zero failures).
Implementation became available while this independent regression was being
authored, so this receipt is **not** claimed as a preimplementation RED.
`combined-red.log` is the separate implementer's feature RED. The injected
interruption is an expected failure inside the passing regression.

This recovery evidence is socket-level and reuses the same still-valid
capability/epoch with fresh envelopes. It is not a durable replay-inbox claim,
revocation-resume qualification, forced guest crash campaign or Internet WAN
result. Native HVF measurements and final source-bound verification are recorded
separately by the parent execution.
