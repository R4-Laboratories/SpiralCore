# Current MVM foundation qualification

This packet qualifies contributor MVM
[`5819ec8dc2d2ed897cc4716aa242131acef9f5c5`](https://github.com/Casey-allard/mvm/commit/5819ec8dc2d2ed897cc4716aa242131acef9f5c5),
tree `6701d61a366e11d23aadea42a1d0b474ea577bc1`, on canonical
`be1a62d2d4d32adb3af7f75a58703dbc915bf6f9`. It reconciles the existing contribution
with the canonical admission/runner splits, shared crypto leaf, build changes
and explicit BDD layout. It provides a reviewable source pin for the compatible
runtime handoff under [MVM #5057](https://github.com/tinylabscom/mvm/issues/5057).

SpiralCore's selected consumer remains MVM `b2def5fd` from merged
[#305](https://github.com/R4-Laboratories/SpiralCore/pull/305). This evidence
publication does not change that selection. Both direct MVM dependencies and
all five SpiralCore packages in the qualified foundation retain reviewed core
`8994951b7c13fe5acf6f5ea3060ad4e51dc473c2`.

## Executed scope

| Gate | Observed result |
| --- | --- |
| Source and complete file set | All 4,746 immutable Git inputs match before and after |
| Formatting, workspace strict Clippy, workspace all-target check | Pass |
| Explicit `mvm-conformance --all-targets --features bdd` check | Compiles; Cucumber scenarios NOT_RUN |
| Canonical `xtask check-all` | 86 clean gates |
| Default-feature workspace binaries | 21,824 passed, zero failed, 69 ignored |
| Rustdoc cases | 27 passed, zero failed, eight ignored |
| Test executable custody | All 285 compiler-selected ELFs recompiled; actual 285 Running paths match exactly; all post-run hashes match |
| Earlier focused current-source gates | 146 admission and 162 runner tests pass, with strict scoped Clippy |

[Test counts](test-counts.json) count only each binary's outer summary, excluding
19 nested subprocess summaries. The 28 rustdoc sections contain 30 disjoint
batches because `mvm_client` and `mvm_core` each emit merged-doctest and
compile-fail batches. The two new crypto targets replace no behavioral test:
the old `world` helper ran zero tests and moved into explicit BDD support;
`mvm_crypto` and `aead_trait` produce the net 284-to-285 target change. Metadata,
the expected set, compiler JSON, ELF digests and actual runtime paths are retained.

The source-footprint command passes in the canonical **measurement-only** mode
introduced by MVM #5573. Its ratchet is suspended; this packet does not call it
active enforcement. Recorded totals are 3,023 Rust files, 1,051,016 source lines
and 1,323,330 physical lines. Source counts are not runtime resource grants.

The workspace test command records **503.80 s wall, 182.09 s user CPU,
90.48 s system CPU and 4,323,192 KiB maximum RSS**. These cover the Cargo/test
command and waited descendants, not isolated sender/receiver, guest or VM
measurements. Compilation and checking have separate timing records. The unit
requests and records 6 GiB memory, zero swap, 200% CPU quota, 512 tasks, 8 MiB
stack and 4,096 descriptors, with two Cargo jobs. No network performance or
energy result is inferred from this qualification.

## Retained failures and review

The archive retains the first three failed scoped campaigns, including moved
imports, the original public rollback facade and measured closure changes.
The fourth campaign's legacy namespace test failed under the controller's
inherited `0002` umask. A controlled pair ran that exact ELF and test with
`0002` (refusal) and `0022` (pass). Restoring the established `0022` qualification
profile required no product, assertion, security or deadline change. Initial
pair setup failures are also retained. The successful fifth scoped campaign
distinguishes verified same-source hostd artifact reuse from a rebuilt runner;
the later full campaign recompiles all 285 test ELFs.

Focused independent source review examined the relocated admission/runner
contribution, public facade, helper ownership and call paths. Independent
artifact review compared all 4,746 manifest entries directly with immutable
Git blobs, derived the 285-target set from metadata, checked executable/runtime
identity, counts, source guards and measurement scope. Both reviews are advisory;
they do not establish acceptance beyond the executed boundaries above.

## Runtime handoff and remaining delivery

The contribution follows Ari's
[CLI → mvm-client → hostd admitted executor → runner direction](https://github.com/tinylabscom/mvm/issues/5520#issuecomment-6901815825),
including opaque one-use admission, admission before warm selection, retained
audit/OCI provenance and Required-tier sync overlap. The networking lane does
not take over the separate CLI/client migration.

Ari still owns compatible signed runtime publication. The producer handoff must
bind the selected architecture, source/artifact digests and authenticated
assembly declaration, including the required `mailbox_reader_v1` profile and
its DirectedJournalV1, ObjectPairV1 and SenderOrdinalV1 features. A release tag
or signed host-helper snapshot alone does not prove guest-reader compatibility,
signature verification or successful boot. Private MVMD integration and
maintainer source-pin agreement remain external gates.

Later canonical main `4047e48` and new real-UID/mode controls are outside this
frozen result. Those controls are undergoing separate qualification. No current
SDK/native consumer, guest boot, WAN, default-profile fit, allocator/concurrency,
capacity, FIPS/CUI or malicious-host guarantee is established here. All five
[SpiralCore #42](https://github.com/R4-Laboratories/SpiralCore/issues/42) outcomes
remain required; [#258](https://github.com/R4-Laboratories/SpiralCore/issues/258)
remains open. No final upstream networking contribution is submitted.

## Compact records

[records.zip](records.zip) contains 284 retained records, 38,926,204 uncompressed
bytes and 7,330,327 archive bytes, SHA-256
`0f25cee6a7dab2bbd7e59c16b3585d4c3ca678af1f3e44fd20b8f2276d0e8edf`.
[record-index.json](record-index.json) binds every member's byte length and
digest. It includes source manifests, compiler/metadata output, before/after
executable digests, command logs, guards and failed campaigns. It contains no
executables, payload stores or private runtime homes.

[run-foundation.sh](run-foundation.sh) and [run-scoped.sh](run-scoped.sh) retain
the exact controllers. They name the original host paths and depend on the
recorded frozen inputs and qualification helpers in the archive; they are not
a claim that another machine has reproduced the campaign. Run `python3
verify.py` from this directory to verify the packet without extraction or builds.
