# Reduce Combined candidate CPU without changing selected bytes

User-approved objective: optimize unnecessary candidate work, preserve byte
savings and zero fidelity loss, record before/after CPU and transfer time against
zstd, keep review bounded and publish to origin/main. Base is
`4b8cb79fe5fca9e9527a2923924c887d9e7ddbf8` (production from `7e698ae`).

The preceding campaign measured Combined generated sender CPU at 4.28 times
Runtime's CPU with the same 771 logical transfer bytes. It also exposed unrelated
selection overhead. Source inspection identifies full SchemaZstd evaluation
behind a weak empty-intermediate bound, and duplicate residual computation when
the acknowledged rolling object equals the immutable prediction. A small native
stage profile must confirm the expensive work before production changes.

## Selected approach and alternatives

Preserve the existing byte-first policy and codecs. Prepare the exact canonical
schema intermediate once. Use its real length and, where the existing bundled
zstd proof applies, a first-block lower-bound certificate before full compression.
Reuse that same intermediate if compression remains necessary. Inconclusive or
unqualified probes use the unchanged full encoder. Never infer that an unmatched
schema cannot beat whole-object zstd. Do not silently extend the qualified probe
past its existing input ceiling.

Where full equality proves the rolling base equals the prepared prediction,
compute one residual and reuse it. Runtime adds its setup frame to the identical
Sparse residual, so Sparse strictly dominates in the current cost model. Preserve
the exact cost/tie semantics and accurate evaluated-versus-pruned receipts.

Heuristic skipping was rejected because it can change winning bytes. Adding new
codecs, approximate fit, a new cache, or a wire protocol would enlarge this task
without establishing why the current candidate work is expensive.

## Invariants

- Same selected mode, canonical payload and offer identity/base for every matched
  before/after operation. Same logical request/response bytes and publication path.
- Full equality, private immutable state and validated current ACKs remain the
  authority for cached identity/readiness. No caller-provided known hash API.
- Preserve Raw/Whole/Sparse/Runtime/Schema tie order, receiver validation,
  namespace binding, errors and 512 MiB session admission. No unbounded new state.
- A prune receipt is a proven lower bound, never an invented exact encoding cost.
- Whole-zstd control settings and production implementation remain unchanged;
  any shared proof-helper extraction must leave its output behavior unchanged.

## Measurement

First record native independent candidate-stage CPU/wall on generated and
unrelated fixtures at both existing tiers, three trials. Preparation and output
verification are separate. Do not add independent stage durations as if they
were one transfer or infer CPU by subtracting network waits.

Then run 18 persistent Linux sessions: pinned Combined-before, Combined-after,
and equally tuned current Whole-zstd, each on unshaped and calibrated 80 ms /
10 Mbps links, three paired trials with a complete arm-order rotation. Same two
tiers and six deliveries per tier: 216 deliveries, 264 operations (216 sends,
24 schema and 24 checkpoint), 1,717,047,288 requested bytes. Both service binaries
are built and frozen on the same runner before traffic. Fixtures/probe/PKI are
shared; stores are fresh per arm. Setup, CPU, complete elapsed, bidirectional
logical bytes and captured IP totals remain explicit. Match before/after every
operation's identity, selected mode, publication path and logical framing.

Retain all trials and losses, raw receipt/source/executable identity and the full
ZIP in the existing private artifact dataset after fresh immutable-download
verification. This is shaped single-runner QUIC, not a new Internet WAN or energy
measurement. A performance win is measured, not assumed. Focused TDD and
cross-author code/security/proof review suffice; no broad audit or coverage dose.
