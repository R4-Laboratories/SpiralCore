# Registered v1 endpoint compatibility comparison

The comparison preserves the current Combined-CPU strategy, fixtures, setup
objects, SCMOBJ01 frames and MVM envelope. It measures the effect of the shared
v1 endpoint API and its immutable profile validation, not a codec improvement.

- Baseline: frozen native Linux binaries from source
  `a9f038c49bcaf69ce7d6780970dadd412a31a573`.
- Candidate: a frozen source commit, recorded in `registration.json` together
  with both host/guest SHA-256 identities and pinned kernel identity.
- Two implementations × guest/host and two-guest stages × nominal 1/16 MiB
  fixture tiers × three matched trials = 24 sequential sessions. Implementation
  order alternates by trial. All sessions use `combined-cpu`, with eight targets
  plus both paid schema/runtime-checkpoint setup publications.
- Each target must reconstruct exactly after reopen. The two-guest path also
  boots a credential-free read-only guest on the receiver disk and compares all
  ten objects again. No failed run may be discarded or have its stores cleaned.
- `combined-session.jq` validates each raw receipt before payload-store cleanup.
  `v1-comparison.jq` checks all 24 unique registration cells and 120 matched
  operation groups, requiring equal object identities, lengths, modes, planned SCMOBJ01 request/response
  frame-byte totals and actual carrier-frame counts across implementations.
  Planned bytes count complete batches before callback and can overcount after
  an early Published reply; they are not claimed as actual transferred bytes.
- Record canonical encrypted envelope/framing bytes separately. Session IDs and
  envelope encoding can produce small size differences without a codec change.
  Two-guest physical mailbox stream accounting counts both hops, excluding
  workload-exit service and virtio credit/control headers.
- Record sender CPU, receiver-processing CPU, sender session elapsed, exchange
  CPU/wall time and full lifecycle. Sender session timing includes fixture and
  codec preparation and paid setup, but excludes independent readback/reboot.
  Endpoint CPU is partial; do not add enclosing worker/VMM CPU to guest CPU.
- Linux has no enforced mailbox rate limiter. `experimental64` is recorded as
  requested intent; effective rate/burst remain null. This is local native
  Firecracker/vsock execution, not WAN, production mailbox or energy evidence.
- Three paired trials provide descriptive medians; no p95, significance or broad
  performance qualification claim. Retain all per-operation regressions.
- The runner reserves 4 GiB free disk and removes only verified owned ephemeral
  payload stores after all required readbacks. Public receipts remain.

Validator tests use explicitly synthetic duplicate labels on the prior campaign
receipts, and test rejection of altered application bytes, object identities and
registration cells. Those fixtures are parser tests, never new transfer results.
