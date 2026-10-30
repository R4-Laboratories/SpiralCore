# Rust shared runtime and exact residual transfer

Status: accepted implementation design, 2026-10-30. Ternary state, Kuramoto dynamics, and cellular automata are the central executable model in this milestone. MVM integration follows this work. This document specifies new work; it is not a performance result.

## Source and scope

The source is the user's existing `spiralcore.py`, SHA-256 `77076e195d3f93d91b79b653e946af58b215c52166382269de9360da83226ad0`. Its embedded `spiralcore_kernel` was extracted with `ast.literal_eval`, without execution; kernel SHA-256 is `2e11825b16a825f066bc249d785d436c3345abe4fd2c0958fbdb156dea9eb269`. The extraction manifest and independent reference vectors are retained with the milestone evidence. Relevant Python definitions are `rule_184`, `rule_110`, `rule_30`, `step_ca`, `kuramoto_step`, `SpiralcoreRuntime.step`, `memory_field`, `think`, and the later `fixed_dt` override of `sync_system_to_kuramoto`.

Port the finite transition slice into `crates/spiralcore-transfer/src/shared_runtime.rs`. The latest six CA rows, oscillator state, ternary roots, and transition parameters are sufficient to continue this slice. GUI history, source-bearing saved cards, wall clock, router logs, audio, and audit strings do not enter its next-state equations and are excluded. Received input is data; it never supplies executable Python or Rust.

This is not the `SystemLawCodec` format or Python's compact `TRACE_STRUCT` format. In that older codec, CA184 checks particle conservation while block deduplication and RLE reconstruct bytes. Here the runtime actually regenerates full state objects, and an exact residual handles any difference between that prediction and the desired object.

## Transition contract

One tick follows the observed Python order:

1. Increment `N`, compute its 3-adic valuation, and append a root containing the payload value, memory count one, and the first four pre-step oscillator complex phases.
2. If `N` is divisible by three, fold the last `min(root_count, 3^v3(N))` roots when at least two exist. Preserve the Python unweighted average of current root values and sum memory counts and complex phase sums. Retain at most 81 roots. This is the actual runtime fold, not the object store's conventional three-equal-scale DAG carry. Its value average is a lossy summary, not a lossless representation of input payload history.
3. Advance logical time from `(N-1)*fixed_dt` to `N*fixed_dt`, starting at zero. Apply the existing elapsed-time clamp `[0.005, 0.25]`, then the Python Kuramoto update. Coupling is `K*sin(psi-phase)` when field magnitude exceeds `1e-12`; it does not acquire an extra multiplication by the order parameter.
4. Rotate the root memory field by the logical clock phase and execute `think`: the secondary phase coupling and optional memory torque both change oscillator phases. Roots and their phase sums affect subsequent replay bytes.
5. Advance all six periodic rings synchronously: rules 184, 110, 30 and their independently evolved counterfactual initial complements. Counterfactual rows are not recomputed as complements on every step.

The logical clock is shared reproducible state. This does not assert equal physical clocks, distributed consensus, or an Internet routing implementation.

## Numeric and binary profile

`RustReplayV1` uses binary64 and exact dependency `libm = 0.2.16`, with default features disabled and `force-soft-floats` enabled. Fix operation order, use no fast-math or explicit fused operations, and use this provider for trigonometry, hypotenuse, angle, and remainder operations. Initial phases are the retained 16-element Python `Random(184)` vector, followed by the existing initial Kuramoto step of 0.05 seconds. Defaults retain four cores, 16 oscillators, memory gain 0.35, secondary coupling 0.15, Kuramoto coupling 1.75, and fixed logical interval 0.35 seconds.

Python reference comparisons use exact CA/integer/carry outcomes and stated float tolerances. Rust canonical byte vectors and replay hashes are exact. Neither pinned software math nor native tests alone establish all-platform bit identity: additional architectures must pass the same vectors before qualification. A predicted-byte digest mismatch fails closed. No Python floating-point byte compatibility is claimed.

Canonical checkpoint magic is `SCRUN001`. Little-endian fields contain `N:u64`, width `u16`, memory-enabled `u8`, one zero reserved byte, four binary64 parameters (`fixed_dt`, `memory_gain`, `k_coupling`, `kuramoto_k`), 16 phases, 16 natural frequencies, root count `u16`, roots, then six MSB-first packed binary rows. Each root holds value binary64, memory count `u64`, and four real/imaginary binary64 phase sums. Reserved bits, trailing bytes, invalid row padding, nonfinite floats, and negative-zero encodings are rejected. Encoding normalizes numerical zero. Checkpoint identity is the existing whole-object identity, including exact length.

Recipe magic is `SCRCP001`, followed by steps `u32`, payload binary64, output kind `u8`, and three zero reserved bytes. Kind zero emits the final canonical checkpoint. Kind one emits `SCTRAC01` followed by each post-step checkpoint as `u32 length + checkpoint`. The trace retains all transition state, including all six rings and all root phase sums. Zero-step final output is the canonical input state; zero-step trace is its eight-byte header.

## Bounds and API

Public API: `State::seeded(width)`, `State::encode`, `State::decode`, `State::step(payload)`, `Recipe::{encode,decode}`, and `replay(checkpoint, recipe, output_limit)`. `OutputKind` is `FinalCheckpoint` or `Trace`. Errors use the crate's existing checked error type.

The first profile fixes four cores and 16 oscillators, limits width to 1–4096, roots to 81, checkpoint input to 64 KiB, steps to 4096, and generated output to 16 MiB. Bound `N` and total remembered count to one billion, payload/root values to magnitude `1e12`, frequencies/couplings/gain to magnitude 16, and `fixed_dt` to `(0,5]`. Validate each root's finite phase components against its memory count, and require total remembered count at most `N`. Reject arithmetic overflow and phase values outside the canonical phase interval. Replay preflights a conservative checked work count `steps * (6*width + 16 + 81*4)` against 134,217,728 update units. Trace growth is checked before appending each record. These are bounded-work units, not CPU-time estimates.

## Transfer integration

Add explicit `RuntimeV1` mode 5. `Offer.base` names a verified canonical checkpoint in the existing authorized object namespace. The first Data body carries the versioned recipe and expected prediction digest, with zero output contribution. Replay regenerates the prediction under reserved resources. Remaining checked sparse COPY/LITERAL frames reconstruct arbitrary target bytes against that prediction, including insertion, append, truncation, and literal fallback. Prediction and target may have different lengths.

Receiver admission reserves `2*MAX_OUTPUT + 2*target_length + 2*checkpoint_length + 2 MiB` before replay, within the existing shared budget. Missing checkpoint, wrong namespace, revoked authorization, unsupported profile, excessive work, malformed state, prediction mismatch, and final target mismatch fail without publication. No implicit retry crosses an authorization or missing-base failure.

The sender evaluates the runtime candidate against the existing configured raw, zstd, sparse, and prefix candidates before Begin, using complete bidirectional protocol-body cost and existing deterministic tie rules. Only a strictly smaller runtime candidate wins a tie with existing modes. All candidate preparation CPU is counted. Falling back does not erase that cost. Cache seeding is recorded separately and amortized explicitly.

Keep the existing final length/digest check, authorization recheck, staging, durable publication, and acknowledgement semantics. A compact recipe alone is never a verified object acknowledgement. The checkpoint must be delivered through the same ordinary verified-object path before a peer can advertise it as available.

## Qualification boundary

Require reference parity, exact replay, canonical parser negatives, resource failures, authorized checkpoint lookup, exact residual reconstruction, and durable restart tests. Measure actual seeded and unseeded transfers against existing codecs on the same output bytes, durability, and transport. Report generation, selection, sender/receiver CPU, bytes in both directions, and verified commit wall time. Include exact generated traces, perturbed traces, unrelated bytes, and parameter/state mismatches. Generated-data savings demonstrate shared deterministic-state reuse; they do not establish universal compression or better WAN performance. A new WAN claim requires a new real WAN run.
