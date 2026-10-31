# SC-OPT-019 implementation plan

Follow the [bounded design](../specs/2026-10-31-unrelated-cpu-design.md). The user
has authorized this direction; production begins only after the baseline stage
profile, committed design and parent release. Reuse the existing build targets,
agents, fixtures, session harness and evidence conventions.

1. **Freeze and profile — parent/measurement owner; low risk.** Preserve baseline
   `56e15db5c77fe2ecdc03476d8e09664cd4213fd1` and extend the existing
   `crates/spiralcore-transfer/examples/support/persistent_session.rs` diagnostic
   dispatch only as needed for independent stages. Record delta, schema, Whole,
   schema-zstd, hashing and complete preparation before production edits: three
   fixed seeds x two sizes x two input families x six stages = 72 observations.
   Keep record-count metadata and all observations under
   `docs/network/evidence/2026-10-31-unrelated-cpu/`. Select the narrow candidate
   only if the measured work is material. Dependency: design commit. Do not
   optimize fixture generation or sum independent stage clocks.
2. **Prove avoided work — assigned codec owner; medium risk.** Add focused tests
   in the existing `crates/spiralcore-transfer/tests/fixtures/sender_delta_checks.rs`
   and reuse `sender_delta_baseline.rs` byte-for-byte. Add a small public-encode
   allocation regression in `tests/unrelated_delta.rs` using the existing
   thread-local allocation-test pattern. Capture a resource-behavior RED for
   whole-literal packing, then implement only the private recognition/helper
   and its two call sites in `crates/spiralcore-transfer/src/delta.rs`. Keep
   `pack`, scans, limits, output order and strict comparisons unchanged.
   Dependency: preserved profile and parent release. This change is independently
   testable and reversible; do not revive the broader failed pruning candidate.
3. **Retain delta-only profile, then accelerate exact schema scans — codec/test
   owners; medium risk.** Preserve the released delta-only diagnostic before
   touching schema production. Add private `records` iterator parity tests in
   the existing schema test module; parent observes RED before implementation.
   In `crates/spiralcore-transfer/src/schema.rs`, use the bounded memchr-backed
   borrowed iterator only in the count and record passes of `encode_with`.
   Promote existing locked memchr 2.8.3 to an exact direct dependency in
   `Cargo.toml` and its crate dependency list in `Cargo.lock`; do not change the
   package version/checksum. Keep training and decoder untouched. Reuse SCVAL001
   and compressed-frame oracles at lexical/binary/record-limit boundaries.
   Dependency: design amendment commit and completed delta-only diagnostic.
   This is independently testable; no entropy or candidate-skipping heuristic.
4. **Qualify exact parity — parent/test owner; medium risk.** Run focused oracle
   comparisons plus existing warm-selection/session tests, including all codec
   winners and 600-byte rolling edits. Reconstruct exact bytes and lengths;
   compare complete encoded frames and operation counts. Preserve actual
   RED/GREEN outputs and any unexpected failure. One final appropriate release
   suite, Check/Clippy and changed-surface coverage follow stable source; avoid
   duplicate broad runs. Dependency: steps 2–3. No decoder/auth/HTTP change.
5. **Matched local campaign — measurement owner; medium risk.** Reuse
   `examples/support/persistent_session.rs` and
   `examples/schema_fitting.rs` dispatch with explicit baseline/candidate/Whole
   roles. Both selectors retain sessions and fit once; overlay exactly baseline
   `src/delta.rs`, `src/schema.rs`, `Cargo.toml` and `Cargo.lock`, restore all four,
   record executable/source identities, and freeze exact
   gates before candidate timing. Run ten pairs per size, 660 operations, with
   actual stopped-store readback and cleanup after verification. Record all
   observations and controls; no builds during measurement. Dependency: source
   freeze and focused independent review. Register the mixed sequence and paid
   artifact seed policy before timing; up to 40 selector artifact publications
   raise the local cap to 700 operations. A failed gate is a retained result,
   not a reason to rerun until it passes.
6. **Bounded WAN pilot and delivery — parent/measurement owner; medium risk.**
   Only after local gates, adapt the existing
   `.github/workflows/spiralcore-persistent-session.yml` launch pattern to a new
   registered source/run. Six pairs per size give 396 base/target transfers in
   six stores, plus at most 24 paid artifact publications (420-operation cap);
   retain the existing <=768 MiB conservative per-store admission and shared
   sender/receiver quotas. Same policy, compute scopes and durability for all
   arms; stop at the first error, retain diagnostics, then verify stopped stores.
   Publish concise before/after/Whole results and any regressions with raw records.
   Dependency: step 5. Earlier failed WAN evidence remains separate.

Success requires unchanged winning bytes and exact output, measured reduction
in unrelated sender CPU at both sizes, and no registered mixed/warm CPU
regression. An improvement over the baseline does not by itself establish a
win over Whole or an energy saving. If profiling instead identifies a different
material path, record one bounded design amendment before its code or timing;
do not expand silently into schema/receiver redesign.

Phase-2 continuation after the retained local gate failure: commit the short
[impossible-record preflight amendment](../specs/2026-10-31-unrelated-cpu-design.md#phase-2-amendment-exact-impossible-record-preflight),
qualify the exact prefix/whitespace/UTF-8 boundary against the existing schema
oracle, then add the early literal branch and exact chunked LF-count helper in
`schema::encode_with`. Compare the helper against the old capped iterator at
record-limit/chunk/tail boundaries; keep the encoding iterator unchanged.
Freeze new candidate source/binaries with the same four-file original baseline.
Run a separately named 700-operation local campaign under unchanged fixtures,
clocks and gates; preserve phase 1 independently. No WAN run follows unless the
new candidate passes. No receiver, transport or codec-selection redesign.
