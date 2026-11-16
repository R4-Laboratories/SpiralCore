# Two-endpoint worker budget candidate

The corrected, reviewed development helper configuration fits the existing
24-worker ceiling. Each owned helper uses one Tokio scheduler worker and keeps
both blocking slots. Synchronous signer dispatch explicitly hands its scheduler
core off within that pool, preserving other connections' original timer/I/O
progress. Both selected Object-TCP endpoints retain seven workers.

This qualifies helper scheduling, native service/retirement and conditional
ledger composition. The installed direct-TCP positive still refuses before
Ready because **FlowMux requires a bound audit sink**. Two Ready endpoints,
booted VMs, joined original-Root allocation peaks and complete MVM delivery
remain open.

## Source and resource composition

Corrected development MVM
[3a4f6870e44e11aa66bcccc7e6f2f2e69db947a0](https://github.com/Casey-allard/mvm/commit/3a4f6870e44e11aa66bcccc7e6f2f2e69db947a0),
tree 1d2b26b5a9f2ceb50a289f67440c41dc098e3155, extends initial worker candidate
c20c090 and audit-port checkpoint b8f9883 on canonical c5de1cc7. Selected
consumer dependencies remain **MVM f2ddbcbf / shared core 8994951b**.

| Reserved component | Earlier configuration | Corrected candidate |
| --- | ---: | ---: |
| Daemon: foreground + scheduler + blocking maximum | 1 + 2 + 2 = 5 | 1 + 1 + 2 = 4 |
| Signer: foreground + scheduler + blocking maximum | 1 + 2 + 2 = 5 | 1 + 1 + 2 = 4 |
| Two original VM observer charges | 2 | 2 |
| First endpoint | 7 | 7 |
| Second endpoint | 7 | 7 |
| Aggregate requirement | **26; refuses Full** | **24** |

Each endpoint retains its foreground controller, session, three collector
workers, TCP relay and codec reader. The ledger ceiling, guest slots, original
child custody, two blocking slots, stack sizes, I/O cap, workspaces, rate
limits, signing, confinement and deadlines retain their existing bounds.
Pinned Tokio 1.53.3 limits the scheduler/blocking/replacement pool to
worker_threads + max_blocking_threads: three runtime threads plus the original
foreground owner per helper.

## Dispatch-stall correction and regression

The initial c20 candidate completed fast signing/retirement and passed its
worker arithmetic, but final review found a real responsiveness gap:
synchronous signer dispatch, including file writes and sync_all, still ran on
the sole scheduler worker. A stalled dispatch could delay another connection's
one-second parse timer. Those initial observations did not qualify this case.

The correction acquires the same asynchronous helper mutex, runs the same
dispatch inside block_in_place, then drops the guard before the existing
response write/shutdown. The original key/file owner, registered stream and
workspace remain retained. No detached signing job, new queue, authority,
deadline or owner is introduced. The owned serve loop refuses a current-thread
runtime before accepting requests; the existing owned binary uses its original
multithread builder.

A private cfg(test) hook holds dispatch after its actual mutex acquisition.
The regression uses a real RegisteredListener, two registered connections and
the original workspace pool. It polls the partial handler to Pending first,
arming the original one-second parse future. A native observer then requires
that exact error while dispatch is still active and unreleased, using state
captured at partial-handler completion. A queued earlier timeout cannot satisfy
the test. The 1.5-second observer allowance does not change the product timeout.

Both original handlers are joined after release, and the complete request must
still produce its correlated canonical Pong. The controlled RED2 removes only
the blocking handoff while keeping the final test assertions: one test passes,
and the held-dispatch deadline test fails. Restoring the corrected source gives
two passes and reproduces the gates2 hostd ELF hash. Source sets match exactly.
The earlier weaker RED1/GREEN1 records remain historical evidence.

The handoff preserves progress for other tasks. It does not make a blocked
fsync cancellable or remove the obligation to retain/join its original
operation. Existing outer refusal and uncertain-retirement rules remain.

## Executed native evidence

Native Linux x86_64, Rust 1.97.1, two build jobs, incremental compilation disabled,
one shared Cargo target/cache on HDD. Execution units use 6 GiB RAM, zero swap,
200% CPU and 512 tasks. Short private tmpfs paths avoid the earlier long-socket
fixture problem. No VM, Nix, GPU or cloud execution was performed locally.

The pressure control holds two original blocking callbacks while a separate
timer task must progress, then releases/joins both and checks runtime thread
termination. Its runtime peak is three, plus the foreground owner.

The native helper test registers vm-1 and vm-2 with the original daemon,
performs concurrent canonical audit calls alongside an offered partial signer
connection, and verifies both held original files against authenticated opening
and terminal inventories. It checks signatures, device/inode, exact length,
32 entries per VM, exact event/sequence content, both distinct workload
identities, unchanged process starts and pre/post executable hashes. Both
client outcomes and every original retirement outcome are collected.

| First corrected worker-only run | Observation |
| --- | ---: |
| Native sampled daemon / signer peaks | **4 / 4** |
| Broker request bytes per VM, including frame prefixes | 5,654 |
| Broker response bytes per VM, including frame prefixes | 5,024 |
| Partial signer connection sent / received bytes | 5 / 0 |
| Verified original stored bytes per VM | 19,224 |
| Setup through verified retirement | 2.480301144 s |
| Paced signing interval including offered partial connection | 2.384508410 s |
| Complete two-test command wall / user CPU / system CPU | 2.49 / 0.11 / 0.04 s |
| Command maximum RSS | 43,672 KiB |

Five worker-only runs pass both controls, each with sampled peaks four/four and
pressure peak three. Their setup-through-retirement range is
2.477753947–2.484661524 s. The complete registered target separately reports
**10 passes and one original bound-audit-sink failure**.

Thread peaks are sampled every 1 ms and may miss shorter transients. The
eight-thread reservation is source-derived and separately exercises the
handoff mechanism. The offered partial native connection has no server-side
occupancy barrier; deterministic stall/deadline overlap belongs to the separate
actual-handler regression.

Broker counters cover successfully written/read local frames; control and
helper-internal traffic are uninstrumented. Stored audit lengths are not
network bytes. Neither counter establishes TLS or WAN traffic. Helper /proc
CPU samples use 100 ticks/s and exclude setup/retirement. Command CPU covers
fixture work, pacing, hashing, verification and retirement; the domains overlap
and must not be added. No energy, send-only time, cold/warm launch or capacity
claim follows from this paced fixture.

Corrected-source workspace formatting and strict hostd/VMM all-target Clippy
pass. Nonzero library selections pass **38** tests: owned daemon 4, audit client
8, owned signer client 1, retained host-agent capability/readiness 6, owned
resources 6 and signer helper 13, including both new regression controls.
The earlier failed/incomplete full workspace campaign remains failed/incomplete.

## Conditional original-ledger control

All 19 SpiralCore canonical-host configuration tests pass. The existing
ten-helper negative retains usage 12 before endpoints, 19 after the first,
and Full for the second at 26, with unchanged usage and NeverIssued state.

The added explicit eight-helper fixture reaches 10, 17 and exactly 24.
One additional worker refuses Full without changing usage. Admission closure
fences both partitions; dropping them without retirement evidence retains the
original debits and prevents supervisor completion. No helper, endpoint or VM
runs in this control; selected MVM f2 binaries remain unchanged.

Strict canonical-host library/test Clippy and changed-file Rust 1.97.1 formatting
pass. The 19-case command records 0.07 s wall / 0.03 s user / 0.04 s system CPU,
separate from build and native-helper costs. Package-wide formatting exposes
existing differences outside this changed test, retained unchanged.

## Source, artifact custody and retained failures

The clean corrected operator checkout's **4,822 tracked-file hashes** match the
explicitly non-Git HDD source export. Its manifest SHA256 is
cf6080bc3e9c5248ec6b2892970ba42bef7123d163611fbf97cb68163dd9a260.
Full content/file sets remain stable through gates2, and are restored exactly
after the controlled no-handoff RED before terminal GREEN2. No Git metadata or
executable identity was fabricated.

Before replacement, the final five used ELFs and five changed source files were
compressed on HDD and streaming replay-hashed: 61,828,072 bytes, SHA256
95942a615dfd8800df8020dab6b1133a77f0ab2eec239a5bee6dfa25f9f21051.
The unchanged VMM test ELF is reused only after matching its earlier
source-bound replay archive; changed hostd and native helper artifacts rebuild.
The controlled RED2 archive preserves its own ELF and exact test/variant source.
GREEN2's rebuilt unit ELF matches the archived final gates2 bytes.

The SDK ELF, config source, manifest and lock also have a replay-verified archive.
[measurements.json](measurements.json) retains source/executable identities,
archives, exact per-process bytes/CPU/time, all repetitions and initial c20
observations under their narrower historical scope. Independent source/security
and artifact review cover these bounded claims.

Retained failures include initial scaffold compile errors, the unpaced audit
burst that hit the unchanged canonical rate, old 26-worker refusal, missing
dispatch handoff, a cached-artifact wrapper rejection before execution, and the
original installed audit-sink refusal. Valid audit calls are paced 75 ms apart.

Initial PR head b4c3c869 passes all three Linux worker cases but fails the stale
workflow guard requiring two. The corrected guard requires the named new
eight-helper case and exactly three passes, zero failures and zero ignores.
The failed jobs remain recorded. No timeout, security setting or original test
assertion is relaxed. Final head/CI/release identity is recorded on PR #318.

## Ari alignment and remaining delivery gates

Canonical main 9a0a2f6, including #5629 audit-context propagation and launch-scoped
runtime verification sharing, is outside this frozen development source.
Current-main launch-path reconciliation needs separate qualification before
consumer adoption. Audit identity propagation does not supply the missing
bound sink/health/deadline-accounting interface.

There is still no reply to the [installed integration request](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6928066239).
Ari's [approved scheduler direction](https://github.com/tinylabscom/mvm/issues/5255#issuecomment-6929441703)
names mvm-mailbox-scheduler in the existing mvm-hostd package: unprivileged,
entirely keyless, one per explicitly identified OS-user/canonical MVM runtime
instance, shared across that instance's tenants. It is not a physical-host or
cross-user singleton. Legacy tenant agents/signers remain unchanged. The ADR
clarification and isolated implementation contract still require review before
authority-bearing code or a positive runtime factory; resource costs are
unmeasured. This existing scoped pair neither implements nor reserves that
additional role.

Ari's [retained launch-proof follow-up](https://github.com/tinylabscom/mvm/issues/5255#issuecomment-6929642211)
clears its exact isolated five-file source review; the operation remains private,
proof-only and unwired. It supplies no durable launch witness, live VM or V3
acceptance. Protocol cutover gates remain authoritative.

The canonical confined audit interface precedes the original two-child Root
allocation/recovery witness. Compatible signed runtime/boot, persistent
bidirectional Call/Cast/Reply, separately admitted owner access, supported native
platform/backend roles, actual WAN, measured capacity and private MVMD remain
separate gates. All five #42 outcomes stay active. The final upstream MVM
contribution waits for the qualified complete supported deliverable.
