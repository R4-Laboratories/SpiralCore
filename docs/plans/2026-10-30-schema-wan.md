# Persistent schema WAN implementation plan

The user approved implementation and real WAN testing. Execute the bounded
[design](../specs/2026-10-30-schema-wan-design.md); parent commits this plan before
production edits. Keep evidence lean and retain actual failures and results.

1. **Expose explicit receiver cache/observation configuration — codec owner.**
   Change `src/wan.rs` and `src/bin/spiralcore-wan.rs` only as needed for an
   optional observed serving wrapper and strict plan boolean. Preserve ordinary
   serving behavior and existing wire bytes. Add focused RED/GREEN tests for
   default-off/invalid configuration, persistent codec hits, scoped observation,
   no-observer behavior and observer-error propagation. Risk: medium; logging
   must remain outside handler timing and must not reveal secrets.
2. **Bridge the existing fixture harness — measurement owner.** Add remote
   plan/session/verification commands alongside the existing schema harness,
   reusing `HttpClient`, fixture generation and ten-target sequence. Freeze 30
   trials per size, six balanced arm orders and three ten-trial shard plans.
   Validate unique scopes/targets, actual object charges, all operation limits,
   fresh outputs and source pins. Tests cover malformed plans, bounds, row
   completeness and a real loopback server followed by stopped-store exact
   verification. Risk: medium; remote acknowledgment and independent readback
   must remain separate evidence.
3. **Freeze and qualify once — parent.** Build current and source-derived
   baseline senders with the identical final harness, substituting only the
   immutable old schema source for the baseline. Pin source, artifact, fixture,
   compiler/flags, lockfile and executable identities. Check baseline/optimized
   raw SCVAL and compressed-frame parity and source-substitution boundaries.
   Run the appropriate full qualification once, then release stable binaries.
   No blanket test repetition or unrelated audit expansion.
4. **Run the registered WAN campaign — parent/infrastructure owner.** Use the
   existing short-lived HTTPS relay arrangement and a genuinely remote sender.
   Pin the workflow/source/attempt and receiver identity; scope credentials to
   the run and avoid logging them. Start one fresh cache per shard with codec
   cache and structured observation enabled. Execute 60 sessions per shard,
   retaining all errors. The three fresh receivers may be started together for
   one workflow, but measured shards run sequentially. After collecting the
   artifact, stop each receiver, independently reopen/compare all 640 expected
   objects, then release its owned cache. Three shards
   yield 1,800 targets plus 120 seeds; no extra exploratory WAN dose is implied.
5. **Analyze and deliver — parent with one independent consistency check.**
   Require all frozen identities, 1,920 exact verification receipts and complete
   paired rows. Report body bytes, sender CPU, receiver handler CPU, their bounded
   sum, durable elapsed, exchange subset and actual setup costs separately.
   Include all drift/unrelated losses and trial variability. Update the ledger
   and delivery note with observed scope; publish the reviewed result through
   the already authorized repository process. No full-path byte, pure-wire-time,
   energy or universal WAN performance claim is supported by these measurements.

Use existing owned build targets, incremental builds disabled and a 4 GiB disk
floor. Coordinate the build lane; all builds/tests/profiling finish before timed
transfers. Do not alter original local runtimes, historical evidence or other
labs. No new REST endpoint, packet-capture infrastructure, decoder format or MVM
integration is part of this step.
