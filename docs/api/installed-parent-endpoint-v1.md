# Original installed-parent endpoint owner

2026-11-04. Implementation addendum to the
[resident service](installed-resident-service-v1.md) and
[parent retirement](installed-parent-retirement-v1.md) contracts. This connects
actual parent reservations/store ownership to canonical endpoint handoff; it
does not qualify new resource numbers, native permission or positive retirement.

## Concrete objects, with no strong ownership cycle

Add a private `RootEndpointOwner` implementing canonical
`EndpointPartitionGuard`. The SAME Arc is retained by GrantLifecycle and moved
into `PreparedEndpointPartition`; do not create a second equivalent guard for
cleanup. Suggested ownership:

```text
VmGrant -> lifecycle -> Arc<RootEndpointOwner>
RootEndpointOwner -> Weak<VmGrant>
RootEndpointOwner -> SAME VM ManagedReservation clone
RootEndpointOwner -> endpoint memory/workers reservation
RootEndpointOwner -> PreparedEndpointStore (disk credit + inventory workspace)
RootEndpointOwner -> exact immutable EndpointPartitionProjection
RootEndpointOwner -> private cleanup state
ParentLaunchOwner (or resident retirement state) -> optional sealed EndpointRetirement
```

The VM reservation clone retains the original reservation allocation, not a
new claim. It preserves guest credit if a guard outlives the VmGrant. The weak
back-edge refuses current use if its original grant disappears and avoids
Grant→guard→Grant cycles. Use a private self Weak initialized with
`Arc::new_cyclic` (or an equivalent private identity-preserving construction),
never locate/adopt a replacement grant from equal metadata. Prepared partition,
handoff and retirement owners already retain this guard and the original
canonical anchor through their existing APIs.

The sealed receipt MUST NOT be stored inside RootEndpointOwner: it itself owns
the same guard Arc, so doing so creates Owner→receipt→Owner. Retain it separately
in ParentLaunchOwner or the resident retirement state, already alongside the
original VmGrant. Cleanup borrows that receipt without moving it into its guard.

RootEndpointOwner's current check compares the full immutable projection,
upgrades the original Weak, validates original grant admission/current identity
and its own reservation, and accepts only the live Preparing/Issued policy
explicitly needed during construction. Cleanup is separate: it uses the SAME
reservations' `check_retained_identity`, never new-launch checks after close.
Its private store mutex protects serialized cleanup; perform disk I/O
without holding GrantLifecycle, outcome or resource-ledger mutexes. The existing
installation owner's filesystem mutex still serializes its disk transaction.
Do not nest the store/cleanup mutex with GrantLifecycle in either direction;
release it before the final lifecycle transition. A cleanup error never advances
that shared completion state.

## Qualified input, not a quota DTO factory

Add a private retained `EndpointOwnerInputs` to the original provider, containing
the SAME global pool, SAME original EndpointDiskInventory, and an optional
private `QualifiedEndpointProfile`. The profile is constructed only by trusted
installed configuration after the actual selected bootstrap/codec/TLS/carrier/
reader/relay/collector model is accepted; it contains the explicit Core limits
and selected profile/version. No public `qualified: bool`, arbitrary limits DTO,
guest header, model digest or public profile constructor grants resources.

Implement the full owner path now. An absent qualified profile refuses BEFORE
Preparing/reservation/filesystem effects. This is a temporary activation gate
on a real implementation, not a permanent Unsupported stub or guessed ceiling.
The future accepted model supplies concrete data to this existing path. Do not
enable native flags merely because component policy tests pass.

Reserve host plaintext/codec/carrier/metadata/stacks/workers once from the SAME
pool for the selected endpoint profile. Disk inventory.prepare separately
reserves exact cache/staging bytes and its existing store bookkeeping; do not
double-charge those disk fields in the endpoint claim. Inventory's 512 KiB
workspace remains retained through its actual stores, as already implemented.
Account owner/Arc/projection/cleanup-capture allocations BEFORE creating them
using the accepted metadata term; cloning a reservation changes no quota.

Bind full HostLaunchBinding, actual admitted plan ID, actual original tenant,
fresh partition reservation serial, and a nonzero endpoint session tied to the
actual admitted execution/nonce. Retain any needed canonical nonce/session facts
at `bind(AdmittedPlan, binding)` from that actual object. A local identifier or
domain-separated derivation is identity only, never a new signed authority.
The store request uses that tenant tag, session and original parent serial.
Its locator is projected solely from the actual prepared store binding.

## One issuance transaction

Override `VmGrant::prepare_endpoint_partition(&self, binding)`:

1. Require exact original binding, fresh current grant, actual admission,
   accepted profile and NeverIssued/non-Retired state. Under GrantLifecycle,
   transition to Preparing before any effectful reservation or store work.
2. Release GrantLifecycle. Reserve endpoint claim, prepare the actual store,
   derive the exact projection and create the one RootEndpointOwner. Retain
   an armed private attempt through errors/unwind. No erase-and-reset on Err.
3. Publish that strong owner and Issued under GrantLifecycle, then RELEASE the
   lock before `PreparedEndpointPartition::new(projection, same_guard)`, because
   its constructor calls guard.check_current and may reenter grant checks.
   Only after successful Core validation return the actual partition.

Preparing is a one-shot claim and rejects a second issuance. Concurrent close
may make final current checks refuse, but must retain any actual effects and
reservations. Before effects, only privately proven no-effects cancellation may
restore NeverIssued. If the existing disk API erases its may-have-effects phase,
treat any returned preparation error conservatively as Uncertain; do not infer
safe reset from missing files. A private no-effects typed result may refine
this later. Explicit store.cancel_unused is valid only before any partition
escapes and only when its positive result is retained through the state update.

Core may reject after the partition was returned but before endpoint handoff
completes. Absent a canonical unused-endpoint certificate, that state remains
Issued/Uncertain and charged. None/absence cannot prove no helper/store effects.
Neither a timeout nor Drop transitions to CleanupComplete.

## Receipt consumer and original resident wiring

Add `ParentLaunchOwner::retire_endpoint(&mut self, receipt:
EndpointRetirement) -> Result<()>` or a private equivalent which first verifies
receipt full projection and `retains_partition_guard(&same_original_guard)`
before recording it beside the grant in ParentLaunchOwner or resident state.
On failure to accept a by-value receipt, return it or retain it in that outer
owner; never erase linear proof.
Prefer a mutable Option input which is taken only after identity validation.

`RootEndpointOwner::cleanup_retired(&EndpointRetirement)` borrows the outer
owner's retained real receipt, checks
cleanup-safe reservation identities, and calls
`PreparedEndpointStore::retire_with_endpoint_receipt` with the SAME guard Arc
and full expected projection. Failure keeps receipt, store, memory/worker and
disk ownership for retry. Success records CleanupComplete under GrantLifecycle,
then releases only the actual completed owners. Keep memory/workers charged
until all original guard/handoff/receipt references have dropped; don't force a
refund while a canonical receipt still owns the guard. No public mark_done,
caller bool or copied projection can complete this state.

Wire both resident retirement states BEFORE existing parent finish:

- Successful outcome: use `take_endpoint_retirement` to move the sealed endpoint
  receipt once, outside any parent lifecycle lock. Store it in the outer owner,
  finish disk cleanup outside the outcome mutex, then `finish_retired(outcome)`.
- Failed startup: keep the original mutable OwnedRetirement in the slot; use
  its `take_endpoint_retirement` once, retain proof beside/clean the same owner, then call
  `finish_pending_retired(&receipt)`. Retain both receipts across failed cleanup.

Missing endpoint receipt is acceptable ONLY for the grant's private NeverIssued
state or because its same outer owner already accepted that exact receipt.
Issued/Preparing/Uncertain with no retained proof refuses. Once CleanupComplete
is recorded, parent completion seals Retired against all future issuance.
Ordinary admission close does not block identity-validated cleanup; poison,
counter corruption, foreign guard and replaced store remain failures.

## Focused TDD before product

1. API absence for private owner preparation/cleanup policy, then capacity gate
   refusal before ledger/disk mutation; NeverIssued→Preparing→Issued and second
   issuance/Retired refusal. Use actual original pool/inventory and admitted
   object. Private state tests are not native receipt witnesses.
2. Real store/credit lifetime: same guard Arc survives external partition Drop;
   no strong cycle; parent Drop/guard Drop cannot refund uncertain disk. Verify
   normal-close retained-identity cleanup and missing/foreign proof refusal.
3. Actual low-level injected disk sync failure preserves the original credit,
   store phase and retained receipt ownership. Repeated owner cleanup must not
   take a receipt twice or publish parent completion early. Do not add a public
   receipt constructor; qualify positive receipt flow later through the actual
   canonical owned child/helper path, with native gates explicitly separate.
4. Resident policy adapters consume each real optional endpoint receipt at most
   once and retain it on error. Existing NeverIssued/unused tests stay valid;
   no old component test substitutes for issued-endpoint positive retirement.

This addendum is source-only; no product edit, build, guest or positive native
receipt was executed for it. Numeric profile activation remains dependent on
the independently completed actual source/model terms, not on this topology.

Read source identities: installed_parent.rs `d23f4a00…cce45a`, installed_disk.rs
`c074f17f…b1250a`, canonical Core endpoint.rs `4e6516b8…c096da` and VMM
owned endpoint retirement `620bf7f4…26f58`. The canonical public types are
`EndpointPartitionGuard`/`PreparedEndpointPartition`; this contract adds no
competing Core-prefixed proof type.

## Implementation checklist at the frozen resident boundary

Reviewed plan against Parent d23f4a00, Disk c074f17f and Resident d09fb8ea:
the proposed lazy optional Shared inputs, private-test-only profile construction,
new_cyclic weak identity and separate outer receipt are compatible. This is a
plan decision, not approval of product source that has not yet been reviewed.

- An absent production qualified profile must refuse before constructing the
  lazy inventory (which itself reserves 512 KiB), Preparing, or any endpoint
  allocation. Tests can construct a private profile under cfg(test); do not
  expose arbitrary-limit construction or a caller-set qualification Boolean.
- Shared owns the same original inventory/pool only while needed. Its lazy
  inputs and endpoint metadata have real last-owner release at resident drain;
  a global strong registry must not recreate the already-fixed permanent charge.
- The attempt must retain every acquired original owner through partial Err or
  unwind. Do not rely on local reservation destructors after the store may have
  effects. Keep an actual partial store/owner when available for later retry;
  an erased disk-preparation error with unknown effects remains quarantined.
- RootEndpointOwner Drop must not normally refund its endpoint memory/workers
  or its clone of VM credit while its effects are uncertain. PreparedEndpointStore
  already quarantines disk credit; that alone does not protect the new fields.
  Positive private no-effects cancellation or CleanupComplete disarms this
  disposition. No-effects means actual private phase evidence, not path absence.
- Publish exactly one owner/serial under lifecycle serialization. Its projection
  must match the reservation actually obtained, actual admitted identity and
  store binding. Core validates projections but does not turn them into credit.
- Mark cleanup phase positively after successful original disk barriers; release
  the store mutex before marking GrantLifecycle complete. A poisoned lifecycle
  after successful disk work still refuses parent completion and retains outer
  proof; it never refunds by a guessed success path.
- Extend Resident Slot/ParentLaunchOwner requested-storage accounting for added
  receipt/owner fields. Existing size_of-based layouts follow their changed
  sizes, but dynamically allocated owner/projection/captures require the new
  accepted endpoint metadata term before construction. Do not infer that old
  d09 inline numbers cover future heap allocations.

Minimal added regressions: profile-absent snapshot and filesystem unchanged;
second concurrent Preparing refusal; close during preparation retains effects;
partial-preparation/unwind Drop retains exact credit; foreign receipt Option
remains Some on rejection; accepted proof is retained across retry; same guard
Arc identity with foreign equal projection refuses; no owner/receipt cycle;
and normal close never changes cleanup to a new-admission request. Actual native
positive receipt tests stay later; private tests must not add public proof
constructors or relabel policy transitions as helper/native exit evidence.
