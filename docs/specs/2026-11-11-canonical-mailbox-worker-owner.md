# Canonical mailbox worker ownership

Status: proposed ADR and executable implementation sequence, 2026-11-11. This
is not an implemented control protocol or a successful native retirement result.
Source inspected: MVM SDK contribution `18b58e1ad2262a1c0bfa23d51f8e5679d3376f60`
on upstream `28afdfe9ef7e36518ca42788d57c54f1d7275f57`.
Coordinates [mailbox service #4629](https://github.com/tinylabscom/mvm/issues/4629)
and [SpiralCore delivery #42](https://github.com/R4-Laboratories/SpiralCore/issues/42).

## Observed ownership gap

`mvm-hostd/src/mailbox_process.rs::spawn_serving` drops its original `Child`
after returning a PID. PID-file publication can itself fail after spawn without
collecting that child. `mvm-vmm/src/host/vm_helpers.rs::reap_vm_host_helpers`
calls `gpu_endpoint_spawn::reap_pid_file`, which signals and polls a numeric PID
and does not collect the original child. Absence of a PID file is not evidence
of collection. Tests that separately call `Child::wait` mask this distinction.

The actual resident registration owner is `HostAgentDaemon` in
`mvm-hostd/src/broker/daemon.rs`: `vms: HashMap<String, VmHandle>` owns broker
socket/task handles; `registrations: HashMap<String, RegisterVm>` contains
journaled registration DTOs. `broker/registry.rs` is not this owner.
`VmHandle::drop`, deregistration, replacement and dead-registration pruning
currently drop/unbind registrations without a mailbox-child collection step.

Do not assume every launch has that registration. The production
`mvm-runtime/src/workload_runner/runner/broker.rs::RealBrokerRegistrar` returns a
no-op for empty services or missing tenant/socket; it can select the per-VM fork
when the resident daemon is disabled. `RegisterVm` has no canonical execution
epoch, admitted-plan binding or opaque owner lease. Its `ControlResponse::Ok`
is a unit variant. The daemon has a public verifying key and signer-helper
socket, not an in-memory host signing seed or mailbox encryption root.

## Decision and compatibility

The existing tenant host-agent must be the actual parent of mailbox workers for
both resident SDK and detached CLI launches. Retain one bounded worker slot in
the original live registration, not a process-global worker map. A mailbox-only
launch explicitly establishes this owner even when broker services are empty;
it does not grant guest broker services. Missing tenant, disabled/unavailable
owner, unsupported version, unavailable FD delegation or unavailable shared budget
refuses a mailbox-granted launch before any worker spawn. Ungranted legacy paths
retain their existing behavior.

Do not alter the serialized shape or JCS bytes of any existing `Register`,
`Deregister`, `HvfInstanceV1` request or `SignedControl`. Propose a new
`ControlRequest::MailboxWorkerV1` tagged variant, with its own tagged operation
and literal version/domain. Sign it through the existing canonical host-control
sign/verify functions. The new variant tag and explicit domain bind this purpose;
do not change the signing domain of old messages. Older daemons reject the new
variant; callers must not fall back to PID-only ownership. Add a new typed
response variant; leave unit `Ok` and legacy response parsing exact.

The new operation must not accept `DerivedMailboxKeys`, a caller-selected helper
binary, arbitrary state paths, raw PID ownership, guest-supplied grants or an
unverified plan JSON blob as authority.

## Proposed bounded control contract

Before wire implementation, coordinate these DTOs with Ari. Names below are
proposed types, not existing APIs. Every request has domain
`mvm-mailbox-worker-v1`, version 1, bounded request nonce and operation, inside
the existing host-signed envelope and 64 KiB control frame cap.

- `Acquire`: host-attested tenant, VM ID, workload ID, exact admitted-plan
  commitment, canonical `MailboxGrant` (including execution epoch), absolute
  admission expiry and admitted backend route kind. The sole production builder
  consumes/references the canonical `AdmittedPlan` and original start context;
  it cannot synthesize an epoch from a name or PID. Validate tenant/workload,
  grant authority and plan/epoch consistency before creating an owner slot.
  The daemon resolves canonical paths and backend route from that bound context;
  it never uses an arbitrary request path to mint a secret or launch a worker.
- `AcquireResult`: daemon boot nonce, cryptographically random owner lease ID,
  registration generation and immutable context commitment. The generation is
  daemon-authored, unique to this acquisition, not a hash of a VM name/PID.
  Generation is distinct from the canonical mailbox execution epoch.
- `Start`, `Query`, `Stop`: signed tenant/VM/workload, daemon boot nonce, lease,
  generation, context commitment, request nonce and bounded operation deadline.
  These must match the current slot. Possessing a lease file alone grants no
  authority; every operation still requires canonical host signature/current
  authorization. `Query` is required after a lost Start or Stop response.
- `WorkerResult`: the same immutable identity and request correlation, plus
  `Reserved`, `Starting`, `Serving`, `Stopping`, `Collected` or `Unresolved`.
  Only `Collected` contains the actual original child's collected status.
  PID and signal success are diagnostic, not alternative terminal results.

Host-control authorization currently trusts its host signing key. This proposal
does not magically prove the daemon independently reverified an entire execution
plan: the new signed Acquire is an explicit host admission attestation produced
only after canonical admission. Bind its exact schema/commitment representation
and validate it in the daemon. Any stronger independent plan verification needs
its own implementation and test; do not call a public digest a proof.

Use actual canonical Unix time for admission expiry and a trusted daemon-local
monotonic origin for operation deadlines. Check expiry/current authorization at
Acquire and again immediately before Start. Bound each control request's future
deadline to five seconds; do not translate absolute Unix expiry into a synthetic
epoch or use a caller-supplied monotonic clock. Stop remains authorized for its
existing owner to clean up after grant expiry; expiry must not strand a child.
Do not retry a lost Start with a fresh lease. Identical request nonces recover
the same slot/result; conflicting reuse refuses. Retain only bounded current
operation/result state per slot, not an unbounded nonce history.

## Resource, custody and cleanup contract

Acquire must reserve from a shared host worker budget and tenant sub-budget;
separate per-tenant daemon limits do not establish a host aggregate limit. Require
an explicit shared budget provider/configuration; initial test configuration is
four host workers, at most two per tenant and one per registration. These are
test bounds, not measured capacity. Reserve startup bytes/work and worker credit
before spawn. A 64 KiB frame cap is not a total resident-memory budget.

Each slot owns an `OwnedMailboxWorker` with the actual `Child`, platform exit
observer, collected status and reservation. Reuse the lifecycle pattern in
`mvm-vmm/src/host/network_endpoint_spawn/owned.rs` and
`mvm-vmm/src/host/process_exit.rs`; do not reuse the network endpoint protocol.
Arm observation before advertising Serving or requesting Stop. After a process
event, collect only through that original `Child`. Preserve wait/observer errors
and retain custody/credit as Unresolved. No generic waitpid, detached waiter
thread or numeric-PID reconstruction can manufacture a collection receipt.

Every failure after spawn, including config write and PID publication, goes
through owned cleanup. Return startup failure with the original slot retained
until collection, not an ordinary error that drops its last owner. No blocking
spawn/config write/exit wait/key-provider I/O under the daemon mutex: mark and
loan the slot's custody under lock, perform the bounded operation outside it,
then restore/check the exact generation. While loaned, conflicting operations
refuse or await an explicit completion event; they cannot replace the slot.

Legacy deregistration, re-register replacement and dead-registration pruning
must not drop a new worker-bearing slot. Collect first, or return/retain an
explicit Unresolved slot. Idle shutdown counts reserved/unresolved workers.
Release worker credit only after collection; release terminal slot state only
after explicit retirement/current-generation teardown. Query is retryable until
that retirement. Further queries return Unknown, never a fabricated old success.

## Key boundary requiring coordination

Do not grant the daemon read access to host signing/encryption roots. The
preferred bounded prototype delegates the original child's stdin write end to
the existing canonical launcher, which already derives launch-local keys. It
writes the existing `MailboxProcessConfig` JSON directly to that pipe, not to
host-agent control. The concrete addendum below replaces the earlier proposal
for a new key-provider helper operation. No new key derivation or root-key
reader is needed. Its ancillary transport and helper mode still require Ari's
wire-contract review before implementation or activation.

## Real call-site propagation

SDK persistent launch is `LocalBackend::launch` -> `boot_persistent` ->
`boot_spec` -> canonical `machine_start` ->
`start_persistent_oci_machine` -> `finish_legacy_persistent` ->
`start_prepared_custodied` -> `AnyBackend` workload runner. Its existing mailbox
spawn follows backend start. `StartedVm` then leaves scope when only
`AdmittedPlan` is returned. Thread the acquired owner context through canonical
start resources, not just `StartedVm` or a new LocalBackend-only map.

CLI admitted boot also reaches canonical start and the workload runner; its
post-start `plan_admission.rs` mailbox spawn must use the same acquired lease.
The runner's existing broker registration alone is insufficient because of its
empty-services and legacy-fork branches, and currently occurs after guest
activation (`runner.rs::register_workload_broker` follows `activate_workload`).
CLI persistent `commands/machine/lifecycle.rs::start_machine` directly calls
`mvm_client::launch::machine_start::start_machine_spec`, so it shares the
persistent branch above; it does not necessarily use the separate
`plan_admission.rs` post-start branch. Cover that separate admitted-boot branch
as well rather than treating its test as CLI-persistent evidence.
Add an explicit mailbox-owner acquisition
seam before backend activation for BOTH entry paths, without admitting broker
services. Test both public paths; do not infer one from the other.

Persist only a private, bounded owner reference in canonical VM runtime state
after Acquire: daemon boot/tenant/VM/workload, lease/generation and context
commitment. It is a locator, not authority. Cross-client `stop_machine` resolves
this reference and sends freshly host-signed Stop/Query to the original daemon;
reconstructing `AnyBackend` does not create an owner. Collect the worker before
deleting the reference/runtime state or releasing its reservation. Update the
supervisor expiry and rollback paths to use the same operation.

If the daemon restarts, its boot nonce changes. Existing registration-journal
replay must NOT recreate a Child or mark a worker collected. Persisted pending
worker admission remains Unresolved and charged until an explicit recovery
policy resolves it; no automatic respawn/epoch reuse. The first slice must refuse
reattachment and expose this gate rather than claim crash recovery qualification.

## Small executable sequence

Use focused RED before each implementation. No VM is needed for steps 1-4.

1. **Contract, high risk:** `mvm-contract/src/protocol/broker_control.rs` and
   `mvm-core/src/protocol/broker_control.rs`: golden old serialized requests and
   signatures; new operation domain, correlation, tamper/expiry/tenant/epoch and
   stale-boot refusals. Add bounded DTOs only after Ari reviews FD delegation and
   host-admission attestation. Existing unit Ok remains identical. This schema
   slice must remain inactive until the following steps land.
2. **Owner, high risk, depends on 1:** `mvm-hostd/src/broker/daemon.rs` plus new
   `broker/mailbox_workers.rs`, `mailbox_process.rs` and focused tests. Exercise
   a real daemon and real child with the direct stdin delegation below. First RED asserts
   canonical Stop returns original collected status without the test waiting
   that child. Add post-spawn publication/config failure and natural-exit tests.
   Prove host/tenant exhaustion, failed cleanup charge retention and lock-free
   external waits. This is a wired daemon control test, not an unused owner unit.
3. **Public callers, high risk, depends on 2:**
   `mvm-vmm/src/host/host_agent_spawn.rs`,
   `mvm-runtime/src/workload_runner/runner{.rs,/broker.rs}`,
   `mvm-hostd/src/plan_admission.rs`,
   `mvm-client/src/launch/persistent.rs` and `mvm-client/src/local.rs`.
   Missing-services mailbox launch RED must acquire a worker owner; disabled
   daemon and unsupported delegation must refuse before backend activation.
   Two real control-client processes prove detached launcher exit followed by
   independent Stop/Query reaches the original child owner. Resident SDK test
   uses the same daemon and stop API. Ordinary no-mailbox controls remain green.
4. **Teardown, high risk, depends on 3:**
   `mvm-vmm/src/host/vm_helpers.rs`,
   `mvm-hostd/src/supervisor/wall_clock.rs`, daemon replacement/pruning and idle
   watcher. RED all premature slot/reference/credit removal paths. Test concurrent
   stop, lost response, restart/stale lease, cleanup timeout and repeated Query.
   Never convert missing daemon/PID sidecar into Collected. Integrate the reviewed
   real descriptor handoff before enabling production acquisition/start.
5. **Native preflight, medium risk, depends on 4:** exact source tests/Clippy and
   independent code/security review, then one bounded canonical Firecracker
   mailbox attempt. Record worker launch/collection, exact payload/readback,
   both-direction bytes and CPU/time separately; preserve prior zombie/failure
   evidence. WAN/platform/fleet acceptance remains separate.

Each implementation step should be split into small test-first commits; the
cross-file integration cannot be released as supported ownership until all
production callers and teardown paths are wired. No product file was changed
or VM launched in preparing this ADR.

## Preferred prototype addendum: direct stdin descriptor handoff

This is the concrete alternative for coordination against current composed MVM
`89c2413`, not a new crypto/key-provider design. Keep all old serialized requests,
JCS signatures, default helper invocation and config JSON bytes unchanged.
Linux and macOS local hosts use standard Unix ancillary descriptor transfer;
Windows remote clients call their admitted remote host and receive no local FD.

**Control DTOs.** `MailboxWorkerV1` wraps `{domain, version:1, request_nonce,
tenant, vm_id, workload_id, plan_commitment, mailbox_grant, admission_expires_at,
operation}` inside existing `SignedControl`. Acquire has no keys/paths/PIDs.
Its response names daemon boot nonce, random lease and generation plus exact
context commitment. `DelegateConfig` and `Query`/`Stop` carry that tuple and fresh
signed request correlation; context equality includes the canonical grant epoch.
The daemon returns a typed `MailboxWorkerV1Result`, never legacy unit `Ok` for a
new operation. Acquire only reserves; DelegateConfig spawns once and passes the
write FD once. Duplicate Acquire/DelegateConfig/Query recover the same state and
must not spawn or delegate another writer. An uncertain/lost FD handoff times out
to owned cleanup, not a second writer or automatic retry with a new generation.
The existing five-second operation deadline and 64 KiB frame bound apply.

**Wire and FD association.** Keep the existing four-byte big-endian JSON-length
prefix. Requests carry zero descriptors. Every new-mode result first sends one
marker byte `0x4d` with sendmsg, then the bounded typed JSON frame on that same
dedicated request connection. Ordinary/error results carry zero descriptors;
only the first successful DelegateConfig carries one `SCM_RIGHTS` integer. The
receiver uses recvmsg for the marker before reading the frame, requires one
marker byte, then validates FD count against the decoded result: exactly one
for first delegation, zero for every other result. Reject
unexpected ancillary records and `MSG_CTRUNC`/`MSG_TRUNC`, and close all received
FDs on every refusal. It checks the complete signed-request correlation and
response tuple before exposing the FD. The marker is not authentication. Use
verified same-user/root peer credentials and the existing private tenant UDS;
derive trust from verified host request and original daemon connection, not a
user-supplied socket path. No generic JSON reader may consume the rights marker.
Old operations use their original response shape and no marker.

The daemon owns `Child` immediately after spawn; take its `ChildStdin`, pass only
that anonymous pipe's writable end and close the daemon's copy after successful
sendmsg. Validate received FD as FIFO/write-only, set CLOEXEC immediately (atomic
receive where supported), then nonblocking bounded writes. No pipe read end,
lifetime channel, helper stdout, process handle, root/key file or control socket
is delegated. Partial send/serialization failure retains the slot/Child and
reservation and runs bounded owned cleanup. Descriptor transfer success alone
never means the config was written or the helper is ready.

**Private helper protocol.** Add explicit `mvm-mailbox --owned-v1`; legacy no-arg
mode stays byte-compatible. The daemon also creates an exclusive socketpair,
retains one end and passes the other as fixed child FD3 with all unrelated FDs
CLOEXEC. FD3 carries bounded nonsecret `PrepareOwnedV1` then `Ready`/`Failed`
frames (four-byte length, at most 4 KiB). Prepare binds the exact owner tuple,
grant, derived canonical state/socket/route and expected parent; it contains no
keys. The launcher uses existing `build_config`/`derive_launch_keys` and writes
existing config JSON directly into delegated stdin, closes it and releases its
temporary secret buffers. Secret bytes never transit host-agent control or FD3.
The helper requires EOF after at most 64 KiB config bytes within five seconds,
compares config grant/route/state exactly to Prepare before serving, and rejects
duplicates/unknown mode or mismatches. The daemon cannot treat a launcher-written
success flag as readiness.

Current `serve_derived` returns a `MailboxLaunch` immediately and retries bind
errors inside its thread; there is no synchronous `Ok` readiness today. Add a
canonical owned-mode readiness result only after `bind_grant` succeeds and its
actual service is installed; route failure remains a delivery failure, not a
false claim of guest connectivity. The helper emits context-bound Ready on FD3
only after that result. Bind/config error or Ready timeout produces Failed and
owned stop/collection; pipe EOF is not Ready. The canonical guest route may need
the VM booted first: reserve/acquire before activation, then spawn/configure at
the existing post-start mailbox seam and withhold launch acceptance until Ready.

**Lifetime and return.** Owned mode watches FD3 EOF as an unconditional parent
death fence: immediately stop/drop mailbox serving and exit even when the VM is
alive. Do not use the legacy launcher-OR-VM survival rule. The daemon-only end
must not be inherited by any helper/launcher; test EOF while the VM-liveness
fixture stays true. The helper must enforce this while waiting for stdin and
while binding/serving. Linux parent-death signal can strengthen, not replace,
the channel; macOS uses the same EOF contract without claiming Linux facilities.
Stop marks the slot stopping, closes its lifetime channel, awaits the original
Child's event and collected status, then returns Collected. Preserve uncertain
wait results and credits. Daemon death changes boot identity and invalidates old
leases; surviving PID evidence cannot reconstruct ownership or a success result.

**Wired RED order.** (1) old signed-byte goldens plus malformed ancillary/type/
count/truncation/FD-close cases; (2) actual daemon spawns a real helper fixture,
launcher receives exactly its stdin writer, agent receives no key bytes, helper
compares context; drop/duplicate/lost response never respawns or clones writer;
(3) canonical owned helper binding failure cannot return Ready, bounded input/
startup deadline and wrong-context refuse; (4) daemon abrupt exit with VM-live
fixture forces serving exit; (5) resident SDK and detached CLI mailbox-only paths
explicitly Acquire despite empty services, retain cross-client locator, then
Stop/Query collects without an external test wait; (6) startup publication error,
budget exhaustion, concurrent Stop and stale epoch/boot preserve original
ownership and shared credit. Only then enable the new path and retry one VM.
The existing `read_frame`/`write_frame` control path, helper unbounded stdin reader,
background bind retry and legacy survival policy cannot implement this contract
unchanged. These are exact prerequisite edits, not evidence the feature exists.
