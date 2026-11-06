# Replace a closed carrier while settling one completed mailbox Call

The initial completed-tail change reuses the original cached Reply and canonical
receipt on a surviving connection. The next change must settle that same
exchange when the original connection closed before the receiver obtained its
final acknowledgment. It retains one protocol and one canonical authority.

1. In `crates/spiralcore-mvm-adapter/src/live.rs`, add an opt-in transport
   result distinguishing confirmed EOF before any next-frame prefix byte from
   truncated prefix or payload, timeout, cancel, and generic socket errors.
   Keep strict receive behavior. Test real TCP clean FIN, partial prefix and
   payload, and sticky closure of stale views in `tests/live.rs` or its existing
   transport test module. Confirm the exact safe-close result before allowing
   recovery or writing a replacement handshake. Risk is medium.
2. In `crates/spiralcore-mvm-live/src/campaign/{canonical,session,completed}.rs`,
   retain the receiver's original post-Reply completion state under the same
   owner. Keep opaque delivery and tracker handles, original cache charge,
   reply identity, both deadline domains and cumulative send/event counters.
   One already-returned caller tail remains charged. An eligible close quiesces
   the old connection. One explicit trusted fixture replacement, with its
   readiness exchange bounded by the original deadline, retries only the
   original cached Reply. The caller accepts an exact canonical Duplicate and
   writes its receipt. The receiver releases the cache once on verified receipt.
   Use the existing canonical service and tracker methods. Risk is high.
3. In `crates/spiralcore-mvm-live/tests/completed_tail.rs` and the actual relay
   fixture, require a complete withheld ACK on the first connection, two joined
   old pumps, original charge retained, exact cached replay on the replacement,
   one application execution and result, then a subsequent exact object
   publication and independent readback. Test wrong peer/epoch/lineage,
   old connection still live, both expiry clocks, partial I/O, stale attempts
   and exhausted counters. Report old/new leg bytes including setup and refused
   writes, CPU on both endpoints and total elapsed time. Trusted test-owned TCP
   sockets establish the state machine. The discriminator is a readiness
   handshake and supplies no peer authentication. Risk is high.
4. At the canonical installed boundary, MVM Core's `InstalledMailboxJob` is
   FnOnce and its handoff consumes the original job. Runtime
   `workload_runner/runner/installed/mailbox_socket.rs` accepts one socket and
   revokes original readiness at invocation. Add an original-owner rearm seam
   only after designing admission under the existing signed plan, grant, route,
   peer bindings and epoch. Preserve isolated queues, original aggregate
   reservation, deadline and audited retirement. Prove on the signed native
   release that the same installed owner can admit exactly one eligible
   replacement and refuses an independently launched guest or new grant.
   Risk is high. Full process-crash recovery, WAN and capacity remain separate.

A local write or clean FIN supplies no remote completion fact. A failed or
ineligible replacement may not create a new Call, return a second application
result, extend lifetime, or convert an unfinished exchange to a success.
