# Reduce remaining schema-candidate compute without heuristic exclusion

Authorized task: attack the remaining unrelated-object schema-candidate cost,
preserving byte savings and exact reconstruction, retaining CPU/time evidence and
publishing validated work. Baseline `0f15e3d3f8b5ae3521386905e4a4d43e838ed6fb`.
Reuse the existing isolated checkout and shared Cargo target, with bounded disk.

Literal-only SCVAL is not proved dominated by Whole-zstd: framing changes its
compressor input, matches and block boundaries. A small destination buffer's
compress2 error is not a proof that the canonical output exceeds the incumbent;
temporary compression workspace can require more than the final output. Neither
shortcut is authorized. Progressive exact output bounds could stop opaque data
only near the end and require wider source qualification, so defer that path.

The first controlled experiment compares the existing separate Whole and Schema
CCtx instances with one reusable same-parameter context: encode the Whole target,
then the canonical schema intermediate, with complete output capacity. compress2
resets its session; both existing encoders use identical new_encoder parameters,
no dictionary/prefix. Exact output equality is a mandatory experimental gate.
Measure context initialization, prepared repeated CPU/elapsed and retained zstd
workspace separately, with matched public fixtures and all observed losses.

A small Rust diagnostic may be added before production changes. Use the same
schema/fixture identities as the prior paired campaign, plus schema-fitting and
compressible controls if needed for context-reset correctness. Freeze source,
executable, toolchain and input identities before timing. CPU stages remain
separate from network transfer; no addition of overlapping measurements.

Only if the diagnostic supports a useful change, implement a narrow private
shared-context seam for Combined selection. Retain full candidate evaluation,
exact canonical compressed bytes, cost ties, limits, error/reset behavior and
receiver/public protocol semantics. Preserve ordinary Whole and standalone schema
behavior. Do not expand the first-block proof domain. Keep unsupported/resource
paths fail-closed. Conservative admission can remain unchanged; distinguish
reserved charge from actual retained codec workspace.

Before production code: focused RED, independent frozen old-encoder parity,
context reuse/reset/boundary/error checks; then GREEN and scoped integration.
Record the exact selected implementation in the plan before editing. No new wire
mode, no heuristic skips, no relaxed byte objective. If the proposed reuse loses
or shows no useful gain, retain the negative and reconsider one concrete bounded
alternative rather than shipping an unmeasured change.

Final qualification uses the existing paired persistent-session campaign on one
Linux runner: pinned before, optimized after and equally configured Whole-zstd;
18sessions,264operations,216exact deliveries at1,047,202 and14,851,384bytes on
LAN and calibrated80ms/10Mbps links. Keep all setup/capture/CPU/elapsed and losses.
Historical analyses remain immutable. This is not new Internet-WAN or energy
qualification. Large originals use existing private HF immutable custody; compact
raw receipts are bundled in Git. Lean cross-author code/security/arithmetic reviews.
