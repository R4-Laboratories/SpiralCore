# Reconciliation retry state: narrow security review

**PASS — state-only source review**, 2026-11-03. Reviewed the 51-line production
delta at `86383931eb8883323e41695448f351d7f1028554` and its two focused state
tests, against the [consumer contract](../../../../api/session-reconciliation-v1.md).
No blocking state finding. Frozen reviewed file SHA-256 identities:

```text
20ae5413ab61cf015dab8c833bed25ee6f033eaff5563e186ef642c42e6342de  crates/spiralcore-mvm-live/src/campaign/session.rs
3175e6784699ab6e66e27909c9d25a93a137e6f1377f12aa93082dd5078429c3  crates/spiralcore-mvm-live/src/campaign/session/tests.rs
```

- Original `send_started` remains authoritative: initial send burns invocation
  one once; repeated initial registration/send cannot reset the counter.
- `prepare_retry` first expires canonical service and tracker, preserving resource
  maintenance even on later refusal. Maintenance Clock/State errors precede local
  fenced/pre-initial/eight-send/lifetime guards; this is deliberate, consistent
  with existing post-send registration maintenance. Original mono/Unix expiry
  remains fixed; expiry can release active credit without forgetting identity.
- Preparation re-registers the retained exact encoded tracker attempt and requires
  the same SendHandle. It never polls, extracts/clones plaintext Reply, resets
  observations/ACK/event count, reseals, advances ordering or replaces Call state.
- `start_retry` rechecks local eligibility and checked-increments before transport;
  the ninth total invocation refuses. These separate private methods are not an
  authorization permit: the consumer must sequence fresh preparation, invocation
  burn and final `retry_call_prepared` validation outside broker dispatch. The
  adapter remains the original Call/generation/commitment/lifecycle send authority.

The focused tests use a real canonical Reply and charged payload, cover cumulative
eight-send/sixteen-event bounds and original expiry/refusal preservation. Their
ready-facts preparation loop tests state/accounting, not a physical retry or
permission to resend while FIFO-blocked. The future wrapper must check completion
first and invoke this seam only when current facts are missing. The lead reports
2/2 focused GREEN after a retained expiry-maintenance failure and correction;
this reviewer ran no tests/builds. Public entry, signal, receiver preamble and
end-to-end consumer behavior remain under construction and outside this verdict.
This is not feature/release, platform, performance or full-C8 approval.
