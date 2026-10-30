# Pre-Begin byte-minimizing codec selection

Date: 2026-10-29. Status: accepted design for the user-authorized next milestone;
implementation and measurement are separate receipts. Starting revision:
`eb4feb930d6910466dbc3dc90bfa3cead51f6b5e`. This decision does not modify the
frozen [broad WAN results](broad-wan-results-2026-10-29.md).

## Context

The twelve-case WAN campaign demonstrated a selection problem. Sparse encoding
used 600 body bytes for a localized warm update, but its cold fallback sent the
entire structured object. Ordinary chunked zstd reduced that cold object's body
traffic by about 93%. Fixed codec selection leaves this benefit unused. Extra
computation is acceptable when it saves bytes, but its CPU and elapsed cost must
be recorded. No existing result establishes the cost of evaluating all codecs.

## Decision

Add an opt-in bounded sender that selects the least expensive **HTTP request plus
reply body** representation before the first protocol exchange. Preserve existing
explicit-codec APIs and receiver behavior. The new entry point is
`client::transfer_adaptive_batched(target, base, transfer_id, warm_probe, exchange)`;
`base` is optional caller-owned immutable local bytes. It returns existing
transfer statistics plus selection telemetry. `selection.rs` owns candidate
preparation and scoring; `client.rs` reuses the winning encoded Data bodies.

```text
immutable target + optional local base
  -> bounded candidate construction -> exact successful-miss body cost
  -> one selected Offer and its already-encoded Data bodies
  -> existing batched sender -> existing authorized verified durable publication
```

Evaluate Raw, ordinary independent-chunk zstd, sparse COPY/LITERAL, then whole-base
zstd prefix. The last two require a supplied base. Strictly smaller cost replaces
the incumbent; ties favor this order. Ordinary zstd retains existing level-one
60 KiB chunks and per-chunk Raw fallback. Sparse and prefix retain their existing
formats, parameters and limits. This is a minimum over these configured candidate
engines, not all possible encodings or all zstd settings. Do not introduce a new
wire mode for the selector or include DAG construction in this candidate set.

Construct one candidate at a time, retain only the incumbent and the candidate
being evaluated, and destroy losing buffers promptly. Send the winning encoded
bodies directly; do not rerun an encoder after selection. Raw can borrow the
target until framing is needed. Do not expose a mutable public prepared payload
that could become inconsistent with its object ID. Selected Raw/zstd offers have
no base identity. Selected sparse/prefix offers bind the exact local base bytes.

For `base == target` with `warm_probe=true`, a separate exact-repeat fast path
may send a Raw identity probe without constructing candidates. Its telemetry must
set `exact_repeat_probe=true`, with unevaluated candidates explicitly distinguished from
failed or losing candidates. This fast path does not claim to minimize a full
transfer after an unexpected receiver miss. Without `warm_probe`, all transmitted
Data would count even if Begin hits; therefore identical local bytes alone must
not force a Raw whole-object upload in the adaptive path.

## Exact scoring and its boundary

For a successful target-cache miss with no repair and one request batch, common
bidirectional body cost is 368 bytes:

| Element | Bytes |
|---|---:|
| Raw/zstd Begin frame | 102 |
| Ready frame | 66 |
| Finish frame | 56 |
| Published frame | 88 |
| Four frame-length words | 16 |
| SCWBAT01 request header | 12 |
| SCWREP01 reply header, including receiver metrics | 28 |

A base-bound Begin adds 32 bytes. Each Data body contributes its own byte length
plus 128 bytes: request frame header 56, Accepted frame 64, and two four-byte
length words. A separate Begin probe followed by a miss adds one 40-byte envelope
pair. Thus, for admissible one-data-batch candidates:

```text
projected_body_bytes = 368 + (32 if base-bound else 0)
                     + sum(data_body_length + 128)
                     + (40 if warm_probe else 0)
```

Use checked arithmetic and verify encoded bodies, frame counts and batch limits
before exchange. Validate the score against actual `wan::HttpClient` request and
response metrics, including empty inputs and probe misses. The 28-byte reply
header matters: counting both directions as 12-byte request envelopes omits
16 bytes per exchange. The ordinary `Stats` request/response fields count protocol
frames only and cannot by themselves prove this HTTP-body total.

The estimate is conditional on a successful miss. An early Published, denial,
missing base, storage error, or shortened response has a different actual cost.
Record actual transport metrics separately. HTTP headers, TLS, TCP/IP, retransmits
and relay-leg traffic are outside this score. Equal body savings do not establish
equal whole-path savings or latency. A repeat Raw probe with Published costs
238 body bytes under the current adapter.

## Resource and failure contracts

Keep both input slices at or below the existing 16 MiB batched ceiling. Preserve
64 KiB complete-frame, 512-frame/20 MiB batch, 8,192 sparse-operation and prefix
encoded/window limits. Reject oversized input before candidate work or exchange.
All selected candidates must fit the declared batch schedule. At this scale Raw,
chunked zstd need at most 274 Data chunks; prefix's compression bound allows 275.
Sparse flushing is bounded
by literal and operation limits; the construction remains below the batch frame
ceiling, and runtime checks must enforce that rather than relying on this prose.

Sequential retention bounds additional encoded candidate buffers; it is not a
process-RSS or total allocator bound. The prefix encoder has its own compression
context, and framing/HTTP encoding add copies. Do not claim that the receiver's
resident quota also reserves sender memory, or that these host limits suit a small
MVM. No size/quota increase or codec-allocation optimization belongs in this change.

Candidate absence such as sparse `None` is a normal recorded outcome. Propagate
unexpected encoder failures; do not disguise malformed/unsupported input or
resource failure as an ordinary losing candidate. The same target identity and
original length govern every encoding. Existing receiver authorization, cache
scope, hash validation, admission and durable publication remain mandatory.

Having local base bytes does not prove remote possession or permission. If the
selected base-bound transfer receives Missing, Unauthorized, epoch/revocation or
any other error, return it without changing mode or issuing another Offer under
the same transfer ID. An authorized caller may decide a new transfer separately.
Do not add base-discovery probes or cache-membership disclosure. Preserve existing
uncertain-commit behavior after a lost Published reply or post-rename error.

## Telemetry and alternatives

Return the selected mode, the exact-repeat flag, each eligible candidate's projected
body cost, selected cost and Raw comparison cost. `candidates` is ordered by tie
priority. Without a base, sparse/prefix are not evaluated; with a base and no repeat
shortcut, an omitted sparse entry means the bounded encoder returned `None` (no
recipe cheaper than Raw). Unexpected encoder errors propagate. A repeat shortcut
lists only Raw. Count all selection,
discarded encoder work, identity hashing and send work inside sender elapsed time.
An outer process-CPU measurement includes candidate work as well. Any separate
selection timer is a subset, not an amount to add again to whole-operation time.
Do not make correctness depend on noisy clocks or expected RTT.

| Approach | Benefit | Tradeoff and disposition |
|---|---|---|
| Exhaustive bounded candidates | Exact comparison among existing engines; simple falsifiable byte contract | Pays losing-encoder CPU and bounded extra buffers; selected for first milestone |
| Sampling or entropy/edit heuristics | Can avoid expensive encoders | Can miss the best byte result; defer until measured against the exact selector |
| Sparse-first proof-bound pruning | Can avoid work while preserving a proven decision | Requires valid bounds across codecs; separately review after this selector exists |

The existing [sparse candidate-pruning proposal](sparse-candidate-pruning-proposal.md)
is a separate optimization. Implementing this selector does not establish its
CPU or memory benefits. The selector itself is opt-in until its measured tradeoffs
are reviewed. Legacy fixed-arm WAN plans and their recorded results remain intact.

## Qualification and later integration

Require exact reopened outputs, selected-cost agreement with actual HTTP body
traffic, deterministic ties, all four selected modes, no-exchange rejection of
oversized input, missing/revoked/wrong-scope bases, malformed replies and no
mid-transfer fallback. Compare unchanged explicit-codec senders on identical
fixtures and receiver warming policy. Record Raw seeds separately.

Measure local CPU and elapsed through durable publication before a separately
registered WAN campaign. Local byte/cost qualification is not new WAN latency
evidence. Record regressions even if the selector wins bytes. CPU is not energy,
and subtracting CPU from elapsed does not produce network-only time.

MVM remains a later adapter to the canonical encrypted vsock mailbox, including
small multi-MVM/orchestrator lifecycle, identity, capability/epoch isolation,
bounded queues and backpressure. No MVM port or interface is invented here.
