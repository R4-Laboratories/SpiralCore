# Original parent bridge for the installed host agent

2026-11-04. Planned Root integration of the existing canonical
[owned host-agent API](installed-host-agent-v1.md). SSC reviewed the direction;
this contract does not activate a profile or assert native completion.

The bridge lives beside the original `InstalledParentProvider` and resident.
It never discovers an agent by socket/PID or calls the legacy ensure/fork path.
One resident owns one actual `OwnedHostAgent` for its declared tenant and keeps
that owner outside individual VM grants. The backend receives only a clone of
that agent's actual `OwnedHostAgentRegistrar` through its existing
`with_installed_host_agent` seam. A clone retains the same original process,
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

Record bounded registration facts only at the existing `bind(&AdmittedPlan,
&HostLaunchBinding)` boundary. The opaque admitted plan supplies tenant,
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
