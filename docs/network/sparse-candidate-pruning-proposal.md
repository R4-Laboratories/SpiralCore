# Sparse candidate pruning proposal

Status: **NOT_IMPLEMENTED / NOT_MEASURED**. This is a future candidate, separate from the running broad WAN campaign. No current result, throughput gain, memory reduction or energy saving is claimed.

Inspected source: `9dce4c57d1f41cef7365b297cd690849afce34e7`. The proposed implementation scope is the sparse encoder in `crates/spiralcore-transfer/src/delta.rs`, with qualification tests. It does not require a protocol, receiver, hashing, transport or cache-policy change.

## Observed work and proposed change

At the pinned source, `delta::encode` constructs a prefix/suffix candidate and an aligned candidate, packs both into serialized frame buffers, selects the cheaper packed candidate, and only then compares its cost with Raw (`delta.rs:68–77`). `pack` allocates frame bodies and copies every literal into them (`delta.rs:95–145`). Thus a losing candidate can allocate and copy much of the target before being discarded. This is a source observation, not a measured attribution of campaign CPU time.

Before packing each candidate, calculate a conservative cost lower bound from its existing operation vector. Skip materialization only when the candidate provably cannot beat Raw, or when the aligned candidate cannot strictly beat the incumbent. Continue using the existing `pack` implementation for every candidate that survives. Do not introduce another serialization implementation, sample-based heuristic, search index or speculative early network exchange.

## Lower-bound derivation

For the encoder's generated operations, whose lengths are positive, let:

- `B` be the sum of literal byte lengths.
- `C` be the number of original COPY operations.
- `L` be the number of original LITERAL operations.
- `F_min = max(ceil(B / MAX_CHUNK), ceil((C + L) / FRAME_OPS))`.

The pinned constants are `MAX_CHUNK = 61,440` and `FRAME_OPS = 256`. Each frame has a nine-byte sparse header. The existing `cost` function adds 128 bytes per frame. Each COPY occupies 13 bytes; each LITERAL has a five-byte prefix plus its data. Therefore:

```text
lower_bound = B + 13*C + 5*L + 137*F_min
actual_existing_cost >= lower_bound
```

The frame count cannot be less than either the total-literal limit or the operation-count limit. Splitting a literal creates additional operation prefixes; frame-size constraints may create additional frames. Neither makes the actual cost smaller than this bound. The empty operation vector has a zero bound; generated nonempty candidates have at least one operation. The implementation should use checked arithmetic and preserve the existing input-limit checks before planning.

Retain the existing selection rule:

```text
raw_cost = target_length + ceil(target_length / MAX_CHUNK) * 133
eligible sparse candidate: actual_existing_cost + 32 < raw_cost
```

The extra 32 bytes are the sparse Begin's required base identity. It is safe to skip either candidate when `lower_bound + 32 >= raw_cost`. It is also safe to skip the aligned candidate when `lower_bound >= incumbent_existing_cost`: equality cannot replace the simple candidate under the current strict comparison. Apply this only as a rejection test. A lower bound below a threshold is not proof that the candidate wins; it must still be packed and evaluated by the existing rules. Constants should be named and explained alongside the encoding they describe so later format changes cannot silently invalidate the proof.

## Required invariants

For every accepted input, the new encoder must produce exactly the legacy result: the same `None` decision or the same serialized frame bytes, frame ordering and operation count. Equal-cost ties must continue favoring the simple candidate. The candidate-generation scans and their existing limits remain unchanged.

Object and frame bounds, the per-frame literal and operation limits, total operation limits, and malformed-input behavior must remain intact. No resource-limit relaxation accompanies this optimization. The decoder's validate-before-mutate behavior, complete reconstruction and final object verification are outside the change.

`transfer_sparse_batched` must retain its 16 MiB sender ceiling, exact-repeat Raw behavior and full target/base object identities. Raw fallback must occur before the first exchange, without requiring a remote base. A missing remote base must remain an error; there is no retry with another offer under the same transfer ID. Encoding and fallback selection remain inside sender elapsed time. Uploaded frames, replies, body bytes and selected mode must remain identical to the legacy sender for the same inputs and replies.

## Qualification before measurement

Retain the pinned encoder as a test-only legacy oracle. Compare `None` versus recipe and, for recipes, every frame byte and operation count; successful reconstruction alone is insufficient. Independently apply every selected recipe and compare all target bytes, including NUL and trailing-zero content.

The fixed matrix must include:

- Empty base/target, exact repeats, single-byte inputs, and lengths around 31/32/33 bytes.
- Prefix and middle insertion, append, deletion, truncation, localized edits, scattered edits, unrelated replacement, periodic content and already compressed content.
- Literal totals and frame boundaries around 61,439/61,440/61,441 bytes; mixtures of COPY and LITERAL operations that hit the serialized-body limit before the literal limit.
- Operation counts around 255/256/257 per frame and 8,191/8,192/8,193 overall, including extra operations introduced by literal splitting.
- Equal candidate costs, the strict Raw eligibility boundary, cases where the simple candidate wins, cases where aligned wins, and cases where both lose to Raw.
- A small deterministic constructed shift where the aligned plan is mostly literal and the prefix/suffix plan is compact, plus fixed-seed random edits. Do not select fixtures after seeing timing results.

Property tests over the bounded generated operations should verify `lower_bound <= legacy packed cost` whenever packing succeeds. Retain sender regressions for pre-exchange Raw fallback, missing-base/no-reoffer behavior, warm-probe cache hits, strict replies and full upload accounting. Observe a meaningful failing regression before implementation and retain the exact qualification commands and outcomes.

## Proposed predeclared acceptance

These are prospective acceptance criteria, not measurements. Freeze the final implementation, test matrix, fixture seeds, release compiler/features and paired schedule before collecting performance data. Retain every observation and failure. A failed gate is a negative result; do not replace fixtures or change the threshold afterward.

The first experiment should stay at the current 16 MiB sender scale. Use three primary cells: a 4 KiB prefix insertion, a 4 KiB middle insertion, and an unrelated full replacement. Insertion bases are shortened so each target remains within 16 MiB. Before timing, qualification must establish which candidate is pruned and identical legacy/new outputs for each fixed fixture. If a registered fixture does not exercise the intended rejection, record that qualification failure instead of substituting a fixture after timing starts.

Run ten paired old/new process comparisons per primary cell on the same host: five AB (legacy then candidate) and five BA (candidate then legacy), interleaved in a fixed AB/BA schedule. Use fixture creation and verification outside the timed encoder call and equal input ownership. Record process CPU and wall time, exact outputs, mode, frame count and serialized byte count. Compare the same encoder call, not a cached result against fresh work. The proposed CPU acceptance is a paired geometric mean of `legacy CPU / candidate CPU >= 1.10` and at least seven candidate wins out of ten **in each primary cell**. Record wall time without claiming a WAN improvement from this local experiment.

Use a separate, untimed allocation diagnostic to measure peak live bytes and cumulative allocation attributable to encoder-owned frame buffers, excluding the identical caller-owned base/target inputs. Count actual allocated Vec capacity, allocation growth/reallocation overlap and frees, not logical serialized length. Measure from encoder entry through return while the returned recipe remains live; record its retained capacity separately and include its eventual drop in a separate end-to-release lifetime. Use the same boundaries for legacy and candidate. Preserve the instrumentation source and its overhead/attribution limitations. The proposed memory acceptance is at least a 50% reduction in peak encoder-owned frame-buffer bytes in each primary cell, with zero serialized bytes materialized for a candidate rejected by the bound. Avoid inferring process RSS or total host memory savings from these buffer measurements.

Run the same ten-pair schedule on controls: exact repeat, a localized 32-byte edit, scattered edits where aligned wins, and periodic structured content. Require identical outputs and no more than a 5% CPU regression in any control, using paired geometric means. Tiny-duration or zero-resolution observations require a predeclared repeated-call batch with equal work; do not discard them selectively. If correctness, memory, CPU or control gates fail, retain the result and do not claim the candidate meets acceptance.

Only after those gates pass should a separately registered WAN comparison evaluate whether sender work reduction improves end-to-end time under equal cache seeds and receiver policy. The current campaign's source and results must remain distinct. CPU time is not a joule measurement; no energy claim follows from this experiment.

## Later 64 MiB applicability

The lower-bound screen scales with the already bounded operation vectors and adds no target-sized buffer or index. It may help future 64 MiB work by avoiding discarded target-sized literal buffers, especially when both legacy candidates are materialized before fallback. The actual peak and reduction remain unmeasured.

This proposal does **not** raise the current batched sender's 16 MiB ceiling. A 64 MiB milestone separately needs validated batch/frame scheduling, receiver reservations, object/cache quotas, temporary-memory accounting and durability/recovery qualification. Caller-owned base/target bytes, the winning encoded recipe, transport copies and receiver state still consume memory. Even if pruning passes at 16 MiB, repeat allocation and correctness qualification at the larger scale before claiming it is safe or effective there.
