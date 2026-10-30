# Receiver quota-scan efficiency decision

Status: proposed narrow optimization; implementation is gated on the recorded baseline profile.
Source reviewed: `fedf38a8e308f3064b6ca3006aaf721a418c0d80`. No performance improvement is claimed yet.

## Decision

Hoist invariant namespace formatting and comparison out of the per-publication loop in
`Endpoint::disk_usage`. Format the requested namespace once per scan, and determine whether a
directory is that namespace once before iterating its files. Keep the complete filesystem scan.
Do not introduce a cached quota catalog, change publication preparation, or change transport
behavior in this refactor.

The existing scan in [`endpoint.rs`](../../crates/spiralcore-transfer/src/endpoint.rs) computes
`hex(&ns)` and obtains the same directory name for each eligible `.object` file. A new publication
normally invokes the scan both at admission and immediately before publication. The current
[`hex` implementation](../../crates/spiralcore-transfer/src/lib.rs) formats each of 32 bytes into a
short string. With P eligible publications, namespace formatting falls from P calls to one per
scan; directory-name comparison falls from P evaluations to at most one per eligible directory.
For two scans at P = 780, that removes 49,856 of 49,920 per-byte formatting operations, leaving 64.
This is a source-level work calculation, not an observed allocation count or CPU saving.

The directory enumeration and file-metadata work remain O(P), plus the cost of walking the
directories. This change cannot remove that filesystem cost or make admission constant-time.

## Invariants and proof obligations

- Enumerate the same root directories and files on every existing scan invocation. Preserve the
  canonical namespace, regular-file and `.object` extension predicates, metadata reads, checked
  additions and publication-count limit. Do not use a previously cached file length.
- Produce the same scoped bytes, aggregate bytes and publication count for the same filesystem
  observations. Namespace formatting and comparison are pure functions of the existing arguments
  and the directory entry; moving them does not move or remove a filesystem observation.
- Preserve the current admission calculation, including unique pending-object reservations,
  duplicate attempts, namespace/global byte limits and maximum publication count. Preserve the
  publication-time quota recheck.
- Preserve current handling of externally added, removed or resized files, including errors from
  enumeration and metadata reads. Do not add an assumption that other processes obey the owner
  lock. This refactor neither fixes nor widens existing concurrent filesystem-observation races.
- Leave metadata/payload verification, authorization checks, exact reconstruction, object and
  manifest identities, staging, rename, file/directory synchronization and commit acknowledgment
  unchanged. Resource limits and wire bytes remain unchanged.

A persistent quota catalog is deferred. Such a catalog would need separate decisions about
external filesystem mutation, startup reconciliation, duplicate publication, failure after rename
but before directory synchronization, and bounds on the catalog itself. Those obligations are
unnecessary for this invariant-hoisting change.

## Verification and measurement

Before implementation, preserve a release baseline with exact source, executable and fixture
identities. Profile receiver work at controlled cache populations, including empty/small caches
and a populated cache representative of the 780-publication warm campaign. Hold object sizes,
namespace distribution, graph-retention policy and target absence constant between versions.
Record quota-scan cost separately where the diagnostic permits it; do not infer it solely by
subtracting unrelated elapsed and CPU measurements.

After the refactor, rerun the existing quota-admission, duplicate-publication, namespace isolation,
corruption and durability tests, then perform balanced paired old/new release measurements against
the same prepared cache state. Retain every trial and describe OS-cache/background-load limits.
Check exact output and byte accounting, report CPU and elapsed distributions, and retain the
empty-cache result even if the added constant formatting work produces a small regression. A test
that merely asserts the placement of this expression is not required; semantic regressions and
recorded paired performance are the relevant evidence for this low-risk refactor.

Accept only if the baseline confirms material repeated-formatting work and the paired result
supports improvement at the populated-cache workload without changing semantics. Report a null or
negative result if the filesystem cost dominates. No reduction in network round trips or energy
consumption follows from this CPU optimization.

## Separate follow-up decisions

Raw/zstd publication currently folds and optionally retains a DAG, while sparse publication does
not. Keep that policy unchanged in this paired refactor so old/new operations perform equal work.
A future raw/zstd versus sparse comparison should explicitly match durable-object readiness and
DAG readiness, or report their difference. Setting `graph_cache_bytes` to zero alone does not
disable the current raw/zstd fold; it only prevents retention. Removing eager preparation also
affects the stored manifest identifier and future DAG-base discovery, so it is a separate contract
decision, not a free baseline adjustment.

The measured sparse path already uses one HTTP batch: mutation tasks set `warm_probe = false`,
and the sender batches Begin, Data and Finish. All 30 sparse seed and transfer observations in
each retained warm/prefix workload report one HTTP request. Exact repeats use one identity probe.
There is no redundant application round trip to remove from these measurements.

Connection establishment, relay transit, receiver accept polling and endpoint work remain possible
latency contributors. The receiver currently polls an idle listener every 10 ms and closes each
origin HTTP connection to prevent an idle connection monopolizing its serial loop. Persistent
connections require bounded fairness and timeout design; sharing a connection between operations
also invalidates the current SYN-owned per-operation packet attribution. Profile those costs and
define the replacement accounting unit before changing transport behavior. Retain the durable
commit acknowledgment as the completion boundary.
