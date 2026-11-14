# SpiralCore / MVM complete restart context — 14 November 2026

This is a dated handoff, not a release declaration or replacement roadmap.
GitHub issues remain authoritative. Refresh live source, issues, PR comments and
CI at restart; do not repeat the broad historical audit. The final MVM deliverable
is incomplete. Do not mark the full goal complete or claim customer readiness.

## Start here in the new window

> Resume the SpiralCore/MVM delivery goal using this handoff. The immediate code
> task is finishing the bounded cooperative reply decoder in draft SpiralCore
> PR #278, especially original-failure custody on Windows ARM, then resolving
> sustained Windows timeouts without relaxing deadlines or authority checks.
> Preserve zero fidelity loss and record actual bytes, CPU and end-to-end time.
> Keep audits lean, use Rust, keep local disks clear, and publish tested/reviewed
> slices through PRs into SpiralCore origin/main. The final upstream MVM PR is
> only for the complete qualified contribution. Ari must publish the compatible
> signed guest runtime; private MVMD transport access is also an external gate.
> These gates do not prevent independent implementation work. Read the current
> repository instructions before editing. Do not weaken tests to make CI green.

The final upstream contribution is not ready. Several older MVM PRs were opened
and closed; they are not the completed final submission. Latest live list:
[upstream PR snapshot](upstream-prs-snapshot.json). The user's objective is to
finish Ari's needs, not perpetually expand audits or invent parallel protocols.

## 1. Full goal and completion boundary

The user explicitly prioritized these five deliverables:

1. Finish the installed mailbox service: worker/VM ownership, aggregate
   plaintext/cache/workspace budgets, shutdown and orphan recovery. Preserve
   independent service queues and canonical MVM authority.
2. Finish persistent-session recovery and sustained operation: final-ACK
   recovery, safe retention/expiry, sessions beyond the current 2,048-Call native
   witness, and large transfers alongside control traffic.
3. Finish FlowMux and owner-access endpoints using the same protocol/core, while
   keeping distinct authorization and lifecycle semantics. Compression runs at
   authorized plaintext endpoints. Host relays can forward encrypted envelopes.
4. Execute canonical two-host WAN and platform acceptance. Native Linux/
   Firecracker and Apple Silicon macOS/HVF witnesses exist. Windows has portable
   native component/client evidence; durable gateway/storage and end-to-end WAN
   qualification are incomplete. Windows is not an established MVM host role.
5. Freeze a supported capacity envelope and package the upstream contribution.
   Existing 2/8/32 logical-endpoint load results and bounded two-guest tests do
   not establish customer fleet capacity, maximum throughput or tenant fairness.

The original goal attachment is preserved [verbatim](original-goal-attachment.md).
Its older progress and disk-space statements are historical, not current status.
The session goal remained active. Do not narrow it to the current Windows fix.

Delivery requires source, protocol/threat-model decisions, installed adapters,
reproducible native/WAN/recovery evidence, platform/capacity limits, and security
qualification gaps. A passing component, a checkpoint merge or an open PR does
not establish that delivery.

## 2. Product architecture and accepted decisions

SpiralCore is one Rust lossless transfer/protocol core with different endpoints.
MVM is the first primary consumer; ordinary networking remains a project track.
Wasm is deferred. Do not create different protocols for mailbox, FlowMux, TUN or
owner access. Their service authority and lifecycle remain distinct.

The central contract is exact byte and length reconstruction from verified
receiver state plus the transmitted recipe/residual. Cache membership or a digest
is never authorization. No zero-fidelity exception is permitted.

Existing mechanisms include immutable content-addressed objects and ternary DAG
sharing, missing-record exchange, exact warm-object references, sparse changes,
schema fitting with compressed value/residual streams, prepared zstd state,
whole-object zstd/raw fallback, and deterministic ternary/Kuramoto/cellular-
automata recipe plus residual paths. The user explicitly considers the ternary,
Kuramoto and CA mechanisms central, not expendable side experiments. Preserve
and measure their actual runtime path. They do not eliminate information that is
absent from both transfer and verified receiver state. Oscillator simulations do
not establish identical physical time on distributed machines.

Sender codec selection was repeatedly optimized to avoid unnecessary candidate,
hashing, residual and setup work. The existing exact-repeat warm path has a
reported 600-byte transfer contract in its original recorded measurements.
Do not extrapolate that figure to every service/transport or substitute planned
frame bytes for actual wire bytes. Read original fixtures and reports before
quoting old performance as current. The latest reply-progress work changes no
codec, identity or wire format.

For MVM, a host-side service sits beside the canonical guest vsock interfaces.
Same-host nodes use host mediation; cross-host transport connects authorized host
services over the IP network. This does not give guests a NIC or bypass MVM's
policy, DNS, egress, rate, audit, signed-plan or lifecycle enforcement. FlowMux
port 5253 is not an invented mailbox endpoint. Earlier mailbox work uses port
5257; verify the current published MVM contract before wiring anything.

Compression must happen before canonical encryption at an authorized plaintext
endpoint, with authentication/decryption before decoding on the other side.
An encrypted relay cannot inspect payloads to achieve compression gains.
Arbitrary already-encrypted Internet traffic cannot be assumed compressible.
Mailbox object commit, mailbox admission, delivery and application processing are
separate acknowledgments. Preserve at-least-once semantics; do not promise
exactly-once application execution.

The user requested both separately admitted development/debug access and sealed
production management modes. This is not permission to install guest SSH or a
second network route: MVM's current agreement says workloads have no NIC and no
SSH. Use approved capability/service contracts and narrow management operations.

## 3. User priorities and operating constraints

- Implement rather than continually auditing. Use focused reviews only where
  trust boundaries/ownership require them. Do not restart the repository audit.
- Rust is the active language. No rewrite in Python. Historical Python research
  and executable cards remain references, not executable network inputs.
- Prioritize fewer bytes, then measure sender/receiver compute and total time.
  Count both directions, setup, acknowledgments, repair and relevant transport
  overhead. Do not invent send-only time by subtracting overlapping clocks.
- Compare against equally tuned whole-object zstd and applicable conventional
  baselines. Retain losing cases. No energy claim from CPU time alone.
- Keep the local HDD clear. No duplicate raw corpora, VM disks, Cargo trees or
  enormous logs. Keep compact evidence in Git, use existing shared build state,
  and remove only owned disposable data after checking evidence preservation.
- Publish reviewed/tested slices to SpiralCore origin/main via PRs. Preserve
  unfinished work on clearly labelled development branches. Do not force history
  or merge known-failing production code just to satisfy a publication request.
- Issues own status, dependencies and acceptance. Never close unchecked scope.
  Keep docs/network/current-status.md and source/main status aligned.
- User authorized own SpiralCore issue/PR workflow. Do not send new messages to
  Ari or other people without explicit applicable authorization. Previous specific
  permission to reply accepting a comment is not blanket future messaging consent.
- User approved MIT for original SpiralCore code/docs, preserving imported
  licenses and excluding third-party research artifacts from blanket licensing.
- User wants professional technical README pictures and clear OSI/workflow
  explanations for network engineers. That presentation work must not displace
  the MVM implementation critical path.
- User removed local worktrees during cleanup and was rightly concerned about
  unpublished work. Keep published source/checkpoints recoverable and clearly
  identify active paths before any cleanup. Preserve user runtime/saved sessions.

## 4. Verified repositories, branches and paths

Values below were checked during this export unless explicitly labelled prior.

| Item | State |
|---|---|
| SpiralCore repository | https://github.com/R4-Laboratories/SpiralCore |
| SpiralCore origin/main before this handoff | `4b1804308ea28698f9a03c7e8b0cfdcbe259ae15` |
| MVM public main | `50b1a239b75855150fa7fc2fd185b202c733ccdd` |
| MVM contribution fork | https://github.com/Casey-allard/mvm |
| MVM contribution branch | `fix/spiralcore-helper-config-custody-20261114` |
| MVM contribution HEAD | `639cdc36ed1a29a85a37e2606075894776747d54` |
| SpiralCore development PR | https://github.com/R4-Laboratories/SpiralCore/pull/278 |
| PR278 branch / HEAD | `codex/duplex-reply-progress` / `e03c878e932ea59139e915e56114543d41a5a37c` |
| PR278 state | OPEN, DRAFT, CONFLICTING with current main; do not merge |
| Held Unix storage PR | https://github.com/R4-Laboratories/SpiralCore/pull/260 |
| PR260 HEAD | `515b87bb9d26b8e4dca1036bef6437a191cf7aaa`, OPEN/DRAFT/HOLD |
| Previous evidence branch | `codex/reply-progress-evidence` at `853ee696003428365f671c5a1a38697b550a1445`, merged through PR279 |

Local active SpiralCore checkout:
`/Users/casey.allard/.codex/worktrees/spiralcore-mvm-resume/spiralcore`

At export creation this checkout was switched from the merged evidence branch
to `codex/restart-context-20261114`, based on origin/main, to publish this handoff.
The production repair is therefore **not** checked out here. Switch deliberately
to the published PR278 branch to edit it. Preexisting untracked `.serena/` is
present and must be preserved. There were no unpublished production code changes.

MVM contribution worktree:
`/Users/casey.allard/.codex/worktrees/mvm-journal-retirement/source`

MVM physical main/operator checkout:
`/Users/casey.allard/spiralcore/local/mvm-028-qualification-20261112/source-submit-guest/source`

This main is clean, on main, at the revision above; contribution worktree is clean.
All MVM Git commands must be launched from this physical main cwd, targeting the
worktree with `git -C`. Never run MVM Git from its worktree or builder VM.

`/Users/casey.allard/spiralcore` also contains user runtime/sessions/local material.
Do not reset, delete or treat that whole directory as disposable test output.
Do not prune preserved MVM local-export refs or another project's files.

## 5. Live issue graph and publication history

The complete open-issue title/URL capture is [here](open-issues-snapshot.json).
Important nodes:

| Issue | Remaining responsibility |
|---|---|
| #42 | Full MVM networking delivery and cross-platform WAN parent |
| #44 | Canonical admitted FlowMux mapping |
| #45 | Separately admitted development and sealed-workload access |
| #46 | Linux/macOS/Windows endpoint support and backend parity |
| #47 | Cross-platform real-WAN and final upstream contribution |
| #49 | Shared budgets, sustained capacity and fairness |
| #75 | Terminal packet-loss completion |
| #83 | FIPS 140-3/security submission evidence |
| #108 | UORC v0.3.0 predictor/kernel comparison; not a delivery dependency |
| #111 | Original mailbox helper lifetime, shutdown and collection |
| #114 | Original host-agent child drain |
| #195 | Retry-ID retirement and durable sequence custody |
| #202 | Independently authorized durable return route |
| #258 | Descriptor-rooted Unix storage |
| #261 | Publicly reproducible installed FlowMux fixture |
| #268 | Canonical host/signed-runtime compatibility |

#42 and #111 were previously reopened after closure with unfinished criteria.
Do not close them merely because one helper test passes.

Recent merged evidence PRs:

- #275: actual unused-parent cancellation, main `cbeacd12391513ded587d072dec098611cc87b4a`.
- #276: qualified helper-configuration custody, main `6e6bc1d5ad7af1ae618b02ae8ada2e3d33431663`.
- #277: paired Windows direction-control results, main `db1aa008ebae150b97f766f841dd4f49eebd90b3`.
- #279: reply-progress Linux evidence and open native failures, main `4b1804308ea28698f9a03c7e8b0cfdcbe259ae15`.

Public upstream MVM PRs by this account found during export: #5340, #5336,
#5328 and older #1831 are CLOSED. See their exact titles/branches in the snapshot.
Do not describe them as the final completed deliverable or assume CLOSED means
merged. Inspect Ari's actual comments/current issue links when resuming upstream
contract work. No new upstream message was sent during this export.

## 6. External gates — do not ask the same questions again

**Signed guest runtime:** user explicitly answered that there is no contributor
process for publishing a newly signed compatible MVM runtime; **Ari must publish
it**. Do not request keys/tokens, bypass verification, patch an old signed binary,
or advertise an unsigned contributor build as a release-qualified artifact.

The signed rc.3 runtime's 2,048-byte boot binding predates the mailbox extensions;
a relevant serialized binding was 2,553 bytes. Current contributor host preflight
refuses the incompatible profile before dispatch. Unsigned compatible guest
builds are available for inspection, but are not accepted release artifacts.

Handoff already published at:
`docs/network/evidence/2026-11-14-runtime-compatibility/publication-handoff.md`.
Prior unsigned guest archive: both architectures, 76 members, 8 KiB boot bound,
22,536,289 bytes, SHA-256
`d596b947bf9e91f50ea73a1e366b8b0226b157fed7580cbeaec41df3ceb33759`.
Check the actual manifest before using it; this handoff does not requalify it.

**Cross-host MVMD:** repository is private; user has no link/access. Ari's public
MVM transport discussion points to MVMD #254 / Iroh/Hydro. Do not invent an
alternate canonical cross-host authority or claim a generic TCP demo satisfies
that integration. Read the public MVM local contracts and retain this external
acceptance gate. Local MVM execution must not depend on mvmd.

**FIPS/security:** starting deployment is unclassified/CUI with a path to FIPS
140-3 validated cryptography. No current FIPS/CUI/classified deployment claim is
established. Inventory every transport, mailbox, access, signing, identity and
storage crypto operation and map exact module/version/environment/approved mode.
Selecting SHA-256 or changing only TLS provider is not validation. Current test
ring/Ed25519/TLS setup is functional evidence, not a FIPS qualification.

## 7. Current implementation: cooperative reply progress, PR278

The original worker decoded both incoming requests and the local sender's
replies. It synchronously handled inbound authorization/storage/durability/
readback/delivery. Thus unrelated local replies queued on the connection could
not be decoded until that inbound handler returned.

A deterministic Linux trusted-connected TCP control pauses one endpoint's
inbound Begin authorization at an armed bounded gate. Its own independent 1 KiB
send starts while the gate is held. Peer TX counters prove a complete extra reply
record exists before gate release. The baseline times out at the original 500 ms
local deadline. The gate is always released and both original worker handles are
physically collected before assertions. Collection alone is not protocol success.

Baseline public source `7cc132d9e2b7381cb5ba5a1e92be1942148e7308`: 0 passed /
1 failed / 32 filtered, 0.52 s, Cargo exit 101. Peer counters go from 222 to 408
wire bytes, 102 to 168 payload bytes, 1 to 2 records. Baseline executable is
6,123,936 bytes, SHA-256
`8e23e367a41978e4f8b15c91abf512375a1ccbae89612e121174e2d21ce6c6e6`.
This is not an authenticated MVM/WAN witness or a diagnosis of the historical
Windows timeout.

Repair files on branch `codex/duplex-reply-progress`:

- `crates/spiralcore-mvm-flowmux/src/duplex.rs`
- `crates/spiralcore-mvm-flowmux/src/duplex/routing.rs` — new shared decoder/event/error custody
- `crates/spiralcore-mvm-flowmux/src/duplex/reply_progress_tests.rs`
- `crates/spiralcore-mvm-flowmux/src/duplex/tests.rs`
- `crates/spiralcore-mvm-flowmux/src/tcp.rs`
- `crates/spiralcore-mvm-flowmux/src/tcp/prefix_tests.rs`
- `crates/spiralcore-transfer/src/resource_profile.rs`
- `.github/workflows/spiralcore-flowmux.yml` — executes library controls on all native runners

The caller may borrow the one original Rx under one lock only while the worker
handles inbound work. It validates the outer header, retains at most nine inner
bytes, and uses frame kind only as a scheduling hint. Full unchanged frame
validation and pending purpose/transfer/sequence/deadline checks determine
acceptance. Requests and APP_FIN/FIN/control records are deferred in sequence to
the original worker; the caller never runs endpoint authority/cache/delivery.
There is no extra worker, stack, wire kind or full inbound-frame handoff queue.

The prefix retains the original deadline and frame charge. Resume does not
recount bytes, reset deadlines or charge twice. Buffered prefixes precede socket
peeking. The original 32-frame operation limit remains enforced. Cancellation,
reply publication and handler completion notify an arm-before-predicate event.
Scope callbacks must remain outside the notification mutex.

Worker Drop takes/drops Rx before completion even if a caller Arc survives.
The receiver metadata charge increases 320→328 KiB, explicitly reserving 8 KiB
inside the original 8 MiB SmallV1 operation ceiling. At peak, five full frame
allocations can coexist: inbound request, prepared reply, encoded reply,
assistant raw input and decoded frame. Existing receiver allowance is six frames;
sender and TLS/carrier budgets remain separate. This is allocation accounting,
not a hard RSS limit.

Commit trail:

- `5c0b4ade`: bounded shared decoder repair.
- `f0e28718427f14806bbf7b3ad4485d4293474065`: cancellation enrollment/native controls.
- `a3da1ce0`: first error-custody successor.
- `e03c878e932ea59139e915e56114543d41a5a37c`: published code checkpoint plus failure documentation.

## 8. CRITICAL unresolved bug — exact next code task

The assisting caller can detect Error::Limit first and cancel the transport.
The worker then sees a consequential cancellation/close/reset. Its original join
must preserve the original refusal rather than replace it with that consequence.
Do not weaken the existing 32-frame join assertion.

The current successor has `Routing.fatal: Mutex<Option<Error>>`. Assistance
publishes the original owned error while still holding Rx. The caller receives
an exhaustive typed copy; I/O preserves raw OS error if available, otherwise kind
with fixed context. `Routing::finish` synchronizes the same Rx lock, then takes
the slot once. It replaces nominal success or Err(Cancelled/Closed), while
preserving an independent worker error. Reader→fatal is the lock order.

This fixes the Linux observations but is **incomplete**:
Windows ARM job **132525657787**, run **43620714331**, source **e03c878e**, fails
`thirty_two_interleaved_requests_bound_a_waiting_local_operation` with:

```text
original terminal refusal: Err(Io(Os { code: 10054, kind: ConnectionReset,
message: "An existing connection was forcibly closed by the remote host." }))
```

44 library tests passed, 1 failed. The held-inbound 1 KiB test itself passed with
no recorded first failure. The reset is not matched by current finish()'s
Cancelled/Closed replacement. Do not indiscriminately replace every worker I/O
error with the slot: an independent authorization/integrity/storage/transport
failure must not be hidden.

A final design question was queued but **not reviewed/implemented**: narrowly
recognize consequential read/write I/O using original error provenance or the
existing first-failure observation, without hiding independent failures. Existing
`FailureObservation` has bounded site/reason/kind; `FailureReason` is Copy.
Same-kind independent I/O can make category-only precedence ambiguous. Consider
explicit receive-path provenance: a worker reading Rx has busy=false, so an
assisting error can only have preceded that later read after original decoder
ownership synchronization. Request-side write errors need their own precedence
reasoning. This is a proposed investigation, not an approved solution.

The focused design reviewer became unavailable due to a usage-limit error during
this last question. Do not claim that proposal was approved. Prior focused
reviews approved the wake correction, five-frame accounting and synchronized
owned error slot, but not blanket I/O substitution.

Next regression controls: consequential socket reset after original Limit;
original decoder collection before credit release; independent worker error
wins where appropriate; cancellation visible before fatal publication; unchanged
32-frame assertion; exact readbacks. Keep the implementation small.

## 9. Latest test evidence and limits

Published full report:
`docs/network/evidence/2026-11-14-reply-progress/README.md`

Source e03c Linux qualification independently compared all 181 archived files
(2,764,328 bytes) to the test overlay. Archive SHA-256:
`14e90ef4c02be432f19f4dd80fa0457b0d465ea243504341fe1caa87cd98f946`.
Executable: 6,223,728 bytes, SHA-256
`849b99f0a6213b61007f9f52c4d114a6856a77499db45eef14dd074b36a4e73d`.

- Full component suite: 45 unit tests plus all integration targets, zero failures
  and no ignored tests. Zero-test doctest target is explicitly not extra coverage.
- Strict release Clippy: all targets and all features, `-D warnings`, passed.
- SmallV1 resource target: 8 passed.
- Held-inbound-work repeat: 100/100 passed, with exact data/FIN assertions and
  original worker collection. Aggregate shell/test-process wall 3.65 s;
  user CPU 0.06 s + system CPU 0.16 s; maximum reported RSS 8,256 KiB.
- Sustained local authenticated component: 4,096 blocks ×16 KiB =64 MiB per
  direction; exact readbacks; wall 43.387066 s; process CPU to snapshot 4.668619 s;
  setup 21.931 ms wall /1.329 ms process CPU; snapshot RSS 6,752 KiB.
- TLS left sent/received 5,395,050 /5,395,015; right sent/received reciprocal.
  Both directions transmitted once =10,790,065 bytes. Do not double-count RX+TX.
  This excludes TCP/IP and lower-layer retransmission accounting.

These are functional local tests. There is no matched raw/zstd CPU/WAN speed
claim for this change. Earlier checkpoint local elapsed values 48.06 s and
79.25 s also occurred; uncontrolled run-to-run variation is not a proved
optimization. No VM booted in the latest reply-progress slice.

Retained negatives: baseline deterministic timeout; one early uncommitted
candidate peer-cancellation failure with missing exact source identity; native
f0e 32-frame join mismatch; e03c Windows ARM reset mismatch; e03c Windows2025
instrumented and default sustained failures. Later passes do not erase these.

Current seven-platform run:
https://github.com/R4-Laboratories/SpiralCore/actions/runs/43620714331

At export snapshot: Ubuntu x64/ARM, macOS ARM and Windows2022 pass; Windows ARM
fails the refusal test; Windows2025 fails sustained transfer; macOS Intel has completed; consult the captured final job state below. Read [captured job states](native-ci-snapshot.json), then refresh live.

**Fresh Windows2025 default failure:** job132525657756, same run/source. Snapshot
22.056598 s wall, 7.968750 s CPU, progress `[425,426,426,425]`. First role0 failure
is ReceiverDelivery/Timeout on frame3; later role1 failure is reply UnexpectedEof.
Test total 23.53 s. No complete 64 MiB fidelity result for this failed row.

Instrumented run43620714278 is completed FAILURE. macOS Intel stream/artifact
jobs and Windows2022 stall job pass; Windows2025 timing job132525545195 fails.
Snapshot 13.210492 s wall /4.984375 s CPU, progress `[290,291,291,290]`, first
CallerRead/Timeout. Maximum receiver Commit span 2.9516094 s exceeds unchanged
2 s operation bound. That span is elapsed time, not isolated CPU or proof of a
specific storage cause. Do not relax the 2 s/240 s gates or call this resolved.

PR278 is now CONFLICTING because the fuller evidence was separately merged into
main through PR279. After preserving results, integrate current main and keep
its richer evidence when resolving documentation overlap. Then qualify the new
exact source. Avoid repeated CI pushes while code is unchanged.

## 10. Earlier Windows paired control — distinct from current failures

Main report: `docs/network/evidence/2026-11-14-duplex-control/`.
Source `13ff238d2ff013bd33012a8140003f058c48f20b`, based on held PR260's515.
Same executable per runner compared simultaneous and alternating directions,
4,096 distinct16KiB blocks each way, original 2 s operation/240 s workload/
270 s enrollment limits, 64 publications and 8 MiB cache bound. Alternation used
two capacity-one ordinal tokens, nonblocking publication, no extra worker.

Run43612000640 completed FAILURE:

| Runner | Simultaneous wall / CPU | Alternating wall / CPU |
|---|---:|---:|
| Windows2022 | PASS204.151819 /99.281250 s | PASS225.826709 /94.203125 s |
| Windows2025 | PASS225.098870 /122.968750 s | PASS206.119197 /109.328125 s |
| Windows ARM | PASS209.055928 /116.484375 s | FAIL241.988984 /92.406250 s snapshot |

ARM failed near `[3990,3989,3989,3990]` at original workload gate. No full fidelity
claim for that row. Original ZIP/member digests were independently verified.
The three compact Git-tracked archives total 11,291 bytes. This does not reproduce
or explain the older simultaneous sender-reply timeout. PR260 remains held.

An effective-token Windows shortcut was considered and **rejected as a drop-in**:
pseudo-token queries can change query-denied impersonation refusal semantics.
Do not implement it without the missing refusal/security controls. Existing
UTF16 reuse and inline ACL/SID buffer optimizations already exist; don't repeat
that work or weaken per-file ownership/authorization checks.

## 11. MVM helper-custody contribution — already qualified, don't redo

Fork branch639cdc36e contains a post-spawn configuration-unwind fix: original
Child/marker/restart state remains outside the caught callback until the original
owner stores it and failed-readiness rollback collects it. Exceptional panic
payload is deliberately forgotten to avoid a destructor double panic. Existing
panic hook still runs. This is not generic panic/abort/no-exec protection.

Production commit `ae2717f0f5c95fa5c6c5cf1f3c4ae98852f392ca`; final fixture correction
639 uses canonical `mvm_core::env_hygiene::helper_command("/bin/sh")` after the
spawn inventory caught a raw Command::new.

Files: `crates/mvm-hostd/src/mailbox_process/original_journal/stop.rs`,
`stop/configuration_tests.rs`, installed collector lifetime contract and lifecycle
review ledger. Four new controls, spawn inventory, full serial workspace tests
and doctests, strict workspace/all-target Clippy and gated Linux/BDD compilation
passed. Final source manifest:4,675 files /64,851,537 bytes, SHA-256
`c5836a6fca7b952bbae0c46130c2b23b0aae06a141847211729d45f7f9f52017`.

Three actual original-helper process fixtures recover exact257-byte payloads.
Guest and VMM peers are fixtures, not booted MVMs. Lost reply57.697 ms;
reconnect81.901 ms through B ACK; A exits207.747/222.935 ms; B10.346/15.343 ms.
Ordinal1 retirement follows signed receipt. CPU/counter scope is documented in
`docs/network/evidence/2026-11-14-helper-configuration-custody/`.

Earlier parallel builder PID-descendant test failure and setup/inventory errors
remain retained. Serial success is not a parallel reliability claim. On Wick's
new4090 driver the correct host test env includes `MVM_GPU_TEST_EXPECT_NATIVE=1`;
this does not authorize GPU training for SpiralCore.

Other completed, bounded witnesses:

- Actual unused-parent cancellation: original256 MiB reservation0→1→0, workers0,
  controller wall1.235 s /CPU0.646 s, expected refusal exit1, no VM boot, debt kept.
  Report `2026-11-14-unused-parent-cancellation`.
- Actual installed rc.3 FlowMux one-guest witness already exists; don't rerun
  because you assume all rc.3 FlowMux is missing. Report
  `2026-11-13-owned-unix-store/native-rc3-fixture.md`: package63,859,200 bytes,
  independent guest ELF readbackPASS (1,328,904 bytes);5,804 exact bytes each
  direction,6,889 TLS bytes. One host, no matched zstd/WAN acceptance.
- Installed journal native11/12 ran startup/readiness and helper exit0 but old
  runtime application compatibility failed. Native13 preexec/callback retry
  passed its bounded failure-cleanup control. Retained debt/protected state is
  not a claim of general recovered shutdown.

## 12. Wick, builds and storage — exact reuse paths

SSH alias `wick` is working. Preserve established account/key/host identity.
Do not paste credentials or invent a replacement host/VM. Other work uses its
4090/DeepSeek, so respect CPU/RAM/disk bounds and isolate processes.

Current shared qualification root:
`/var/tmp/sc-direct-flowmux.iptmjZre`

Current reply-progress source overlay:
`/var/tmp/sc-direct-flowmux.iptmjZre/duplex-reply-progress`

Reuse:

```text
CARGO_HOME=/var/tmp/sc-direct-flowmux.iptmjZre/source/.mvm-test/cargo
CARGO_TARGET_DIR=/var/tmp/sc-direct-flowmux.iptmjZre/source/.mvm-test/target
CARGO_BUILD_JOBS=2
CARGO_INCREMENTAL=0
TMPDIR=/var/tmp/sc-direct-flowmux.iptmjZre/duplex-reply-progress/tmp
Rust toolchain=1.97.1
```

Bounded user systemd units used MemoryMax6G, MemorySwapMax0, CPUQuota200%.
No new VM was booted in this slice. Prior native allowance is at most two guests,
2 total vCPUs and latest512 MiB requested guest RAM; don't expand a host budget
because logical-endpoint tests used32 connections. No cloud rental/spend/Nix/GPU
training authorized by this handoff.

Successful final jobs are terminal:
`sc-duplex-reply-final2` and `sc-duplex-reply-final-repeats`.
Earlier failed units retained: `sc-duplex-reply-candidate-lib`,
`sc-duplex-reply-progress-red`, `sc-duplex-reply-progress-red2`,
`sc-duplex-reply-refusal-lib`. They are failed historical units, not running jobs.
Scripts/logs below q include `reply-progress-validate.sh`,
`reply-progress-repeats100.sh`, source archive, full/clippy/resources/repeats logs
and source-verification.json. Tests operate on source overlays; do not advertise
that directory as a clean Git release checkout.

At export: Mac about58 GiB free; Wick about224 GiB free. Recheck before work.
Do not delete the shared target wholesale, active worktrees, evidence or another
project. Git report is only76 KiB. Main already contains compact output and hashes.

Local optional scratch/recovery files, not the authoritative source:
`/tmp/sc-restart-export-20261114/`, `/tmp/sc-reply-final-evidence/`,
`/tmp/sc-reply-e03c-arm-failure.log`, `/tmp/sc-reply-timing-failure.log`.
A previous local `gh run watch` process may still be attached (exec session46221,
log `/tmp/sc-reply-native-watch.log`); inspect or stop only that owned watcher.
Do not assume /tmp persists across restart. No future agent depends on those
session IDs, review agents or the deleted old worktrees.

## 13. Execution and security agreement for restart

Read applicable AGENTS.md. For security-sensitive MVM edits read
`.agents/skills/secure-execution-review/SKILL.md` before design. The MVM agreement
requires ponytail and rust-skills; they were unavailable and that was reported.
Report availability honestly rather than silently claiming the skills ran.

MVM: all edits in worktrees; main stays synchronized main; Git launched only
from physical main operator checkout. All Nix/build/eval, ordinary mvmctl runtime
and Linux-specific operations stay in the project builder boundary, except the
explicitly approved native HVF and bounded Wick/KVM test-provider exceptions.
Never use Lima for builds/evals. No ad-hoc billable cloud host.

Host cargo is the default for portable MVM Rust work; Linux-only checks need the
approved Linux environment. Required workspace tests, strict Clippy, check and
feature/platform-gated compilation must pass before claiming full completion.
No new clippy suppression to bypass a design issue. No production unwrap. Reuse
canonical config/env/policy/backend helpers; don't create another data path.

Long builds/CI watches run in background and wait through their owned handles.
Do not use sleep-and-check loops. Owned resources use events; timers use monotonic
clocks; crash/external state uses bounded reconciliation. Retain original child/
thread/socket/resource ownership through physical termination, not merely a
notification or cancelled flag. Never delete protected state/debt just to make
cleanup counters zero.

GitHub issue criteria are completion authority. No final PR auto-close keywords
for unfinished checkboxes. Keep docs decisions separate from progress ledgers in
MVM specs. No assistant/tool attribution in MVM commits, PRs or shipped docs.
No secrets in source/logs. No remote card-source execution.

## 14. Recommended execution order from this checkpoint

1. Read this handoff, current SC/MVM AGENTS, live #42/#44/#111/#268 and PR278/260.
   Verify clean branches and current main. Refresh only relevant Ari comments and
   linked contracts; do not recursively audit specs or all repo files.
2. Switch the existing SC checkout deliberately to `codex/duplex-reply-progress`.
   Preserve this published handoff. Resolve its overlap with current main while
   retaining the fuller main evidence, then make one narrow error-provenance fix.
3. Prove original Limit survives consequential OS reset without overriding an
   independent error. Keep the existing32-frame assertion. Test deterministic
   cancellation-before-publication and original handle/decoder retirement.
4. Run focused failing target first, then required Linux component/resource/
   strict-Clippy gates on exact source. Reuse existing shared target and tiny data.
   Record final source/file/executable identities and actual byte/CPU/time scope.
5. Run native platform acceptance once there is a concrete source change. Inspect
   retained ARM/Windows2025 failures before dispatching another large matrix.
   Do not retry unchanged tests until green or extend deadlines to hide failure.
6. Resolve sustained Windows ReceiverDelivery/CallerRead timeouts with a bounded,
   source-linked experiment. Distinguish elapsed storage/authority/scheduling
   delays from CPU; prior timing spans do not isolate a cause. Keep metadata,
   ownership checks, fairness and backpressure intact.
7. Merge only the reviewed/tested slice to SC main. PR260 remains independent
   HOLD until its own updated source/platform/resource gates pass. Update issues
   and current-status with executed evidence and retained failure rows.
8. Continue installed original-owner mailbox/FlowMux/access integration. Consume
   Ari's accepted signed-runtime artifact when published and available. Do not
   bypass the external signing gate or private MVMD interface.
9. Complete native sustained sessions/recovery/expiry, large data plus control,
   same-host and canonical two-host direct/relay WAN, supported role matrix,
   capacity/fairness envelope, crypto inventory/security qualification and final
   upstream package. Only then submit the complete final MVM contribution.

If a reviewer/tool is unavailable, don't claim approval or spend a new broad audit.
Do useful independent work; keep the full goal active unless the user explicitly
pauses it or all required delivery work has actually finished.


## Final CI capture at handoff publication

Run43620714331 is completed with aggregate conclusion failure.

- matrix: success
- TLS/object stream / ubuntu24-x64: success
- TLS/object stream / windows2022-x64: success
- TLS/object stream / macos26-intel: success
- TLS/object stream / windows2025-x64: failure
- TLS/object stream / macos26-arm64: success
- TLS/object stream / windows11-arm64: failure
- TLS/object stream / ubuntu24-arm64: success

[Retained native failure excerpts](native-failure-excerpts.json) preserve the
Windows ARM terminal-reset refusal mismatch and Windows2025 sustained failure.
