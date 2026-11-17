# Four upstream changes through 106d28d

Development MVM [c52b0d26fd51d59b5b25fe937e5fdbc4b5ccc387](https://github.com/Casey-allard/mvm/commit/c52b0d26fd51d59b5b25fe937e5fdbc4b5ccc387),
tree `3e3889b77f8393344c42db23cb6e9c2bb3b0046e`, merges the previously qualified
8ec3f12 networking branch with canonical 106d28d. This slice incorporates exactly
four new canonical commits. Selected SpiralCore consumers remain **MVM f2ddbcbf /
shared core 8994951b**; the source is published for development qualification.

| Canonical change | Adopted behavior and evidence boundary |
| --- | --- |
| [#5639 / 842e24e](https://github.com/tinylabscom/mvm/pull/5639) | Lower-precedence CARGO_BUILD_RUSTFLAGS preserves target-specific musl shared-object flags. This repairs the source mechanism behind the earlier missing .so, without proving a rebuilt signed runtime. |
| [#5641 / 08a2d86](https://github.com/tinylabscom/mvm/pull/5641) | One canonical denial-stage shape across lookup, connect and datagram, with canonical readers. It adds no installed sink/health capability or missed-decision interface. |
| [#5643 / 6cd4036](https://github.com/tinylabscom/mvm/pull/5643) | Original eight-shape HVF startup assertion and explicit kernel-descriptor-only legacy limitations. |
| [#5644 / 106d28d](https://github.com/tinylabscom/mvm/pull/5644) | Regression preserving the musl cdylib target flags on both target architectures. This is a component regression, not executed guest-runtime assembly. |

The only merge conflict retained both the original deadline tests and the
canonical stage tests. Production record_before still uses the original
absolute deadline. A combined test proves that expired refusal builds no labels
or late signing work, and a valid datagram replaces a bogus stage while carrying
the exact original deadline and restoring its scope. Existing assertions,
independent worker errors, signer ownership, 24-worker ceiling, rate/DNS/security
checks and service distinctions remain.

## Executed qualification

[Machine-readable measurements](measurements.json) bind command exits, actual
bytes/costs, source and executable custody, retained receipts and limitations.

| Frozen Linux x86_64 scope | Result |
| --- | --- |
| Canonical policy gates | 87 pass |
| Formatting, workspace/all-target check, explicit BDD compilation | Pass; BDD scenarios NOT_RUN |
| Strict workspace/all-target Clippy | Pass |
| Compiler-selected executables | 293 test + 65 other; all 358 freshly compiled, zero reused |
| Focused selected scopes | 78 passed / zero failed / one ignored across 13 nonempty scopes |
| Complete binary harness set | Exact 293 expected paths, all 293 complete |
| Outer binary totals | 22,125 passed / one failed / 72 ignored |
| Rustdoc | 28 sections; 29 passed / zero failed / eight ignored |
| All 358 executable identities | Unchanged before/after focused and full runs |
| Original source and Git metadata | All 4,992 original files unchanged |
| Prepared full file set | All 4,993 files unchanged, including the explicitly observed canonical empty lock |

The unchanged failure is
registered_prepare::registered_direct_tcp_transfers_exact_bytes_and_refunds_a_failed_peer:
**FlowMux requires a bound audit sink**, before Ready. The complete command and
its wrapper both exit **101**. All source/file-set/executable guard fields are
zero. This is a failed workspace result and does not promote the consumer pin.

The genuine source comes from normal Cargo Git acquisition. All 4,828 tracked
files match operator Git blob identity, exact length, SHA-256 and executable mode.
No source snapshot was supplied with fabricated Git metadata. Policy generation
completed before workspace compilation/execution; no concurrent policy emitter
mutated the shared target. The full run used Rust 1.97.1, two build jobs, disabled
incremental/debug info, private short tmpfs paths, 6 GiB RAM, zero swap,
200% CPU, 512 tasks, NOFILE 4096 and an 8 MiB stack limit.

The full workspace command took **493.18 s wall / 178.41 s user CPU / 85.17 s
system CPU**, maximum RSS 4,301,044 KiB. Build, policies, checks and focused
command costs are recorded separately in the JSON. These are qualification
costs, not transfer latency, per-endpoint CPU or energy.

Before execution, all 358 selected executables were retained under the shared
Cargo-target lock in a **1,112,100,185-byte** HDD archive, SHA-256
`85a54cffe56006a29d6a6618823b018281ea335587c4031511bfcf169303ec68`.
Every member was replay-hashed and the original executables post-hashed while
that lock remained held. The source-tree proof manifest SHA-256 is
`2b99d5cf62210bdd170d611f03838313562303175e9d68fe5ba809304f6c4ab3`.
The prepared-before and separately enumerated post-run source/Git manifests both
hash to `5334d32296a64a2d1311ebf791de4cdb56a4b0dd1fbca5cf949c8a8cd1ae762d`.
The latter was collected after the full command and its unconditional guards;
it is not represented as a manifest written by the EXIT trap.

## Native macOS ARM64 control

[SpiralCore PR #320](https://github.com/R4-Laboratories/SpiralCore/pull/320)
merged at `6dfbbdb2780e1fde3ce9b837244faf43120e0cdc`, preserving reviewed/CI tree
`e9b5a54bcaf9a03d754b94a9888b0003d0858ef4`.
Its [native macOS ARM64 job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/43993274020/job/133738971709)
passes the exact original one-test/eight-boolean-shape assertion and strict
production-binary Clippy. The actual ARM64 Mach-O is 2,859,968 bytes, SHA-256
`f3418e4da411e5a823a330bb6fb2b206715d3ba2b882c594516118d776baf68e`.
All 4,828 source hashes and Git tree rows match the operator's c52 committed tree;
original/copied executable pre/post hashes match. Artifact 14289625377 is
1,338,414 bytes, ZIP SHA-256
`e1acc000c6b3be50c79c0d47e2ad59d3788fcb4b6f87a044ecdc60a218a1d37f`.
The ZIP CRC, each member hash and Mach-O ARM64 header are verified, with the
original ZIP and binary retained directly on HDD beyond hosted retention.

The pure shape test reports 0.00 real/user/system seconds at BSD time's printed
precision; this is not a zero-cost claim. Raw time reports 6,569,984-byte maximum
RSS. The complete hosted job took 6m6s including compilation and tooling.
The assertion allows 111 as checked startup and 000/010 as legacy startup;
the other five boolean shapes refuse. It does **not** prove original caller
consumption, FD handoff, signed-bundle HVF boot, audit/seccomp readback, a booted
MVM or WAN. Linux compilation alone would not have exercised this gated test.

## Actual helper bytes and costs

The frozen source's two native helper tests pass. These are two original
registrations in the existing daemon/signer pair, not two installed Ready
endpoints or booted guests. Original-process exact signing/readback and verified
retirement remain required by the tests.

| Helper-only observation | Recorded value |
| --- | ---: |
| Sampled daemon / signer peak threads | 4 / 4 |
| Framed broker request / response bytes per registration | 5,654 / 5,024 |
| Offered partial signer request / response bytes | 5 / 0 |
| Original exact stored bytes / signed entries per registration | 19,224 / 32 |
| Setup through verified retirement | 2.508039403 s |
| Signing window with partial frame | 2.384690955 s |
| Two-test command wall / user CPU / system CPU | 2.52 / 0.10 / 0.07 s |
| Command maximum RSS | 40,212 KiB |

The separate blocking-pressure case observes three runtime threads plus its
foreground owner. Raw before/after helper /proc CPU ticks and executable/store
hashes are in the JSON. Those ticks cover post-registration through post-signing
and exclude retirement; whole-command CPU includes orchestration and verification.
Framed IPC counts do not include unmeasured setup traffic and are not WAN bytes.
No setup-only/per-side CPU, send-only time, network throughput or energy is inferred.
No new cold/warm transfer comparison is performed.

## Review and retained qualification repairs

Focused independent source/security review closed the audit merge and original
runtime/kernel owner boundaries with no remaining actionable finding. Focused
workflow review caught missing-baseline and unguarded copied-hash handling;
both were repaired before the tested PR #320 head. Required ponytail/rust-skills
and Trail tooling were unavailable; manual source tracing and independent review
were the stated fallback, not a claim that those tools ran.

Producer fixtures executed before the full campaign and generated only the
canonical empty target/tool-cache/zigbuild-lock.lock. Preparation records that
output separately; the full run starts from this exact prepared baseline while
still checking every original build-source/Git hash. The earlier 8ec campaign's
file-set failure remains in its original packet and is not rewritten.

Review of the new guard found that a prepared lock's shape, and then a
nonregular object at the same name, could evade its first versions. The final
guard directly lstat-checks the name, opens a regular file with NOFOLLOW/NONBLOCK,
checks named/original/held inode identity, one link, current UID and empty bytes,
and rejects substitutes. Eighteen isolated guard controls pass, including
nonempty, directory, FIFO, symlink and hardlink refusals. These are guard tests,
not storage-acceptance evidence. The full command began before this correction;
its unconditional postguard uses the independently reviewed final SHA-256
`58c0d730425e670ee218a5a85b6e7271dfb1a6dfd327ea9d33bbb135b4f274b1`.
Earlier versions and one syntax-failed standalone guard invocation remain
retained. That invocation was corrected before the full run's postguard and
changed no product source or executable. The packet names the actual versions
rather than claiming the qualification scripts were unchanged throughout.

## Ari's path and remaining delivery gates

There is no new Ari response to the
[confined audit-interface request](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6932825147).
The installed child still needs MVM's canonical confined bound sink, durable
epoch-health owner and original-deadline missed-decision accounting. A fixture
sink, relocated disposable health state, broader audit-directory RW, late
append, suppressed failure or alternate readiness path is not substituted.

Ari's [latest structural update](https://github.com/tinylabscom/mvm/issues/5255#issuecomment-6934838447)
closes review of a private test-only declaration model and starts a separate
pure lifecycle model. The completed declaration checks use synthetic records;
no second-model results, production validator, service activation, canonical
identity, mailbox authority, measured budget or readiness follows. Neither model
is imported. Manager-created unattended unprivileged user services remain Ari's
integration direction, separate from this networking implementation.

Canonical main was also observed at 93ec7898c024c104c022598f002b323fa9587f1f
(#5645 CLI type reexports), outside this frozen four-change qualification.
The 106d28d [main CI run](https://github.com/tinylabscom/mvm/actions/runs/43987459245)
was cancelled; boot-latency and live frozen-volume lanes supply no new successful
runtime/boot evidence. Its aggregate Test(main) failed because lanes did not
complete. The successful merge-group CI included lint/build and explicitly
skipped runtime suites; it is not substituted for those missing gates.

All five #42 delivery outcomes remain open: installed custody/budgets/recovery;
sustained persistent bidirectional Call/Cast/Reply and retention/recovery;
shared-core FlowMux and separately admitted access; supported
role/version/architecture/backend matrix plus executed Linux/macOS/Windows
end-to-end WAN; capacity/fairness/backpressure/tails/recovery/saturation and the
complete qualified upstream contribution. Both #258 allocation/recovery criteria,
accepted source pins, Ari's compatible signed runtime and private MVMD remain
open. No final upstream contribution is submitted.

Builds, the shared Cargo cache, raw receipts and unique executable archives stay
on HDD. This slice creates no local payload store, VM disk or Cargo target;
qBittorrent incomplete and media paths remain untouched.
