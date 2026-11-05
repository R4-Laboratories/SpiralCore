# B1 pre-clone ordering successor — author handoff

2026-11-04. AUTHOR response to independent P2, not an independent acceptance. First freeze502620604f55099765737b73ec35db8b4e95fc0e152f7bae749e0995e86b4ed2 and independent review372d098 remain preserved. Source is now frozen for OTHER independent delta review; no active Cargo or new work is queued.

Only two files differ from that freeze: retained_transfer.rs and retained_transfer_tests.rs. `successor.diff` is the exact complete delta. One private nonallocating validate_shape predicate checks key/generation, <=16 rows, record.valid, origins and the existing duplicate origin/session/path/state-ID rules. The encoder calls it before Vec allocation; reverse_source calls it before either cloning branch. Closed CatalogMode type/decode tags preserve supported modes. Separate source-version/mode/origin/restored-header checks remain. No byte layout, domain, quota, empty-catalog semantics, I/O, issuer, key/state mutation or B2 activation changed.

The cfg(test)-only thread-local hook is immediately before each actual reverse cloning expression. Four real ordering REDs (two branches × excess rows/path) fail against the preceding source, observing [1,0] or [0,1] instead of [0,0]. Both valid controls reach their hook; an empty-catalog compatibility case passes. This is ordering evidence, not an allocator/RSS measurement or native exploit. The successful canonical bytes and independent old golden are covered by the original ten tests.

Actual records under /var/tmp/spiralcore-retained-catalog-20261104/validation/:

- retained-v2-clone-red: release build0; 3PASS/4FAIL; test101 at exact invalid-shape-reached-cloning assertions. Source, protected object and failed output remain separate.
- retained-v2-clone-green: release build0 (26.11s); all17 selected testsPASS/0ignored/162filtered (0.00s); scoped release Clippy --lib--tests -Dclippy::all0 (3.98s). Compiler JSON:2 ordinary build warnings and52 ordinary scoped-Clippy unused/dead-code messages for unconnected production paths. No masks, zero-warning, all-target or new broad-library claim.

All138 compiled source/manifest/lock/golden inputs and protected executable readbacks match before/after. Protected successor SHA747b49246ec9d6b4aae2a6b3e3862661bd79ed528c9dd577f227d83b476eac5a. SDK9a/adapter9153/sharedCA graph unchanged. Every file in source.sha256.json matches compiled-inputs.json. Contract1e504e44 unchanged; provisional memory worksheet and constructor/native-domain gates remain as previously recorded.

This closes the tested author ordering defect only. Independent successor acceptance, B2 transaction/ledger/live integration, retained-stop lifecycle, actual native phase4 producer/receipt, useful service restart, full matrix/capacity/platform/WAN and compliance qualification remain open. Session's lane was returned at actual terminal before this handoff.
