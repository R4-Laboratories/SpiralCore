# Fatal delivery fence and clean completion

Root approved this reason-state correction after actual authenticated/live-scope
buffered delivery reproduced a semantic RED. No frame, codec, profile, sequence,
identity, timeout, capacity or authority enrollment change is made.

Fatal cancellation is monotonic and distinct from physical I/O having ended.
Explicit cancel fences future queued/dequeued/buffer-copy delivery even after
graceful I/O is already closed. Normal completion closes I/O only after both
application halves and pending response state have drained; verified queued data
may then drain under fresh current scope. Graceful close never clears a fatal
flag. Tx/Rx Drop fallback must not upgrade previously graceful closure. Errors,
unwind and unjoined guard Drop remain fatal. Successful join observes completion;
its own Drop does not invent cancellation afterward. Unknown/failed/timed-out
joins fence rather than confer delivery permission.

A delivery check is the admission/linearization point for one bounded copy or
dequeue. It does not promise to revoke bytes halfway through a concurrent copy
that was already admitted. Empty reads transfer no data. Previously returned
bytes cannot be retrospectively erased. Publication acknowledgment remains
separate from application consumption or continued permission after cancel.

Preserve the original Tx synchronization before treating post-AppFIN EOF as
clean: the peer can observe FIN before the writer publishes local_closed.
The Tx guard orders that observation with actual FIN flush and flag publication;
release it before complete() relocks Tx. Existing Tx→Pending order remains.

One real authenticated regression checks fatal cancellation with still-valid
scope and unchanged output, clean physical close with exact buffered drain,
write-after-FIN refusal without destroying that drain, explicit cancel after
clean closure, clone fencing, and successful joined-worker queued drain with
reciprocal actual byte counters. Original saturated-queue acknowledgments,
fifth refusal, deadlines/reentry/failed joins remain; post-fatal delivery now
requires typed Cancelled. No metadata/path/PID substitute or alternative ACK.
