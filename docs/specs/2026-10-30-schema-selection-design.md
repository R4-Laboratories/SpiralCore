# SC-OPT-014: byte-first schema selection against whole-object zstd

Approved by the user; baseline `3fd3199ef46984ef889b800630f721c2518af0e3`.
Implement the smallest complete transfer among Raw, equally configured
whole-object zstd and compressed schema. Preserve original bytes, existing
formats, authorization and durable commit. Existing adaptive selection remains
unchanged. This is a greedy decision for the current transfer, including any
required seed, not a globally optimal prediction of future traffic.

## Codec and receiver

Add explicit `Mode::ZstdWholeV1 = 9`, with no manifest or artifact/base ID. It
encodes one bounded, checksummed zstd frame over the original bytes using the
same level 3, window 25, LDM, single-thread, content-size and checksum parameters
as compressed schema. Reuse the strict whole-frame validation: exact content
length, one frame, no dictionary field/reserved bits, window at most 32 MiB,
checked decompression and the unchanged final whole-object identity check.
Objects remain limited to 16 MiB. Existing modes retain their bytes and behavior.

Expose `zstd_whole::PreparedEncoder` and `PreparedDecoder`, plus free wrappers.
Both new Whole and existing Schema decoders use the same opt-in, eight-entry
cache, 5 MiB admission charge, namespace/mode isolation, pinning and fallback
policy. Whole uses a distinct mode key with a fixed no-artifact identifier;
this is not authorization. Schema still rereads/verifies its admitted artifact
on Begin. A selector session may retain two decoder entries; record that cost.

## Prepared selection contract

`schema_selection::PreparedSelector::new(artifact)` owns a prepared whole encoder
and prepared compressed-schema encoder. It exposes `artifact()`, `artifact_id()`,
`retained_bytes()` and conservative `retained_charge()`.

`prepare(&mut self, target, schema_ready: bool, warm_probe: bool)` returns a
`PreparedTransfer` holding a private immutable offer/selected payload and a
read-only `Selection` receipt. Its `candidates` are `CandidateCost` values with
`mode`, `target_http_body_bytes`, `setup_http_body_bytes` and
`total_http_body_bytes`; the receipt also has `selected_mode` and
`needs_schema_seed`. `PreparedTransfer::selection()` returns that receipt.

Prepare encodes both compressed candidates sequentially, retains only the
current winner plus the candidate being evaluated, hashes the target once, and
retains the winning encoded bodies without recomputation. Raw is represented
without constructing its full frame list unless it wins. Ties prefer Raw,
Whole, then Schema. No heuristic or destination-capacity error excludes a
candidate that could save bytes. All candidate and hash work must be timed.

Costs cover both directions of the existing successful-miss HTTP batch bodies.
At the current bounded one-batch ceiling, the common cost is 368 bytes, plus
40 for a separate warm-probe miss. Raw adds its payload and 133 bytes per Data;
Whole adds compressed payload and 134 bytes per Data; Schema adds the same
per-Data cost and 32 bytes for the artifact ID. Add Schema to the existing
cost helper's base-ID cases without changing prior-mode costs. Cache hits and
errors have different observed costs; report actual transport counters too.

If `schema_ready` is false, Schema additionally pays the full separate Raw
artifact transfer with no warm probe: 670 body bytes for the current 169-byte
artifact, computed generally from its length. Only a strict effective-cost win
requests a seed. The caller sends that seed, marks readiness only after its
commit acknowledgment, then consumes the plan with
`client::transfer_schema_selection_batched(plan, transfer_id, exchange)`.
Readiness is caller knowledge, never permission; a missing/revoked artifact
still fails at the endpoint without an implicit retry or new offer. Seed and
target transfer IDs must differ. Selection, actual seed and send are disjoint
measured phases; the consuming API does not encode or hash the target again.

The equally tuned control is
`client::transfer_prepared_zstd_whole_batched(encoder, target, id, warm_probe,
exchange)`, with exact Raw fallback. Sender context construction is separately
charged: the selector owns two contexts, the control one. Existing compressed
schema remains a paid-upfront-seed reference arm, clearly labeled as such.

## Qualification and real-WAN measurement

Use RED/GREEN tests for Mode 9, strict hostile frames, exact reconstruction and
restart, cache authorization/resource/recovery behavior, repeated prepared-byte
parity, deterministic ties, cold setup thresholds, missing seed and measured
HTTP-body equality to every candidate receipt. Keep compatibility tests.
Run focused changed-surface coverage with exact current objects; do not claim
branch coverage without instrumentation.

The persistent WAN comparison retains the existing ten-target sequence, two
sizes, 30 counterbalanced paired trials and bounded sharding. Compare the paid
schema reference, Whole/Raw and selector; seed the selector only when selected.
Record all context construction, preparation, seed and target bytes/CPU/wall;
the charged target includes its actual seed once, and session totals must not
add it again. Both Whole and Schema decoder contexts are reusable under the
same policy. Verify every actual seed and target by exact regenerated readback
after stopping receivers. Retain drift/unrelated losses and all failures. HTTP
body bytes, sender CPU, receiver-handler CPU and durable elapsed are separate
claims; no packet-total, pure-wire, energy, MVM or compliance inference follows.
