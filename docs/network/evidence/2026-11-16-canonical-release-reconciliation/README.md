# Networking reconciliation with the canonical release path

Development [MVM b60d9fd901e75d030572cb9c497bb8eac37921eb](https://github.com/Casey-allard/mvm/commit/b60d9fd901e75d030572cb9c497bb8eac37921eb),
tree `a5379cada5db75c213cdf4772e9c38d20f12890d`, merges frozen c52b0d26 with
canonical `edb076999e386d31058359fd2446311cc2dd57d6`. It adopts the approved
protocol rollback and current admitted CLI dispatch while retaining the original
networking, process, storage and runtime-evidence owners. Selected SpiralCore
consumers remain **MVM f2ddbcbf / shared core 8994951b**.

## Canonical composition and checks

[MVM #5652](https://github.com/tinylabscom/mvm/pull/5652) reverts #5628's
protocol-3 cutover because it landed before its required fresh security review,
live boot/latency evidence and rc.9/demo sequence. The design remains approved
for later qualification. This source follows canonical protocol-2 authentication;
it introduces no alternate negotiation, cached advertisement or compatibility
fallback. Historical c52 qualification remains under its original source scope.

The retained RPC helper differs from canonical only in its generic Read+Write
signature. Canonical authenticated write/read, request-shape/local-compatibility
checks, checked responses and terminal behavior remain. All three original FC
entrypoint/journal/test files are byte-identical to pre-cutover 3a4f687. The 13
test declarations include the ignored peer-subprocess fixture; the twelve
ordinary controls pass. Actual original process, first guest key and later
journal key pin, absolute DeadlineIo, once-only opening, partial-frame poisoning,
independent errors, bounded data, collection and retirement assertions remain.

Two tests added for protocol-3 advertised offers retire with that withdrawn
feature. Their source and compiled preimages remain in frozen c52 history and
its replayed archive. They are not disabled current tests or replaced with a
competing protocol. Current source makes no advertised-capability claim.
Canonical capability/profile/signed-grant enforcement retains its own tests.

[MVM #5618](https://github.com/tinylabscom/mvm/pull/5618) routes ordinary cold
CLI starts through one-use admitted hostd dispatch. The resolved branch moves
its actual writable-image locks and retained verified release, checks the final
post-gate config and keeps canonical deferred CLI completion/rollback. Immediate
dispatch retains its original runtime-evidence path and RunPosture gate. Grant
readback, budget charging and launch narrative are not moved between these
source-specific paths.

The ordinary/CLI tail cannot return both selected-object worker owners. Its
existing early refusal stays; the old common-tail selected-object branch was
unreachable and is removed. Dedicated original-object APIs and fresh native
LaunchObservation remain. A new signed **test-only** snapshot exercises both
entrypoints, requires the exact owner refusal, and observes no backend instance
or launch state. This is a dispatch-boundary negative, not genuine installed
admission or permission to start a selected pair through the CLI tail.

Canonical #5645/#5653 keep CLI build types and diagnostic/cache probes behind
mvm-client. #5646 adds the no-MVMD dependency gate. #5648's function/continuation
references remain Proposed and grant no runtime, wire or mailbox authority.
The shared runtime keeps the untrusted-release/unchanged-config assertions;
its duplicate client test is removed. The test uses canonical protocol
constants, currently 2, instead of two literal 2 values. Legacy signed-runtime
and workload-verity negatives remain.

## Executed qualification

[Measurements and retained receipt hashes](measurements.json) bind exact source,
commands, bytes, costs, original failures and custody. Scopes overlap and are
not summed into a fleet or release claim.

| Frozen Linux x86_64 scope | Result |
| --- | --- |
| Canonical policies | 88 pass |
| Formatting, workspace/all-target and explicit BDD compilation | Pass; BDD scenarios NOT_RUN |
| Strict workspace/all-target Clippy | Pass |
| Compiler-selected executables | 293 test + 65 other; all 358 freshly compiled, zero reused |
| Focused selection | 67 passed / zero failed / six ignored across 16 nonempty scopes |
| Full binary set | Exact 293 expected paths; all 293 complete |
| Outer binary totals | 22,132 passed / one failed / 72 ignored |
| Rustdoc | 28 sections; 29 passed / zero failed / eight ignored |
| Original source/Git metadata | All 4,998 original files unchanged |
| Prepared full set | All 4,999 files unchanged, including the explicitly produced canonical empty lock |
| Executable custody | All 358 unchanged before/after focused and full runs |

The original registered exact-transfer assertion is unchanged and still refuses
before Ready: **FlowMux requires a bound audit sink**. Its target reports ten
passes / one failure. The full command and aggregate both retain **101**; every
source/file-set/executable guard field is zero. This is a failed full workspace
result, not an all-green MVM release or consumer promotion.

The full workspace command takes **501.31 s wall / 183.38 s user CPU / 90.10 s
system CPU**, maximum RSS 4,327,296 KiB. Compilation, policies, checks and focused
costs remain separate in the JSON. These are qualification costs, not transfer
latency or energy. Native Linux uses Rust 1.97.1, two build jobs, disabled
incremental/debug info, 6 GiB RAM, zero swap, 200% CPU, 512 tasks, NOFILE 4096,
an 8 MiB stack and private short tmpfs paths. No VM, Nix, cloud or scheduler
service is activated. The literal just check::gated wrapper is NOT_RUN
because just, cargo-zigbuild and zig are unavailable. Its two documented blind
spots are checked natively by Linux workspace/all-target compilation and the
explicit required-feature BDD check. No exact wrapper invocation, all-features,
cross-platform execution or BDD scenario result is claimed.

Normal Cargo Git acquisition supplies the genuine build checkout. All 4,832
tracked files match operator Git blob IDs, lengths, SHA-256 and executable modes;
no fabricated Git identity is supplied. The source tree proof SHA-256 is
`7a07ccb09654de44c1e703948aee117328f94f1d42337744f88d824aa4fba6ef`.
Before rebuilding, all prior c52 executable hashes and its retained archive were
verified. Before test execution, all 358 new executables were retained under the
Cargo target lock in a **1,111,468,128-byte** archive, SHA-256
`4209432b0e149d8f629911983bc5b898aa389ce83f4ece1ba32020999289fe86`.
Every archive member was replay-hashed and original bytes post-hashed while
that lock stayed held.

Policy generation finishes before workspace build/test. Producer fixtures then
explicitly create the canonical empty cache lock; the full run starts from that
observed prepared set while independently checking every original build input.
The direct-name/NOFOLLOW/inode/owner/link/empty-file guard stays unchanged
through this campaign. All eighteen synthetic guard controls pass; they are
not storage acceptance. Prepared, before-run and separately enumerated post-run
manifests share SHA-256
`7b2c4b08df78a5614439070d987de83d74f32254c760b97f8edc453881f56151`.
The post-run manifest is collected after the EXIT guards, not represented as
an output of the trap itself.

## Native macOS ARM64 controls and retained failure

[SpiralCore PR #323](https://github.com/R4-Laboratories/SpiralCore/pull/323)
pins this MVM source and runs five scopes totaling **eight passed tests, zero
failed and zero ignored**, plus strict hostd/agentd all-target Clippy. The scopes
cover two retained RPC controls, ordinary/CLI selected-pair refusal, four
canonical CLI finalization/refusal controls and the original HVF test. The last
is one assertion over eight boolean startup shapes, not eight VM launches.

The [successful native job](https://github.com/R4-Laboratories/SpiralCore/actions/runs/44005627546/job/133780163716)
selects three fresh ARM64 Mach-O test executables from Cargo output, verifies
exact source paths and retains their bytes. All 4,832 source hashes/Git tree
rows match the operator's committed tree; original and copied executable
pre/post manifests agree. The original ZIP is retained directly on HDD beyond
hosted retention: **49,475,188 bytes**, artifact `14294822712`,
SHA-256 `210904c4193b3cac3491e81126bbb7f323b2ad0dba9937b59324774ecc581e0e`. Every ZIP member CRC and hash and every actual
Mach-O ARM64 header is checked.

| Actual retained executable | Bytes | SHA-256 |
| --- | ---: | --- |
| mvm-hvf-supervisor | 2,859,824 | `a412841d75eb977adeb1b1fdec89a7cad71d56b1196022c9290ed0da6ea1ae6d` |
| mvm_agentd | 50,361,120 | `481de7b95e96005ec5ef555af0bf50358fc6a2b79313990b7ee8e7020ea95ec5` |
| mvm_hostd | 171,649,328 | `4164193ee91ec938fa4660bf88c59283462417afdbe9d384708bbb868f8d6691` |

Raw BSD wall/user/system timing for every scope is in the JSON. A printed
0.00 is limited by timer output precision and does not prove zero cost. These
are native component controls. They do not establish original installed caller
custody, actual VM boot, installed audit readiness, signed-runtime, Windows,
WAN or capacity qualification.

The [first native run](https://github.com/R4-Laboratories/SpiralCore/actions/runs/44004221158/job/133775470340)
reported eight passing controls, strict Clippy and successful custody checks,
but artifact upload failed because generated test-label filenames contained
colons. The original run remains **failed**. Its 32,199-byte job log is retained,
SHA-256 `055e76272efabc1d1496b685a468c568d7fcbe4353d8c788a917ce70904f0c01`.
The artifact API reports zero artifacts. Its **three original native executable
preimages, hash manifests and per-command BSD timing files were not retained**
and are not recovered. The fix changes only evidence filenames, preserving
selectors, source, tests, checks and the 25-minute deadline. Independent review
verified that one-line correction and all five labels. The successful new-head
run retains its own binaries/evidence; it does not retroactively recover the
first run or establish byte equality with its missing originals.

## Actual helper bytes and costs

The frozen source's helper-only witness signs, verifies and retires both original
registered chains. It is two registrations in the existing helpers, not two
installed Ready endpoints or booted guests.

| Helper-only observation | Recorded value |
| --- | ---: |
| Sampled daemon / signer peak threads | 4 / 4 |
| Framed broker request / response bytes per registration | 5,654 / 5,024 |
| Partial signer request / response bytes | 5 / 0 |
| Exact original stored bytes / signed entries per registration | 19,224 / 32 |
| Setup through verified retirement | 2.494982818 s |
| Signing window with partial frame | 2.383666746 s |
| Two-test command wall / user CPU / system CPU | 2.51 / 0.11 / 0.05 s |
| Command maximum RSS | 40,408 KiB |

The separate blocking-pressure case observes three runtime threads plus its
foreground owner. Raw helper CPU ticks cover post-registration through
post-signing and exclude retirement. Whole-command CPU includes orchestration,
pacing and verification. Framed IPC excludes unmeasured setup traffic and is
not WAN traffic. No setup-only/per-side CPU, send-only time, throughput or
physical energy is inferred. No new cold/warm comparison is performed.

## Ari's integration path and remaining gates

Focused independent source/security review closes the five conflict resolutions,
original control checks, runtime owner, dispatch custody and workflow source.
Fresh execution remains the separate evidence above. Required Trail,
ponytail and rust-skills tooling remains unavailable; manual source tracing
and focused independent review were the explicit fallback, not claimed tool runs.

There is still no Ari answer to the [canonical confined sink/health and
original-deadline missed-decision interface request](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6935221842).
The original installed signer append route exists, but no health-only storage
capability or canonical missed-decision operation completes the bound sink.
A successful helper probe, ordinary launch audit emitter, fixture sink,
disposable health relocation, broader audit-directory grant or late append is
not substituted. The unchanged readiness assertion must pass before the joined
two-child installed allocation/retirement witness can proceed.

Ari's [function/continuation reference](https://github.com/tinylabscom/mvm/issues/5544#issuecomment-6935330368)
remains Proposed. Private declaration/lifecycle models stay separate and
unimported. The [referenced validation release run](https://github.com/tinylabscom/mvm/actions/runs/43986564433)
was observed queued at canonical 6cd4036. Visible published guest-runtime tags
remain rc.5/rc.4 at that refresh; signed source host-helper archives are separate.
No compatible signed-runtime/boot qualification follows. Ari must supply the
compatible release/source pairing; private MVMD remains external.

Both #258 criteria remain unchecked. All five #42 outcomes remain active:
installed original custody/aggregate budgets/shutdown/recovery; sustained
persistent bidirectional Call/Cast/Reply with retention/expiry/recovery;
shared-core FlowMux and separately admitted owner access; explicit supported
role/version/architecture/backend matrix and executed Linux/macOS/Windows
end-to-end WAN; measured capacity/fairness/backpressure/tails/recovery/saturation
and the complete qualified upstream contribution. No final upstream MVM PR is
submitted.

Shared Cargo caches, genuine sources, raw receipts and unique archives stay on
HDD. No local build target, payload store or VM disk is created; torrent/media
paths remain untouched. Only owned disposable scratch copies may be removed
after retained-copy verification.
