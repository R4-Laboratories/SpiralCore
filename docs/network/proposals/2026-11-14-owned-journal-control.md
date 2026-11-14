# Owned journal control using canonical guest sessions

Status: reviewed implementation proposal; native qualification pending.
Scope: installed Linux/Firecracker journal endpoint, not a new wire protocol.

The signed rc.3 guest handles PID-1 activation synchronously. After sending
the authenticated activation ACK it waits for the original continuation to
finish before starting its mailbox listener, telemetry and concurrent control
accept loop. The existing installed entrypoint retains that continuation for
one empty-input invocation. The ordinary persistent SDK instead opens another
connection and waits for Ping/Pong. Combining those two lifecycles leaves
startup waiting, even though the warm worker has started.

The seventh native attempt reproduced the readiness refusal. Its rollback also
collected B's original mailbox helper unassisted with exit 0 and successfully
published its original endpoint retirement into the shared parent. It did not
complete a transfer or the pair. Earlier attempts retain the missing kernel,
finite address-space exhaustion, unsupported transcript/verb shape and unsafe
helper-mode refusals. One pre-helper failure needed assisted orphan containment;
that remains failed recovery evidence.

## Selected transition

Keep the immutable signed guest and its existing protocol. For the explicitly
admitted journal mode, finish the original activation-only connection after its
authenticated ACK and final original-parent/kernel checks. This lets PID 1 enter
its normal post-activation initialization. Preserve the existing one-shot
entrypoint behavior for other modes.

Retain a typed journal-control owner under the actual installed native slot.
It holds the original RunningVm/Firecracker custody and anchor, original host
identity and first authenticated guest key. Its readiness and one-shot input
calls use the existing protected Ping and RunEntrypoint messages on separate
one-request sessions, connected through that original VM. Each session must
authenticate the actual Firecracker peer and the saved guest key before sending
input. VM-name discovery and a newly learned replacement key cannot supply this
authority.

Selection and each call require the actual signed journal-mailbox grant and
boot binding, exact activation/run verbs, ephemeral workload streams, current
parent and unexpired admission. The two verbs alone do not select this mode.
Durable mailbox messages, epochs, journal directions, receipts and retained
telemetry keep their existing semantics and authorities.

Only one control operation may be active per original VM. Bound one-shot input
at 64 KiB and cumulative output at 128 KiB, under the already funded controller
workspace. Every connection has an absolute I/O deadline and current-authority
checks. An observed authenticated terminal permits another application call;
ambiguous write/read, key change, timeout, authority loss or callback unwind
fences further calls. There is no automatic replay of an uncertain workload.
Fencing must preserve the original stop and retirement authority.

Expose the closed operations through the retained SDK owner. Adapt the witness
call boundary to this owner while retaining its existing enrollment, transfer,
receipt, exact-byte verification and byte meters. Share the protocol logic;
do not duplicate the mailbox codec or envelope.

## Qualification

Independent source/design review accepted this host-only direction. Required
controls cover ACK-before-close, completed post-activation readiness, original
peer/key/parent checks, foreign/stale identity refusal, concurrency, bounds,
terminal-gated reuse, uncertain-call fencing, continued original retirement and
unchanged one-shot behavior. The actual two-guest transfer and retirement must
then pass with exact artifact identities, both directions of traffic, CPU and
time. No native success, release, WAN, fleet or FIPS claim follows from the
proposal or component tests.
