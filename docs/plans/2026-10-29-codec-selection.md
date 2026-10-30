# Implementation plan: pre-Begin codec selection

Starting revision: `eb4feb930d6910466dbc3dc90bfa3cead51f6b5e`.
Design: [codec-selection decision](../network/codec-selection-decision.md).
Status at design writing: implementation was in progress; new performance
measurements NOT_RUN. The executable/source and exact measurement schedule are
frozen separately before collection; this design is not a preregistered result.
The user authorized the next standalone optimization step; MVM and WASM remain
outside this implementation. Keep historical campaign records unchanged.

## Requirements

- Minimize projected successful-miss request plus reply HTTP-body bytes over the
  existing bounded Raw, chunked-zstd, sparse and zstd-prefix candidates.
- Preserve exact bytes/length, identities, authorization, limits and publication.
- Select before Begin, retain encoded winning bodies, and never change an offer
  after a receiver error. Keep explicit-codec APIs and historical fixtures stable.
- Expose selection decisions and record CPU, elapsed, byte and seed costs without
  claiming an unmeasured WAN/energy improvement.

## Phase 1: sender API and deterministic selection

1. **Behavioral RED** (`crates/spiralcore-transfer/tests/adaptive.rs`). Add a failing
   cold-compressible sender test that requires exact output and fewer body bytes
   than Raw, with no caller-selected codec. Run and retain the expected failure.
   Dependency: none. Risk: low.
2. **Selection data** (`crates/spiralcore-transfer/src/selection.rs`,
   `src/lib.rs`). Define bounded candidate metadata, decision reason and private
   prepared payload. Implement checked full body-cost accounting, including
   SCWREP01's 28-byte header. Add focused ties/boundary tests before each behavior.
   Dependency: step 1. Risk: medium.
3. **Cold candidates** (`src/selection.rs`). Evaluate Raw and the existing chunked
   zstd settings; retain only the winning encoded representation. Test Raw wins
   ties and incompressible data, while structured data selects zstd. Dependency:
   step 2. Risk: medium.
4. **Warm candidates** (`src/selection.rs`). Add existing sparse and prefix
   candidates using optional local base bytes, strict byte-cost order, and an
   explicit exact-repeat probe decision if implemented. Validate both input
   limits before work. Dependency: step 3. Risk: medium.
5. **Send prepared winner** (`src/client.rs`). Add the adaptive entry point and
   generic encoded-body payload branch reusing existing batch reply validation.
   Include preparation in elapsed time. Keep original explicit APIs' behavior
   unchanged. Dependency: step 4. Risk: high.

## Phase 2: correctness, trust and transport qualification

6. **Exact matrix** (`tests/adaptive.rs`). Add empty/one-byte/NUL/trailing-zero,
   chunk-boundary, append/truncate, prefix/middle insertion/deletion, localized,
   scattered, unrelated, structured and opaque-compressed cases. Independently
   run every eligible explicit codec, verify outputs, and compare the adaptive
   choice with the minimum actual successful-miss body cost. Include a fixture
   where each of Raw, Zstd, SparseV1 and ZstdPrefixV1 wins. Dependency: phase 1.
   Risk: medium.
7. **Failures** (`tests/adaptive.rs`). Test oversized target/base with zero
   exchange calls; missing and wrong-scope bases; authorization/epoch revocation;
   interrupted and malformed replies; exact-repeat hit/probe-miss distinctions;
   no reoffer on error. Reopen successful publications and inspect failed cases
   under the existing uncertain-commit contract. Dependency: step 6. Risk: high.
8. **HTTP accounting** (`tests/adaptive.rs`, existing WAN test helpers as needed).
   Use the real loopback HTTP client/server path to prove projected versus actual
   successful-miss request/reply body counts, both probe policies, 16 MiB bounds,
   and full uploaded-body accounting on a target hit. Dependency: step 7.
   Risk: high.
9. **Review and verification** (existing Cargo targets). Independent Rust/code
   and security reviews, full ordinary tests, explicitly run ignored large tests,
   doctest, formatting, Clippy and release build. Verify old wire/benchmark vector
   behavior survives. Bind command outcomes and source/binary hashes to the
   evidence packet. Dependency: step 8. Risk: medium.

## Phase 3: recorded local tradeoff experiment

10. **Measurement driver** (`crates/spiralcore-transfer/examples/codec_selection.rs`).
    Add a native Rust driver using the same endpoint and publication policy for
    explicit Raw, chunked zstd, sparse, prefix and adaptive senders. Keep fixture
    construction outside the timer; retain every arm's identity and mode. Time
    through verified durable Published, count both actual body directions, and
    measure sender and receiver CPU scopes without double counting overlapping
    process intervals. Test output schema/counts before timing. Dependency: phase
    2. Risk: medium.
11. **Freeze registration** (`docs/network/evidence/2026-10-29-codec-selection/`).
    Before timing, record exact source/binary/compiler/features, fixture seeds,
    workloads/sizes, deterministic paired order, sample count, cache policy,
    timing scopes, excluded work and cleanup. No builds or other benchmarks
    concurrent with timing. A 30-pair local comparison is preferred for stable
    distribution reporting; any smaller run must be labeled diagnostic rather
    than acceptance. Dependency: step 10. Risk: medium.
12. **Run and retain** (same evidence directory). Include cold random and cold
    structured controls plus warm localized and unrelated-structured replacement
    workloads at 1 and 16 MiB sizes. Scattered and insertion cases remain in the
    correctness suite; they are not new timed acceptance strata. Every baseline gets the same inputs,
    fresh target-cache scope and publication durability; separately Raw-seed any
    reusable base. Record actual lengths for boundary-preserving edits. Exact
    reopen verification occurs outside timing and before owned synthetic cache
    cleanup. Retain failures and no trial replacement. Dependency: step 11.
    Risk: medium.
13. **Report** (`docs/network/codec-selection-results-2026-10-29.md`,
    `docs/network/optimization-ledger.md`, `docs/network/delivery.md`). Report
    medians/p95, per-pair observations, selected mode/cost, minimum configured
    baseline comparison, all compute/elapsed penalties and seed cost. Primary
    feature gate is exact correctness and minimum admissible projected body
    cost, with actual-miss agreement. Do not invent a CPU speedup gate for an
    exhaustive selector. Source or measurement failures remain explicit.
    Dependency: step 12 and independent evidence review. Risk: medium.

The measurement registration may choose a smaller explicitly labeled diagnostic
matrix before the full comparison. Its scope must be frozen before collection;
do not promote a subset into the predeclared full matrix after observing results.
No new real-WAN claim follows from this local phase. A WAN comparison needs a
separate frozen plan, fixed sources and the existing strict capture/custody gates.

## Success and rollback

Feature acceptance requires exact reconstruction and deterministic body-cost
selection under documented successful-miss assumptions, unchanged trust and
resource boundaries, retained timing evidence and independent review. Legacy
senders remain available; the new selector is explicitly opt-in. Rollback removes
the additive adaptive API/caller without altering stored bytes or wire formats.

Do not combine candidate pruning, a 64 MiB client, new codec parameters, a new
transport, or MVM integration with this milestone. Those require their own
reviewable implementation and measurement boundaries.
