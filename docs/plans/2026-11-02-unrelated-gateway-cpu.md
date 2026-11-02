# Implement unrelated-object residual search optimization

Contract: docs/specs/2026-11-02-unrelated-gateway-cpu-design.md.
The user has authorized implementation and publishing; no further approval cycle.

1. Core owner: preserve existing candidate_profile baseline receipts and identities
   in docs/network/evidence/2026-11-02-unrelated-gateway-cpu. Add focused delta tests
   before changing crates/spiralcore-transfer/src/delta.rs, then implement exact
   window search. Medium risk: boundaries and canonical recipe preservation.
   Reuse frozen witnesses; retain RED/GREEN and scoped regressions. One Cargo
   window, Rust1.97.1 locked,2jobs,no incremental,existing sharedtarget.
2. Measurement owner: update tools/gateway-network/{runtime-object-netns.sh,
   runtime-object-infra-check.sh,analyze-runtime-objects.rs} only for explicit new
   schema→baseline pin and shared paired policy. Update existing candidate-CPU
   workflow statically. New registration/method in current evidence directory.
   Low risk: no new transport/fixtures. Wrong-pin negatives and historical
   byte-identical analysis checks guard policy changes.
3. Independent reviewer checks exact run-search equivalence/error bounds and
   frozen frame witnesses; parent reviews infrastructure. Final targeted transfer
   regressions plus gateway/core tests and strict Clippy. No coverage campaign.
4. Freeze source; run one registered same-run before/after/zstd campaign after
   final checks. Root verifies original artifacts/source/binaries, reproduces
   analysis and obtains narrow independent arithmetic review. Keep all trials.
5. Archive originals with fresh immutable verification; publish report, readable
   analysis and compact receipt bundle. Merge implementation/results through PRs,
   sync both checkouts to origin/main and preserve unrelated user/lab files.
