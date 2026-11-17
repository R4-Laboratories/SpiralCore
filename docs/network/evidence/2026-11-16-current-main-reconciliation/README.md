# Current-main networking reconciliation

This development slice reconciles the installed networking branch with MVM
canonical main `190536177eda3879eac3a745efd0005345bdd967`. It preserves original
process, storage and deadline custody while adopting canonical guest capability
negotiation, launch-scoped runtime evidence and admitted audit context.

Development source is MVM
[`8ec3f12739737f12cff3a4ecec1b1f659c6acde9`](https://github.com/Casey-allard/mvm/commit/8ec3f12739737f12cff3a4ecec1b1f659c6acde9),
tree `d24fc2d98a4bdb26ff09a3d3465c82a7f4552f70`. It corrects a new test's Ping
assumption after merge commit `9a518dc`. That merge's parents are the
previously qualified helper candidate `3a4f6870e44e11aa66bcccc7e6f2f2e69db947a0`
and canonical `190536177eda3879eac3a745efd0005345bdd967`. All 4,827 tracked
files match the initial explicitly non-Git HDD build snapshot and the later
genuine Cargo-fetched Git checkout. The clean committed
source manifest SHA-256 is
`848ab0cd77aeb42321a676159987d2bdbca1a2b8d63f14e592b3160a06f05e1b`.
All 4,827 Git blob IDs, SHA-256 values, exact lengths and executable modes also
match the operator's actual committed tree. The independent tree-proof manifest
SHA-256 is `ee773df2345c7005eaa1cc1f43bf37a5e788c5d3f3d626be1c71827f22183a4c`.

Selected SpiralCore consumers remain **MVM f2ddbcbf / shared core 8994951b**.
This packet selects no new runtime and does not establish two installed Ready
endpoints, a booted guest, WAN, fleet capacity or final upstream readiness.

## Reconciled behavior

The retained original control channel now uses MVM's `HostControlSession`.
Requests with a canonical capability requirement check the authenticated guest
advertisement before protected bytes are written. Baseline version-only verbs
retain their canonical rule. The generic entrypoint helper retains the original absolute-deadline
I/O adapter. Journal sessions retain their original guest key pin and use each
session's actual advertisement; a refusal fences the continuation without
reopening or replacing its owner.

New original-process controls omit RunEntrypoint and omit RunEntrypoint only
on a later journal session. They require typed capability
refusals, peer-observed EOF before any operation bytes, no observer events,
no reopening and collection of the original child. Existing positive, expired
deadline, partial-frame, changed-key and uncertain-retirement assertions remain.
The journal also completes canonical baseline Ping/Pong without a Ping
advertisement: MVM deliberately gates that probe by protocol version alone.

Installed bundle selection has one implementation in mvm-runtime. The client
forwards its existing host-key callback and receives the same authenticated,
launch-owned release carrier. Exact quoted pin/length checks, composed trust,
canonical kernel custody and refusal to materialize a missing quoted bundle
remain. Both persistent entry paths retain their lexical LaunchProofScope
through preparation and dispatch. Admitted audit identity comes from the held
canonical plan; it does not supply a sink or new authority.

Capture retains canonical project-directory execution and distinguishes failures
before tracer creation from failures after creation. It does not rerun a failed
command. Optional-tracer checks state their host precondition. An isolated
empty-PATH subprocess must report NotStarted and leave an otherwise executable
absolute command unexecuted. Started records wrapper/tracer creation, not proof
that the requested command reached its entrypoint.

The owned-helper scheduler and two blocking slots, original signer dispatch
handoff, seven-worker endpoint quotes, 24-worker ceiling, signed-plan, audit,
DNS, rate and confinement checks retain their previous behavior and bounds.

## Qualification boundaries

The [machine-readable measurements](measurements.json) bind the separate
campaigns, command results, costs, archive identities and remaining gates.

| Final genuine-checkout scope | Executed result |
| --- | --- |
| Formatting, Linux workspace/all-target check, explicit BDD compilation | Pass |
| Strict workspace/all-target Clippy | Pass |
| Canonical policy gates | 87 pass |
| Compiler-selected executables | 358 freshly compiled, zero reused |
| Actual binary harness set | Exact 293 expected paths; all 293 complete |
| Outer binary totals | 22,114 pass / one fail / 72 ignored |
| Rustdoc | 28 sections; 29 pass / zero fail / eight ignored |
| Executable custody | All 358 unchanged before/after; pre-execution archive replayed |
| Original source and Git metadata | All 4,989 original file hashes unchanged |

The failed binary test remains
registered_prepare::registered_direct_tcp_transfers_exact_bytes_and_refunds_a_failed_peer:
the registered target reports 10 passes / one original bound-audit-sink failure.
No assertion, deadline, authority or readiness path is weakened. This is a
failed full workspace command, not an all-green release.

The full command exits 101. Its wrapper retains aggregate 90 because the
complete-file-set check detected an added empty ignored cache lock at
target/tool-cache/zigbuild-lock.lock. No original input or executable changed.
Canonical guest build-lock acquisition explains this generated footprint.
After the unit finished and no holder was observed, cleanup opened the same
regular, single-link, current-UID zero-byte file without following a symlink,
acquired its exclusive lock, checked its named/held inode and unlinked that
original file. The empty parents were removed. The full original source/Git
file set then matched exactly. The original file-set failure and aggregate 90
remain unchanged in the measurements; cleanup does not rewrite a run green.

The initial export run remains separately failed: all 293 binary harnesses
complete with 22,110 passes / five failures / 72 ignored. Four failures require
real Git source identity; one is the same audit refusal. Rustdoc reports 26
passes / three SDK failures / eight ignored, plus an aborted CLI section with
no completion footer. Concurrent policy generation rebuilt three non-test SDK
emit binaries and shared SDK library artifacts; all 293 test ELFs stayed
unchanged. That orchestration error and its failed custody receipts remain.
All overwritten emit preimages survive in the replayed prior archive. The
genuine campaign serializes policy generation before workspace build/test;
the Git-dependent cases and all doctests pass there. These campaigns are not
summed into a green result.

Focused execution separately reports 199 outer passes / two ignored across
17 nonempty selected scopes. One optional child-trace observation returns early
because it yields no events; it is a conditional gap, not tracer qualification.
The isolated missing-tracer refusal and original-process capability controls
execute successfully. BDD scenarios, new signed-runtime/guest boot, native
macOS/Windows qualification, real WAN and capacity remain NOT_RUN here.

Native Linux x86_64 qualification uses Rust 1.97.1, two build jobs, disabled
incremental compilation and the existing shared Cargo target/cache on HDD.
Build/test units use 6 GiB RAM, zero swap, 200% CPU and 512 tasks. Tests use a
short private tmpfs root. No VM, Nix, cloud or new scheduler service is activated.

The initial compile refused one remaining old session.write call on journal
Ping. Its failed result and unchanged source receipts are retained. The call
now uses canonical capability-checking send through the same deadline adapter.
The first focused control run reports 13 passes / one failure / one ignored:
a newly added test incorrectly expected Ping omission to refuse. The canonical
version-only Ping behavior was correct. The final test pairs successful
baseline Ping with the later capability-gated RunEntrypoint refusal; product
enforcement is unchanged. The original failed source/run and all 358 compiler
ELFs are retained in a replay-verified HDD archive before rebuilding.
Independent source/security review also found and closed optional-tracer test
portability and qualification-script attribution gaps. Runtime-owner review
found no additional actionable defect in the resolved patch.

Qualification evaluates command exits, source hashes, complete file sets and
compiler-selected executable hashes separately. Focused tests require a nonzero
completed outer libtest result, preserve ignored and conditional-skip counts,
and do not aggregate nested peer harness totals. Execution is bound to the
compiler's complete source baseline, with unconditional post-checks even on
failure. The initial build launcher reported source-guard fields separately
without folding them into its exit; those original fields must all pass and
remain part of the acceptance evidence.
The genuine build launcher also retains policy status separately; acceptance
requires its recorded policy exit zero as well as every build/source/Git/file
guard zero. Its final exit alone is not an aggregate policy verdict.

## Actual helper bytes and costs

The same frozen source's native helper-only run signs and verifies both original
registered chains and collects the original processes. This is two registrations
in existing helpers, not two installed Ready endpoints or booted VMs.

| Helper-only observation | Recorded value |
| --- | ---: |
| Native sampled daemon / signer peak threads | 4 / 4 |
| Broker framed IPC request / response bytes per registration | 5,654 / 5,024 |
| Partial signer request / response bytes | 5 / 0 |
| Exact original stored bytes / signed entries per registration | 19,224 / 32 |
| Setup through verified retirement | 2.523440918 s |
| Signing window with offered partial frame | 2.385733939 s |
| Complete two-test command wall / user CPU / system CPU | 2.53 / 0.13 / 0.07 s |
| Command maximum RSS | 42,524 KiB |

The separate blocking-pressure control observes three runtime threads plus its
foreground owner. Native helper CPU ticks cover post-registration through
post-signing and exclude retirement; whole-command CPU includes orchestration,
pacing and verification. No per-side/setup-only CPU, VM launch latency, physical
energy or network throughput is inferred. No new handshake byte total is
measured by the capability negatives; they prove zero protected operation bytes
after a refused capability.

Both helper tests pass with command exit zero. Its broad wrapper retains 91
because it also checked the three unrelated SDK emit paths already regenerated
by the preceding policy stage. The actual test/daemon/signer bytes remain
unchanged and source-bound; all original emit preimages are preserved. That
aggregate result is not hidden or reclassified as successful.

## Ari's integration path and remaining gates

The [audit-interface request](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6928066239)
still needs the intended confined bound sink/durable epoch-health capability
and original-deadline missed-decision accounting. No fixture sink, whole-audit
directory grant, disposable health relocation, late append or suppression
substitute is selected.

Ari's [new manager-originated service direction](https://github.com/tinylabscom/mvm/issues/5255#issuecomment-6931080540)
requires an unattended supported profile, actual manager-created services and
one keyless mvm-mailbox-scheduler executable in the existing mvm-hostd package
per unprivileged OS-user/canonical runtime instance. Its contract, current-
generation transactions, confinement, resource/lifetime qualification and
setup authority remain separate. This networking reconciliation implements
none of that role and does not import the isolated private proof operation.

Ari's [later U1 assignment](https://github.com/tinylabscom/mvm/issues/5255#issuecomment-6932578338)
permits only a pure test-only declaration model inside mvm-hostd. Both expected
and reported records are untrusted; syntax, checked arithmetic and subset
agreement establish no installed/enforced/Ready/authorized state. Its fixture
values are synthetic, not measured production budgets. No production API,
signed DTO, persisted format, manager client, unit renderer or model import is
part of this networking slice. Its planned checks are not completed results,
and its declarations do not fund the additional scheduler role.

Both unchecked #258 criteria and all five #42 outcomes remain open: joined
ownership/resource/recovery; sustained persistent bidirectional traffic and
recovery; shared-core FlowMux plus separately admitted access; supported native
roles/backends/platforms and two-host WAN; capacity/fairness/backpressure/tails/
recovery/saturation and the complete upstream contribution. Compatible signed
guest-runtime publication remains Ari's gate; private MVMD integration remains
external. No final upstream MVM contribution is submitted.

The canonical-basis CI snapshot contains failed
[boot-latency](https://github.com/tinylabscom/mvm/actions/runs/43972742942/job/133668846081)
and [live frozen-volume](https://github.com/tinylabscom/mvm/actions/runs/43972742942/job/133668846223)
jobs. Both refuse during guest-runtime assembly because the musl build has no
libmvm_host_services.so after reporting an unsupported cdylib crate type. They
do not reach their boot/latency witness; no latency-ceiling breach or successful
guest qualification is inferred. The original logs are retained. This snapshot
belongs to canonical 1905361, separately from the local development run.

A later read-only refresh finds canonical
`08a2d86d4c1297c94d6e7f4479e8eb8a9fdb3708`. Its
[#5639 / 842e24e](https://github.com/tinylabscom/mvm/pull/5639) preserves a musl
shared object's own target flags instead of overriding them with encoded global
flags, directly addressing the source mechanism behind that assembly failure.
A successful rebuilt signed runtime and boot remain separate evidence.
[#5641](https://github.com/tinylabscom/mvm/pull/5641) adds canonical denial-stage
labels and readers; it supplies no bound sink/health or missed-decision
interface. Both changes remain outside frozen 8ec3f12 qualification. Their
bounded reconciliation is required before adopting a newer consumer; this
packet does not imply qualification of a moving main. The
[coordination update](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6932825147)
records this distinction and avoids duplicating the upstream build repair.

## Storage and retained history

Before cache cleanup, the previous seven additional compiler-produced helper
binaries were losslessly archived and replay-hashed: 47,590,949 compressed bytes,
SHA-256 `c2529e1730b7e23c1e74e3632d2228c29623036c981f13a93e4a49c49156aea5`.
That complements the prior executed-ELF archives without asserting that every
compiler-produced binary was executed. All 4,822 previous source files matched
their frozen manifest before replacing the owned build snapshot. Only the
verified renamed old audit-client path was retired.

Workspace-local disposable compilation entries were cleaned, reporting 2.1 GiB
removed; shared dependencies and unique evidence remain. The former 107 GiB
scratch store remains verified/private-archived on HDD. No local Cargo target
or duplicate raw payload dataset is created; torrent/media paths remain intact.
