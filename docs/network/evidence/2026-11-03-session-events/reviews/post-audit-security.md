# Step 2 final security and evidence confirmation

Date: 2026-11-03. Verdict: **PASS for this bounded source/packet publication and
protected-PR merge gate**. No blocking security or evidence finding remains in
this confirmation. This is not full C8, goal completion, certification or WAN
acceptance. No literal quality-gate command was available; this follows the
documented Step 2 after the [procedural PASS](../coverage/procedural-gate.md).

The three frozen production hashes still match the prior source approvals:

| File | SHA-256 |
|---|---|
| `campaign/canonical.rs` | `8540ea581d6b7e323b6502e945031cb187cdf47ead20a26eaa242fa8e0d8969c` |
| `campaign/session.rs` | `8d8f27e50fe0a6dca43c317b26f8a3eeffce5ab2b60f6f060d442b2153010e13` |
| Adapter `receipt.rs` | `f72d2f84a675cd7221dd94fa4a10c7d84331fb7c04a95d7200d782795d07c305` |

This reuses the optional-observer, [caller](caller-late-security.md),
[receiver/deadline](receiver-replay-security.md) and
[composed source/runner](final-source-security-performance.md) approvals; it does
not reopen their audits. The cumulative batch bound is now explicitly recorded
as on-Wick `timeout 1800` in execution pins, resolving the launch precondition.

The public results report, README and partial-C8 plan agree with the independently
checked [measurement review](../measurement-review.md) and [summary](../summary.json):
22 host cells/11 pairs with 192 exact publications/readbacks, four native cells
with 36 publications/36 extra keyless readbacks, matched counters and unchanged
resource bounds. Paired medians, individual losses, schema-fit/drift regressions,
cold canonical overhead and source/baseline differences remain visible. CPU/timer
domains are separate; no causal speedup, universal cold win, energy or send-only
claim is introduced. Native-local evidence is not WAN or fleet qualification.

Coverage/qualification wording matches the procedural receipts: 91 overlapping
instrumented tests; 287/309 changed lines (92.88%), 1,495/1,582 scoped production
lines (94.50%); branch mapping unavailable. Hosted client rows are 81 each across
seven rows (567), plus eight Unix checks; Linux scopes are 92 release and 124
shared checks. They are not pooled unique experiments or native Windows MVM
evidence. The retained source-tree receipt binds tested merge `3e7c94b2` to head
`5c41d6ac` at tree `ea05391bd4f077339a6338a102b68e62d83f8815`.
The root-owned seven ZIP/236-inner-hash custody verification was confirmed from
its receipt, not repeated here.

Packet inventory and targeted publication-content checks found no private worker
bootstrap, credential/token material, global argv/environment capture, payload
disk or VM image. Retained process snapshots are the approved native PID checks.
Compact public receipts/scripts/statistics, failure records and original CI ZIPs
retain their documented custody; raw profiles/instrumented executables remain
remote. Host in-engine successful cleanup and native archive-before-owned-disk
cleanup are distinguished, with 1,073,741,824 apparent native disk bytes rather
than an asserted freed-block count. Other labs and failures remain untouched.

The plan keeps automatic retry/resume, long retention beyond 2,048 Calls, installed
host ownership, shared plaintext/cache budgets, larger windows, remaining native
roles, cross-host WAN and final upstream contribution open. No product edits,
builds, measurements, external actions or repeated archive audit occurred here.
