# SC-OPT-016 implementation plan

The user has authorized the bounded optimization. Implement the
[approved design](../specs/2026-10-30-bounded-selection-design.md) after its parent
commit; no new approval pause. Preserve existing formats and byte-minimum decisions.

1. **Freeze and guard (parent; medium risk).** Preserve baseline production from
   `4234591e25b4ca87db6a4670bdf407f48a581575` before edits. In
   `crates/spiralcore-transfer/build.rs`, `build_support/` and `Cargo.toml`/lock,
   implement exact bundled-source guard with direct pinned zstd-sys metadata and
   sha2 build hashing. Hash the full C/header tree plus dependency build script;
   reject foreign/pkg-config/override cases by leaving the cfg disabled. Retain
   guard RED/GREEN fixtures and baseline-disable behavior. Dependency: design
   commit; rationale: a source-specific proof must not silently cover a different
   zstd implementation. Do not alter native source or active codec settings.
2. **First-block helper (selector owner; medium risk).** In
   `crates/spiralcore-transfer/src/zstd_whole.rs`, add the small public availability
   diagnostic, private header/bound parser and guarded probe as separate bounded
   changes. In `tests/zstd_whole_probe.rs`, first retain meaningful RED, then prove
   header/size parity against the unchanged full encoder and bounds against its
   complete output. Include block boundaries, compressible/random/structured data,
   repeated abort/reset, unsupported guard and malformed header unit cases.
   Dependency: baseline preservation and guard API; rationale: stop after a proven
   loser without changing any transmitted encoding. Reset/resource/codec errors
   stay errors; unexpected shape is inconclusive.
3. **Selector integration (selector owner; medium risk).** Change only the Whole
   branch of `src/schema_selection.rs` after existing static pruning. Convert an
   actual first-block bound through existing framing and tie rules; otherwise use
   unchanged `compress2`. Extend `tests/warm_selection.rs` for exhaustive-reference
   cost/mode/body parity, actual new pruning and all existing winner/error cases.
   Dependency: helper GREEN. The parent owns any mechanical old expectation update
   outside these files; no endpoint/client change is needed.
4. **Matched qualification (measurement owner; medium risk).** Adapt existing
   `examples/support/warm_object.rs` profile and dispatch, preserving old commands.
   Use one common harness with baseline production overlay vs candidate and Whole;
   bind cfg availability in metadata without calling a symbol absent in baseline.
   Retain context/fitting/base charges, exact/pruned receipts, failures and stopped
   readback. Add focused harness RED/GREEN before one coordinated parent final
   qualification. Dependency: stable codec APIs; rationale: isolate this change
   without new measurement infrastructure or hidden setup amortization.
5. **Registered measurements (parent; medium risk).** After source/executable
   freeze and bounded independent review, run local10 paired trials per1/16 MiB
   size, three arms,120 seed/update transfers across two fresh five-pair stores.
   Measure compressible fallback CPU separately as a retained control. Gate WAN on
   small-target median paired sender-CPU ratio <=0.80 plus exact output/minimum
   preservation; do not retune thresholds or discard failures. If qualified,
   parent-owned `.github/workflows/spiralcore-bounded-selection.yml` runs30 pairs
   per size/360 transfers over six five-pair stores with existing scoped authority
   and quotas. Otherwise retain the local negative outcome and stop WAN advancement.
   Dependency: final qualification and local gate. Evidence goes under
   `docs/network/evidence/2026-10-30-bounded-selection/`; no speed claim before data.

Coordinate the single build/timing lane. Reuse existing Rust1.97.1 targets with
incremental disabled and4 GiB disk floor. Focused tests precede one final full
qualification/Clippy and lean changed-code coverage; do not run builds/tests during
measurements. Each phase preserves a usable original full-encode fallback and can
be reviewed separately. Record exact commands and raw RED/GREEN/measurement output;
no new audit framework, benchmark-driven fixture changes or unqualified full
streaming output is part of this milestone.
