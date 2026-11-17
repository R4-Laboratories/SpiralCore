# Canonical Call/Reply core reconciliation

Networking development
[`86899304562ed3a2f4ef856762dd6eeede20caff`](https://github.com/Casey-allard/mvm/commit/86899304562ed3a2f4ef856762dd6eeede20caff),
tree `19fa1bdd481ce503ca3b7b0d839757205805e5df`, merges Ari's canonical
[`aba2f61` / #5647](https://github.com/tinylabscom/mvm/pull/5647) into the previously
reviewed `eb22dd2` networking source. Exactly three files change: the canonical
376-line conversation module, its three-line mailbox export and the 207-line
ADR057 amendment. The module and ADR match incoming canonical Git blobs.
All other inherited paths, including original ownership/deadline/readiness
assertions, remain unchanged.

[Native-control PR #327](https://github.com/R4-Laboratories/SpiralCore/pull/327)
merged at `67101e6b3bb53f5785ba8c009c2ce4752fd13bab` with identical reviewed/native-CI/release tree
`20a5c4e5986a12943f3c7254469f856757c1fee7`. The final reviewed workflow head is
`f645249f82c5adbb5d2daa4c9cc39f4842e216be`.
[Measurements](measurements.json) retain exact source, executable/artifact identities,
commands, bytes, CPU/lifecycle costs and raw-receipt hashes. The
[predecessor packet](../2026-11-17-rc9-persistent-dispatch/README.md) retains its
separate source-specific results and historical failed/lost outcomes.

Selected consumers remain **MVM f2ddbcbf / shared core 8994951b**, in both direct
core dependencies and all five locked packages. This development result does
not promote a selected installed consumer. The unchanged original readiness
assertion still refuses before Ready: **FlowMux requires a bound audit sink**.
Both #258 criteria and the full five-part delivery remain open.

## Contract and authority

The conversation core remains pure and unwired. Its exact principal/epoch/
correlation key rejects live duplicates, remembered reuse and a full pending
table without evicting a live call. Replies distinguish matched, late, duplicate,
unknown and wrong-principal/epoch cases. Expiration includes `deadline <= now`;
epoch drop resolves the matching pending state as advisory Unavailable.

The caller owns time and admission. `complete` and `register` consult no clock;
a future production owner must expire before deadline-dependent completion,
check admission/currentness and authenticate the reply separately. Epoch drop
does not revoke authority or prohibit a later registration. The bounded ring
forgets old correlations, so caller non-reuse remains required beyond its
window; a zero-sized ring retains none. Timeout/cancel means the caller stopped
waiting, not non-execution, authoritative termination or permission to return
host budget credit.

These are logical entry-count bounds. The maps/sets/ring and expiration/epoch
vectors allocate; this is not a fallible funded-allocation, RSS, transient-memory,
durable receipt or process-custody witness for #258. No service table, worker,
queue, audit sink, protocol fallback, guest frame or new authority is activated.
Original mailbox, host FlowMux and separately admitted owner-access semantics
remain distinct. ADR D1–D6 stay Proposed. D7 preserves MVMD/Iroh mailbox peer
transport ownership; it does not replace separately admitted host forwarding.
Private MVMD integration remains an external gate.

## Executed final-source qualification

| Scope | Observed result |
|---|---|
| Canonical check-all | All 88 policy gates pass |
| Format / native Linux workspace-all-target compilation | Pass |
| Explicit required-feature BDD compilation | Pass; scenarios NOT_RUN |
| Strict workspace-all-target Clippy | Pass |
| Selected executable set | 293 test + 65 other; 358 rebuilt / 0 verified same-root reuse |
| Focused Linux | 21 scopes: 107 passed / 0 failed / 6 ignored |
| Canonical conversation scope | 11 passed / 0 failed / 0 ignored |
| Repeated full 293 harnesses, one bulk fixture on HDD | **22,147 passed / 1 original audit-sink failure / 72 ignored** |
| Rustdoc | 29 passed / 0 failed / 8 ignored; 28 sections / 30 harness groups |
| Native macOS ARM64 | Nine scopes: **23 passed / 0 failed / 0 ignored**; strict hostd/agentd/client/core all-target Clippy passes |

Full Cargo command and wrapper retain **101**. The sole failed case is still
`registered_prepare::registered_direct_tcp_transfers_exact_bytes_and_refunds_a_failed_peer`.
All 293 unique expected Cargo binary paths complete their outer footers; nested
subprocess footers are excluded. The only failed target remains the original
registered-direct-TCP test. This is a failed full-workspace result with a known
installed gate, not an all-green MVM release.

Every original source hash, exact prepared file-set and 358-executable before/
after guard is zero. All 5,007 original inputs and the separately
collected exact 5,008-file prepared before/postverified manifests
remain unchanged. The only producer preparation is its canonical empty lock;
direct name/type/inode/link/owner/content checks reject unknown or substituted
objects. All 18 synthetic guard controls pass. The prior 358 executable preimages
and prior archive were verified before shared-target cleanup/reuse.

The literal `just check::gated` wrapper is **NOT_RUN**: just/cargo-zigbuild/zig
are unavailable. Native Linux all-target and explicit conformance `features=bdd`
compilation cover its documented blind spots. No exact-wrapper, all-features,
cross-platform or BDD-execution claim follows.

Before focused/full execution, the current 358 executables were archived under
the shared target lock, replay-hashed and original-post-hashed. Archive:
**1,106,446,409 bytes**, SHA-256 `61846bcfb1e2f5d628e64c87287aa13028a61d33c313957590113908f23f37eb`.
Native [run 44118810800](https://github.com/R4-Laboratories/SpiralCore/actions/runs/44118810800)
retains artifact **14345337640**, **107,418,535 bytes**, SHA-256
`63aab31a5d52c238f2bca90fc973046a28ad0e360e11e51b16bf8999fa702359`. All 76 unique ZIP members pass CRC/hash checks;
all 4,833 source SHA/Git path/mode/blob rows match the operator proof. All five
retained binaries are actual fresh compiler-selected ARM64 Mach-O MH_EXECUTE
files with matching original/copy pre/final hashes. Raw Cargo records, exact
selectors/lists, exits, last outer footers and BSD timing records remain on HDD.

## Bulk-fixture quota refusal and explicit placement correction

The initial full run remains **22,146 passed / 2 failed / 72 ignored**, with
29 passing doctests and every source/executable guard zero. Alongside the original
Ready failure, the unchanged streaming-bundle memory fixture failed during
installation with `tar read: Disk quota exceeded (os error122)`.

A bounded failed-syscall trace of the same frozen ELF reproduces kernel
`write(.../big.mvmpkg, ...) = -1 EDQUOT` on quota-enabled tmpfs, now during sealing.
The HDD counterpart passes the exact test. Both retain the 96 MiB patterned
rootfs, 8 MiB counting-allocator bound, signature/manifest checks and installed
length assertion. The trace establishes a carrier-dependent quota refusal,
not its threshold. Original extraction and diagnostic sealing failures remain
separate observations; tracer overhead is included only in diagnostic timings.

The repeated full campaign uses a reviewed qualification-only Rust Cargo runner.
It matches only the exact compiler-selected bundle-fixture path and SHA-256,
requires its private HDD scratch directory, and directly execs the original
binary with unchanged arguments. Other targets keep their original short tmpfs
environment and original exit status. There is no runtime fallback, source patch,
quota increase, test skip, heap/deadline/security relaxation or service activation.
All 293 original Cargo paths/outer footers and all 358 payload-ELF hashes remain
required. The runner source/binary are hash-checked before/after and its empty
HDD scratch must be removed. Original input and executable identities are shared
between campaigns; their outcomes and filesystem profiles are not conflated.

The actual full-run marker identifies exactly one routed fixture. Its passing
result supports fixed-heap behavior with HDD-backed temporary files, not that
the original tmpfs quota is sufficient or that #258 funded process/iterator
memory is qualified. Runner/hash-check overhead is whole-campaign qualification
cost. Full1 remains failed even when this repeated profile passes the bundle test.

## Actual measurement domains

The full qualification command records **8:29.13 wall**,
**186.60s user / 87.76s system CPU**,
maximum RSS **4,299,876 KiB**. Separate fetch, policy-build/policy,
format, workspace, BDD, Clippy and test-build costs are retained. These are
qualification/setup/whole-command costs, not send-only transfer time or energy.
The pure conversation tests supply deterministic state coverage, not network
throughput, service timing or platform capacity. Printed 0.00 CPU is finite
precision rather than proof of zero computation.

The actual original-helper witness samples peaks **4/4 threads**. Each of two
registrations moves **5,654 request / 5,024 response framed IPC bytes**;
partial signer sends/receives **5/0 bytes**.
Each original chain retains 32 entries and exactly 19,224 bytes. Setup→verified
retirement: **2.510244505s**; signing/partial-frame window:
**2.386256665s**. Whole two-test command records
**0:02.52 wall / 0.12s user / 0.07s system CPU**.
Helper proc ticks cover post-registration→post-signing and exclude retirement;
no setup or per-side end-to-end CPU is inferred. IPC is not WAN/payload traffic;
two helpers are not two Ready endpoints or two booted VMs. The unchanged
24-worker endpoint boundary does not include the proposed destination-authority
or shared-scheduler role costs. Those finite profiles and density remain gated.

## Ari's current path and outstanding gates

The [audit-interface request](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6935221842)
remains unanswered at the recorded refresh. Relevant canonical sink, recorder,
endpoint binding and signer interfaces are unchanged through the observed later
main. The installed child still lacks canonical durable journal-side epoch-health
capability and missed-decision accounting under its original deadline. No wider
audit write grant, disposable health relocation, late append or fake sink is
substituted to pass readiness.

Ari's [current production checkpoint](https://github.com/tinylabscom/mvm/issues/5255#issuecomment-6939724669)
publishes [#5668](https://github.com/tinylabscom/mvm/pull/5668), head `453d8ae`, for
review: signed V3 selection/early refusal, retained trusted-host local V2 receipts
and a runtime-cache publication repair. It is open/unmerged and is not imported.
Its checks do not establish positive V3 destination authority or the full service.
The approved milestone remains explicit encrypted A→DEST durable acceptance→A
exits→independently admitted B receives and ACKs, with restart/lost-ACK/retry/refusal
coverage. Destination receipt/history/anchor custody stays scoped in existing
hostd; scheduler keyless, payload keys endpoint-owned, strict currentness and
commit-before-receipt required. No competing schema, queue or authority is added.

Ari identifies [tag run 44020798280](https://github.com/tinylabscom/mvm/actions/runs/44020798280)
at rc.9 source `77a91c3` as the authoritative runtime-build checkpoint. Guest-bins,
proof-input and signed-candidate stages pass, but Linux Firecracker shard0 fails;
release/promotion are skipped. Candidate artifact **14303986195** is observed through
metadata/CI only; it is not downloaded, authenticated or booted for this source.
Neither rc.9 nor rc.10 has a visible published release endpoint at the observation.
A compatible published runtime and accepted source/runtime pairing remain gates.

Later main `3d474dd` contains eleven commits / 207 changed files beyond aba2f61,
including lifecycle/current-generation/history/runtime/resource changes and rc.10.
That separate source delta is observed, not qualified by this frozen campaign.
The canonical main checkout is kept clean and synchronized. This bounded merge
adds no unreviewed pending contribution or later-main acceptance claim.

Both [#258](https://github.com/R4-Laboratories/SpiralCore/issues/258) criteria
remain open: joined original-parent/selected-child iterator/allocation peaks,
complete transient/native funding and remaining identity/retirement/quota/
negative/crash/recovery/default-profile/platform witnesses. All five
[#42](https://github.com/R4-Laboratories/SpiralCore/issues/42) outcomes remain:
installed ownership/aggregate budgets/shutdown/recovery; sustained persistent
bidirectional delivery/retention/expiry/recovery; shared-core FlowMux and separately
admitted access; explicit supported role/version/architecture/backend matrix with
executed Linux/macOS/Windows two-host WAN; measured capacity/fairness/backpressure/
tails/recovery/saturation before the complete qualified upstream contribution.
Private MVMD, compatible signed runtime and accepted source pairing stay external.
No final upstream MVM contribution is submitted.

## Retention and local disk hygiene

Builds, Cargo cache, actual executables, original ZIP and raw/archive records stay
on HDD under `/mnt/plexpool/spiralcore-work`. No local target, payload store or VM
disk is introduced. Owned local proof duplicates are removed only after surviving
HDD length/SHA checks. Runtime/session, torrent-incomplete and media paths remain
untouched. Prior stopped/failed/malformed/profile-distinct evidence and unrecovered
historical preimages remain in the predecessor packet; this run does not recover
them or rewrite their outcomes.
