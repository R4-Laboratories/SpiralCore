# Ordinary V1 publication cannot erase V2 history — narrow checkpoint

Base: independently reviewed B2 c0a10d50. This is a prerequisite to live V2 inventory/writers, not their implementation and not useful/native restart acceptance.

The actual existing private normal V1 publisher accepted a Catalog at the current migrated owner epoch, overwrote SCRETN02 as SCRETN01, and erased immutable origin tuples. The focused real-filesystem case failed101 after build0; its existing valid closed-commit control passed. No new scope, grant, receipt or metadata charge was fabricated: the probe republished only the actually held original records under the same original owner/closed funding custody.

The eight-line production correction invokes the existing bounded canonical V1 reader before a legacy publication when a canonical file already exists. Migrated V2/unknown formats refuse before any catalog intent/write. There is no downgrade/fallback or competing codec. V2-aware ordinary new-row/update paths remain required. The existing metadata/native allocation model remains provisional; this bounded extra legacy read is not a new capacity/RSS claim.

Exact GREEN: release build0, 14 selected B2 cases PASS, fixed ABI test 1 PASS, scoped release --lib --tests -Dclippy::all0. Build JSON retains 2 ordinary dead_code messages; strict JSON 75 ordinary messages (74 dead_code + 1 unused_imports). No default full-suite rerun or zero-warning claim in this short batch. All 139 compiled input and protected executable before/after hashes match; dependency locks unchanged in this batch. Protected binary SHA256 5a78919de6845dda21793d92b6dfd3cbd58efb80a4dde3543bcc34cd4f51a232 remains on Wick at validation/retained-v2-b2-history-downgrade-green/protected-test.

source.tar.gz SHA256 24195e4bc1395009a2f0daa55a948077bb1c7204ccf66f62506bb721db0f345f contains three exact source/test files; delta.diff versus c0a10d50 SHA256 ad7692a0be70ee250f1c788d5fe07424dec54d27702db3da2c70e1ff59718ca0. Only retained_telemetry.rs changes production behavior; other changes are the regression and test-only helper visibility/probe.

receipts.tar.gz SHA256 0dee6b72388f8ab9ff709f58e869ca3975216028198a5fb2fc49d987acde6bd0, 179,316 bytes, 54 hashed receipts plus index = 55 members. Raw RED/GREEN and protected executables are preserved on Wick. Archive and all member hashes were verified locally.

Separate uncompiled preparation: retained_live_tests.rs contains original 1/16-row physical table/no-duplicate-metadata and held-membership corruption tests for the next prepare_live_inventory API RED. It is deliberately NOT in this qualified source/compiled139 list. No live-table, activation, holder conversion, stop crash-recovery, native guest, WAN/scale or regulatory proof follows from this guard.

Lane 2 returned at terminal; no Cargo or Git mutation. Independent review is required before accepting this production delta. Existing immutable B2 freeze, HOLD/successor review and earlier failures are untouched.
