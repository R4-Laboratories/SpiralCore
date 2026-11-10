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
