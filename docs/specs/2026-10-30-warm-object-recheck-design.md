# SC-WAN-005: verified warm-object recheck

The user authorized returning to SpiralCore's prior-object savings after the
schema comparison. This design starts from merged `921d971589d8dca267f150a727be0fc52510ca32`.
It adds a bounded comparison, not a new production codec or selector.

SC-OPT-014 reused codec contexts and a schema, but did not offer the previous
object as a sparse/prefix base. Its result therefore does not establish a
regression of the historical warm-object path. The existing
`client::transfer_sparse_batched` evaluates only equality, the bounded delta,
object identities and transfer; it does not encode all other candidates.
Ordinary adaptive selection still evaluates chunked zstd, sparse and prefix and
is excluded here. Runtime checkpoint/replay remains a distinct mechanism.

## Matched comparison

Use two current-source arms: **forced Sparse/Raw** through the existing sparse
API, and **prepared Whole zstd/Raw** through the SC-OPT-014 API. Whole retains
its level-3/window-25/LDM/checksum/content-size/single-worker profile. All
comparison, encoding, fallback and hashing work remains timed. Preserve existing
wire formats, authorization, resource admission and exact durable commit.

Each arm receives the same actual Raw base seed in its own fresh authorized
namespace, then one previously absent target. Reuse the historical xorshift64-v1
fixture definition and exact 32-byte midpoint overwrite at **1 MiB and 16 MiB**;
freeze generator inputs and full base/target identities in the plan. All 32 edit
bytes must differ. Use `warm_probe=false`; this measures a changed object, not
an exact-repeat object-cache shortcut. The expected successful sparse request
plus response body cost is **600 bytes**, checked against actual observations.

Both arms use the same persistent receiver with `ColdGraphPolicy::Disabled`.
Enable the existing bounded codec cache equally; Sparse does not require a
prepared zstd decoder. Whole context construction is separately timed and
charged once to its session. Record the actual seed cost and full seed-plus-update
lifecycle for both arms. The identical Raw seed is the matched first delivery
for these pseudorandom inputs; do not describe receiver warm state as free.
This is one update per seeded object, not a long update-sequence qualification.

## Dose, proof and measurement

Run **30 paired trials per size**, with deterministic counterbalanced A/B and
B/A ordering (15 each). Three shards contain ten trials per size apiece. Total:
**120 sessions, 120 seeds, 120 changed targets, 240 transfers**. Each shard has
40 scopes and 80 transfers. Retained payload is exactly **680 MiB** per shard;
adding 40 MiB decoder retention, 16 MiB slack and 80 × 4 KiB publication/header
allowance gives **736.3125 MiB**, below the 768 MiB campaign bound and unchanged
1 GiB store quota. Keep existing attempt, frame, object and resident limits.

Before WAN execution, use the same two arms and fixture rules in a small local
qualification: three paired trials per size, 24 seed/update transfers, fresh
store, exact reopen and measured compute. Reuse existing transport/measurement
machinery. Local correctness and accounting must pass; a local speed advantage
is not a launch prerequisite because WAN byte savings can change completion time.

Time encoding/comparison start through validated durable ACK. Record sender CPU,
disjoint receiver-handler CPU, request/response bodies, exchange elapsed and
actual setup/lifecycle totals. Local execution must count endpoint CPU only once;
if the receiver runs in the sender process, report combined CPU and the nested
handler scope rather than adding it again. Report warm updates and paid
lifecycles separately, with thirty paired ratios, median/p95 and wins/losses.
Exchange elapsed includes remote computation/waiting and is not pure wire time.
Body counts exclude HTTP/TLS/TCP/IP, retransmission and extra relay-hop traffic;
the CPU subtotal excludes relay and receiver socket/logging work.

After each stopped receiver is exclusively reopened, regenerate and compare
every seed and target byte-for-byte. Reject missing/mismatched rows, bases,
acknowledgments, source identities or costs. Keep failures; do not retry or
replace a trial. Freeze source, executable, plan and receiver policy before the
registered WAN run. Historical SC-WAN-002 used chunked zstd and unequal optional
DAG warming: report this new matched baseline explicitly, without transferring
its old speed ratios to the new experiment. No energy, MVM, FIPS or general WAN
acceptance claim follows from this one synthetic route.
