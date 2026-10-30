# SC-WAN-005 implementation and measurement plan

User approval covers this bounded warm-object recheck. Execute the
[design](../specs/2026-10-30-warm-object-recheck-design.md) after the parent commits
these documents. Production changes require a demonstrated defect; no codec
rewrite, schema-selector expansion or broader audit is planned.

1. **Measurement owner:** reuse the existing Rust fixture, HTTP client, operation
   timer, source/plan identity, persistent receiver and stopped-store verification
   machinery. Add only the bounded two-arm plan/session/summary adaptation for
   forced Sparse and prepared Whole, with identical paid Raw seeds. Preserve
   historical commands and evidence. Qualify new accounting/plan behavior with
   targeted RED then GREEN tests, including target absence, actual 600-byte
   sparse receipt, full readback, scope/base failure and seed/setup accounting.
2. **Parent and measurement owner:** run the registered small local qualification
   (three paired trials × two sizes × two arms × seed/update = 24 transfers).
   Retain compute/body/durable-time observations and all exact reopened objects.
   Use this to distinguish current compute costs from WAN waiting; do not infer
   a robust performance result from three trials or require a local timing win.
3. **Parent:** complete one focused qualification/review of changed harness and
   workflow surfaces, then freeze source, executable, plans and fresh stores.
   Reuse the persistent foreground supervisor and authenticated controlled relay.
   Run three ten-trial shards sequentially: 240 actual seed/update transfers,
   with both sizes and both arms present in every shard. Do not overlap builds,
   profiling or uploads with the timed run; do not retry failed trials.
4. **Measurement owner and parent:** stop receivers, exclusively reopen every
   store, verify all 240 complete objects, and validate plan/order/source/body
   joins. Report warm-update and paid-lifecycle paired results versus matched
   Whole zstd, preserving negative CPU/time outcomes and historical-baseline
   differences. Only after custody and exact verification may owned expanded
   campaign stores be cleaned. Preserve formal raw evidence and originals.

Use existing build targets, explicit Rust 1.97.1, incremental-off and the 4 GiB
disk floor. The campaign payload bound is 680 MiB per shard, with the registered
736.3125 MiB conservative bound below existing quotas. No new measurement
framework, packet capture, additional codec arm or MVM integration is required.
