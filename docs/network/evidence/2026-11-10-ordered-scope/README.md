# Ordered Small cache and authenticated scope

This component port builds on the Windows wake correction in PR #104,
`8535b10d3596c903a1225917326f46fdbe68b69f`. The source originated in installed
candidate `228331df7d8d57d31dd267a246964cb5435043c9`; this extraction excludes
the unrelated Unix publication/directory-sync change and installed-service
work. The [mapping contract](../../../api/mvm-flowmux-v1.md) specifies scope
versions, ordered lanes, retention and failure semantics.

The earlier regression published four distinct 4 KiB applications, admitted
one 2 KiB application, then refused an artifact while that transfer was active.
Before the fix, refusal evicted two verified publications: retained count fell
from three to one and evictions rose from one to three. The test now requires
unchanged publication counts, byte readbacks and replay after refusal.
Victims are planned without mutation and removed only after canonical Begin
admission. Post-admission I/O uncertainty can partially evict and fences without
a reply; rollback and recovery are not claimed.

General profile 1 preserves frozen IPv4 and unscoped IPv6 scope commitments.
Small profile 2 uses distinct v2 serialization and commitment domains, binds the
profile and complete IPv6 address fields, and rejects a capability/enrollment
mismatch before creating the object channel. Existing experimental Small peers
must update together.

## Executed paired TLS check, 2026-11-10

Wick/Linux x64, Rust 1.97.1, locked offline debug build, two build jobs and the
existing shared target. **55 FlowMux tests passed, zero failed or ignored.**
The full list includes exact bytes, authority, half-close, failed-wake and
deadline controls, plus ordered Warm/Runtime/Combined artifact retention.

The 4,096-block test used one persistent paired loopback TLS connection. Both
directions carried 16,384-byte blocks and compared every recovered byte.

| Recorded quantity | Result |
|---|---:|
| Logical bytes, each direction | 67,108,864 |
| Actual TLS bytes A sent / received | 5,395,050 / 5,395,015 |
| Peer sent / received | 5,395,015 / 5,395,050 |
| Setup elapsed | 8,177 microseconds |
| Complete lifecycle elapsed | 15,751,471 microseconds |
| Process user / system CPU | 17.75 / 1.16 seconds |
| Process high-water RSS | 9,356 KiB |
| Declared core pair charge, each endpoint | 8,284,092 bytes |
| Explicit reader stack, each endpoint | 2,097,152 bytes |
| Retained attempts, each receiver | 1 |
| Retained publications, each receiver | 64 |
| Retained private files / bytes, each receiver | 64 / 1,051,712 |
| Evictions, each receiver | 4,032 |

CPU is the Linux process counter delta at 100 ticks/second, covering both
endpoints in this test process. CPU can exceed elapsed time because threads
run concurrently. RSS differs from declared admission charges. Actual TLS
counters are cross-checked with the peer and include setup; they do not count
TCP/IP headers or retransmission. There is no send-only time measurement,
raw/zstd timing control, WAN impairment or booted MVM in this fixture.
These compressible synthetic blocks establish sustained bounded component
operation; they do not establish comparative speed or a fleet capacity envelope.

Warm, Runtime and Combined each additionally preserved exact 8,616-byte
outputs across 129 blocks in each direction while keeping their setup artifacts
pinned through eviction. At completion their cache contained 64 publications,
with respectively one, one and two pinned artifacts.

## Qualification scope and retained failures

The first copied-source build lacked the restored `v1/mod.rs` exports after
an interrupted file sync. Completing the copy and invalidating that source's
Cargo timestamp corrected the build. This was a source-assembly failure, not
a product regression. An all-target shared-core attempt then lacked a schema
fixture and, after that was copied, the workflow fixture used by three workflow
gate tests. Those failures are separate from the focused product checks;
the broad attempt is not reported as a passing full suite. It also encountered
two pre-existing ignored tests.

After restoring the immutable schema/vector/plan fixtures and workflow files,
the focused shared-core command passed **86 tests with zero failures or skips**:
47 library, 21 endpoint, seven ordered-retention, eight Small-resource and
three workflow-gate tests. Both shared-core and FlowMux strict all-target
Clippy passed, and FlowMux formatting passed. The executed product source
was unchanged after those checks.

```sh
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-transfer/Cargo.toml \
  --lib --test endpoint --test ordered_retention --test small_resources \
  --test bounded_selection_workflow
cargo +1.97.1 test --locked --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml \
  --all-targets -- --test-threads=1 --nocapture
cargo +1.97.1 clippy --locked --manifest-path crates/spiralcore-transfer/Cargo.toml \
  --all-targets -- -D warnings
cargo +1.97.1 clippy --locked --manifest-path crates/spiralcore-mvm-flowmux/Cargo.toml \
  --all-targets -- -D warnings
```

The hosted FlowMux matrix additionally executes all 15 ordered-retention and
Small-resource regression tests on each supported native process role, with
nonzero enumeration, no skips and strict failure checks. Native results and
merge identity are recorded separately after execution. Installed MVM adoption
of the v2 Small scope, canonical direct/relay WAN, recovery and scale remain
open under [#44](https://github.com/R4-Laboratories/SpiralCore/issues/44),
[#46](https://github.com/R4-Laboratories/SpiralCore/issues/46) and
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49).

## Scoped coverage receipt

The [exact candidate receipt](coverage/summary.json) pins `f26b7803` against
`8535b10d`. Selective LLVM instrumentation executed 55 FlowMux and 86 focused
shared-core tests: **141 passed, zero failed or ignored**. All 189 source-file
hashes matched the Git archive before/after collection; all 18 protected
executables stayed unchanged. The fixture-only successor `de4907e3` changes
no product source.

The aggregate changed-production gate covered **538/618 executable lines
(87.055%)**. Per-file changed coverage includes ordered retention 277/316,
FlowMux duplex 90/96, scope 25/25 and endpoint 138/173 (79.769%). The endpoint
file is below 80% individually. All mapped production coverage was 91.711%
for FlowMux and 67.533% for transfer, a combined 72.993%. This is not an 80%
whole-core claim. The [uncovered lines](coverage/uncovered-changed.tsv) remain
visible. Linux provides no mapping for the changed Windows backend, which
receives no coverage credit; LLVM emitted no branch mappings.

Raw profiles, JSON/LCOV exports and original binaries remain on Wick under
`/var/tmp/spiralcore-ordered-coverage-f26b7803-20261110`. This repository retains
only the compact summaries, exclusions and [artifact hashes](coverage/artifact.sha256).
The temporary directory fixture correction was tested separately; it did not
alter the measured production source.

## Native fixture-budget failures and correction

At `de4907e3`, [run 42891282541](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42891282541)
passed the Linux and macOS ARM64 FlowMux rows but failed macOS Intel's
4,096-block fixture at its original 90-second watchdog (92.25-second test).
Windows Server 2022, Server 2025 and Windows 11 ARM64 stopped the 129-block
fixture after respectively 69, 70 and 72 blocks. Those tests enrolled scope
for only three seconds while allowing five seconds for transfer; their test
durations were 3.52, 3.40 and 3.38 seconds. Scope expiry is consistent with
the observed `Closed`, but the original log did not retain the worker's exact
semantic error, so it does not establish that cause conclusively.

The sustained fixtures now use a single fixed enrollment deadline covering
their entire bounded run: 35-second enrollment/30-second watchdog for 129
blocks, and 270-second enrollment/240-second watchdog for 4,096 blocks.
Enrollment remains below the product's 300-second maximum. The product's
two-second per-operation deadlines, authorization checks, workload counts,
cache bounds and exact-byte assertions are unchanged. This corrects fixture
lifetime coverage; it does not qualify the original 90-second performance
bound. Failure diagnostics include the ordinal, retained state and current
scope checks. CI retains successful test output so per-role timing and TLS
counters remain available alongside failures.

A subsequent Wick run passed all 11 tests in the two affected targets. The
129-block case took 300,691 microseconds with both scopes current. The same
4,096-block workload retained the identical TLS byte counts and cache bounds,
with 11,980,047-microsecond lifecycle, 20.07/1.30 seconds process user/system
CPU and 9,296 KiB process high-water RSS. These are repeat observations under
different scheduling, not evidence of a product speed optimization. Native
success after the correction remains a separate acceptance gate.

## Windows sustained performance refusal

At `b43a1da0`, [Server 2022 job 130104398191](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42894601561/job/130104398191)
passed all 129 blocks in 5.78 seconds, but the 4,096-block workload missed the
unchanged 240-second watchdog at 2,626/2,619 blocks in the two directions.
Both scopes were current. Each receiver still held one attempt, 64 publications
and 1,051,712 cache bytes; evictions were 2,562/2,555. The test took 242.12
seconds including secondary reader teardown. This is an incomplete workload,
not a throughput acceptance pass or evidence of fidelity failure. CPU and
complete TLS traffic for that failed dose were not recorded.

The source performs quota scans at admission and publication. At steady state
that checks roughly 64+63 files per distinct object. The failed dose implies
about 658,000 quota-file opens and the complete dose projects about 1.03 million.
These are source-derived operation counts, not measured CPU attribution.
Each old Windows open repeats ancestor opens, volume admission, SID, descriptor
construction and held-file privacy checks.

The [quota scan repair](../../../specs/2026-11-10-windows-quota-batch.md) shares
directory setup within one scan while retaining filesystem observation and
per-file checks. Its first Windows-target test build failed E0425 because the
new API did not exist. The implementation then passed the pinned Windows-target
metadata and strict Clippy checks, and 83 focused Linux core regressions passed.
This does not execute Windows behavior. The two existing native suites now also
check pin/release, ACL, hardlink, sharing, type and token-change refusals and
record six alternating paired 64-file metadata comparisons, including actual
process CPU counters. Native execution and the original 4,096-block/240-second
workload remain mandatory before this successor can merge.

The first batch prototype at `a3100c50` failed native execution: all three
Windows FlowMux rows stopped after two verified blocks with current scopes;
Server 2022's parallel endpoint suite passed 17 and failed three during store
opening. [FlowMux refusal](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42902543720/job/130132370658),
[endpoint refusal](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42902543422/job/130132277593).
Its directory-list pins denied write sharing on common ancestors, including
Temp. Shared-ancestor interference is consistent with those failures, but the
original errors did not retain a specific Windows sharing code. The prototype
is unaccepted; no speedup is attributed to it.

The successor captures only the owned root and namespace with strict pins,
enumerates by handle and opens immediate children with `NtCreateFile` relative
to those handles. Shared ancestors receive no persistent new write exclusion.
It matches full enumerated/opened file IDs and retains per-file checks. A new
direct regression creates/writes/renames/removes an unrelated sibling while
the scan holds its namespace; owned root/namespace rename must still refuse
and work after release. Native backend preflight now runs before larger
endpoint tests so a refusal cannot hide its own diagnostic suite.

At successor `d6e3d669`, the two native Windows primitive suites passed on
Server 2022, Server 2025 and Windows 11 ARM64. Server 2025's portable endpoint
job also passed. These are component observations, not a full matrix result.
The Server 2022 FlowMux job failed earlier than the sustained test: the
queue-backpressure fixture timed out while filling its first four delivery
slots. Its shared 180 ms operation budget applied to successful durable writes
as well as the intended fifth-send timeout. The original authenticated
[6,445-byte artifact](native-d6e3d669/flowmux-windows2022-failed.zip) and
[API/hash receipt](native-d6e3d669/receipt.json) retain that failure; all four
inner file checksums verify. [Native job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42911665467/job/130163745487).

The fixture successor moves the same nine TCP tests into the internal duplex
test module, preserving one setup helper and the 55-test total. This allows
the queue fixture to use the existing private absolute-deadline operation:
four positive sends use the normal two-second budget, then the fifth send has
its original fresh 180 ms deadline. The two-second negative bound, failed
joins, subsequent-send refusal and cancellation of queued delivery remain.
No public API, product timeout or sustained-workload gate changes. Linux and
native successor execution are required before acceptance.

Server 2025 at the same `d6e3d669` source reached the full sustained fixture and
failed after 243.51 seconds: the unchanged 240-second watchdog stopped writers
at blocks 2,879 and 2,880. Both scopes remained current. Each receiver retained
one attempt, 64 publications and 1,051,712 cache bytes, with 2,815/2,816
evictions. The queue-fixture correction does not address this separate
performance failure. The original [7,436-byte artifact](native-d6e3d669/flowmux-windows2025-sustained-failed.zip)
and [verified receipt](native-d6e3d669/windows2025-receipt.json) preserve it.
[Native job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42911665467/job/130163745489).
Actual CPU and complete traffic were not printed on this failing path; those
measurements remain missing, rather than inferred from elapsed time.

The [three original native portable archives and custody receipts](native-d6e3d669/portable-windows-receipts.json)
verify against the API archive digests, every inner checksum and the exact
tested synthetic-merge tree. All seven portable roles passed at `d6e3d669`;
that separate matrix does not establish sustained FlowMux or WAN acceptance.
The Windows primitive comparison records these sums over six alternating
paired scans of the same 64-file namespace:

| Native role | Old scan wall ms | Batched scan wall ms | Old process CPU ms | Batched process CPU ms |
|---|---:|---:|---:|---:|
| Server 2022 x64 | 117.982 | 33.126 | 125.000 | 31.250 |
| Server 2025 x64 | 57.813 | 17.405 | 62.500 | 15.625 |
| Windows 11 ARM64 | 133.421 | 33.080 | 140.625 | 31.250 |

CPU comes from actual process kernel-plus-user counters in 100 ns units;
their coarse observed increments limit interpretation of these short totals.
The six rounds are one local metadata microbenchmark, not independent WAN
trials or proof that quota scans dominate object transfer. Server 2025's new
scan averaged 2.90 ms; the sustained fixture still missed its gate. Transport
park/poll scheduling and other per-object work require measured attribution.

The queue-fixture successor `8a109ae4` cleared that queue test on all seven
native roles. Its [seven original FlowMux archives and verified receipts](native-8a109ae4/receipts.json)
total 70,108 compressed bytes, with API digests, all inner checksums and
synthetic-merge tree equality checked. All four Linux/macOS roles completed
their full component, ordered/Small and strict-Clippy stages. All three
Windows roles still failed the unchanged sustained gate:

| Windows role | Writer ordinals at 240 s | Test including failed teardown |
|---|---|---:|
| Server 2022 x64 | 3,806 / 3,809 | 242.11 s |
| Server 2025 x64 | 3,222 / 3,224 | 242.07 s |
| Windows 11 ARM64 | 3,505 / 3,505 | 242.15 s |

Scopes remained current and each receiver kept 64 publications and 1,051,712
cache bytes. Later stages skipped after those failures have no pass claim.
Different hosted runners and incomplete doses do not establish a causal
before/after throughput improvement. CPU and full traffic on these failure
paths remain missing; the optional timing witness is being implemented to
retain those observations without changing the workload or deadline.

## Optional timing witness and its retained failures

At `f9cf39bc`, the diagnostic feature is default-off. Its fixed state records
requested/actual TLS pauses, poll outcomes, sender/receiver frame phases and
read/acknowledgment ordinals. The witness adds actual process CPU and TLS
counters on workload failure. Preparation failures retain their original
errors; the report guard starts only after both streams are prepared. Snapshot
times are concurrent and overlapping, and are not send-only or per-phase CPU.

The [first native diagnostic artifact and verified receipt](native-f9cf39bc/receipts.json)
retain a Windows Server 2025 operation-timeout failure before the overall
watchdog: 1,720 exact blocks read and acknowledged per direction,
137.719083 s to snapshot, 52.968750 s actual process CPU including both local
peers, setup 0.101080 s wall/0.015625 s CPU. TLS counters at that snapshot were
left sent/received 2,266,614/2,266,571 bytes and right sent/received
2,266,777/2,266,614 bytes. In-flight/failed work means the counters need not
balance. This incomplete dose is no successful-transfer result.

Commit handler elapsed totals were 68.313372/65.179094 seconds across
1,721/1,720 calls (means 39.694/37.895 ms). Those spans cover shared endpoint
commit and ordered cache bookkeeping, including durable publication and
readback. They exclude the later application delivery read. TLS session
pauses requested 1.790/1.677 seconds in aggregate but took
15.762247/13.293386 seconds, with maxima around 16.6 ms. These overlapping
measurements identify both costs without attributing every Commit millisecond
to a particular syscall or adding wait totals to wall time.

The initial native coverage collector also refused its profile set after two
primitive tests passed in the attempted instrumented run. Instrumentation was
not established without profiles or the actual compiler invocation. Its original artifact is retained in the
same packet. The log did not record profile discovery, so missing versus
oversized files and their locations remain unresolved. The collector successor
uses one compile/run pattern, records bounded profile discovery and separates
source-pinned test hooks from the production denominator. That successor still
requires native execution; no Windows production coverage is credited yet.

The `5fc29bd4` collector's discovery then established zero profiles after both
enumeration and successful tests on all three Windows roles. Source review
identified the environment-removal defect: the current PowerShell .NET setter
with null retains an empty `CARGO_ENCODED_RUSTFLAGS`, and Cargo gives that
variable precedence over `RUSTFLAGS`. The successor explicitly removes and
checks absence, captures the actual verbose test compiler invocation and
requires the coverage flag, and restores absent versus empty environment
states distinctly. Actual native proof of this correction remains pending;
there is no coverage-threshold exemption. [PowerShell contract](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_environment_variables?view=powershell-7.5#use-the-systemenvironment-methods),
[Cargo precedence](https://doc.rust-lang.org/cargo/reference/config.html#buildrustflags).

Independent review found that the original returned timing observer exposed
producer mutators publicly. The successor makes those methods crate-private;
an external compile-fail example first compiled unexpectedly (RED), then
refused after the visibility fix (GREEN). A focused diagnostic-label regression
also first observed `Resume` for wire kind 4, then passed with the canonical
`Probe` label. The two timing unit checks, external privacy doctest and strict
feature-on Clippy passed on Wick after these fixes. No protocol frame or
transport-authority change follows from diagnostic visibility or labels.

## Exact-source diagnostic coverage

The [fresh Linux receipt](coverage-f9/summary.json) pins `f9cf39bc` against
`dc7098fc` and executes **143 checks: 57 feature-on FlowMux and 86 transfer**,
zero failed or ignored. Changed mapped production is **794/890 = 89.213%**;
new timing production is **92/92 = 100%**. Changed duplex is 172/183,
TLS 61/70, endpoint 159/196 and ordered retention 277/316. All mapped FlowMux
is 2,432/2,646; transfer is 5,616/8,315; union is 8,048/10,961 (73.424%).
Those broader limits remain, and branch coverage is NA without mappings.
Windows code is unmapped and receives no Linux credit.

All 197 source pins and 17 protected original/copied executable pins stayed
stable. Eight files in the earlier shared narrow snapshot differed from the
frozen source. The collector left it intact and assembled an isolated exact
copy; the [assembly delta](coverage-f9/assembly-delta.json) and original
collector-only refusal remain. Earlier runs that matched only the changed
FlowMux files are not promoted to whole-revision performance evidence. Raw
profiles, complete exports and helper scripts stay on Wick under
`/var/tmp/spiralcore-f9-timing-coverage-20261110`; the repository retains compact
summaries, pins, exclusions and uncovered lines. New observer visibility and
label edits are later than this frozen receipt and need final-source accounting.

## Release-driven ownership waits and remaining native gates

Source `661c74e7b223736e440e1e41c3cd70603d050d80` replaces Session/TX
ownership retry sleeps with original-owner release/cancellation notifications.
The [native receipts](native-661c74e7/receipts.json) retain all seven original
FlowMux artifacts from [run 42922116102](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42922116102),
three Windows portable/coverage artifacts and the Windows timing artifact.
Their 207,114 archive bytes match GitHub API digests; all 235 inner checksum
entries verify. Actual tested synthetic-merge trees equal the intended source
tree. Both successful and failed executions are retained.

Six FlowMux roles pass, including all three native Windows roles. Each completed
sustained witness transfers 4,096 exact 16 KiB blocks per direction (67,108,864
bytes each way), with 5,395,050/5,395,015 actual TLS bytes sent/received by one
peer and matching opposite counters. These are compressible synthetic component
inputs on one runner, not WAN, MVM, IP/TCP/retransmission accounting or a matched
zstd performance comparison. The uninstrumented observations are:

| Native role | Lifecycle wall seconds | Both-peer process CPU seconds | Sustained gate |
| --- | ---: | ---: | --- |
| Ubuntu 24 x64 | 24.897403 | 10.487072 | Pass |
| Ubuntu 24 ARM64 | 10.557735 | 12.416549 | Pass |
| macOS 26 ARM64 | 30.259422 | 22.831194 | Pass |
| Windows 2022 x64 | 195.181567 | 150.828125 | Pass |
| Windows 2025 x64 | 189.383965 | 170.328125 | Pass |
| Windows 11 ARM64 | 228.861835 | 178.765625 | Pass |
| macOS 26 Intel | Failed; snapshot 59.840499 | 35.786480 to snapshot | Operation timeout |

Successful receivers retain 64 publications, 1,051,712 cache bytes and 4,032
evictions each. Windows/macOS RSS is unavailable in this witness; Linux process
high-water observations are 6,544 KiB x64 and 5,328 KiB ARM64. These observations
do not establish a capacity envelope or a causal speedup across different runs.

The separate timing-enabled Windows 2025 run completes in 224.491075 lifecycle
seconds and 170.984375 process CPU seconds, with the same exact bytes and bounded
cache. Intel macOS fails its two-second operation at block 1,790, before the
240-second overall watchdog, while scope checks still succeed. Its phase timing
was disabled. The [next diagnostic](../../../plans/2026-11-10-macos-intel-flow-timing.md)
keeps the workload/deadlines and adds native phase observations; the cause is
unresolved and the seven-role release gate remains open.

The [portable run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42922116307)
passes functional portable tests on all seven roles. Native x64 Windows quota
coverage passes **216/228 production physical lines = 94.7368%** on both 2022
and 2025 after only pinned test hooks are excluded. ARM64 executes the two
instrumented tests and emits a profile, but profile merging refuses; no ARM64
coverage percentage is accepted. The original collector did not preserve merge
stderr in the artifact. The successor retains bounded stdout/stderr, tool exit
and full-file/profile hash receipts before the unchanged refusal. It neither
waives that gate nor attributes the failure to a toolchain bug without evidence.

The [fresh Linux coverage](coverage-661/summary.json) executes **154 checks**
(68 feature-on FlowMux, 86 transfer), zero failures/ignored. Changed mapped
production is **997/1,103 = 90.390%**, new lock-wait code **103/108 = 95.370%**
and timing **101/101**. All FlowMux is 2,622/2,847, transfer 5,616/8,315 and
their union 8,238/11,162 (73.804%); branch mappings are unavailable and Windows
code receives no Linux credit. All 198 source pins and 17 original/copied
executable pins stayed stable. Raw profiles/exports and helper scripts remain
on Wick at `/var/tmp/spiralcore-661-lock-coverage-20261110`; only compact evidence
is retained here, with its own inventory. No payload store, VM disk or local
Cargo target was added. The later workflow/documentation changes need their
own native execution; these results are bound to `661c74e7`.

## Subsequent Intel macOS timing and ARM64 collector diagnosis

At `1ae9fa35`, all seven native FlowMux roles pass
[run 42925924918](https://github.com/R4-Laboratories/SpiralCore/actions/runs/42925924918).
The [retained archives and custody](native-1ae9fa35/receipts.json) include all
seven full component artifacts, both timing artifacts and the ARM64 coverage
failure: 138,486 archive bytes, matching API digests and 126 inner checksum
entries. The tested PR-merge tree equals the intended source tree. No Rust
source changed from `661c74e7`.

The Intel macOS feature-off witness completes in 61.437129 lifecycle seconds
and 37.515710 process CPU seconds. Its separate instrumented observation takes
70.160776/39.570219 seconds. Both recover every 4,096 block per direction with
unchanged traffic and retention; these are not paired causal timing estimates.
The [phase analysis](native-1ae9fa35/spiralcore-macintel-timing-analysis-20261110.md)
finds no observed phase near the two-second operation deadline. Session/TX
ownership maxima remain below 1.5 ms; Commit handler maxima are 88.629/76.022 ms.
The earlier failure has no phase trace and remains unexplained. Neither these
passes nor the non-FIFO mutex/condition variable establishes absence of rare
tails or multi-tenant fairness. No product patch or deadline relaxation follows
from this successful diagnostic.

The new ARM64 artifact retains the actual LLVM error: **malformed instrumentation
profile data: symbol name is empty**, with merge exit 1 for one fresh 42,168-byte
profile. This is a tooling-stage failure before source-line export, not a failed
functional role. The [coverage plan](../../../plans/2026-11-10-windows-quota-coverage.md)
records a coverage-only bundled-LLD candidate motivated by a pinned external
project's reported ARM64 linker alignment problem. The precise section defect
has not been measured in our failed executable. Default functional builds keep
their original linker. Acceptance still requires native tests, fresh profile
merge/export and the unchanged production threshold; no profile rewriting,
skipped role or coverage waiver is introduced.

## ARM64 coverage passes; sustained Windows cost remains open

The [bf7107e6 receipts](native-bf7107e6/receipts.json) retain 12 original
archives totaling 217,268 bytes, matching API digests, 252 inner checksums and
the intended full source tree. The seven-role portable run passes. All three
Windows roles now collect accepted quota coverage at **216/228 = 94.7368%**,
with two actual instrumented tests each. Only ARM64 coverage uses the pinned
bundled LLD linker; its ordinary functional build keeps the native default.
The failed earlier profile and external-mechanism attribution limits remain.

The same-head sustained matrix passes five roles, but Windows 2025 x64 and
Windows 11 ARM64 fail. The instrumented Windows 2025 run also reaches the
240-second overall gate at 3,065 acknowledged/read blocks in each direction.
Its 242.002462-second failure snapshot records 103.046875 process CPU seconds;
scope and cache bounds remain valid. This is not a successful complete transfer.

The [phase analysis](native-bf7107e6/windows-timing-analysis.md) records Commit
means of 38.966/41.512 ms while Session/TX ownership maxima stay below 1 ms.
Commit includes storage work; the counters do not isolate every substep.
The [next bounded repair](../../../plans/2026-11-10-windows-metadata-open.md)
removes one Windows checked preflight handle that is discarded immediately
before a fully checked open on the actual consumed handle. It preserves quota,
authorization, byte verification, durability and all deadlines. Its 85 portable
checks and strict transfer Clippy pass. Native old/new paired costs and the
unchanged sustained acceptance remain pending. No speed improvement is claimed
for the unexecuted Windows candidate.
