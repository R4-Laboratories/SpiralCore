# Remaining schema-candidate CPU: exact reuse experiments

The remaining unrelated-object cost is full schema-stream compression. Reusing
the preceding Whole-zstd compressor context preserves compressed bytes, but does
not materially remove that cost. The context experiment's repeated large opaque
Whole-plus-schema pair improves only **0.37%** by paired CPU ratio. The follow-up
combining context reuse with retained output allocations improves it only
**0.53%**, with small regressions on the large controls. **Neither variant is
promoted to production. The unrelated-object CPU gap remains open.**

This is a bounded local diagnosis against production baseline
`0f15e3d3f8b5ae3521386905e4a4d43e838ed6fb`. It does not supersede the
[last measured gateway transfers](unrelated-gateway-cpu-results-2026-11-02.md).
The ternary/Kuramoto/CA, warm-cache and exact selector production paths remain
unchanged. The experiment changes context scheduling, not candidate eligibility,
compression parameters, codec framing or reconstruction.

## Context-reuse result

Three canonical fixture trials, two object sizes, four data classes and six
balanced repetitions produce **144 pairs / 288 arm observations**. Every pair
compares complete Whole and schema frames between separate and shared contexts,
checks schema frames against the original standalone schema encoder, and decodes
both arms to the exact original bytes and length. All checks pass. Inputs are
1,047,202 and 14,851,384 bytes; the largest intermediate in this dose is
15,142,482 bytes, within the diagnostic's explicitly bounded Whole input domain.

Repeated measurements below contain five repetitions per fixture trial, fifteen
pairs per row. First use is retained separately, not discarded. Values are
medians of measured Whole-plus-schema compression CPU and elapsed intervals;
changes use the median of matched ratios and can differ from ratios of medians.

| Object/class | Separate CPU | Shared CPU | Paired CPU change | Separate elapsed | Shared elapsed | Paired elapsed change |
|---|---:|---:|---:|---:|---:|---:|
| 1,047,202 B, opaque | 3.138 ms | 3.094 ms | -1.48% | 3.139 ms | 3.096 ms | -1.36% |
| 14,851,384 B, opaque | 51.042 ms | 51.105 ms | -0.37% | 51.202 ms | 51.583 ms | -0.83% |
| 14,851,384 B, generated | 94.891 ms | 94.677 ms | -1.44% | 97.615 ms | 96.183 ms | -1.57% |
| 14,851,384 B, residual | 88.273 ms | 87.368 ms | -0.44% | 88.612 ms | 87.519 ms | -0.60% |
| 14,851,384 B, fitting | 109.208 ms | 110.791 ms | +0.01% | 109.785 ms | 110.844 ms | +0.31% |

For repeated large opaque data, the schema stage alone uses 33.556 ms separate
and 33.895 ms shared, with a paired change of -0.68%. Ordinary Whole uses
17.493/16.924 ms in those arms. These stages are nested inside the pair interval:
do not add them to it or to an earlier gateway sender measurement.

The active arm's reported compressor workspace halves from 4,769,824 to
2,384,912 bytes. Both arms coexist in the process, so this is not a measured
process-RSS reduction. The experiment does not establish that production
resource admission can be halved. Both profiles use unchanged level 3, window
25, long-distance matching, one compression thread and checksummed frames.

## Why the apparent shortcuts are not safe exclusions

If no template matches, the schema intermediate contains literal records. Its
uncompressed size exceeds the target by a 48-byte header and 5 bytes per literal.
That fact alone does not prove its compressed size exceeds Whole-zstd: framing
changes matches, block boundaries and entropy coding. This investigation did not
produce a concrete counterexample; it identified the missing domination proof.

Likewise, a destination-too-small error is not a proof that the canonical final
frame exceeds an incumbent. Compressor internals can need temporary writer
space. The existing qualified first-block bound is too small to exclude the
large opaque candidate. Turning either observation into an unconditional skip
would change the selector's exact byte objective.

## Context plus output-allocation reuse

The second registered dose uses the same fixtures and 144 balanced pairs / 288
arm observations. Its separate arm uses two raw compressor contexts and fresh
output vectors; its shared arm uses one context and retains two output vectors.
Both provide the full compression-bound capacity. Both are compared against
independent original Whole and SchemaZstd encodings and decoded exactly. All
compressed-frame and reconstruction checks pass.

| Object/class | Separate CPU | Reused CPU | Paired CPU change | Separate elapsed | Reused elapsed | Paired elapsed change |
|---|---:|---:|---:|---:|---:|---:|
| 1,047,202 B, opaque | 3.134 ms | 3.046 ms | -3.00% | 3.139 ms | 3.046 ms | -2.47% |
| 14,851,384 B, opaque | 49.238 ms | 49.307 ms | -0.53% | 49.259 ms | 49.349 ms | -0.53% |
| 14,851,384 B, generated | 87.722 ms | 92.220 ms | +0.45% | 87.900 ms | 93.220 ms | +0.73% |
| 14,851,384 B, residual | 87.297 ms | 87.423 ms | +0.07% | 87.457 ms | 87.586 ms | +0.07% |
| 14,851,384 B, fitting | 107.950 ms | 108.114 ms | +0.22% | 108.029 ms | 108.308 ms | +0.36% |

These are repeated-phase medians and paired ratios, defined as in the first
table. This is a **joint context-and-output-reuse variant**, not an isolated
allocation experiment. The canonical intermediate is prepared equally for both
arms, so no intermediate-allocation saving is claimed. This dose uses raw
compressor calls with frame validation outside both compression timers, whereas
the first dose uses the public Whole wrapper. Do not subtract absolute timings
across the two doses to attribute a speedup. Reused output capacity remains
allocated between calls and is reported separately from context workspace.

## Implementation decision

The added Rust diagnostic and retained observations are the delivered changes.
Production sources, codec parameters, selected bytes, warm-cache behavior and
ternary/Kuramoto/CA integration are unchanged. The experiment does not justify
new production context-sharing complexity or retained output memory to solve
the large unrelated-object gap. No further alternative or network campaign was
run after these two registered doses.

A larger reduction requires avoiding most of the losing compression work, not
just recycling allocations. Under the current exact-selection contract, that
requires a valid exclusion bound or a substantially faster equivalent encoder.
A workload-informed CPU-budget policy is another engineering option, but it
could miss a smaller schema encoding. Such a policy can preserve exact decoding
while changing the byte objective; it must be explicit and measured separately.
It is not silently introduced by this change.

## Accounting and limits

These are native macOS Rust compression diagnostics, with release binaries and
source/input hashes frozen before timing. Preparation, input loading, frame
comparison and decoding are outside compression timers; context construction and
canonical intermediate construction have separate observations. First use means
first compression on a context, not a cold machine or cold filesystem. All
observations, order, failures and controls are retained.

No new network transfer, durable commit, MVM boundary, energy, statistical
significance or broad workload qualification is claimed. Production remains
unchanged, so the existing network results remain the applicable evidence; a
new network run would not test a new optimization. The last shaped campaign
still reports 27.39% more sender CPU than Whole-zstd for large unrelated objects,
while the full mixed session retains 58.26% fewer captured IP bytes and 53.17%
less elapsed time. Those are prior campaign results, not gains from this work.

Evidence and reproduction: [retained diagnostic](evidence/2026-11-02-schema-context-cpu/README.md).
