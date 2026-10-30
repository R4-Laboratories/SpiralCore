# Warm-object selection implementation plan

The user authorized this integration. Follow the
[accepted design](../specs/2026-10-30-warm-selection-design.md) after its parent
commit; no additional approval loop is needed for this bounded work.

1. **Selector owner — medium risk:** change only
   `crates/spiralcore-transfer/src/schema_selection.rs` and add
   `tests/warm_selection.rs`. Retain meaningful RED evidence, then implement
   additive warm preparation and distinct exact/pruned receipts. Reuse existing
   delta encoding and zstd structural bound. Preserve old `prepare` behavior;
   no `zstd_whole` change, capped compression, base cache or receiver change.
   Prove mode/frame/cost equality against exhaustive unchanged codecs, including
   all four winners, ties, cold/schema/base states and boundary inputs.
2. **Parent — medium risk:** add the consuming client function and focused
   integration tests. Plans carry fully hashed selected Offers and owned bodies.
   Test actual serialized costs, explicit optional schema seed, missing/revoked
   state, errors without reoffer, and exact reopened bytes. Existing wire formats,
   decoding, authorization, durability and quotas remain unchanged.
3. **Measurement owner — low/medium risk:** adapt existing warm-object harness
   and summaries to three arms and six five-trial shards. Keep historical
   commands/results unchanged. Record exact/pruned decisions, actual schema
   fitting/context construction and all base seeds. Qualify the registered
   36-transfer local run, then freeze source/executables/plans for the single
   360-transfer WAN comparison. Use paired order and all three arms on identical
   inputs; no performance-driven changes to fixtures, dose or codec settings.
4. **Parent and independent reviewers — bounded gate:** coordinate one focused
   and final Rust qualification plus a lean cross-author source/security review.
   No tests/builds during timed campaigns. Stop receivers and exactly verify
   every actual seed/update before publishing byte, CPU and durable-time results.
   Distinguish improvement over Whole from selector overhead versus forced
   Sparse. Preserve raw failures/negative results and actual setup costs.

Success requires unchanged eligible byte-minimum decisions and exact output,
truthful pruning evidence, explicit base/schema readiness, and recorded local/WAN
costs. Faster timing remains a measured outcome, not permission to weaken the
minimum. Reuse existing Rust1.97.1 build targets with incremental-off, the4GiB
disk floor and existing receiver limits; no new audit or measurement framework.
