# SC-OPT-009: Packed CA replay and prepared sender state

Status: accepted for implementation on 2026-10-30 under the user's explicit instruction to optimize packed CA evolution and reuse prepared state. No additional approval is required for these bounded changes. Baseline: `d4532b0`; the measurement owner preserved that source before edits.

The shared-runtime experiment reduced bytes substantially on matching traces, but replay and candidate construction consumed more CPU than raw transfer. This iteration reduces that computation without changing the coupled ternary, memory, or Kuramoto equations, canonical bytes, codec selection decisions, authorization, publication, or resource limits.

## Decision

Replay will pack all six CA rings into private `u64` words once after checkpoint validation. Each word applies rule 184, 110, or 30 with bitwise operations. Neighbor shifts wrap at the actual ring width, including widths one and two and partial final words. Padding remains zero. A reusable scratch buffer avoids allocating a row on every transition. Checkpoint emission consumes the packed words directly; it does not unpack all cells or allocate a temporary checkpoint on each trace step.

The public scalar `State` representation and `State::step` remain available with their current behavior. Numeric transition code and checkpoint header serialization are shared with the packed replay path. Floating operations retain their exact order. Initial decoding still validates the entire checkpoint; each emitted checkpoint validates the evolving numeric state, while packed ring dimensions and padding are invariant by construction. Exact output-size preflight and the 16 MiB output, 64 KiB checkpoint, 4096-step, width and work limits remain unchanged.

Separately, the sender gains an immutable `PreparedPrediction`, owning private boxed checkpoint, prediction and setup bytes. Construction validates, replays and hashes once. Read-only checkpoint/prediction access and retained-byte accounting expose its cost; preparation for a target reuses those bytes and computes its exact residual. `transfer_prepared_runtime_batched` uses the same candidate comparison and commit path as the existing API. Existing calls retain their behavior. Each prediction remains bounded by the existing 16 MiB output and 64 KiB checkpoint limits; caller retention and cold preparation are reported explicitly. No receiver replay cache or authorization/cache-quota change is introduced.

Flow: validated checkpoint → numeric transition plus packed CA transition → identical canonical prediction → existing residual/candidate selector → existing verified durable commit. Prepared state reuses the prediction on the sender only.

## Alternatives and risks

Packing and unpacking each public step would preserve the API but retain substantial per-cell work. Changing `State` to expose packed fields would unnecessarily break callers. SIMD, floating-point approximation, new CA rules and receiver caching introduce separate compatibility or security questions and are deferred. Private packed replay is the narrowest change that removes both scalar CA evolution and per-step scalar serialization.

The primary correctness risks are partial-word periodic boundaries, padding, stale scalar state during serialization, and accidental floating-point reassociation. Scalar-rule comparisons, full replay byte comparisons and the existing native vectors address those risks. Performance remains a measurement question: fewer operations do not establish a speed or energy claim.

## Qualification

Test rules 184/110/30 against the preserved scalar equations across boundary widths and varied initial rings; compare complete final checkpoints and traces against repeated public scalar steps. Preserve exact native vectors and existing malformed-input/output-budget tests. Capture a targeted RED before implementation and GREEN afterward. Measure archived baseline versus optimized replay, preparation, prepared reuse and complete durable transfer with unchanged data and transport. Record CPU and wall time separately, cold preparation and repeated retained memory, and unchanged bytes/exact recovery. Local results do not establish a new WAN result; no energy claim follows without energy measurement.
