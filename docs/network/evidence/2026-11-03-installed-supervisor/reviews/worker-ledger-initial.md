# Worker ledger: ongoing CODE/RUST review

**Ledger primitives acceptable; actual launcher integration remains HOLD.**
Reviewed committed fd76a3585f09a5145ab41b1d438a2e98d7ebdc45 worker delta and
5b0903ca budget-fence propagation. No new authority/serial/accounting escape
was found in those primitives. This is not a release approval: the real consumer
is pending and strict Clippy correctly reports its unused private APIs. Do not
make them public or suppress warnings to claim integration. No product edits,
builds or tests were performed by this reviewer.

The fixed two slots, checked global serial, original role and Arc identity bind
a grant to one controller/occupant. Stale and foreign grants cannot bind/reap a
replacement. Refused binding leaves the exact caller Option<Child> intact.
try_wait operates on the retained Child; positive reaping is cached, failed or
unknown outcomes close admission, and a success alone does not recycle a slot.
Only the private backend-confirmation path frees it. Reaped failed/unconfirmed
slots may participate in final cleanup only through original kernel holder EX,
not authorize reuse. Unbound/live/unknown slots still block try_finish. Transport
snapshot locks are released before worker/owner locks; no reverse nesting found.
5b correctly propagates budget underflow/poison into the shared admission flag
without taking worker/owner locks under the transport mutex.

## Concrete integration gaps and focused countercases

1. **Definite spawn failure has no positive cleanup route.** Reserve followed
   by Command::spawn Err leaves child=None/reaped=None, so try_finish remains
   Busy despite zero usage/no Child ever created. Grant Drop intentionally
   cannot prove/refund this. Encapsulate reserve→fixed spawn→bind with an owned
   pending-child transaction; distinguish proven no-spawn from abandoned or
   ambiguous reservation. Countercase: inject a definite spawn failure, close
   admission, prove no live child, then positive final cleanup rather than an
   unexplained permanent Busy. Preserve the existing abandoned-token refusal.
2. **Bound Child cleanup must use its exact owned handle.** Once bind consumes
   the Child, existing public Worker cleanup cannot access it directly. Current
   ledger exposes polling but no cancellation path. Child Drop alone neither
   kills nor reaps; last authority Drop can therefore discard a live Child if
   the future Worker does not perform cleanup first. Add a narrow private exact-
   child cancel/reap path and test startup error/Worker Drop after bind, no slot
   reuse before reaping, plus unknown outcome retaining charge. Never reconstruct
   a PID or call persisted cleanup-worker to satisfy this boundary.

These are pending launcher obligations, not an asserted escape through the
currently absent installed entry. Current conservative behavior is preferable
to pretending an unbound/unknown slot completed, but cannot be the entire final
error lifecycle. Root was asked for the two focused countercases, not a broad
test expansion.

## Use the private API through the actual launcher

Installed Worker::start should reserve a role, create its fixed command and
socketpair, and immediately take exact Child custody through that transaction.
Worker retains the non-Clone grant; keep it outside WorkerLedger, because storing
a Worker-with-grant in its own authority would introduce an Arc cycle. Current
Child-only slots have no such cycle. Legacy dispatch remains explicit.

The parent must check the exact current grant, unreaped Child and open authority
for fresh channel responses. Report/channel/backend-contract failure must close
admission even if the process exits0. confirm_worker_exit is trusted bookkeeping,
not a validator of the report. Reserve(Verify) is currently capacity-only and
can succeed before primaries; the actual launcher must gate it on both primary
backend contracts and positive teardown. Add that ordering countercase at the
consumer, rather than treating an available slot or success status as proof.

Read worker-first-green:2 new private worker tests pass,8 filtered, exit0. They
cover exact custody/stale token, two-slot retention, successful confirmation,
failed exit closure and final real-holder Busy→exit→finish. Existing accounting
fault tests check immediate flag closure before controller shutdown. Strict
worker-clippy is FAIL on five dead-code diagnostics; it is not a product behavior
failure or a gate already cleared. Current new test extensions are in progress
and were excluded from this committed-source verdict. Full native/worker channel,
backend lifetime, default integration and all five Ari outcomes remain ongoing.

Committed source pins:

```text
2b5f4479b0979211c7b2a745da565a6983616515f91060908b5d13f2c9d2eec4  src/installed_supervisor.rs
bcea3aaa9f582f0ce8c01b1259be4aa39389c6f456f6b8842a7ec53e950f2194  src/host_budget.rs
a384e087244fa5840c5c22af8a8ed1dfabec4406acb9fcb53637b33a5d7d0aff  src/installed_supervisor/tests.rs
```

PR70's completed release and sealed packets were not re-audited or modified.
