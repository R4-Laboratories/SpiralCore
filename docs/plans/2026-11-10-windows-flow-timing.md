# Implementation plan for the sustained FlowMux timing witness

The work item is the native Windows sustained FlowMux gap under SpiralCore #44 and the #49 capacity gate. Start from reviewed source `8a109ae4966b0f19ef0cbb41e0311ee3cb926f6d`, recheck the actual issue and branch, and preserve ongoing quota and MVM work. The contract is [the timing specification](../specs/2026-11-10-windows-flow-timing.md).

## 1. Add the smallest diagnostic owner

Add default-off `witness-timing` in `crates/spiralcore-mvm-flowmux/Cargo.toml` and a feature-gated timing module. Use fixed atomic counters, elapsed totals/maxima and overflow indicators. No new production dependency is needed. Add `cpu-time` only as a dev dependency, resolving against its already locked closure and checking that unrelated package versions stay fixed.

Define closed pause, poll, lock and frame buckets plus scalar snapshots. Add focused tests for saturating arithmetic, duration conversion and all named buckets. The tests should expose a concrete overflow or omitted classification regression rather than replicate every field mechanically.

Retain timing owners in the existing TLS shared carrier and duplex shared state. Feature-gated cloneable observers share the same owner without allocating an event stream. Record exact diagnostic storage bytes. Feature-off construction and execution should compile without those fields or timing calls.

## 2. Instrument current TLS operations

Work in `src/tls.rs`. Pass closed pause reasons at the existing `pause` call sites. Around `PollState::wait_checked`'s actual `poll` call, record requested timeout, actual elapsed and bounded event/outcome flags. Keep owner checks, interest registration, notification and deadline decisions in their current order.

Record `with_session` success/contention and successful operation duration without extending the session lock. Use guards or a single observation helper so errors and interruptions still produce data. Existing errors remain authoritative. Counter overflow sets a diagnostic flag and never becomes a TLS error.

First verification is the existing readiness, lost-wake, EOF and closure test set with the feature enabled. Confirm the feature does not introduce an extra poll, lock, sleep, retry or wake. Independent review checks that instrumentation observes the wait arguments already computed by product code.

## 3. Instrument duplex exchange and receiver work

Work in `src/duplex.rs`. Around the request and response TX acquisition sites, record elapsed acquisition and actual contention. Record per-kind send duration and response wait status in `DuplexWriter::exchange`. Record receiver request processing duration and response send duration around existing calls. Leave generic mutex utilities and protocol matching semantics intact.

Increment ACK ordinals only after the original validated response is obtained. Ordinals carry counts rather than transfer identifiers or payload-derived labels. The fixed observer may be shared with the witness but must never control admission or stream progress. Reuse paired duplex and refusal tests for the feature-on acceptance check.

## 4. Emit one bounded witness snapshot

Work in `tests/ordered_stream.rs`. Retain the carrier/duplex observers before moving those owners into the stream. Add process CPU samples using `ProcessTime` at start, after setup and at success/failure. Retain each direction's successful write/ACK and exact application-read ordinals as atomics.

Use a common bounded reporting path before existing failure assertions and at successful completion. Include elapsed/setup wall times, process CPU, TLS traffic, scope status, retention and timing snapshots. Failure reporting must not acquire blocking locks. Preserve the original failure and teardown even if optional observations are unavailable. Keep synthetic fixtures and never print private material.

No workload reduction or deadline extension is in scope. The workload remains 4,096 blocks each direction, exact byte comparisons, the same authority scope, cache bounds and original 240-second deadline.

## 5. Run and interpret the targeted evidence

Run the failing native Windows role first with the feature enabled and captured successful output. Preserve the exact source, toolchain, feature flag, command, original failure if any, nonzero counts and compact counters. Reuse the seven-role matrix for portable regression coverage when source is ready. Keep the feature default-off in ordinary component jobs unless a separate diagnostic invocation explicitly enables it.

Compare one matched feature-off run before claiming a product improvement. Classify the largest observed wait: TLS park scheduling, poll empty waits, TX contention, receiver processing or response channel wait. Nested and cross-thread spans overlap. Report them alongside actual CPU, lifecycle wall and bytes rather than adding their totals.

The next behavior change requires a concrete explanation supported by these observations and its own focused regression. If the data is inconclusive, retain the uncertainty and identify one bounded measurement that resolves it. This plan ends at reviewed instrumentation and measured diagnosis; it does not presume a particular transport optimization.
