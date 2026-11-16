# Final installed-store removal: current source and retained failures

[PR #311](https://github.com/R4-Laboratories/SpiralCore/pull/311) merged at
`00ec196bc70b013aa470c4b7a20affaa05c634b1`. Reviewed head `35efbb61`, CI merge
`40575540` and release have identical tree
`ff95d48508419daa17f9f6bace0e8a6dea15b162`.
Consumers select contributor MVM
[`cc2ed208`](https://github.com/Casey-allard/mvm/commit/cc2ed208a7b1adde7fa1f075bbfff2428f911e18),
tree `52e3463526c94a5941a16fa187158fd630c258bd`, on canonical `2b1cd835`.
Both direct MVM dependencies and all five locked SpiralCore packages retain
reviewed core `8994951b`. Canonical journal aliases at `48fa5b48` remain explicit.

## Delivered boundary

The new explicit unprivileged test causes a real parent-relative directory
removal to fail with `EACCES` after the codec has removed its files and unlinked
the owner filename. The original inode stays locked; the original child,
lease and all resident/disk/carrier debits remain quarantined. Restoring fixture
permissions does not reopen admission or return credit. The native workflow
keeps the original fourteen controls, their order and the destruction-before-credit
assertion; it appends control 14 and requires fifteen nonzero executions.

Linux x64, Linux ARM64 and macOS ARM64 execute all **45 native controls**.
Each new refusal control first runs the unchanged successful retirement check.
Its canonical fixture publishes and exactly reads back **49 bytes**, recording
**268/218 serialized in-memory request/reply bytes and zero network bytes**.
The fixture pool is not a VM/Root reservation.

The selected MVM also reconciles the canonical ADR-061 framing owner: the two
existing contributor helper bodies move byte-for-byte to core, and hostd keeps
its facade. Four executed compatibility controls cover bytes, caps and validation
before typed parsing. The final cc2 correction compares native `rustix::Stat`
fields in the new test, preserving device/inode identity without casts.

## Current source proof and historical executable loss

Current cc2 source execution passes **21,884 outer binary tests / 27 doctests**,
zero failures, with 72/eight ignored. Formatting, strict workspace Clippy,
all-target and explicit BDD compilation and all **86 policy gates** pass.
All 4,757 source inputs and complete file sets match before/after. Metadata,
compiler output and actual runtime identify the same **286 freshly compiled
test executables**, with matching original before/after hashes. BDD scenarios
are **NOT_RUN**; footprint enforcement remains suspended.

**The overall full2 campaign exits 1.** An additional historical-preservation
check found that shared Cargo output paths overwrote **79 older 5fe executables**;
207 remained unchanged. A separate source export did not prevent those output-path
collisions. Independent review maps every failed old hash to the freshly compiled
current cc2 hash. The older 79 original byte sequences are not recovered.
The original outcome, failure maps, old hashes and current receipts remain intact.
No passing product test was rerun to conceal this loss.

A separately reviewed strict current-source prerequisite verifies the successful
current test/source/runtime/hash records while explicitly requiring the original
exit-1 outcome and the exact 79-failed/207-unchanged partition. It does not mark
historical preservation green. Before subsequent compilation, a lossless archive
preserved 596 currently observed executables: 286 current cc2 and 310 surviving
historical artifacts. All members were decompressed, compared and posthashed.
Incremental archives similarly preserve nine SDK and four paired executables.
These executable archives remain on Wick; this public packet contains their
inventories and witnessed receipts, not executable bytes or a reconstructed
historical preimage.

The earlier 5fe full1 run separately passed its tests but exited 1 at a stale
285-target postprocessor. Its delayed 286-target postprocessing stays labelled
as such. Original 34e locked-resolution failures and the 038 macOS compile
failure are retained in [decoded-log envelopes](ci/). The envelopes preserve
the tool-decoded text, not the original HTTP/compression container.

## Qualification and clocks

| Scope | Executed result |
| --- | --- |
| Focused cc2 Linux | 12 ordinary passes, three ignored; explicit final-rmdir and two root UID passes; four framing passes |
| SDK | 175 live / 29 owner / 74 supervisor / one addressed pass; one live ignore remains unrun |
| SDK source/custody | 490 SDK inputs, 4,757 exported MVM inputs and 4,757 actual Cargo Git-checkout inputs; nine compiler-selected fresh test ELFs and matching original pre/post/runtime records |
| Invoking profiles | Core and crypto optimization level 2, debug assertions and overflow checks verified from actual artifacts in four roots |
| Paired path workspace | 18 passes, one explicitly ignored emitter; four freshly compiled/executed targets, including two zero-test binaries; 17 paired inputs and five unchanged fixtures |
| Dependency graphs | Eight strict graphs equal reviewed 5fe after exact source/root normalization; canonical Linux alias equals canonical live |
| Exact-head native CI | 35 successful checks; one genuine signed export skipped/NOT_RUN |

The failed overall full2 unit records **1,347.362 s wall / 1,715.543 s aggregate
CPU / manager-rounded 6 GiB peak / zero swap**, including compilation and checks.
The successful workspace test command alone records **501.83 s wall / 184.18 s
user / 91.50 s system CPU**. SDK2 records **477.812 s wall / 760.561 s aggregate
CPU / 4.3 GiB rounded peak**; paired2 records **25.410 s / 33.849 s / 1.6 GiB**.
These are command/unit costs, not workload CPU or send-only time. Wick product
units retain 6 GiB memory, zero swap, 200% CPU, 512 tasks, 4,096 descriptors and
8 MiB stack. These limits are not asserted for hosted runners.

Both Linux original-Root/store-load witnesses pass 64 publications and exact
1 MiB per direction under the existing explicit 2 GiB profile, seven observed
threads and 295,698,432-byte address-space limit. VM debit remains retained
without a VM-exit receipt. Their child shutdown/receipt intervals exclude later
parent cleanup. Seven native FlowMux rows recover **64 MiB per direction**,
cross-match **10,790,065 actual TLS bytes** and pass unchanged 2 s/240 s guards.
Windows ARM records **183.000709 s lifecycle / 96.578125 s whole-process CPU**.
These observations establish no causal speedup, booted MVM, WAN or fleet capacity.
[Root measurements](native/root-measurements.json) ·
[FlowMux measurements](native/flowmux-measurements.json).

## Recheck the compact packet

The packet retains **28 original ZIPs / 894 members / 644 embedded checksums /
25,005 source-hash rows** and **560 Wick records**. The records container is
1,921,298 bytes for 95,202,737 bytes of captured files. It excludes runtime state,
source exports, executable archives, VM disks and payload stores.

From the repository root, with Python 3 and `zstd` available:

```sh
packet=docs/network/evidence/2026-11-16-final-store-removal
(cd "$packet" && shasum -a 256 -c SHA256SUMS)
python3 "$packet/operator/verify-records.py"
python3 "$packet/native/verify-consumers.py"
python3 "$packet/native/verify-native.py" "$packet/native" . "$packet/native/frozen-mvm-inputs.sha256"
```

Default verification is read-only: it checks captured data without executing
archived scripts or extracting files. Native ZIPs contain runner-recorded
executable hashes, not executable bytes; their verification is not a downloaded
binary rehash. The native source verifier reads the permanent source release
`00ec196b`, whose tree equals the reviewed/tested tree.

Housekeeping removed **10,399 verified obsolete rlib/rmeta files**,
37,730,446,048 apparent bytes, after identity/hash and no-observed-handle checks.
Current qualification paths were protected. Free-space changes include other
host activity. No local build tree, VM disk or payload store was created.

## Remaining delivery

[#258](https://github.com/R4-Laboratories/SpiralCore/issues/258) remains open:
prepare and explicitly fund quarantine retention storage before effects, then
execute the joined original-Root/selected-process simultaneous iterator and
allocator peak witness. Parent-fsync fault execution and remaining platform
enforcement stay explicit. This slice does not claim all cleanup failures covered.

All five [#42 outcomes](https://github.com/R4-Laboratories/SpiralCore/issues/42)
remain required: installed ownership/budgets/recovery, sustained recovery and
large/control traffic, shared-core FlowMux and separately admitted access,
supported platform/backend/two-host WAN acceptance, and capacity/final upstream
delivery. Ari's compatible signed runtime publication/authentication/boot,
maintainer source-pin agreement and private MVMD remain external gates.
Ari's CLI → mvm-client → hostd admitted executor → runner remains the canonical
direction; this work does not take over the CLI cold migration. No final upstream
contribution, FIPS/CUI, host-blind, energy or production-readiness claim is made.
