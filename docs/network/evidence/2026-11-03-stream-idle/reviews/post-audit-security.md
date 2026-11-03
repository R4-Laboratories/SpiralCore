# Step 2 final security/content confirmation — receive Idle

Date: 2026-11-03. Verdict: **PASS for this bounded source/API and public evidence
delivery**. No blocking finding remains in this confirmation. It follows the
[Step 1 procedural PASS](../coverage/procedural-gate.md); no literal quality-gate
command was exposed. Full C8, the broader goal and remaining release gates are
not complete. No new audits, builds, runs or product edits occurred here.

Frozen source `960e73c59d7eab62f15a16667f95e594e1cabc8b` and live.rs SHA-256
`29010dded67185207d677dca8bcbf10a447336c96b689e0e5308aac214ec9eac`
retain the [source approval](idle-source-security.md) and approved API contract.
The [prelaunch review](idle-prelaunch.md) and
[independent measurement review](../measurement-review.md) are reused rather
than reopened. The public report, README checkpoint label and partial-C8 plan
accurately distinguish the delivered transport seam from future consumer retry.

The report preserves input/lock precedence, positive zero-prefix-only Idle,
original absolute deadline once any prefix byte is consumed, fatal owned framing
errors and strict EOF/Darwin closure behavior. It does not promise socket closure
for a contended-lock error without ownership. No error variant, wire, codec,
authentication, resource ceiling, consumer/timer or resume was introduced.

Results match the reviewed summary: 12 strict-transfer host cells/six pairs with
108 exact publications/readbacks and cleanup; two current native cells with
18 publications/18 additional keyless readbacks. Host paired losses, native
schema/unrelated losses, cold canonical overhead, CPU domains and actual lengths
remain visible. There is no fresh native before/after comparison. Both normal
engines use strict receive, so no Idle wake-CPU, timer/retry performance, energy,
send-only, causal/statistical speedup, WAN or fleet claim follows.

The final [independent qualification receipt](../qualification/portable/verification.md)
records seven original ZIP/API matches and 236 inner checksum checks, 88 tests
per seven native client rows (616) plus eight Unix storage checks. Linux release
is 92; shared scope is 123 adapter plus eight object-v1 (131). These scopes overlap
and are not pooled unique experiments or Windows MVM evidence. Tested merge
`1da75b1a` and reviewed head `960e73c5` have the same tree
`7d4d1568fd6f2143992b145abf68a790b1b1d4ae`. Original archive verification is
owned by that independent receipt and was not repeated here.

Coverage wording retains 88 overlapping tests, changed 96/97 lines (98.97%),
whole-file mapped 673/733 (91.81%), the one uncovered defensive changed line and
all 60 uncovered mapped lines. Branch mapping is unavailable. No repository,
semantic-path or MC/DC percentage is invented; instrumented timings are excluded.

Scoped packet inventory/content checks found no private bootstrap, credentials/
tokens, global argv/environment capture, payload disk/VM image or raw coverage
profile. Original compact CI/measurement archives and public receipts retain
their documented custody; protected binaries/profiles remain remote. Scripts
remain within approved owned paths and recorded batch/resource limits. Successful
host cleanup after verified-record emission is distinguished from native archive-
before-disk cleanup: 41 members and 536,870,912 apparent native disk bytes, without
claiming freed blocks or touching failed records/other labs.

The plan leaves original-identity retry, uncertainty reconciliation, explicit
resume, long retention beyond 2,048 Calls, installed ownership/shared plaintext
budgets, larger windows, remaining native roles, WAN and upstream contribution
open. This PASS supports the protected source PR and subsequent results
publication; it is not a merge receipt, certification or goal-completion claim.
