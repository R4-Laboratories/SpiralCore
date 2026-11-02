# Reduce unrelated-object residual search work

User authorization: optimize remaining unrelated-object CPU overhead, preserve
byte savings and exact reconstruction, record CPU and transfer time against zstd,
keep audits lean and publish to origin/main. This is the next bounded step after
candidate pruning. Baseline: `4a8ec79c9b185c85407e9767653dbcc4af97bf7e`.
Reuse the existing isolated worktree and shared Cargo target; preserve user files.

The fresh18-row native stage diagnosis finds large opaque medians of2289/2273us
for Runtime/Sparse residuals,2460us schema intermediate,39110us schema-zstd and
18098us Whole. These stages are independent diagnostics, not additive transfer
cost or Linux/WAN timings. Schema remains dominant; no cheap sound general proof
was found to exclude it when unrelated data falls back to Raw/Whole.

Alternatives: heuristic schema exclusion could lose byte savings and is rejected;
a broader zstd proof/encoding redesign exceeds this narrow task; exact residual
search reduces real avoidable work with a bounded proof and is selected. The
expected complete-transfer gain is modest, not elimination of the entire gap.

The aligned delta recipe only emits equal runs of at least32bytes. Probe one
anchor every32positions: every eligible32-byte equal run contains an anchor.
On an equal anchor, expand backward at most31bytes and forward to recover the
same maximal run. After a short run, resume past its known mismatch with a new
anchor31bytes ahead; reject shorter runs and preserve ordering. Keep prefix/
suffix candidate, literal boundaries, operation caps, costs/ties and packed frame
bytes identical. No unsafe code, dependencies, new codec, wire format, decoder,
identity/authorization changes or Whole-zstd control changes.

Before implementation, demonstrate a focused failing run-search contract, then
compare against an independent scalar oracle over offsets and31/32/33 boundaries,
short inputs, unequal lengths, dense/overlapping runs and operation limits. Frozen
baseline delta/Combined witnesses must preserve canonical Offer/Data bytes and
reconstruction. Reuse existing negative trust-state tests; do not expand audits.
Record a bounded before/after native stage comparison with unchanged fixtures.

Network comparison reuses the three-arm paired policy: current pinned baseline,
optimized Combined and unchanged Whole-zstd. Same18persistent sessions,
264operations/216deliveries, two sizes, three complete arm-order rotations,
LAN and calibrated80ms/10Mbps links. Explicit new registration schema maps to
this exact baseline; historical policies and analyses remain byte-identical.
Static baseline/registration update in the existing workflow avoids new duplicate
workflows. Fixture generation and transport stay unchanged. Logical byte parity,
exact receiver readback, CPU and full elapsed must be recorded, including losses.
This is shaped Linux QUIC, not a new Internet-WAN or physical-energy qualification.

Retain readable reports/analysis and one compact raw-receipts tarball+manifest in
Git; full originalZIPs/binaries/captures go to the existing private immutable HF
archive with fresh download verification. No broad cleanup. Success requires exact
bytes, bounded-resource correctness and measured search CPU reduction. Network
changes are descriptive; if they do not resolve the unrelated-object gap, say so.
