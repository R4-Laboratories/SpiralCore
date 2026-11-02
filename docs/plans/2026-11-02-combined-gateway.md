# Execute the combined runtime/cache gateway

Approved objective and contract: `docs/specs/2026-11-02-combined-gateway-design.md`.
Reuse the isolated worktree; preserve .serena and the primary Python/saved work.

1. **Core selector/session** — transfer `session.rs`, `schema_selection.rs`,
   narrowly needed `client.rs` glue and focused new tests. Core owner implements
   RED→GREEN, mode-bound IDs, cheap repeats, acknowledgment promotion, scoped
   invalidation and bounded single-session retention. Medium correctness risk;
   independent security/Rust review covers the changed boundary.
2. **Gateway facade/config** — gateway-core `src/object.rs`, gateway
   `src/object_service.rs`, focused `tests/object_combined.rs` and service tests.
   Root implements only the new profile/receipt dispatch after failing API and
   real-QUIC tests exist. Preserve all existing transport/publication paths.
3. **Matched measurement** — `examples/object_fixtures.rs`, existing bounded
   runtime harness/analyzer with explicit campaign policy, a new combined workflow
   and registration/evidence under `2026-11-02-combined-gateway`. Infrastructure
   owner verifies unchanged fixture hashes, dose gates and historical compatibility.
   Static tests and independent infrastructure review precede one frozen run.
4. **Verify and measure** — focused transfer tests, full gateway/core targets,
   strict Clippy, then frozen Linux campaign. Single Cargo build window; no local
   performance run concurrent with compilation. Fix concrete failures and retain
   their receipts. No coverage expansion unrelated to this mechanism.
5. **Deliver** — reproduce arithmetic; independently inspect source/artifact and
   numeric joins; archive full ZIP in existing private HF dataset and verify a
   fresh immutable download before removing owned working copies. Record report,
   ledger and raw results, merge reviewed source and results via PRs and bring
   both checkouts to origin/main. No hidden losing cases or broad cleanup.

Acceptance: real Runtime/Sparse/cheap-repeat behavior within one session, no raw
forced first target, exact reopened bytes after all sends, negative missing-state/
failed-ACK/namespace/capacity tests, measured setup and full-session comparison
against prepared zstd and both separate profiles. A performance win is measured,
not assumed. Final scope remains cooperating object service, not automatic TUN
flow transformation.
