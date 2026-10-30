# Cache scan options under the existing storage contract

Status: proposed options and qualification gates; no production optimization selected.
Reviewed baseline: `f0966ebedcb8994f0985b54e9a2b961eeb8be914`.

## Decision and recommendation

Keep fresh filesystem quota scans at admission and immediately before publication.
Profile their actual CPU, elapsed time, allocations and filesystem work before choosing
a change. The first eligible implementation is a small, safe Rust reduction in path
construction, **only if that work is material in the profile**. If metadata and directory
enumeration dominate, leave the current implementation in place pending separate review
of a bounded platform backend. A permanent optimization is not a required outcome.

Do not add a persistent usage catalog, reuse the admission scan at publication, skip
other namespaces, change limits, or assume that the owner lock excludes arbitrary local
filesystem edits. Do not restore SC-OPT-002's rejected formatting hoist as an unmeasured
part of another change. Its [negative result](receiver-cost-results-2026-10-29.md) remains
part of the decision history.

This document records options, not completed architecture. The current
[object profile](../contracts/spiralcore-object-mailbox-v1.md), durable publication and
authorization rules remain the baseline; no baseline writeback or migration is needed.

## What the current implementation observes

[`Endpoint::disk_usage`](../../crates/spiralcore-transfer/src/endpoint.rs) walks the root,
then every nonsymlink directory with a lowercase 64-character hexadecimal name. It counts
every regular file with the exact `object` extension and obtains its current metadata
length. It uses checked byte additions and rejects a publication count beyond the limit.

Important details must survive an optimization:

- The count is of directory entries, not unique inodes or valid object identifiers.
  A regular `not-an-id.object` file counts. Hard links count once per eligible entry.
  Whole file length includes the publication header; allocated blocks are not the quota.
- The scan does not validate object headers or payload hashes. A malformed regular
  `.object` file still consumes quota. Object lookup/read separately performs private-file,
  header, length and full object-identity validation. Those checks must remain unchanged.
- Symlink entries are skipped by the current entry-type predicates. Enumeration,
  entry-type and metadata errors propagate; an unreadable or disappeared entry must not
  become a zero-byte success. An optimized classifier must preserve non-UTF-8 filename
  behavior and exact extension semantics.
- `admit_disk` adds unique active reservations keyed by namespace and ObjectId, with a
  conservative 81-byte header allowance. Duplicate attempts cannot reduce or duplicate
  the reservation improperly; cancel/expiry and completed publications affect it.
- `publish` scans again before creating a new publication. An existing publication instead
  follows its existing exact-byte verification and idempotent acknowledgment path.
- The `.owner` lock excludes another cooperating Endpoint. It does not turn a mutable
  directory into an immutable database. Neither the current scan nor this decision
  promises an atomic snapshot against concurrent external mutation. A completed external
  change between Begin and Finish must still be observed by the later scan.

The scan's work depends on both directory count and eligible file count. Existing
background-object experiments used two objects per namespace, so their measurements do
not isolate those two costs. The hardware SHA experiment's Begin/Finish stage times
also contain hashing, reads and publication work; they are not a per-function scan profile.

## Bounded options

| Option | Proposed reduction | Contract and resource obligations | Disposition |
|---|---|---|---|
| A. Avoid constructing a full path for the extension predicate | Inspect the entry's basename with `Path::extension` instead of allocating `f.path()` for every regular file. Keep the existing `DirEntry::metadata` call. | Preserve predicate order, error propagation, exact basename/extension behavior and every existing filesystem observation. Do not combine with namespace-hoisting, formatting, new quota logic or a different metadata API. Transient storage stays bounded to the current entry. | Smallest eligible candidate, conditional on allocation/profile evidence. Expected gain is unknown and may be negligible. |
| B. Bounded parallel fresh scans | Overlap independent directory/metadata work with a small fixed worker count and bounded queue. | Enumerate every eligible directory and file on every scan; retain all errors and checked reductions. Charge queue buffers, worker stacks and results within existing resource ceilings. Do not accumulate all paths without a bound. Specify error precedence and changed observation timing before implementation. | Defer. It may reduce elapsed waiting while increasing total CPU and memory. It is not a computation/energy saving by construction. |
| C. A bounded platform bulk-metadata backend | Obtain names, entry types and logical lengths in batches instead of issuing the existing sequence of individual operations. | Demonstrate equivalent complete observations, symlink classification, per-entry errors, exact logical lengths, checked record parsing and bounded buffers. Preserve a portable reference path. Unsupported-operation fallback must restart a complete reference scan, without accepting a partial total. | Research candidate only if syscall work dominates. Requires separate platform/dependency review; not authorized by this record. |

For option A, retaining `file_type` before the extension predicate avoids gratuitously
changing which filesystem errors are observed. Using the filename alone for an extension
test removes only the parent-path construction; it does not remove the metadata query.
Do not assume `file_type` and `metadata` each produce a syscall on every platform, or
that replacing them with a single `metadata` call saves one. That needs source/profile
evidence on the actual runtime.

Option B must not require `Authority: Sync` merely to parallelize filesystem work. It
would also add scheduling and ordering obligations to a currently serialized Endpoint.
It is unsuitable if the target is CPU reduction and the benefit is only wall time.

For option C, the installed Apple SDK declares `getattrlistbulk` and bulk name/returned
attribute flags. That establishes an API exists, not equivalence to Rust metadata or
availability of a suitable safe dependency. The production crate forbids unsafe code.
A raw FFI parser must not be slipped into this optimization; any audited safe wrapper,
dependency, fallback and target support need their own explicit decision. In particular,
per-entry attribute failure is not an empty file, and an incomplete buffer is not a
complete scan. A Linux or Apple-only result must be labeled with that target.

## Alternatives outside the current authority

An in-memory counter, persistent catalog, directory-mtime cache, watcher cache or a reused
Begin snapshot is not equivalent to the present behavior. A correct replacement would
need an explicitly changed filesystem ownership/invalidation contract, recovery and
reconciliation rules, and a bounded authenticated state format. Directory timestamps or
notifications alone have not been established as evidence that every nested file length
is unchanged. This phase does not authorize that change.

Early rejection when a partial byte sum already exceeds a limit could save work on
overfull caches, but can change which later storage error is returned. That is a separate
failure-behavior decision and does not improve successful transfers. Flattening the cache
layout or changing publication readiness would introduce migration or protocol consequences
and is also outside this scan experiment.

## Mandatory correctness and security gates

Before accepting any candidate, retain baseline/candidate observations for the same static
filesystem fixtures, and exercise these behaviors through public Endpoint operations:

1. Namespace bytes, aggregate bytes and publication count at just-below, exact and
   just-above limits; empty files and both 49/81-byte publication headers; pending unique
   reservations, duplicate attempts and cancellation/expiry. Preserve all default limits.
2. Canonical and noncanonical directories; regular files with valid and invalid object
   stems; exact `.object` extension edge cases and non-UTF-8 names; nested directories,
   symlinks and hard links. Do not accidentally follow an external path or deduplicate
   hard links for quota purposes.
3. External creation, removal and resizing completed between Begin and Finish, including
   a different namespace that exhausts the global quota. Corrupt headers, trailing bytes
   and modified payloads must still fail the existing publication/read validation.
   Metadata/enumeration failures must remain failures, never skipped entries.
4. Revocation at publication and after reads, namespace/principal/epoch separation,
   duplicate immutable publication, reply/replay bounds and resident reservations. A
   scan optimization cannot make unauthorized bytes readable or publishable.
5. Restart ownership and staging cleanup: retain the existing marker/private-path/lock
   checks; remove only owned staging names in canonical directories. Preserve unrelated
   files and failed-operation evidence. Existing durable objects must verify after restart.

Staging, file synchronization, authorization immediately before rename, atomic rename,
directory synchronization and Published acknowledgment are unchanged. For a candidate
that touches those boundaries or introduces retained state, require focused interruption
and injected-I/O-failure tests before measurement: pre-rename failure cannot publish;
post-rename failure may leave a file that the next operation must count and validate;
retry cannot acknowledge corrupted bytes. A rename followed by directory-sync failure
is not a guaranteed rollback. Process-kill/reopen tests do not prove power-loss durability.
For a pure option-A predicate refactor, existing restart/publication tests plus the new
classification/quota regressions are sufficient; broad crash-hook infrastructure is not
justified by that change.

## Baseline and comparison design

Use native Rust for new probes, fixture generation, receipt validation and statistical
analysis. Preserve source, lock, feature tree, compiler/target, executable hashes, fixture
and cache inventories, build arguments and every raw result. Diagnostic instrumentation
is separate from the uninstrumented full-operation comparison.

The first diagnostic should distinguish directory enumeration, file metadata, pure
classification/path construction and the rest of Begin/Finish. Use the current
implementation as the reference; do not sum unrelated microbenchmarks into a reconstructed
WAN time. If exact syscall/allocation counts cannot be observed, label them unavailable
rather than infer them from source calls or stage differences.

Separate publication and namespace count. A bounded useful matrix is the existing
one-base/no-background fixture, 64 background objects in one other namespace, and
768 background objects in either one or 384 other namespaces. Keep background payloads,
permissions and inventory fixed, recording actual directory/publication counts. Add
near-limit populations as correctness checks, not an automatically enlarged timed run.

For old/new performance, use fresh private synchronized copies of identical closed cache
templates, with an absent target and the same verified base. Counterbalance execution
order, preserve failures and outliers, and do not retry only unfavorable observations.
Use the same SHA backend, node budget, graph retention, endpoint quotas and transport in
both arms. The native Apple hardware feature is now a qualified opt-in; mixing default
software and hardware builds would invalidate attribution to a scan change.

Retain the 1 MiB localized-update fixture as the historical anchor. Any 16 MiB work is
a separately identified size stratum with its own source/fixtures and complete seeding
costs. The batched client currently caps objects at 16 MiB; the 64 MiB protocol maximum
does not by itself establish a working or measured 64 MiB WAN path. Node-budget effects
on optional DAG readiness must be recorded rather than hidden by changing policy.

Freeze the candidate, allowed source diff, chosen primary cell, sample count and numeric
performance gates after the diagnostic but before candidate timing. A proposed minimum
gate for a successful-transfer CPU optimization is an old/new paired CPU geometric mean
of at least 1.10 in the selected populated-cache cell with at least 6/8 wins, and no more
than 5% CPU regression in either the empty-cache anchor or same-population alternate
namespace layout. Report elapsed ratios and every secondary cell even if CPU passes.
These thresholds are proposed, not an already registered campaign. A parallel I/O candidate
would require an explicitly different wall/CPU tradeoff decision before running it.

Every accepted operation must reconstruct exact target bytes and ObjectId and retain the
same protocol byte accounting, authorization and durable acknowledgment. Report scan-only
and complete-operation results separately. A failed gate means retain the evidence and
leave or restore production to the baseline. CPU reductions do not establish joules saved,
and local operation timing does not establish a new WAN speedup.
