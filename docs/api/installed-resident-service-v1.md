# Installed resident launch service

2026-11-04. Implementation addendum for step 3 of the
[resident-parent contract](../specs/2026-11-03-installed-resident-parent.md).
It composes the existing [parent owner](installed-parent-v1.md),
[retirement](installed-parent-retirement-v1.md) and
[unused preparation](unused-host-lifetime-preparation-v1.md) APIs. It defines no
endpoint profile, new admission authority, native receipt or remote protocol.

## Concrete custody and API

Implement a `canonical-host` resident facade owning ONE original `LocalBackend`,
the SAME supervisor/provider and `[Option<ResidentSlot>; 2]`. Install the concrete
parent provider once, before invocation. The process-wide install registry is
not reset or replaced on shutdown. Its provider entry must be weak, as specified
below, so registration itself cannot permanently retain charged bookkeeping.
Store the backend in one private Arc only to
move that same allocation into owned operation futures; do not expose it or
construct `for_started_vm`, `new` or `with_hypervisor` during cleanup.

Reserve the exact requested inline owner/table metadata from the SAME managed
pool before allocating its Box/Arc or publishing the resident. Retain that
reservation together with the actual provider and its existing bookkeeping.
Before boxing an operation future, cover its concrete requested storage and
retained request/outcome ownership in the original bootstrap/component claim.
`size_of` can establish inline storage, not the heap behind captured Strings,
Arcs or runtime internals. This addendum invents no numeric allowance or DTO
limit: the dynamic bootstrap/resource term remains an explicit activation gate.

Recommended minimal interface (local names may follow existing crate style):

```text
InstalledResident::new(supervisor, backend) -> Result<Self>
begin_launch(&mut self, request, trusted_tenant) -> Result<ResidentId>
request_stop(&mut self, id) -> Result<()>
drive_once(&mut self) -> Result<ResidentProgress>       // async
close_admission(&mut self)
try_finish(&mut self) -> Result<()>
```

`ResidentId` is a bounded slot plus original monotonically allocated grant
serial, only an in-process lookup key. It is not a retirement capability.
Reject stale IDs, duplicate names and a third occupied slot. Do not keep an
unbounded completion/error history. A completed slot can be removed only after
all its original operation/outcome/certificate owners have been dropped.

Before first canonical poll, `begin_launch` checks slot capacity, calls the SAME
provider's `prepare_vm`, and arms `begin_launch`. It installs the actual
`ParentLaunchOwner` and full original binding into the slot before any invocation.
Preparation errors before that armed handoff use ordinary unused Drop. The
resident has exclusive slot mutation; no general caller obtains an outcome,
lifetime, owner or replaceable backend handle.

## Cancellation-safe operation storage

Each slot stores its actual launch or stop future as a pinned owned future,
capturing the same backend Arc and the original input/outcome. The future is a
field of the resident, not a local future created inside `drive_once`. Polling
can use `poll_fn` over that field. Dropping a caller's `drive_once` future stops
driving; it does NOT drop or replace the stored operation or release a slot.
No detached task, spawned runtime or `Send` requirement is necessary for this
initial single-owner implementation. Never hold a parent/ledger mutex across a
poll or perform cleanup within an outcome receipt callback.

Poll both occupied slots fairly, rotating the first inspected slot after each
drive turn. Register both pending futures' wakers before sleeping; readiness of
one must not starve the other's launch/cleanup. No mutex protects a native
operation across await. A bounded poll turn provides scheduling fairness only;
it cannot preempt a blocking canonical method or prove a shutdown time bound.

`request_stop` latches intent even during Launching. It does not abort the
stored launch. Continue polling it to a terminal result, then clean up through
the original backend. Shutdown drives all occupied slots rather than selecting
a cancellation branch that drops losing launch futures. Do not invent a
timeout-success result. A caller may stop driving, but no progress guarantee
is made until the original owner resumes its driver.

| Slot state | Retained custody and permitted next action |
|---|---|
| Launching | Original parent owner, binding and canonical launch future; stop request only latches |
| Running | Original parent owner and actual LaunchOutcome; stop calls original backend `stop_owned` |
| Unused | Original error and sealed unused certificate; call `cancel_unused` before dropping certificate/owner |
| Pending | Converted/error-without-certificate or interrupted cleanup; original parent owner/binding/backend remain; call `stop_owned_pending` |
| Stopping | Stored original stop future and same outcome/binding; caller cancellation preserves it |
| Retiring | Actual outcome with sealed receipt, or retained linear pending receipt; retain any endpoint cleanup owner and retry on error |
| Quarantined | Failed/unknown cleanup, poison or unwind; retain custody, close new admission, no slot reuse |
| Vacant | Reached only after positive unused cancellation or matched retirement and last-owner release |

A pure unused result uses `HostLifetimeLaunchFailure::into_parts`, preserving
the real certificate through `cancel_unused`. An error after conversion cannot
enter Unused. For startup failure use `stop_owned_pending(original_binding)`;
unknown/absent/consumed native history is failure, never proof of no effects.
After a pending receipt is returned, store it before later cleanup and retry
using that same receipt; do not call the consuming retrieval again.

For successful launch use `stop_owned(&original_outcome)`. Only a sealed receipt
in that same outcome can support `finish_retired`. Ordinary `stop_machine`
success, MachineState/PID/name/marker lookup and Mock outcomes are insufficient.
The canonical original stop may have partial progress on Err; retain the same
outcome and retry rather than reconstructing the backend.

## Cleanup, future endpoints and Drop

For a future issued endpoint, retain its actual cleanup guard and move its
sealed endpoint receipt once into that guard. Complete helper/collector/store
cleanup OUTSIDE the outcome mutex. An empty `take_endpoint_retirement` result
does not establish NeverIssued; the grant's shared private issuance state does.
Only the original owner may establish CleanupComplete. Then call the existing
parent completion entry, drop the original outcome/receipt and completed parent
owner, and free the slot after their last grant reference is gone.

Today concrete endpoint partition creation is Unsupported and leaves NeverIssued.
This service must preserve that refusal. No fabricated Small/TLS/stack/carrier
limits or placeholder endpoint may be activated to make native launch succeed.
The facade can be implemented and tested conservatively before those terms are
qualified. Normal admission closure must still permit already-owned cleanup.

`try_finish` first requires no occupied/uncertain slots or stored operations,
then closes the provider and releases its strong provider reference and its
drained table metadata reservation before using the SAME supervisor's existing
global finish checks. Merely closing admission is insufficient. Other real
resource owners and live holders must still make global finish Busy. Dropping
an incomplete resident must not destroy original backend
custody: quarantine the entire bounded inner owner (backend, slots/futures,
provider and supervisor), not only the guest counter. This is fail-closed leaked
custody, not background cleanup; production requires an explicit drive-to-drain
shutdown. Panic/poison closes admission and follows the same quarantine rule.

## Install-once registry must not own permanent charged state

MVM777's `OnceLock<Arc<dyn ParentAdmissionProvider>>` keeps the concrete
provider's Shared alive forever, including its 4 KiB ManagedReservation. Thus
even an empty resident cannot satisfy `resources.ensure_drained`. Do not hide
that charge, exempt it from global zero checks or label local drain a completed
installation retirement.

The implemented prerequisite is an additive ownership correction in canonical
Hostd: store `OnceLock<Weak<dyn ParentAdmissionProvider>>`; `install` downgrades
the same provider allocation, while the resident retains its strong Arc.
`require_installed() -> Result<Arc<dyn ParentAdmissionProvider>>` upgrades it or
refuses. The OnceLock stays occupied even after expiry, so no second provider
can install in this process. No public uninstall/reset/replace API is added.
Callers of install must explicitly retain their provider; tests passing a sole
temporary Arc need a fixture owner, not a permanent registry reference.

This correction is implemented in canonical MVM
`f708539f0c2317acf2e74f8f7833d6072a5abafe`; the initial public777 regression
failed on permanent retention. Successor public-graph execution is pending,
not implied by the source implementation or this decision.

There are three production call sites in MVM777: the canonical client pure
launch preflight, Hostd plan-admission preflight, and `bind_current`. The two
presence checks need no borrowed-reference semantics and can drop the returned
Arc immediately; they do not authorize later work. `bind_current` keeps its
freshly upgraded Arc through synchronous `bind`. The actual returned permit
retains the original grant, not the registry. A provider that disappears between
preflight and binding causes fresh binding refusal, never a fallback. Existing
current checks and monotonic provider close remain authoritative.

Shutdown order is close launch admission, positively drain all slots and their
original grant owners, drop stored operations/outcomes/certificates, then drop
the resident's provider Arc and table metadata charge. A concurrently borrowed
provider Arc may still retain 4 KiB and original authority; global finish must
remain Busy until that actual last reference drops. Retry global finish in a
closed, locally-drained state without recreating the provider or backend. Other
registered factories/owners must independently release their actual charges;
the weak parent entry alone is not proof that the entire installation is zero.

This supersedes the earlier allowance for an idle strong parent registry to
retain exclusion until process exit. Original incomplete/quarantined ownership
still deliberately retains exclusion; only clean local drain releases the
provider. Add tests showing weak expiry refuses, reinstall still refuses,
scoped upgraded Arc keeps the original bookkeeping charged, and last drop
allows the SAME supervisor's real global finish when all other owners are zero.

## Focused REDs before implementation

1. Missing facade API compile RED; then two fixed slots/third refusal/stale ID
   tests with actual managed reservations. Test-only controlled futures establish
   stored-future Drop counters: cancel `drive_once` while pending, assert the
   original operation and parent charge survive, resume and process its result.
   These tests prove driver policy, not native retirement.
2. Actual public canonical pure-preflight refusal returns the sealed unused
   certificate; resident cancels the original owner and refunds only after its
   certificate is dropped. A foreign certificate cannot free it. Preserve a
   same-identity retry test separately; no automatic retry is introduced.
3. An actually admitted Mock outcome cannot produce native completion. A
   converted failure with no original backend record remains charged/occupied;
   repeated stop and global finish refuse, and no fresh backend is constructed.
4. Stop requested before launch completion survives driver cancellation; the
   completed outcome follows original stop, with cleanup-error state retained.
   Do not expose a receipt constructor for a synthetic positive test. Positive
   native success, failed-startup recovery and issued-endpoint cleanup need their
   later actual canonical one/two-guest witnesses on the final source.

Source basis: canonical MVM
`77752e1967d20caf8bad710de864a0ccd967f9e6` (`LocalBackend`, launch/stop/outcome
methods) and installed_parent.rs SHA-256
`2d75dc18d99054666e90de18a4d0f1587c417e3293bb140fc3ea02377eb2e4c2`.
This decision is source-only: no guest, native receipt or build was executed for
this addendum. Existing component records retain their original scopes.

## Implemented lifecycle checkpoint

`installed_parent::InstalledResident` now implements the interface above under
`canonical-host`. Move the original configured supervisor and backend into
`new`; it installs its actual provider once. `begin_launch` returns a local
`ResidentId`, and `drive_once` returns one bounded `ResidentProgress`. The initial
canonical LocalBackend profile uses tenant `local`; the facade refuses a different
tenant rather than changing canonical admission. `last_error` borrows a retained
diagnostic without exporting an outcome, receipt or backend.

An orchestrator can select between `drive_once` and its existing admitted control
input. Dropping the losing driver await preserves the stored operation. On
shutdown, close admission, continue driving original cleanup and call
`try_finish`; an incomplete slot or any actual outstanding global charge refuses.
A caught operation panic quarantines the whole owner before resuming the panic,
and permanently denies subsequent launch, poll and finish calls.

The slot retains its pre-reserved maximum concrete launch/stop-future storage
through cleanup. It does not request fresh operation storage after close.
Inline custody is bounded by the fixed two-slot table; the two documented enum
lint allowances introduce no extra boxes or broad compiler-policy exception.

The [recorded lifecycle results](../network/evidence/2026-11-04-resident-service/README.md)
supersede the source-only execution status for this implemented slice: three
private driver-policy tests and one actual canonical preflight/unused two-slot
case pass. The earlier source-only decision remains above as history. No guest
boots in these tests. Positive native retirement, actual endpoint/store ownership
and complete resource/platform/WAN acceptance remain separate required gates.

## Original helper consumer

The resident's fixed `Live` stores one original `RootHostAgent` beside its SAME
backend and provider, outside individual VM slots. `start_owned_host_agent`
uses the canonical LocalBackend tenant `local`; it refuses after admission
close, any VM operation, or an earlier helper installation. The default missing
qualified profile refuses before effects. This API adds no profile constructor
or numeric/resource activation.

Reserve Live's changed inline layout before allocating it, using its existing
size-based reservation. The provider separately reserves the helper's accepted
claim from the SAME pool before starting it. After successful preparation,
store the actual owner BEFORE obtaining/installing its original registrar into
the existing backend through the nonconsuming SDK API. Installation failure
closes admission and retains the owner/backend for explicit cleanup; do not
reconstruct either. A private consuming downcast adopts the original owner from
typed startup failure. An unrecognized failure is refundable only in the
provider's privately observed Unused/no-attempt phase. Otherwise retain the
error and the whole resident, close admission, and refuse global completion.
Any panic quarantines the whole original live table before resuming unwind.

`try_finish` first closes admission and requires all original VM/endpoint
operations to retire. Then it calls actual helper shutdown outside bookkeeping
locks. Failure preserves `Live`. Only positive shutdown permits dropping the
ENTIRE table, including the backend's registrar reference, before the original
supervisor's global drain check. An external original alias can still keep that
check Busy; after its last Drop, the same supervisor may finish. Incomplete
resident Drop with any helper/error custody quarantines the table even if both
VM slots are empty. Neither EOF nor an empty slot is a helper-exit receipt.

TDD starts with actual E0599 against the previously qualified library metadata.
Focused cases cover missing profile/no effects, VM/closed admission refusal,
poisoned helper-state whole-owner retention, and a separately selected real
canonical daemon/signer case. The latter uses a fresh synthetic home and
protected SAME-revision helper binaries, proves original backend identity and
alias-credit retention across shutdown/drain, and boots no guest. The final locked Linux release scope passes 22 normal parent entries with two
helper cases excluded by that invocation, then explicitly executes the new
canonical-helper case: one PASS, zero ignored. It proves actual daemon/signer
shutdown followed by original backend destruction while an external registrar
alias still retains the original credit; its last Drop permits the SAME
supervisor to finish. A later regression first failed after genuine unused VM
retirement because the original preparation had been forgotten. A monotonic
flag, set immediately after successful prepare_vm and never reset, now enforces
helper installation before the first preparation. Strict Clippy over the
library and test targets exits zero. Source and protected-binary before/after
readbacks agree. Earlier helper-only results retain their original source
identity. These cases boot no guest and add no numeric profile, object-transfer
or WAN qualification. Retained telemetry custody and native guest acceptance
remain separate integration requirements.
