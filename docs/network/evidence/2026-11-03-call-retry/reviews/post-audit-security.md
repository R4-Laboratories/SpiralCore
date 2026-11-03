# Registered Call retry: Step 2 final security confirmation

**PASS — local final security/evidence confirmation**, 2026-11-03, following
the [Step 1 procedural PASS](../coverage/procedural-gate.md). Reuses the
[source security approval](source-security.md) and [test/manifest review](tests-infra.md);
no new source audit, build, test or measurement was performed.

Frozen source identities match the reviewed/qualified records:

```text
5563e59811266047b582651aa6e08682ff3c368d417c2aabdbee8930a1217794  crates/spiralcore-mvm-adapter/src/service.rs
a81c0ff09047261e0430af853ad030a10fa67f0a574a143cbaade9d393d991fb  crates/spiralcore-mvm-adapter/tests/call_send.rs
11c07ee150221854383254f13730e771fa1b013b3eb0d0ed42335ca5bc466cac  docs/platforms/mvm-mailbox-portable.json
```

Retained normal-release results total **127 passed, zero failed/ignored, in 22
result scopes**; release and warnings-denied Clippy exits are zero. Selective
coverage records **125 passed in 19 separately protected/retained binaries**,
with source verification 285/285 before and after. Changed mapped production
lines are **31/31 (100%)**; whole production `service.rs` is **2,010/2,169
(92.67%)**, with all **159** uncovered lines retained. Branch mapping is NA,
not a branch-coverage result. The two release example tests explain the distinct
test totals. Missing-API RED and subsequent semantic qualification stay distinct.

The compact local packet is approximately 548 KiB; raw instrumented objects and
profiles remain remote with custody hashes. Retained records support original
request/Reply ownership, typed pre-connect refusals, unchanged charged payload
and accounting, strict-send compatibility and uncertain-write behavior. No new
authority, wire, timer, budget or campaign consumer is introduced. The API remains
unused by the normal engine: no performance or end-to-end recovery claim follows.

At this confirmation, hosted runs **41649886275** (Linux) and **41649886300**
(portable) are still active per the lead's recorded checkpoint. This is a local
PASS, conditional on their successful final exact-source qualification and the
protected merge gate; it is not hosted PASS or a merge receipt. Future untracked
reconciliation drafts are outside this approval. Full C8, closed-stream resume,
general ACK-loss recovery, scale/platform/WAN and parent goal gates remain open.
