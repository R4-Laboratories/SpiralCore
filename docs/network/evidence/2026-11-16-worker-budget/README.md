# Two-endpoint worker budget candidate

The reviewed development helper configuration fits the existing 24-worker
ceiling. It reduces each owned helper's Tokio scheduler pool from two workers
to one, retaining the multithreaded runtime and both blocking slots. The two
selected Object-TCP endpoints retain their complete seven-worker requirement.

This qualifies helper scheduling, native helper service/retirement and a
conditional ledger composition. The installed direct-TCP positive still refuses
before Ready because **FlowMux requires a bound audit sink**. Two installed Ready
endpoints, booted VMs, joined original-Root allocation peaks and complete MVM
delivery remain open.

## Source and accounting

Development MVM commit
[`c20c09006f4b306b42f6c8803df32e54272827a5`](https://github.com/Casey-allard/mvm/commit/c20c09006f4b306b42f6c8803df32e54272827a5),
tree `1c080f46b3f3b5aee3cb525a6624c00ac4527ca6`, extends reviewed development
`b8f9883` on canonical `c5de1cc7`. Its production delta is the single shared
owned-scheduler constant. Both owned binary builders already consume that
constant. Foreground ownership, blocking slots, stack sizes, I/O tick cap,
workspaces, registration, rate limits, signing, confinement and deadlines retain
their original implementations.

| Reserved component | Earlier configuration | Candidate |
| --- | ---: | ---: |
| Daemon: foreground + scheduler + blocking maximum | 1 + 2 + 2 = 5 | 1 + 1 + 2 = 4 |
| Signer: foreground + scheduler + blocking maximum | 1 + 2 + 2 = 5 | 1 + 1 + 2 = 4 |
| Two original VM observer charges | 2 | 2 |
| First endpoint | 7 | 7 |
| Second endpoint | 7 | 7 |
| Aggregate requirement | **26; refuses Full** | **24** |

Each endpoint's seven workers remain: foreground controller, session, three
collector workers, TCP relay and codec reader. The ledger's 24-worker ceiling,
guest slots and security checks are unchanged.

Pinned Tokio 1.53.3 keeps a multithread runtime's scheduler and blocking or
replacement threads within `worker_threads + max_blocking_threads`. The pressure
control independently exercises the relevant replacement path: two original
`block_in_place` callbacks enter and remain held while a separately spawned timer
task must progress. Both callbacks are released and joined, all independent
errors are collected, and counted runtime threads must stop on shutdown.

The SpiralCore ledger control retains the existing ten-helper negative: usage
12 before endpoints, 19 after the first, and `Full` for the second at 26. The new
conditional eight-helper case reaches 10, 17 and exactly 24. One further worker
refuses `Full` without changing usage. Closing admission fences both partitions;
dropping them without retirement evidence retains their original charges and
prevents supervisor completion. These controls start no helper, endpoint or VM.
They do not change the selected consumer's runtime pin.

## Executed native controls

Native Linux x86_64, Rust 1.97.1, two build jobs, incremental compilation disabled,
one shared Cargo target and cache on HDD. Each execution unit was capped at
6 GiB RAM, zero swap, 200% CPU and 512 tasks. Short private tmpfs directories
avoid the previously observed long Unix-socket fixture path problem. No VM,
Nix, GPU or cloud execution was performed.

Tests were added before changing the scheduler constant. The final paced
old-configuration RED run successfully signs and retires both exact 32-entry
chains, then both new tests fail their 26-versus-24 funding assertion. Its
pressure runtime peaks at four threads, plus its foreground owner. The first
GREEN run passes both tests; the pressure runtime peaks at three, plus the owner.

The real helper test registers `vm-1` and `vm-2` with the original daemon,
performs concurrent canonical audit calls alongside an offered partial signer
connection, and verifies each held original file against authenticated opening
and terminal inventories. It checks signatures, device/inode, exact length,
32 entries, exact event/sequence content, the two distinct workload identities,
unchanged process start identity and pre/post executable hashes. It collects
both client results and every original retirement outcome. The partial
connection is an offered adversarial input; server-side workspace occupancy is
not instrumented. Deterministic blocking overlap belongs to the separate
pressure control.

| First matched observation | Old configuration | Candidate |
| --- | ---: | ---: |
| Native sampled daemon / signer thread peaks | 5 / 3 | 4 / 2 |
| Audit broker request bytes per VM, including frame prefixes | 5,654 | 5,654 |
| Audit broker response bytes per VM, including frame prefixes | 5,024 | 5,024 |
| Partial signer connection sent / received bytes | 5 / 0 | 5 / 0 |
| Verified original stored bytes per VM | 19,224 | 19,224 |
| Setup through verified retirement | 2.505024462 s | 2.482476376 s |
| Paced signing interval including partial connection | 2.389111094 s | 2.382459044 s |
| Complete two-test command wall / user CPU / system CPU | 2.51 / 0.11 / 0.06 s | 2.49 / 0.10 / 0.03 s |

Thread peaks are sampled every 1 ms and may miss shorter transients. The
source-derived reservation remains eight helper threads even though the native
candidate sample observes six. `/proc` CPU values use 100 ticks per second and
cover post-registration through post-signing, excluding retirement. The command
CPU scope includes fixture work, pacing, identity hashing, verification and
retirement; it overlaps the helper CPU domain and must not be added to it.
Neither domain establishes physical energy.

Five further candidate repetitions pass both tests, each with pressure peak
three and sampled native peaks four/two. Setup through verified retirement spans
2.500378951–2.514540548 s in those repetitions. This small paced fixture supplies
concurrency evidence; it does not qualify throughput, fleet capacity, fairness,
tails or the cold/warm launch requirements. Broker and partial-frame counters
describe only their named local streams. Initial control and helper-internal
traffic is uninstrumented; these counters are not total MVM, TLS or WAN bytes.

Focused MVM workspace formatting and strict hostd/VMM all-target Clippy pass.
Nonzero library selections pass 36 tests: owned daemon 4, audit client 8, owned
signer client 1, retained host-agent capability/readiness 6, owned resources 6
and signer helper 11. Two initial zero-match selectors are retained as no-test
observations and replaced with verified nonzero selections. The entire unchanged
registered-TCP target reports **10 passes and one failure**, the existing bound
audit-sink refusal. The earlier failed/incomplete full workspace campaign remains
failed/incomplete. No full workspace or MVM acceptance claim follows.

The SpiralCore canonical-host config suite passes all 19 cases, including the
old refusal and new conditional pair. Strict library/test Clippy and Rust 1.97.1
formatting of the changed source file pass. Selected consumer dependencies remain
at `f2ddbcbf`; the conditional eight-helper reservation is an explicit fixture
input, not a claim that those selected binaries changed.

## Custody, review and retained failures

The clean operator checkout's 4,821 tracked-file hashes match the explicitly
non-Git HDD build snapshot at the published commit. The source manifest SHA256 is
`1732c0dd789ffde26ab0e6fd7f13d1384687a778f6d740f63c59486bb4c60ce4`.
Full snapshot file sets and content hashes remain unchanged through builds and
execution. This is an honest source-copy build; no Git metadata or executable
source identity was fabricated.

The semantic RED and GREEN test plus both helper executables were compressed on
HDD and replay-hashed before subsequent builds could replace them. Both focused
library executables were also archived and replay-hashed. Archive sizes/hashes,
per-process identities, original chain hashes, actual costs and repeated
observations are in [measurements.json](measurements.json). Raw compiler, source
manifests and test receipts remain in the bounded HDD evidence area.
RED3 rebuilds the test ELF and uses the cached old helper executables from the
initial clean build; GREEN1 rebuilds the test and both used helpers. Compiler
receipts, native executable hashes and archive replay bind those distinct cases.

Two focused independent source/security reviews found no remaining blocking
finding in this configuration/test slice. Their clearance is for development
publication and the conditional ledger control. Initial scaffold compile
failures and an unpaced burst that reached the existing audit rate remain
negative evidence; the valid fixture now spaces requests by 75 ms while the
product rate and deadlines stay fixed. Package-wide SpiralCore formatting also
exposes pre-existing formatting differences outside this changed test; those
files are retained unchanged and the changed file is checked with Rust 1.97.1.

## Integration and remaining delivery gates

The selected delivered consumer remains MVM `f2ddbcbf` / shared core `8994951b`.
This packet does not select the new helper runtime for installed service use.
Current canonical main was refreshed to `9a0a2f6`: Ari's
[#5629 audit-context propagation](https://github.com/tinylabscom/mvm/pull/5629)
and launch-scoped runtime-verification sharing are outside this frozen source.
Audit identity propagation does not supply the missing bound sink/health API.
Current-main launch-path reconciliation needs its own exact-source qualification
before consumer adoption.

There is no new reply to the
[@auser integration request](https://github.com/tinylabscom/mvm/issues/5057#issuecomment-6928066239)
at this checkpoint. Ari subsequently posted the
[persistent ownership decision brief](https://github.com/tinylabscom/mvm/issues/5255#issuecomment-6928741837):
it recommends a separate host-wide keyless scheduling role for focused review
and maintainer decision. That role is unimplemented and its resource costs are
unmeasured. The brief's legacy three-process topology and the existing scoped
owned pair measured here are distinct paths. An approved additional scheduler
will require its own budget and qualification; this 24-worker composition does
not reserve an unknown future process. MVM retains canonical admission, signed
plans, capabilities, DNS/rate/audit checks, runtime production and lifecycle
ownership. #5628 protocol cutover gates remain authoritative. This slice adds no
protocol, role, authority or readiness fallback.

Next installed evidence requires the canonical confined audit sink, durable
health/deadline accounting, current source reconciliation, then the original
two-child Root allocation/recovery witness. Compatible signed runtime/boot,
persistent bidirectional Call/Cast/Reply, separately admitted owner access,
supported platform/backend roles, actual WAN, measured capacity and private
MVMD consumer integration remain separate gates. All five #42 outcomes remain
active. The final upstream MVM contribution waits for the qualified complete
supported deliverable.
