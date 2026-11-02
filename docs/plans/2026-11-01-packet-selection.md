# Packet CPU and byte-selection implementation

Contract: [packet-selection design](../specs/2026-11-01-packet-selection.md),
preserving [packet v1 schema](../contracts/spiralcore-gateway-packet-v1.schema.json)
and [shared gateway boundaries](../specs/2026-11-01-shared-gateway.md).
The parent reviews and commits this design before production changes. The user's
optimization instruction already authorizes this bounded work.

1. Measurement owner freezes the baseline release executable and source hashes,
   then records the existing Raw/Zstd packet results and a matched parameter-only
   diagnostic. Hold production edits until that freeze is confirmed.
2. Write focused RED tests for FastZstd/Adaptive API and counters, exact packet
   recovery, a current positive repetition guard, mandatory probes, flow eviction,
   control/unsupported packet fallback, and changed-pattern byte-regression
   reporting. Observe expected failures before implementation.
3. In `gateway-core/src/packet.rs` only, add the packet-specific prepared encoder,
   reusable bounded output, copied counters and private bounded advisory flow
   state. Keep Raw/Zstd, decoder, wire framing and object/MVM engines unchanged.
4. Observe focused GREEN and run the existing packet negative cases. Root owns
   service mode exposure and the final native/cross checks. Keep Cargo at two
   jobs; no builds or diagnostics during timed measurements.
5. Measurement owner runs the same four-arm packet workload. Compare byte counts
   and CPU independently and against equally tuned FastZstd. Preserve misses,
   failing targets and all raw receipts; do not tune away adverse vectors after
   observing them.
6. Root registers the constrained network dose and runs matched four-arm tests
   only after source/binary identities are frozen. Review full captured bytes,
   exact application results, CPU, complete wall time and setup separately.
7. Publish the implementation and measured disposition. Existing configuration
   defaults remain unchanged; experimental Adaptive is not promoted merely
   because it saves CPU on an opaque fixture.

Ownership: architect/planner owns this design and core packet source/tests;
measurement owner owns fixture/benchmark and before/after evidence; root owns
gateway configuration, network registration/runner, builds and publication.
Lean independent source and arithmetic review closes this milestone. No new
repository-wide audit or coverage project is part of the task.
