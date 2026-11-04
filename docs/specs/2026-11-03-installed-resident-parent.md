# Canonical resident parent and typed resource handoff

Status: architect-reviewed concrete implementation contract for the approved installed
lifetime and parent-ledger work. Track #43/#44/#45/#49. This does not replace
canonical MVM admission or create a mailbox, network or owner-access envelope.

## Resident embedding

Use an additive `canonical-host` feature in the existing Rust live/host crate.
It reuses the SAME `InstalledSupervisor`, installation owner and managed ledger.
The legacy demonstration entry retains its existing declared scope. A separate
new owner implementation or copied state parser would split exclusion and is
rejected. A new remote administrative protocol is outside this contract.

The resident facade owns one original `LocalBackend`, installed factory
registries, a fixed two-slot VM table and original launch outcomes. It installs
providers once before launch and retains them for its process lifetime. It does
not reconstruct a backend per operation or use name/PID attachment for installed
stop. Supported native roles are Linux/Firecracker and Apple Silicon macOS/HVF;
an unavailable owned backend refuses before native effects.

Root VM grants are distinct from the existing experimental Sender/Receiver/
Verify worker grants. Each private `ParentVmGrant` holds an actual reservation
from the original managed pool, its unique private slot/serial, fresh installation
binding, exact VM/backend/CPU/RAM projection and stable tenant accounting tag.
The table has at most two entries and does not grant additional resource limits.
The initial native profile remains at most one vCPU/one GiB per VM and at most
two simultaneous guests, with no guest NIC. Requested grants cannot widen it.

Before calling the canonical launcher, the facade reserves the guest envelope
and its bounded host bookkeeping, creates the native lifetime holder from the
same active installation, and installs that grant in its private slot. A
definite pre-invocation failure can release unused ownership. Once invocation
may have created resources, the original typed failure/native owner remains in
the slot until positive cleanup; formatting an error cannot refund it.

## Typed parent gate

The approved lower/upper seam avoids a Core-to-Hostd/SpiralCore dependency cycle:

```rust
// mvm-core::host_lifetime
pub trait CoreCurrentParentPermit: Send + Sync {
    fn check_current(&self, binding: &HostLaunchBinding) -> anyhow::Result<()>;
}

// mvm-hostd::host_lifetime_parent
pub trait ParentAdmissionProvider: Send + Sync {
    fn bind(&self, admitted: &AdmittedPlan, binding: &HostLaunchBinding)
        -> anyhow::Result<Arc<dyn CoreCurrentParentPermit>>;
}
```

The resident facade installs its private concrete provider using the canonical
install-once hostd pattern. Its `bind` receives the actual opaque `AdmittedPlan`
from canonical admission. It locates the exact original VM grant, validates the
same installation/profile/epoch/serial/name/backend/resources and stable tenant,
checks current ledger and original owner state, and binds that grant once to
the actual admitted plan. Other slots, replacement reservations, DTOs and an
independently created quota pool refuse.

The core anchor installs the returned current permit only after binding the
original admission. Native claim and final pre-activation both require it and
check it freshly; absent, stale, foreign or closed permits refuse before the
relevant phase effect. If revocation occurs after native allocation but before
application activation, retain the charged native/startup owner and refuse
activation; do not claim no VM effects or refund that pending launch.
The existing generic retained `parent_owner` establishes ownership only and
cannot substitute for this gate. The concrete current permit retains the SAME
actual parent reservation. A trait implementation is a local trusted provider
extension, not a wire credential or proof that arbitrary external embedders
perform correct accounting; the shipped installed facade registers only its
concrete private provider.

All canonical signed-plan, capability, DNS, rate, transforms, audit and nonce
checks remain under their original MVM authority. The anchor separately binds
the full actual plan body, nonce and plan identity through final native handoff.
Resource admission is subtractive; it never adds a service or permission.

## Endpoint partitions and replies

Before endpoint/helper allocation or spawn, reserve the selected profile's
actual host plaintext, codec, carrier, metadata, explicit stack, disk and worker
claims against the same parent pool. Guest allocations stay inside their already
charged guest RAM envelope and are not double charged as host heap. A concurrency
ceiling does not issue an independent allocation. Public profile counts and
serialized limits remain projections of these held reservations.

Use the canonical endpoint's existing private owned stdin/stdout handshake:
fresh child nonce, immutable parent grant bound to the actual plan/VM/session/
serial, then Ready. The child creates its local subpool before codec/buffer
allocation and readiness. Retain the parent partition through helper join,
store cleanup and all returned reply serialization/write owners. Preserve native
holder continuity through the existing scoped/sudo spawn; do not assume arbitrary
FDs survive privilege wrapping or authorize an alternate launcher.

Mailboxes, FlowMux and owner access have separate queues and policy/lifecycle
under one protocol/core. Missing selected provider/capacity fails before bind or
Ready. No silent raw fallback, alternate guest NIC, unauthenticated console port
or inspection of encrypted Internet traffic is introduced.

## Shutdown and acceptance

Close operation admission and revoke service/flow access before stopping the
original native owner. Positively reap native and helper lifetimes and complete
cleanup before dropping their partitions. Uncertain cleanup preserves custody
and excludes successor admission; no counter reset or timeout-based success.
Global retirement additionally requires the original trustworthy zero ledgers
and kernel-holder exclusion. The parent provider's registry is install-once but
weak: the resident and actual scoped calls own its reservation. After positive
drain, release those owners before global finish. Other actual owners can still
retain exclusion; a stopped API never permits attachment by a second owner.

Focused RED/GREEN requirements: missing provider; generic ownership without a
permit; stale/foreign/substituted plan or slot; aggregate contention; activation
failure and pending cleanup; reply outliving dispatch; original native stop and
holder retention; explicit unsupported role without fallback. Then execute
actual canonical one-guest and two-guest witnesses, death/recovery and final
platform/WAN/capacity qualification with exact bytes and separate CPU/latency/
memory domains. Earlier component tests are not substituted for these gates.
