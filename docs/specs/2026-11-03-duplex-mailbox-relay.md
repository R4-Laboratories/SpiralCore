# ADR: bounded duplex forwarding before canonical session retry

Date: 2026-11-03. Status: approved architecture for the C8 relay prerequisite;
implementation, exact-source review and executed tests remain required.
Baseline: product `91baf28`, merged at `0e02dd7`. Authority:
[persistent-service plan C8](../plans/2026-11-03-persistent-mvm-service.md).

## Problem and decision

`orchestrator::relay` currently reads forward Call, reverse receipt, reverse
Reply, forward Reply receipt in that fixed order. A missing reverse event blocks
forward repair, and reverse traffic cannot progress before the first forward
frame. Pipeline one does not make that choreography safe for C8 retry.

Use two scoped blocking direction pumps after the existing readiness handshakes:
one A→B and one B→A. Each direction has exactly one reader and one writer, using
the existing Unix streams plus four fallible `try_clone` handles: two for each
pump, while the original two handles retain supervisor shutdown authority. Clones are
bounded file descriptors for the same two admitted links, not additional links.
No reader/writer mutex spans socket I/O. Keep canonical framing, route admission,
encryption/signatures, endpoint authority, the 2,048-Call cap and pipeline one.
This prerequisite adds no automatic retry or peer state-machine change.

The sequential alternative cannot satisfy independent progress. A larger async
reactor adds an unnecessary runtime/scheduling change for two owned connections;
two joined threads provide the required bounded seam.

## Frame and EOF contract

Add `live::read_ciphertext_frame_or_eof` with the existing read parameters and
`LiveResult<Option<Vec<u8>>>`. `None` means a read returned zero before **any**
byte of the next four-byte length prefix. Detect this using that operation's
local prefix offset, not cumulative counter differences or an error string.
Return `Some` only after the full valid nonzero bounded length and payload.
Allocate payload only after validating length against `MAX_ENVELOPE_BYTES`.

Clean EOF must not invoke `Shutdown::Both`. EOF after 1–3 prefix bytes or during
payload is fatal truncated framing; malformed length, deadline, cancellation,
counter overflow and I/O failures remain fatal. Count every actual prefix and
payload byte, including partial failure; increment frame count only on completion.
Preserve the existing helper's strict API/behavior: its wrapper converts `None`
to fatal `UnexpectedEof` and closes both halves. Existing endpoint users retain
their current failure semantics. Neither helper authenticates a mailbox frame.

On clean input EOF, the pump shuts down **only its output's Write half**, then
exits successfully. The opposite pump continues draining buffered or subsequent
reverse frames. A half-close is not cancellation or destination acceptance.
After both pumps finish, close remaining owned handles. On any other error,
cancel the shared token and shut down both logical sockets with `Shutdown::Both`
to wake the sibling. Both threads are joined on success and error; panic and
partial thread-start failure must also trigger shutdown/join, never detach work.

Retain the fixed 30-second bound for each read and each write operation, using
the existing sliced cancellation-aware I/O. A partial operation cannot refresh
its deadline; successful frames may start another bounded operation. An idle
read therefore fails after 30 seconds. This is not an indefinitely idle service
or a guarantee of maximum total session duration.

## Admission, reservations and accounting

Use the same `HostTransportBudget` and existing two `LinkReservation`s. Before
activating either pump, acquire one maximum-frame reservation for each direction;
if the second refuses, release the first and start neither pump. Clone/start
failure likewise releases only owned resources after any started pump is joined.
The existing owner/tenant ceilings remain **two frame slots / 524,288 reserved
serialized-buffer bytes**, with 262,144 bytes per direction. Do not raise them.

Each pump retains its own reservation through idle read, frame decoding, route
validation and output write; drop all owned frame representations before the
next read or releasing credit. At most one wire buffer per direction is live.
Idle high water of two slots/524,288 bytes is expected reserved capacity, not
measured allocation or RSS. Decoded-envelope copies retain the existing bounded
exclusion from serialized-buffer accounting; no total-memory claim is added.
Ledger guards cover reserve/release only and never read, write, wait or join.
Apply the existing provisioned direction's `RouteAdmission::validate` to every
decoded outer envelope before forwarding. The keyless relay neither classifies
trusted receipt traffic nor creates receipts, retries, grants or plaintext proof.

Each pump owns separate input/output counters. Assemble endpoint A from A's
handshake counters plus forward-input and reverse-output counters; assemble B
from its handshake counters plus reverse-input and forward-output counters.
Use checked arithmetic for every field and aggregate. No shared mutable counter
is accessed during I/O. Preserve partial counters in failure diagnostics, join
both results before returning, and never label a failed relay as completed.
Aggregate process CPU remains one relay measurement; do not sum overlapping
process-clock worker intervals. Frame/link credit returns after buffers/tasks end.

## Focused RED/GREEN acceptance

Use tiny canonical route-valid encrypted frames over Unix socket pairs, bounded
watchdogs and explicit synchronization; join test workers even on failed gates.

- Deliver a reverse frame before any forward Call and observe its exact bytes
  at A while forward remains withheld. The old sequential relay must fail.
- Forward two Calls while the reverse event is withheld; both reach B unchanged
  before reverse release. This proves forwarding freedom, not peer retry support.
- Half-close A's Write with a reverse frame buffered, then drain that frame at A
  and half-close B's Write. Both directions finish cleanly with exact counters.
- Inject partial prefix and partial payload EOF: fail, cancel/wake both pumps,
  preserve actual partial counters, join both, and return all owned reservations.
- Cover owner refusal before activation, bad route/length, blocked write timeout,
  cancellation and startup failure; peak stays within the unchanged owner budget.
  A blocked direction must not prevent the other admitted direction's progress.

Keep successful existing direct and relayed campaign checks as compatibility
gates. These tests do not establish endpoint demultiplexing, cached Reply replay,
acknowledgment recovery, same-owner long retention, WAN or fleet capacity. Full
C8 still requires both-peer state handling and direct/relayed failure regressions
before retransmission can be enabled.
