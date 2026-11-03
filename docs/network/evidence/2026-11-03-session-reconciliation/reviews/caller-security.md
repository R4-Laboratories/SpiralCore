# Public embedding and caller: narrow source security review

**PASS — caller/public-entry source slice**, 2026-11-03. No blocking trust-boundary
finding in committed `fdc8918` → `f95c0f4076f2d8ca442124fbbe857f56f2cf4179`.
Reuses [state review](state-security.md); receiver behavior, metrics completion
and release/end-to-end loss qualification are outside this approval.

Reviewed commit's SHA-256 identities (later receiver edits require their own pin):

```text
27a6827d592847a3c560eb4187660ee3dae57e3f583b9964eecbba3c4c672ece  crates/spiralcore-mvm-live/src/campaign.rs
dd59442c9cb662060ba9866c2f0152be7aa7ef48e05fcef1c3b6de52c6346a69  crates/spiralcore-mvm-live/src/campaign/canonical.rs
92da1719c1a45e9cbbcd33b1b9a297b00e68e9749556b76c1f5b897f5a8ba39c  crates/spiralcore-mvm-live/src/campaign/reconciliation.rs
```

- The public opt-in sender reaches the real Client/canonical path. One private
  factory owns `sync_channel(1)` and opaque endpoints; no arbitrary receiver or
  socket/authorization capability is exposed. Callback receives only the signal
  outside broker dispatch, and embedding owns its controller/thread lifecycle.
  Callback error drops the unused requests endpoint; invocation exit drops its
  moved endpoint, so stale signals cannot select a later Call. Strict sender passes None, creates no
  channel and continues strict receive, with no Idle-triggered retry.
- Opt-in uses zero-prefix Idle under the fixed original Instant. Idle first
  checks original mono/Unix lifetime; lifetime refusal maintains canonical service
  and tracker expiry. Completion/FIFO polling precedes command consumption: when
  facts are ready but delivery remains Pending, it neither consumes the command
  nor resends. Empty/disconnected requests emit no bytes. Frames retain the
  authenticated existing demultiplexer and cumulative sixteen-event bound.
- Explicit retry prepares only the exact retained tracker attempt and original
  SendHandle, without payload polling or new identity. A second fresh broker-time
  and Instant check precedes invocation burn. Transport runs after dispatch via
  the canonical retry API, which remains original Call/generation/commitment/
  status/mono/Unix/cancellation authority. No outer policy lock, reseal, sequence,
  timer-driven send or acceptance invention appears. Eight invocation and original
  lifetime bounds do not reset. A pre-send time check is not a promise that a
  blocking write completes before that Instant; transport retains its own bounds.
- Extracted `poll_ready_reply` preserves existing strict caller terminal mappings
  and one-time payload extraction. No new uncharged plaintext Reply copy is added.
  Unknown writes/errors retain original pending state; current post-send latch
  blocks a new request/re-entry rather than providing reconnect/resume.

The lead reports public TCP 1/1, signal 2/2 and state 2/2 GREEN; this reviewer ran
no tests/builds and does not credit loss recovery from those counts. Receiver
receipt behavior is unchanged in this slice. Fixed normal four-frame reporting
has not yet gained actual reconciliation fields, and failure-path processing/
preamble metrics remain final acceptance gates. No performance, Windows consumer,
native guest opt-in, general ACK-loss/resume or full-C8 claim is approved.
Only this review was written; no product/test edits or broader audit occurred.
