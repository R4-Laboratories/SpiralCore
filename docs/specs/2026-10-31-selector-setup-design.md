# SC-OPT-017: selector fitting and compressible fallback costs

The user authorized this optimization while preserving minimum transfer bytes
and exact reconstruction. Baseline: `4bb79f976e0939f3ef8b48c3830b07086b98f280`.
UTC date verified 2026-10-31. Implement after the parent commits this design;
the existing authorization covers routine implementation decisions.

## Decision

Make two independent, bounded changes; retain existing public APIs, native
codec parameters, first-block certificate, source guard, receiver and wire formats.

1. **Recognize repeated training layouts without rebuilding them.** In
   `schema::fit`, try the first learned template using the same exact fragment,
   UTF-8, primitive-token and complete-record checks already used by
   `write_first_template`. Extract that structural matcher into a private helper
   usable by both paths; it returns at most 32 borrowed value slices on the stack.
   A proven match charges exactly `2 * literal_len`, preserving `find` saturation
   and termination, then skips duplicate insertion. A miss uses the unchanged
   general parser/hash/index. Preserve the initial total-byte/record admission
   pass, first-seen ordering, all template/schema limits and every artifact byte.
   Do not inspect held-out targets when fitting or silently reuse a prior fit.
2. **Schedule the optional probe using a cheap repetition screen.** After the
   existing static and maximum-certificate eligibility checks, compare eight
   64-byte windows at offsets `i * 16384`, for `i = 0..7`, within the first
   128 KiB. First require the initial 64 bytes to repeat with period 1, 2, 4 or 8;
   if that condition holds and all windows equal the first, skip the native probe and
   execute the existing complete `compress2` candidate. This examines at most
   512 sampled bytes plus bounded short-period comparisons, allocates nothing and
   applies only to an otherwise eligible probe.
   It is a CPU scheduling heuristic, never a compression bound or pruning rule.
   A misleading sample can increase CPU but cannot discard a winning codec.

The structural matcher preserves lexical bytes, duplicate keys, punctuation,
whitespace and value spellings. Unsupported, binary, nested, truncated or
incompatible records retain the original path. Exact first-template recognition
also preserves hash-collision behavior: index zero is already the first candidate
for that exact layout. At the work limit, fitting stops exactly where the old
implementation stopped; encoding keeps its previous literal fallback.

`PreparedSelector::new` receives an artifact; it does **not** perform fitting.
The campaign deliberately pays training generation, actual fitting and context
construction per selector session. Optimizing `fit` therefore removes real paid
work without pretending that setup is free. Constructor behavior and conservative
context charges stay unchanged.

## Alternatives and boundaries

- Continuing the probe stream would avoid repeated compression, but complete
  streaming output parity with the original `compress2` is not established.
  Reject this approach for the present milestone.
- Repetition screening plus unchanged full encoding preserves selected bytes
  without a new native proof. It only helps recognizable repetitive samples;
  nonsampled content may defeat the prediction. Measure that control explicitly.
- Lazy contexts or shared fitted-artifact caches may reduce other setup costs,
  but alter lifecycle/error timing or introduce ownership and amortization policy.
  Defer them; duplicate-layout fitting is a concrete repeated operation today.

Static and actual first-block pruning keep existing tie order
Raw/Whole/Sparse/Schema. Skipped probes yield an exact completed Whole cost, not a
fabricated lower bound. Executed codec/reset/resource errors remain errors; no
new catch-and-retry or implicit reoffer is introduced. All original input limits,
source guard fallback and exact commit verification continue to apply.

## Qualification and measurements

Pin the original `schema.rs` as a test oracle. Require identical fitted artifacts,
matching limit/error behavior and unchanged encoding across repeated layouts,
mixed layouts/order, lexical variants, invalid bytes, zero/32 slots and all
training/template/work limits. A meaningful allocation/parse-work regression test
must show repeated first-layout fitting avoids per-record temporary allocation.
Retain source-derived oracle bytes and one compact RED/GREEN record.

For the screen, test constant/period-1/2/4/8 samples, period-64 negative controls,
random blocks, sampled short-period repetition with random gaps, inverse
first/tail mixtures, block boundaries, all winners and
ties. Compare winner, actual serialized winning frames and costs with exhaustive
full encoding; alternate screened/probed/full calls through one reused context.
The random warm fixture must still send 600 body bytes; the compressible fixture
must retain the 570-byte Whole winner. Existing authorization/error tests remain.

Reuse the existing common-harness baseline overlay, three arms and persistent
receiver machinery. Overlay exactly baseline `schema.rs` and
`schema_selection.rs`; `zstd_whole.rs` stays unchanged and both builds keep the
qualified native probe enabled. This is not an old full-revision claim. Use
three cells: 1 MiB random, 1 MiB zero-filled, and 16 MiB random bases, each with
the same 32-byte midpoint XOR edit. Local ten pairs per cell give 90 sessions
and 180 seed/update transfers; WAN thirty pairs per cell give 270 sessions and
540 transfers in six five-pair stores. Each store holds 45 sessions/90 transfers:
540 MiB object payload plus 40 MiB codec cache, 16 MiB in-flight allowance and
90 * 4096 bytes metadata = 596.3515625 MiB, below the existing 768 MiB bound.
Actual setup and identical Raw seeds remain charged once: 60 actual fits locally
and 180 over WAN. Also retain twenty compressible prepare-only samples and
bounded periodic/deceptive-sample controls, explicitly separate from WAN dose.

Local advancement gates, with the exact metric/cell joins fixed in the committed
measurement contract before candidate measurements: exactness throughout;
median paired candidate/baseline setup CPU ratio <= 0.80; compressible prepare
CPU ratio <= 0.95; random 1 MiB target sender CPU ratio <= 1.05. Report the unchanged
16 MiB path and all distributions, losses and failed gates without reruns or
threshold retuning. These are prospective gates, not measured results.

Report body bytes, sender and receiver-handler CPU, durable elapsed time and paid
setup/seed/update lifecycle separately. Exchange is a nested elapsed scope;
body counts omit transport headers/retransmissions/relay hops, and energy remains
unmeasured. No source change is justified solely by an aggregate timing claim.
