# Original custody after final parent-sync refusal

[Source PR #315](https://github.com/R4-Laboratories/SpiralCore/pull/315) merged at
`a0068f9b060f61e0934e0bbf1762756c88aa2430`. Reviewed head `698c1fc5` and CI merge
`76665cad` have the same tree as that source merge:
`8438038ddb3717b48752b69301a121a45fd6e779`.

The selected contributor MVM source is
[`f2ddbcbf1039405698e1ac0dcb16e29a5bf3b676`](https://github.com/Casey-allard/mvm/commit/f2ddbcbf1039405698e1ac0dcb16e29a5bf3b676),
tree `165f5cc95fda4cba7da885e6546d22be8d3e94a5`, based on canonical `2b1cd835`.
Both direct MVM dependencies and all five shared-core packages retain reviewed
SpiralCore `8994951b7c13fe5acf6f5ea3060ad4e51dc473c2`. The canonical journal
aliases at `48fa5b48` remain separate. Later upstream changes are not
qualified by this frozen source packet.

This is a test-only MVM consumer slice. It adds no production authority, codec,
protocol, dependency, deadline or budget change. The 16 revision-only consumer
files contain exactly 159 pin substitutions. All remaining #258 and five-part
#42 acceptance gates stay open.

## Exercised retirement boundary

Successful final directory removal is insufficient to return resource credit
when syncing its original parent refuses. The Linux fixture opens an `O_PATH`
parent during initial preparation and retains it throughout. Relative operations
work; `fsync` on that descriptor returns `EBADF`. No live FD is replaced or
invalidated and no syscall failure is injected.

A nondestructive errno probe occurs while the child remains named and the
original owner remains linked. The permit's **first cleanup attempt is its
Drop**. After removal, the same held child and original owner have zero links,
but the original writer lock and lease survive in quarantine. The original
parent/child Arcs, name/inode commitment, all resource charges and prepared
vector pointer/capacity remain. Admission is fenced, drain refuses and the store
destruction observer is uncalled.

The canonical fixture publishes and independently reads back 49 exact bytes,
with 268/218 serialized in-memory request/reply bytes and **zero network bytes**.
The retained active operation charge is 9,340,860 resident bytes, 17,825,792 disk
bytes and 1,048,576 carrier bytes. The reported `cleanup_drop_wall_ns` includes
the whole synchronous Drop/cleanup/quarantine attempt; `errno_probe` describes
the earlier nondestructive probe. Phase CPU is not measured inside this fixture;
the original external command CPU/time records remain separate.

This is a funded pool fixture, not a registered-child fsync failure. Native
Linux x64 and ARM64 each execute the new control 20 after the unchanged controls
0–19. macOS ARM64 retains those 20 controls and explicitly records the Linux
mechanism as `NOT_RUN`. All **62 controls** pass. Original root-UID controls,
the explicit unprivileged final-rmdir test and destruction-before-returned-credit
assertion retain their order and privileges.

## Source, SDK and paired qualification

Frozen MVM passes **21,891 outer binary tests / 27 doctests**, zero failures,
with 72 binary and eight doctest ignores. Formatting, strict workspace Clippy,
native Linux all-target checking, explicit BDD compilation, footprint measurement
and all 86 policies pass. BDD scenarios are **NOT_RUN** and the footprint ratchet
remains measurement-only.

All 4,758 source inputs and complete before/after file sets match. The 286
compiler-selected test executables are exactly the actual execution set. All
original pre/post hashes agree: **260 were recompiled; 26 reuse the verified
original same-root `1298ffe9` pre/post chain**. The current hostd test executable
was rebuilt. Nested subprocess summaries are excluded from outer binary totals.

The full test command records **482.38 s wall / 175.20 s user / 88.33 s system**.
The entire qualification unit records **952.197810 s wall / 1,060.562761 s
aggregate CPU**, 6,442,450,944 bytes charged peak, zero swap and zero OOM/OOM-kill
events. Its 5,002 cgroup `max` events show that it reached the existing 6 GiB
ceiling. Charged memory includes the build unit's accounting; it is not process
RSS or evidence of unused headroom.

SDK live **175/one ignored**, owner **29**, supervisor **74** and addressed
journal **one** pass. Nine actual executed SDK identities match original
compiler/pre/post records. All 490 exported SDK inputs, 4,758 MVM inputs and the
actual Cargo Git checkout match their full sets; its sole additional file is an
empty `.cargo-ok`. Four invoking roots retain actual `mvm-core`/`mvm-crypto`
**opt2, debug assertions and overflow checks**, with all 16 artifact postchecks.
The SDK unit records 326.178886 s wall / 512.547916 s aggregate CPU, 4,214,304,768
bytes charged peak and zero swap. Its small ext4 fixture is removed only after
an empty-directory check; builds and cache remain on HDD.

Paired component tests pass **18/one ignored emitter**, across four actual
executed targets. All four report `fresh:true` and exactly match the prior
`retention-paired1` `fresh:false` compiler/pre/post records and preserved
executables. This is verified byte reuse, **not new compilation**. All 17 paired
inputs, five fixtures and SDK/MVM guards match. Six path packages resolve through
the original source link. Actual paired crypto **opt0**, debug assertions and
overflow checks remain distinct from SDK opt2. The six unused-patch warnings are
identical to the predecessor; no lock update or feature widening was used.
The paired test command records 0.07 s wall / 0.04 s user / 0.02 s system; the
whole cached qualification unit is 0.694895 s wall / 0.625774 s aggregate CPU.
These are component/regression costs, not cold guest-launch measurements.

Eight strict normalized dependency graphs plus the canonical Linux alias match
the predecessor byte-for-byte after only expected source/root normalization.
Package, node, edge and feature counts remain in the captured graph summaries.

## Native bytes and costs

All **35 executed source-PR checks pass**. Genuine signed export is skipped and
remains **NOT_RUN**. Six workflow identities, all 36 check identities and all
28 original ZIPs are retained. The ZIPs total **3,667,234 bytes**, containing
912 members, 644 embedded member hashes and 25,010 source-hash verification rows.

Each of seven FlowMux rows recovers exactly **67,108,864 bytes in each direction**.
Its actual TLS counters cross-match the peer: **5,395,050 + 5,395,015 = 10,790,065
bytes**. Original two-second channel and 240-second fixture guards remain.
Lifecycle ends after worker joins and store cleanup. These are native host-process
component rows; neither their clocks nor their byte savings establish WAN results.

| Native role | Lifecycle wall, s | Whole-process CPU, s |
|---|---:|---:|
| Linux x64 | 3.440390 | 5.318766 |
| Linux ARM64 | 11.384553 | 12.772461 |
| macOS ARM64 | 19.214980 | 18.054873 |
| macOS Intel | 77.644388 | 53.767645 |
| Windows Server 2022 x64 | 38.336320 | 48.515625 |
| Windows Server 2025 x64 | 67.484762 | 83.796875 |
| Windows 11 ARM64 | 138.934894 | 96.609375 |

Setup clocks and unavailable per-phase CPU/RSS fields remain in the original
records. Whole-process CPU is not sender-only or receiver-only CPU, physical
energy or a causal optimization result.

Both Linux original-Root rows retain 64 distinct publications and exact 1 MiB
each direction, the existing explicit 2 GiB host envelope and 295,698,432-byte
child address-space limit. Each has 64 same-process samples and a peak of seven
threads. Actual peer TLS bytes are 103,046 received / 102,623 sent; sealed guest
load is 1,078,631 request / 1,078,502 response bytes. Sender-thread CPU, peer-thread
CPU, whole-child CPU collected at retirement and whole-fixture GNU time remain
separate. Endpoint shutdown timing excludes the later original-parent cleanup.
VM retirement is **NOT_RUN** and the original VM debit stays retained. These
rows boot no new guest and establish no fleet capacity.

Native ZIPs contain runner/compiler-recorded executable pre/post hashes, not the
executables themselves. No downloaded-binary rehash or artifact execution is
claimed.

## Original outcomes and preservation

The first passing focused test body called cleanup before Drop. Independent
review found that the later Drop encountered `NotFound`, so that body did not
directly qualify first-Drop fsync refusal. Its exact patch and observed executable
remain preserved. Corrected `fa806a9` focused tests pass, but full1 fails the
public-sibling naming policy. `63ef6b0` renames the fixture helper and passes its
behavior scopes but fails formatting. Final `f2ddbcb` applies the formatter and
passes every final-source gate above. Those earlier outcomes are not relabelled.

Before later builds, target-locked private HDD archives stream/replay every
member and post-hash the original paths. The current archive preserves all 286
test ELFs plus 66 prospectively captured non-test executables: 4,889,106,128 raw
bytes → 1,075,445,304 compressed bytes, SHA256
`25444da43f2e00659ff0dc8ea5010823db358d5507e6f05f4feb642138cf08d8`.
The SDK archive preserves 35 members including nine executed test ELFs, SHA256
`cc8dd6e6b6676d8732b3079f9309c2fd75dd80494f5bd7522cd4210ff5584f07`.
Paired reuse binds the original four-member archive, SHA256
`662b16d6b6b2e274142087358419a259d6e06cee1a4dd98b02873e8d85eb880b`.
Raw executable archives remain private; the public verifier replays their
captured receipts, not their absent bytes. The earlier **79 historical executable
preimages remain lost**; this campaign does not recover them.

The record container retains **478 files / 74,612,613 raw file bytes** in
**1,352,013 compressed bytes**. It includes original current source/test/failure
records and the necessary predecessor hash/reuse/graph records. It excludes
executable archives, payload stores, runtime state and credentials. Original
native ZIPs are unchanged. Download/metadata-size/inherited-verifier setup
corrections are explicit in the native controller notes; none changes CI results.

The former 107 GiB SSD scratch remains a private, verified HDD archive and an
8 KiB SSD pointer. That migration is a prior private operator observation,
outside this packet's replay. This work uses one HDD Cargo target and small
disposable fixtures; qBittorrent incomplete and media/Unmanic paths are untouched.

## Read-only replay

From the SpiralCore checkout:

```sh
packet=docs/network/evidence/2026-11-16-parent-fsync-custody
(cd "$packet" && shasum -a 256 -c SHA256SUMS)
python3 "$packet/operator/verify-records.py"
python3 "$packet/native/verify-consumers.py"
python3 "$packet/native/verify-native.py" "$packet/native" . "$packet/native/frozen-mvm-inputs.sha256"
```

The programs validate known hashes and parse records in memory. They do not
extract files, execute archived code/binaries, boot guests or contact a network.
The native verifier reads public Git blobs at the source merge. Independent
source/security, full-command, executable-archive, SDK/paired and native-artifact
reviews passed before the source merge; publication has a separate review.
No runtime workflows are selected for this documentation-only publication
(**NOT_RUN**, not runtime CI success).

## Remaining delivery gates

A read-only refresh after this qualification observed canonical MVM
`d5fb0873b72bd589a600882a643969af43c70a94`, the merged
[bound-flow audit slice #5590](https://github.com/tinylabscom/mvm/pull/5590).
Its new decision-audit path binds the admitted plan/epoch before network effects;
this frozen packet does not qualify that delta. The next current-owner port must
retain that audit path, bounded writes and original cleanup/resource custody
before the joined witness. The final PR leaves VM lifecycle behavior unchanged. Stopping the exact workload
after three consecutive production audit-write failures remains unimplemented
under #5538/#5522; notifications or health metadata do not establish original-owner
rollback.

Both unchecked #258 criteria stay open: joined original-Root/selected-process
simultaneous iterator/allocation phases, complete transient/native allocation
funding and the remaining negative/recovery cases. A valid two-traversal witness
needs two independently admitted `max_streams=1` children. Active-load and
retirement-verification lifetimes must be measured separately; independently
maximal memory, worker and FD terms cannot simply be added. Default-profile fit,
registered-child fsync failure and unavailable platform enforcement remain gates.

All five #42 outcomes remain required: installed ownership/budgets/recovery;
sustained recovery and large/control traffic; shared-core FlowMux and separately
authorized access; supported roles/backends/platforms and two-host WAN; measured
capacity/fairness/backpressure/recovery/tails/saturation and the final contribution.

Ari's canonical/V3 work still owns original admission, native/helper custody and
guest launch-witness commitment before message activation, encrypted independently
authorized bidirectional Call/Cast/Reply and separate permissions. ACK/Cast is
partial. Accepted source/runtime pins, compatible signed runtime publication and
authentication/actual boot, and private MVMD integration remain external gates.
Host-helper snapshots do not establish compatible guest runtime bytes. This
packet submits no final upstream contribution and closes no MVM, WAN, FIPS/CUI,
host-blind security or fleet-capacity acceptance.
