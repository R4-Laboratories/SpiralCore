# Production admission for one installed mailbox replacement

1. Add a distinct opt-in signed launch profile under Hostd's existing
   installed-mailbox policy asset. Bind dedicated channel identities, route
   and recovery version in the signed enrollment. Cover absent, duplicate,
   foreign and legacy selection against the original host admission checks.
2. In MVM Core's installed host-lifetime/mailbox-socket modules, tie a
   non-cloneable one-use capability to the original bound job and listener
   identity. Keep boot readiness revoked after initial accept. The original
   worker retains custody of the same listener and stop wakeup. Arm only
   after qualified frame-boundary EOF and old transport quiescence. Burn the
   attempt before accepting or authenticating. No second job or VM.
3. In Runtime's installed mailbox socket worker, publish the replacement
   accepted descriptor under the existing stop control after checking the
   original route and stopped state. Stop can interrupt both accept and
   authentication. A worker cannot reacquire the installed slot operation
   lock because Stop may hold it while joining that worker. Use a narrow
   immutable route proof with live original ownership and backend liveness.
   Retain the original bridge and listener with no path rebind.
4. Reuse the canonical MVM authenticated session and sealed carrier. Verify
   both enrolled keys and bind a fresh mailbox-specific channel session to
   the original plan, live boot, grant and one replacement attempt. The
   default host session accepts a self-presented guest key. Use its pinned
   handshake machinery or a narrow expected-peer constructor before any
   mailbox effect. Keep canonical mailbox frame bytes and receipt semantics.
   Recheck lease, grant and both deadlines before the replay. Readiness and
   Unix peer credentials alone cannot establish guest application identity.
5. Native acceptance keeps the same live VM, worker and ledger from original
   ACK loss through eligible FIN and authenticated redial. Require one exact
   Reply replay, one verified ACK, one application execution and cache release.
   Test unauthorized local socket, wrong peer/enrollment/epoch, prequeued and
   second attempt, wrong or dead original bridge, truncation/reset, expiry
   and Stop during accept and authentication. A fresh challenge proves fresh
   authentication. An assertion about the time of dial itself also requires
   an authenticated after-arm signal. Report handshake and carrier bytes
   separately, CPU, elapsed, reservations and original-owner retirement.

This plan is based on the pinned Runtime and SDK used by the installed
development witness. The authenticated replacement has not yet run inside
a native VM. Review and qualify concrete deployment cryptography separately.
