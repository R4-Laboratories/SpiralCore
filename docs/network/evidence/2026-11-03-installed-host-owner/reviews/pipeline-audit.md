# PR #68 component pipeline audit — Step 1

**PROCEDURAL PASS; required hosted qualification remains OPEN / MERGE HOLD.**
Eligible harness-optimizer applied the documented sequential quality gate to
this task's retained commands, outputs, exits, commit checkpoints and reviews.
No evasion or completed-task stale claim was found. This permits Step 2 reviews;
it is not source merge approval or completion of B6a. No global shell history,
private process argv, product/test edits, builds or tests were used for this audit.

The product pin is d4f2ed412ae961c5d2bd267caf0ec1a2628157c1 / installed_owner.rs
SHA-256 b75172d56b284adf9b69417fac3d6835fd9cb2edf311b871cb518222bda1ae57.
Current head at check is 64d93fed72fdac6784d99e6e1a258052bba605f6, with later
documentation/workflow changes and unchanged product/tests. API/plan preceded
the TDD sequence. Verified commit order and actual failure output:

| Checkpoint | Retained evidence and disposition |
| --- | --- |
| API absence | 3caf00ee harness precedes product 9fc90dfe; module is absent in the test commit; api-red exits101 on missing library source |
| First implementation | lifecycle-first exits101 with5 passes/4 typed-refusal failures; 99882ffa correction yields9 entries passing; original failures retained |
| ABA and durability | c1677e75 deterministic wrong-FD SH/EX and postrename-sync REDs precede4d1ad793; trust-boundary-green passes2 unit+9 lifecycle entries; expanded intent barriers pass4+9 |
| ACL | 0054ccc6 exposes all five privacy fixtures; a0347f23 still fails ordinary no-ACL ENOENT; d4f2ed41 fixes that case, focused1PASS and complete Mac4+10PASS; empty-selection and failed attempts remain distinct |
| Prior independent review | core-code-rust.md historical BLOCK is retained; core-code-rust-final.md approves exactb751 delta and resolves it, without native-integration claims |

Read validation/commands.md's expanded invocations against actual Cargo output
and numeric exits. The normal Linux parallel component run remains FAIL; its
premature core-final-linux filename is explicitly corrected in the command
receipt. Same-source exact and serial lifecycle execution passes; bounded owned
child pre-exec-delay tracing demonstrates temporary inherited-description lock
custody, while attribution of the original untraced incident remains labelled
inference. Serial qualification preserves Busy and all assertions. Linux has
4 unit plus9 lifecycle entries, Mac4 plus10; each includes one subprocess helper.
These are overlapping component scopes, not native VM/resource recovery.

Actual main-linux output and exit0 support **115 live checks** from
`--release --lib --tests` (not an all-target test run), across nine nonempty
targets and one explicit zero-test binary. Strict Linux main Clippy used
`--all-targets -- -D warnings` and exits0. Mac focused Clippy also exits0.
Main/focused private unit cases overlap and are not summed as unique scenarios.

Read coverage/collect.sh and actual protected-binary execution/export receipts.
It instruments only the bounded standalone harness, jobs2, with two fresh copied
mode0500 objects, isolated profiles/exports and serial execution:4+9PASS. Five
source pins verify against current files and match before/after; both executed
objects' after-checks pass. Independently recounted line-hits.tsv:
**402/428 mapped production lines (93.93%),26 unhit**. The module is new, so
changed and whole mapped-module denominators coincide. Linux-unmapped macOS ACL
code and cfg-test hooks are explicitly outside the denominator; branches NA,
not100%. Test/merge/export stderr is zero; build.stderr is compiler progress.
Coverage time is not performance, and raw objects/profiles stay on Wick.

Task-local .agent-sync/ROUTING.md has no in-progress claim rows. DAILY.md is
absent; explicit task messages, retained invocations/results and review records
provide this task's scoped ledger instead. No global ledger/config was changed.
Official failed/positive evidence remains preserved; no cleanup was required.

**Hosted hold:** directly queried original run41697401676: completed/failure,
head da575216, zero jobs. It is not executed platform evidence. At this check,
corrected head64d93fed has Linux41697837631 in progress and installed-owner
41697837824 queued; accepted scheduling is not execution PASS. Merge requires
successful exact-head required jobs and scoped source/artifact custody, plus
final independent Step2 confirmations. No hosted pass or delivered merge is
claimed here. Full installed-default-path/aggregate-budget closure, actual
worker/VMM lease handoff, B6a/B6b, scale and WAN remain open.
