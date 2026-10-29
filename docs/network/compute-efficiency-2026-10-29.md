# Making warm transfers computationally cheap

Historical status: source audit and bounded diagnostics at `354b616257eda62cd6bdcc2fb37902fc4cb4a5e8`, following the first real WAN campaign. The descriptions of the active implementation below refer to that pinned baseline. The proposed changes have since been implemented and measured in [the subsequent efficiency report](efficiency-results-2026-10-29.md); retain this document as the original cost analysis and plan.

## What executes today

The measured transfer path uses exact byte identities, domain-separated SHA-256, an ordered ternary DAG with 32-byte leaves, missing-record exchange, raw/zstd fallback and verified durable publication. E8 routing, Kuramoto oscillators, cellular automata and the other research mechanisms are not running on this measured path.

For a warm mutation, the sender rebuilds the base graph, builds the entire target graph, scans target node IDs for differences, and serializes the new records. The receiver reads and verifies the base bytes, rebuilds their graph, decodes the new records, traverses the closure, reconstructs the output, verifies its whole-object digest, and publishes it durably.

The active `Graph::fold` is an extraction/compatibility bridge: legacy fold → legacy full rebuild/check → legacy record decode → new SHA-256 node conversion/indexing → active whole-object identity. It constructs both old and new representations. See [dag.rs](../../crates/spiralcore-transfer/src/dag.rs).

The active durable cache stores flat bytes, object identity and an optional manifest identity. It does not retain the graph nodes. A completed attempt clears its temporary nodes. The next update rebuilds them. Raw/zstd publication also folds the object synchronously to obtain a manifest ID and discards the graph. See [endpoint.rs](../../crates/spiralcore-transfer/src/endpoint.rs).

## Measured costs

In the 30-trial WAN receipts, sender base preparation alone had these CPU medians:

| DAG case | Base preparation ms | Total sender CPU ms | Median per-trial base share |
|---|---:|---:|---:|
| Repeat | 99.543 | 207.002 | 48.15% |
| Localized 32-byte edit | 99.476 | 213.366 | 46.67% |
| Scattered 32×32-byte edits | 103.530 | 223.033 | 46.49% |
| One-byte prefix insertion | 100.482 | 241.013 | 41.54% |

Those sender measurements came from the Linux runner. The remaining time is not solely hashing: it includes target construction, map operations, allocations, encoding and HTTP/TLS. The receiver's localized-update aggregate was 237.202 ms CPU. Removing sender base preparation alone is therefore insufficient.

A separate Apple M1 release-build diagnostic used deterministic 1 MiB data, one warmup and seven sequential samples per operation. Hash/copy samples average 100 calls; graph samples measure one call. The table reports process CPU, including allocation/drop. There was no network, disk publication or energy measurement. Page caches were uncontrolled; this is cost ranking, not a paired WAN speedup experiment. Legacy-fold wall time had substantial scheduling noise, so do not use it as an elapsed-time estimate.

| Operation on 1 MiB random data | Median CPU ms |
|---|---:|
| Current full `Graph::fold` | 116.996 |
| Validate missing-node closure | 39.566 |
| Reconstruct and verify existing graph | 36.928 |
| Encode every node | 32.033 |
| Recompute every node ID | 25.994 |
| Select changed records from prepared graphs | 3.156 |
| Flat whole-object identity | 3.054 |
| Clone existing graph | 2.269 |
| Copy input bytes | 0.0258 |

These operations overlap conceptually; do not sum their microbenchmarks into a transfer-time estimate. Nearby results such as 3.156 ms and 3.054 ms are approximately comparable, not a statistically established ordering. [Diagnostic source](evidence/2026-10-29-compute/probe.rs), [measurements](evidence/2026-10-29-compute/results.jsonl), [build/source identity](evidence/2026-10-29-compute/metadata.json).

The random object has 32,768 unique leaves and 16,376 parents: 49,144 nodes. A localized overwrite needs only 10 new records; the scattered overwrite needs 243. Prefix insertion needs 49,147 new nodes because all fixed leaf boundaries shift. A periodic 64-byte fixture has only 19 unique nodes, yet reconstruction still visits and rehashes occurrences, taking 28.674 ms CPU. Structural sharing should also reuse verified computation.

## Recommended implementation sequence

1. **Build the active format directly.** Replace the legacy build/rebuild/convert bridge with a direct canonical v1 builder. Preserve every node ID, record encoding, manifest/root order, original length, zero padding, node/work limit and conflict check. Keep the legacy path as a test oracle. This is a constant-factor improvement that can preserve the wire format; it does not make arbitrary whole-object preparation sublinear.

2. **Use ordinary identity lookup for known exact repeats.** The existing raw-CAS control already used 2.538 ms sender CPU and 145.156 ms elapsed, versus DAG's 207.002 ms CPU and 352.028 ms elapsed. Avoid building either graph just to discover that the exact object is present. A raw Begin that misses creates an immutable raw attempt: do not switch it to DAG under the same transfer ID. A generic lookup needs an explicitly specified operation or cancellation plus a fresh attempt, with its extra round trips counted.

3. **Retain bounded, verified immutable graphs.** Preserve prepared base graphs and indexes at both endpoints with shared ownership and per-transfer staging overlays. Avoid cloning the whole map. Charge shared graphs once to global cache/memory budgets; pin active references; keep authorization, policy epochs, corruption detection and eviction behavior explicit. Public mutable `Graph` values cannot simply be relabeled trusted. Raw publication can defer optional DAG preparation only if readiness remains distinct from durable object commit and all deferred work is counted.

4. **Update changed leaves and ancestors.** With a retained graph and a trustworthy change list, a localized overwrite updates approximately the 10 affected records, rather than rebuilding 49,144 nodes twice. Validate each distinct immutable node once on ingress/load, then reuse that validation. Combine closure checking and reconstruction where the trust invariants permit it. Retain final byte-length and whole-object digest checks. Authoritative dirty ranges must belong to a consistent immutable snapshot; an arbitrary file with unknown changes still needs comparison/scanning.

5. **Use a sparse byte recipe where fixed leaf alignment is the wrong representation.** A one-byte prefix insertion can conceptually be represented as `LITERAL(new_byte); COPY(verified_base, 0, old_length)`. A localized overwrite can be two COPY ranges and one short LITERAL. Coarse chunk indexes plus bounded sparse residuals can reduce hashes, metadata and matching work while preserving tiny edits. Specify ranges, overflow/output bounds, base identity, authorization and final verification explicitly; version any new encoding. Compare against [VCDIFF](https://www.rfc-editor.org/rfc/rfc3284.html) and [rsync](https://rsync.samba.org/tech_report/node2.html), which already use reference/literal reconstruction mechanisms. No novelty or superiority is established by this proposal.

After these changes, inspect contiguous node arenas, cached node encodings, allocation reduction and the exact SHA-256 implementation's CPU acceleration. More parallelism should not substitute for removing unnecessary work, especially when the objective includes energy.

## Separate hardware-SHA feature builds

The installed `sha2` 0.10.9 source uses its software backend on this aarch64 build with default/std features; its hardware backend is gated by `asm`. A separate local package enabled `sha2/asm` for an A/B diagnostic without changing shipping dependencies, features or source. Both builds detected the M1's SHA2 instructions. All five fixtures rebuilt exactly, with matching whole-object identities and fingerprints over every encoded manifest and 196,598 encoded node records across the fixtures.

| Operation | Software CPU median ms | Hardware-enabled CPU median ms |
|---|---:|---:|
| 1 MiB whole-object identity | 3.081 | 0.615 |
| All 49,144 node IDs | 26.228 | 12.390 |
| Full graph fold | 111.506 | 154.133 |

The primitives improved by approximately 5.01× and 2.12×, respectively, but full folding regressed in this run. Non-hash controls also slowed: byte copy was 24.98 versus 34.56 microseconds and prepared-graph delta selection was 3.400 versus 4.286 ms. The host was not isolated and the software run preceded hardware; host/order effects or other causes remain unresolved. This is a promising primitive-level candidate, **not a demonstrated overall graph, WAN or energy improvement**. It requires controlled paired follow-up before adoption. No FIPS validation follows from using hardware instructions.

[A/B comparison](evidence/2026-10-29-compute/ab-comparison.json), [exact build/method metadata](evidence/2026-10-29-compute/ab-metadata.json), [comparison probe](evidence/2026-10-29-compute/probe-verified.rs).

## The remaining floor

The desired affected-node/hash work for a known localized change is approximately O(k log N), with shared ancestors counted once, rather than repeated whole-graph construction and map scans. Ordered-map lookup/update can add another logarithmic factor unless the index/path structure changes. This is a reduction in graph work, not an equivalent factor in end-to-end speed.

The current v1 object identity is a flat domain-separated SHA-256 of the length and complete byte string. An arbitrary interior edit still requires an O(n) hash pass; an incremental Merkle root is not a drop-in replacement for that digest. Materializing and durably publishing a full output also entails O(n) output work. A copy-on-write or lazy Merkle publication contract could change that floor, but requires a separate versioned design. The first useful goal is changed-node work plus a fast full verification pass.

Avoiding 1 MiB represents about 0.84 seconds of link serialization at 10 Mbps, but only 8.4 ms at 1 Gbps, before framing. This is an illustrative calculation, not a measured shaped-WAN result. The encoder's extra critical-path compute and protocol latency must fit inside the transmission time avoided. Persistent authenticated connections and batching can then reduce setup/round-trip costs, with corresponding lifecycle and packet-attribution tests.

The first implementation gate should require byte-identical old/new graph outputs across retained vectors, empty/trailing-zero/boundary and randomized fixtures; unchanged hostile-input and publication tests; and paired release measurements of total bytes, CPU, elapsed time and memory. Cache seeding and deferred indexing must be reported separately and amortized. A lower CPU figure alone is not an energy result: measure joules with suitable platform counters or a power meter, including cache and relay costs, before claiming an energy-saving factor.
