# Independent B1 pre-clone successor review

2026-11-04. Independent Code/Rust/Security delta review by `/root/closed_recovery_review`, who authored neither this fix nor the codec. Scope is the sole P2 condition from `b1-independent-review.md`; no broader audit, builds, source edits, new test executions, guests, agents or publication were performed.

**PASS — the identified P2 condition is resolved for this exact pure B1 codec successor.** No remaining blocker was found in this narrow change. Together with the preceding canonical/inverse/legacy source review, this accepts the pure B1 format/intent component. It accepts no B2 transaction, capacity/native domain, migration profile activation, actual stop/restart or customer delivery.

The first freeze and original independent report are preserved. Original report SHA remains `372d09867ce2d690e89068f73a7715f3ac22270666bd3068d0c326b1d014c18a`; its original successful tests and lint failure are not rewritten by this successor.

## Exact identity

Frozen directory: `/tmp/spiralcore-retained-catalog-20261104/stage-b/clone-successor-freeze/`.

| Artifact | SHA-256 |
|---|---|
| `successor.diff` | `2a5ef6d45fab1f88708f354c22a8b358b9b4be4679c0ede2ae9abdb65afeef51` |
| `source.tar.gz` | `7329c297acd8b481782a31193a9e26f35b031436ad8457904193520299aa777a` |
| `source.sha256.json` | `19c6d3298772905465814fe66ab3a03995f90ccde5a0fedd7f8a16502e1a832d` |
| `compiled-inputs.json` | `675cfe30639956bd8a3b1eac1d86c8d9f16108d829639cd32f5ce2ed3fb31035` |
| `author-response.md` | `c6bdf1db055b8bad2847e85f079eef3d4e7183ade3163698948bec15bd63a318` |
| Unchanged `contract.md` | `1e504e444d6554b587b4c653f2de413f1034b82b984c2866badd43aacbab6ea1` |

All five current mapped inputs and all five archive members match the frozen source map. All 138 current compiled-input entries match the frozen compiled closure. Comparing the old and successor 138-entry closures confirms exactly two changed files:

- `crates/spiralcore-mvm-live/src/installed_owner/retained_transfer.rs`: `2926d3b435845653894365c96cb7cbe053860d160b78931373b2dc2e7c4145ca`.
- `crates/spiralcore-mvm-live/src/installed_owner/retained_transfer_tests.rs`: `96086113231cc981ddfdbd4f0dee1f424ee16b3533a7800d119fa88989ba7e9c`.

The shared v1 parent, old golden and supervisor profile include are unchanged from the first freeze. Roster `5e31d33c344d39b0` was refreshed successfully. No manifest/lock/dependency or wire-format change is part of the delta.

## Source disposition

`CatalogV2::validate_shape` at successor recovery-transfer leaf lines 102-125 is nonallocating: it compares keys/generation/count, iterates borrowed rows and origins, calls the existing borrowed `record.valid()`, and checks duplicate tuples/session/path/state IDs through bounded borrowed iterators. `check_key` only compares fixed arrays. No Vec creation, path construction, clone, formatting or I/O occurs in that predicate. The closed CatalogMode enum and its existing decoder enforce supported modes.

The encoder invokes it at 127 before Vec allocation at 128. `reverse_source` invokes it at 201 before either actual reverse clone at 219/230. Thus an excessive row count or oversized path refuses before the requested clone boundary. The <=16 guard precedes row iteration and allocation-size arithmetic, preserving the bounded work and raw output limit. Origin/generation and duplicate rules remain the existing ones.

The actual successful encoding expressions, offsets, padding, magic, domains and hash operations are unchanged. `record.valid()` moved before allocation while `encode_base` retains its check. Successful canonical input bytes are therefore preserved. Separate source-version/mode/origin/restored-header checks remain. The v1 encoder/decoder and golden input are unchanged, and empty catalogs remain valid codec values while the intent separately refuses zero rows. No permission, new owner, receipt, quota or profile acceptance is added.

The test hook is cfg(test)-only immediately before each actual reverse cloning expression, backed by thread-local counters. Invalid values are constructed privately; they cannot create production authority. Both valid controls reach the expected branch's boundary, ruling out a hook that never runs. Hook counts establish ordering only; this is not allocator, capacity, RSS, native exploit or actual restart evidence.

## Actual causal records checked independently

Raw prefix: `/var/tmp/spiralcore-retained-catalog-20261104/validation/retained-v2-clone-{red,green}/`. I inspected records read-only over SSH and recomputed the actual executable hashes. I did not execute tests/compiler.

The RED overlay's actual test-source SHA is `96086113231cc981ddfdbd4f0dee1f424ee16b3533a7800d119fa88989ba7e9c`, matching the GREEN source map and frozen successor. These are the same test assertions across RED/GREEN.

- RED: release build 0; **3 passed / 4 failed**, test/batch 101. Each invalid branch/path or branch/count case fails on `invalid shape reached cloning before refusal`, reporting `[1,0]` or `[0,1]` against `[0,0]`. Both valid controls and empty compatibility pass. Actual RED executable SHA `ef107a5ad759532bd3ca9aa2c77619d2658d3e4c2df4c82f75c6a0114b8a35ef`.
- GREEN: release build 0; **17 passed / 0 failed / 0 ignored**, 162 filtered; test 0. This includes the four corrected ordering cases, two valid controls, empty compatibility and all original ten pure format/intent/golden regressions.
- Scoped release Clippy: `--lib --tests -- -D clippy::all`, strict/batch 0, error JSON count 0. There are **52 ordinary unused/dead-code diagnostic messages** and **2 ordinary test-build warnings**. This is not zero-warning, `-D warnings`, all-target or broad-library qualification.

Actual GREEN executable SHA `747b49246ec9d6b4aae2a6b3e3862661bd79ed528c9dd577f227d83b476eac5a`; after-test and after-Clippy readbacks match. The GREEN 138-entry source-before/after maps match frozen compiled-inputs SHA `675cfe30639956bd8a3b1eac1d86c8d9f16108d829639cd32f5ce2ed3fb31035`. Historical four-file fixture maps match unchanged SHA `3ac4ac70557be27b85dcafb4e07a73154a557a54c091972456a9278d1d17d81c`.

Selected GREEN evidence identities:

| File | SHA-256 |
|---|---|
| `test.stdout` | `cfc29f75b6be644368501f11e66726213d905a1c58694486fc65b9c9c61e9879` |
| `build.jsonl` | `1fb9d4511cf7023c156b418f29db78b6f77b7d8ebcd270b1c8572a458f39b7b5` |
| `strict.jsonl` | `85c8350016232ca000e33a3f58d5c6b2e778292291723de4404188f2e02e5197` |
| `strict-warnings.json` | `c59b2641ad2d950b3f23f7d5bf03e9e77f9223941455d5f95c1e5e501efa7ed5` |

## Unchanged open gates

The accepted result is the pure local codec/intent component. The provisional 930,152-byte requested-storage worksheet is still not a guarded constructor/capacity model or native allocator/kernel/TLS/RSS proof. B2 must implement and qualify actual original-custody, same-ledger transaction, finite crash recovery, owner Record/Storage-key transition, holders handoff and retained-stop lifecycle. It must compare intent digests with actual pinned source/beacon/object facts and compiled exact old-to-current compatibility. No new profile is accepted by this review.

Native phase-4 producer/EndpointRetirement evidence, useful admitted transfer after restart with old debt charged, two-guest failure/recovery, full matrix, capacity, platforms/WAN, regulatory qualification and upstream customer delivery remain open. No source or resource allowance was changed to imply these gates passed.
