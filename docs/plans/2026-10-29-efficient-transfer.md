# Implementing computationally efficient exact transfers

User approved the measured efficiency plan in `docs/network/compute-efficiency-2026-10-29.md`.
Baseline: `354b616257eda62cd6bdcc2fb37902fc4cb4a5e8`; hardware SHA features remain unchanged.
Reuse the attached isolated worktree on `codex/spiralcore-efficient`; preserve the WAN branch and evidence.

## Contract and acceptance

Exact bytes, original length, scoped authorization, bounded resources and durable publication remain mandatory.
Existing v1 raw/zstd/DAG wire identities and immutable fixtures must remain compatible. Any sparse recipe gets an explicit version and fail-closed mode discriminator. Energy savings require joule measurements; CPU reductions do not establish them.

Performance hypotheses: direct graph construction should at least halve CPU on 1 MiB random fixtures; retained/incremental warm work should at least halve endpoint CPU against the measured implementation without increasing mutation body bytes; sparse recipes should avoid prefix-insertion expansion. These are targets, not achieved claims. Preserve failures and compare complete setup plus amortized use.

## Sequence

1. **Direct canonical builder** — `crates/spiralcore-transfer/src/dag.rs`, `tests/direct_dag.rs`.
   Replace legacy fold/rebuild/convert with direct SHA-256 ternary construction. Keep the legacy path as a test oracle. Verify exact manifests/node encodings, padding and unique-node/work limits over random, structured and boundary fixtures. Risk: medium; no dependencies.
2. **Verified prepared state** — `src/dag.rs` or a dedicated `src/prepared.rs`, `tests/prepared.rs`.
   Introduce encapsulated immutable prepared graphs, bounded accounting, shared tree structure and incremental changed-range/byte comparison. Preserve canonical v1 identity, output length and full-byte verification. Test poisoned public graphs, exact repeats, same-length edits, fallback on length changes, and resource bounds. Risk: high; depends on 1.
3. **Receiver graph retention** — `src/endpoint.rs`, `tests/endpoint.rs` plus focused cache tests.
   Retain scoped verified base graphs under the resident-memory budget. Pin references in active attempts and use staged overlays; evict only unpinned entries. Recheck disk publication integrity/authority and preserve restart behavior. Tests cover reuse, eviction, corruption, wrong namespace/epoch, cancellation and aggregate memory admission. Risk: high; depends on 2.
4. **Prepared sender and repeat path** — `src/client.rs`, CLI/benchmark callers and focused sender tests.
   Use retained immutable state for target preparation and record selection. Known identical targets use raw/CAS without first folding a graph. A descriptor remains immutable after Begin; misses use explicit safe fallback. Test request counts, byte accounting, repair scope, and exact reconstruction. Risk: medium; depends on 2–3.
5. **Versioned sparse recipe** — protocol decision in `docs/network/`, `src/delta.rs`, `src/protocol.rs`, `src/client.rs`, `src/endpoint.rs`, tests.
   Reviewed binary contract: `docs/network/sparse-v1.md` (architecture/security review before code).
   Bounded COPY-from-verified-base plus LITERAL operations, with explicit object identity, lengths, codec version and op/work/frame limits. Compare aligned-run and prefix/suffix recipes; choose raw fallback when a recipe is not smaller. Check every offset/length and final identity before publication. Cover empty/NUL, append/truncate, prefix insertion/deletion, scattered edits, missing/corrupt bases, malformed recipes, cancellation and restart. Risk: high; depends on 3–4.
6. **Measurements and delivery** — benchmark tools, WAN workflow, reports and receipts.
   Run balanced old/new release diagnostics; include preparation, retained memory and cold seeding. Repeat real WAN transfers using the existing temporary free-runner/relay method where feasible, with 30 paired trials and exact reopened-cache comparison. Keep packet-accounting limitations explicit. Compare conventional delta behavior where available. Run full tests, clippy/fmt/build and coverage, independent code/security review; commit and publish the branch. Remove only owned diagnostic debris, processes and temporary secrets.

Each phase uses RED→GREEN tests, followed by spec and Rust/security review. Implementation is sequential to avoid overlapping file ownership. Diagnostic artifacts and exact source identities remain separate from production qualification.

## Progress

- Baseline full transfer suite: 77 tests passed in this turn.
- Baseline executable hashes retained in `local/efficiency-20261029/baseline.json` in the primary checkout.
- Phases 1–6: in progress; results will be recorded as implemented and measured.
