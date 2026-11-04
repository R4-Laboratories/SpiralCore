# Installed canonical parent provider

Implements the reviewed resident-parent contract under `canonical-host`.
This local API reserves resources; canonical MVM admission grants permission.

`InstalledParentProvider::new(&InstalledSupervisor)` is fallible. It reserves
4 KiB for its bounded two-slot registry before allocating it. `prepare_vm`
accepts a read-only `HostLifetimeIntent`, a bounded stable tenant name and the
actual native backend. The initial profile permits explicitly named transient
VMs, one vCPU and 1–1024 MiB each, at most two live original grants. It reserves
guest RAM and 4 KiB of host bookkeeping from the SAME managed pool, then exports
only the original installation holder. It does not boot or authorize a VM.

The returned non-Clone prepared VM exposes a read-only binding and can be
consumed into the canonical `PreparedHostLifetime`. Its actual reservation is
retained inside that original lifetime. Dropping unused preparation refunds its
reservation. Once launched, original backend custody and the explicit retirement
receipt retain the lifetime until positively completed cleanup; a weak registry
entry, PID or marker is never a retirement proof.

The provider implements Hostd's `ParentAdmissionProvider`. Binding requires the
original private slot/serial and exact installation, generation, name, backend,
CPU/RAM and stable tenant, plus the actual opaque `AdmittedPlan`. It binds once
to the canonical plan identity. Core separately binds the full admitted body
and nonce. The returned current permit retains the SAME reservation, checks
fresh owner/ledger state, and refuses after parent admission closes. No generic
owner, projection or replacement pool creates a permit. Endpoint partition
creation remains explicitly unsupported until its separate held-resource and
disk-cleanup implementation is connected.

Closing admission is monotonic and serialized with slot publication. Existing
charges remain until their actual owners drop. Hostd's install-once registry now
holds a weak reference; the resident retains the actual provider strongly through
operation and shutdown. Each admission call upgrades that same allocation for
its scope. Expiry refuses, and the consumed registry cannot accept a replacement.
This permits the real bookkeeping reservation to drain after its last owner,
without exempting it from accounting. Development source pins do not establish
complete MVM release qualification.

The resident launch transaction additionally retains a strong owner of the SAME
grant before handing the lifetime into canonical launch. `begin_launch` arms
this linear owner and returns it alongside the prepared canonical lifetime.
Both successful original outcomes and uncertain failed launches keep it in the
fixed two-slot resident table. An error, cancellation or Drop after this point
cannot infer absence of native effects. Incomplete Drop quarantines its charge
and exclusion; at most two such guest grants can consume the existing envelope.
Only a matching opaque original `OwnedRetirement` receipt, plus positively
drained endpoint/disk partitions, can complete the owner. All original binding
fields must match. A separate unarmed preparation may be canceled by Drop.
The transaction and explicit completion methods are implemented in the published
development checkpoint. Focused policy, accounting and opaque-admission tests
exercise them without booting a guest. A positive original native retirement
receipt and a connected resident endpoint provider remain execution gates.

The next disk-to-endpoint mapping exposes only a borrowed read-only projection
of the original prepared store: installation/profile/generation, stable tenant,
session, parent and actual reservation serial, generated path and captured
device/inode. Reading this projection performs no filesystem reopen or new
reservation. It cannot construct a disk owner. Core's versioned optional store
locator is derived from this projection and is required for codec endpoints;
telemetry-only endpoints retain their separate no-store profile. The SAME
prepared store/credit must remain in the endpoint guard through private child
handoff and positive original-child retirement. The locator is never permission.

## Positive completion of the original launch owner

The [parent-retirement addendum](installed-parent-retirement-v1.md) specifies the
implemented `finish_retired` and `finish_pending_retired` entries: exact sealed native receipt and actual
parent-permit Arc, private endpoint issuance/cleanup state, and a Retired state
that denies later issuance. Normal admission close permits already-authorized
cleanup. Completion disarms uncertain Drop but refunds only when all original
grant owners drop. Endpoint creation remains Unsupported; the addendum issues
no codec profile or limits, and a positive sealed/native consumer is still a
separate execution gate.

`cancel_unused` is a separate completion path. It borrows the canonical client's
sealed `UnusedHostLifetimePreparation`, compares the full original binding and
the actual private parent Arc, then checks cleanup-safe ledger identity. It
requires no previous admission and exactly NeverIssued endpoint state. A copied
binding, a Mock outcome, an error after conversion, or uncertain Drop supplies no
such proof. Success retires the grant without refunding it before its last owner
drops. The [unused-preflight contract](unused-host-lifetime-preparation-v1.md)
defines the four pre-conversion branches that can return the original lifetime.
