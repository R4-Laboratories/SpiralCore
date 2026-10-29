# Finer quota-scan attribution: proposed registered diagnostic

Status: source and qualification preparation; performance NOT_RUN. Only this new local
directory is owned. Production source and prior evidence remain unchanged. Parent must
commit the umbrella registration and authorize a coordinated quiet interval before timing.

Baseline is `d03fe1630bba787a28cc3fa3ae201c4a1a75388b`, native Apple ARM with the optional
hardware SHA feature on. Reuse the receiver probe's 1 MiB per-byte xorshift64 seed42 base
and 32-byte localized XOR target. Preserve normal max_nodes262144, graph retention zero,
all quotas, fresh absent targets, private synchronized template clones, exact Ready/
Accepted/Published checks and byte-for-byte verification after closing and reopening.

## Fixed proposed matrix

| Layout | Background publications | Background namespaces | Total starting publications / namespaces |
|---|---:|---:|---:|
| empty | 0 | 0 | 1 / 1 |
| compact64 | 64 | 1 | 65 / 2 |
| compact768 | 768 | 1 | 769 / 2 |
| spread768 | 768 | 384 | 769 / 385 |

Two arms: raw and sparse, normal node budget only. Three observation blocks, cyclically
ordered. Three instrumentation configurations: counters-only, fine spans, and empty-span
clock control. Total: **4 layouts ×2 arms ×3 configurations ×3 blocks =72 observations**.
No favorable-cell filtering, retry or post-hoc replacement. Tiny node budget is omitted:
this experiment isolates scan mechanisms and namespace layout with a fixed policy.
Raw still performs optional graph warming; sparse does not. That readiness distinction
is recorded rather than redefined as a pure transport comparison.

## Snapshot-only instrumentation

Keep every original filesystem expression, predicate ordering, checked sum, quota decision
and error result. Desugar iteration only enough to bracket ReadDir creation and next().
The six disjoint source-expression groups are:

1. Directory iterator creation and advance, including end-of-iteration checks.
2. DirEntry::file_type queries, preserving short-circuit behavior.
3. Canonical namespace basename classification, including its original temporaries.
4. Full-path construction, extension predicate and temporary destruction.
5. DirEntry::metadata().len(), preserving all errors.
6. Namespace filename plus hex formatting/comparison, including temporary destruction.

The arithmetic, iterator/entry destruction and instrumentation bookkeeping not contained
in a named span remain in the coarse scan residual. Do not rename that residual as I/O.
Fixed stack counters and a constant-size thread-local receipt replace no operations.
No path list, catalog, watcher, new filesystem calls or per-entry log/storage is added.
API counts remain distinct from actual syscalls. The path-expression span includes
allocation-related work but does not count allocator events; exact allocations and OS
syscalls remain UNMEASURED without a separately reviewed profiler. No unsafe hooks.

Counters-only executes the original expressions without fine clocks. Fine mode brackets
each selected expression. Empty mode executes an empty black-box span with the same
number of clock pairs, then executes the original expression outside that span. Each mode
still measures whole-scan, handler and operation CPU/wall. Read/reset remains outside the
handler clocks. All modes carry the same receipt shape. Per-entry clocks can materially
perturb code generation and runtime: report raw span totals, empty spans, coarse residual,
and fine/empty operation inflation versus counters-only. **Never subtract a synthetic
overhead estimate and present the remainder as exact production cost.**

Use paired block/layout/arm identities for control comparisons, retain each of three raw
observations, midpoint median (ordinary central value for n3), min/max and paired ratios.
The three configurations rotate order by block. Layout and raw/sparse order rotate too.
No network, energy, syscall or allocator count is inferred from these clocks.

## Qualification before timing

Runtime RED then GREEN tests must show that absent/misclassified category counters are
detected. Preserve success totals and partial counts/error stage for Storage and Limit.
Test exact call accounting for ignored/canonical namespaces, non-object files, directories,
symlinks, hard links and non-UTF8 filenames; compare the reference and instrumented scan
results on the same static fixtures. Both modes must observe completed external growth,
addition and deletion between Begin and Finish. No default quota/authority changes.
Qualify returned IDs, body schema, stage sums, fixed storage overflow marking, thread
isolation and each layout's exact inventory. Complete existing profile tests and relevant
snapshot tests; preserve compiler/lock/features/source/executable/fixture identities.

Preparation qualification completed: 11 probe tests and 136 snapshot all-target tests
passed; one existing release-only 16 MiB local HTTP qualification remains unrun in this
snapshot's default test command. Both Clippy passes with warnings
denied succeeded. The three category counter tests first failed at runtime with recording
disabled, then passed; success, partial Limit and early Storage outcomes match the frozen
reference scan. All three instrumentation controls preserve exact wire bodies/reopened
bytes and observe external growth/addition/deletion between Begin and Finish. Actual
non-UTF8 file creation is unavailable on this host (OS92 Illegal byte sequence), recorded
in `qualification/setup-findings.json`; only pure non-UTF8 path classification is tested.
No frozen release binary, template preparation or timed campaign has run yet. Source
review is queued before freezing the executable and exact nine-invocation schedule.

## Attribution and separate candidate gates

Attribution is descriptive. A category is a useful candidate lead only if its three
fine-span CPU observations consistently exceed the corresponding empty-span cost and
constitute a material share of operation CPU. A proposed screen is median fine span
at least4× empty span and at least10% of counters-only operation CPU in spread768 sparse;
it is a screen, not proof of attainable savings. If aggregate fine-clock inflation is
above25%, label fine allocation/classification ranking instrumentation-sensitive and do
not select a candidate from those ranks alone. Keep negative/noise findings.

Any production optimization needs a separate committed source diff and gates before
implementation/measurement. Proposed uninstrumented acceptance: hardware SHA on in both
arms, 8 balanced AB/BA process pairs; spread768 sparse whole-operation CPU old/new
geometric mean≥1.10 with≥6/8 wins; compact768 sparse and empty-cache raw/sparse CPU
geometric mean≥0.95; preserve all raw cells, elapsed, exact bytes, readiness and application
byte counts. Require classification/quota/external-mutation tests and unchanged fresh
Begin/Finish filesystem observations. No cached usage catalog or authority change is
included. SC-OPT-002's failed formatting hoist cannot be folded in as an unmeasured change.

If the safe path-expression option is not material, record that result and defer a platform
bulk-metadata backend for separate architecture/dependency/error-equivalence review.
Do not weaken quota observation to force a positive optimization result. MVM integration
and WASM remain outside this diagnostic.
