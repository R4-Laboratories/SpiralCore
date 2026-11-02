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

## Diagnostic decision

The 144-pair context experiment preserved all compressed frames and decoded bytes,
but the large opaque prepared pair improved only 0.37% by paired CPU ratio.
Do not ship context sharing as a solution to the unrelated-object CPU gap.
One further bounded diagnostic will retain full-capacity output allocations with
the identical compressor parameters and canonical input. Compare original frames,
retain balanced repeated timings, and freeze the helper before execution. No
production changes are authorized by this diagnostic result alone. If allocation
reuse is also ineffective, publish the negative results and leave the production
selector unchanged; repeating network tests of unchanged production would add no
new evidence about an optimization.

The second 144-pair dose (shared context plus retained output vectors) preserves
every compressed frame and decoded byte but improves large opaque paired CPU
only 0.53%; large generated/residual/fitting controls regress slightly. Reject
both production variants. Deliver the bounded Rust diagnostic, immutable
observations, independent code/security/arithmetic review, custody and report.
No production test expansion or new network campaign is warranted for unchanged
production. The remaining CPU gap is explicitly open, not declared solved.
