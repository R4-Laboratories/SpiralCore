# rc.9 persistent admitted dispatch

Networking development source
[`eb22dd2f724583bb0a12e2526046605e0ce05d37`](https://github.com/Casey-allard/mvm/commit/eb22dd2f724583bb0a12e2526046605e0ce05d37),
tree `28728a915bf963acdec87be7524ccdd67a3bb612`, reconciles canonical rc.9
`77a91c3` and persistent admitted dispatch `e01504e`/[MVM #5651](https://github.com/tinylabscom/mvm/pull/5651).
The protocol-2 rollback remains intact. Selected consumers stay **MVM f2ddbcbf /
shared core 8994951b**. The unchanged original readiness test still refuses
before Ready: **FlowMux requires a bound audit sink**. This is a development
component checkpoint; neither #258 nor the five-part MVM delivery is complete.

[PR #325](https://github.com/R4-Laboratories/SpiralCore/pull/325) merged at
`c9ffa5354efa1d37811d68cb79165b4e17dc8c85`, with identical reviewed/native-CI/release tree
`881a02ea706883194b0127a42709a81ab5342dec`. The final workflow head is
`4bc688eaff33ebd959fc3cb9008835d449c698c2`. [Measurements](measurements.json)
retain exact source, commands, profiles, costs, original artifacts and raw-receipt
hashes. All large artifacts are on HDD under `/mnt/plexpool/spiralcore-work`.

## Behavior and ownership

Ordinary persistent starts move the one-use admitted plan, actual writable-image
locks and retained verified runtime through canonical client→hostd dispatch.
Mailbox startup uses the returned post-gate config. Grant reporting, host-budget
charge, volume commit and launch acceptance remain after authenticated readiness.
CLI transient finalization preserves its own audit timing and uncertain-stop path.

Original object starts retain their dedicated dispatch and original-worker join.
Owned journals retain the actual anchor/kernel, prepared pack/runtime, helper and
process dispositions, rollback and retirement carrier. Journal-specific pack
preparation remains with those original owners. The shared ordinary/CLI/persistent
tail refuses a selected object pair before backend/state effects because it cannot
return both original worker owners. The existing refusal assertion remains and
now covers all three entrypoints.

The original `RunPosture::verify_launch_runtime` checks the persistent final config,
signed identity and required mailbox reader. Canonical prepared built-runtime
verification remains separate; retained release config is checked immediately
before custodied backend handoff. No competing protocol, widened grant, alternate
audit sink, deadline extension or suppressed readiness assertion is introduced.

## Refusal audit regression

The intermediate 638 merge moved Unix journal `require_servable` outside the old
failure handler. It returned a refusal without the original `backend-start`
`plan.failed` and failed-session seal. The corrected helper restores them before
fresh-state commitment, returns the original error and exits before a later
dispatcher could record the failure again. Original b60 Unix scope is preserved;
this is not broader non-Unix installed-journal qualification.

The regression drives the actual servability helper with the existing real
no-mailbox admission. The valid no-mailbox control succeeds without a failure
or seal. An unadmitted mailbox config projection must refuse with exactly one
failure at `backend-start`, one seal, no launched record and no backend/state
effect. It supplies no installed-journal authority or complete finish/VM witness;
fresh-state placement is source-reviewed.

RED source `c1c5fadcbc1c1e6cd7ae960df898fcf5b1b0ca2e`, tree
`cbc309e23bb8c2258203e3da2891a4456e97905d`, produced **0 passed /1 failed**:
the failed-record count was **0 instead of 1**. Its original executable is
145,003,224 bytes, SHA-256
`541577310c9876285cb5c0cd3a5ae2241fea095c204cacde9d5103193d3b6e6b`.
All 4,832 tracked RED files match operator Git blobs/modes/SHA; its complete
source/Git manifest is unchanged. The final one-line `inspect_err` correction
passes the unchanged assertion. RED's package-only features and GREEN's
workspace-unified features are separately recorded; both retain opt0, debug
assertions and overflow checks. This is not a matched-feature performance pair.

## Qualification

| Exact final-source scope | Observed result |
|---|---|
| Canonical policies | All 88 pass |
| Formatting; native Linux workspace/all-target check | Pass |
| Required-feature BDD compilation | Pass; scenarios NOT_RUN |
| Strict workspace/all-target Clippy | Pass |
| Executable selection | 293 tests + 65 other; 358 rebuilt / 0 verified same-root reuse |
| Focused Linux | 20 scopes: 96 passed / 0 failed / 6 ignored |
| Exact full 293 binary harnesses | **22,136 passed / 1 original audit failure / 72 ignored** |
| Rustdoc | 29 passed / 0 failed / 8 ignored; 28 sections / 30 harness groups |
| Native macOS ARM64 | Eight scopes: **12 passed / 0 failed / 0 ignored**; strict hostd/agentd/client all-target Clippy passes |

Full command and wrapper retain **101**. The sole failed case remains
`registered_prepare::registered_direct_tcp_transfers_exact_bytes_and_refunds_a_failed_peer`.
This is not an all-green MVM workspace or release. Every original source hash,
prepared file-set and 358-executable custody guard is zero. All 293 unique Cargo
Running paths match the expected set and complete their outer footers; only the
original registered-direct-TCP target is reported failed. Nested binary footers
are excluded from the outer totals. All 5004 original inputs and
the exact 5005-file prepared baseline remain unchanged.
The canonical empty producer lock is checked by direct name/type/inode/link/
owner/empty-content rules; unknown additions do not disappear from comparison.
All 18 synthetic guard controls pass.

The literal `just check::gated` wrapper is **NOT_RUN** because just/cargo-zigbuild/
zig are unavailable. Native Linux all-target and explicit `features=bdd`
compilation cover the recipe's two documented blind spots. No exact wrapper,
all-features, cross-platform or BDD-execution claim follows.

Before focused/full execution, all 358 current executables were archived under
the target lock, replay-hashed and original-post-hashed. Archive:
**1,106,719,336 bytes**, SHA-256 `2082aeef242b05fe211f6333c47484c2ef5739bb7fe4501de952dabe561e4d7a`.
Native [run 44020946880](https://github.com/R4-Laboratories/SpiralCore/actions/runs/44020946880)
retains artifact **14302155365**, **85,650,969 bytes**, SHA-256
`0be403106cd5504d18ac52d490daef92f8a8c29e8252fb22433cba590bb241ce`. All 69 ZIP members pass CRC/hash checks;
all 4,832 source/Git rows match the operator proof. Four actual fresh
compiler-selected ARM64 Mach-O MH_EXECUTE binaries match original/copy pre/final
hashes. Raw Cargo records, selectors, lists, exits, outer footers and BSD timings
are retained. Printed 0.00 CPU is finite precision, not zero cost.

## Actual costs and bytes

The full command records **8:39.40 wall /187.09s user /
93.27s system CPU**, maximum RSS **4,300,944 KiB**.
Separate fetch, policy-build/policy, format, workspace, BDD, Clippy and test-build
costs are retained. These qualification/setup/whole-command costs are not
send-only transfer time or physical energy.

Two original helper registrations sample peaks
**4/4 threads**. Each moves
**5,654 request / 5,024 response framed IPC bytes**;
the partial signer sends/receives **5/0 bytes**.
Each original chain retains 32 entries/exact 19,224 bytes. Setup→verified retirement:
**2.526325440s**; signing/partial-frame window:
**2.389280288s**. Whole two-test command:
**0:02.54 wall / 0.12s user / 0.08s system**.
Helper proc ticks cover post-registration→post-signing, excluding retirement;
no full setup or per-side CPU is inferred. IPC bytes are not WAN/payload traffic.
Two helpers are not two Ready endpoints or booted VMs. The 24-worker endpoint
boundary does not fund or qualify the new destination-authority/scheduler roles.

## Ari's integration path and open delivery gates

The [audit-interface request](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6935221842)
remains unanswered at the recorded refresh. Relevant canonical sink, recorder,
endpoint binding and signer interfaces are unchanged through e015. Original
signer append exists; the installed child still lacks canonical durable
journal-side epoch-health capability and missed-decision accounting under the
original deadline. Pre-confinement construction grants no later durable writes.
No broad audit-directory grant, disposable health relocation or late append is
substituted.

Ari's [current milestone](https://github.com/tinylabscom/mvm/issues/5255#issuecomment-6937094235)
is explicit encrypted A→DEST durable acceptance→A exits→independently admitted B
receives and ACKs, with restart/lost-ACK/retry/refusal coverage. His
[bounded destination-authority decision](https://github.com/tinylabscom/mvm/issues/5255#issuecomment-6938079069)
keeps it in existing hostd, with scoped receipt/history/anchor custody,
strict currentness, commit-before-receipt and existing-only recovery. Scheduler
stays keyless, payload keys endpoint-owned; approval is to build/qualify, not
activate/install. The [signed-selector seam](https://github.com/tinylabscom/mvm/issues/4629#issuecomment-6937564728)
remains with the coordinated integration stream. No competing schema, queue,
receipt purpose or authority is added. Private models remain unmerged/unimported.
The [audit-only #5590 clarification](https://github.com/tinylabscom/mvm/issues/5538#issuecomment-6938605529)
supplies no new positive mailbox authority or Call/Cast/Reply schema.

Runtime observation `2026-11-17T05:19:21.493276+00:00` finds no published rc.9 guest
release. Validation-only run 43986564433 uses 6cd4036 and finished cancelled overall;
guest-bins/Linux archive
smoke passing is not accepted source/runtime pairing, signature authentication
or actual boot for this final development source. Visible source-build host-helper
archives/signature bundles are separate from the required guest runtime.

A final read-only refresh observes canonical `aba2f6173a65b6c2e065105e949fecc07ea21c89`,
merged [#5647](https://github.com/tinylabscom/mvm/pull/5647). It adds a pure bounded
I/O-free Call/Reply pending/tombstone core and ADR update, explicitly unwired to
services. Its three-file delta is outside frozen eb22 qualification; it does not
supply the missing audit interface. No duplicate conversation authority or
service integration is invented here. Canonical main is clean/synchronized at
that observed head; the next bounded reconciliation must adopt and qualify it.

Both [#258](https://github.com/R4-Laboratories/SpiralCore/issues/258) criteria
remain open: joined original-parent/selected-child iterator/allocation peaks,
complete transient/native funding, remaining identity/retirement/quota/negative/
crash/recovery and platform/default-profile gates. All five
[#42](https://github.com/R4-Laboratories/SpiralCore/issues/42) outcomes stay required:
installed ownership/aggregate budgets/shutdown/recovery; sustained persistent
bidirectional delivery/retention/expiry/recovery; shared-core FlowMux and separately
admitted access; explicit supported role/version/architecture/backend matrix and
executed Linux/macOS/Windows two-host WAN; measured capacity/fairness/backpressure/
tails/recovery/saturation before the complete qualified upstream contribution.
Ari's compatible signed runtime, accepted source pairing and private MVMD remain
external gates. No final upstream MVM PR is submitted.

## Retained failures and disk hygiene

Intermediate 638's separate native 11-test run 44019981327 retains its original
85,637,675-byte ZIP/four binaries; it does not qualify this final correction.
Initial 638 Linux policies were administratively stopped before completion;
raw records and the 29,560,568-byte policy binary remain. Its zero shell-trap
receipt is not completion. An initial guard-receipt redirection corrupted JSON;
the malformed original is retained, separate-output rerun passes all 18 controls,
and no product source/cache changed. Earlier three lost native failed-upload
preimages/manifests/timings and 79 historical overwritten executables remain
unrecovered; these new results have independent custody.

Builds/cache/raw receipts/original ZIPs/archives stay on HDD. No local target,
payload store or VM disk is introduced. Local proof copies are removed only
after surviving length/SHA checks. Runtime files, saved sessions and unrelated
work are preserved; qBittorrent incomplete/media paths are untouched. The former
107GiB scratch stays verified/private-archived on HDD.
