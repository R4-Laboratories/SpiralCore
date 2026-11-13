# Descriptor-rooted Unix object storage

The opt-in SmallV1 receiver can use directories that its resource owner has already opened. It provides a descriptor-backed alternative to the synthesized procfs path associated with the installed macOS FlowMux preparation failure. The installed adapter still needs to consume this API. The existing Path constructors and Windows backend retain their behavior. Object identity, v1 frames, codecs and per-operation authorization use the same implementation.

## Custody and operation

1. The embedding reserves its aggregate storage, workspace, descriptor and traversal capacity before creation. It retains the original parent, generated child name and opened child on that reservation.
2. `OwnedStorePolicy::small_v1` binds the exact authenticated `Context::namespace()` IDs and the receiver limits. Its closed lane input permits one or two distinct IDs. Membership does not grant authorization.
3. `OwnedStoreLease::capture` checks the original descriptors and named child without creating or removing anything. The embedding attaches a lease clone to its reservation before endpoint initialization.
4. `ObjectEndpoint::open_at_with_resource_profile` claims that lease once. It installs the actual opened owner file in retained custody before further validation, holds the kernel writer lock, initializes or validates the profile, and performs bounded recovery.
5. Reads, quota scans, staging, publication and ordered eviction use checked names relative to those descriptors. Begin and Commit each make a fresh quota decision. Before a positive cached receipt, the receiver verifies the bytes and successfully syncs the file, namespace and original root. A failed post-rename sync can leave a named file, but cannot justify a positive receipt until that barrier succeeds.
6. After admission ends and every endpoint and reader is drained, the original owner calls `retire_contents` with exclusive lease custody. It preflights the complete store before deleting data and removes the owner name last. The embedding retains the original locked file through its own parent-relative removal, parent sync and confirmed absence. Failure retains the original reservation and attained resources in quarantine.

The default policy processes at most 101 entries across both lanes and requires funding for 102 observations, including overflow detection. Markers, directories, publications, stages and unknown names share that walk budget. Recovery and retirement preflight also enforce the publication and staging sublimits; Begin and Commit independently enforce publication quota. Cleanup permits at most 103 bounded walks, with no automatic retry after uncertainty.

## Platform and resource boundary

Linux requires working descriptor-based mount-ID lookup. macOS requires the checked local, non-union APFS mount shape. The embedding must own the private mount tree and exclude independent same-UID writers and mount substitution. Relative stat and unlink are not an atomic compare-and-delete against a hostile concurrent host process.

One controller shares its parent descriptor. Each prepared store retains two descriptors, for its child and original owner file; lease clones duplicate neither. Serialized nested traversal is tested with at most four transient descriptors. The embedding must fund the corresponding maximum concurrent stores, iterator allocations, bounded names, metadata, verification buffers and stacks before activation. A descriptor count or observed RSS alone does not establish that memory budget.

The implementation is an opt-in component, not evidence of installed MVM or WAN acceptance. Native component tests cover original-parent renaming and substitution, exact trailing-NUL reconstruction, warm reuse, live authority, independent Commit quota, subprocess interruption, failed sync and original-lock retention. Installed permit integration, native relay acceptance, supported-capacity qualification and final cross-host acceptance remain separate gates. [Approved design](proposals/2026-11-13-unix-owned-store.md) · [Implementation plan](../plans/2026-11-13-unix-owned-store.md) · [Acceptance tracker](https://github.com/R4-Laboratories/SpiralCore/issues/258).
