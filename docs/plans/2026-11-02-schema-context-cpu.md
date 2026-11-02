# Execute schema-context CPU investigation and optimization

Contract: docs/specs/2026-11-02-schema-context-cpu-design.md.
User authorization covers implementation, measurement and publishing; no new
permission ceremony. The exact byte-selection objective remains authoritative.

1. Core owner adds a bounded Rust diagnostic in the transfer crate, using identical
   new_encoder parameters and public canonical fixture inputs. Freeze provenance,
   run separate/shared-context comparison, verify full compressed bytes and exact
   reconstruction. Medium risk: benchmark state/order must match the hypothesis.
2. Root chooses the measured path and records exact production files/steps here
   before edits. Core owner uses focused RED/GREEN and frozen original-encoder
   comparisons; independent reviewer checks state/reset/resource/byte boundaries.
3. Measurement owner minimally reuses existing paired workflow with explicit new
   schema and pinned baseline, preserving historical outputs. No transport or
   fixture changes without a demonstrated need. One Cargo window at a time.
4. Final gateway/core tests and strict Clippy; one frozen paired campaign; source,
   artifact and executable verification; narrow independent arithmetic review.
5. Report all results and limits, archive full originals with fresh verification,
   merge reviewed implementation/results PRs and sync both local checkouts.

Initial scope is the diagnostic; no production optimization has yet been selected.
