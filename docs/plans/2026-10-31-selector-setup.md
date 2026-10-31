# SC-OPT-017 implementation plan

Follow the [design](../specs/2026-10-31-selector-setup-design.md). The user has
authorized the direction; parent commit releases production implementation.
Each optimization is independently testable and preserves the existing encoder.

1. **Freeze and profile (parent/measurement owner; low risk).** Preserve production
   baseline `4bb79f976e0939f3ef8b48c3830b07086b98f280` and the actual setup/prepare
   profile before editing. Reuse existing targets, fixtures, binaries and evidence
   conventions under `docs/network/evidence/2026-10-31-selector-setup/`. Finalize
   prospective gates in the common measurement contract before candidate timing.
   Dependency: design commit. No new benchmark framework or discarded warmups.
2. **Fit duplicate-layout fast path (assigned codec owner; medium risk).** In
   `crates/spiralcore-transfer/src/schema.rs`, extract a private exact first-layout
   matcher and use it in both the current encoder and fitting. Preserve input
   preflight, comparison charge, saturation, ordering and bounded fallback.
   Add focused fit-oracle tests under `crates/spiralcore-transfer/tests/`, with an
   immutable baseline fixture under `tests/fixtures/`; retain a behavioral RED
   demonstrating repeated-record allocation/work before GREEN. Dependency:
   preserved baseline. Do not change artifact/stream formats or context ownership.
3. **Probe scheduling (assigned selector owner; low risk).** In
   `crates/spiralcore-transfer/src/schema_selection.rs`, add the private fixed-window
   short-period-plus-repeated-window screen only after existing probe eligibility checks. A screen hit
   falls through to full encoding; it cannot create a pruning receipt. Extend
   focused selector tests for sampled deception, real random pruning, compressible
   full encoding and exact exhaustive winner/frame parity. Dependency: baseline
   freeze; independently deliverable from step 2. Native helper/guard stay intact.
4. **Matched qualification (measurement owner/parent; medium risk).** Adapt the
   current `examples/support/warm_object.rs` bounded profile and corresponding
   workflow/dispatch to explicitly bind baseline and candidate source. Retain actual
   per-session fitting/context setup and Raw seeds. Run focused RED/GREEN, one
   coordinated full qualification/Clippy and targeted changed-source coverage.
   Existing source identity, exclusive output, quota, failure retention and stopped
   exact readback machinery remain. Dependency: stable implementations. Independent
   reviews are limited to changed code and evidence claims.
5. **Local then WAN (parent; medium risk).** After source/executable freeze, run
   180 local transfers across the three registered cells and twenty interleaved
   compressible prepare samples; retain bounded period-64/deceptive-gap controls. Apply
   the preregistered exactness/setup/fallback/random-path gates once. If they pass,
   run the existing six-store machinery with the registered 540-transfer WAN dose
   including compressible transfers, against equally tuned
   Whole. If a gate fails, preserve the negative and stop WAN advancement. Record
   CPU, body bytes, durable wall, paired ratios and paid lifecycle; distinguish
   algorithm savings from network variability. Dependency: qualification and local
   gate. No speed result is claimed before these observations exist.

Coordinate the single build/timing lane. Rust 1.97.1, locked dependencies,
explicit native target, hardware SHA feature, incremental disabled and the 4 GiB
disk floor remain. No builds/tests/profilers overlap registered timing. Parent
owns commits/publication; do not modify other labs, sealed evidence or user data.
