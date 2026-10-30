# SC-OPT-007: proven rejection before sparse frame allocation

Date: 2026-10-30. Status: approved design scope; implementation and measurements
are not established by this document. Baseline:
`cc0199ffd255b9e2e002afe8c96f10462a3e474f`.
This implements the existing [sparse-pruning proposal](../network/sparse-candidate-pruning-proposal.md)
after the user authorized reducing candidate-encoding work while preserving
minimum-byte decisions. MVM, WASM, larger clients and new WAN qualification remain
separate milestones.

## Context and decision

`delta::encode` generates two bounded operation lists: prefix/suffix sharing and
aligned sharing. It serializes both candidates before selecting by encoded cost,
then rejects an inefficient winner against Raw. The adaptive sender additionally
evaluates ordinary zstd and whole-base zstd prefix. SC-OPT-006 recorded the extra
CPU of evaluating losing codecs. This change removes only provably unnecessary
**sparse frame construction**; it does not skip prefix compression or promise to
remove the adaptive sender's entire excess cost.

Insert a checked lower-bound rejection before each sparse `pack` call. Keep
operation generation, its scans/limits, `pack`, decoder, public API and all wire
bytes unchanged. Surviving candidates still use the existing serializer and exact
cost comparison. Raw and incumbent rejection are proofs that a candidate cannot
win, not predictions of its compressed size.

```text
unchanged bounded operation generation
  -> checked cost lower bound
  -> reject only a proven loser, or call existing pack unchanged
  -> existing strict cheaper-candidate selection and Raw eligibility check
  -> identical Recipe frames/operation count, or identical None
```

## Lower-bound proof

The proof domain is the encoder's positive-length generated operations. Let `B`
be total literal bytes, `C` the number of COPY operations, and `L` the number of
LITERAL operations. Define:

```text
F_min = max(ceil(B / 61_440), ceil((C + L) / 256))
LB    = B + 13*C + 5*L + 137*F_min
```

Every copied interval contributes a 13-byte operation. Every original literal
contributes at least a five-byte operation prefix plus its `B` bytes. Splitting a
literal adds prefixes; it cannot remove any required data or original operation.
Each frame contains at most 61,440 literal bytes and 256 serialized operations.
There are at least `C+L` serialized operations, so the number of frames is at least
`F_min`. Each frame contributes its nine-byte sparse header plus 128 bytes in the
existing `cost` accounting. Therefore every successfully packed candidate satisfies
`actual_cost >= LB`.

The empty list has bound zero. Arbitrary zero-length synthetic operations are
outside the positive-operation premise because `pack` skips them: tests must
either exclude them explicitly or use a bound implementation that ignores them.
Do not assert the formula over counted zero-length operations. A packing failure
from the existing operation limit still yields no candidate; a low bound does not
prove feasibility or eligibility.

The unchanged comparison is:

```text
raw_cost = target_len + ceil(target_len / 61_440) * 133
sparse is eligible iff actual_cost + 32 < raw_cost
```

The 32 bytes are the required sparse base ID in Begin. Common successful-transfer
costs cancel. This is the sparse encoder's existing comparison, not a new full
HTTP-body accounting formula.

Consequently:

- If `LB + 32 >= raw_cost`, skip that candidate. Its actual cost cannot satisfy
  the strict Raw eligibility comparison.
- For the aligned candidate, if an incumbent exists and `LB >= incumbent_cost`,
  skip it. Its actual cost cannot strictly beat the incumbent; equality must
  preserve the prefix/suffix candidate.
- Otherwise call `pack` and apply the original strict comparison and final Raw
  filter. Never treat a lower bound as the candidate's actual cost.

Use checked arithmetic for counts, lengths, multiplication and additions. Keep
the existing object-limit rejection before planning. The proof follows the current
format constants; name and explain them so a future format change cannot silently
invalidate rejection.

One historical proposed test condition is unreachable with the current packing
limits. Before a literal/op-count flush, a frame body is at most
`9 + 61_440 + 13*256 = 64_777`, below the allowed `65_536 - 56 = 65_480` bytes.
Therefore a valid generated-operation fixture cannot reach the serialized-body
limit before those other limits. Preserve malformed/frame-boundary decoder tests
and the runtime frame-size guard; record this derivation instead of inventing an
impossible positive fixture. Literal splitting and operation boundaries remain
required tests.

## Exact preservation contract

For every accepted base/target byte string, compare with the pinned legacy encoder:

1. The result is the same error class, `None`, or recipe.
2. A recipe has the same ordered frame bytes and operation count.
3. Applying it reconstructs the same target bytes and exact original length.

The existing 64 MiB core object limit, 16 MiB batched sender limit, 8,192-operation
limit, per-frame limits, and failed-frame atomicity remain unchanged. Candidate
generation is not reordered or skipped. No raw-fallback boundary or tie is changed.

Because the adaptive selector receives the same sparse result and its other
engines are unchanged, its candidate costs and selected wire representation must
also remain identical. Qualify that implication through explicit/adaptive sender
regressions, including both probe policies, repeats, pre-Begin Raw fallback,
missing/revoked bases and strict reply validation. No new cache membership query,
authorization interpretation, mode retry or offer mutation is introduced.

## Alternatives

| Approach | Decision and tradeoff |
|---|---|
| Checked lower bound before the unchanged packer | Selected: small scope, proof-backed rejection, exact output oracle available; scanning work remains |
| Exact dry-run pack-size calculation | Deferred: potentially tighter rejection, but duplicates packing state and increases drift risk |
| Sampling/edit heuristics or suppressing zstd prefix | Rejected for this milestone: may change the smallest-byte decision; not implied by the sparse proof |
| Reorder sparse candidates or redesign operation generation | Deferred: can change tie/serialization behavior and complicates the preservation contract |

## Qualification and instrument separation

Retain the baseline encoder as an immutable test/benchmark oracle with source
hash, extraction record and compatibility checks. Do not let the oracle call the
new rejection helper or share the changed selection decision. Test frame-byte
identity in addition to reconstruction. Qualification includes fixed seeds and
generated positive operation lists verifying `LB <= actual packed cost`.

Normal performance binaries retain their normal allocator. Allocation diagnostics
use a separately built custom-configuration binary with the same encoder semantics
and an explicitly documented diagnostic allocator policy. The accepted refinement
uses a bounded thread-local registry to identify frame-body buffers from the
unique nine-byte, alignment-one header allocation during an exclusive encoder
interval. Verify that uniqueness against both frozen encoder paths. Follow only
those buffers through growth, movement and frees; caller inputs and operation
vectors are outside this metric. Disable discovery when encode returns, while
continuing to track retained registered buffers through final drop.

For tracked growth, this diagnostic allocator deliberately allocates the new
block, copies, and frees the old block. Thus old/new overlap and requested
capacities are actually observed under the **same forced-moving diagnostic policy
for both versions**. The normal timed executable does not use this allocator.
Qualify registry overflow, allocation failure, stale/missing entries, recursion
and lifecycle handling; any incomplete accounting invalidates the memory gate.
Keep unsafe allocator mechanics outside the production crate, narrowly justified
and independently reviewed. Verify diagnostic and normal output bytes/operation
counts agree.

Record peak live requested frame-buffer capacity, cumulative allocation, growth
events, peak growth overlap, capacity retained on return and final free. These
are not allocator-internal usable size, process RSS or production allocator peak.
In-place growth in the normal allocator is not inferred from these measurements.
The memory acceptance below is explicitly scoped to this controlled diagnostic
policy; no general host or small-MVM memory claim follows.

## Registered measurement contract

Freeze code, compiler/features, both normal/diagnostic executable hashes, immutable
oracle, fixture generator/seeds, schedule and counters before timing. The seven
cells remain 16 MiB-class: three primaries (4 KiB prefix insertion, 4 KiB middle
insertion, unrelated full replacement) and four controls (exact repeat, localized
32-byte edit, scattered edits where aligned wins, periodic structured content).
Insertion bases are shortened to keep the target at 16 MiB. Before timing, prove
the registered primary fixtures actually exercise their intended rejection and
that both encoders produce identical results. A failed prerequisite stays a
qualification failure; do not substitute a favorable fixture after timing.

Use ten paired fresh-process comparisons per cell, five AB and five BA in an
interleaved fixed schedule. A is the frozen legacy encoder; B is the candidate.
No build, test, profile or owned competing benchmark runs during collection.
Record shared-host interference rather than manipulating unrelated labs.

The primary timer ends immediately when `encode` returns. Hashing, recipe
comparison, reconstruction verification and returned-recipe destruction are
outside that interval. After verification, a separate timer records recipe drop.
Also record the continuous entry-to-release interval and its explicit verification
gap, but do not use that contaminated interval as encoder performance evidence.
An `encode_plus_drop` value, if emitted, is labeled the sum of disjoint scopes,
not a continuous elapsed-time measurement. Primary CPU gates use encode-only CPU.
Preserve each timed result's exact output verification. Tiny/zero CPU observations
require an already registered equal repeated-call policy or an invalid-result
disposition; never replace observations selectively.

Gates are unchanged from the proposal:

- In **each primary cell**, geometric mean of paired `legacy_cpu / candidate_cpu`
  is at least **1.10**, and the candidate has at least **seven strict CPU wins out
  of ten**. Ties are not wins.
- In **each primary cell**, diagnostic peak live frame-buffer capacity falls by
  at least **50%**, and candidates rejected by the bound materialize zero frame
  bytes. Preserve the allocator policy and measurement qualification with this
  claim.
- In **each control**, geometric mean of paired `candidate_cpu / legacy_cpu` is
  at most **1.05**. Equivalently the reverse ratio is at least `1/1.05`, not 0.95.
- Every correctness/identity/resource prerequisite passes. Record all wall-time
  observations and regressions separately; no new WAN or energy gate is implied.

Any failed gate is retained and prevents claiming this candidate meets acceptance.
Do not adjust thresholds, discard pairs or change workloads after observing the
results. If not qualified, retain the experiment and revert/defer the production
candidate; do not publish a purported optimization solely because it compiles.

## Consequences and later work

The intended gain is less allocation/copying of sparse candidates that cannot win.
The base/target scans, required SHA-256 passes, receiver work and zstd-prefix work
remain. A passing encoder result is not an end-to-end adaptive or WAN speedup.
Only a separately registered follow-up can make those claims. Larger-client
resource qualification and the canonical encrypted MVM/vsock mailbox bridge for
a small orchestrated MVM environment remain later work.
