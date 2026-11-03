# Explicit session reconciliation: final production source security review

**PASS — scoped product source, not release qualification**, 2026-11-03.
Reviewed `e984d2d64cdf0483d50d6d59319276c85e7df644` →
`07e1bf837458af0b5d3bb7d050d7749a2071263d`; subsequent head
`b80bfdcd35e5674b1549efc4a4eafec4ae9b831f` changes only a deadline fixture and
preserves these product hashes. No blocking product trust-boundary finding.
Roster `d1927d9e7ce51354` freshly verifies security-reviewer; its bootstrap
instructions were read. Reviewer authored contracts/tests, not product changes;
this is an independent product-source check, not independent approval of those tests.

```text
d55ca44a3812d80c88d94811c7889fbf142340f16776a68d1b8129702a4318b2  crates/spiralcore-mvm-adapter/src/live.rs
27a6827d592847a3c560eb4187660ee3dae57e3f583b9964eecbba3c4c672ece  crates/spiralcore-mvm-live/src/campaign.rs
7ff5e5ec5ed907334fbfe200f37b3ed6f3e537f43a8da955a25b6196accb1b86  crates/spiralcore-mvm-live/src/campaign/canonical.rs
601b40bad57fe4b1f39f04db531aa73c95dd862d965e9e0ad7d4ab96b3797708  crates/spiralcore-mvm-live/src/campaign/reconciliation.rs
20ae5413ab61cf015dab8c833bed25ee6f033eaff5563e186ef642c42e6342de  crates/spiralcore-mvm-live/src/campaign/session.rs
d22755584c91af51eb4532bc7fe84fde26cfa115fb72950fb30c58985eb7c1a1  crates/spiralcore-mvm-live/src/orchestrator.rs
```

- Public opt-in embedding gives its callback only an opaque one-Call signal,
  outside broker dispatch. Private construction enforces one queued command;
  invocation exit disconnects stale signals. It exposes no payload, bootstrap,
  socket or authority and starts no thread. Strict sender/native guest selection
  allocates no channel and does not poll Idle. No protocol, dependency, backend,
  port, capability, route or host-budget enlargement accompanies this API.
- Caller Idle alone emits no bytes. Original lifetime and ordinary completion/
  FIFO checks precede command dequeue; current ready facts with FIFO Pending keep
  the command queued and never skip the predecessor. Retry preserves the exact
  original wire/options/Call and tracker handle, with no reseal, new sequence or
  payload poll. Fresh time precedes a burned invocation and the canonical final
  Call/generation/commitment/lifecycle send gate outside dispatch. Eight sends,
  sixteen incoming frames, fixed lifetime and the original send latch remain.
- All eight canonical write sites now use the retained fixed-deadline view.
  The previous raw-parent retry timeout reset is corrected through connect,
  channel reuse and write progress, not merely checked before invocation. The
  opaque owned Arc view exposes no raw socket/concrete type or authority bypass;
  default adapter connect, framing/counters and existing cancellation/owned fence
  semantics stay unchanged. Unowned mutex failure still cannot promise closure.
- Inactive preamble uses canonical admit-first only with room for a later ACK.
  Duplicate receipt-only handling separately rechecks original local ownership,
  old monotonic/signed lifetime and absent cache before reservation and again
  before bounded send. Canonically verified inner/outer expiry equality justifies
  the signed cap; None never proves completion. Frame sixteen cannot admit a new
  Call. Active replay instead prechecks exact current cache before admission,
  revalidates both bindings/exact cache/same tracker before Reply-first resend,
  and repeats those checks after that write before the closing actual receipt.
  No object reexecution, cache fabrication or acceptance-level invention appears.
- Errors preserve the caller identity or receiver no-resume latch. Completion
  requires original FIFO extraction or matching authenticated cache ACK; transport
  success is not remote observation. Checked v2 counters/processing phases retain
  unfinished work without emitting payloads; cumulative/process/per-Handled timing
  domains remain overlapping rather than additive or exclusive-thread CPU claims.
  The orchestrator delta is only the cfg(test) wrapper around the unchanged relay.

Retained execution receipts show late-write old/fixed exits 101/0 and six direct/
actual-relay faults plus three public campaigns GREEN. Root reports the three
boundary fixtures GREEN; the broader canonical run still had two deadline-fixture
assertion failures under correction. No all-green release/CI/final-head freeze is
credited here. This reviewer ran no builds/tests, dependency audits or unrelated
scans. Exact frozen-head confirmation follows fixture qualification separately.
General ACK loss after caller completion, closed-stream resume, cap-2,048 boundary,
long-lived scale/ownership, Windows consumer/storage and native/WAN gates remain
open. This is no performance, energy, full-C8 or parent-goal completion approval.
