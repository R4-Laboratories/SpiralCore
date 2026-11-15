# Selected endpoint worker funding — 2026-11-15

[SpiralCore PR #303](https://github.com/R4-Laboratories/SpiralCore/pull/303)
repairs an admitted two-worker declaration for a serial Object-TCP runtime that
can retain seven concurrent workers. This is a tested source/component slice
under [#258](https://github.com/R4-Laboratories/SpiralCore/issues/258),
[#49](https://github.com/R4-Laboratories/SpiralCore/issues/49) and the full
five-part [#42 delivery program](https://github.com/R4-Laboratories/SpiralCore/issues/42).
It does not close those issues or submit the final upstream contribution.

## Source identities

| Identity | Revision / tree |
| --- | --- |
| Reviewed SC candidate | `3f7ca46fc26b411bd07ac9e9bf7d3fa867f4f42c` |
| Tested CI merge | `602402a0e22c83971e175461ebfe73af82369340` |
| Actual SC merge | `959cb7f52d18c7070fb4dcf0880b32c887bceed3` |
| Identical candidate / CI / release tree | `1c5bb6368557b3f87cbe54502d9c61206afab385` |
| Contributor MVM | `8345cb682ecbc755c7312ba7a5a28c641aa667d0` |
| Contributor MVM tree | `9b6fe6888a499b3f7dc4f89d92dcdc558fed62fe` |
| Qualified canonical MVM base | `7eb015747545051fb5e26ae2c6d5fa4c86e4de00` / MVM #5557 |
| Both direct MVM core dependencies / all five locked packages | Reviewed `8994951b7c13fe5acf6f5ea3060ad4e51dc473c2` |

The CI merge's [actual Git API receipt](native/ci-merge-commit.json) records
base `82ee11a0` and the candidate as parents. Sixteen SC consumer declarations
advance coherently to the contributor pin. Eight matched metadata graphs retain
packages, edges and features after exact pin validation. Canonical journal
`48fa5b48` aliases remain explicit and unchanged.

## Runtime change and preserved limits

The canonical runtime requirements compose one foreground controller, one
session, three collector workers, one owned TCP relay and one codec reader.
The provider and actual selected child check the same seven-worker quote before
component claims, listeners, collector or session effects. The selected profile
keeps `max_streams=1`; this is not a pipelining implementation.

The original shared limit stays **24 workers**. The existing ten-worker helper
class plus two VM exit-capture workers already charges 12. One selected endpoint
raises that to 19; a second would require 26 and must refuse with `Full` before
issuance. Two guest slots do not promise two fully selected endpoints. Tests use
actual software admission and retained original launch owners, without fabricating
a VM/backend retirement receipt or returning uncertain credits.

The original reader/store custody controls, destruction-before-returned-credit
assertion, independent worker failures, authority, security and deadlines stay.
No codec, protocol, signed-plan authority, resource allowance or guest NIC is added.
The complete MVM Rust tree is 2,996 files, **1,050,291 SLOC / 1,322,491 physical
lines**, including tests, generated and vendored source. Relative to the reviewed
predecessor that is +70 SLOC / +79 physical lines; exact ceilings have no headroom.

## Frozen-source execution

| Scope | Result |
| --- | --- |
| MVM format / strict workspace Clippy / all-target / explicit BDD | Pass |
| Canonical `check-all` | All 86 gates clean |
| Outer binary harnesses | 21,811 passed, zero failed, 69 ignored; 284 targets; 19 nested summaries excluded |
| Doctests | 27 passed, zero failed, eight ignored; 29 summaries in 27 sections |
| SDK | 175 live, 29 owner, 74 supervisor; strict all-target Clippy and adapter/witness/guest checks pass |
| Replacement addressed journal | One pass, distinct retained application receipt and executable identity |
| Registered direct TCP | One pass, eight filtered; exact positive path at the seven-worker declaration |
| Native CI | 35 successful checks; one genuine-export skip / NOT_RUN |

Final coherent MVM invocation `51dee07d51664d2bbc86999815a564c6` binds all
**4,716 inputs** before/after and an unchanged complete file set. Compiler JSON
selects **284 actual test ELFs / 3,783,962,232 observed bytes before execution**.
Every selected executable has zero prior-checkout-root occurrences; mvm-build
is freshly compiled and names the current checkout root. All 284 hashes match
after the complete workspace command. No executable binary is copied locally
or distributed here. This repairs the disclosed stale-root first-run limitation.

The warm workspace test command records **9:07.08 wall / 261.29 s user / 107.87 s
system**. Its separate no-run code-generation command records **7:50.03 wall /
822.43 s user / 105.92 s system**. The complete qualification unit records
**21:56.328 wall / 28:48.432 aggregate CPU**, 6 GiB cgroup peak, no swap and a
two-CPU quota. These are qualification costs, not endpoint memory or energy.

SDK invocation `676668a1dac14dbe83fe1bd147352415` preserves all **486 SDK inputs**
and MVM inputs; 4:05.159 unit wall / 5:38.536 aggregate CPU / 3 GiB peak.
Its selected live executable SHA-256 is
`7ddf05e8cc1b852c8062395cb35813767bd6d7b7365a64ac98bcb2cc3f47575a`,
with post-execution identity retained. Addressed replacement invocation
`c005cad01a714216a2b277bb7889fedc` independently guards the same 486 source inputs
and executable pre/post identity. Original overwritten stdout remains a loss.

## Actual bytes and transfer costs

The registered host-process fixture verifies **5,804 application bytes each way**,
with **3,657 TLS bytes sent / 3,234 received**. Setup is **64.989665 ms**;
valid Open through verified half-close **15.913897 ms**; retirement including
archive retry **8.551530 ms**. Whole fixture: **0.09 s wall / 0.02 s user /
0.01 s system**. The selected executable plus host-agent, signer and endpoint
helpers have unchanged pre/post hashes. This positive uses `FixtureStoreCredit`;
it is not the joined original Root-ledger-to-child load witness.

The addressed-journal replacement uses a **257-byte body / 313-byte frame /
3,750-byte envelope**, with **5,090 sent / 1,014 received** on each admit/retry.
Admit is **20.850 ms wall / 0.903 ms sender-thread CPU / 1.649 ms process CPU**;
retry **20.689 / 0.906 / 1.395 ms**. Setup is **0.403 ms wall / 0.402 ms
sender-thread CPU**. Body and frame hashes and exact retained receipt are in
the archive. This is an addressed Unix-socket fixture, not booted MVM/WAN.

Seven native FlowMux rows each pass 89 component plus 20 ordered controls,
reconstruct exactly **67,108,864 application bytes each direction**, and
cross-match **5,395,050 / 5,395,015 TLS bytes** (10,790,065 total). The two-second
operation and 240-second bulk guards remain unchanged.

| Native component | Lifecycle seconds | Whole-process CPU seconds |
| --- | ---: | ---: |
| Linux x64 | 7.395001 | 10.848809 |
| Linux ARM64 | 6.818525 | 9.965009 |
| macOS Intel | 88.310905 | 57.591354 |
| macOS ARM64 | 34.042603 | 29.289390 |
| Windows 2022 x64 | 39.401831 | 62.359375 |
| Windows 2025 x64 | 34.624883 | 61.031250 |
| Windows 11 ARM64 | 118.463831 | 91.687500 |

[native-measurements.json](native-measurements.json) retains the full scalar
observations, raw compound rows, identities, setup clocks and unknown fields.
These fresh samples include slower rows than the previous packet; no causal
speedup, per-side CPU, physical energy or send-only time is inferred.

## Native evidence and reproduction

All **28 original ZIPs / 1,729,906 bytes** verify: **821 members, 644 embedded
checksums and 15,356 source-hash rows**. Three native installed-store consumers
each execute the original eight distinct nonzero controls plus the new worker
quote control. Both Linux canonical consumers execute the two SDK funding
controls. The original control-6 drop-order probe remains. Native executable
assurance is runner-recorded pre/post hashing; native binary bytes are absent.
A portable macOS quote control is not selected-process enforcement evidence.

```sh
python3 operator/verify-native-artifacts.py native /path/to/exact/SpiralCore operator/frozen-mvm-inputs.sha256
python3 operator/verify-wick-records.py operator/wick-records.zip operator/wick-records.json
```

The native verifier is read-only by default; its explicit `--write` option is
only for initial generation. The compact Wick archive retains 231 records /
36,192,703 uncompressed bytes in 5,107,133 ZIP bytes, with every member length
and SHA-256 indexed. It contains source guards, compiler JSON, selected binary
hashes, commands, stdout, clocks, metadata graphs and failures, not private
runtime homes, keys, source exports, executable binaries or payload stores.
See [failure journal](operator/failure-journal.md) for the valid RED, stale-root
run, Clippy failure, overwritten stdout, invalid footprint and preparation errors.

## Ari alignment and remaining delivery

The qualified canonical base remains #5557 / `7eb01574`. A bounded live refresh
observed main `870ac13d`, two commits ahead: #5558 changes installed-release
fixture expectations; #5560 changes demo preparation. They change four scripts,
not Rust launch ownership. They are not silently adopted into this frozen run.

Ari's newer [#5520 decision](https://github.com/tinylabscom/mvm/issues/5520#issuecomment-6900348999)
selects **CLI → mvm-client launch facade → mvm-hostd admitted executor → workload
runner**, with library/hostd migration first. The first migration is still under
validation/review; the completed refactor is not claimed landed. Preserve CLI
behavior pending its migration, typed source variants, local-only verification
evidence and one admission authority. [The bounded authority record](operator/ari-alignment.json)
retains those latest decisions and the observed main delta.

The joined original-budget-to-selected-child/store-load witness and executed
wrong-owner/broad-mode controls remain open under #258. Signed runtime
publication/boot, private MVMD and source-pin agreement remain gates. All five
#42 outcomes stay active: installed mailbox aggregate ownership/shutdown/recovery;
persistent final-ACK/retention/expiry and sustained large/control traffic;
FlowMux and separately authorized access; canonical two-host WAN with explicit
Linux/macOS/Windows role/version/architecture/backend acceptance; and supported
capacity/fairness/backpressure/recovery/tails/saturation plus the final upstream
package. No FIPS/CUI, host-blind, energy or complete-delivery claim is made.

No local Rust build target, VM disk, payload store or native executable was
created. Shared Cargo caches, user runtime/session data and pre-existing
`.serena/` are preserved. Owned temporary duplicates are removed only after
reviewed publication and verification.
