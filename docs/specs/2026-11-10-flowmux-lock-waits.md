# Release-driven waits for FlowMux ownership

Status is an architecture decision for the next measured Windows repair at source `16efe4c2586fd627dbdfedaf0f982a8331a2ac54`. Native evidence reported for the preceding timing sources is diagnostic input. The 5fc Windows 2025 sustained gate stopped at 3,661/3,664 blocks after 240 seconds with about 153 process CPU seconds. Earlier f9 timing recorded session park totals of 15.76/13.29 seconds against requested totals of 1.790/1.677 seconds, multi-second receiver TX acquisition, and Commit means around 38–40 ms. The data motivates removing mutex-contention sleeps; it does not establish that this change alone closes the gate.

## Decision

Use a small standard-library condition-variable release gate around the original `std::sync::Mutex` for TLS Session and shared TX. Keep the actual std mutex guard through the protected operation, preserving its poisoning behavior. Add no dependency. The present FlowMux lock closure contains neither parking_lot nor lock_api. Existing TCP_NODELAY is already enabled.

The readiness waker remains responsible for socket and TLS readiness. It has one poll owner and coalesced hints. Using it as a mutex-release event would mix ownership predicates and could miss releases that make no byte progress. A parking_lot timed mutex could supply timed acquisition, but importing it would also require explicit replacement of std poison semantics and a dependency review. The standard release gate is the smaller change.

## Primitive and ordering

The private owner contains `Mutex<T>`, one short gate `Mutex<()>` and `Condvar`. No waiter list or worker is needed. Its guard owns the original `MutexGuard<T>` and a borrowed gate reference, retains the std guard's thread restrictions, and exposes only the existing protected value operations.

A contended acquisition locks the short gate, checks owner/cancellation state and the original absolute deadline, and attempts the data mutex with `try_lock`. If unavailable, `wait_timeout` atomically releases the gate until notified or the remaining deadline expires. Loop on the actual predicate after spurious wakeups. Always recheck deadline and owner state after acquiring the data guard before exposing it. A timed-out observation cannot become a late successful operation.

Guard release drops the original data guard first, then synchronizes through the short gate and notifies all waiters. Every release notifies, including an operation with no TLS progress or an error return. A waiter holds the gate while checking the data predicate, so release cannot fall between its final check and enrollment in the condition-variable wait. Never block acquiring data while holding the gate. Never perform protected operations, I/O, authority callbacks, sleeps or logging while holding the gate.

Check callbacks run outside the gate where they can acquire another lock. Recheck their outcome before exposing the acquired guard. The gate critical section contains only predicate checks on the data mutex, remaining-time calculations and condition-variable enrollment. Maintain the data-unlock-before-gate order on every release. Descheduling and condition-variable mutex reacquisition can delay return beyond the requested time, but cannot authorize late work.

Preserve data poison as the existing TLS error or duplex `Error::Worker`. Never recover and use a poisoned data value. A poisoned gate also refuses acquisition. Guard Drop must notify during unwinding and must not panic if the gate is poisoned. It may obtain the poisoned gate guard solely for notification, without treating poison as permission to resume work.

## Integration and fencing

In `tls.rs`, replace only SESSION ownership retry sleeps. Keep the distinction between inability to own Session and socket WouldBlock after entering Session. If peek currently owns the poller, release that poller before waiting for Session ownership and reacquire/recheck the existing TLS readiness sequence afterward. Do not sleep on a condition variable while retaining a poller needed by another operation. Existing read/write/flush/handshake readiness paths keep their own deadlines and behavior.

In `duplex.rs`, replace timed TX contention at sender, receiver and half-close acquisition. Keep pending-request publication, frame checks, response matching and bounded channels intact. Existing brief diagnostic TX access must use the same guard/release wrapper so it cannot bypass notification. Timing-enabled and ordinary builds use the same acquisition primitive; counters observe it rather than choosing a different loop.

`Shared::stop` marks closure before notifying Session waiters. `Control::close/stop` must wake TX waiters as well as shutting down the carrier. Install one fixed weak reference to the original TX release gate before the reader starts. Refuse conflicting registration and avoid an ownership cycle. Notification never acquires the data mutex. Check cancellation and current authority after wake and before the original operation. A release, timeout or owner shutdown is a wake source; a condition-variable notification itself grants no authority.

Do not invent prompt scope-revocation notification if the current scope owner has no registration API. Preserve its actual admission checks and disclose that an independently revoked policy is rechecked at the next release/deadline unless the existing canonical cancellation path closes the owner. There is no successful operation after a failed authority check. No new periodic contention polling, spin loop, timer-resolution request, deadline extension, crypto change or protocol message belongs to this slice.

## Evidence

First reproduce release-before-wait, release-with-no-progress, multiple waiters, spurious notification, deadline expiry, shutdown while held, gate poison and data poison in focused deterministic tests. Use barriers and controlled ownership rather than performance assertions. Exercise both TX response ownership and TLS peek/poller order. Ensure reader cancellation wakes the original TX waiter and all original owners remain retained through failed cleanup.

Record condition-variable counts/requested/actual elapsed separately from existing TLS readiness parks. Keep per-frame send/response and receiver processing spans, process CPU, actual TLS bytes and all progress/retention/scope evidence. Overlapping spans remain nonadditive. The native gate remains exactly 4,096 blocks per direction and 240 seconds with exact byte equality and the same grants. Compare feature-on measurements against the preceding feature-on source, then confirm feature-off acceptance. Commit processing may remain storage-bound; use the observations to decide its next optimization separately.
