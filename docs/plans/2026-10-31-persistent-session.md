# SC-OPT-018 implementation plan

Implement the [accepted design](../specs/2026-10-31-persistent-session-design.md)
after parent commit. The user approved the direction; no additional approval pause.
Parent owns the build/timing lane and publication. Keep changes independently
reviewable and retain compact RED/GREEN evidence.

1. **Freeze contracts and baseline (parent/measurement owner; low risk).** Preserve
   `28a373ddb7ab9d236fd4b872db128559d4404cb8`. Freeze identical once-per-session
   training, owned-input semantics, ten-update sequence, counters and store budget
   in `docs/network/evidence/2026-10-31-persistent-session/measurement-contract.md`.
   Profile receiver Begin/Data/Finish externally over 44 exact objects in two
   sizes/two clock modes; require median instrumentation overhead <=1.10 before
   using the attribution. No speculative cache/durability changes. Dependency:
   design commit.
2. **Owned acknowledged state (session owner; medium risk).** Add
   `crates/spiralcore-transfer/src/session.rs`, its export in `src/lib.rs` and
   focused `tests/persistent_session.rs`. First capture promotion/failure/ownership
   RED tests. Implement bounded immutable base retention, Context binding,
   readiness invalidation, explicit schema seed/recovery and Stats/selection
   reporting. Add only a crate-private verified-base selector seam in
   `src/schema_selection.rs`; preserve existing public APIs. Target promotion uses
   its existing validated object ID and same owned Vec. Dependency: baseline and
   tests; purpose: eliminate repeated ownership/hash work without trusting hints.
3. **Prepared runtime session (same owner; medium risk).** In the same module and
   focused tests, add the separate `RuntimeSession` around existing
   `shared_transfer::PreparedPrediction` and prepared-runtime sender. Retain one
   immutable prediction/checkpoint, explicitly seed/reseed it and test multiple
   exact targets plus checkpoint loss/recovery. No runtime math, decoder or
   candidate-policy changes. Dependency: shared session invariants from step 2.
4. **Bounded HTTP persistence (parent; medium risk).** In `src/wan.rs`,
   `src/bin/spiralcore-wan.rs` and `tests/wan.rs`, retain default-close wrappers and
   add explicit keep-alive configuration. RED/GREEN must prove repeated requests
   on one connection, per-request authorization, idle/global/per-connection limits,
   explicit close, malformed/truncated-body closure and clean shutdown. Add
   connection telemetry and strict optional plan configuration. Dependency:
   existing adapter boundaries; independent of session codec changes.
5. **Matched sequence harness (measurement owner; medium risk).** Extend existing
   `examples/support/warm_object.rs` machinery and dispatch, preserving historical
   profiles. All arms fit once if needed, own identical inputs, use the same
   keep-alive policy and advance only after ACK. Record construction, ownership,
   seed, each of ten updates, cumulative sender CPU and actual connection counts.
   Regenerate/reopen every object after shutdown. Dependency: stable APIs.
6. **Qualify and measure (parent; medium risk).** One coordinated focused/full
   qualification and changed-path coverage, then scoped cross-author review.
   Freeze source/binaries and run 660 local transfers. Gate at exactness plus
   cumulative setup-and-ten-update sender CPU ratio <=1.05 for both sizes. If
   qualified, run the registered six-pair/size 396-transfer WAN pilot using existing
   workflow/relay infrastructure and six quota-bounded stores. Preserve negative
   outcomes, exact byte checks and recovery tests. Report body bytes, CPU, durable
   wall and paid seed/lifecycle separately; no guaranteed gain is preregistered.

Use Rust 1.97.1, locked dependencies, existing shared targets, incremental disabled
and the 4 GiB disk floor. No builds/tests/profilers overlap registered timing. No
new generic framework, receiver trust shortcut, full streaming codec, background
retry, research-clock synchronization or MVM integration belongs to this milestone.
