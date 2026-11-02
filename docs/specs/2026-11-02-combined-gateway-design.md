# One runtime and rolling-cache gateway session

Base: `84dbf5f6891a9f93bda4cc6d9b2c8c1a7e3bf273`. The user approved proceeding
with the preceding recommendation: combine runtime prediction and rolling warm
reuse, remove unnecessary repeat work, preserve exact bytes, and record costs.
This approval authorizes implementation without another approval ceremony.

## Decision and alternatives

Add one CombinedSession to the existing transfer engine and expose it through
ObjectSender and the reliable object gateway. Retain one PreparedPrediction,
one acknowledged rolling object and one PreparedSelector. Reuse all existing
wire modes, decoders, exact publication, namespace checks and transport.

Running two independent Runtime/Warm sessions would duplicate retained bytes and
codec work and make publication state ambiguous. A new decoder or broad automatic
recipe search would change the experiment. The chosen implementation combines
existing candidates before Begin and makes the chosen base identity explicit.
Transparent TUN flow batching is a subsequent integration, not this change.

## Contract

`session::CombinedSession::new(&Context, PreparedPrediction, schema:&[u8])`,
`seed_checkpoint`, `seed_schema`, `send(context, Vec<u8>, transfer_id, warm_probe,
exchange)->CombinedStats`, invalidation and readiness/retention getters.
`CombinedStats` contains existing Stats and `session::CombinedSelection`:
candidates/pruned using existing schema-selection cost records, selected_mode,
exact_repeat_probe, checkpoint_ready and schema_ready. Readiness in the selection
receipt describes eligibility at selection, not newly granted authorization.

`ObjectSender::with_combined(&Context, PreparedPrediction, &[u8])` constructs it.
Add ObjectMode::Combined and ObjectSelection::Combined. Generic `new(Combined)`
rejects missing prediction. Generic target `seed` remains unsupported for this
profile; first data is a normal selected send. Auxiliary schema/checkpoint seeding
is explicit. The gateway config adds mode `combined`, requiring both checkpoint/
recipe and schema paths, with checkpoint/schema/send actions. Existing profiles
and packet service behavior stay compatible.

Before selection, compare input bytes with the acknowledged rolling base. An exact
repeat uses the existing private cached-ID Raw Begin-only probe, with same-offer
Raw recovery on receiver eviction. It does no residual construction or new target
hash. This receipt explicitly describes the repeat path, not a codec minimum.

Changed targets require a seeded checkpoint. Candidates: Raw, prepared Whole,
rolling SparseV1, fixed-prediction RuntimeV1, and SchemaZstdV1 only when schema
readiness is acknowledged. Ties: Raw, Whole, Sparse, Runtime, Schema. Evaluate
Runtime then Sparse, followed by the existing proven Whole/Schema pruning. Do not
add Prefix or a new codec. Compute target identity once; bind Runtime's offer to
the checkpoint ID, Sparse to the rolling ACK ID, Schema to its artifact ID, and
Raw/Whole to no base. Candidate costs retain the existing HTTP-body labels;
actual QUIC channel and captured IP bytes are measured independently.

Promote the owned target only after the existing client validates Published.
Auxiliary publication never replaces the rolling base. Attempted-exchange failure
invalidates all receiver readiness; local preflight failure preserves prior ACK
state. Namespace change poisons the session and drops retained prepared state.
No hidden retry, reseed, authorization by digest, or received code execution.

Retained quota includes prediction, rolling capacity and both codec allowances.
Use a justified 128 MiB sequential scratch allowance plus owned input capacity
under the existing 512 MiB session ceiling, not the old 384 MiB runtime allowance
that also covered freshly built contexts. Keep incumbent plus current candidate
only. Worst advertised retained ~288.6 MiB +16 MiB input +128 MiB scratch remains
below the ceiling. Outer callers still enforce shared multi-session/process limits.

## Measurement

Reuse the previous deterministic fixtures unchanged: 1,047,202 and 14,851,384
bytes; generated, 512-byte residual edit, repeat, unrelated opaque, opaque edit,
repeat. Arms: Whole-zstd, Warm, Runtime, Combined. Local and bidirectional
80 ms RTT/10 Mbps profiles; three rotated matched trials. Total24 sessions,
288 deliveries,336 operations:276send+12seed+24schema+24checkpoint. All sends
receive equal target-cache probe opportunity. Combined pays checkpoint+schema
setup and preparation, then six selected sends with no forced raw target seed.

Preserve the strict capture-drain correction, calibration, frozen-source/binary
identity, finite-queue costs, exact reopened bytes and original archive custody.
Require actual Combined RuntimeV1 generated transfers, SparseV1 opaque edits,
and Raw cache-hit repeats. Do not force its residual winner. Record all candidates,
setup/first/through-six and complete-session byte/CPU/time results, including
losses. Three descriptive trials do not establish p95, Internet WAN or energy.
Older long research campaigns become explicit label opt-ins; keep quick TUN
compatibility automatic. No new hardware, paid host, VM or large local corpus.
