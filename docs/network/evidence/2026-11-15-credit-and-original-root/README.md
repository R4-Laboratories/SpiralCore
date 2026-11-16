# Returned upload credit and original-store retirement — 2026-11-15

[PR #305](https://github.com/R4-Laboratories/SpiralCore/pull/305) fixes returned
upload credit in the canonical FlowMux validator and adopts Ari's typed local
admission stages. [PR #306](https://github.com/R4-Laboratories/SpiralCore/pull/306)
qualifies original Root funding through a populated store and actual selected
endpoint retirement. Both reviewed/tested source slices are merged. They advance
[#258](https://github.com/R4-Laboratories/SpiralCore/issues/258),
[#44](https://github.com/R4-Laboratories/SpiralCore/issues/44) and the full
five-part [#42 program](https://github.com/R4-Laboratories/SpiralCore/issues/42).
The complete MVM contribution remains unfinished.

## Exact source and publication

| Identity | Credit/admission slice #305 | Original Root witness #306 |
| --- | --- | --- |
| Reviewed SC head | `bb188c9c8ef6fc690c291ce2ea0f6ce4d5475aca` | `ee02e14e9b44bf530f485036e9a3708c87f556d4` |
| Tested CI merge | `c844746c01f7c869269d0f8f985df0deaa8ba4ac` | `9c221043e95dbba142fb16caf785ec64956ed0bc` |
| Actual release merge | `758c7ee0a63d308d3e0216f45472fb27a4ebe0df` | `f02241885ff4c92062e283299a9695cb81d7e8e3` |
| Identical reviewed/CI/release tree | `5cf6545da4cd206ca3f7a276530442fa7d8f3c15` | `31872b2099b14007446621186784a2e5d296f8c7` |
| Successful checks | 35 | 11 |
| Genuine signed export | Skipped / NOT_RUN | Skipped / NOT_RUN |

Both slices select contributor MVM
`b2def5fd556e6ab9aabeb281b554c6ef6bb060d9`, tree
`81a712066d4c7dfb28e48cfd718c93e39ee4c47c`, incorporating canonical main
`dd2af0833eeaf1e2abd738e694dc1b463140ff25` and merged MVM #5564. Both direct
MVM SpiralCore dependencies and all five locked core packages remain at reviewed
`8994951b7c13fe5acf6f5ea3060ad4e51dc473c2`. Canonical journal `48fa5b48`
aliases remain explicit and unchanged. Eight graph comparisons validate exact
pins: initial rc.5→rc.6 changes only MVM-owned package labels; final graphs
preserve versions, packages, edges and features from the repaired predecessor.

## Credit refusal and minimal repair

The isolated test-only source `54cf0ae1` fails at round 16, the seventeenth
16 KiB block: `stream 1 sent 16384 bytes with only 0 credit`. The host had
returned WindowUpdate credit without replenishing the same canonical validator
that debits uploads. Source `41169543` admits the canonical HostToGuest
WindowUpdate facts through that validator before writing the frame. The
validator guard is released before session/writer work. Exact 320 KiB upload
and echo pass, together with 119 scoped FlowMux controls and large-download
coverage. Zero, overflow, lifecycle, credit caps and deadlines remain enforced.

The original installed test still requires all **64 distinct 16 KiB blocks**.
Its earlier failure after 16 publications is retained independently; that raw
Root failure records a closed guest barrier and independent peer error, not the
explicit GoAway text. The isolated RED supplies that more precise diagnosis.
Independent errors and both peer joins remain required on success and failure.

## Canonical admission integration and full-source qualification

The networking path composes with private hostd
ResolvedLocal → VerifiedLocal → AdmittedLocal stages and the existing one-use
dispatch tail. Ordinary launches keep their canonical stages. The selected
signed-runtime branch keeps bundle authentication, plan/image/config projection,
original parent and workspace custody, backend/kernel/root checks and original
resource owners before effects. The private optional authority carrier avoids
an oversized enum without boxing or suppressing a lint. Admission and its audit
are each recorded once. This is not the later CLI/client migration.

| Frozen corrected-source scope | Result |
| --- | --- |
| Format, strict workspace Clippy, all-target and explicit BDD compilation | Pass |
| Canonical check-all | 86 gates pass |
| Outer binary tests | 21,814 pass, zero fail, 69 ignored, 284 targets; 19 nested summaries excluded |
| Doctests | 27 pass, zero fail, eight ignored; 29 summaries in 27 sections |
| SDK | 175 live, 29 owner, 74 supervisor and one addressed-journal control pass |
| Input guards | 4,717 MVM / 486 SDK inputs and complete file sets unchanged |
| Executed binary identities | All 284 actual Running paths match prehash set; all posthashes pass |

MVM invocation `695498d341a849c093056d7e78b92a4e` rebuilds 260 executables and
verifies 24 unchanged artifacts from the same physical root. No checked
prior-root strings remain; freshly built mvm-build binds the current source.
Its 284 selected binaries total **3,781,944,272 observed bytes** on Wick and
are not distributed here. Source accounting is **2,997 Rust files /
1,050,561 SLOC / 1,322,770 physical lines**, exact ceilings without headroom.

Workspace test command: **9:03.03 wall / 261.21 s user / 109.94 s system**,
including 49.58 s redundant recompilation whose executable hashes stayed
identical. Whole bounded qualification: **20:57.371 wall / 27:32.520 aggregate
CPU / 6 GiB cgroup peak**. SDK invocation `af891af6384f4f98ac60a6986d262c75`
records **4:10.304 wall / 5:53.505 aggregate CPU / 3.7 GiB cgroup peak**.
These are command costs, not transfer latency, endpoint memory or energy.
Runs retain two CPUs, zero swap, NOFILE 4096 and 8192 KiB stack. Queue scripts
request TasksMax 512, but the final MVM/Root snapshots do not record that field;
this packet does not prove its effective value for those final invocations.

The separate `41169543` baseline passes 21,812 binary/27 doc tests and 86 gates;
it does not qualify the later admission merge. The first merged `a0ee61bb`
candidate stops in strict Clippy. Both receipts remain separate.

## Original Root → child → populated store → retirement

The witness follows actual signed software admission and audit through the
original VM reservation, launch anchor, registrar/helper, InstalledHostServices,
Root partition and selected Linux child. Self-enrolled test trust is not an
official runtime signature or VM boot. It verifies exact trailing-zero readbacks,
**64 distinct original-directory publications / 1,048,576 application bytes
each direction**, canonical credit handling and actual encrypted guest I/O/TLS.

The explicit existing **2 GiB operator profile** funds the selected partition;
the default 512 MiB profile and aggregate 24-worker ceiling remain unchanged.
The child has a **295,698,432-byte hard address-space limit**. All 64 samples
bind its PID/start time, observe seven concurrent threads and at most three
classified original-store FDs. Samples do not establish transient allocator,
RSS or descriptor peaks. Funding remains unchanged throughout load.

The observer binds original device/inode, canonical flow/namespace, 64 expected
object identities, private store profile and held writer lock. Actual child
shutdown yields the original one-use retirement receipt; parent cleanup then
unlinks the original directory and releases its lock. Disposable store disk and
4,096-byte parent metadata credit return. VM, FD, worker and other retained
resource debits stay pending actual VM exit; no VM receipt is fabricated.

Final Wick invocation `8ad3de5f65674df6880df24fc5572997` passes strict all-target
Clippy and the exact ignored selector. All **490 committed Root inputs**, all
4,717 MVM inputs and four compiler-selected executable hashes pass before/after.
The actual Cargo Git dependency's source also matches the frozen MVM manifest.
Whole unit: **7.629 s wall / 9.443 s aggregate CPU / 1.1 GiB cgroup peak**.

| Original Root host-process measurement | Wick | Native Linux x64 | Native Linux ARM64 |
| --- | ---: | ---: | ---: |
| Setup ms | 70.256044 | 105.021818 | 222.023401 |
| Open through verified half-close ms | 372.721632 | 1089.045518 | 1206.144199 |
| Sender thread CPU ms | 143.380211 | 349.875932 | 388.740392 |
| Independent peer thread CPU ms | 19.671367 | 51.222169 | 54.678088 |
| Whole child CPU collected at retirement ms | 229.618 | 652.711 | 747.587 |
| Child shutdown/receipt wall ms | 27.802680 | 53.327111 | 67.610324 |
| Whole fixture wall/user/system seconds | 0.47 / 0.36 / 0.07 | 1.27 / 0.96 / 0.25 | 1.51 / 1.00 / 0.37 |

Each row records **1,078,631 sealed request / 1,078,502 sealed response bytes**
for guest load, plus **103,046 received / 102,623 sent TLS bytes** at the
independent peer. These are separate accounting boundaries; do not sum them as
one link. Child CPU is whole-child cost collected at wait, not shutdown-only
CPU. Shutdown/receipt clocks end before parent cleanup; the whole fixture clock
includes that cleanup. Full raw samples and ledger snapshots remain archived.

Native CI retains the eight canonical endpoint controls, four collector
controls and two worker controls, then executes the exact Root case on Linux
x64/ARM64. Fresh compiler-selected fixture/helpers, four executable hashes,
4,717-source clean Git/tree/full-set guards and EXIT checks bind each result,
including failure. Mac retains its existing controls but does not run this
Linux-only selected endpoint. The canonical job's 30-minute limit is unchanged.

## Native FlowMux component measurements

Seven rows each pass 89 component and 20 ordered controls, with exact
**67,108,864 application bytes each way** and cross-matched **5,395,050 /
5,395,015 TLS bytes** (10,790,065 total). Original two-second operation and
240-second bulk guards remain. These are native host-process components.

| Native component | Lifecycle seconds | Whole-process CPU seconds |
| --- | ---: | ---: |
| Linux x64 | 3.405328 | 5.283821 |
| Linux ARM64 | 7.752550 | 11.192633 |
| macOS Intel | 61.295514 | 41.469264 |
| macOS ARM64 | 19.801478 | 18.832076 |
| Windows 2022 x64 | 28.368763 | 55.171875 |
| Windows 2025 x64 | 34.812042 | 62.609375 |
| Windows 11 ARM64 | 145.188718 | 100.765625 |

Windows ARM is slower than the prior 118.463831-second sample. No causal
performance improvement, per-side process CPU, physical energy or send-only time
is inferred. [Credit native measurements](native/credit/native-measurements.json)
and [Root native measurements](native/root/native-measurements.json) preserve
full clocks, accounting scopes, identities, raw samples and unknown fields.

## Evidence and reproduction

The 28 original #305 ZIPs verify **1,731,278 bytes / 824 members / 644 embedded
checksums / 15,359 source-hash rows**. Three installed native consumers retain
the original eight controls, worker quote and returned-credit regression: ten
distinct passing controls each. The original destruction-before-returned-credit
probe stays. The ten original #306 ZIPs verify **3,061,826 bytes / 194 members /
24,785 source-hash rows**. Those ten contain zero embedded SHA256SUMS manifests;
their API ZIP digests, source guards and runner-recorded executable identities
are checked directly. Native executable bytes are absent from both sets.

The [operator archive](operator/wick-records.zip) retains **410 records /
67,474,159 uncompressed bytes / 10,712,279 ZIP bytes**, with per-member lengths
and SHA-256 in its index. It contains commands, compiler JSON, source/executable
guards, full stdout, timings, dependency graphs and failed phases. It excludes
private runtime homes, signing keys, payload stores, source exports and binaries.
[Failure journal](operator/failure-journal.md) records the scope of each loss.

Run from this packet directory, with the SpiralCore checkout containing both
immutable source commits:

```sh
python3 operator/verify-credit-native.py native/credit /path/to/SpiralCore operator/frozen-mvm-inputs.sha256
python3 operator/verify-root-native.py native/root /path/to/SpiralCore operator/frozen-mvm-inputs.sha256
python3 operator/verify-wick-records.py operator/wick-records.zip operator/wick-records.json
python3 operator/verify-qualification.py /path/to/SpiralCore
```

Native verifiers read committed Git blobs at the exact reviewed heads, not the
caller's current working-tree bytes. They are read-only by default; `--write`
is for initial report generation only. Packet SHA256SUMS excludes itself.

## Current authority and remaining acceptance

Ari's [latest sequencing note](https://github.com/tinylabscom/mvm/issues/5520#issuecomment-6901815825)
retains CLI → mvm-client → hostd, opaque one-shot admission, admission before
warm selection and existing audit/OCI provenance and Required-tier sync overlap.
The networking contribution does not take over the #5569 client migration.

At publication refresh canonical main is `05227676`, five commits beyond the
qualified `dd2af083` base. In addition to #5570's installed-release demo script,
#5559 changes build/crypto ownership and explicit BDD dependencies; #5571 updates
contribution instructions; #5573 suspends the footprint ratchet; #5565 splits
the workload runner. These changes are **not qualified by this packet** and need
bounded reconciliation. The frozen contribution's measured source accounting
and security/deadline checks remain. [Authority snapshot](operator/ari-alignment.json).

Official rc.5 exists, but the retained archive inspection records absent
`mailbox_reader_v1` declarations for both architectures. It does not establish
contribution compatibility. Ari's compatible signed publication, authentication
and actual boot qualification remain under #268; private MVMD and #5057
source-pin coordination remain external gates.

Both remaining #258 acceptance items stay open: full namespace/entry/iterator/
allocator/concurrent funding and the complete negative matrix, including executed
foreign-UID and broad-mode refusals. The explicit 2 GiB witness does not prove
default-profile fit or macOS selected enforcement. All five #42 outcomes remain
required: installed ownership/aggregate budgets/shutdown/recovery; persistent
final-ACK recovery/retention/expiry and sustained large/control traffic; FlowMux
and separately authorized owner access; supported Linux/macOS/Windows roles and
canonical two-host WAN; capacity/fairness/backpressure/recovery/tails/saturation
and the final reviewed upstream contribution. No FIPS/CUI, host-blind, energy,
fleet or complete-delivery claim is made.

No local Rust build tree, VM disk or raw payload store was created. Shared Cargo,
runtime/session data, historical evidence and pre-existing `.serena/` remain.
Owned disposable duplicates may retire only after reviewed publication and
no-handle checks. The unexpected local free-space drop is still under
investigation; it is not attributed to these small archives without evidence.
The read-only [disk observation](operator/disk-observation.json) records 16 GiB
of allocated encrypted swap with about 14.4 GiB in use. There is no earlier
swap baseline establishing the cause of the entire free-space delta. Active
swap, other applications and conversation databases are preserved.
