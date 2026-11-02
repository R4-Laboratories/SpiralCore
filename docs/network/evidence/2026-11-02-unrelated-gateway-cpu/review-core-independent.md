# Independent aligned-run scanner review

Reviewer: `mvm_tests`, independent of the implementation and tests.
Date: 2026-11-02. Verdict: **PASS; no material finding.**
Baseline: `4a8ec79c9b185c85407e9767653dbcc4af97bf7e`.
Scope: the `delta.rs` change, its new scalar-run oracle, and frozen encoder
fixture. No builds, tests, production edits, or broader audit were performed by
this reviewer.

| Reviewed file | SHA-256 |
|---|---|
| `crates/spiralcore-transfer/src/delta.rs` | `c0c1cad51c3217169d98ec875f99c6fd135e4244b24f929ecbfd0a90c63e2e3b` |
| `crates/spiralcore-transfer/tests/fixtures/unrelated_cpu_checks.rs` | `a6caabb93ddc13e7f9ca7e7b8c854ed94303aeef186671f27d8d61dc35c37dae` |
| `crates/spiralcore-transfer/tests/fixtures/unrelated_delta_before.rs` | `22c179661648dd846531b8726f76f26471044afd015605d1083745b2f4a7ccc7` |

## Proof and implementation

The search invariant is valid: an anchor lies at the 32nd position of each
unexamined interval, then advances by 32 after a known mismatch. Every eligible
32-byte equal run must contain such an anchor. At an equal anchor, the previous
mismatching anchor or restart boundary proves the run cannot begin more than
31 bytes earlier. Backward expansion to `max(lower, anchor-31)` and exact forward
expansion therefore recover the same first maximal eligible run as scalar search.

When an equal run is shorter than 32 bytes, its forward end is either EOF or a
known mismatch. Restarting at `end+1+31` cannot skip a later eligible run. A final
tail shorter than 32 bytes cannot introduce a qualifying run after that known
mismatch. This covers the important 31-equal / one-mismatch / 32-equal boundary.
All comparisons are exact; there is no hash, entropy or probabilistic exclusion.

Indexing is bounded by the shorter input. Anchor additions are checked; backward
subtraction follows an anchor initialized at least 31; forward endpoints remain
within the compared slices. The caller retains the existing object-size limit.
Backward work is at most 31 bytes per sampled equality, and restarts advance, so
the change introduces neither unsafe memory access nor an unbounded search loop.

The caller preserves the existing prefix reuse and the unchanged prefix/suffix
candidate, literal boundaries, `aligned.len() <= MAX_OPS` stopping condition,
packing, strict cost comparison and Raw fallback. Only previously discarded
short equal runs stop receiving individual visits. Removing the old unequal-byte
scanner and its direct tests is appropriate because its replacement has a scalar
oracle for the new required result, rather than assuming equivalent internals.
No decoder, identity, authorization, publication, ACK, or codec setting changes.

## Independent witness and observed evidence

The reviewer compared the pinned Git source with the new frozen fixture: its
entire **8,963-byte production suffix beginning at `pub const MAX_OPS` is
byte-identical**, and the original import is retained. Only provenance comments
and original unit-test exclusion differ. Thus the full-frame oracle does not
reuse the new anchor algorithm.

The retained RED is accurately a missing `next_copy_run` API failure (exit 101).
The focused GREEN receipt records **two passing tests**, covering scalar search
across offsets and 31/32/33 boundaries, short/dense runs, unequal lengths, EOF,
complete packed-frame and operation-count equality, exact reconstruction, chunk
boundaries, append/truncate, and operation-cap edges at 4,095/4,096/4,097/8,193
runs. The independent oracle compares actual frames, not only lengths or hashes.

The scoped regression receipt also exited zero: 39 library tests and 5/9/10/9
integration tests passed, with one existing ignored test. This includes the
existing Combined trust-state negatives and older frozen encoder controls.
Finite tests support the reviewed invariant; they are not formal verification of
every input. Search CPU and complete-transfer performance remain measured claims
for the separate matched diagnostics/campaign, including any remaining losses.
