# Shared-core SmallV1 resource profile

Proposed additive local resource contract, reviewed before implementation.
General constructors and their codec settings remain unchanged. SmallV1 uses
the existing frames, artifacts, authority, object IDs and exact decoded bytes;
compressed bytes may differ under its explicit bounded encoder settings.

```rust
pub enum ResourceProfile { General, SmallV1 }
impl ResourceProfile {
    pub fn operation_charge(self) -> Result<u64>;
    pub fn max_target_len(self) -> usize;
    pub fn max_artifact_len(self) -> usize;
    pub fn max_prediction_len(self) -> usize;
    pub fn receiver_limits(self) -> Limits;
}
```

SmallV1 admits application targets <=16KiB and schema/checkpoint artifacts and
runtime predictions <=64KiB. Input Vec capacity is checked independently of
length before candidate preparation. Auxiliary Raw artifact publication may be
64KiB; a carrier must enforce its separate application-purpose16KiB bound
before core dispatch. Local resource selection is not remote authorization.
Existing negotiated object limits still apply. `Capabilities::small_v1()`
selects existing capability profile field2 (General remains1), max object64KiB
and the bounded Raw/Sparse/Prefix/Runtime/Schema/SchemaZstd/Whole modes.
SmallV1 excludes legacy streaming Zstd and retains the one-shot Whole and
Prefix zstd modes. SmallV1 Runtime selection uses Whole for its independent
compression candidate when the checkpoint and prediction offer little reuse.
Both authenticated/pinned peers must select2; mixed1/2 refuses Unsupported,
without fallback. Capability/frame layout and codec versions do not change.
Constructors require the explicit local ResourceProfile to match that agreement.

`operation_charge()` returns the conservative envelope to reserve in the ONE
shared outer ledger before constructing sender/prediction/receiver state. The
SmallV1 envelope is <=8MiB for one sender plus one receiver/active operation,
including metadata, bounded replay and transient overlap. It is not RSS, an
independent per-endpoint quota, or permission to retain unlimited histories.
Receiver Limits/history/cache ceilings must fit this declared envelope; SmallV1
initially refuses enabling prepared-decoder caching. The outer reservation
remains held through destruction of all retained state. General cannot claim
this8MiB envelope; its preexisting conservative charges remain authoritative.
SmallV1 receiver_limits fixes one active attempt,32 retained states,64 publication
records, no graph/decoder cache and disabled eager cold graph construction.
Passing larger resident/state/history settings to a SmallV1 endpoint refuses
before filesystem mutation; disk cache/namespace defaults are bounded8MiB.
This slice does not retire acknowledged terminal histories or evict published
objects on progress. Consequently32 states/64 publications can saturate after
roughly512KiB/1MiB of unique16KiB blocks (seeds also count). Sustained FlowMux
requires its separate persistent128-block and replay-safe retirement/eviction
gate; no production sustained-readiness follows from bounded admission alone.

## Source-qualified zstd workspace

Use safe zstd-safe CCtx/DCtx methods only; retain transfer forbid(unsafe_code).
SmallV1 requires the existing exact bundled zstd1.5.7 source qualification and
64-bit supported ABI. Unqualified/system/replaced sources return Unsupported
before context creation. sizeof is an additional drift guard, never the proof
used to admit an allocation.

Use one-shot compression, NbWorkers0, WindowLog17, HashLog14, fast strategy,
MinMatch4, LDM disabled, content size/checksum enabled and dictionary-ID field
disabled. No optimal parser, chain table, row matcher, streaming buffers,
external sequence producer or trained dictionary context is introduced by
this profile. Schema uses its existing prepared SCVAL001 transform; its maximum
input is `schema::encoded_cap(16KiB)=98,352` bytes. Prefix refers to the bounded
immutable checkpoint without copying it into a trained CDict.

The Rust model bounds the qualified C `ZSTD_estimateCCtxSize_usingCCtxParams_internal`
terms BEFORE creation/use: fixed context/entropy/block/slack allowance256KiB,
hash table4*2^14, and <=4*n token/sequence bytes plus4KiB alignment allowance,
where n is the relevant bounded one-shot source length. Source reset frees the
old workspace before allocating its replacement. No LDM/chain/optimal or
streaming terms apply. Both persistent Whole/Schema contexts are charged;
Runtime's sequential conventional candidates use the same bounded factory.
One-shot decoder fixed structures/reference-prefix overhead are bounded by
512KiB; no frame-window-sized streaming allocation is used. Existing schema
prepared512KiB and parser limits remain charged, not reduced from observed RSS.
The8MiB inequality must explicitly include sender candidate/frame copies,
receiver ledger/scratch/encoded/output, runtime state and prediction overlap.
Any unproved term must conservatively increase the declaration or refuse;
do not silently weaken the ceiling to fit a carrier budget.

Pinned maximum-envelope arithmetic (bytes; all additions checked in code):

| Sender term | Bytes |
| --- | ---: |
| Whole K(16,384) |397,312|
| Schema K(98,352) |725,184|
| Prepared schema allowance |524,288|
| Last base + four64KiB ownership/metadata allowances |278,528|
| Candidate/framed-client/runtime-conventional scratch |2,097,152|
| **Sender subtotal** |**4,022,464**|

| Receiver term | Bytes |
| --- | ---: |
| Existing active ledger/scratch allowance |2,097,152|
| Four encoded/intermediate/output copies, compressBound(98,352)=98,751 |395,004|
| Four artifact/base copies + two prediction-cap overlaps |393,216|
| One-shot decoder allowance |524,288|
| Prepared schema allowance |524,288|
| Bounded runtime state256KiB + metadata64KiB |327,680|
| **Receiver subtotal** |**4,261,628**|

Sum **8,284,092 <8,388,608**. This covers ONE sender+receiver pair; two
duplex engines/four endpoints require two shared reservations. Payloads kept
after a pair drops still require their separate parent ownership charge.
Carrier/TLS/global-process mappings and storage capacity have their own admitted
budgets; this is a codec/payload envelope, not total process or VM RSS.

The fixed encoder allowance bounds the64-bit CCtx structure (<64KiB), two
compressed-block entropy states (<32KiB), temporary workspace (<9KiB), alignment/
redzones and remaining fixed bookkeeping with substantial slack. Fast/minMatch4
has no chain/hash3/row/optimal tables; token storage is n literals plus <=n/4
8-byte sequences and three one-byte code arrays, below4*n. Context reset frees
the prior workspace before replacement. Rust scratch2MiB includes the bounded
delta operation vectors (<=8195 entries, geometric capacity<=16,384 entries at
24 bytes each on the qualified ABI), incumbent/candidate frames, SCVAL001 intermediate,
compressed output and batched client copies. Small seeds are bounded64KiB and
targets16KiB before any of those operations. No post-hoc RSS/sizeof observation
was substituted for these source bounds.

Primary source: [zstd1.5.7 compression implementation](https://github.com/facebook/zstd/blob/v1.5.7/lib/compress/zstd_compress.c),
[context structures](https://github.com/facebook/zstd/blob/v1.5.7/lib/compress/zstd_compress_internal.h),
[safe CCtx API](https://docs.rs/zstd-safe/7.3.0/zstd_safe/struct.CCtx.html).
The bundled source digest, not these moving descriptions or a version string,
is the executable qualification gate.

## Additive constructors and receiver checks

Add `new_with_resource_profile` to Whole/Schema prepared encoders, PreparedSelector,
WarmSession, RuntimeSession and PreparedPrediction; Combined's corresponding
constructor accepts its existing SelectionPolicy plus ResourceProfile. Existing
constructors delegate General and preserve original behavior. Runtime client/
selection helpers thread the profile through all evaluated candidates.

ObjectSender gains resource-profile variants of its existing constructors;
ObjectEndpoint gains `open_with_resource_profile` carrying one immutable
profile through the core Endpoint. Receiver Begin admission bounds application
and Raw-artifact shapes before staging. Runtime reserves only the profile's
prediction cap and bounded state, then rejects setup's declared length or
recipe output shape over that cap BEFORE runtime evolution/output allocation.
Existing dual-ID/final output verification, signed authority, publication and
failure state remain unchanged. All arithmetic is checked; errors use existing
Limit/Unsupported/Malformed equivalents. No hidden retry or alternate codec.

Exact facade additions (existing constructors keep General):

```rust
ObjectSender::new_with_resource_profile(mode, context, schema, negotiated, resources);
ObjectSender::with_runtime_resource_profile(context, prediction, negotiated, resources);
ObjectSender::with_combined_resource_profile(context, prediction, schema, policy, negotiated, resources);
ObjectEndpoint::open_with_resource_profile(root, limits, authority, negotiated, resources);
PreparedPrediction::new_with_resource_profile(checkpoint, recipe, resources);
```

## Required focused gates

Missing public profile/constructor RED, then actual Whole/Warm/Runtime/Combined
transfers with exact readbacks under one declared8MiB admission. Prove boundary
input/capacity/artifact/recipe refusals before exchange/state evolution; unknown
source profile refuses rather than falling back. Use equal, edited, unrelated,
schema-fit/drift, short600-byte and existing large General regressions. Compare
before General and after Small fixture bytes, setup/send CPU/wall and actual
context/buffer high water separately from declared admission; retain every byte
or time regression. No energy, WAN, native-MVM or universal speedup inference.
