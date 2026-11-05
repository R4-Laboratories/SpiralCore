# Original Runtime slot attachment for the installed mailbox

The already-verified Core handoff supplies the exact original parent owner.
A Linux-only socket job comes from that owner through a default-refusing
hook. Runtime supplies the existing private directory descriptor and name.
The job is an in-process implementation detail of the trusted provider.
Plan JSON, guest input and ordinary start options cannot supply a callback,
listener path, credentials or a completion flag.

The Runtime slot runs attachment as an exclusive custody operation. Check
the exact anchor, take the one-shot handoff and ask its original owner for
the job. Prepare the worker with the held directory descriptor and handoff
as its charge. Put the complete worker in the original slot before bind or
spawn. The operation releases the slot mutex and restores the owner on
success, refusal or unwind. Duplicate or foreign attachment refuses.

Normal Runtime stop first closes mailbox admission and wakes I/O. After
the existing original native-exit barrier it joins the worker and verifies
socket removal before dropping the handoff. The host-only fixture exercises
the same attachment and socket cleanup code without manufacturing a native
retirement receipt. Its original parent is a fixture. Real ledger and
canonical UDS behavior were independently witnessed in earlier slices.

This phase leaves Hostd and Core native refusals in force. The provider's
shipping socket hook still defaults to unsupported until it has the actual
authorized mailbox session and can budget the complete I/O workspace. No
guest spec entry is emitted here. Before enabling one, bind the real tenant
broker, authenticated bootstrap, and cancellable shared Standard rate and
verify the full installed ownership lifecycle.
