# Verified warm-object codec selection

Status: accepted direction under the user's instruction to integrate warm reuse,
reduce unnecessary encoding, preserve minimum bytes and exact fidelity, and
measure CPU and real WAN completion. Base: `531df9ee7cf721a0b706a0d741627c468792dede`.
Implementation starts after the parent commits this design and its plan.

## Decision and alternatives

1. **Exhaustively encode Raw/Whole/Sparse/Schema:** straightforward reference
   behavior and exact minimum, but it performs expensive codecs even when their
   format overhead already exceeds a good sparse recipe.
2. **Sparse-first with proven structural lower bounds — selected:** obtain an
   incumbent from the existing delta encoder, skip a candidate only when it
   cannot win, and otherwise perform its unchanged full encoding. This preserves
   the reference decision without guessing compression ratio. Some 1 MiB cases
   still pay Whole encoding; correctness takes priority over avoiding that work.
3. **Capacity-limited `compress2` or a heuristic Sparse shortcut — excluded:**
   recognizing `dstSize_tooSmall` does not prove that the normal uncapped output
   exceeds that capacity. Pinned zstd1.5.7 uses conservative header-space checks
   and capacity-dependent raw-block fallback. The prior
   [source analysis](2026-10-30-sender-cpu-design.md#prefix-structural-bound)
   records these cases. This milestone performs no aborted/capped compression
   and makes no unsupported repeated-context or output-parity claim for it.

## Additive API and flow

Keep the existing schema selector's `prepare` and three-candidate receipt
unchanged. Add in `schema_selection.rs`:

```rust
PreparedSelector::prepare_warm(
    &mut self, target: &[u8], base: Option<&[u8]>,
    schema_ready: bool, warm_probe: bool,
) -> Result<PreparedWarmTransfer>;
PreparedWarmTransfer::selection(&self) -> &WarmSelection;
```

`WarmSelection` contains `candidates: Vec<CandidateCost>` (existing exact
target/setup/total fields), `pruned: Vec<PrunedCandidate>`, `selected_mode` and
`needs_schema_seed`. Each pruned receipt contains `mode`,
`target_http_body_lower_bound`, `setup_http_body_bytes`, and
`total_http_body_lower_bound`; it is never represented as an encoded size.
An absent Sparse candidate means no supplied base or the existing bounded
encoder returned `None`, not a fabricated exact cost. Sort receipts in tie order.

The parent adds `client::transfer_warm_selection_batched(plan, id, exchange)`.
The immutable plan owns the selected Offer and payload; send consumes them
without another encoding or identity calculation. Flow:

```text
bounded inputs -> Raw cost -> optional Sparse -> eligible Whole -> eligible Schema
               -> immutable winning plan -> optional admitted schema seed -> send
```

Use tie priority **Raw, Whole, Sparse, Schema**, regardless of evaluation order.
This preserves the old Raw/Whole/Schema relative order. A lower bound prunes a
candidate only if `(bound, priority)` cannot beat the incumbent pair. A Whole
bound tying Sparse must still be encoded because Whole has earlier priority.
Only the incumbent and current candidate need coexist. Hash the target once
and the borrowed base only if Sparse wins; do not introduce an immutable-base
copy/cache. Enforce existing 16 MiB input limits before any codec work.

## Bounds and accounting

Let `W = 40` for a separate warm-probe miss, otherwise zero. The checksummed
zstd frame minimum is `L(N) = 10 + 3*max(1,ceil(N/131072))`, using the existing
reviewed `zstd_prefix::minimum_encoded_size`. One Whole Data frame costs another
134 body bytes, giving the safe lower bound `368 + W + 134 + L(N)`.
At 16 MiB this is **896+W**, above a **600+W** localized sparse update, so Whole
can be skipped. At 1 MiB the bound is **536+W**, so full Whole encoding remains
necessary whenever that candidate could win.

The compressed schema stream always has a nonempty SCVAL001 intermediate, so
its checksummed frame is at least13 bytes. Schema's base identity and one Data
frame give **547+W** target-body bytes as a conservative minimum. If the artifact
is not ready, add its exact separate Raw seed body cost, with no warm probe.
The existing169-byte artifact therefore costs at least **1217+W** in total.
Warm schema remains eligible whenever its lower bound could beat the incumbent;
do not confuse a missing schema with a missing prior-object base. Exact encoded
costs use the existing complete bidirectional HTTP-body cost function.

Sparse is eligible only when the caller supplies a base already expected to be
admitted remotely. `base=None` is the cold path and disables Sparse; this API
does not automatically seed a prior object. `schema_ready` remains a caller hint,
not permission. A selected cold schema requires the existing explicit paid seed
before consuming its plan. Missing/revoked/evicted bases or artifacts, policy
changes and transport failures remain errors without hidden retry/reoffer.
Receiver formats, scoped authorization, resource quotas and durable exact
length/digest verification remain unchanged. Target-cache hits may transfer
less than the successful-miss receipt predicts; no special repeat shortcut
weakens this new selector's successful-miss minimum contract.

## Qualification and measurement

Compare selected mode, complete selected frame bodies and exact cost with an
exhaustive four-candidate reference using the unchanged encoders. Include Raw,
Whole, Sparse and Schema winners; cold/warm schema and missing base; length
changes, binary/lexical inputs, ties, 128 KiB and16 MiB boundaries; repeated
prepared-context calls after rejected inputs. Check each pruned lower bound
against the reference's actual cost. Whole/Schema encoder errors still propagate
when evaluated; static skips do not invoke those encoders. Verify false-warm
hints, revoked/missing state and exact reopened publication through integration.

Reuse the SC-WAN-005 fixtures/receiver machinery with three arms: forced Sparse,
new selector and prepared Whole. Include actual selector schema fitting and
context setup in its lifecycle, plus identical paid Raw base seeds for all arms.
Use six shards of five trials per size:30 paired trials at1/16MiB,180 sessions,
**360 seed/update transfers**,510 MiB payload/shard, plus56 MiB allowance and
60×4KiB headers = **566.234375 MiB**, below768 MiB and existing1 GiB quota.
The registered localized fixture must select Sparse/600 body bytes in both
Sparse arms; a surprise is retained as a qualification failure, not discarded.
All-codec winner fixtures belong to tests, not post-hoc changes to this campaign.
Local qualification uses three trials per size/three arms:36 transfers.

Record evaluated/pruned candidate receipts outside timing; include every actual
comparison, encoding, hashing and send inside sender CPU and completion timing.
Report sender/receiver-handler CPU, body bytes, durable elapsed and paid lifecycle
against Whole and forced Sparse, with exact stopped-store readback. Preserve
negative outcomes. No pure-wire-time, energy, MVM, FIPS or general WAN claim.
