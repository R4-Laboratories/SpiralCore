# Original parent launch-owner completion

Architectural decision for the next installed-parent implementation slice,
2026-11-04. Addendum to [installed-parent-v1](installed-parent-v1.md) and the
[resident contract](../specs/2026-11-03-installed-resident-parent.md).
This adds no endpoint profile, native permission, memory ceiling or wire format.

## Immediate API and authority

Add `ParentLaunchOwner::finish_retired(&mut self, outcome: &mvm_client::LaunchOutcome)
-> anyhow::Result<()>`. It disarms uncertain Drop only after matching the sealed
original native retirement and the grant's private endpoint-issuance state.
The current owner remains armed on every error, poison or unwind.

Failed startup has the additive
`finish_pending_retired(&mut self, &mvm_client::OwnedRetirement) -> anyhow::Result<()>`
entry. It borrows the linear receipt returned by the **same original**
`LocalBackend::stop_owned_pending` and uses the identical full-binding,
parent-permit identity, retained-ledger and private endpoint-state checks.
The caller retains that receipt across failed cleanup and retries; an erased
launch error, absent pending record or replacement backend cannot supply it.
The outcome callback and pending entry share one private validation/completion
implementation; neither creates a receipt or executes endpoint/disk cleanup.

Use a short `outcome.with_owned_retirement` callback to require both:

- Full `OwnedRetirement::binding()` equality with this owner's original
  HostLaunchBinding, including installation/profile/generation/actual serial,
  VM/backend/CPU/RAM.
- `OwnedRetirement::retains_parent_permit` against an Arc trait-object coercion
  of this **same** VmGrant allocation. Equal DTOs, another provider/slot, PID,
  sidecars, an ordinary admitted outcome or generic retained ownership fail.

Leave the outcome mutex before endpoint/disk I/O, other owner acquisition or
resource destruction. The callback performs comparison only. It does not return
a new public proof, adopt metadata, or execute cleanup under that mutex.

## Private endpoint issuance is mandatory

The grant must contain a private, synchronized issuance/lifecycle record shared
with its eventual `prepare_endpoint_partition` implementation. A separate Boolean
on ParentLaunchOwner that the factory cannot update is insufficient.

| Private endpoint state | VM-owner completion |
| --- | --- |
| NeverIssued | Eligible only because this concrete grant has never issued or begun creating an endpoint |
| Preparing / effects uncertain | Refuse; original reserved owners remain charged |
| Issued / cleanup pending | Refuse; retain actual endpoint guard, projection, receipt and optional store |
| CleanupComplete | Eligible only after the original owned endpoint cleanup implementation records positive completion |

For this immediate slice, the concrete factory still returns Unsupported before
reservation/effects and leaves NeverIssued. It must not create placeholder limits,
enable Small/Ordered or infer completion from `take_endpoint_retirement() == None`.
The opaque native receipt does not report whether an endpoint receipt was never
created or was already moved; private issuance state resolves that distinction.

Before a future factory can make any potentially effectful reservation, store
preparation or handoff, it must mark Preparing in this same record. It then
retains the actual root endpoint owner and its exact guard/projection/store.
An unused preparation can return to NeverIssued only through that private owner's
proven no-effects cancellation; an erased error cannot reset it. CleanupComplete
is produced only by methods on that actual owner after sealed matched drain/
Child retirement and, for a store, successful `retire_with_endpoint_receipt`.
No public `mark_done(bool)`, caller-selected projection or serialized record can
produce this state. No endpoint factory is activated by this addendum.

## Completion and lifetime order

After the short native receipt comparison, acquire the private grant lifecycle
record, require NeverIssued or CleanupComplete, and atomically seal the grant
Retired against any later issuance. The same state must make launch-current
checks and future endpoint creation refuse. Only then mark ParentLaunchOwner
completed so ordinary Drop releases its Arc instead of quarantining a clone.
This transition must serialize with the factory's Preparing transition; a
check-then-unlock gap must not allow endpoint issuance after completion.

Normal parent/supervisor admission close is compatible with this cleanup.
Do not call `VmGrant::check_open`, the current launch permit, or the existing
ManagedReservation::check_current as if cleanup were a new launch: those
deliberately reject closed admission. Any added accounting/epoch validation must
be cleanup-safe and use the same original reservation, not a replacement pool.
Accounting corruption, poisoned state and identity mismatch remain failures.

Completion does not forcibly refund a reservation still owned elsewhere. The
outcome's original anchor and ParentLaunchOwner retain the same VmGrant Arc.
After successful finish, the resident removes/drops the original outcome and
completed owner; only the last actual owner refunds the charge and frees its
weak registry slot. Do not assert guests=0 while the outcome still retains it.
Repeated completion must remain harmless and must not accept a foreign outcome
or repeat a resource refund. Global installation finish still separately requires
trustworthy zero ledgers and original holder exclusion.

## Focused TDD and remaining execution

1. Record missing-method API RED against the actual public signature before
   implementing it. Preserve the original uncertain-Drop test unchanged.
2. A genuinely admitted Mock LaunchOutcome has no opaque native retirement;
   finish must refuse, the callback must not invent one, and original charge/
   exclusion remain. Existing MVM tests already establish the underlying mock
   outcome's refusal semantics. Do not expose a test constructor for the receipt.
3. Qualify private state transitions separately: Preparing/Issued/uncertain
   refuse; NeverIssued cannot become issued after Retired; no-effects Unsupported
   leaves NeverIssued; close does not turn unfinished cleanup into completion.
   Label these state-policy tests, not positive native retirement witnesses.
4. When the actual owned backend path can supply its sealed receipt, execute
   matching completion, foreign binding/permit refusal, close-compatible cleanup,
   repeated finish and final-last-owner accounting. With an actual endpoint,
   retain its real root owner through injected cleanup error and retry; no caller
   Boolean or absence may bypass it.

The public finish implementation can land as a conservative component while
endpoint creation remains Unsupported. A positive sealed/native consumer and
the full endpoint resource/collector/source model remain explicit execution and
activation gates. This addendum does not claim they have run.
