# Installed host owner: B6a design proposal

Status: architecture proposal for root review; no implementation or qualification.
Base: main after PRs 66/67 (`7c220cc8`). This is a local installation ownership
boundary, not a canonical MVM wire, guest capability or plaintext-budget change.

## Source boundary

- `host_budget.rs` owns an in-process `Arc` ledger and validates only
  `scope=orchestrator`. Independent constructors have independent ceilings.
- `orchestrator.rs::session_owned` creates a new run directory; its JSONL
  `emit` flushes userspace writes, with `sync_all` at session end. That journal
  is evidence, not a durable pre-spawn installation admission record.
- `Worker` retains `Child`; Drop kills/waits that child and Linux invokes
  `cleanup-worker` in its private `MVM_HOME`. SIGKILL bypasses Drop.
- Linux workers own an external Firecracker and observe exit through pidfd;
  macOS ARM64 workers execute HVF in their worker process. Disk sidecar custody
  protects a writable image, not installation-wide activation.
- Pinned MVM `ae12e9d` supplies Linux `ProcessIdentity` (boot ID + start ticks),
  `ProcessExitObserver`, and `require_owned_child_custody`. `FcDriver::attach`
  reads a persisted PID; its existence alone does not confer restart-safe kill
  authority. An observer is not a signaling authorization.

## Three choices

| Approach | Benefit | Cost / limitation |
| --- | --- | --- |
| Stable OS lock + durable activation fence | Smallest cross-platform safety slice; reuses current Child/backend cleanup | An owner crash may leave the installation unavailable until positive reconciliation; no automatic orphan killing |
| Guardian with inherited lease/lifeline | Can preserve exclusion while orphan children drain | Must prove every worker and external Linux VMM is covered across fork/exec, worker death and FD closure; larger launch/lifecycle change |
| Managed service/process domain | systemd/cgroup or launchd can own lifecycle centrally | Platform deployment, privilege and service-manager contract; not the current witness's portable minimal slice |

Recommend phased combination of the first two: durable refusal first, then
lease coverage of every actual resource holder and positive restart
reconciliation. Refusal alone is partial B6a. A released supervisor lock never
proves children exited. Full B6a requires the recovery stages below.

## Private owner and stable storage

Use a crate-private `InstallationOwner` which owns one OS-exclusive owner lock,
one installation state and one existing `HostTransportBudget`. Require its
borrow for every installed session/Worker launch. No network-selected scope.
One installed session at a time is sufficient; no daemon or shared counter IPC.

The installation directory is a fixed trusted local provisioning choice,
separate from each new run directory. Initialize it explicitly once; opening an
existing installation never recreates missing state/lock as a clean owner.
Use owned private directory/files, no symlinks, held descriptors, checked
device/inode identity and descriptor-relative operations. Keep the lock inode
stable: never unlink, rename, truncate or replace it. Use nonblocking exclusive
`flock` on `owner.lock` on Linux/macOS, with CLOEXEC; reject unsupported
filesystems/platforms. Full recovery adds a second stable `holders.lock` inode
for resource occupancy, not a second admission authority.
Changing the configured installation path creates a different administrative
scope, not evidence of a larger host ceiling. Trusted same-UID/root operators
must not replace installation objects or run bypass launchers.

`Busy` means another owner holds the lock; `RecoveryRequired` means acquired
lock but nonterminal state; invalid/torn/missing state or durability failure
fences admission. Do not classify these by error strings. A poisoned owner
cannot construct a fresh local budget to continue. No lease expires by time.

Keep one bounded current state (e.g. <=16 KiB, two simultaneous worker slots),
not an unbounded journal scan. Fields: version, installation ID, epoch ID,
phase (`Clean`, `Active`, `Unresolved`), lease protocol version, config digest, run path
identity and up to two launch-slot records. Slot records contain fixed role,
launch intent, private run/home identity, backend kind, observed process
identities where supported and terminal proof flags; PIDs are diagnostic only.
No bootstrap, capability secrets or payloads. Preserve per-run JSONL evidence.

Publish state with private temporary file + sync + atomic rename + directory
sync, outside the short transport-ledger mutex. Failed publication is an error,
never permission to activate. An Active record survives all ambiguous exits;
Drop must not mark Clean. Storage faults during cleanup leave admission fenced.

## Admission and retirement order

1. Open/validate fixed installation, acquire OS lock, read bounded state.
   Only durable Clean permits a new epoch; nonterminal state requires the
   recovery path before new run files, sockets, boot or application activation.
   Invalid state always refuses.
2. Reserve the existing fixed transport links and conservative two-worker
   grant (total <=2 vCPU / 2 GiB). Persist Active plus a slot's launch intent
   **before spawn**, then start the child. Persist observed identity as available.
   A crash in the intent-to-identity interval stays unresolved, not child-free.
3. Retain the owner throughout relay task joins, worker/recovery-worker stages
   and all cleanup. Slot reuse for the verifier requires confirmed prior exit;
   never overlap a third guest. Link/frame credit remains existing RAII.
4. Reap owned children, positively observe backend exit, synchronize/release
   owned disk custody, join tasks and destroy buffers. Only then persist Clean
   and permit next activation. Result/guest_done alone is insufficient. Failed
   attempts may become Clean only with the same complete teardown proof.
5. Release OS lock last. Owner death instead leaves durable Active; successor
   can hold the lock for diagnosis, but cannot admit work or reset old charges.

Check `require_owned_child_custody` before spawn and retain exclusive Child
reaping ownership; reaped numeric PID is never subsequently a kill target.
Normal cleanup must not launder Linux VMM kill failure into successful release.
Before calling any PID-based backend termination, obtain/revalidate the existing
backend ownership binding; if exact lifetime-safe signaling is unavailable,
keep the installation unresolved instead of asserting that a name/comm matches.
This design does not claim that identity-check-then-kill(PID) is atomic.

## Partial stage: restart refusal

No successor invokes `cleanup-worker PID` solely from persisted records. A
recorded Linux `ProcessIdentity::has_exited` can prove the original lifetime
ended (including reboot/reuse); permission/read/parse failure cannot. That
proof alone cannot settle a launch intent with an unrecorded VMM, or a worker
which could still spawn after observation. All launchers plus all backend
lifetimes must be covered, and owned disk locks must be reacquirable without
modifying data. Capture identities while the original owner has custody.

macOS's pinned `ProcessIdentity` is Unsupported. Fresh kqueue registration of a
stored PID is not proof of the old lifetime. The partial stage therefore refuses
automatic restart reclamation there. No `reset --force`, marker deletion, timeout,
kill(pid,0), executable name or PID disappearance converts ambiguity to Clean.
Safe refusal on both platforms is useful; it does not complete B6a.

## Full B6a: holder leases and safe reconciliation

Private seams (semantic names; no public network DTO):

```rust
InstallationOwner::open_existing(path) -> Result<InstallationOwner>;
InstallationOwner::begin_epoch(config) -> Result<InstalledEpoch>;
InstalledEpoch::prepare_worker(role) -> Result<PreparedWorkerLease>;
InstalledWorkerLease::prepare_backend() -> Result<PreparedBackendLease>;
InstalledEpoch::finish(proof: OwnedTeardownProof) -> Result<()>;
```

Guards have private fields, are non-serializable and bind stable installation
objects, epoch, role/slot and bounded config. Only the supervisor mutates durable
state. Lease handoff is a local process resource operation, not guest authority.
An environment string or reconstructed PID cannot construct a valid guard.

Before first spawn, the owner holds SH on `holders.lock` and durably records
Active with lease protocol v1. Each worker obtains/validates SH before preparing
or booting a backend; retain it for the entire process lifetime, including
failed boot and cleanup. Every backend launcher obtains SH and validates the
original Active epoch/slot while holding SH before resource creation/boot.
Retain SH through exec and the actual VMM lifetime. Parent custody covers the
handoff. No explicit `LOCK_UN`: close/drop duplicates only when finished.

Mac HVF runs in the worker, so retained worker custody covers its VM lifetime.
Linux Firecracker must itself retain a locked descriptor, including diskless
sender and verifier paths. Worker-only custody cannot qualify Linux recovery.

Pinned `LockedWritableImage` demonstrates dup/last-close custody. However,
`fc/daemon.rs::reopen_handoff_script` crosses sudo by reopening
`/proc/<worker>/fd/N`: sudo closes inherited descriptors, and this source
explicitly rejects dependence on `sudo -C`/sudoers changes. Reopen preserves the
object, not its original open-file description/flock. Inode inventory does not
prove lock custody. Do not encode a host lease as a fake writable disk.

Add a small explicit host-process lease seam to the pinned MVM host launcher,
with recorded patch provenance. Two eligible mechanisms:

- Direct privileged launch preserving a true dup through exec, only where the
  current launcher can retain canonical scope/resource enforcement. No silent
  root/direct-launch bypass.
- Keep the existing privileged launcher, reopen the exact held lease object,
  then a narrow trusted host helper explicitly acquires SH and validates the
  original epoch/slot before exec. Retain that newly locked description in
  Firecracker. These are independent SH leases; reopen is not treated as dup.

Recommend the second for compatibility. Helper/lock/epoch/scope/descriptor
failure refuses boot. No sudo policy extension or broad PID signaling is needed.
A delayed launcher cannot boot into an obsolete epoch: either it already holds
SH and blocks successor EX, or successor acquires EX and retires the epoch,
then the delayed launcher obtains SH and rejects the changed epoch. No resource
may be created before this SH-plus-epoch gate. Test death at every boundary.

A successor owns `owner.lock`, then attempts EX on `holders.lock`. WouldBlock
means live/uncertain holders: refuse activation and signal nobody. EX success
for a v1 epoch proves all compliant old holders which could own/create resources
released custody; late uncommitted launchers are excluded by the epoch check.
Retain EX while reconciling only recorded private run objects, reacquiring disk
custody and synchronizing cleanup; never delete unknown objects. Persist old
epoch retirement/Clean before enabling another epoch. Crash during recovery
retains a nonterminal state. Legacy/missing lease coverage remains fenced.
Keep owner authority through the occupancy EX-to-new-SH transition and publish
the new epoch before spawning; no conversion gap may revive old launchers.

An orphan holding SH may run to its existing bounded exit or block recovery;
no new guest is admitted meanwhile. Positive recovery means recognizing final
release and safely readmitting. Force-termination of an unresponsive holder
requires separate exact OS-handle/privileged authority, not attach().kill(PID).

## Minimal implementation and focused gates

1. **Partial safety**, new private owner/state helper beside `host_budget.rs`: stable lock, bounded
   typed state, durable transitions and fail-closed open. RED: two subprocesses
   contend on the same inode; no second owner; aliases cannot bypass the lock.
2. `orchestrator.rs` / explicit installed host entry in `main.rs`: owner borrow
   required before installed activation. Preserve existing development paths
   as explicitly orchestrator-scoped; they cannot emit installed/host coverage.
   RED: refusal before filesystem/Worker start, including Stage One and verifier
   paths; no default constructor silently bypasses the installed entry.
3. **Full holder coverage**, `host.rs`, `linux_host.rs`, `orchestrator.rs`,
   `main.rs` plus minimal pinned MVM host-launch patch/provenance: SH/epoch gates
   on disk and diskless branches, preserving CPU scope. RED: killed supervisor
   cannot release occupancy while worker lives; killed supervisor AND Linux
   worker cannot release it while the actual VMM lives. Fake holders test
   ordering first; actual backends establish resource coverage.
4. **Crash/orphan reconciliation**: inject failure before spawn, after
   spawn/before identity, during relay,
   backend cleanup and state sync. Kill owner while two bounded test children
   live; successor acquires OS lock but refuses activation and signals neither.
   Exercise torn/oversized/missing state, lock replacement and failed sync.
   Delay a privileged helper until successor retires an epoch: it must refuse
   before boot. Crash before/after SH acquisition and epoch validation. Missing
   actual VMM lease fails launch. After final holder exits, successor must
   safely reconcile and admit exactly one new epoch. Malformed/legacy state or
   unreleased disk custody retains the fence. Positive readmission is mandatory.
5. Prove teardown and no unsafe signaling separately: legacy missing identity,
   denied process table and PID reuse cannot authorize PID cleanup. Competing
   reaper, failed cleanup or held disk custody retain the fence. Fully qualified
   v1 occupancy EX is the cross-platform lifetime proof; it does not require a
   reconstructed PID. Normal fully observed exit permits the next epoch.
6. Native Linux and macOS ARM64 qualification uses at most two actual guests,
   2 vCPU / 2 GiB total, unchanged no-NIC routes. Record exclusion, crash refusal,
   backend exit and resource release independently. No new run is authorized by
   this proposal. Windows native MVM remains unavailable; macOS Intel is not HVF
   execution evidence for this crate.

Keep `scope=orchestrator` until executed B6a exclusion/reconciliation evidence
and installed-launch closure justify a reviewed report change. Even then,
coverage is the installation-managed serialized transport domain, not machine
RSS, plaintext, arbitrary same-host MVMs or total host memory. B6b conservative
signed service partitions/shared plaintext authority remains required; neither
tail ciphertext accounting nor >2048-session retention is delivered here.
