# Implement the existing SpiralCore runtime as a shared transfer model

The user authorized implementation of ternary/Kuramoto/CA as the central mechanism now, with MVM integration afterward. Execute the accepted [design](../specs/2026-10-30-shared-runtime-design.md) continuously; routine implementation choices do not require another approval pause.

## 1. Preserve source and establish independent references

- Retain original embedded-kernel extraction hashes and the exact extraction procedure. Extract only the required Python definitions for an isolated reference fixture generator; do not execute received saved-card source.
- Record periodic CA truth tables and evolution; Random(184) initial phases; Kuramoto and memory-coupler vectors; observed folds through N=9 and larger carry boundaries. Label the unweighted folded value as a lossy summary.
- Add `crates/spiralcore-transfer/tests/shared_runtime.rs`; record a real failing test before introducing the module. Test ownership is separate from production engine ownership.

Risk: medium. Float parity is tolerance based across languages; exact integers and Rust wire bytes are separate acceptance claims.

## 2. Port the finite runtime engine

- Add `crates/spiralcore-transfer/src/shared_runtime.rs` and the module export. Implement the actual six rings, Python fold order, Kuramoto phase update, and phase-sum memory torque.
- Pin the approved pure-software math dependency in the crate manifest/lockfile. Implement canonical checkpoint and recipe encoding, checked decoding, bounded replay, final-state output, and full-state trace output.
- Test canonical round trips; exact same-checkpoint/same-recipe bytes; changed phase, CA, memory, and payload effects; split replay/checkpoint continuation; CA184 conservation; work/output/length limits; invalid floats, reserved bits, padding, and truncation.

Risk: high. Preserve transition order and the observed fold rather than replacing it with a more familiar algorithm. No math-provider change after canonical vectors are frozen without a profile/version decision.

## 3. Integrate exact residual transfer

- Root owns `src/shared_transfer.rs`, `src/client.rs`, `src/endpoint.rs`, `src/protocol.rs`, and any required `src/selection.rs` integration. Add mode 5 and a checked recipe setup body; reuse sparse residual application and the existing commit path.
- Seed checkpoints as ordinary verified objects. Authorize cache lookup, reserve aggregate replay resources before generation, compare predicted digest, reconstruct the exact target, then use existing finish verification and durable publication.
- Compare complete runtime-candidate protocol cost against existing configured candidates before Begin. Preserve deterministic ties and record all preparation cost, including losing runtime candidates.
- Add endpoint tests for exact generated trace, changed trace with residual, final checkpoint, different prediction/target lengths, missing checkpoint, wrong domain/capability/epoch, malformed recipe, excessive work, prediction mismatch, cancellation, failed final digest, and restart. Failed attempts must not expose a committed target.

Risk: high. Runtime output is a prediction, not authority; only existing namespace authorization and final object verification authorize publication.

## 4. Record actual performance without changing the question

- Build an explicit Rust harness. Retain source/configuration/input/output/executable hashes, command, raw observations, and independent summaries.
- Use matching raw, zstd, current adaptive, and runtime-enabled candidate selection on identical bytes. Include seed cost, exact generated state traces, localized changes, append/truncate, and unrelated controls.
- Measure deterministic generation and residual/selection work separately; measure sender/receiver process CPU and complete durable-commit wall time. Count both directions, all protocol setup and replies, and every repair/retry. Report body-byte scope separately from transport overhead.
- Begin with local actual endpoint transfers to qualify fidelity and measurement. Use bounded paired trials, disclose observed sizes and repetition counts, and retain negative outcomes. Real WAN qualification follows only after correctness and local instrumentation pass; do not relabel local or encoder-only results as WAN speedups.

Risk: medium. Shared generated traces are an intentionally favorable workload. The unrelated control and seeding cost constrain interpretation. Small wire recipes can still spend excessive CPU regenerating state.

## 5. Independent review and delivery

- Run required Rust check, Clippy, focused tests, complete relevant release targets, and object-store checks; avoid builds during timed measurements.
- Review runtime/source fidelity, parser/resource limits, cache authorization, commit ordering, exact cost accounting, and report claims independently. Resolve findings before publication.
- Preserve the evidence and update the claim ledger with implemented scope, exact tests, measured results, failed gates, and unqualified architectures. Keep the earlier rejected sparse-pruning candidate and its sealed evidence unchanged.

Completion means a working Rust shared-state transfer using the actual retained runtime mechanisms, exact reconstruction and durable commit, reproducible measurements, and an explicit list of remaining qualification work. MVM/vsock and orchestrator integration remain subsequent milestones.
