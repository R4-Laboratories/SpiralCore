# SC-OPT-002: remove repeated receiver work with recorded controls

Status: profiling plan. User authorized continued optimization and persistent reviewable results.
Baseline: `fedf38a8e308f3064b6ca3006aaf721a418c0d80` (same production code measured at `b552017`).
Use the current managed worktree. Preserve the completed SC-OPT-001 source and evidence.

## Hypotheses and boundaries

1. `Endpoint::disk_usage` enumerates every namespace and publication at admission and publication.
   The completed WAN campaign accumulated hundreds of publications. This may make an unrelated
   object's commit cost proportional to cache population. The exact share is unmeasured.
2. Raw/zstd Finish currently constructs a DAG; sparse Finish does not. Measure this readiness-policy
   difference explicitly before attributing total CPU savings to byte encoding alone.
3. Sparse mutations already upload Begin, Data and Finish in one HTTP batch. There is no redundant
   application round trip in that measured path. Connection setup and relay latency remain separate.

The invariants stay fixed: complete byte/length/digest checks, authorization, shared quota admission,
bounded staging, exclusive cache ownership, atomic publication and directory/file synchronization.
Do not change the wire format, digest or trusted-cache rules merely to improve a benchmark.

## First experiment: baseline receiver cost

Implement a standalone Rust diagnostic against the pinned public API, with tests for fixture,
receipt and stage accounting. Use public deterministic 1 MiB targets and a verified base. Populate
fresh caches through the public API with small unrelated objects, using controlled namespace counts
to model the WAN campaign. Sample empty/small/medium/large cache populations; record actual object,
directory and byte counts rather than the requested population alone.

Measure raw and sparse updates under the default graph budget and a separate `max_nodes=1`
graph-admission policy. The latter prevents the random 1 MiB object from obtaining a DAG manifest,
but empty/one-leaf objects can still be folded. It also changes reservation/admission constraints,
so it is not a pure warming toggle or a drop-in general-purpose policy. Set graph retention to zero
in both policies; record actual graph stats and publication sizes/readiness. Use identical budgets
when comparing baseline and candidate. Include seed/setup costs separately. The diagnostic measures
local endpoint work, not WAN or joules.

Use counterbalanced trial ordering, retain every observation and exact output identity, and compare
CPU/wall distributions per stage (Begin, Data, Finish) as well as the complete operation. Object
reopening and exact comparison must pass. Clear the timing boundary around fixture generation,
transport framing, input allocation and verification. Do not mix production builds/tests with runs.

## Conditional implementation

First profile the narrowest candidate: `disk_usage` repeatedly converts the same namespace ID to
hexadecimal inside the per-publication loop. Hoisting that conversion once per scan and the
namespace match once per directory preserves every filesystem enumeration/stat and quota check.
Use existing quota/admission adversarial tests and paired measurements for this loop-invariant
refactor; do not add a test that merely asserts the source-code layout.
The architecture decision and invariants are in
[receiver-quota-scan-decision.md](../network/receiver-quota-scan-decision.md).

After recording baseline results, select one demonstrated hotspot. A bounded cache-usage catalog
is a later candidate, not an approved shortcut: it requires a startup scan under exclusive ownership,
checked counters and namespace accounting, correct pending reservations, publication failure/crash
handling, idempotent retry and bounded metadata memory. Read/verify cached bytes as before. Review
the threat boundary for external filesystem edits and preserve or explicitly strengthen existing
behavior. If proof obligations do not fit a bounded change, retain the finding and choose a safer
optimization.

Write failing tests first, then implement one change, then run Rust/security review. Preserve the
old probe executable and repeat paired old/new release measurements with the same fixtures and
cache policies. Report the effect at each population and on raw versus sparse, including negatives.
Only claim a WAN improvement after a new matched WAN campaign; a local CPU improvement is not one.

## Delivery

Add source/executable/configuration identities, raw rows, independent calculations, correctness and
resource tests, review outcome and remaining gaps under a new evidence directory. Update
`docs/network/optimization-ledger.md` as each phase completes. Keep earlier measured artifacts
unchanged. A final report must identify which cost moved and which cost remains.
