# Installed supervisor v1: one epoch, one budget, explicit worker grants

Status: proposed next implementation contract. Depends on
[installed owner](installed-host-owner-v1.md) and
[Firecracker lease](firecracker-host-lease-v1.md). No product/native qualification
is established here. One installed session, at most2guests/2vCPU/2GiB, unchanged
canonical wire and signed admission. B6b plaintext/tail accounting remains open.

## Controller and ownership

```rust
pub struct InstalledConfig { /* validated private fixed profile */ }
pub struct InstalledSupervisor { /* non-Clone; private budget+authority */ }
impl InstalledSupervisor {
    pub fn start(owner: InstallationOwner, config: InstalledConfig)
        -> Result<Self, SupervisorError>;
    pub fn budget(&self) -> &HostTransportBudget;
    pub fn close_admission(&mut self);
    pub fn try_finish(&mut self) -> Result<(), SupervisorError>;
}
// Native-host orchestrator only; consumes this supervisor's single session use.
pub fn session_installed(controller: &mut InstalledSupervisor, root: &Path,
    kernel: &Path, guest: &Path, plan: Plan) -> Result<Value>;
```

New `installed_supervisor.rs` owns private `InstalledAuthority`:
`owner: Mutex<InstallationOwner>`, immutable `Epoch`, `admission_open: AtomicBool`,
and a bounded private worker ledger. `InstalledSupervisor` owns one Arc to this
authority and one budget. `host_budget.rs::Shared` gains an optional private Arc
to that same authority; scopes/links/frames already retain Shared transitively.
Authority has no budget back-reference, so no cycle. No public owner/Arc/Mutex,
raw lock-file getter, take-owner or installed-budget constructor exists.

`start` consumes the owner once. Validate all config and fixed-profile digest
against the owner's fresh key/profile before `begin`; add a narrow crate-private
profile check in installed_owner.rs, not an alternate state parser. Begin exactly
one epoch, create exactly one installed Shared, and never restart/rebind it.
Allocation/unwind after begin retains Active; Drop does not repair it. Config
retains current2links/2frame slots/524288 serialized bytes and two fixed worker
grants of1vCPU/1024MiB each. It does not create a second plaintext quota.

Public `HostTransportBudget::new` remains independent/orchestrator-scoped.
Only this private factory can attach installed authority. A cloned installed
budget is accounting access, not a new controller or launch grant. Existing
public `session_with_budget` must reject installed-bound budgets before files
or activation; otherwise it would route them into legacy unleased workers.
The installed entry calls the private shared session core with an explicit
borrowed launch context. Existing independent development sessions remain
explicitly outside installed coverage, never silently relabelled host scope.

## Admission, close and outliving guards

Use controller phases Open, Closing, Finished and Fenced. The controller is
non-Clone; session execution borrows it mutably, so another session/finish cannot
race supported worker starts. Installed budget scope/link/frame admission checks
the shared admission flag under the existing short ledger mutex. Poison,
underflow or authority error closes admission; no independent replacement owner.
Releases and snapshots remain available where accounting is still trustworthy.

`close_admission` irreversibly clears the flag, then takes/releases the budget
ledger mutex as a barrier against charges already in flight. No owner/worker
mutex or filesystem I/O under that ledger mutex; no reverse nesting. New scope,
link, frame and worker reservations refuse (existing budget Fenced error is
sufficient; controller errors distinguish Closed/Busy/State/Owner/Budget).

`try_finish` first closes admission. Require a successful zero-usage budget
snapshot and all owned Child handles reaped, then call original owner.finish
with the retained epoch outside accounting locks. Actual VMM leases must still
make holders EX Busy. Busy preserves the closed controller and exact epoch for
retry; storage/accounting failure stays fenced, never resets usage or epoch.
A failed worker cannot authorize a verifier/new spawn, but final cleanup may
still finish after positive global holder exclusion. Canonical owner storage
fencing is not bypassed. No implicit retry/start/reopen occurs in Drop.

Successful finish does not extract/drop the owner from the authority. Even an
idle budget/scope clone therefore retains owner EX until its final drop. Live
link/frame/worker guards also retain it. Controller Drop only closes admission;
last authority Drop releases descriptors and leaves Active unless explicit
finish already succeeded. No Clean-on-Drop and no Arc-count-based cleanup proof.

## One launcher interface and fixed grants

Every `Worker::start`, including sender, receiver, verifier and Stage One, takes
one private `LaunchContext`: explicit Legacy or a borrowed Installed controller.
Only legacy public entry constructs Legacy. Installed paths cannot select it
on error/missing configuration. A non-Clone WorkerGrant reserves a bounded slot,
role and fixed resource/profile grant before spawn; Worker owns it and the exact
Child. Ledger accounting is private and never reconstructed from guest counters.
Permit retirement follows reaped owned child plus the known host worker's
successful backend-exit contract. Unknown/error outcome closes/fences admission;
no slot reuse or verifier launch on that failure. Verifier requires both primary
workers' successful teardown. Final owner.finish remains the global lease proof.

Stage One uses the same owner/worker gate; its host plaintext transport remains
explicitly unaccounted by serialized relay v1, with the existing null/coverage
label. It cannot bypass installed ownership merely because relay links are unused.

## Worker activation: private live-controller channel

Key/generation/role strings and reacquiring owner EX are not grant proof: after
owner death a fresh independent open could take EX over old Active state. Do not
export owner EX into the child or VMM. Instead, each reserved slot gets one
anonymous `UnixStream::pair`, with no listener/path/port. Controller endpoint
stays private in its Worker; only the child endpoint crosses exec at a fixed
validated descriptor. Set CLOEXEC again in worker before any backend spawn.
At most two live pairs; no daemon, new server thread or generic command service.

`worker-installed` requires that descriptor; no missing-channel or legacy
fallback. Its private context can be constructed only by the bounded exchange:

1. Child gets kernel peer PID with existing
   `mvm_vmm::host::uds_peer::connected_peer_pid`, validates expected live parent,
   arms existing ProcessExitObserver, then sends a fresh32-byte challenge.
2. `Worker::start` checks its exact unreaped Child, reserved slot and open
   controller, then answers that challenge with key/profile/epoch/role/slot.
   Parent metadata is not accepted as independent authentication of the socket.
3. Child validates the response and fixed native profile, acquires the original
   HolderLease while checking Active epoch, and sends Ready. Parent then supplies
   the existing bounded credential-bearing initramfs on its original pipe.
4. After reading that input, child sends a second fresh challenge before backend
   activation. The same parent issues Go only for that still-live owned Child
   and original reserved grant. No grants for a different role/epoch/socket.

Five fixed bounded control records (<=1024 bytes each), one original bounded
worker-start deadline (reuse40s maximum), explicit cancellation/EOF/error refusal
and no retry/rebind. No payload/credential in these control records or logs.
The parent drives this synchronously in Worker::start, not through broker I/O.
Retain the worker context/SH through all host boot/cleanup/reporting. Linux passes
only the existing prepared holder handoff to actual FC; macOS keeps SH through
the in-process HVF lifetime. Parent endpoint must not be delegated to another
process. Socketpair credentials identify its creator: do not claim the parent's
peer PID magically becomes the post-fork child's PID; Child custody binds that end.
Linux SO_PEERCRED captures identity at socketpair creation, as specified by
[unix(7)](https://man7.org/linux/man-pages/man7/unix.7.html). Darwin LOCAL_PEERPID
behavior needs an actual native pair/fork/exec fixture; unsupported or ambiguous
peer identity refuses installed activation, never falls back to an argv PID.

EOF before a fresh grant refuses. A valid Go can race later controller death;
do not promise atomic death-to-boot cancellation. That grant was already charged,
and retained worker/VMM SH prevents retirement/readmission while it is live.
Old endpoints/nonces are not recovered after restart. Hostile same-UID/root
channel/record forgery remains outside the established provisioning model.

## Exact integration and RED gates

- `host_budget.rs` / new `installed_supervisor.rs`: private one-shot factory,
  flag/charge barrier, authority retention and no cycle. RED: drop controller
  while a sole scope/link/frame/budget clone survives; competing owner remains
  Busy, no Clean; all final guards release exclusion. Closed clones cannot charge.
- `orchestrator.rs`: one explicit launch context across all starts and private
  common session core. RED: legacy wrapper rejects installed budget before files;
  third worker refuses; error primary forbids verifier; no hidden fallback.
- `main.rs`, `host.rs`, `linux_host.rs`: explicit installed session/worker
  commands using validated fixed local provisioning. Initialization is separate;
  worker cannot initialize/reset. RED: missing/replayed/wrong-peer channel, old
  nonce, role/profile/epoch mismatch and controller death before Go boot nothing.
  Use deterministic subprocess barriers and exact owned Child cleanup.
- Reuse pinned MVM uds_peer/process_exit; add direct native-Mac dependency
  visibility if necessary, keeping the same source pin and feature closure.
  Do not enable linux-host in guest builds or alter hardware-SHA feature defaults.
- Installed shutdown must not call persisted-PID cleanup-worker/attach().kill.
  Normal worker owns backend termination; parent cancels/reaps only its exact
  Child. Unknown backend lifetime leaves SH/Active and Busy until actual exit.
  RED: finish Busy retains identity, later positive holder exit permits finish;
  no new owner until outliving accounting guards also drop.

Retain orchestrator report scope until actual native all-path custody and
owner/worker-death/readmission qualification. Scope is installation-managed
resources, not unrelated same-host labs or mandatory machine-wide control.
Native Windows MVM, B6b, >2048 retention and the wider delivery goal stay open.
