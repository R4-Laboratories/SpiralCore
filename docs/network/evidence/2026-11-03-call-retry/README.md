# Registered Call retry checkpoint

[Public report](../../registered-call-retry-results-2026-11-03.md) ·
[API contract](../../../api/registered-call-retry-v1.md) ·
[Implementation plan](../../../plans/2026-11-03-registered-call-retry.md).

Source PR #64 merged at `5fb35d928a10a9cb924c78f1d574199b8f8772c5`.
Reviewed/hosted head `d14271dd`, tested merge `1746e57c` and actual source merge
have identical tree `e5c32fa15b132ca1ad28cfdc5914787079b18df2`.
Crate source remains identical to qualified `4d34c6a7`.

The retained missing-API RED at `7414ba28` is compiler absence: exit 101,
two E0599 errors. The initial three-test GREEN has its distinct pre-formatting
hashes. Expanded/final six-test gates retain exact original Call/Reply bytes,
charged payload ownership, one-time extraction, 27 pre-connect refusals and five
transport faults including a completed authorized write followed by an error.
Passing expanded cases are qualification results, not invented additional REDs.

- [Validation](validation/): 127 adapter all-target release tests, zero failures
  or ignored tests, strict Clippy, complete 285-entry source checks before/after.
- [Coverage](coverage/README.md): 125 overlapping tests in 19 protected fresh
  binaries; changed executable lines 31/31 (100%), mapped production service.rs
  2,010/2,169 (92.67%), all 159 unhit lines retained, branch mapping unavailable.
  Two example tests account for the normal all-target count of 127.
- [Hosted custody](qualification/portable/verification.md): seven native-client
  rows of 92 each (644), eight separate Unix storage checks; Linux live92 and
  shared135 (8 object-v1 +127 adapter), strict Clippy/default guest/analyzers.
  All seven original ZIP digests and 236 inner checksums verify. Original archives
  remain; only duplicate extracted copies were removed after verification.
- [Reviews](reviews/): source Rust/security, test/manifest infra, procedural
  Step 1 and final parallel confirmations. The security confirmation retains its
  earlier pending-CI checkpoint; hosted custody and actual merge resolve that
  condition without changing the approved source.

These test scopes overlap. Windows durable storage is not executed, and native
client rows do not establish native Windows MVM, guest recovery, WAN or fleet
capacity. Raw coverage profiles, LCOV and executed instrumented binaries remain
on Wick; no Mac build target, payload store or VM disk was created here. The
normal transfer engine does not call the new API, so no new performance result
or repeat native campaign is claimed. Full C8, endpoint reconciliation, general
ACK-loss recovery, closed-stream resume and the parent roadmap remain open.
