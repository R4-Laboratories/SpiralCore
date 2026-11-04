# Preboot worker ledger: source review and Drop regression

**BLOCK at fd76a358 pending owned-child Drop cleanup.** Reviewed its private
ledger and budget-fault closure5b0903ca against the full five-outcome Ari plan.
This is intermediate work, not release/Clippy approval or completed B6a.

Reviewed controller SHA-256:
`2b5f4479b0979211c7b2a745da565a6983616515f91060908b5d13f2c9d2eec4`.
Root authorized this reviewer to append the focused Drop test after the source
finding; this reviewer did not change product code. Test SHA-256:
`7fb3af51a8e0e0b9ebf70a62aa6570e8d99f9b556303ded6b6d343fa98352019`.

## Actual finding and RED

InstalledAuthority owns ordinary std Child values. Controller/last-grant drop
can drop that authority; its owner field releases EX before the worker ledger,
and ordinary Child Drop neither terminates nor reaps a process. A bound live
child can therefore outlive every Rust custody handle and owner exclusion.
Future VMM SH does not repair this exact-child ownership defect.

The appended test transfers a real READY-signaled, stdin-blocked /bin/sh Child,
drops grant/controller, then observes waitpid(WNOHANG) from its original parent.
Actual RED: return0, child still live; expected ECHILD after product reaping.
Errno in the raw return0 observation is stale and carries no meaning. Fallback
closes the test's private pipe and positively reaps before post-Drop assertions;
fresh successor Active1 and subsequent explicit reconciliation are checked.
No numeric PID is used to signal any process. Failure leaves no live orphan.

Retained `validation/worker-drop-red.txt` and exit101; source pins are
`worker-drop-red-source.sha256`. This was one exact serial harness execution,
0 passed/1 failed/10 filtered. Existing unused-private-API warnings remain
unsuppressed. Root owns the product fix and subsequent GREEN/qualification.

The proposed bounded drain is aligned: close admission, take at most two exact
Children under the worker lock, release it, then kill/poll outside locks under
one shared5s monotonic deadline. Reaping is distinct from backend confirmation.
Restore unresolved handles and quarantine one authority Arc on unknown/timeout
to retain EX until process exit; no hidden reaper, PID signaling, slot refund,
epoch reset or Clean. Admission cannot reopen that fixed installation. Kill/wait
must retain supported exclusive Child reaper/SIGCHLD custody assumptions.

## Preserved ledger properties

Two slots bound occupancy. Monotonic checked serial, role and Arc identity
prevent stale/foreign tokens operating on replacements. Rejected bind preserves
the caller's Option<Child>. Successful reap alone does not free a slot; trusted
backend confirmation is separate. Failed exit closes launch but final finish
can still use zero usage, positively reaped children and original holders EX.
Unknown Child reaping remains blocking, unlike a known-reaped unknown backend.
Storage/accounting fencing still disallows finish. Shared poison/underflow now
atomically closes authority admission without taking another mutex.

No cancellation may refund the spawn-to-bind window. Before actual integration,
encapsulate reserve/spawn/bind or mark Launching before effects. An unbound
reservation remains charged; Drop cannot silently cancel it. Ordinary tests
cover two slots, reap/confirm, stale serial/foreign Arc and global finish but
do not replace the new shutdown regression.

## Minimal actual consumer wiring

Worker stores `Legacy(owned Child)` or `Installed(WorkerGrant)` custody. Start
receives one explicit launch context, reserves first, and uses a temporary owned
Child rollback guard until bind succeeds. On refusal, that guard terminates/
reaps its exact Child; missing context never selects legacy. Finish polls
try_reap briefly under the private worker lock, waits outside it, reads bounded
host receipts outside locks, then confirms only the trusted backend-exit
contract. Any unknown receipt/exit closes launch and forbids verifier/reuse.
Installed cleanup never invokes persisted-PID cleanup-worker/attach().kill.

Retain the five-record live grant channel from the approved supervisor contract:
child challenge, parent bound grant, SH-held Ready, fresh boot challenge, Go.
The parent owns one private endpoint per slot; installed child owns the other.
Keep all channel/pipe I/O outside owner/accounting/worker mutexes. A short
private validation method copies original epoch/slot/serial/role and checks
open/custody; it does not return an Arc, owner or MutexGuard. Use existing
kernel peer identity/observer and exact unreaped Child association. Socketpair
creator credentials do not become child credentials after fork. EOF/stale
nonce/error refuses and actual backend receives only holder custody.

The actual consumer must use these private APIs before strict Clippy can pass.
Do not publish them or suppress dead-code warnings to manufacture release
qualification. Native worker/VM ownership, actual death/readmission, aggregate
plaintext/cache budgets and the remaining four delivery outcomes remain open.
