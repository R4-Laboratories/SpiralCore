# SC-OPT-002 receiver cost evidence

Status at this checkpoint: baseline characterization complete; candidate comparison pending.
See the [registered plan](../../../plans/2026-10-29-receiver-efficiency.md) and
[optimization ledger](../../optimization-ledger.md).

`baseline-block0.jsonl` contains the original metadata and 16 observations. Every target and base
matched exactly after reopening. This single observation per cell characterizes costs and noise;
it is not the eight-pair old/new comparison. `baseline-characterization.json` validates inventory,
stage arithmetic, graph readiness and exactness. `baseline-receipt.json` records qualification,
reviews and source/build identities. The original failed relative-path setup occurred before any
measurement and is retained separately; the absolute-path setup succeeded.

`build-old/` contains the immutable native build metadata and helper source snapshot. The compiled
executable remains at
`/Users/casey.allard/spiralcore/local/receiver-efficiency-20261029/build-old/receiver-probe`, SHA-256
`eed98c450a926a8c3caaf564efc2fbeecfea972b71f2745f258815e4f30eb198`. Its compile revision is
`74ee59df7b7a55476fa573b17f5d947fc28fb030`; production code is unchanged from `fedf38a`/`b552017`.
The build manifest's `compiled_not_measured` status is the original build-time state; the subsequent
baseline receipt establishes execution. Do not rewrite the original manifest as results arrive.

The snapshot preserves its original absolute Cargo path dependency for provenance. To reproduce on
another checkout, copy the helper into a new directory with `src/lib.rs` and `src/main.rs`, point the
Cargo dependency and build script's source root at the selected production checkout, and retain a
new manifest of those portability edits. Use the recorded lockfile, compiler and native target.
`probe-method.md` and `schedule.json` describe exact timing, setup and process-order boundaries.

The helper uses a per-byte xorshift fixture, distinct from the prior WAN fixture. Both node-budget
policies disable graph retention. Population N means N unrelated objects plus one active base, not
an empty cache. Modeled frame bytes are not observed network traffic. Handler CPU is a subset of
the local-operation CPU and must not be added again. Other host activity and OS caches are
uncontrolled; all observations are retained. No WAN or energy claim follows from this diagnostic.
