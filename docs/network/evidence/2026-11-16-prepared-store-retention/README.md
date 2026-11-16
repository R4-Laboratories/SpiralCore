# Prepared installed-store retention and original funding

[PR #313](https://github.com/R4-Laboratories/SpiralCore/pull/313) merged at
`3eb6fefb5c6e7ddcb1e8724c404441c14fd67054`. Reviewed head `448da7a4`, CI merge
`ccfa6b25` and release share tree `17067b9b2d6c8f5bc764fc10a8efdaf42d156231`.
Consumers select contributor MVM
[`1298ffe9`](https://github.com/Casey-allard/mvm/commit/1298ffe9b3cfa45cecb56cba8e328de00f845987),
tree `228b513d088458f17ba70f857d0bb4410c134bd0`, based on canonical `2b1cd835`.
Both direct MVM dependencies and all five shared-core packages retain reviewed
SpiralCore `8994951b`. A later canonical-main refresh reached `0858b362`; its
three relevant pool/auxiliary owner paths were unchanged, and its other changes
are outside this frozen qualification.

## Delivered boundary

The pool prepares its quarantine vector fallibly before publication or store
effects. Its checked quote covers one actual `OwnedPayloadStore` slot per
admitted `max_active`, capped at 32, and new retention-funding bookkeeping.
Installed construction spends that quote from the same original auxiliary
metadata partition and retains the actual permit. Generic parent callers
permanently carve the quote from their original resident ceiling. Successful
operation refunds and a drained pool do not release still-live metadata.
Field declaration order destroys backing storage before retained funding and
the original parent. Uncertain cleanup retains original custody and all charges
without vector growth. Cleanup checks destruction authority and auxiliary
poison/fence state; ordinary admission closure permits verified destruction.

The RED at test-only `e5091811` fails with capacity **0 versus 2**, exit 101.
The final source passes the preparation/funding tests, exact byte/identity
comparison after poisoning the real retained auxiliary ledger, and actual
registered-process cleanup through admission closure. The existing final-rmdir
refusal additionally proves unchanged backing pointer/capacity. Original controls
0–14, including destruction-before-credit control 6, remain unchanged; five
new controls run as 15–19. Three Unix targets execute all **60 controls**.
The qualification pool remains explicitly DEV arithmetic, not installed authority.

## Source, consumer and native qualification

| Scope | Recorded result |
| --- | --- |
| Frozen MVM workspace | 21,890 outer binary tests / 27 doctests; zero failures; 72/eight ignored; all 86 policies clean |
| Required source gates | Formatting, strict workspace Clippy, native Linux all-target and explicit BDD compilation pass; BDD scenarios NOT_RUN; footprint ratchet suspended/measurement only |
| Source/executable identity | All 4,758 inputs and full file sets match; all 286 selected test ELFs are freshly compiled, actually executed and match original pre/post hashes |
| SDK | 175 live / 29 owner / 74 supervisor / one addressed pass; one live installed-load witness explicitly ignored here |
| SDK custody | 490 SDK files, 4,758 exported MVM files and the actual 4,758-file Cargo Git checkout; only additional Git-checkout file is empty `.cargo-ok`; nine executed ELF identities match |
| SDK profiles | Actual core and crypto artifacts in live, adapter, witness and journal roots retain opt2, debug assertions and overflow checks |
| Paired component | 18 passes, one explicit ignored emitter, four fresh compiler-selected/executed targets; 17 unchanged paired files and five fixtures; shared source links, no duplicate tree |
| Paired profiles | Actual paired core/crypto and test targets use opt0 with debug assertions and overflow checks; separate from the four SDK roots |
| Dependency graphs | Eight strict comparisons plus the canonical Linux alias equal published cc2 graphs after exact expected source/root normalization |
| Native CI | All 35 executed checks succeed; genuine signed export skipped/NOT_RUN; Linux x64/ARM64 and macOS ARM64 each execute 20 custody controls |

The full test command records **486.79 s wall / 175.34 s user / 88.46 s system**.
The continued full2 unit, including policy checks and compilation, records
**866.028 s wall / 937.409 s aggregate CPU / manager-rounded 6G peak / zero swap**.
SDK2 records **315.042 s / 544.176 s / rounded 4G**. Paired compilation records
**18.30 s wall / 30.41 s CPU**; its Cargo test command records **0.07 s / 0.06 s**.
These are command/unit costs, not workload or send-only CPU. Wick qualification
units retain 6 GiB memory, zero swap, two CPUs, 512 tasks, 4,096 soft descriptors
and 8 MiB stack. Hosted-runner limits are recorded independently.

Seven native FlowMux rows recover **67,108,864 bytes in each direction**,
cross-match **10,790,065 actual TLS bytes**, and pass unchanged two-second
operation deadlines and the 240-second lifecycle bound. Windows ARM records
**98.043522 s lifecycle / 96.640625 s whole-process CPU**. The lifecycle includes
worker joins, endpoint destruction and store cleanup. This is an observed run,
not evidence of a causal speedup or WAN performance.

Both Linux original-Root/selected-child witnesses retain **64 publications** and
exact **1 MiB per direction**, the existing explicit **2 GiB** host profile,
**295,698,432-byte** hard address-space limit, sampled seven-thread envelope and
original descriptor bounds. VM debit stays retained without an actual VM-exit
receipt. Their child shutdown interval excludes later parent cleanup.
[Directional counts and clocks](native/root-measurements.json) ·
[Seven FlowMux rows](native/flowmux-measurements.json).

## Original failures and preservation

The first implementation compile at `8ff2ee6` fails because its new fixture lacks
the existing `Debug` supertrait; `16ae69c` corrects that. Full1 retains exit 1:
its successful source gates and 85 policies precede a missing archived `uvx`
tool. Restoring only reusable codegen tools/caches to HDD, auditing all 4,709
entries and separately passing offline `check-stubs` with no drift supports the
continued full2 run. Full2 explicitly binds those unchanged-source successful
prerequisites and then executes all 86 policies and full tests. Full1 is not
relabelled green.

SDK1 retains exit 101 with **174 passed / one failed / one ignored**. Its existing
startup test correctly refuses the new ZFS working directory: that model accepts
ext4/XFS. The existing `SPIRALCORE_TEST_DISK` option selects a small private ext4
test directory. The exact failed case passes with the same original executable,
unchanged 1 GiB address-space/filesystem/sysfs assertions, and original pre/post
hashes. SDK2's cached live ELF is accepted only through that exact source/hash
chain. The first SDK1 post-failure observation remains explicitly late. Both
small ext4 roots are removed after verification; product filesystem admission
and deadlines remain unchanged.

One initial CLI artifact request returns a 156-byte timeout response; 27 other
CLI downloads match GitHub digests. Its original stderr was not retained. The
[partial response](native/cli-download-partial.json) and
[failure inventory](native/cli-download-failure.json) remain. The connector
subsequently captured all 28 original ZIPs, with every GitHub digest matching.
No native ZIP was repacked.

The earlier campaign's **79 lost executable preimages remain lost**, as recorded
in the [preceding packet](../2026-11-16-final-store-removal/README.md). Before
subsequent compilation, new private HDD archives preserve/replay/posthash 352
current compiler executables including all 286 current test ELFs, 35 SDK
compiler executables including nine executed test ELFs, and four paired ELFs.
These archives do not reconstruct old lost bytes. This public packet contains
their inventories/receipts, not executable bytes.

## Read-only verification

The packet retains **28 original ZIPs / 909 members / 644 embedded checksums /
25,010 source-hash rows** and **507 Wick records**. The operator container is
**1,528,676 bytes** for **88,972,212 bytes** of files. Recompression of this newly
generated records container preserves its exact tar stream; original and compact
identities are in [archive-repack.json](operator/archive-repack.json).

From the repository root, with Python 3, Git and `zstd` available, and source
release `3eb6fefb` present in the local Git object database:

```sh
packet=docs/network/evidence/2026-11-16-prepared-store-retention
(cd "$packet" && shasum -a 256 -c SHA256SUMS)
python3 "$packet/operator/verify-records.py"
python3 "$packet/native/verify-consumers.py"
python3 "$packet/native/verify-native.py" "$packet/native" . "$packet/native/frozen-mvm-inputs.sha256"
```

Default verification reads data without running archived code or extracting
files. The native source verifier reads the permanent product release, whose
tree equals the reviewed/CI tree. Native ZIPs carry compiler/runner-recorded
before/after hashes, not executable bytes; this is not an independent downloaded
binary rehash. Independent reviews cover source/security, frozen-source proof,
SDK/paired integration and native artifacts, with live head/check verification.

## Storage and remaining delivery

The former approximately **107 GiB SSD scratch** is now a verified private HDD
archive; its original SSD tree was retired only after full archive comparison,
metadata checks and fsync. The old location holds an 8 KiB pointer. The archive
is **24,879,409,976 bytes**, SHA256
`bcacad1ed9760e832d982f3212df3a89a739db305f09f2cb143fff6761780832`.
These migration figures are operator observations against private receipts.
The three public replay programs do not verify the private cold archive or its
migration inventories.
New Cargo caches, source exports and executable archives use a separate HDD
workspace. Runtime/key/payload contents of that cold archive are not distributed.
Torrent incomplete paths and media service/settings were untouched. Free-space
changes also reflect other host activity.

[#258](https://github.com/R4-Laboratories/SpiralCore/issues/258) remains open for
the joined original-Root/selected-process simultaneous iterator/allocator peak,
complete transient/native allocation funding and remaining negative/recovery
cases. Parent-fsync faults, default-profile fit and unavailable platform
enforcement remain explicit. The prepared-retention sub-gap is delivered;
component tests do not complete the issue.

All five [#42 outcomes](https://github.com/R4-Laboratories/SpiralCore/issues/42)
stay active: installed ownership/budgets/recovery; sustained recovery and
large/control traffic; shared-core FlowMux and separately admitted owner access;
supported roles/platforms/backends and two-host WAN; measured capacity/fairness/
backpressure/recovery/tails/saturation and final upstream delivery.

Ari's [structural migration](https://github.com/tinylabscom/mvm/issues/5520#issuecomment-6911694133)
preserves source-specific CLI grant/budget/audit behavior. His
[V3 ownership/acceptance mapping](https://github.com/tinylabscom/mvm/issues/5255#issuecomment-6910030849)
owns original admission, retained native custody and guest launch witness before
message activation. The approved
[one-address inbox/pending-outbox model](https://github.com/tinylabscom/mvm/issues/5544#issuecomment-6909874977)
and [encryption/custody requirements](https://github.com/tinylabscom/mvm/issues/5544#issuecomment-6909984703)
require independent sends in both directions and first-class correlated
Call/Reply; ACK/Cast is partial. This slice stays in installed store/resource
custody. Ari's compatible signed runtime, accepted source/runtime pins, actual
authenticated boot and private MVMD remain external gates. No final upstream
contribution, FIPS/CUI, host-blind or energy qualification is claimed.
