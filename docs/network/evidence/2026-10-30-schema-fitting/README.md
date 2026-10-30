# SC-OPT-011 recorded schema-fitting comparison

Frozen implementation: `fe98bf0a40cad50f513652690548f7f0bb4f948f`.
Executable SHA-256: `50e6fb5216a1ae0170543c82932988361f0c646be64bcf5082c8b520d68a45e8`.

The [result report](../../schema-fitting-results-2026-10-30.md) interprets the
single registered ten-trial campaign. It completed 400 target transfers and
240 artifact seeds with exact reopened bytes; 400 successful worker stores were
removed. No performance rerun, dropped observation or held-out refitting occurred.

- [Measurement contract](measurement-contract.md): fixed scope, costs and exclusions.
- [Frozen plan](plan.json): training and held-out fixture hashes, source hashes and counts.
- [Raw rows](paired.jsonl): fitting, every seed/target operation, nested CPU/wall timers,
  byte counts, mode/fallback, reopen verification and cleanup.
- [Summary](summary.json): all 40 family/size/arm cells, median/p95, cold costs and
  actual sender reuse including all destination seeds.
- [Artifacts](artifacts/): synthetic training records and the exact schema/dictionary.
- [Review receipt](final-review.json): procedural audit followed by independent
  code, Rust and input/security review of the frozen implementation.

All byte counts are serialized protocol-body equivalents in both directions,
including replies and the equivalent timing-envelope fields. Total elapsed time
starts before encoding and ends after the verified durable acknowledgment.
Sender and receiver CPU are counted once; receiver and exchange intervals are
nested subsets. Reopen comparisons are outside those timers and separately recorded.
This mechanism run uses no network sockets. No WAN send-time, complete-wire-byte,
peak-RSS or energy result is inferred.

Qualification receipts retain the expected RED failures, the resolved test binding
and fixture-lint errors, and final GREEN outputs. `all-tests.txt` records 241
library/integration passes and two historical ignored HTTP qualification tests;
the focused new tests are included in that count. `doc-tests.txt` adds one doctest,
and `harness-final-tests.txt` adds two fixture tests. `coverage.txt` records 129
repeated coverage-test passes; changed library files have 83.01–97.06% line coverage.
Final all-target Clippy and the release example build passed. These counts are
overlapping validation scopes, not a sum of independent tests or formal assurance.
