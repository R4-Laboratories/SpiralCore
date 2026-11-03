# Registered Call retry: narrow source security review

Verdict: **PASS — source review**, 2026-11-03. No blocking finding in the
`service.rs` delta from `a8a4a9a8daba67c00447a067b18632ed515d95b2` to
`318f05f02db385e928f719f3de6e36dc073f18cc`, against the
[approved API contract](../../../../api/registered-call-retry-v1.md).
Reviewed file SHA-256:
`5563e59811266047b582651aa6e08682ff3c368d417c2aabdbee8930a1217794`.

- Both wrappers use the same immutable `registered_call_send_expectation`.
  Strict send passes false; retry passes true. Original shutdown, clock,
  service/binding/exact-handle, original mono/Unix expiry and terminal-status
  guard order/error mappings are preserved. Only retry admits `Replied(Some)`;
  consumed `Replied(None)` remains `Conflict`.
- The validator returns the retained generation and semantic request commitment.
  Both wrappers invoke unchanged `send_prepared_inner`: cancellation, one
  canonical verification, existing policy-lock generation/commitment comparison,
  route/capability/outbound authorization and size checks precede connect/send.
  No outer lock or competing authority path is added. Transport failures retain
  existing uncertain-outcome semantics; success is a write, not remote acceptance.
- Neither new method nor validator mutates accounting clocks, Call/FIFO status,
  expiry, registration, Reply payload or its reservation. No `poll_call`, payload
  clone/take, resource reset or additional retained retry state is introduced.
  Existing same-service immutable-borrow and binding-policy lock boundaries hold.
- The shared service pins semantic request identity, rather than storing another
  exact outer envelope. The producer still owns original nonce/delivery/encoded
  attempt and canonical tracker registration. This API neither enforces a retry
  schedule/count nor creates a new attempt; those remain consumer obligations.

No builds, tests, measurements or external actions were performed for this review.
Focused lifecycle/payload/byte-readback qualification and release CI remain the
lead/test owner's separate gates. The normal engine does not use the API in this
slice: no performance, consumer recovery, automatic resend, reconnect/resume or
full-C8/scale completion claim follows from this approval.
