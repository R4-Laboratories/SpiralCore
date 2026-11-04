# FlowMux original reader completion v1

Status: additive contract before API-absence tests and implementation. The
existing protocol, framing, object commitment and ordinary join/drop behavior
remain unchanged. These methods establish cleanup of the original reader;
they confer no network, scope or resource authority.

```rust
impl JoinGuard {
    pub fn try_join_until(&mut self, deadline: std::time::Instant) -> Result<()>;
}
impl MappedStream {
    pub fn close_and_join_until(&self, deadline: std::time::Instant) -> Result<()>;
}
```

Both methods use the caller's original absolute monotonic deadline throughout.
Timeout returns `Error::Timeout` while retaining the exact original JoinHandle
and completion receiver. It cancels the same original carrier and never detaches,
replaces or launches a reader. A fresh explicit cleanup attempt may finish the
same handle; it cannot reopen I/O or reset authority. An already positively
joined reader is idempotently complete. A panic returns `Error::Worker` and
remains a failed cleanup outcome.

The non-consuming JoinGuard method waits on the existing original completion
channel. Only after its message or disconnection can a bounded one-millisecond
compatibility observation wait for std's final native thread epilogue. A handle
is taken and joined only after `is_finished()` establishes that the original
thread has finished. A fresh deadline check follows the join. The original
worker's semantic result/counters are retained for the existing consuming
`join()` method; cleanup success alone is not a successful object transfer.

MappedStream first cancels its existing shared carrier and obtains its guard
with the same deadline. It calls the non-consuming completion operation without
removing the guard on timeout. All clones share that one guard. This adds no
reader, watcher, pool, route, retry or codec. Legacy callers retain their existing
consuming join and Drop semantics; installed callers must explicitly complete
the original reader before their final mapped-stream/resource release.

The installed host adapter forwards this operation through its existing payload
trait and FlowSocket. Plain sockets have no codec reader. A custom mapped
implementation that lacks completion support refuses the installed cleanup
operation; legacy paths do not start using it implicitly. The owned relay group
retains a pending-reader phase after joining the relay thread. Reader completion
and final mapped/store release run outside the ledger. Timeout retains the
original mapped wake, reader/codec permit and auxiliary/lease custody, fences
admission and cannot produce Drained. A later explicit original-handle cleanup
may finish. The group's one-Arc uncertainty quarantine remains held until
original-child exit; pool drainage alone does not claim auxiliary/lease release.

Qualification requires the real reader blocked in canonical authority handling:
an original-deadline timeout must return while its endpoint remains alive, then
the same reader must positively join after release. Cover direct JoinGuard,
shared mapped aliases, idempotent completion, expired deadline, retained legacy
result and the installed pending-reader path. Stalled TLS setup is a separate
original-socket wake test/fix. Native pthread/TLS/cache and full endpoint resource
and lifecycle qualification remain separate acceptance gates.
