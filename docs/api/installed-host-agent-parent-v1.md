# Original parent bridge for the installed host agent

2026-11-04. Implementation checkpoint for the existing canonical
[owned host-agent API](installed-host-agent-v1.md). SSC reviewed the direction;
this contract does not activate a profile or assert native completion.

The bridge lives beside the original `InstalledParentProvider` and resident.
It never discovers an agent by socket/PID or calls the legacy ensure/fork path.
One resident owns one actual `OwnedHostAgent` for its declared tenant and keeps
that owner outside individual VM grants. The backend receives only a clone of
that agent's actual `OwnedHostAgentRegistrar` through the additive nonconsuming
`install_owned_host_agent` seam (the consuming wrapper delegates to it). A clone retains the same original process,
holder, parent permit and registration ledger; it starts nothing.

## Admission and resource lifetime

A private `QualifiedHostAgentProfile` has no public/default constructor. Its
shipping constructor remains unavailable until the current helper model's
outstanding terms are closed. Current candidate managed requested storage is
52 MiB (40 MiB shared operations plus 12 MiB retained), with separate explicit
32 MiB Linux / 34 MiB macOS usable thread-stack declarations and two worker
processes. These are model candidates, not measured RSS or accepted capacity.
Retained journal/audit disk authority must be separately held and accounted;
disposable codec storage cannot stand in for it or erase it during cleanup.

Before original agent spawn, reserve the helper's entire selected claim from
the SAME `InstalledSupervisor` managed pool and its actual tenant partition.
Prepare the holder through the original supervisor issuer, binding its original
directory/holder descriptions and unchanged epoch expectation to the canonical
`PreparedHostLifetimeLease`. Do not allocate independent per-process host quotas
or reconstruct holders from paths. Reserve bounded bridge/registry/future storage
before copying tenant, plan or path facts.

The trusted `HostAgentParentPermit` retains that actual reservation and a weak
reference to the original parent registry. Currency checks verify both original
resource identity and current admission/epoch. No booleans, projected limits,
tenant string or digest can construct this permit. Calls may inspect the registry
under short locks; filesystem, process, socket and signed-control operations run
outside those locks.

## Canonical registration

Record bounded registration facts at `bind_config(&AdmittedPlan,
&VmStartConfig, BackendKind)` through the original canonical caller. Its default
implementation preserves the existing binding-only provider behavior. The
installed provider requires that config's actual anchor retain the SAME private
VM grant, repeats `anchor.bind_admitted` against the opaque admission, and checks
config tenant/name and exact admitted resource/backend shape before publishing
its first binding. No current-parent permit exists yet at this stage; comparing
the original erased owner is necessary and exports no owner or authority.

The signed workload label may differ from the generated VM name only on this
verified config/anchor path. The older `bind(&AdmittedPlan,
&HostLaunchBinding)` retains its exact workload/name check. Canonical helper
`RegisterVm.workload_id` remains the actual config VM name, as in MVM's runtime;
the separately admitted plan remains the authorization source. Registration
facts retain only a Weak anchor so the actual grant cannot form a strong cycle.

The opaque admitted plan supplies tenant,
workload, service set and canonical capability/controller-proxy facts. Preserve
the exact original VM grant and anchor identity. Record or retain canonical VM
state-directory and broker-socket authority from that original launch, with
descriptor/identity checks where filesystem effects are possible. A decoded
registration, path name or repeated VM name cannot manufacture that binding.

`check_registration` must find the matching original live grant and compare
tenant/workload/VM, original epoch/serial, services, capabilities, proxies and
original state/broker paths. It must reject unsupported selected services,
foreign scope, stale/retired grants and modified facts before registration or
endpoint activation. Canonical MVM admission remains the authorization source;
the parent budget can restrict it but cannot grant missing services.

The runtime's installed broker seam calls `registrar.register` only through
this actual registrar and retains the returned `OwnedVmRegistration` immediately
in its original launch custody BEFORE endpoint provisioning or guest boot.
Uncertain registration retains its typed cleanup owner. A fresh per-registration
check before Ready verifies original registrar identity, exact active serial and
unchanged canonical registration facts, then original parent/holder/process
currency. The local registration commitment is an identity check, not authority.
No installed failure falls back to legacy discovery, registration or defuse.

## Shutdown, errors and resident custody

Canonical per-VM stop positively retires its own signed registration and endpoint
before final native retirement can release the original VM grant. Closing parent
admission must still permit cleanup through retained original handles; cleanup
must not request a new grant. Independent mailbox, network, access and control
queues and permissions remain distinct.

Only after both original VM slots and registrations retire may the resident
shut down its actual host agent. Require positive original daemon AND signer
cleanup through `OwnedHostAgent::shutdown` before refunding its helper reservation.
Retained audit/telemetry storage follows its separate retention policy. It is
not deleted or refunded merely because a codec pool drains.

Failed spawn/registration carries typed original owners where available; no
type-erased error path may refund uncertain effects. Cancellation retains the
same stored operation. Panic, poison, timeout, failed signed deregistration or
unproven process exit fences admission and retains the actual owner/charge.
Incomplete resident Drop quarantines bounded original custody rather than
claiming successful shutdown. PIDs and marker removal are never exit evidence.

## Required implementation evidence

Focused tests first refuse a missing/unqualified profile with zero new child or
resource effects. Exercise wrong original pool/tenant/VM/epoch, altered admitted
facts, duplicate helper configuration and revocation while preserving cleanup.
Use actual helper processes for registration/independent stop, uncertainty and
daemon/signer exit. Preserve original admission and failed controls in evidence.

Then run the actual installed native path: one guest transfer with exact bytes
and lengths, full original endpoint/registration shutdown and durable retirement,
followed by two guests and failure/recovery. Record both-direction bytes, setup
and endpoint/host CPU, verified transfer/lifecycle time and memory domains.
Component tests or a typed API alone cannot satisfy these native/WAN gates.

## Config binding implementation checkpoint

The original public f708 dependency lacks `bind_config`; an actual Rust metadata
probe reproduced E0599 against its unchanged compiled Hostd library before this
consumer was written. The local successor adds that trait seam and an identity-
only Core anchor accessor. New tests use actual synthesized/admitted plans and
two independently prepared original owners with equal projected bindings.
They cover generated VM names, foreign owner refusal, signed body/tenant
substitution, one-shot registration and the actual metadata charge. An isolated Linux successor build passes 19 parent checks, including these
four new cases; one real-helper case remains ignored by that invocation.
Independent review caught an ordering defect: registration preflight originally
consumed the binding before it could refuse. Actual execution reproduced that
failure before the fix. The registration candidate and its real 32 KiB claim
now prepare before the one atomic plan/session/registration commit; failed or
losing preparation releases outside the lifecycle lock.

The three API additions are published in MVM development checkpoint
`bef4146c0de18b22ac392118acfd898a369cc64c`. Adapter dependency binding
`7f638428522e70b36a82d68700a6374267ea7e40` preserves its original product/test
bodies and all locked package versions while repairing older source identities.
A mixed graph first resolved two MVM Core/Contract identities and was rejected.
The corrected public Git graph resolves one of each and one shared transfer
core without path overrides. Its locked release build passes the same 19 parent
checks (one real-helper case remains ignored), and strict Clippy over the library
and test targets exits zero with 13 acknowledged Rust warnings. All 131 supplied
source inputs match candidate `8ff9e559`; the fetched SDK tree matches its pinned
commit. The [recorded packet](../network/evidence/2026-11-04-parent-config/README.md)
retains both diagnostic and final Git scopes, original failures and exclusions.
No VM registration, native guest or object transfer is exercised by these tests.

Retained telemetry is a separate disk domain. The canonical projection's cache
total includes the 8 MiB records cap; disposable codec storage must receive only
the checked remainder. Endpoint Drained and original VM retirement do not prove
that retained encrypted records or their wrapped key may be deleted or refunded.
Actual retained-storage custody and durable accounting must survive those
transitions. Canonical installed stop preserves the VM state directory, while
later generic reconciliation can recursively remove it; retained installed
scopes need protection or an authorized durable transfer before that path can
run. A credit-only split without actual retention ownership is insufficient.
